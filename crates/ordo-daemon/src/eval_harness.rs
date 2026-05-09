use anyhow::{ensure, Context, Result};
use chrono::{DateTime, Duration, Utc};
use rusqlite::{Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};

use crate::conversations::{
    conversation_queue, create_conversation_handoff, create_conversation_message,
    create_conversation_participant, find_or_create_canonical_conversation,
    may_agent_post_publicly, record_staff_activity_sets_human_led, CanonicalConversationRequest,
    ConversationHandoffCreateRequest, ConversationMessageCreateRequest, ConversationMode,
    ConversationParticipantCreateRequest, ConversationRole, PublicPostContext, QueueScope,
};
use crate::feedback::{
    capture_feedback, create_review_candidate, list_private_feedback, list_public_reviews,
    propose_feedback_tag, set_feedback_starred, transition_review, CustomerFeedbackInput,
    FeedbackTagInput, ReviewCandidateInput, ReviewStatus,
};
use crate::llm_gateway::{DeterministicLlmProvider, LlmGateway, LlmGatewayRequest, PromptSlot};
use crate::policy::{
    authorize_protected_daemon_action, authorize_resource_access, record_policy_decision,
    ActorContext, ActorKind, PolicyAction, PolicyDecisionCorrelation, ProtectedAccessEvidence,
    ResourceKind, ResourceRef,
};

pub const EVAL_HARNESS_SCHEMA_VERSION: &str = "ordo.eval_harness.v1";
pub const EVAL_ARTIFACT_PACKET_SCHEMA_VERSION: &str = "ordo.eval_artifact_packet.v1";

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
    FeedbackReviewRecords,
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
            Self::FeedbackReviewRecords => "feedback_review_records",
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvalFindingCategory {
    SchemaGap,
    EventGap,
    PolicyGap,
    PrivacyGap,
    PromptGap,
    HandoffGap,
    AnalysisGap,
    AccountingGap,
    UxContractGap,
    ProviderGap,
    TestFixtureGap,
}

