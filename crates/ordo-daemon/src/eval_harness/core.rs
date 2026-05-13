use super::*;
use crate::security::redaction;
use anyhow::{ensure, Context, Result};
use chrono::Duration;
use chrono::{DateTime, Utc};
use rusqlite::{Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct DeterministicEvalClock {
    current: DateTime<Utc>,
    tick: Duration,
}

impl DeterministicEvalClock {
    pub fn new(start: DateTime<Utc>, tick: Duration) -> Self {
        Self {
            current: start,
            tick,
        }
    }

    pub fn fixed() -> Self {
        Self::new(
            DateTime::parse_from_rfc3339("2026-05-09T00:00:00Z")
                .expect("fixed eval clock timestamp is valid")
                .with_timezone(&Utc),
            Duration::seconds(1),
        )
    }

    pub fn next_timestamp(&mut self) -> String {
        let timestamp = self.current.to_rfc3339();
        self.current += self.tick;
        timestamp
    }
}

#[derive(Debug, Clone)]
pub struct DeterministicEvalHarness {
    clock: DeterministicEvalClock,
    artifact_path: Option<String>,
}

impl DeterministicEvalHarness {
    pub fn new(clock: DeterministicEvalClock) -> Self {
        Self {
            clock,
            artifact_path: None,
        }
    }

    pub fn with_artifact_path(mut self, artifact_path: impl Into<String>) -> Self {
        self.artifact_path = Some(artifact_path.into());
        self
    }

    pub fn run_case<F>(
        &mut self,
        connection: &Connection,
        case: &EvalCase,
        mut step_runner: F,
    ) -> Result<EvalScorecardSummary>
    where
        F: FnMut(&Connection, &EvalStep) -> Result<()>,
    {
        let evidence_before = collect_evidence_snapshot(connection, self.clock.next_timestamp())?;
        for step in &case.steps {
            step_runner(connection, step)?;
        }
        let evidence_after = collect_evidence_snapshot(connection, self.clock.next_timestamp())?;
        let assertion_results = case
            .expected_assertions
            .iter()
            .map(|assertion| {
                let actual_count = evidence_after.count_for(assertion.channel);
                let passed = actual_count >= assertion.minimum_after_count;
                EvalAssertionResult {
                    assertion_id: assertion.id.clone(),
                    channel: assertion.channel,
                    expected_minimum: assertion.minimum_after_count,
                    actual_count,
                    passed,
                    note: if passed {
                        "minimum evidence count satisfied".to_string()
                    } else {
                        "minimum evidence count not satisfied".to_string()
                    },
                }
            })
            .collect::<Vec<_>>();
        let passed = assertion_results.iter().all(|result| result.passed);

        Ok(EvalScorecardSummary {
            schema_version: EVAL_HARNESS_SCHEMA_VERSION.to_string(),
            case_id: case.id.clone(),
            title: case.title.clone(),
            fixture_hash: case.fixture_hash.clone(),
            actor_roles: case.actor_roles.clone(),
            step_count: case.steps.len(),
            provider_mode: provider_mode_for_case(&case.id).to_string(),
            network_enabled: false,
            evidence_before,
            evidence_after,
            assertion_results,
            passed,
            artifact_path: self.artifact_path.clone(),
            generated_at: self.clock.next_timestamp(),
        })
    }
}

pub(crate) fn provider_mode_for_case(case_id: &str) -> &'static str {
    if case_id == "replay_provider_fixture" {
        "replay_fixture"
    } else {
        "deterministic_only"
    }
}

