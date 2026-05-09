use anyhow::{ensure, Result};
use chrono::Utc;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::conversations::append_conversation_event;
use crate::events::RealtimeEvent;
use crate::llm_gateway::{CompiledPrompt, LlmGatewayRequest, LlmUsageMetadata};

pub const LLM_ACCOUNTING_SCHEMA_VERSION: &str = "llm.accounting.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LlmInvocationStatus {
    Started,
    Completed,
    Failed,
    Cancelled,
}

impl LlmInvocationStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Started => "started",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LlmUsageRollup {
    pub key: String,
    pub token_count: i64,
    pub estimated_cost_micros: i64,
    pub entry_count: i64,
}

pub fn estimate_tokens(content: &str) -> i64 {
    let trimmed = content.trim();
    if trimmed.is_empty() {
        return 0;
    }
    let word_count = trimmed.split_whitespace().count() as i64;
    let character_estimate = ((trimmed.chars().count() as i64) + 3) / 4;
    word_count.max(character_estimate).max(1)
}

pub fn record_invocation_started(
    connection: &Connection,
    request: &LlmGatewayRequest,
    prompt: &CompiledPrompt,
    policy_decision_id: &str,
) -> Result<()> {
    let now = Utc::now().to_rfc3339();
    connection.execute(
        "INSERT INTO llm_invocations (
            id,
            conversation_id,
            segment_id,
            capability_id,
            provider_id,
            model_id,
            status,
            prompt_hash,
            privacy_transform_run_ids_json,
            policy_decision_id,
            started_at,
            metadata_json
         ) VALUES (?1, ?2, ?3, 'llm.invoke', ?4, ?5, ?6, ?7, '[]', ?8, ?9, ?10)",
        params![
            request.run_id,
            request.conversation_id,
            request.segment_id,
            request.provider_id,
            request.model_id,
            LlmInvocationStatus::Started.as_str(),
            prompt.prompt_hash,
            policy_decision_id,
            now,
            json!({
                "schemaVersion": LLM_ACCOUNTING_SCHEMA_VERSION,
                "promptId": prompt.prompt_id,
                "clientIdHash": request.client_id.as_ref().map(|client_id| stable_hash(client_id)),
                "userMessageHash": stable_hash(&request.user_message),
                "providerUsageSource": "pending",
            })
            .to_string(),
        ],
    )?;
    Ok(())
}

pub fn record_prompt_slot_usage(
    connection: &Connection,
    request: &LlmGatewayRequest,
    prompt: &CompiledPrompt,
    policy_decision_id: &str,
) -> Result<Vec<RealtimeEvent>> {
    let now = Utc::now().to_rfc3339();
    let mut events = Vec::new();
    for slot in &prompt.slots {
        let slot_usage_id = format!("llm_slot_usage_{}", Uuid::new_v4());
        let estimated_tokens = estimate_tokens(&slot.content);
        connection.execute(
            "INSERT INTO llm_prompt_slot_usage (
                id,
                invocation_id,
                slot_id,
                slot_version,
                source_refs_json,
                visibility,
                estimated_tokens,
                actual_tokens,
                content_hash,
                included,
                truncation_reason,
                created_at
             ) VALUES (?1, ?2, ?3, 'v1', ?4, ?5, ?6, NULL, ?7, 1, NULL, ?8)",
            params![
                slot_usage_id,
                request.run_id,
                slot.id,
                json!(slot.source_refs).to_string(),
                slot.visibility_ceiling,
                estimated_tokens,
                slot.content_hash,
                now,
            ],
        )?;
        events.push(append_conversation_event(
            connection,
            &request.conversation_id,
            request.segment_id.as_deref(),
            None,
            "llm.prompt.slot.accounted",
            json!({
                "runId": request.run_id,
                "slotUsageId": slot_usage_id,
                "slotId": slot.id,
                "slotVersion": "v1",
                "visibility": slot.visibility_ceiling,
                "estimatedTokens": estimated_tokens,
                "contentHash": slot.content_hash,
                "included": true,
            }),
            Some(policy_decision_id),
        )?);
    }
    Ok(events)
}