impl EvalFindingCategory {
    pub fn all() -> Vec<Self> {
        vec![
            Self::SchemaGap,
            Self::EventGap,
            Self::PolicyGap,
            Self::PrivacyGap,
            Self::PromptGap,
            Self::HandoffGap,
            Self::AnalysisGap,
            Self::AccountingGap,
            Self::UxContractGap,
            Self::ProviderGap,
            Self::TestFixtureGap,
        ]
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EvalLedgerEntry {
    pub ledger: String,
    pub id: String,
    pub occurred_at: Option<String>,
    pub entry_type: String,
    pub payload: Value,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EvalRedactionSummary {
    pub redaction_applied: bool,
    pub redacted_value_count: usize,
    pub private_term_count: usize,
    pub detectors: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EvalArtifactReviewPlaceholder {
    pub status: String,
    pub finding_categories: Vec<EvalFindingCategory>,
    pub note: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EvalArtifactPacket {
    pub schema_version: String,
    pub case_id: String,
    pub case_title: String,
    pub fixture_hash: String,
    pub actor_roles: Vec<EvalActorRole>,
    pub steps: Vec<EvalStep>,
    pub scorecard: EvalScorecardSummary,
    pub transcript: Vec<EvalLedgerEntry>,
    pub timeline: Vec<EvalLedgerEntry>,
    pub conversation_event_ledger: Vec<EvalLedgerEntry>,
    pub realtime_replay_ledger: Vec<EvalLedgerEntry>,
    pub policy_decision_ledger: Vec<EvalLedgerEntry>,
    pub prompt_slot_ledger: Vec<EvalLedgerEntry>,
    pub privacy_transform_ledger: Vec<EvalLedgerEntry>,
    pub token_ledger: Vec<EvalLedgerEntry>,
    pub analysis_candidate_ledger: Vec<EvalLedgerEntry>,
    pub handoff_ledger: Vec<EvalLedgerEntry>,
    pub artifact_ledger: Vec<EvalLedgerEntry>,
    pub surface_brief_ledger: Vec<EvalLedgerEntry>,
    pub feedback_ledger: Vec<EvalLedgerEntry>,
    pub review_ledger: Vec<EvalLedgerEntry>,
    pub redaction_summary: EvalRedactionSummary,
    pub artifact_review: EvalArtifactReviewPlaceholder,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EvalArtifactManifest {
    pub schema_version: String,
    pub run_id: String,
    pub case_ids: Vec<String>,
    pub validation_status: String,
    pub source_commit: String,
    pub actor_roles: Vec<EvalActorRole>,
    pub packet_path: String,
    pub scorecard_path: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvalArtifactPaths {
    pub packet_path: PathBuf,
    pub scorecard_path: PathBuf,
    pub manifest_path: PathBuf,
}

#[derive(Debug, Clone)]
pub struct EvalArtifactWriter {
    output_dir: PathBuf,
    source_commit: String,
    private_terms: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct EvalWorkflowRun {
    pub case: EvalCase,
    pub scorecard: EvalScorecardSummary,
    pub artifact_paths: EvalArtifactPaths,
}

impl EvalArtifactWriter {
    pub fn new(output_dir: impl Into<PathBuf>, source_commit: impl Into<String>) -> Self {
        Self {
            output_dir: output_dir.into(),
            source_commit: source_commit.into(),
            private_terms: Vec::new(),
        }
    }

    pub fn with_private_terms(mut self, private_terms: Vec<String>) -> Self {
        self.private_terms = private_terms;
        self
    }

    pub fn build_packet(
        &self,
        connection: &Connection,
        case: &EvalCase,
        scorecard: &EvalScorecardSummary,
    ) -> Result<EvalArtifactPacket> {
        let mut packet = EvalArtifactPacket {
            schema_version: EVAL_ARTIFACT_PACKET_SCHEMA_VERSION.to_string(),
            case_id: case.id.clone(),
            case_title: case.title.clone(),
            fixture_hash: case.fixture_hash.clone(),
            actor_roles: case.actor_roles.clone(),
            steps: case.steps.clone(),
            scorecard: scorecard.clone(),
            transcript: transcript_ledger(connection)?,
            timeline: timeline_ledger(connection)?,
            conversation_event_ledger: conversation_event_ledger(connection)?,
            realtime_replay_ledger: realtime_replay_ledger(connection)?,
            policy_decision_ledger: policy_decision_ledger(connection)?,
            prompt_slot_ledger: prompt_slot_ledger(connection)?,
            privacy_transform_ledger: privacy_transform_ledger(connection)?,
            token_ledger: token_ledger(connection)?,
            analysis_candidate_ledger: analysis_candidate_ledger(connection)?,
            handoff_ledger: handoff_ledger(connection)?,
            artifact_ledger: artifact_ledger(connection)?,
            surface_brief_ledger: surface_brief_ledger(connection)?,
            feedback_ledger: feedback_ledger(connection)?,
            review_ledger: review_ledger(connection)?,
            redaction_summary: EvalRedactionSummary {
                redaction_applied: false,
                redacted_value_count: 0,
                private_term_count: self
                    .private_terms
                    .iter()
                    .filter(|term| !term.trim().is_empty())
                    .count(),
                detectors: vec![
                    "email".to_string(),
                    "phone".to_string(),
                    "bearer_token".to_string(),
                    "api_key".to_string(),
                    "private_term".to_string(),
                ],
            },
            artifact_review: EvalArtifactReviewPlaceholder {
                status: "not_run".to_string(),
                finding_categories: EvalFindingCategory::all(),
                note: "Artifact review classification is implemented by #140.".to_string(),
            },
        };
        let mut redaction_summary = EvalRedactionSummary {
            redaction_applied: false,
            redacted_value_count: 0,
            private_term_count: self
                .private_terms
                .iter()
                .filter(|term| !term.trim().is_empty())
                .count(),
            detectors: packet.redaction_summary.detectors.clone(),
        };
        redact_serializable(&mut packet, &self.private_terms, &mut redaction_summary)?;
        packet.redaction_summary = redaction_summary;
        Ok(packet)
    }

    pub fn write_packet(
        &self,
        connection: &Connection,
        case: &EvalCase,
        scorecard: &EvalScorecardSummary,
    ) -> Result<EvalArtifactPaths> {
        fs::create_dir_all(&self.output_dir)?;
        let packet = self.build_packet(connection, case, scorecard)?;
        let packet_path = self.output_dir.join(format!("{}-packet.json", case.id));
        let scorecard_path = self.output_dir.join(format!("{}-scorecard.json", case.id));
        let manifest_path = self.output_dir.join("manifest.json");

        write_json(&packet_path, &packet)?;
        write_json(&scorecard_path, &packet.scorecard)?;

        let manifest = EvalArtifactManifest {
            schema_version: EVAL_ARTIFACT_PACKET_SCHEMA_VERSION.to_string(),
            run_id: format!(
                "eval_run_{}",
                case.fixture_hash.chars().take(12).collect::<String>()
            ),
            case_ids: vec![case.id.clone()],
            validation_status: if scorecard.passed {
                "passed".to_string()
            } else {
                "failed".to_string()
            },
            source_commit: self.source_commit.clone(),
            actor_roles: case.actor_roles.clone(),
            packet_path: packet_path.to_string_lossy().to_string(),
            scorecard_path: scorecard_path.to_string_lossy().to_string(),
        };
        write_json(&manifest_path, &manifest)?;

        Ok(EvalArtifactPaths {
            packet_path,
            scorecard_path,
            manifest_path,
        })
    }
}

pub fn run_relationship_conversation_message_eval(
    connection: &Connection,
    output_dir: impl Into<PathBuf>,
    source_commit: impl Into<String>,
) -> Result<EvalWorkflowRun> {
    let case = relationship_conversation_message_case()?;
    let packet_path = output_dir
        .into()
        .join("relationship_conversation_message-packet.json");
    let mut harness = DeterministicEvalHarness::new(DeterministicEvalClock::fixed())
        .with_artifact_path(packet_path.to_string_lossy());
    let scorecard = harness.run_case(connection, &case, run_relationship_conversation_step)?;
    let output_dir = packet_path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    let writer = EvalArtifactWriter::new(output_dir, source_commit)
        .with_private_terms(vec!["package".to_string(), "Project Orchid".to_string()]);
    let artifact_paths = writer.write_packet(connection, &case, &scorecard)?;
    Ok(EvalWorkflowRun {
        case,
        scorecard,
        artifact_paths,
    })
}

pub fn run_privacy_gateway_roundtrip_eval(
    db_path: &Path,
    connection: &Connection,
    output_dir: impl Into<PathBuf>,
    source_commit: impl Into<String>,
) -> Result<EvalWorkflowRun> {
    let case = privacy_gateway_roundtrip_case()?;
    let packet_path = output_dir
        .into()
        .join("privacy_gateway_roundtrip-packet.json");
    let mut harness = DeterministicEvalHarness::new(DeterministicEvalClock::fixed())
        .with_artifact_path(packet_path.to_string_lossy());
    let scorecard = harness.run_case(connection, &case, |connection, step| {
        run_privacy_gateway_roundtrip_step(db_path, connection, step)
    })?;
    let output_dir = packet_path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    let writer = EvalArtifactWriter::new(output_dir, source_commit)
        .with_private_terms(vec!["Project Orchid".to_string()]);
    let artifact_paths = writer.write_packet(connection, &case, &scorecard)?;
    Ok(EvalWorkflowRun {
        case,
        scorecard,
        artifact_paths,
    })
}

pub fn run_role_lifecycle_anonymous_to_client_eval(
    connection: &Connection,
    output_dir: impl Into<PathBuf>,
    source_commit: impl Into<String>,
) -> Result<EvalWorkflowRun> {
    run_role_lifecycle_eval(
        connection,
        role_lifecycle_anonymous_to_client_case()?,
        output_dir,
        source_commit,
        run_role_lifecycle_anonymous_to_client_step,
    )
}

pub fn run_role_lifecycle_staff_manager_owner_eval(
    connection: &Connection,
    output_dir: impl Into<PathBuf>,
    source_commit: impl Into<String>,
) -> Result<EvalWorkflowRun> {
    run_role_lifecycle_eval(
        connection,
        role_lifecycle_staff_manager_owner_case()?,
        output_dir,
        source_commit,
        run_role_lifecycle_staff_manager_owner_step,
    )
}

pub fn run_role_lifecycle_agent_silence_eval(
    connection: &Connection,
    output_dir: impl Into<PathBuf>,
    source_commit: impl Into<String>,
) -> Result<EvalWorkflowRun> {
    run_role_lifecycle_eval(
        connection,
        role_lifecycle_agent_silence_case()?,
        output_dir,
        source_commit,
        run_role_lifecycle_agent_silence_step,
    )
}

pub fn run_feedback_capture_private_business_intelligence_eval(
    connection: &Connection,
    output_dir: impl Into<PathBuf>,
    source_commit: impl Into<String>,
) -> Result<EvalWorkflowRun> {
    run_role_lifecycle_eval(
        connection,
        feedback_capture_private_business_intelligence_case()?,
        output_dir,
        source_commit,
        run_feedback_capture_private_business_intelligence_step,
    )
}

pub fn run_review_candidate_consent_publication_boundary_eval(
    connection: &Connection,
    output_dir: impl Into<PathBuf>,
    source_commit: impl Into<String>,
) -> Result<EvalWorkflowRun> {
    run_role_lifecycle_eval(
        connection,
        review_candidate_consent_publication_boundary_case()?,
        output_dir,
        source_commit,
        run_review_candidate_consent_publication_boundary_step,
    )
}

fn run_role_lifecycle_eval<F>(
    connection: &Connection,
    case: EvalCase,
    output_dir: impl Into<PathBuf>,
    source_commit: impl Into<String>,
    step_runner: F,
) -> Result<EvalWorkflowRun>
where
    F: FnMut(&Connection, &EvalStep) -> Result<()>,
{
    let packet_path = output_dir.into().join(format!("{}-packet.json", case.id));
    let mut harness = DeterministicEvalHarness::new(DeterministicEvalClock::fixed())
        .with_artifact_path(packet_path.to_string_lossy());
    let scorecard = harness.run_case(connection, &case, step_runner)?;
    let output_dir = packet_path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    let writer = EvalArtifactWriter::new(output_dir, source_commit)
        .with_private_terms(vec!["package".to_string(), "Project Orchid".to_string()]);
    let artifact_paths = writer.write_packet(connection, &case, &scorecard)?;
    Ok(EvalWorkflowRun {
        case,
        scorecard,
        artifact_paths,
    })
}

fn relationship_conversation_message_case() -> Result<EvalCase> {
    EvalCase::new(
        "relationship_conversation_message",
        "Relationship conversation message",
        &json!({
            "fixture": "relationship_conversation_message",
            "version": 1,
            "sensitiveFixtureKinds": ["email", "phone", "api_key", "private_term"],
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
            )?,
            EvalStep::new(
                "submit_message",
                EvalActorRole::AnonymousVisitor,
                "message.submit",
                vec![
                    EvalEvidenceChannel::SqliteRows,
                    EvalEvidenceChannel::ConversationEvents,
                    EvalEvidenceChannel::RealtimeReplay,
                ],
            )?,
        ],
        vec![
            EvalAssertion::minimum_count(
                "conversation_events_exist",
                EvalEvidenceChannel::ConversationEvents,
                2,
            )?,
            EvalAssertion::minimum_count(
                "realtime_replay_exists",
                EvalEvidenceChannel::RealtimeReplay,
                2,
            )?,
        ],
    )
}

fn privacy_gateway_roundtrip_case() -> Result<EvalCase> {
    EvalCase::new(
        "privacy_gateway_roundtrip",
        "Privacy gateway roundtrip",
        &json!({
            "fixture": "privacy_gateway_roundtrip",
            "version": 1,
            "providerMode": "deterministic_only",
        }),
        vec![
            EvalActorRole::Staff,
            EvalActorRole::OrdoAgent,
            EvalActorRole::LlmToolProviderBoundary,
        ],
        vec![EvalStep::new(
            "run_deterministic_llm_completion",
            EvalActorRole::LlmToolProviderBoundary,
            "llm.run.request",
            vec![
                EvalEvidenceChannel::ConversationEvents,
                EvalEvidenceChannel::PolicyDecisions,
                EvalEvidenceChannel::PromptSlotAccounting,
                EvalEvidenceChannel::PrivacyTransforms,
                EvalEvidenceChannel::TokenLedger,
                EvalEvidenceChannel::RealtimeReplay,
            ],
        )?],
        vec![
            EvalAssertion::minimum_count(
                "policy_decision_recorded",
                EvalEvidenceChannel::PolicyDecisions,
                1,
            )?,
            EvalAssertion::minimum_count(
                "prompt_slots_accounted",
                EvalEvidenceChannel::PromptSlotAccounting,
                2,
            )?,
            EvalAssertion::minimum_count(
                "privacy_transform_recorded",
                EvalEvidenceChannel::PrivacyTransforms,
                1,
            )?,
            EvalAssertion::minimum_count(
                "token_ledger_recorded",
                EvalEvidenceChannel::TokenLedger,
                2,
            )?,
            EvalAssertion::minimum_count(
                "conversation_events_recorded",
                EvalEvidenceChannel::ConversationEvents,
                7,
            )?,
        ],
    )
}

fn role_lifecycle_anonymous_to_client_case() -> Result<EvalCase> {
    EvalCase::new(
        "role_lifecycle_anonymous_to_client",
        "Role lifecycle anonymous visitor, client member, and affiliate boundaries",
        &json!({
            "fixture": "role_lifecycle_anonymous_to_client",
            "version": 1,
            "roles": ["anonymous_visitor", "client_member", "affiliate"],
            "providerMode": "deterministic_only",
        }),
        vec![
            EvalActorRole::AnonymousVisitor,
            EvalActorRole::ClientMember,
            EvalActorRole::Affiliate,
        ],
        vec![
            eval_step_with_metadata(
                "anonymous_visitor_relationship_message",
                EvalActorRole::AnonymousVisitor,
                "conversation.message.submit",
                vec![
                    EvalEvidenceChannel::SqliteRows,
                    EvalEvidenceChannel::ConversationEvents,
                    EvalEvidenceChannel::RealtimeReplay,
                ],
                json!({
                    "surface": "chat",
                    "subjectKind": "visitor_session",
                    "visibilityExpectation": "relationship_conversation_without_staff_admin_internals",
                }),
            )?,
            eval_step_with_metadata(
                "authenticated_client_relationship_continuity",
                EvalActorRole::ClientMember,
                "conversation.relationship.attach",
                vec![
                    EvalEvidenceChannel::SqliteRows,
                    EvalEvidenceChannel::ConversationEvents,
                    EvalEvidenceChannel::RealtimeReplay,
                    EvalEvidenceChannel::PolicyDecisions,
                ],
                json!({
                    "surface": "client_portal",
                    "subjectKind": "connection",
                    "visibilityExpectation": "one_client_visible_relationship_conversation",
                }),
            )?,
            eval_step_with_metadata(
                "affiliate_unrelated_customer_denied",
                EvalActorRole::Affiliate,
                "conversation.boundary.denied",
                vec![EvalEvidenceChannel::PolicyDecisions],
                json!({
                    "visibilityExpectation": "affiliate_cannot_access_unrelated_customer_conversation",
                }),
            )?,
        ],
        vec![
            EvalAssertion::minimum_count(
                "conversation_events_recorded",
                EvalEvidenceChannel::ConversationEvents,
                4,
            )?,
            EvalAssertion::minimum_count(
                "realtime_replay_recorded",
                EvalEvidenceChannel::RealtimeReplay,
                4,
            )?,
            EvalAssertion::minimum_count(
                "role_boundary_policy_decisions_recorded",
                EvalEvidenceChannel::PolicyDecisions,
                2,
            )?,
        ],
    )
}

fn role_lifecycle_staff_manager_owner_case() -> Result<EvalCase> {
    EvalCase::new(
        "role_lifecycle_staff_manager_owner_boundaries",
        "Role lifecycle staff, manager, owner, and system internals boundaries",
        &json!({
            "fixture": "role_lifecycle_staff_manager_owner_boundaries",
            "version": 1,
            "roles": ["staff", "manager_admin", "owner_system_admin"],
        }),
        vec![
            EvalActorRole::Staff,
            EvalActorRole::ManagerAdmin,
            EvalActorRole::OwnerSystemAdmin,
        ],
        vec![
            eval_step_with_metadata(
                "seed_staff_handoff",
                EvalActorRole::Staff,
                "handoff.create",
                vec![
                    EvalEvidenceChannel::SqliteRows,
                    EvalEvidenceChannel::ConversationEvents,
                    EvalEvidenceChannel::RealtimeReplay,
                    EvalEvidenceChannel::HandoffState,
                ],
                json!({
                    "defaultQueue": "my_handoffs",
                    "assignedActorId": "actor_staff_eval_1",
                }),
            )?,
            eval_step_with_metadata(
                "assert_queue_role_boundaries",
                EvalActorRole::ManagerAdmin,
                "conversation.queue.read",
                vec![
                    EvalEvidenceChannel::HandoffState,
                    EvalEvidenceChannel::PolicyDecisions,
                ],
                json!({
                    "staffDefault": "my_handoffs",
                    "managerAllowed": "team_queue",
                    "ownerAllowed": "all_conversations",
                    "staffDenied": "all_conversations",
                }),
            )?,
            eval_step_with_metadata(
                "assert_owner_system_boundary",
                EvalActorRole::OwnerSystemAdmin,
                "daemon.system.route.boundary",
                vec![EvalEvidenceChannel::PolicyDecisions],
                json!({
                    "ordinaryBrowser": "denied_without_loopback_or_token",
                    "ownerSystem": "allowed_with_loopback",
                }),
            )?,
        ],
        vec![
            EvalAssertion::minimum_count(
                "handoff_state_recorded",
                EvalEvidenceChannel::HandoffState,
                1,
            )?,
            EvalAssertion::minimum_count(
                "queue_policy_decisions_recorded",
                EvalEvidenceChannel::PolicyDecisions,
                2,
            )?,
            EvalAssertion::minimum_count(
                "conversation_events_recorded",
                EvalEvidenceChannel::ConversationEvents,
                2,
            )?,
        ],
    )
}

fn role_lifecycle_agent_silence_case() -> Result<EvalCase> {
    EvalCase::new(
        "role_lifecycle_agent_silence_boundary",
        "Role lifecycle Ordo agent silence during human-led active mode",
        &json!({
            "fixture": "role_lifecycle_agent_silence_boundary",
            "version": 1,
            "mode": "human_led_active",
        }),
        vec![
            EvalActorRole::Staff,
            EvalActorRole::OrdoAgent,
            EvalActorRole::ClientMember,
        ],
        vec![
            eval_step_with_metadata(
                "staff_reply_sets_human_led_active",
                EvalActorRole::Staff,
                "conversation.mode.human_led_active",
                vec![
                    EvalEvidenceChannel::SqliteRows,
                    EvalEvidenceChannel::ConversationEvents,
                    EvalEvidenceChannel::RealtimeReplay,
                ],
                json!({
                    "mode": "human_led_active",
                    "ledByActorId": "actor_staff_eval_1",
                }),
            )?,
            eval_step_with_metadata(
                "agent_public_post_blocked_without_delegation",
                EvalActorRole::OrdoAgent,
                "agent.public_post.denied",
                vec![EvalEvidenceChannel::PolicyDecisions],
                json!({
                    "expectedDecision": "human_led_active_requires_tag_delegation_or_policy",
                    "clientVisibleMechanics": "hidden",
                }),
            )?,
        ],
        vec![
            EvalAssertion::minimum_count(
                "mode_events_recorded",
                EvalEvidenceChannel::ConversationEvents,
                2,
            )?,
            EvalAssertion::minimum_count(
                "agent_silence_policy_evidence_recorded",
                EvalEvidenceChannel::PolicyDecisions,
                1,
            )?,
        ],
    )
}

fn feedback_capture_private_business_intelligence_case() -> Result<EvalCase> {
    EvalCase::new(
        "feedback_capture_private_business_intelligence",
        "Customer feedback capture as private business intelligence",
        &json!({
            "fixture": "feedback_capture_private_business_intelligence",
            "version": 1,
            "feedbackContract": "private_business_intelligence",
        }),
        vec![
            EvalActorRole::ClientMember,
            EvalActorRole::Staff,
            EvalActorRole::ManagerAdmin,
        ],
        vec![
            eval_step_with_metadata(
                "seed_feedback_source_message",
                EvalActorRole::ClientMember,
                "conversation.message.submit",
                vec![
                    EvalEvidenceChannel::ConversationEvents,
                    EvalEvidenceChannel::RealtimeReplay,
                ],
                json!({
                    "source": "durable_message",
                    "containsPrivateContactFixture": true,
                }),
            )?,
            eval_step_with_metadata(
                "capture_private_feedback",
                EvalActorRole::Staff,
                "feedback.capture",
                vec![
                    EvalEvidenceChannel::FeedbackReviewRecords,
                    EvalEvidenceChannel::RealtimeReplay,
                ],
                json!({
                    "visibility": "private_business_intelligence",
                    "notReview": true,
                    "notTestimonial": true,
                }),
            )?,
            eval_step_with_metadata(
                "star_and_tag_feedback_candidate",
                EvalActorRole::Staff,
                "feedback.star_and_tag",
                vec![
                    EvalEvidenceChannel::FeedbackReviewRecords,
                    EvalEvidenceChannel::RealtimeReplay,
                ],
                json!({
                    "starMeans": "staff_signal_not_customer_rating",
                    "tagCandidateState": "proposed",
                }),
            )?,
        ],
        vec![
            EvalAssertion::minimum_count(
                "feedback_records_created",
                EvalEvidenceChannel::FeedbackReviewRecords,
                2,
            )?,
            EvalAssertion::minimum_count(
                "feedback_events_recorded",
                EvalEvidenceChannel::RealtimeReplay,
                4,
            )?,
            EvalAssertion::minimum_count(
                "conversation_source_evidence_recorded",
                EvalEvidenceChannel::ConversationEvents,
                2,
            )?,
        ],
    )
}

fn review_candidate_consent_publication_boundary_case() -> Result<EvalCase> {
    EvalCase::new(
        "review_candidate_consent_publication_boundary",
        "Review candidate consent and publication boundary",
        &json!({
            "fixture": "review_candidate_consent_publication_boundary",
            "version": 1,
            "reviewLifecycle": [
                "candidate",
                "requested",
                "received",
                "consent_confirmed",
                "approved",
                "published",
                "featured",
                "retired"
            ],
        }),
        vec![
            EvalActorRole::ClientMember,
            EvalActorRole::Staff,
            EvalActorRole::ManagerAdmin,
        ],
        vec![
            eval_step_with_metadata(
                "create_review_candidate",
                EvalActorRole::Staff,
                "review.candidate.create",
                vec![
                    EvalEvidenceChannel::FeedbackReviewRecords,
                    EvalEvidenceChannel::RealtimeReplay,
                ],
                json!({
                    "source": "private_feedback",
                    "initialVisibility": "private_until_approved",
                }),
            )?,
            eval_step_with_metadata(
                "assert_publish_blocked_before_consent_approval",
                EvalActorRole::Staff,
                "review.publish.denied",
                vec![EvalEvidenceChannel::FeedbackReviewRecords],
                json!({
                    "requiredBeforePublish": ["consent_evidence", "approval_evidence"],
                }),
            )?,
            eval_step_with_metadata(
                "complete_review_consent_approval_publication_lifecycle",
                EvalActorRole::ManagerAdmin,
                "review.lifecycle.transition",
                vec![
                    EvalEvidenceChannel::FeedbackReviewRecords,
                    EvalEvidenceChannel::RealtimeReplay,
                ],
                json!({
                    "terminalState": "retired",
                    "publishedOnlyAfter": ["consent_confirmed", "approved"],
                }),
            )?,
        ],
        vec![
            EvalAssertion::minimum_count(
                "review_records_created",
                EvalEvidenceChannel::FeedbackReviewRecords,
                2,
            )?,
            EvalAssertion::minimum_count(
                "review_events_recorded",
                EvalEvidenceChannel::RealtimeReplay,
                8,
            )?,
        ],
    )
}

fn eval_step_with_metadata(
    id: impl Into<String>,
    actor_role: EvalActorRole,
    action: impl Into<String>,
    expected_evidence: Vec<EvalEvidenceChannel>,
    metadata: Value,
) -> Result<EvalStep> {
    let mut step = EvalStep::new(id, actor_role, action, expected_evidence)?;
    step.metadata = metadata;
    Ok(step)
}

fn run_relationship_conversation_step(connection: &Connection, step: &EvalStep) -> Result<()> {
    match step.id.as_str() {
        "create_canonical_conversation" => {
            find_or_create_canonical_conversation(connection, &visitor_conversation_request())?;
        }
        "submit_message" => {
            let conversation =
                find_or_create_canonical_conversation(connection, &visitor_conversation_request())?;
            let participant = create_visitor_participant(connection, &conversation.id)?;
            create_conversation_message(
                connection,
                &ConversationMessageCreateRequest {
                    conversation_id: conversation.id,
                    segment_id: None,
                    participant_id: participant.id,
                    message_kind: "user".to_string(),
                    body_markdown:
                        "I need help choosing a package. Email me at alex@example.com or 555-123-4567. sk-eval-secret"
                            .to_string(),
                    visibility: "participants".to_string(),
                    client_message_id: "eval-client-message-1".to_string(),
                    reply_to_message_id: None,
                    undo_expires_at: None,
                },
            )?;
        }
        other => anyhow::bail!("unsupported relationship workflow eval step: {other}"),
    }
    Ok(())
}

fn run_privacy_gateway_roundtrip_step(
    db_path: &Path,
    connection: &Connection,
    step: &EvalStep,
) -> Result<()> {
    match step.id.as_str() {
        "run_deterministic_llm_completion" => {
            let (conversation_id, assistant_id) = conversation_and_assistant(connection)?;
            let gateway = LlmGateway::new(DeterministicLlmProvider::new("local_fake", "fake-chat"))
                .with_private_terms(vec!["Project Orchid".to_string()]);
            gateway.run_completion(
                db_path,
                connection,
                &ActorContext::local_owner("eval_privacy_gateway_roundtrip"),
                LlmGatewayRequest {
                    run_id: "eval_llm_run_privacy_roundtrip".to_string(),
                    conversation_id,
                    segment_id: None,
                    assistant_participant_id: assistant_id,
                    client_id: Some("eval-client-llm-1".to_string()),
                    provider_id: "local_fake".to_string(),
                    model_id: "fake-chat".to_string(),
                    user_message: "Draft a reply for Project Orchid. Contact alex@example.com, 555-123-4567, sk-eval-secret.".to_string(),
                    prompt_slots: vec![
                        PromptSlot::new(
                            "ethical_business_persuasion",
                            "Ethical Business Persuasion",
                            "Use verified evidence only; preserve client agency.",
                            vec![
                                "docs/architecture/conversation-realtime/product-doctrine.md"
                                    .to_string(),
                            ],
                            "Business communication lens required by product doctrine.",
                            "staff_private",
                        )?,
                        PromptSlot::new(
                            "conversation_brief",
                            "Conversation Brief",
                            "Project Orchid needs a grounded next step.",
                            vec!["conversation_event_eval_1".to_string()],
                            "Current conversation evidence.",
                            "participants",
                        )?,
                    ],
                },
            )?;
        }
        other => anyhow::bail!("unsupported privacy workflow eval step: {other}"),
    }
    Ok(())
}

fn run_role_lifecycle_anonymous_to_client_step(
    connection: &Connection,
    step: &EvalStep,
) -> Result<()> {
    match step.id.as_str() {
        "anonymous_visitor_relationship_message" => {
            run_relationship_conversation_step(
                connection,
                &EvalStep::new(
                    "submit_message",
                    EvalActorRole::AnonymousVisitor,
                    "message.submit",
                    vec![],
                )?,
            )?;
        }
        "authenticated_client_relationship_continuity" => {
            let conversation = find_or_create_canonical_conversation(
                connection,
                &CanonicalConversationRequest {
                    surface: "client_portal".to_string(),
                    subject_kind: "connection".to_string(),
                    subject_id: "connection_eval_1".to_string(),
                    connection_id: Some("connection_eval_1".to_string()),
                    visitor_session_id: None,
                    created_by_actor_id: Some("actor_client_eval_1".to_string()),
                },
            )?;
            let repeated = find_or_create_canonical_conversation(
                connection,
                &CanonicalConversationRequest {
                    surface: "client_portal".to_string(),
                    subject_kind: "connection".to_string(),
                    subject_id: "connection_eval_1".to_string(),
                    connection_id: Some("connection_eval_1".to_string()),
                    visitor_session_id: None,
                    created_by_actor_id: Some("actor_client_eval_1".to_string()),
                },
            )?;
            ensure!(
                conversation.id == repeated.id,
                "client/member lifecycle must preserve one relationship conversation"
            );
            let participant = create_conversation_participant(
                connection,
                &ConversationParticipantCreateRequest {
                    conversation_id: conversation.id.clone(),
                    participant_kind: "connection".to_string(),
                    actor_id: Some("actor_client_eval_1".to_string()),
                    connection_id: Some("connection_eval_1".to_string()),
                    visitor_session_id: None,
                    display_name: "Eval Client".to_string(),
                    role: "client".to_string(),
                },
            )?;
            create_conversation_message(
                connection,
                &ConversationMessageCreateRequest {
                    conversation_id: conversation.id.clone(),
                    segment_id: None,
                    participant_id: participant.id,
                    message_kind: "user".to_string(),
                    body_markdown:
                        "Authenticated client asks for the next step without staff internals."
                            .to_string(),
                    visibility: "participants".to_string(),
                    client_message_id: "eval-client-member-message-1".to_string(),
                    reply_to_message_id: None,
                    undo_expires_at: None,
                },
            )?;
            let denied = authorize_protected_daemon_action(
                ActorContext::new(
                    ActorKind::BrowserOperator,
                    "client_member_eval",
                    Some("actor_client_eval_1".to_string()),
                ),
                PolicyAction::Read,
                ResourceRef::new(ResourceKind::DaemonRoute, "/admin/policy-decisions"),
                Some("policy.decisions.query"),
                ProtectedAccessEvidence {
                    loopback: false,
                    token: false,
                },
            );
            ensure!(
                !denied.allowed(),
                "client/member should not satisfy protected admin route access"
            );
            record_policy_decision(
                connection,
                &denied,
                PolicyDecisionCorrelation {
                    request_id: Some("eval_client_admin_boundary".to_string()),
                    ..Default::default()
                },
            )?;
        }
        "affiliate_unrelated_customer_denied" => {
            let denied = authorize_resource_access(
                connection,
                ActorContext::new(
                    ActorKind::BrowserOperator,
                    "affiliate_eval",
                    Some("actor_affiliate_eval_1".to_string()),
                ),
                PolicyAction::Read,
                ResourceRef::new(
                    ResourceKind::Conversation,
                    "conversation_unrelated_customer",
                ),
                Some("conversation.read"),
            );
            ensure!(
                !denied.allowed(),
                "affiliate should not access unrelated customer conversation"
            );
            record_policy_decision(
                connection,
                &denied,
                PolicyDecisionCorrelation {
                    request_id: Some("eval_affiliate_boundary".to_string()),
                    ..Default::default()
                },
            )?;
        }
        other => anyhow::bail!("unsupported anonymous/client role lifecycle eval step: {other}"),
    }
    Ok(())
}

fn run_role_lifecycle_staff_manager_owner_step(
    connection: &Connection,
    step: &EvalStep,
) -> Result<()> {
    match step.id.as_str() {
        "seed_staff_handoff" => {
            let conversation = find_or_create_canonical_conversation(
                connection,
                &CanonicalConversationRequest {
                    surface: "client_portal".to_string(),
                    subject_kind: "connection".to_string(),
                    subject_id: "connection_eval_1".to_string(),
                    connection_id: Some("connection_eval_1".to_string()),
                    visitor_session_id: None,
                    created_by_actor_id: Some("actor_staff_eval_1".to_string()),
                },
            )?;
            create_conversation_handoff(
                connection,
                &ConversationHandoffCreateRequest {
                    conversation_id: conversation.id,
                    segment_id: None,
                    connection_id: Some("connection_eval_1".to_string()),
                    requested_by_actor_id: Some("actor_client_eval_1".to_string()),
                    assigned_to_actor_id: Some("actor_staff_eval_1".to_string()),
                    reason: "Client needs staff follow-up".to_string(),
                    urgency: "high".to_string(),
                    required_capability_id: "conversation.handoff.manage".to_string(),
                    evidence_summary: "Client asked for a human review of the next step."
                        .to_string(),
                    allowed_context: vec![
                        "conversation_summary".to_string(),
                        "client_request".to_string(),
                    ],
                    policy_decision_id: None,
                },
            )?;
        }
        "assert_queue_role_boundaries" => {
            let staff_rows = conversation_queue(
                connection,
                ConversationRole::Staff,
                Some("actor_staff_eval_1"),
                None,
            )?;
            let manager_rows = conversation_queue(
                connection,
                ConversationRole::Manager,
                None,
                Some(QueueScope::TeamQueue),
            )?;
            let owner_rows = conversation_queue(
                connection,
                ConversationRole::Owner,
                None,
                Some(QueueScope::AllConversations),
            )?;
            let staff_all = conversation_queue(
                connection,
                ConversationRole::Staff,
                Some("actor_staff_eval_1"),
                Some(QueueScope::AllConversations),
            );
            ensure!(staff_rows.len() == 1, "staff should default to My Handoffs");
            ensure!(
                manager_rows.len() == 1,
                "manager/admin should access Team Queue"
            );
            ensure!(
                owner_rows.len() == 1,
                "owner/system admin should access All Conversations"
            );
            ensure!(
                staff_all.is_err(),
                "ordinary staff should not access All Conversations"
            );
        }
        "assert_owner_system_boundary" => {
            let denied = authorize_protected_daemon_action(
                ActorContext::browser_operator(),
                PolicyAction::Read,
                ResourceRef::new(ResourceKind::DaemonRoute, "/diagnostic-logs"),
                Some("diagnostic_logs.read"),
                ProtectedAccessEvidence {
                    loopback: false,
                    token: false,
                },
            );
            let allowed = authorize_protected_daemon_action(
                ActorContext::local_owner("eval_role_lifecycle"),
                PolicyAction::Read,
                ResourceRef::new(ResourceKind::DaemonRoute, "/diagnostic-logs"),
                Some("diagnostic_logs.read"),
                ProtectedAccessEvidence {
                    loopback: true,
                    token: false,
                },
            );
            ensure!(!denied.allowed(), "non-owner browser should be denied");
            ensure!(allowed.allowed(), "owner/system loopback should be allowed");
            record_policy_decision(
                connection,
                &denied,
                PolicyDecisionCorrelation {
                    request_id: Some("eval_staff_system_boundary_denied".to_string()),
                    ..Default::default()
                },
            )?;
            record_policy_decision(
                connection,
                &allowed,
                PolicyDecisionCorrelation {
                    request_id: Some("eval_owner_system_boundary_allowed".to_string()),
                    ..Default::default()
                },
            )?;
        }
        other => anyhow::bail!("unsupported staff/manager/owner role lifecycle eval step: {other}"),
    }
    Ok(())
}

fn run_role_lifecycle_agent_silence_step(connection: &Connection, step: &EvalStep) -> Result<()> {
    match step.id.as_str() {
        "staff_reply_sets_human_led_active" => {
            let conversation = find_or_create_canonical_conversation(
                connection,
                &CanonicalConversationRequest {
                    surface: "client_portal".to_string(),
                    subject_kind: "connection".to_string(),
                    subject_id: "connection_eval_1".to_string(),
                    connection_id: Some("connection_eval_1".to_string()),
                    visitor_session_id: None,
                    created_by_actor_id: Some("actor_staff_eval_1".to_string()),
                },
            )?;
            record_staff_activity_sets_human_led(
                connection,
                &conversation.id,
                "actor_staff_eval_1",
            )?;
        }
        "agent_public_post_blocked_without_delegation" => {
            let decision = may_agent_post_publicly(
                ConversationMode::HumanLedActive,
                &PublicPostContext::default(),
            );
            ensure!(
                !decision.allowed,
                "agent should stay silent publicly during human-led active mode"
            );
            ensure!(
                decision.reason == "human_led_active_requires_tag_delegation_or_policy",
                "agent silence decision should cite the human-led boundary"
            );
            let policy_decision = authorize_resource_access(
                connection,
                ActorContext::new(
                    ActorKind::System,
                    "ordo_agent_eval",
                    Some("actor_system".to_string()),
                ),
                PolicyAction::Create,
                ResourceRef::new(ResourceKind::ConversationMessage, "public_agent_message"),
                Some("conversation.message.create"),
            );
            record_policy_decision(
                connection,
                &policy_decision,
                PolicyDecisionCorrelation {
                    request_id: Some("eval_agent_public_post_blocked".to_string()),
                    ..Default::default()
                },
            )?;
        }
        other => anyhow::bail!("unsupported agent silence role lifecycle eval step: {other}"),
    }
    Ok(())
}

fn run_feedback_capture_private_business_intelligence_step(
    connection: &Connection,
    step: &EvalStep,
) -> Result<()> {
    match step.id.as_str() {
        "seed_feedback_source_message" => {
            seed_feedback_source_message(connection)?;
        }
        "capture_private_feedback" => {
            let (conversation_id, message_id) = seed_feedback_source_message(connection)?;
            let (feedback, _) = capture_feedback(
                connection,
                CustomerFeedbackInput {
                    connection_id: Some("connection_eval_1".to_string()),
                    conversation_id,
                    segment_id: None,
                    message_id: Some(message_id.clone()),
                    feedback_kind: "praise".to_string(),
                    body_summary: "Client says the onboarding was clear and useful.".to_string(),
                    source_refs: vec![message_id.clone()],
                    evidence_refs: vec![message_id],
                    provenance: json!({
                        "workflow": "feedback_capture_private_business_intelligence",
                        "source": "conversation_message",
                    }),
                },
            )?;
            ensure!(
                feedback.visibility == "private_business_intelligence",
                "feedback must remain private business intelligence"
            );
            ensure!(
                list_public_reviews(connection)?.is_empty(),
                "private feedback must not create a public review"
            );
        }
        "star_and_tag_feedback_candidate" => {
            let feedback_id = latest_feedback_id(connection)?;
            let (starred, _) = set_feedback_starred(
                connection,
                &feedback_id,
                true,
                vec!["staff_review_1".to_string()],
            )?;
            ensure!(
                starred.is_starred,
                "starred feedback should persist as staff signal"
            );
            let (tag, _) = propose_feedback_tag(
                connection,
                &feedback_id,
                FeedbackTagInput {
                    tag: "clear_onboarding".to_string(),
                    confidence: 0.82,
                    evidence_refs: starred.evidence_refs.clone(),
                    provenance: json!({
                        "workflow": "feedback_capture_private_business_intelligence",
                        "candidate": true,
                    }),
                },
            )?;
            ensure!(
                tag.candidate_state == "proposed",
                "feedback tags should default to proposed candidates"
            );
            let private_rows = list_private_feedback(connection, &starred.conversation_id)?;
            ensure!(
                private_rows.len() == 1,
                "feedback should be visible in private staff intelligence list"
            );
        }
        other => anyhow::bail!("unsupported feedback capture workflow eval step: {other}"),
    }
    Ok(())
}

fn run_review_candidate_consent_publication_boundary_step(
    connection: &Connection,
    step: &EvalStep,
) -> Result<()> {
    match step.id.as_str() {
        "create_review_candidate" => {
            let feedback_id = ensure_review_feedback(connection)?;
            let (review, _) = create_review_candidate(
                connection,
                &feedback_id,
                ReviewCandidateInput {
                    review_body: "The onboarding made the next decision easy.".to_string(),
                    evidence_refs: vec![feedback_id.clone()],
                    provenance: json!({
                        "workflow": "review_candidate_consent_publication_boundary",
                        "source": "private_feedback",
                    }),
                },
            )?;
            ensure!(
                review.status == ReviewStatus::Candidate,
                "review should start as candidate"
            );
            ensure!(
                review.publication_visibility == "private_until_approved",
                "review candidate must remain private before consent and approval"
            );
        }
        "assert_publish_blocked_before_consent_approval" => {
            let review_id = latest_review_id(connection)?;
            let blocked = transition_review(
                connection,
                &review_id,
                ReviewStatus::Published,
                vec!["publish_attempt_1".to_string()],
                "attempted early publication",
            );
            ensure!(
                blocked.is_err(),
                "review publication should fail closed before consent and approval"
            );
            ensure!(
                list_public_reviews(connection)?.is_empty(),
                "early publication failure should not expose public reviews"
            );
        }
        "complete_review_consent_approval_publication_lifecycle" => {
            let review_id = latest_review_id(connection)?;
            let (requested, _) = transition_review(
                connection,
                &review_id,
                ReviewStatus::Requested,
                vec!["review_request_1".to_string()],
                "request review",
            )?;
            let (received, _) = transition_review(
                connection,
                &requested.id,
                ReviewStatus::Received,
                vec!["review_received_1".to_string()],
                "review received",
            )?;
            let (consented, _) = transition_review(
                connection,
                &received.id,
                ReviewStatus::ConsentConfirmed,
                vec!["review_consent_1".to_string()],
                "customer consent confirmed",
            )?;
            let (approved, _) = transition_review(
                connection,
                &consented.id,
                ReviewStatus::Approved,
                vec!["review_approval_1".to_string()],
                "staff approved publication",
            )?;
            let (published, _) = transition_review(
                connection,
                &approved.id,
                ReviewStatus::Published,
                vec!["review_publish_1".to_string()],
                "publish public review",
            )?;
            ensure!(
                list_public_reviews(connection)?.len() == 1,
                "published review should become public only after consent and approval"
            );
            let (featured, _) = transition_review(
                connection,
                &published.id,
                ReviewStatus::Featured,
                vec!["review_feature_1".to_string()],
                "feature review",
            )?;
            let (retired, _) = transition_review(
                connection,
                &featured.id,
                ReviewStatus::Retired,
                vec!["review_retire_1".to_string()],
                "retire review",
            )?;
            ensure!(
                retired.status == ReviewStatus::Retired,
                "review lifecycle should support retired state"
            );
            ensure!(
                list_public_reviews(connection)?.is_empty(),
                "retired reviews should leave the public review list"
            );
        }
        other => anyhow::bail!("unsupported review workflow eval step: {other}"),
    }
    Ok(())
}

fn seed_feedback_source_message(connection: &Connection) -> Result<(String, String)> {
    let conversation =
        find_or_create_canonical_conversation(connection, &feedback_conversation_request())?;
    let participant = create_conversation_participant(
        connection,
        &ConversationParticipantCreateRequest {
            conversation_id: conversation.id.clone(),
            participant_kind: "connection".to_string(),
            actor_id: Some("actor_client_eval_1".to_string()),
            connection_id: Some("connection_eval_1".to_string()),
            visitor_session_id: None,
            display_name: "Eval Client".to_string(),
            role: "client".to_string(),
        },
    )?;
    let message = create_conversation_message(
        connection,
        &ConversationMessageCreateRequest {
            conversation_id: conversation.id.clone(),
            segment_id: None,
            participant_id: participant.id,
            message_kind: "user".to_string(),
            body_markdown:
                "The onboarding was clear. Please keep my email alex@example.com private."
                    .to_string(),
            visibility: "participants".to_string(),
            client_message_id: "eval-feedback-source-message-1".to_string(),
            reply_to_message_id: None,
            undo_expires_at: None,
        },
    )?;
    Ok((conversation.id, message.id))
}

fn ensure_review_feedback(connection: &Connection) -> Result<String> {
    if let Some(id) = connection
        .query_row(
            "SELECT id FROM customer_feedback ORDER BY created_at DESC LIMIT 1",
            [],
            |row| row.get(0),
        )
        .optional()?
    {
        return Ok(id);
    }

    let (conversation_id, message_id) = seed_feedback_source_message(connection)?;
    let (feedback, _) = capture_feedback(
        connection,
        CustomerFeedbackInput {
            connection_id: Some("connection_eval_1".to_string()),
            conversation_id,
            segment_id: None,
            message_id: Some(message_id.clone()),
            feedback_kind: "praise".to_string(),
            body_summary: "Client says onboarding made the next decision easy.".to_string(),
            source_refs: vec![message_id.clone()],
            evidence_refs: vec![message_id],
            provenance: json!({
                "workflow": "review_candidate_consent_publication_boundary",
                "source": "conversation_message",
            }),
        },
    )?;
    Ok(feedback.id)
}

fn latest_feedback_id(connection: &Connection) -> Result<String> {
    connection
        .query_row(
            "SELECT id FROM customer_feedback ORDER BY created_at DESC LIMIT 1",
            [],
            |row| row.get(0),
        )
        .optional()?
        .ok_or_else(|| anyhow::anyhow!("expected feedback row"))
}

fn latest_review_id(connection: &Connection) -> Result<String> {
    connection
        .query_row(
            "SELECT id FROM customer_reviews ORDER BY created_at DESC LIMIT 1",
            [],
            |row| row.get(0),
        )
        .optional()?
        .ok_or_else(|| anyhow::anyhow!("expected review row"))
}

fn feedback_conversation_request() -> CanonicalConversationRequest {
    CanonicalConversationRequest {
        surface: "client_portal".to_string(),
        subject_kind: "connection".to_string(),
        subject_id: "connection_eval_1".to_string(),
        connection_id: Some("connection_eval_1".to_string()),
        visitor_session_id: None,
        created_by_actor_id: Some("actor_client_eval_1".to_string()),
    }
}

fn visitor_conversation_request() -> CanonicalConversationRequest {
    CanonicalConversationRequest {
        surface: "chat".to_string(),
        subject_kind: "visitor_session".to_string(),
        subject_id: "visitor_session_eval_1".to_string(),
        connection_id: None,
        visitor_session_id: Some("visitor_session_eval_1".to_string()),
        created_by_actor_id: None,
    }
}

fn create_visitor_participant(
    connection: &Connection,
    conversation_id: &str,
) -> Result<crate::conversations::ConversationParticipantView> {
    create_conversation_participant(
        connection,
        &ConversationParticipantCreateRequest {
            conversation_id: conversation_id.to_string(),
            participant_kind: "visitor".to_string(),
            actor_id: None,
            connection_id: None,
            visitor_session_id: Some("visitor_session_eval_1".to_string()),
            display_name: "Visitor".to_string(),
            role: "client".to_string(),
        },
    )
}

fn conversation_and_assistant(connection: &Connection) -> Result<(String, String)> {
    let conversation = find_or_create_canonical_conversation(
        connection,
        &CanonicalConversationRequest {
            surface: "client_portal".to_string(),
            subject_kind: "connection".to_string(),
            subject_id: "connection_eval_1".to_string(),
            connection_id: Some("connection_eval_1".to_string()),
            visitor_session_id: None,
            created_by_actor_id: Some("actor_staff_eval_1".to_string()),
        },
    )?;
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
    )?;
    Ok((conversation.id, assistant.id))
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
            EvalEvidenceCount {
                channel: EvalEvidenceChannel::FeedbackReviewRecords,
                count: table_count(connection, "customer_feedback")?
                    + table_count(connection, "feedback_tags")?
                    + table_count(connection, "customer_reviews")?,
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
        "customer_feedback",
        "feedback_tags",
        "customer_reviews",
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

fn transcript_ledger(connection: &Connection) -> Result<Vec<EvalLedgerEntry>> {
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

fn timeline_ledger(connection: &Connection) -> Result<Vec<EvalLedgerEntry>> {
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

fn conversation_event_ledger(connection: &Connection) -> Result<Vec<EvalLedgerEntry>> {
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

fn realtime_replay_ledger(connection: &Connection) -> Result<Vec<EvalLedgerEntry>> {
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

fn policy_decision_ledger(connection: &Connection) -> Result<Vec<EvalLedgerEntry>> {
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

fn prompt_slot_ledger(connection: &Connection) -> Result<Vec<EvalLedgerEntry>> {
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

fn privacy_transform_ledger(connection: &Connection) -> Result<Vec<EvalLedgerEntry>> {
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

fn token_ledger(connection: &Connection) -> Result<Vec<EvalLedgerEntry>> {
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

fn analysis_candidate_ledger(connection: &Connection) -> Result<Vec<EvalLedgerEntry>> {
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

fn handoff_ledger(connection: &Connection) -> Result<Vec<EvalLedgerEntry>> {
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

fn artifact_ledger(connection: &Connection) -> Result<Vec<EvalLedgerEntry>> {
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

fn surface_brief_ledger(connection: &Connection) -> Result<Vec<EvalLedgerEntry>> {
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

fn feedback_ledger(connection: &Connection) -> Result<Vec<EvalLedgerEntry>> {
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

fn review_ledger(connection: &Connection) -> Result<Vec<EvalLedgerEntry>> {
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

fn collect_rows(
    rows: rusqlite::MappedRows<
        '_,
        impl FnMut(&rusqlite::Row<'_>) -> rusqlite::Result<EvalLedgerEntry>,
    >,
) -> Result<Vec<EvalLedgerEntry>> {
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(Into::into)
}

fn parse_json_value(raw: String) -> Value {
    serde_json::from_str(&raw)
        .unwrap_or_else(|_| json!({ "unparseableHash": stable_text_hash(&raw) }))
}

fn write_json<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    let encoded = serde_json::to_string_pretty(value)?;
    fs::write(path, encoded).with_context(|| format!("write JSON artifact {}", path.display()))?;
    Ok(())
}

fn redact_serializable<T: Serialize + for<'de> Deserialize<'de>>(
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

fn redact_value(value: &mut Value, private_terms: &[String], summary: &mut EvalRedactionSummary) {
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

fn redact_text(text: &str, private_terms: &[String], summary: &mut EvalRedactionSummary) -> String {
    let mut output = Vec::new();
    for token in text.split_whitespace() {
        let trimmed = token.trim_matches(|character: char| {
            character == ',' || character == '.' || character == ';' || character == ':'
        });
        let mut replacement = None;
        if looks_like_email(trimmed) {
            replacement = Some("[REDACTED:email]");
        } else if looks_like_phone(trimmed) {
            replacement = Some("[REDACTED:phone]");
        } else if looks_like_secret(trimmed) {
            replacement = Some("[REDACTED:secret]");
        } else if private_terms
            .iter()
            .filter(|term| !term.trim().is_empty())
            .any(|term| trimmed.eq_ignore_ascii_case(term.trim()))
        {
            replacement = Some("[REDACTED:private_term]");
        }
        if let Some(replacement) = replacement {
            summary.redacted_value_count += 1;
            output.push(replacement.to_string());
        } else {
            output.push(token.to_string());
        }
    }
    output.join(" ")
}

fn looks_like_email(value: &str) -> bool {
    let Some((local, domain)) = value.split_once('@') else {
        return false;
    };
    !local.is_empty() && domain.contains('.') && !domain.ends_with('.')
}

fn looks_like_phone(value: &str) -> bool {
    let digit_count = value
        .chars()
        .filter(|character| character.is_ascii_digit())
        .count();
    digit_count >= 10
        && value
            .chars()
            .all(|character| character.is_ascii_digit() || "()+-. ".contains(character))
}

fn looks_like_secret(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    lower.starts_with("sk-")
        || lower.starts_with("api_")
        || lower.starts_with("pat_")
        || lower.starts_with("ghp_")
        || lower == "bearer"
        || lower.starts_with("bearer_")
        || lower.starts_with("bearer-")
}

fn stable_json_hash(value: &Value) -> Result<String> {
    let encoded = serde_json::to_string(value)?;
    let mut hasher = Sha256::new();
    hasher.update(encoded.as_bytes());
    Ok(format!("{:x}", hasher.finalize()))
}

fn stable_text_hash(value: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(value.as_bytes());
    format!("{:x}", hasher.finalize())
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

#[cfg(test)]
mod tests {
    use super::*;

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
            .run_case(
                &connection,
                &relationship_conversation_message_case().unwrap(),
                run_relationship_conversation_step,
            )
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
        let case = relationship_conversation_message_case().unwrap();

        let mut first = DeterministicEvalHarness::new(DeterministicEvalClock::fixed());
        let mut second = DeterministicEvalHarness::new(DeterministicEvalClock::fixed());
        let first_scorecard = first
            .run_case(&first_connection, &case, run_relationship_conversation_step)
            .unwrap();
        let second_scorecard = second
            .run_case(
                &second_connection,
                &case,
                run_relationship_conversation_step,
            )
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

    #[test]
    fn artifact_packet_writer_emits_redacted_packet_scorecard_and_manifest() {
        let connection = isolated_eval_connection().unwrap();
        let case = relationship_conversation_message_case().unwrap();
        let temp_dir = tempfile::TempDir::new().unwrap();
        let mut harness = DeterministicEvalHarness::new(DeterministicEvalClock::fixed())
            .with_artifact_path(
                temp_dir
                    .path()
                    .join("relationship_message_foundation-packet.json")
                    .to_string_lossy(),
            );
        let scorecard = harness
            .run_case(&connection, &case, run_relationship_conversation_step)
            .unwrap();
        let writer = EvalArtifactWriter::new(temp_dir.path(), "test-commit")
            .with_private_terms(vec!["package".to_string()]);

        let paths = writer.write_packet(&connection, &case, &scorecard).unwrap();

        assert!(paths.packet_path.exists());
        assert!(paths.scorecard_path.exists());
        assert!(paths.manifest_path.exists());
        let packet_json = fs::read_to_string(&paths.packet_path).unwrap();
        let scorecard_json = fs::read_to_string(&paths.scorecard_path).unwrap();
        let manifest_json = fs::read_to_string(&paths.manifest_path).unwrap();
        assert!(packet_json.contains(EVAL_ARTIFACT_PACKET_SCHEMA_VERSION));
        assert!(packet_json.contains("\"conversationEventLedger\""));
        assert!(packet_json.contains("\"realtimeReplayLedger\""));
        assert!(packet_json.contains("\"policyDecisionLedger\": []"));
        assert!(packet_json.contains("\"promptSlotLedger\": []"));
        assert!(packet_json.contains("\"artifactReview\""));
        assert!(packet_json.contains("[REDACTED:email]"));
        assert!(packet_json.contains("[REDACTED:phone]"));
        assert!(packet_json.contains("[REDACTED:secret]"));
        assert!(packet_json.contains("[REDACTED:private_term]"));
        assert!(!packet_json.contains("alex@example.com"));
        assert!(!packet_json.contains("555-123-4567"));
        assert!(!packet_json.contains("sk-eval-secret"));
        assert!(scorecard_json.contains("\"providerMode\": \"deterministic_only\""));
        assert!(manifest_json.contains("\"sourceCommit\": \"test-commit\""));
    }

    #[test]
    fn normalized_artifact_packets_are_deterministic() {
        let first_connection = isolated_eval_connection().unwrap();
        let second_connection = isolated_eval_connection().unwrap();
        let case = relationship_conversation_message_case().unwrap();
        let mut first_harness = DeterministicEvalHarness::new(DeterministicEvalClock::fixed());
        let mut second_harness = DeterministicEvalHarness::new(DeterministicEvalClock::fixed());
        let first_scorecard = first_harness
            .run_case(&first_connection, &case, run_relationship_conversation_step)
            .unwrap();
        let second_scorecard = second_harness
            .run_case(
                &second_connection,
                &case,
                run_relationship_conversation_step,
            )
            .unwrap();
        let writer = EvalArtifactWriter::new("unused", "test-commit")
            .with_private_terms(vec!["package".to_string()]);

        let first_packet = writer
            .build_packet(&first_connection, &case, &first_scorecard)
            .unwrap();
        let second_packet = writer
            .build_packet(&second_connection, &case, &second_scorecard)
            .unwrap();

        assert_eq!(
            normalized_packet_for_comparison(&first_packet),
            normalized_packet_for_comparison(&second_packet)
        );
    }

    fn normalized_packet_for_comparison(packet: &EvalArtifactPacket) -> Value {
        json!({
            "schemaVersion": packet.schema_version,
            "caseId": packet.case_id,
            "fixtureHash": packet.fixture_hash,
            "actorRoles": packet.actor_roles,
            "stepCount": packet.steps.len(),
            "scorecardPassed": packet.scorecard.passed,
            "transcriptTypes": packet.transcript.iter().map(|entry| entry.entry_type.clone()).collect::<Vec<_>>(),
            "conversationEventTypes": packet.conversation_event_ledger.iter().map(|entry| entry.entry_type.clone()).collect::<Vec<_>>(),
            "realtimeReplayTypes": packet.realtime_replay_ledger.iter().map(|entry| entry.entry_type.clone()).collect::<Vec<_>>(),
            "policyCount": packet.policy_decision_ledger.len(),
            "promptSlotCount": packet.prompt_slot_ledger.len(),
            "redactedCount": packet.redaction_summary.redacted_value_count,
        })
    }

    #[test]
    fn relationship_workflow_eval_writes_artifacts() {
        let connection = isolated_eval_connection().unwrap();
        let temp_dir = tempfile::TempDir::new().unwrap();

        let run =
            run_relationship_conversation_message_eval(&connection, temp_dir.path(), "test-commit")
                .unwrap();

        assert!(run.scorecard.passed);
        assert_eq!(run.case.id, "relationship_conversation_message");
        assert!(run.artifact_paths.packet_path.exists());
        let packet = fs::read_to_string(run.artifact_paths.packet_path).unwrap();
        assert!(packet.contains("\"caseId\": \"relationship_conversation_message\""));
        assert!(packet.contains("\"conversationEventLedger\""));
        assert!(packet.contains("[REDACTED:email]"));
        assert!(!packet.contains("alex@example.com"));
    }

    #[test]
    fn privacy_gateway_roundtrip_eval_records_privacy_accounting_and_artifacts() {
        let connection = isolated_eval_connection().unwrap();
        let temp_dir = tempfile::TempDir::new().unwrap();
        let db_path = temp_dir.path().join("local.db");
        let artifact_dir = temp_dir.path().join("artifacts");

        let run =
            run_privacy_gateway_roundtrip_eval(&db_path, &connection, &artifact_dir, "test-commit")
                .unwrap();

        assert!(run.scorecard.passed);
        assert_eq!(run.case.id, "privacy_gateway_roundtrip");
        assert!(run.artifact_paths.packet_path.exists());
        assert!(
            run.scorecard
                .evidence_after
                .count_for(EvalEvidenceChannel::PolicyDecisions)
                >= 1
        );
        assert!(
            run.scorecard
                .evidence_after
                .count_for(EvalEvidenceChannel::PromptSlotAccounting)
                >= 2
        );
        assert!(
            run.scorecard
                .evidence_after
                .count_for(EvalEvidenceChannel::PrivacyTransforms)
                >= 1
        );
        assert!(
            run.scorecard
                .evidence_after
                .count_for(EvalEvidenceChannel::TokenLedger)
                >= 2
        );
        let packet = fs::read_to_string(run.artifact_paths.packet_path).unwrap();
        assert!(packet.contains("\"caseId\": \"privacy_gateway_roundtrip\""));
        assert!(packet.contains("\"promptSlotLedger\""));
        assert!(packet.contains("\"privacyTransformLedger\""));
        assert!(packet.contains("\"tokenLedger\""));
        assert!(!packet.contains("Project Orchid"));
        assert!(!packet.contains("alex@example.com"));
        assert!(!packet.contains("555-123-4567"));
        assert!(!packet.contains("sk-eval-secret"));
    }

    #[test]
    fn role_lifecycle_anonymous_client_affiliate_eval_writes_boundary_artifacts() {
        let connection = isolated_eval_connection().unwrap();
        let temp_dir = tempfile::TempDir::new().unwrap();

        let run = run_role_lifecycle_anonymous_to_client_eval(
            &connection,
            temp_dir.path(),
            "test-commit",
        )
        .unwrap();

        assert!(run.scorecard.passed);
        assert_eq!(run.case.id, "role_lifecycle_anonymous_to_client");
        assert_eq!(
            run.scorecard.actor_roles,
            vec![
                EvalActorRole::AnonymousVisitor,
                EvalActorRole::ClientMember,
                EvalActorRole::Affiliate,
            ]
        );
        assert!(
            run.scorecard
                .evidence_after
                .count_for(EvalEvidenceChannel::PolicyDecisions)
                >= 2
        );
        assert_eq!(
            connection
                .query_row(
                    "SELECT COUNT(*) FROM conversations
                     WHERE subject_kind = 'connection' AND subject_id = 'connection_eval_1'",
                    [],
                    |row| row.get::<_, i64>(0),
                )
                .unwrap(),
            1
        );
        let packet = fs::read_to_string(run.artifact_paths.packet_path).unwrap();
        assert!(packet.contains("\"caseId\": \"role_lifecycle_anonymous_to_client\""));
        assert!(packet.contains("\"anonymous_visitor\""));
        assert!(packet.contains("\"client_member\""));
        assert!(packet.contains("\"affiliate\""));
        assert!(packet.contains("affiliate_cannot_access_unrelated_customer_conversation"));
        assert!(packet.contains("[REDACTED:email]"));
        assert!(!packet.contains("alex@example.com"));
        assert!(packet.contains("\"handoffLedger\": []"));
        assert!(packet.contains("\"promptSlotLedger\": []"));
        assert!(packet.contains("\"privacyTransformLedger\": []"));
        assert!(packet.contains("\"tokenLedger\": []"));
    }

    #[test]
    fn role_lifecycle_staff_manager_owner_eval_asserts_queue_and_system_boundaries() {
        let connection = isolated_eval_connection().unwrap();
        let temp_dir = tempfile::TempDir::new().unwrap();

        let run = run_role_lifecycle_staff_manager_owner_eval(
            &connection,
            temp_dir.path(),
            "test-commit",
        )
        .unwrap();

        assert!(run.scorecard.passed);
        assert_eq!(run.case.id, "role_lifecycle_staff_manager_owner_boundaries");
        assert!(
            run.scorecard
                .evidence_after
                .count_for(EvalEvidenceChannel::HandoffState)
                >= 1
        );
        assert!(
            run.scorecard
                .evidence_after
                .count_for(EvalEvidenceChannel::PolicyDecisions)
                >= 2
        );
        let packet = fs::read_to_string(run.artifact_paths.packet_path).unwrap();
        assert!(packet.contains("\"staff\""));
        assert!(packet.contains("\"manager_admin\""));
        assert!(packet.contains("\"owner_system_admin\""));
        assert!(packet.contains("\"staffDefault\": \"my_handoffs\""));
        assert!(packet.contains("\"managerAllowed\": \"team_queue\""));
        assert!(packet.contains("\"ownerAllowed\": \"all_conversations\""));
        assert!(packet.contains("Protected daemon route requires loopback access"));
    }

    #[test]
    fn role_lifecycle_agent_silence_eval_records_human_led_boundary() {
        let connection = isolated_eval_connection().unwrap();
        let temp_dir = tempfile::TempDir::new().unwrap();

        let run =
            run_role_lifecycle_agent_silence_eval(&connection, temp_dir.path(), "test-commit")
                .unwrap();

        assert!(run.scorecard.passed);
        assert_eq!(run.case.id, "role_lifecycle_agent_silence_boundary");
        assert!(
            run.scorecard
                .evidence_after
                .count_for(EvalEvidenceChannel::PolicyDecisions)
                >= 1
        );
        let packet = fs::read_to_string(run.artifact_paths.packet_path).unwrap();
        assert!(packet.contains("\"ordo_agent\""));
        assert!(packet.contains("\"human_led_active\""));
        assert!(packet.contains("human_led_active_requires_tag_delegation_or_policy"));
        assert!(packet.contains("\"clientVisibleMechanics\": \"hidden\""));
        assert!(packet.contains("\"promptSlotLedger\": []"));
        assert!(packet.contains("\"privacyTransformLedger\": []"));
        assert!(packet.contains("\"tokenLedger\": []"));
    }

    #[test]
    fn feedback_capture_eval_records_private_feedback_and_tag_candidate() {
        let connection = isolated_eval_connection().unwrap();
        let temp_dir = tempfile::TempDir::new().unwrap();

        let run = run_feedback_capture_private_business_intelligence_eval(
            &connection,
            temp_dir.path(),
            "test-commit",
        )
        .unwrap();

        assert!(run.scorecard.passed);
        assert_eq!(
            run.case.id,
            "feedback_capture_private_business_intelligence"
        );
        assert!(
            run.scorecard
                .evidence_after
                .count_for(EvalEvidenceChannel::FeedbackReviewRecords)
                >= 2
        );
        assert_eq!(list_public_reviews(&connection).unwrap().len(), 0);
        let packet = fs::read_to_string(run.artifact_paths.packet_path).unwrap();
        assert!(packet.contains("\"feedbackLedger\""));
        assert!(packet.contains("\"reviewLedger\": []"));
        assert!(packet.contains("private_business_intelligence"));
        assert!(packet.contains("staff_signal_not_customer_rating"));
        assert!(packet.contains("clear_onboarding"));
        assert!(packet.contains("[REDACTED:email]"));
        assert!(!packet.contains("alex@example.com"));
    }

    #[test]
    fn review_candidate_eval_blocks_publication_until_consent_and_approval() {
        let connection = isolated_eval_connection().unwrap();
        let temp_dir = tempfile::TempDir::new().unwrap();

        let run = run_review_candidate_consent_publication_boundary_eval(
            &connection,
            temp_dir.path(),
            "test-commit",
        )
        .unwrap();

        assert!(run.scorecard.passed);
        assert_eq!(run.case.id, "review_candidate_consent_publication_boundary");
        assert!(
            run.scorecard
                .evidence_after
                .count_for(EvalEvidenceChannel::FeedbackReviewRecords)
                >= 2
        );
        assert_eq!(list_public_reviews(&connection).unwrap().len(), 0);
        let packet = fs::read_to_string(run.artifact_paths.packet_path).unwrap();
        assert!(packet.contains("\"reviewLedger\""));
        assert!(packet.contains("private_until_approved"));
        assert!(packet.contains("public_review"));
        assert!(packet.contains("consentEvidenceRefs"));
        assert!(packet.contains("approvalEvidenceRefs"));
        assert!(packet.contains("\"entryType\": \"retired\""));
        assert!(!packet.contains("fake review"));
    }
}