pub fn collect_evidence_snapshot(
    connection: &Connection,
    captured_at: String,
) -> Result<EvalEvidenceSnapshot> {
    Ok(EvalEvidenceSnapshot {
        captured_at,
        channels: vec![
            EvalEvidenceCount {
                channel: EvalEvidenceChannel::SqliteRows,
                count: total_evidence_rows(connection)?,
            },
            EvalEvidenceCount {
                channel: EvalEvidenceChannel::ConversationEvents,
                count: table_count(connection, "conversation_events")?,
            },
            EvalEvidenceCount {
                channel: EvalEvidenceChannel::RealtimeReplay,
                count: table_count(connection, "realtime_events")?,
            },
            EvalEvidenceCount {
                channel: EvalEvidenceChannel::PolicyDecisions,
                count: table_count(connection, "policy_decisions")?,
            },
            EvalEvidenceCount {
                channel: EvalEvidenceChannel::PromptSlotAccounting,
                count: table_count(connection, "llm_prompt_slot_usage")?,
            },
            EvalEvidenceCount {
                channel: EvalEvidenceChannel::PrivacyTransforms,
                count: privacy_transform_count(connection)?,
            },
            EvalEvidenceCount {
                channel: EvalEvidenceChannel::TokenLedger,
                count: table_count(connection, "llm_token_ledger_entries")?,
            },
            EvalEvidenceCount {
                channel: EvalEvidenceChannel::AnalysisCandidates,
                count: table_count(connection, "conversation_analysis_candidates")?
                    + table_count(connection, "conversation_brief_candidates")?
                    + table_count(connection, "conversation_memory_candidates")?,
            },
            EvalEvidenceCount {
                channel: EvalEvidenceChannel::HandoffState,
                count: table_count(connection, "conversation_handoffs")?,
            },
            EvalEvidenceCount {
                channel: EvalEvidenceChannel::ArtifactRecords,
                count: table_count(connection, "artifacts")?,
            },
            EvalEvidenceCount {
                channel: EvalEvidenceChannel::SurfaceBriefRecords,
                count: table_count(connection, "surface_briefs")?,
            },
            EvalEvidenceCount {
                channel: EvalEvidenceChannel::FeedbackReviewRecords,
                count: table_count(connection, "customer_feedback")?
                    + table_count(connection, "feedback_tags")?
                    + table_count(connection, "customer_reviews")?,
            },
            EvalEvidenceCount {
                channel: EvalEvidenceChannel::ProductSurfaceRecords,
                count: table_count(connection, "business_facts")?
                    + table_count(connection, "offers")?
                    + table_count(connection, "business_outcomes")?,
            },
        ],
        conversation_event_max_sequence: max_i64(connection, "conversation_events", "sequence")?,
        realtime_replay_max_cursor: max_i64(connection, "realtime_events", "cursor")?,
    })
}

pub(crate) fn total_evidence_rows(connection: &Connection) -> Result<i64> {
    let tables = [
        "conversations",
        "conversation_participants",
        "conversation_messages",
        "conversation_events",
        "realtime_events",
        "policy_decisions",
        "llm_invocations",
        "llm_prompt_slot_usage",
        "llm_token_ledger_entries",
        "conversation_analysis_jobs",
        "conversation_analysis_candidates",
        "conversation_brief_candidates",
        "conversation_memory_candidates",
        "knowledge_graph_node_candidates",
        "knowledge_graph_edge_candidates",
        "conversation_handoffs",
        "artifacts",
        "artifact_deliverables",
        "surface_briefs",
        "customer_feedback",
        "feedback_tags",
        "customer_reviews",
        "business_facts",
        "offers",
        "business_outcomes",
    ];
    tables
        .iter()
        .try_fold(0, |sum, table| Ok(sum + table_count(connection, table)?))
}

pub(crate) fn privacy_transform_count(connection: &Connection) -> Result<i64> {
    let raw: Option<String> = connection
        .query_row(
            "SELECT json_group_array(privacy_transform_run_ids_json) FROM llm_invocations",
            [],
            |row| row.get(0),
        )
        .optional()?;
    let Some(raw) = raw else {
        return Ok(0);
    };
    if raw.trim().is_empty() {
        return Ok(0);
    }
    let groups = serde_json::from_str::<Value>(&raw).unwrap_or_else(|_| json!([]));
    let count = groups
        .as_array()
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.as_str())
                .filter_map(|item| serde_json::from_str::<Value>(item).ok())
                .filter_map(|item| item.as_array().map(|values| values.len() as i64))
                .sum()
        })
        .unwrap_or(0);
    Ok(count)
}

pub(crate) fn table_count(connection: &Connection, table: &str) -> Result<i64> {
    ensure_identifier(table)?;
    let sql = format!("SELECT COUNT(*) FROM {table}");
    let count = connection.query_row(&sql, [], |row| row.get(0))?;
    Ok(count)
}

pub(crate) fn max_i64(connection: &Connection, table: &str, column: &str) -> Result<Option<i64>> {
    ensure_identifier(table)?;
    ensure_identifier(column)?;
    let sql = format!("SELECT MAX({column}) FROM {table}");
    let value = connection
        .query_row(&sql, [], |row| row.get(0))
        .optional()?;
    Ok(value.flatten())
}