pub fn record_privacy_transform_runs(
    connection: &Connection,
    invocation_id: &str,
    transform_run_ids: &[String],
) -> Result<()> {
    connection.execute(
        "UPDATE llm_invocations
         SET privacy_transform_run_ids_json = ?2
         WHERE id = ?1",
        params![invocation_id, json!(transform_run_ids).to_string()],
    )?;
    Ok(())
}

pub fn record_invocation_completed(
    connection: &Connection,
    request: &LlmGatewayRequest,
    usage: Option<&LlmUsageMetadata>,
    policy_decision_id: &str,
) -> Result<Vec<RealtimeEvent>> {
    let now = Utc::now().to_rfc3339();
    connection.execute(
        "UPDATE llm_invocations
         SET status = ?2,
             completed_at = ?3,
             metadata_json = json_set(metadata_json, '$.providerUsageSource', ?4)
         WHERE id = ?1",
        params![
            request.run_id,
            LlmInvocationStatus::Completed.as_str(),
            now,
            if usage.is_some() {
                "provider_reported"
            } else {
                "unreported"
            },
        ],
    )?;
    let mut events = Vec::new();
    if let Some(usage) = usage {
        events.push(record_ledger_entry(
            connection,
            request,
            "provider_input",
            usage.input_tokens,
            json!({ "source": "provider_reported", "costKind": "not_priced" }),
            policy_decision_id,
        )?);
        events.push(record_ledger_entry(
            connection,
            request,
            "provider_output",
            usage.output_tokens,
            json!({ "source": "provider_reported", "costKind": "not_priced" }),
            policy_decision_id,
        )?);
        update_slot_actuals(connection, &request.run_id, usage.input_tokens)?;
    }
    Ok(events)
}

pub fn record_invocation_failed(
    connection: &Connection,
    invocation_id: &str,
    code: &str,
    message: &str,
) -> Result<()> {
    update_terminal_invocation(
        connection,
        invocation_id,
        LlmInvocationStatus::Failed,
        Some(code),
        Some(message),
    )
}

pub fn record_invocation_cancelled(connection: &Connection, invocation_id: &str) -> Result<()> {
    update_terminal_invocation(
        connection,
        invocation_id,
        LlmInvocationStatus::Cancelled,
        None,
        None,
    )
}

pub fn rollup_usage_by_conversation(connection: &Connection) -> Result<Vec<LlmUsageRollup>> {
    rollup_usage(connection, "conversation_id")
}

pub fn rollup_usage_by_provider_model(connection: &Connection) -> Result<Vec<LlmUsageRollup>> {
    let mut statement = connection.prepare(
        "SELECT provider_id || '/' || model_id AS rollup_key,
                COALESCE(SUM(token_count), 0),
                COALESCE(SUM(estimated_cost_micros), 0),
                COUNT(*)
         FROM llm_token_ledger_entries
         GROUP BY provider_id, model_id
         ORDER BY rollup_key ASC",
    )?;
    collect_rollups(&mut statement)
}

pub fn rollup_usage_by_capability(connection: &Connection) -> Result<Vec<LlmUsageRollup>> {
    rollup_usage(connection, "capability_id")
}

pub fn rollup_usage_by_prompt_slot(connection: &Connection) -> Result<Vec<LlmUsageRollup>> {
    let mut statement = connection.prepare(
        "SELECT slot_id,
                COALESCE(SUM(estimated_tokens), 0),
                0,
                COUNT(*)
         FROM llm_prompt_slot_usage
         GROUP BY slot_id
         ORDER BY slot_id ASC",
    )?;
    collect_rollups(&mut statement)
}

