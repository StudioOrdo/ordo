use anyhow::{ensure, Result};
use chrono::{DateTime, Duration, Utc};
use rusqlite::{Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

pub const EVAL_HARNESS_SCHEMA_VERSION: &str = "ordo.eval_harness.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvalActorRole {
    AnonymousVisitor,
    ClientMember,
    Affiliate,
    Staff,
    ManagerAdmin,
    OwnerSystemAdmin,
    OrdoAgent,
    LlmToolProviderBoundary,
}

impl EvalActorRole {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::AnonymousVisitor => "anonymous_visitor",
            Self::ClientMember => "client_member",
            Self::Affiliate => "affiliate",
            Self::Staff => "staff",
            Self::ManagerAdmin => "manager_admin",
            Self::OwnerSystemAdmin => "owner_system_admin",
            Self::OrdoAgent => "ordo_agent",
            Self::LlmToolProviderBoundary => "llm_tool_provider_boundary",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvalEvidenceChannel {
    SqliteRows,
    ConversationEvents,
    RealtimeReplay,
    PolicyDecisions,
    PromptSlotAccounting,
    PrivacyTransforms,
    TokenLedger,
    AnalysisCandidates,
    HandoffState,
    ArtifactRecords,
    SurfaceBriefRecords,
}

impl EvalEvidenceChannel {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::SqliteRows => "sqlite_rows",
            Self::ConversationEvents => "conversation_events",
            Self::RealtimeReplay => "realtime_replay",
            Self::PolicyDecisions => "policy_decisions",
            Self::PromptSlotAccounting => "prompt_slot_accounting",
            Self::PrivacyTransforms => "privacy_transforms",
            Self::TokenLedger => "token_ledger",
            Self::AnalysisCandidates => "analysis_candidates",
            Self::HandoffState => "handoff_state",
            Self::ArtifactRecords => "artifact_records",
            Self::SurfaceBriefRecords => "surface_brief_records",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EvalStep {
    pub id: String,
    pub actor_role: EvalActorRole,
    pub action: String,
    pub expected_evidence: Vec<EvalEvidenceChannel>,
    pub metadata: Value,
}

impl EvalStep {
    pub fn new(
        id: impl Into<String>,
        actor_role: EvalActorRole,
        action: impl Into<String>,
        expected_evidence: Vec<EvalEvidenceChannel>,
    ) -> Result<Self> {
        let id = id.into();
        let action = action.into();
        require_text("eval step id", &id)?;
        require_text("eval step action", &action)?;
        Ok(Self {
            id,
            actor_role,
            action,
            expected_evidence,
            metadata: json!({}),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EvalCase {
    pub id: String,
    pub title: String,
    pub fixture_hash: String,
    pub actor_roles: Vec<EvalActorRole>,
    pub steps: Vec<EvalStep>,
    pub expected_assertions: Vec<EvalAssertion>,
}

impl EvalCase {
    pub fn new(
        id: impl Into<String>,
        title: impl Into<String>,
        fixture: &Value,
        actor_roles: Vec<EvalActorRole>,
        steps: Vec<EvalStep>,
        expected_assertions: Vec<EvalAssertion>,
    ) -> Result<Self> {
        let id = id.into();
        let title = title.into();
        require_text("eval case id", &id)?;
        require_text("eval case title", &title)?;
        ensure!(
            !actor_roles.is_empty(),
            "eval case must declare at least one actor role"
        );
        ensure!(
            !steps.is_empty(),
            "eval case must declare at least one step"
        );
        Ok(Self {
            id,
            title,
            fixture_hash: stable_json_hash(fixture)?,
            actor_roles,
            steps,
            expected_assertions,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EvalAssertion {
    pub id: String,
    pub channel: EvalEvidenceChannel,
    pub minimum_after_count: i64,
}

impl EvalAssertion {
    pub fn minimum_count(
        id: impl Into<String>,
        channel: EvalEvidenceChannel,
        minimum_after_count: i64,
    ) -> Result<Self> {
        let id = id.into();
        require_text("eval assertion id", &id)?;
        ensure!(
            minimum_after_count >= 0,
            "eval assertion minimum count cannot be negative"
        );
        Ok(Self {
            id,
            channel,
            minimum_after_count,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EvalAssertionResult {
    pub assertion_id: String,
    pub channel: EvalEvidenceChannel,
    pub expected_minimum: i64,
    pub actual_count: i64,
    pub passed: bool,
    pub note: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EvalEvidenceCount {
    pub channel: EvalEvidenceChannel,
    pub count: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EvalEvidenceSnapshot {
    pub captured_at: String,
    pub channels: Vec<EvalEvidenceCount>,
    pub conversation_event_max_sequence: Option<i64>,
    pub realtime_replay_max_cursor: Option<i64>,
}

impl EvalEvidenceSnapshot {
    pub fn count_for(&self, channel: EvalEvidenceChannel) -> i64 {
        self.channels
            .iter()
            .find(|entry| entry.channel == channel)
            .map(|entry| entry.count)
            .unwrap_or(0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EvalScorecardSummary {
    pub schema_version: String,
    pub case_id: String,
    pub title: String,
    pub fixture_hash: String,
    pub actor_roles: Vec<EvalActorRole>,
    pub step_count: usize,
    pub provider_mode: String,
    pub network_enabled: bool,
    pub evidence_before: EvalEvidenceSnapshot,
    pub evidence_after: EvalEvidenceSnapshot,
    pub assertion_results: Vec<EvalAssertionResult>,
    pub passed: bool,
    pub artifact_path: Option<String>,
    pub generated_at: String,
}

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
            provider_mode: "deterministic_only".to_string(),
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
        ],
        conversation_event_max_sequence: max_i64(connection, "conversation_events", "sequence")?,
        realtime_replay_max_cursor: max_i64(connection, "realtime_events", "cursor")?,
    })
}

fn total_evidence_rows(connection: &Connection) -> Result<i64> {
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
    ];
    tables
        .iter()
        .try_fold(0, |sum, table| Ok(sum + table_count(connection, table)?))
}

fn privacy_transform_count(connection: &Connection) -> Result<i64> {
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

fn table_count(connection: &Connection, table: &str) -> Result<i64> {
    ensure_identifier(table)?;
    let sql = format!("SELECT COUNT(*) FROM {table}");
    let count = connection.query_row(&sql, [], |row| row.get(0))?;
    Ok(count)
}

fn max_i64(connection: &Connection, table: &str, column: &str) -> Result<Option<i64>> {
    ensure_identifier(table)?;
    ensure_identifier(column)?;
    let sql = format!("SELECT MAX({column}) FROM {table}");
    let value = connection
        .query_row(&sql, [], |row| row.get(0))
        .optional()?;
    Ok(value.flatten())
}

fn stable_json_hash(value: &Value) -> Result<String> {
    let encoded = serde_json::to_string(value)?;
    let mut hasher = Sha256::new();
    hasher.update(encoded.as_bytes());
    Ok(format!("{:x}", hasher.finalize()))
}

fn require_text(label: &str, value: &str) -> Result<()> {
    ensure!(!value.trim().is_empty(), "{label} is required");
    Ok(())
}

fn ensure_identifier(value: &str) -> Result<()> {
    ensure!(
        value
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || character == '_'),
        "SQL identifier contains unsupported characters"
    );
    Ok(())
}

#[cfg(test)]
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
    Ok(connection)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::conversations::{
        create_conversation_message, create_conversation_participant,
        find_or_create_canonical_conversation, CanonicalConversationRequest,
        ConversationMessageCreateRequest, ConversationParticipantCreateRequest,
    };

    fn eval_case() -> EvalCase {
        EvalCase::new(
            "relationship_message_foundation",
            "Relationship message foundation",
            &json!({
                "fixture": "relationship_message_foundation",
                "version": 1,
            }),
            vec![
                EvalActorRole::AnonymousVisitor,
                EvalActorRole::Staff,
                EvalActorRole::LlmToolProviderBoundary,
            ],
            vec![
                EvalStep::new(
                    "create_canonical_conversation",
                    EvalActorRole::AnonymousVisitor,
                    "create_or_find_conversation",
                    vec![
                        EvalEvidenceChannel::SqliteRows,
                        EvalEvidenceChannel::ConversationEvents,
                    ],
                )
                .unwrap(),
                EvalStep::new(
                    "submit_message",
                    EvalActorRole::AnonymousVisitor,
                    "message.submit",
                    vec![
                        EvalEvidenceChannel::SqliteRows,
                        EvalEvidenceChannel::ConversationEvents,
                        EvalEvidenceChannel::RealtimeReplay,
                    ],
                )
                .unwrap(),
            ],
            vec![
                EvalAssertion::minimum_count(
                    "conversation_events_exist",
                    EvalEvidenceChannel::ConversationEvents,
                    2,
                )
                .unwrap(),
                EvalAssertion::minimum_count(
                    "realtime_replay_exists",
                    EvalEvidenceChannel::RealtimeReplay,
                    2,
                )
                .unwrap(),
            ],
        )
        .unwrap()
    }

    fn run_relationship_step(connection: &Connection, step: &EvalStep) -> Result<()> {
        match step.id.as_str() {
            "create_canonical_conversation" => {
                find_or_create_canonical_conversation(
                    connection,
                    &CanonicalConversationRequest {
                        surface: "chat".to_string(),
                        subject_kind: "visitor_session".to_string(),
                        subject_id: "visitor_session_eval_1".to_string(),
                        connection_id: None,
                        visitor_session_id: Some("visitor_session_eval_1".to_string()),
                        created_by_actor_id: None,
                    },
                )?;
            }
            "submit_message" => {
                let conversation = find_or_create_canonical_conversation(
                    connection,
                    &CanonicalConversationRequest {
                        surface: "chat".to_string(),
                        subject_kind: "visitor_session".to_string(),
                        subject_id: "visitor_session_eval_1".to_string(),
                        connection_id: None,
                        visitor_session_id: Some("visitor_session_eval_1".to_string()),
                        created_by_actor_id: None,
                    },
                )?;
                let participant = create_conversation_participant(
                    connection,
                    &ConversationParticipantCreateRequest {
                        conversation_id: conversation.id.clone(),
                        participant_kind: "visitor".to_string(),
                        actor_id: None,
                        connection_id: None,
                        visitor_session_id: Some("visitor_session_eval_1".to_string()),
                        display_name: "Visitor".to_string(),
                        role: "client".to_string(),
                    },
                )?;
                create_conversation_message(
                    connection,
                    &ConversationMessageCreateRequest {
                        conversation_id: conversation.id,
                        segment_id: None,
                        participant_id: participant.id,
                        message_kind: "user".to_string(),
                        body_markdown: "I need help choosing a package.".to_string(),
                        visibility: "participants".to_string(),
                        client_message_id: "eval-client-message-1".to_string(),
                        reply_to_message_id: None,
                        undo_expires_at: None,
                    },
                )?;
            }
            other => anyhow::bail!("unsupported eval step in test fixture: {other}"),
        }
        Ok(())
    }

    #[test]
    fn isolated_eval_store_initializes_current_schema() {
        let connection = isolated_eval_connection().unwrap();
        let table_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'conversation_events'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(table_count, 1);
    }

    #[test]
    fn harness_runs_steps_in_stable_order_and_records_scorecard() {
        let connection = isolated_eval_connection().unwrap();
        let mut harness = DeterministicEvalHarness::new(DeterministicEvalClock::fixed())
            .with_artifact_path("tests/evals/backend/scorecards/relationship_message.json");

        let scorecard = harness
            .run_case(&connection, &eval_case(), run_relationship_step)
            .unwrap();

        assert!(scorecard.passed);
        assert_eq!(scorecard.schema_version, EVAL_HARNESS_SCHEMA_VERSION);
        assert_eq!(scorecard.step_count, 2);
        assert_eq!(scorecard.provider_mode, "deterministic_only");
        assert!(!scorecard.network_enabled);
        assert_eq!(
            scorecard.actor_roles,
            vec![
                EvalActorRole::AnonymousVisitor,
                EvalActorRole::Staff,
                EvalActorRole::LlmToolProviderBoundary,
            ]
        );
        assert_eq!(
            scorecard
                .evidence_after
                .count_for(EvalEvidenceChannel::PromptSlotAccounting),
            0
        );
        assert_eq!(
            scorecard
                .evidence_after
                .count_for(EvalEvidenceChannel::TokenLedger),
            0
        );
        assert!(scorecard
            .evidence_after
            .conversation_event_max_sequence
            .is_some());
        assert!(scorecard
            .evidence_after
            .realtime_replay_max_cursor
            .is_some());
    }

    #[test]
    fn repeated_harness_runs_are_stable_except_durable_ids() {
        let first_connection = isolated_eval_connection().unwrap();
        let second_connection = isolated_eval_connection().unwrap();
        let case = eval_case();

        let mut first = DeterministicEvalHarness::new(DeterministicEvalClock::fixed());
        let mut second = DeterministicEvalHarness::new(DeterministicEvalClock::fixed());
        let first_scorecard = first
            .run_case(&first_connection, &case, run_relationship_step)
            .unwrap();
        let second_scorecard = second
            .run_case(&second_connection, &case, run_relationship_step)
            .unwrap();

        assert_eq!(first_scorecard.fixture_hash, second_scorecard.fixture_hash);
        assert_eq!(first_scorecard.actor_roles, second_scorecard.actor_roles);
        assert_eq!(first_scorecard.step_count, second_scorecard.step_count);
        assert_eq!(
            first_scorecard.provider_mode,
            second_scorecard.provider_mode
        );
        assert_eq!(
            first_scorecard.network_enabled,
            second_scorecard.network_enabled
        );
        assert_eq!(
            first_scorecard.assertion_results,
            second_scorecard.assertion_results
        );
        assert_eq!(
            first_scorecard
                .evidence_after
                .count_for(EvalEvidenceChannel::ConversationEvents),
            second_scorecard
                .evidence_after
                .count_for(EvalEvidenceChannel::ConversationEvents)
        );
        assert_eq!(
            first_scorecard
                .evidence_after
                .count_for(EvalEvidenceChannel::RealtimeReplay),
            second_scorecard
                .evidence_after
                .count_for(EvalEvidenceChannel::RealtimeReplay)
        );
    }
}