pub(crate) fn transcript_ledger(connection: &Connection) -> Result<Vec<EvalLedgerEntry>> {
    let mut statement = connection.prepare(
        "SELECT m.id, m.created_at, m.message_kind, m.body_markdown, m.conversation_id,
                m.participant_id, m.sequence, p.participant_kind, p.role, m.client_message_id
         FROM conversation_messages m
         JOIN conversation_participants p ON p.id = m.participant_id
         ORDER BY m.conversation_id ASC, m.sequence ASC",
    )?;
    let rows = statement.query_map([], |row| {
        Ok(EvalLedgerEntry {
            ledger: "transcript".to_string(),
            id: row.get(0)?,
            occurred_at: Some(row.get(1)?),
            entry_type: row.get(2)?,
            payload: json!({
                "body": row.get::<_, String>(3)?,
                "conversationId": row.get::<_, String>(4)?,
                "participantId": row.get::<_, String>(5)?,
                "sequence": row.get::<_, i64>(6)?,
                "participantKind": row.get::<_, String>(7)?,
                "role": row.get::<_, String>(8)?,
                "clientMessageId": row.get::<_, Option<String>>(9)?,
            }),
        })
    })?;
    collect_rows(rows)
}

pub(crate) fn timeline_ledger(connection: &Connection) -> Result<Vec<EvalLedgerEntry>> {
    let mut timeline = conversation_event_ledger(connection)?;
    timeline.extend(realtime_replay_ledger(connection)?);
    timeline.sort_by(|left, right| {
        left.occurred_at
            .cmp(&right.occurred_at)
            .then_with(|| left.ledger.cmp(&right.ledger))
            .then_with(|| left.id.cmp(&right.id))
    });
    Ok(timeline)
}

pub(crate) fn conversation_event_ledger(connection: &Connection) -> Result<Vec<EvalLedgerEntry>> {
    let mut statement = connection.prepare(
        "SELECT id, occurred_at, event_type, conversation_id, segment_id, handoff_id,
                sequence, policy_decision_id, realtime_cursor, payload_json
         FROM conversation_events
         ORDER BY conversation_id ASC, sequence ASC",
    )?;
    let rows = statement.query_map([], |row| {
        Ok(EvalLedgerEntry {
            ledger: "conversation_events".to_string(),
            id: row.get(0)?,
            occurred_at: Some(row.get(1)?),
            entry_type: row.get(2)?,
            payload: json!({
                "conversationId": row.get::<_, String>(3)?,
                "segmentId": row.get::<_, Option<String>>(4)?,
                "handoffId": row.get::<_, Option<String>>(5)?,
                "sequence": row.get::<_, i64>(6)?,
                "policyDecisionId": row.get::<_, Option<String>>(7)?,
                "realtimeCursor": row.get::<_, Option<i64>>(8)?,
                "payload": parse_json_value(row.get::<_, String>(9)?),
            }),
        })
    })?;
    collect_rows(rows)
}

pub(crate) fn realtime_replay_ledger(connection: &Connection) -> Result<Vec<EvalLedgerEntry>> {
    let mut statement = connection.prepare(
        "SELECT cursor, occurred_at, event_type, family, schema_version, job_id,
                task_key, job_sequence, payload_json
         FROM realtime_events
         ORDER BY cursor ASC",
    )?;
    let rows = statement.query_map([], |row| {
        let cursor = row.get::<_, i64>(0)?;
        Ok(EvalLedgerEntry {
            ledger: "realtime_replay".to_string(),
            id: cursor.to_string(),
            occurred_at: Some(row.get(1)?),
            entry_type: row.get(2)?,
            payload: json!({
                "cursor": cursor,
                "family": row.get::<_, String>(3)?,
                "schemaVersion": row.get::<_, String>(4)?,
                "jobId": row.get::<_, Option<String>>(5)?,
                "taskKey": row.get::<_, Option<String>>(6)?,
                "jobSequence": row.get::<_, Option<i64>>(7)?,
                "payload": parse_json_value(row.get::<_, String>(8)?),
            }),
        })
    })?;
    collect_rows(rows)
}

pub(crate) fn policy_decision_ledger(connection: &Connection) -> Result<Vec<EvalLedgerEntry>> {
    let mut statement = connection.prepare(
        "SELECT id, decided_at, action, actor_kind, actor_id, actor_origin,
                resource_kind, resource_id, capability_id, outcome, reason, metadata_json
         FROM policy_decisions
         ORDER BY decided_at ASC, id ASC",
    )?;
    let rows = statement.query_map([], |row| {
        Ok(EvalLedgerEntry {
            ledger: "policy_decisions".to_string(),
            id: row.get(0)?,
            occurred_at: Some(row.get(1)?),
            entry_type: row.get(2)?,
            payload: json!({
                "actorKind": row.get::<_, String>(3)?,
                "actorId": row.get::<_, Option<String>>(4)?,
                "actorOrigin": row.get::<_, String>(5)?,
                "resourceKind": row.get::<_, String>(6)?,
                "resourceId": row.get::<_, String>(7)?,
                "capabilityId": row.get::<_, Option<String>>(8)?,
                "outcome": row.get::<_, String>(9)?,
                "reason": row.get::<_, String>(10)?,
                "metadata": parse_json_value(row.get::<_, String>(11)?),
            }),
        })
    })?;
    collect_rows(rows)
}