fn record_ledger_entry(
    connection: &Connection,
    request: &LlmGatewayRequest,
    usage_kind: &str,
    token_count: i64,
    pricing_snapshot: Value,
    policy_decision_id: &str,
) -> Result<RealtimeEvent> {
    ensure!(token_count >= 0, "token count cannot be negative");
    let ledger_entry_id = format!("llm_ledger_entry_{}", Uuid::new_v4());
    let now = Utc::now().to_rfc3339();
    connection.execute(
        "INSERT INTO llm_token_ledger_entries (
            id,
            invocation_id,
            conversation_id,
            capability_id,
            provider_id,
            model_id,
            usage_kind,
            token_count,
            estimated_cost_micros,
            pricing_snapshot_json,
            metadata_json,
            created_at
         ) VALUES (?1, ?2, ?3, 'llm.invoke', ?4, ?5, ?6, ?7, 0, ?8, ?9, ?10)",
        params![
            ledger_entry_id,
            request.run_id,
            request.conversation_id,
            request.provider_id,
            request.model_id,
            usage_kind,
            token_count,
            pricing_snapshot.to_string(),
            json!({
                "schemaVersion": LLM_ACCOUNTING_SCHEMA_VERSION,
                "costEstimate": true,
                "rawPromptStored": false,
            })
            .to_string(),
            now,
        ],
    )?;
    append_conversation_event(
        connection,
        &request.conversation_id,
        request.segment_id.as_deref(),
        None,
        "llm.ledger.entry.recorded",
        json!({
            "runId": request.run_id,
            "ledgerEntryId": ledger_entry_id,
            "usageKind": usage_kind,
            "tokenCount": token_count,
            "estimatedCostMicros": 0,
            "pricingSnapshot": pricing_snapshot,
        }),
        Some(policy_decision_id),
    )
}

