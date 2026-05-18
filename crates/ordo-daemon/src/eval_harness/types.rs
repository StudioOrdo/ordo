use super::*;
use anyhow::{ensure, Result};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::fs;
use std::path::PathBuf;

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
    ProductSurfaceRecords,
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
            Self::ProductSurfaceRecords => "product_surface_records",
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
        crate::conversations::require_text("eval step id", &id)?;
        crate::conversations::require_text("eval step action", &action)?;
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
        crate::conversations::require_text("eval case id", &id)?;
        crate::conversations::require_text("eval case title", &title)?;
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
        crate::conversations::require_text("eval assertion id", &id)?;
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
    pub product_surface_ledger: Vec<EvalLedgerEntry>,
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
            product_surface_ledger: product_surface_ledger(connection)?,
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