pub(crate) fn prompt_slot_ledger(connection: &Connection) -> Result<Vec<EvalLedgerEntry>> {
    let mut statement = connection.prepare(
        "SELECT id, created_at, slot_id, invocation_id, slot_version, source_refs_json,
                visibility, estimated_tokens, actual_tokens, content_hash, included,
                truncation_reason
         FROM llm_prompt_slot_usage
         ORDER BY created_at ASC, id ASC",
    )?;
    let rows = statement.query_map([], |row| {
        Ok(EvalLedgerEntry {
            ledger: "prompt_slots".to_string(),
            id: row.get(0)?,
            occurred_at: Some(row.get(1)?),
            entry_type: row.get(2)?,
            payload: json!({
                "invocationId": row.get::<_, String>(3)?,
                "slotVersion": row.get::<_, String>(4)?,
                "sourceRefs": parse_json_value(row.get::<_, String>(5)?),
                "visibility": row.get::<_, String>(6)?,
                "estimatedTokens": row.get::<_, i64>(7)?,
                "actualTokens": row.get::<_, Option<i64>>(8)?,
                "contentHash": row.get::<_, String>(9)?,
                "included": row.get::<_, i64>(10)? == 1,
                "truncationReason": row.get::<_, Option<String>>(11)?,
            }),
        })
    })?;
    collect_rows(rows)
}

pub(crate) fn privacy_transform_ledger(connection: &Connection) -> Result<Vec<EvalLedgerEntry>> {
    let mut statement = connection.prepare(
        "SELECT id, started_at, privacy_transform_run_ids_json, metadata_json
         FROM llm_invocations
         WHERE privacy_transform_run_ids_json != '[]'
         ORDER BY started_at ASC, id ASC",
    )?;
    let rows = statement.query_map([], |row| {
        Ok(EvalLedgerEntry {
            ledger: "privacy_transforms".to_string(),
            id: row.get(0)?,
            occurred_at: Some(row.get(1)?),
            entry_type: "privacy_transform_runs".to_string(),
            payload: json!({
                "transformRunIds": parse_json_value(row.get::<_, String>(2)?),
                "metadata": parse_json_value(row.get::<_, String>(3)?),
            }),
        })
    })?;
    collect_rows(rows)
}

pub(crate) fn token_ledger(connection: &Connection) -> Result<Vec<EvalLedgerEntry>> {
    let mut statement = connection.prepare(
        "SELECT id, created_at, usage_kind, invocation_id, conversation_id, capability_id,
                provider_id, model_id, token_count, estimated_cost_micros,
                pricing_snapshot_json, metadata_json
         FROM llm_token_ledger_entries
         ORDER BY created_at ASC, id ASC",
    )?;
    let rows = statement.query_map([], |row| {
        Ok(EvalLedgerEntry {
            ledger: "token_ledger".to_string(),
            id: row.get(0)?,
            occurred_at: Some(row.get(1)?),
            entry_type: row.get(2)?,
            payload: json!({
                "invocationId": row.get::<_, String>(3)?,
                "conversationId": row.get::<_, String>(4)?,
                "capabilityId": row.get::<_, String>(5)?,
                "providerId": row.get::<_, String>(6)?,
                "modelId": row.get::<_, String>(7)?,
                "tokenCount": row.get::<_, i64>(8)?,
                "estimatedCostMicros": row.get::<_, i64>(9)?,
                "pricingSnapshot": parse_json_value(row.get::<_, String>(10)?),
                "metadata": parse_json_value(row.get::<_, String>(11)?),
            }),
        })
    })?;
    collect_rows(rows)
}

pub(crate) fn analysis_candidate_ledger(connection: &Connection) -> Result<Vec<EvalLedgerEntry>> {
    let mut statement = connection.prepare(
        "SELECT id, created_at, candidate_kind, job_id, conversation_id, segment_id,
                candidate_state, confidence, evidence_refs_json, provenance_json,
                prompt_slot_ids_json, llm_run_id, content_hash, summary_text, body_json,
                visibility
         FROM conversation_analysis_candidates
         ORDER BY created_at ASC, id ASC",
    )?;
    let rows = statement.query_map([], |row| {
        Ok(EvalLedgerEntry {
            ledger: "analysis_candidates".to_string(),
            id: row.get(0)?,
            occurred_at: Some(row.get(1)?),
            entry_type: row.get(2)?,
            payload: json!({
                "jobId": row.get::<_, String>(3)?,
                "conversationId": row.get::<_, String>(4)?,
                "segmentId": row.get::<_, Option<String>>(5)?,
                "candidateState": row.get::<_, String>(6)?,
                "confidence": row.get::<_, f64>(7)?,
                "evidenceRefs": parse_json_value(row.get::<_, String>(8)?),
                "provenance": parse_json_value(row.get::<_, String>(9)?),
                "promptSlotIds": parse_json_value(row.get::<_, String>(10)?),
                "llmRunId": row.get::<_, Option<String>>(11)?,
                "contentHash": row.get::<_, String>(12)?,
                "summary": row.get::<_, String>(13)?,
                "body": parse_json_value(row.get::<_, String>(14)?),
                "visibility": row.get::<_, String>(15)?,
            }),
        })
    })?;
    collect_rows(rows)
}