fn update_slot_actuals(
    connection: &Connection,
    invocation_id: &str,
    input_tokens: i64,
) -> Result<()> {
    let slot_count: i64 = connection.query_row(
        "SELECT COUNT(*) FROM llm_prompt_slot_usage WHERE invocation_id = ?1",
        params![invocation_id],
        |row| row.get(0),
    )?;
    if slot_count == 0 {
        return Ok(());
    }
    let mut remaining = input_tokens.max(0);
    let mut statement = connection.prepare(
        "SELECT id, estimated_tokens
         FROM llm_prompt_slot_usage
         WHERE invocation_id = ?1
         ORDER BY rowid ASC",
    )?;
    let slots = statement
        .query_map(params![invocation_id], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let estimated_total: i64 = slots.iter().map(|(_, estimated)| *estimated).sum();
    for (index, (slot_usage_id, estimated_tokens)) in slots.iter().enumerate() {
        let actual_tokens = if index == slots.len() - 1 {
            remaining
        } else if estimated_total > 0 {
            ((input_tokens.max(0) * *estimated_tokens) / estimated_total).max(0)
        } else {
            0
        };
        remaining = remaining.saturating_sub(actual_tokens);
        connection.execute(
            "UPDATE llm_prompt_slot_usage SET actual_tokens = ?2 WHERE id = ?1",
            params![slot_usage_id, actual_tokens],
        )?;
    }
    Ok(())
}

fn update_terminal_invocation(
    connection: &Connection,
    invocation_id: &str,
    status: LlmInvocationStatus,
    code: Option<&str>,
    message: Option<&str>,
) -> Result<()> {
    let now = Utc::now().to_rfc3339();
    let message_hash = message.map(stable_hash);
    connection.execute(
        "UPDATE llm_invocations
         SET status = ?2,
             completed_at = ?3,
             failure_code = ?4,
             failure_message_hash = ?5
         WHERE id = ?1",
        params![invocation_id, status.as_str(), now, code, message_hash],
    )?;
    Ok(())
}

fn rollup_usage(connection: &Connection, column: &str) -> Result<Vec<LlmUsageRollup>> {
    ensure!(
        matches!(column, "conversation_id" | "capability_id"),
        "unsupported usage rollup column"
    );
    let sql = format!(
        "SELECT {column},
                COALESCE(SUM(token_count), 0),
                COALESCE(SUM(estimated_cost_micros), 0),
                COUNT(*)
         FROM llm_token_ledger_entries
         GROUP BY {column}
         ORDER BY {column} ASC"
    );
    let mut statement = connection.prepare(&sql)?;
    collect_rollups(&mut statement)
}

fn collect_rollups(statement: &mut rusqlite::Statement<'_>) -> Result<Vec<LlmUsageRollup>> {
    let rows = statement.query_map([], |row| {
        Ok(LlmUsageRollup {
            key: row.get(0)?,
            token_count: row.get(1)?,
            estimated_cost_micros: row.get(2)?,
            entry_count: row.get(3)?,
        })
    })?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

fn stable_hash(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    format!("sha256:{:x}", hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capabilities::seed_builtin_capabilities;
    use crate::conversations::{
        create_conversation_participant, find_or_create_canonical_conversation,
        CanonicalConversationRequest, ConversationParticipantCreateRequest,
    };
    use crate::llm_gateway::PromptSlot;
    use crate::policy::{
        record_policy_decision, ActorContext, PolicyAction, PolicyDecision,
        PolicyDecisionCorrelation, PolicyOutcome, ResourceKind, ResourceRef,
    };
    use crate::schema::init_schema;

    fn test_connection() -> Connection {
        let connection = Connection::open_in_memory().unwrap();
        init_schema(&connection).unwrap();
        seed_builtin_capabilities(&connection).unwrap();
        connection
            .execute(
                "INSERT INTO actors (id, actor_kind, display_name, status, metadata_json, created_at, updated_at)
                 VALUES ('actor_staff', 'staff', 'Staff', 'active', '{}', 'now', 'now')",
                [],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO connections (
                    id, connection_type, display_name, status, identity_json, scope_json, metadata_json, created_at, updated_at
                 ) VALUES ('connection_1', 'client', 'Client', 'active', '{}', '{}', '{}', 'now', 'now')",
                [],
            )
            .unwrap();
        connection
    }

    fn request_and_prompt(connection: &Connection) -> (LlmGatewayRequest, CompiledPrompt) {
        let conversation = find_or_create_canonical_conversation(
            connection,
            &CanonicalConversationRequest {
                surface: "client_portal".to_string(),
                subject_kind: "connection".to_string(),
                subject_id: "connection_1".to_string(),
                connection_id: Some("connection_1".to_string()),
                visitor_session_id: None,
                created_by_actor_id: Some("actor_staff".to_string()),
            },
        )
        .unwrap();
        let assistant = create_conversation_participant(
            connection,
            &ConversationParticipantCreateRequest {
                conversation_id: conversation.id.clone(),
                participant_kind: "assistant".to_string(),
                actor_id: None,
                connection_id: None,
                visitor_session_id: None,
                display_name: "Ordo".to_string(),
                role: "assistant".to_string(),
            },
        )
        .unwrap();
        let slots = vec![
            PromptSlot::new(
                "ethical_business_persuasion",
                "Ethical Business Persuasion",
                "Use verified evidence only.",
                vec!["source_1".to_string()],
                "Respectful business communication.",
                "staff_private",
            )
            .unwrap(),
            PromptSlot::new(
                "conversation_brief",
                "Conversation Brief",
                "Client asked for next steps.",
                vec!["source_2".to_string()],
                "Current conversation evidence.",
                "participants",
            )
            .unwrap(),
        ];
        let prompt_hash = stable_hash(
            &slots
                .iter()
                .map(|slot| slot.content_hash.as_str())
                .collect::<Vec<_>>()
                .join("|"),
        );
        (
            LlmGatewayRequest {
                run_id: "llm_run_1".to_string(),
                conversation_id: conversation.id,
                segment_id: None,
                assistant_participant_id: assistant.id,
                client_id: Some("client_llm_1".to_string()),
                provider_id: "local_fake".to_string(),
                model_id: "fake-chat".to_string(),
                user_message: "What next?".to_string(),
                prompt_slots: slots.clone(),
            },
            CompiledPrompt {
                prompt_id: "prompt_1".to_string(),
                prompt_hash,
                slots,
            },
        )
    }

    fn policy_decision_id(connection: &Connection, run_id: &str) -> String {
        record_policy_decision(
            connection,
            &PolicyDecision {
                outcome: PolicyOutcome::Allowed,
                actor: ActorContext::local_owner("test"),
                action: PolicyAction::Generate,
                resource: ResourceRef::new(ResourceKind::LlmRun, run_id),
                capability_id: Some("llm.invoke".to_string()),
                reason: "test policy".to_string(),
            },
            PolicyDecisionCorrelation {
                request_id: Some(run_id.to_string()),
                ..PolicyDecisionCorrelation::default()
            },
        )
        .unwrap()
    }

    #[test]
    fn estimated_tokens_are_deterministic() {
        assert_eq!(estimate_tokens(""), 0);
        assert_eq!(estimate_tokens("one two three"), 4);
        assert_eq!(estimate_tokens("abcdefgh"), 2);
    }

    #[test]
    fn records_invocation_slots_ledger_and_rollups_without_raw_prompt_text() {
        let connection = test_connection();
        let (request, prompt) = request_and_prompt(&connection);
        let policy_decision_id = policy_decision_id(&connection, &request.run_id);

        record_invocation_started(&connection, &request, &prompt, &policy_decision_id).unwrap();
        let slot_events =
            record_prompt_slot_usage(&connection, &request, &prompt, &policy_decision_id).unwrap();
        record_privacy_transform_runs(
            &connection,
            &request.run_id,
            &["privacy_transform_1".to_string()],
        )
        .unwrap();
        let ledger_events = record_invocation_completed(
            &connection,
            &request,
            Some(&LlmUsageMetadata {
                input_tokens: 12,
                output_tokens: 4,
            }),
            &policy_decision_id,
        )
        .unwrap();

        assert_eq!(slot_events.len(), 2);
        assert_eq!(ledger_events.len(), 2);
        let invocation_status: String = connection
            .query_row(
                "SELECT status FROM llm_invocations WHERE id = ?1",
                params![request.run_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(invocation_status, "completed");
        let slot_count: i64 = connection
            .query_row("SELECT COUNT(*) FROM llm_prompt_slot_usage", [], |row| {
                row.get(0)
            })
            .unwrap();
        let token_total: i64 = connection
            .query_row(
                "SELECT SUM(token_count) FROM llm_token_ledger_entries",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(slot_count, 2);
        assert_eq!(token_total, 16);
        assert_eq!(
            rollup_usage_by_conversation(&connection).unwrap()[0].token_count,
            16
        );
        assert_eq!(
            rollup_usage_by_provider_model(&connection).unwrap()[0].key,
            "local_fake/fake-chat"
        );
        assert_eq!(
            rollup_usage_by_capability(&connection).unwrap()[0].key,
            "llm.invoke"
        );
        assert_eq!(
            rollup_usage_by_prompt_slot(&connection)
                .unwrap()
                .iter()
                .map(|rollup| rollup.entry_count)
                .sum::<i64>(),
            2
        );

        for raw in [
            "Use verified evidence only.",
            "Client asked for next steps.",
            "What next?",
        ] {
            for table in [
                "llm_invocations",
                "llm_prompt_slot_usage",
                "llm_token_ledger_entries",
                "conversation_events",
            ] {
                let count: i64 = connection
                    .query_row(
                        &format!(
                            "SELECT COUNT(*) FROM {table} WHERE CAST(COALESCE(metadata_json, '') AS TEXT) LIKE ?1"
                        ),
                        params![format!("%{raw}%")],
                        |row| row.get(0),
                    )
                    .unwrap_or(0);
                assert_eq!(count, 0, "{table} leaked {raw}");
            }
        }
    }

    #[test]
    fn terminal_statuses_record_safe_failure_hashes() {
        let connection = test_connection();
        let (request, prompt) = request_and_prompt(&connection);
        let policy_decision_id = policy_decision_id(&connection, &request.run_id);
        record_invocation_started(&connection, &request, &prompt, &policy_decision_id).unwrap();
        record_invocation_failed(
            &connection,
            &request.run_id,
            "provider_failed",
            "secret payload",
        )
        .unwrap();

        let (status, message_hash): (String, String) = connection
            .query_row(
                "SELECT status, failure_message_hash FROM llm_invocations WHERE id = ?1",
                params![request.run_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(status, "failed");
        assert!(message_hash.starts_with("sha256:"));
        let leaked_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM llm_invocations WHERE failure_message_hash LIKE '%secret payload%'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(leaked_count, 0);
    }
}