pub(crate) fn handoff_ledger(connection: &Connection) -> Result<Vec<EvalLedgerEntry>> {
    let mut statement = connection.prepare(
        "SELECT id, created_at, status, conversation_id, segment_id, connection_id,
                requested_by_actor_id, assigned_to_actor_id, reason, urgency,
                required_capability_id, evidence_summary, allowed_context_json,
                policy_decision_id, receipt_json
         FROM conversation_handoffs
         ORDER BY created_at ASC, id ASC",
    )?;
    let rows = statement.query_map([], |row| {
        Ok(EvalLedgerEntry {
            ledger: "handoffs".to_string(),
            id: row.get(0)?,
            occurred_at: Some(row.get(1)?),
            entry_type: row.get(2)?,
            payload: json!({
                "conversationId": row.get::<_, String>(3)?,
                "segmentId": row.get::<_, Option<String>>(4)?,
                "connectionId": row.get::<_, Option<String>>(5)?,
                "requestedByActorId": row.get::<_, Option<String>>(6)?,
                "assignedToActorId": row.get::<_, Option<String>>(7)?,
                "reason": row.get::<_, String>(8)?,
                "urgency": row.get::<_, String>(9)?,
                "requiredCapabilityId": row.get::<_, String>(10)?,
                "evidenceSummary": row.get::<_, String>(11)?,
                "allowedContext": parse_json_value(row.get::<_, String>(12)?),
                "policyDecisionId": row.get::<_, Option<String>>(13)?,
                "receipt": parse_json_value(row.get::<_, String>(14)?),
            }),
        })
    })?;
    collect_rows(rows)
}

pub(crate) fn artifact_ledger(connection: &Connection) -> Result<Vec<EvalLedgerEntry>> {
    let mut statement = connection.prepare(
        "SELECT id, created_at, artifact_kind, title, status, visibility_ceiling,
                summary, source_kind, source_id, evidence_refs_json, provenance_json,
                content_hash, storage_uri, health_status, created_by_job_id
         FROM artifacts
         ORDER BY created_at ASC, id ASC",
    )?;
    let rows = statement.query_map([], |row| {
        Ok(EvalLedgerEntry {
            ledger: "artifacts".to_string(),
            id: row.get(0)?,
            occurred_at: Some(row.get(1)?),
            entry_type: row.get(2)?,
            payload: json!({
                "title": row.get::<_, String>(3)?,
                "status": row.get::<_, String>(4)?,
                "visibilityCeiling": row.get::<_, String>(5)?,
                "summary": row.get::<_, String>(6)?,
                "sourceKind": row.get::<_, Option<String>>(7)?,
                "sourceId": row.get::<_, Option<String>>(8)?,
                "evidenceRefs": parse_json_value(row.get::<_, String>(9)?),
                "provenance": parse_json_value(row.get::<_, String>(10)?),
                "contentHash": row.get::<_, String>(11)?,
                "storageUri": row.get::<_, Option<String>>(12)?,
                "healthStatus": row.get::<_, Option<String>>(13)?,
                "createdByJobId": row.get::<_, Option<String>>(14)?,
            }),
        })
    })?;
    collect_rows(rows)
}

pub(crate) fn surface_brief_ledger(connection: &Connection) -> Result<Vec<EvalLedgerEntry>> {
    let mut statement = connection.prepare(
        "SELECT id, created_at, surface_kind, status, subject_kind, subject_id,
                artifact_id, title, brief_markdown, evidence_refs_json, limitations_json,
                created_by_job_id, generated_at, completed_at, superseded_at,
                failure_message
         FROM surface_briefs
         ORDER BY created_at ASC, id ASC",
    )?;
    let rows = statement.query_map([], |row| {
        Ok(EvalLedgerEntry {
            ledger: "surface_briefs".to_string(),
            id: row.get(0)?,
            occurred_at: Some(row.get(1)?),
            entry_type: row.get(2)?,
            payload: json!({
                "status": row.get::<_, String>(3)?,
                "subjectKind": row.get::<_, Option<String>>(4)?,
                "subjectId": row.get::<_, Option<String>>(5)?,
                "artifactId": row.get::<_, Option<String>>(6)?,
                "title": row.get::<_, String>(7)?,
                "brief": row.get::<_, String>(8)?,
                "evidenceRefs": parse_json_value(row.get::<_, String>(9)?),
                "limitations": parse_json_value(row.get::<_, String>(10)?),
                "createdByJobId": row.get::<_, Option<String>>(11)?,
                "generatedAt": row.get::<_, String>(12)?,
                "completedAt": row.get::<_, Option<String>>(13)?,
                "supersededAt": row.get::<_, Option<String>>(14)?,
                "failureMessage": row.get::<_, Option<String>>(15)?,
            }),
        })
    })?;
    collect_rows(rows)
}

pub(crate) fn feedback_ledger(connection: &Connection) -> Result<Vec<EvalLedgerEntry>> {
    let mut statement = connection.prepare(
        "SELECT id, created_at, feedback_kind, status, visibility, connection_id,
                conversation_id, segment_id, message_id, body_summary, is_starred,
                source_refs_json, evidence_refs_json, provenance_json
         FROM customer_feedback
         ORDER BY created_at ASC, id ASC",
    )?;
    let rows = statement.query_map([], |row| {
        Ok(EvalLedgerEntry {
            ledger: "customer_feedback".to_string(),
            id: row.get(0)?,
            occurred_at: Some(row.get(1)?),
            entry_type: row.get(2)?,
            payload: json!({
                "status": row.get::<_, String>(3)?,
                "visibility": row.get::<_, String>(4)?,
                "connectionId": row.get::<_, Option<String>>(5)?,
                "conversationId": row.get::<_, String>(6)?,
                "segmentId": row.get::<_, Option<String>>(7)?,
                "messageId": row.get::<_, Option<String>>(8)?,
                "summary": row.get::<_, String>(9)?,
                "isStarred": row.get::<_, i64>(10)? == 1,
                "sourceRefs": parse_json_value(row.get::<_, String>(11)?),
                "evidenceRefs": parse_json_value(row.get::<_, String>(12)?),
                "provenance": parse_json_value(row.get::<_, String>(13)?),
            }),
        })
    })?;
    collect_rows(rows)
}

pub(crate) fn review_ledger(connection: &Connection) -> Result<Vec<EvalLedgerEntry>> {
    let mut statement = connection.prepare(
        "SELECT id, created_at, status, feedback_id, connection_id, conversation_id,
                review_body, publication_visibility, consent_evidence_refs_json,
                approval_evidence_refs_json, evidence_refs_json, provenance_json,
                published_at, featured_at, retired_at
         FROM customer_reviews
         ORDER BY created_at ASC, id ASC",
    )?;
    let rows = statement.query_map([], |row| {
        Ok(EvalLedgerEntry {
            ledger: "customer_reviews".to_string(),
            id: row.get(0)?,
            occurred_at: Some(row.get(1)?),
            entry_type: row.get(2)?,
            payload: json!({
                "feedbackId": row.get::<_, String>(3)?,
                "connectionId": row.get::<_, Option<String>>(4)?,
                "conversationId": row.get::<_, String>(5)?,
                "reviewBody": row.get::<_, String>(6)?,
                "publicationVisibility": row.get::<_, String>(7)?,
                "consentEvidenceRefs": parse_json_value(row.get::<_, String>(8)?),
                "approvalEvidenceRefs": parse_json_value(row.get::<_, String>(9)?),
                "evidenceRefs": parse_json_value(row.get::<_, String>(10)?),
                "provenance": parse_json_value(row.get::<_, String>(11)?),
                "publishedAt": row.get::<_, Option<String>>(12)?,
                "featuredAt": row.get::<_, Option<String>>(13)?,
                "retiredAt": row.get::<_, Option<String>>(14)?,
            }),
        })
    })?;
    collect_rows(rows)
}

pub(crate) fn product_surface_ledger(connection: &Connection) -> Result<Vec<EvalLedgerEntry>> {
    let mut entries = business_fact_ledger(connection)?;
    entries.extend(offer_ledger(connection)?);
    entries.extend(business_outcome_ledger(connection)?);
    entries.sort_by(|left, right| {
        left.occurred_at
            .cmp(&right.occurred_at)
            .then_with(|| left.ledger.cmp(&right.ledger))
            .then_with(|| left.id.cmp(&right.id))
    });
    Ok(entries)
}

pub(crate) fn business_fact_ledger(connection: &Connection) -> Result<Vec<EvalLedgerEntry>> {
    let mut statement = connection.prepare(
        "SELECT id, created_at, fact_key, subject_type, subject_id, value_json,
                source_kind, source_label, source_uri, provenance_json, visibility,
                publication_state, published_at, archived_at
         FROM business_facts
         ORDER BY created_at ASC, id ASC",
    )?;
    let rows = statement.query_map([], |row| {
        Ok(EvalLedgerEntry {
            ledger: "business_facts".to_string(),
            id: row.get(0)?,
            occurred_at: Some(row.get(1)?),
            entry_type: row.get(2)?,
            payload: json!({
                "subjectType": row.get::<_, String>(3)?,
                "subjectId": row.get::<_, String>(4)?,
                "value": parse_json_value(row.get::<_, String>(5)?),
                "sourceKind": row.get::<_, String>(6)?,
                "sourceLabel": row.get::<_, Option<String>>(7)?,
                "sourceUri": row.get::<_, Option<String>>(8)?,
                "provenance": parse_json_value(row.get::<_, String>(9)?),
                "visibility": row.get::<_, String>(10)?,
                "publicationState": row.get::<_, String>(11)?,
                "publishedAt": row.get::<_, Option<String>>(12)?,
                "archivedAt": row.get::<_, Option<String>>(13)?,
            }),
        })
    })?;
    collect_rows(rows)
}

pub(crate) fn offer_ledger(connection: &Connection) -> Result<Vec<EvalLedgerEntry>> {
    let mut statement = connection.prepare(
        "SELECT id, created_at, slug, title, summary, status, visibility,
                publication_state, trial_days, source_kind, source_ref, terms_json,
                metadata_json, published_at, archived_at
         FROM offers
         ORDER BY created_at ASC, id ASC",
    )?;
    let rows = statement.query_map([], |row| {
        Ok(EvalLedgerEntry {
            ledger: "offers".to_string(),
            id: row.get(0)?,
            occurred_at: Some(row.get(1)?),
            entry_type: row.get(2)?,
            payload: json!({
                "title": row.get::<_, String>(3)?,
                "summary": row.get::<_, String>(4)?,
                "status": row.get::<_, String>(5)?,
                "visibility": row.get::<_, String>(6)?,
                "publicationState": row.get::<_, String>(7)?,
                "trialDays": row.get::<_, i64>(8)?,
                "sourceKind": row.get::<_, String>(9)?,
                "sourceRef": row.get::<_, Option<String>>(10)?,
                "terms": parse_json_value(row.get::<_, String>(11)?),
                "metadata": parse_json_value(row.get::<_, String>(12)?),
                "publishedAt": row.get::<_, Option<String>>(13)?,
                "archivedAt": row.get::<_, Option<String>>(14)?,
            }),
        })
    })?;
    collect_rows(rows)
}

pub(crate) fn business_outcome_ledger(connection: &Connection) -> Result<Vec<EvalLedgerEntry>> {
    let mut statement = connection.prepare(
        "SELECT id, created_at, outcome_kind, status, connection_id, conversation_id,
                segment_id, offer_id, ask_id, artifact_id, entry_point_id,
                visitor_session_id, referral_id, evidence_refs_json, provenance_json
         FROM business_outcomes
         ORDER BY created_at ASC, id ASC",
    )?;
    let rows = statement.query_map([], |row| {
        Ok(EvalLedgerEntry {
            ledger: "business_outcomes".to_string(),
            id: row.get(0)?,
            occurred_at: Some(row.get(1)?),
            entry_type: row.get(2)?,
            payload: json!({
                "status": row.get::<_, String>(3)?,
                "connectionId": row.get::<_, Option<String>>(4)?,
                "conversationId": row.get::<_, Option<String>>(5)?,
                "segmentId": row.get::<_, Option<String>>(6)?,
                "offerId": row.get::<_, Option<String>>(7)?,
                "askId": row.get::<_, Option<String>>(8)?,
                "artifactId": row.get::<_, Option<String>>(9)?,
                "entryPointId": row.get::<_, Option<String>>(10)?,
                "visitorSessionId": row.get::<_, Option<String>>(11)?,
                "referralId": row.get::<_, Option<String>>(12)?,
                "evidenceRefs": parse_json_value(row.get::<_, String>(13)?),
                "provenance": parse_json_value(row.get::<_, String>(14)?),
            }),
        })
    })?;
    collect_rows(rows)
}

pub(crate) fn collect_rows(
    rows: rusqlite::MappedRows<
        '_,
        impl FnMut(&rusqlite::Row<'_>) -> rusqlite::Result<EvalLedgerEntry>,
    >,
) -> Result<Vec<EvalLedgerEntry>> {
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(Into::into)
}

pub(crate) fn parse_json_value(raw: String) -> Value {
    serde_json::from_str(&raw)
        .unwrap_or_else(|_| json!({ "unparseableHash": stable_text_hash(&raw) }))
}

pub(crate) fn write_json<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    let encoded = serde_json::to_string_pretty(value)?;
    fs::write(path, encoded).with_context(|| format!("write JSON artifact {}", path.display()))?;
    Ok(())
}

pub(crate) fn redact_serializable<T: Serialize + for<'de> Deserialize<'de>>(
    value: &mut T,
    private_terms: &[String],
    summary: &mut EvalRedactionSummary,
) -> Result<()> {
    let mut json_value = serde_json::to_value(&*value)?;
    redact_value(&mut json_value, private_terms, summary);
    *value = serde_json::from_value(json_value)?;
    summary.redaction_applied = summary.redacted_value_count > 0;
    Ok(())
}

pub(crate) fn redact_value(
    value: &mut Value,
    private_terms: &[String],
    summary: &mut EvalRedactionSummary,
) {
    match value {
        Value::String(text) => {
            let redacted = redact_text(text, private_terms, summary);
            *text = redacted;
        }
        Value::Array(items) => {
            for item in items {
                redact_value(item, private_terms, summary);
            }
        }
        Value::Object(map) => {
            for item in map.values_mut() {
                redact_value(item, private_terms, summary);
            }
        }
        _ => {}
    }
}

pub(crate) fn redact_text(
    text: &str,
    private_terms: &[String],
    summary: &mut EvalRedactionSummary,
) -> String {
    let redacted = redaction::redact_eval_text(text, private_terms);
    summary.redacted_value_count += redacted.redacted_count;
    redacted.text
}

pub(crate) fn stable_json_hash(value: &Value) -> Result<String> {
    let encoded = serde_json::to_string(value)?;
    let mut hasher = Sha256::new();
    hasher.update(encoded.as_bytes());
    Ok(format!("{:x}", hasher.finalize()))
}

pub(crate) fn stable_text_hash(value: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(value.as_bytes());
    format!("{:x}", hasher.finalize())
}

pub(crate) fn ensure_identifier(value: &str) -> Result<()> {
    ensure!(
        value
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || character == '_'),
        "SQL identifier contains unsupported characters"
    );
    Ok(())
}

pub fn isolated_eval_connection() -> Result<Connection> {
    let connection = Connection::open_in_memory()?;
    crate::schema::init_schema(&connection)?;
    crate::capabilities::seed_builtin_capabilities(&connection)?;
    crate::templates::seed_builtin_templates(&connection)?;
    connection.execute(
        "INSERT INTO tracked_entry_points (
            id, slug, label, status, source_kind, source_label, destination_surface,
            destination_id, public_path, qr_payload_json, attribution_json, metadata_json,
            created_at, updated_at
         ) VALUES (
            'entry_point_eval_1', 'eval-chat', 'Eval Chat', 'active', 'eval',
            'Eval Harness', 'chat', NULL, '/chat', '{}', '{}', '{}', 'now', 'now'
         )",
        [],
    )?;
    connection.execute(
        "INSERT INTO visitor_sessions (
            id, entry_point_id, entry_point_slug, status, destination_surface,
            destination_id, attribution_json, user_agent_hash, created_at, updated_at,
            last_seen_at
         ) VALUES (
            'visitor_session_eval_1', 'entry_point_eval_1', 'eval-chat', 'active',
            'chat', NULL, '{}', 'eval-user-agent-hash', 'now', 'now', 'now'
         )",
        [],
    )?;
    connection.execute(
        "INSERT INTO actors (id, actor_kind, display_name, status, metadata_json, created_at, updated_at)
         VALUES ('actor_staff_eval_1', 'staff', 'Eval Staff', 'active', '{}', 'now', 'now')",
        [],
    )?;
    for (actor_id, actor_kind, display_name) in [
        ("actor_client_eval_1", "client", "Eval Client"),
        ("actor_affiliate_eval_1", "affiliate", "Eval Affiliate"),
        ("actor_manager_eval_1", "manager", "Eval Manager"),
        ("actor_owner_eval_1", "owner", "Eval Owner"),
    ] {
        connection.execute(
            "INSERT INTO actors (id, actor_kind, display_name, status, metadata_json, created_at, updated_at)
             VALUES (?1, ?2, ?3, 'active', '{}', 'now', 'now')",
            rusqlite::params![actor_id, actor_kind, display_name],
        )?;
    }
    connection.execute(
        "INSERT INTO connections (
            id, connection_type, display_name, status, identity_json, scope_json, metadata_json, created_at, updated_at
         ) VALUES (
            'connection_eval_1', 'client', 'Eval Client', 'active', '{}', '{}', '{}', 'now', 'now'
         )",
        [],
    )?;
    Ok(connection)
}
