use anyhow::{anyhow, ensure, Result};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

use crate::conversations::{
    create_conversation_message, create_conversation_participant,
    find_or_create_canonical_conversation, CanonicalConversationRequest,
    ConversationMessageCreateRequest, ConversationParticipantCreateRequest,
};
use crate::entry_points::{
    create_entry_point, create_visitor_session, EntryPointStatus, EntryPointWriteRequest,
    PublicDestinationSurface, VisitorSessionCreateRequest,
};
use crate::eval_harness::{
    DeterministicEvalClock, DeterministicEvalHarness, EvalActorRole, EvalArtifactPaths,
    EvalArtifactWriter, EvalAssertion, EvalCase, EvalEvidenceChannel, EvalStep,
};
use crate::eval_personas::{load_persona_dir, EvalPersona};
use crate::llm_gateway::{
    DeterministicLlmProvider, LlmGateway, LlmGatewayRequest, LlmProviderAdapter,
    OpenAiCompatibleConfig, OpenAiCompatibleProvider, OpenAiCompatibleTransport, PromptSlot,
    ReqwestOpenAiTransport,
};
use crate::offers::{
    accept_public_offer, create_offer, OfferAcceptanceCreateRequest, OfferStatus, OfferWriteRequest,
};
use crate::policy::ActorContext;
use crate::{
    attribution::{list_attributions_for_outcome, list_outcomes_by_offer},
    business::{BusinessFactVisibility, PublicationState},
};

pub const LIVE_EVAL_RUNNER_SCHEMA_VERSION: &str = "ordo.live_eval_runner.v1";
pub const LIVE_OPENAI_SMOKE_CASE_ID: &str = "live_openai_compatible_smoke";
pub const LIVE_JOURNEY_RUNNER_SCHEMA_VERSION: &str = "ordo.live_journey_runner.v1";
pub const QR_TO_TRIAL_JOURNEY_SCHEMA_VERSION: &str = "ordo.qr_to_trial_journey_eval.v1";
pub const QR_TO_TRIAL_JOURNEY_CASE_PREFIX: &str = "live_journey_qr_to_trial";

const DEFAULT_MAX_CASES: u32 = 1;
const DEFAULT_BUDGET_MICROS: u64 = 10_000;
const ESTIMATED_CASE_COST_MICROS: u64 = 1_000;
const ESTIMATED_JOURNEY_CASE_COST_MICROS: u64 = 1_000;
const DEFAULT_OPENAI_BASE_URL: &str = "https://api.openai.com/v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LiveEvalStatus {
    Allowed,
    Skipped,
    Blocked,
    Completed,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveEvalGuardDecision {
    pub status: LiveEvalStatus,
    pub reason: String,
    pub network_enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LiveJourneyCaseStatus {
    Planned,
    Skipped,
    Blocked,
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveEvalConfig {
    pub provider_id: String,
    pub model_id: String,
    pub base_url: String,
    pub timeout_ms: u64,
    pub max_cases: u32,
    pub budget_micros: u64,
    pub api_key_configured: bool,
    #[serde(skip, default)]
    api_key: String,
}

impl fmt::Debug for LiveEvalConfig {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("LiveEvalConfig")
            .field("provider_id", &self.provider_id)
            .field("model_id", &self.model_id)
            .field("base_url", &self.base_url)
            .field("timeout_ms", &self.timeout_ms)
            .field("max_cases", &self.max_cases)
            .field("budget_micros", &self.budget_micros)
            .field("api_key_configured", &self.api_key_configured)
            .field("api_key", &"[redacted]")
            .finish()
    }
}

impl LiveEvalConfig {
    pub fn from_env() -> (LiveEvalGuardDecision, Option<Self>) {
        let values = std::env::vars().collect::<BTreeMap<_, _>>();
        Self::from_env_map(&values)
    }

    pub fn from_env_map(
        values: &BTreeMap<String, String>,
    ) -> (LiveEvalGuardDecision, Option<Self>) {
        let live_enabled = env_is_one(values, "ORDO_LIVE_LLM_EVALS");
        if !live_enabled {
            return (
                skipped("ORDO_LIVE_LLM_EVALS=1 is required for live LLM evals."),
                None,
            );
        }
        if !env_is_one(values, "ORDO_LIVE_LLM_ALLOW_NETWORK") {
            return (
                skipped("ORDO_LIVE_LLM_ALLOW_NETWORK=1 is required for live LLM evals."),
                None,
            );
        }

        let provider_id =
            env_trimmed(values, "ORDO_LIVE_LLM_PROVIDER").unwrap_or_else(|| "openai".to_string());
        if provider_id != "openai" {
            return (
                blocked(format!(
                    "unsupported live LLM provider {provider_id}; only openai is implemented"
                )),
                None,
            );
        }
        let Some(model_id) = env_trimmed(values, "ORDO_LIVE_LLM_MODEL") else {
            return (
                blocked("ORDO_LIVE_LLM_MODEL is required for live LLM evals."),
                None,
            );
        };
        let Some(api_key) = env_trimmed(values, "OPENAI_API_KEY")
            .or_else(|| env_trimmed(values, "API__OPENAI_API_KEY"))
        else {
            return (
                blocked("OPENAI_API_KEY or API__OPENAI_API_KEY is required for live LLM evals."),
                None,
            );
        };

        let max_cases = match parse_optional_u32(values, "ORDO_LIVE_LLM_MAX_CASES") {
            Ok(value) => value.unwrap_or(DEFAULT_MAX_CASES),
            Err(reason) => return (blocked(reason), None),
        };
        if max_cases == 0 {
            return (
                blocked("ORDO_LIVE_LLM_MAX_CASES must allow at least one case."),
                None,
            );
        }

        let budget_micros = match parse_optional_usd_micros(values, "ORDO_LIVE_LLM_BUDGET_USD") {
            Ok(value) => value.unwrap_or(DEFAULT_BUDGET_MICROS),
            Err(reason) => return (blocked(reason), None),
        };
        if budget_micros < ESTIMATED_CASE_COST_MICROS {
            return (
                blocked(format!(
                    "live LLM budget is below the conservative per-case estimate of {ESTIMATED_CASE_COST_MICROS} micros"
                )),
                None,
            );
        }
        if max_cases > 1 {
            return (
                blocked("Phase 6 live runner only supports one smoke case per run."),
                None,
            );
        }

        let timeout_ms = match parse_optional_u64(values, "ORDO_LIVE_LLM_TIMEOUT_MS") {
            Ok(value) => value.unwrap_or(30_000),
            Err(reason) => return (blocked(reason), None),
        };
        if timeout_ms == 0 {
            return (blocked("ORDO_LIVE_LLM_TIMEOUT_MS must be positive."), None);
        }

        (
            LiveEvalGuardDecision {
                status: LiveEvalStatus::Allowed,
                reason: "live LLM eval guards satisfied".to_string(),
                network_enabled: true,
            },
            Some(Self {
                provider_id,
                model_id,
                base_url: env_trimmed(values, "ORDO_LIVE_LLM_BASE_URL")
                    .unwrap_or_else(|| DEFAULT_OPENAI_BASE_URL.to_string()),
                timeout_ms,
                max_cases,
                budget_micros,
                api_key_configured: true,
                api_key,
            }),
        )
    }

    fn openai_config(&self) -> Result<OpenAiCompatibleConfig> {
        OpenAiCompatibleConfig::new(
            self.provider_id.clone(),
            self.model_id.clone(),
            self.base_url.clone(),
            self.api_key.clone(),
        )?
        .with_timeout_ms(self.timeout_ms)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveEvalRunSummary {
    pub schema_version: String,
    pub status: LiveEvalStatus,
    pub guard: LiveEvalGuardDecision,
    pub case_id: Option<String>,
    pub provider_id: Option<String>,
    pub model_id: Option<String>,
    pub max_cases: Option<u32>,
    pub budget_micros: Option<u64>,
    pub estimated_case_cost_micros: u64,
    pub attempted_cases: u32,
    pub completed_cases: u32,
    pub latency_ms: Option<u128>,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub packet_path: Option<String>,
    pub scorecard_path: Option<String>,
    pub manifest_path: Option<String>,
    pub message: String,
}

impl LiveEvalRunSummary {
    pub fn skipped_or_blocked(guard: LiveEvalGuardDecision) -> Self {
        Self {
            schema_version: LIVE_EVAL_RUNNER_SCHEMA_VERSION.to_string(),
            status: guard.status.clone(),
            guard,
            case_id: None,
            provider_id: None,
            model_id: None,
            max_cases: None,
            budget_micros: None,
            estimated_case_cost_micros: ESTIMATED_CASE_COST_MICROS,
            attempted_cases: 0,
            completed_cases: 0,
            latency_ms: None,
            input_tokens: 0,
            output_tokens: 0,
            packet_path: None,
            scorecard_path: None,
            manifest_path: None,
            message: "live LLM eval did not run".to_string(),
        }
    }
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveJourneyConfig {
    pub provider_id: String,
    pub model_id: String,
    pub base_url: String,
    pub timeout_ms: u64,
    pub max_cases: u32,
    pub budget_micros: u64,
    pub api_key_configured: bool,
}

impl fmt::Debug for LiveJourneyConfig {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("LiveJourneyConfig")
            .field("provider_id", &self.provider_id)
            .field("model_id", &self.model_id)
            .field("base_url", &self.base_url)
            .field("timeout_ms", &self.timeout_ms)
            .field("max_cases", &self.max_cases)
            .field("budget_micros", &self.budget_micros)
            .field("api_key_configured", &self.api_key_configured)
            .finish()
    }
}

impl LiveJourneyConfig {
    pub fn from_env_map(
        values: &BTreeMap<String, String>,
    ) -> (LiveEvalGuardDecision, Option<Self>) {
        let live_enabled = env_is_one(values, "ORDO_LIVE_LLM_EVALS");
        if !live_enabled {
            return (
                skipped("ORDO_LIVE_LLM_EVALS=1 is required for live journey evals."),
                None,
            );
        }
        if !env_is_one(values, "ORDO_LIVE_LLM_ALLOW_NETWORK") {
            return (
                skipped("ORDO_LIVE_LLM_ALLOW_NETWORK=1 is required for live journey evals."),
                None,
            );
        }

        let provider_id =
            env_trimmed(values, "ORDO_LIVE_LLM_PROVIDER").unwrap_or_else(|| "openai".to_string());
        if provider_id != "openai" {
            return (
                blocked(format!(
                    "unsupported live LLM provider {provider_id}; only openai is implemented"
                )),
                None,
            );
        }
        let Some(model_id) = env_trimmed(values, "ORDO_LIVE_LLM_MODEL") else {
            return (
                blocked("ORDO_LIVE_LLM_MODEL is required for live journey evals."),
                None,
            );
        };
        if env_trimmed(values, "OPENAI_API_KEY")
            .or_else(|| env_trimmed(values, "API__OPENAI_API_KEY"))
            .is_none()
        {
            return (
                blocked(
                    "OPENAI_API_KEY or API__OPENAI_API_KEY is required for live journey evals.",
                ),
                None,
            );
        }

        let max_cases = match parse_optional_u32(values, "ORDO_LIVE_LLM_MAX_CASES") {
            Ok(value) => value.unwrap_or(DEFAULT_MAX_CASES),
            Err(reason) => return (blocked(reason), None),
        };
        if max_cases == 0 {
            return (
                blocked("ORDO_LIVE_LLM_MAX_CASES must allow at least one case."),
                None,
            );
        }

        let budget_micros = match parse_optional_usd_micros(values, "ORDO_LIVE_LLM_BUDGET_USD") {
            Ok(value) => value.unwrap_or(DEFAULT_BUDGET_MICROS),
            Err(reason) => return (blocked(reason), None),
        };
        if budget_micros < ESTIMATED_JOURNEY_CASE_COST_MICROS {
            return (
                blocked(format!(
                    "live journey budget is below the conservative per-case estimate of {ESTIMATED_JOURNEY_CASE_COST_MICROS} micros"
                )),
                None,
            );
        }

        let timeout_ms = match parse_optional_u64(values, "ORDO_LIVE_LLM_TIMEOUT_MS") {
            Ok(value) => value.unwrap_or(30_000),
            Err(reason) => return (blocked(reason), None),
        };
        if timeout_ms == 0 {
            return (blocked("ORDO_LIVE_LLM_TIMEOUT_MS must be positive."), None);
        }

        (
            LiveEvalGuardDecision {
                status: LiveEvalStatus::Allowed,
                reason: "live journey eval guards satisfied".to_string(),
                network_enabled: true,
            },
            Some(Self {
                provider_id,
                model_id,
                base_url: env_trimmed(values, "ORDO_LIVE_LLM_BASE_URL")
                    .unwrap_or_else(|| DEFAULT_OPENAI_BASE_URL.to_string()),
                timeout_ms,
                max_cases,
                budget_micros,
                api_key_configured: true,
            }),
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveJourneyPlanRequest {
    pub persona_dir: PathBuf,
    pub selected_persona_ids: Vec<String>,
    pub output_dir: PathBuf,
    pub source_commit: String,
    pub private_terms: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlannedLiveJourneyCase {
    pub case_id: String,
    pub persona_id: String,
    pub persona_content_hash: String,
    pub person_type: String,
    pub expected_pressure_subsystems: Vec<String>,
    pub status: LiveJourneyCaseStatus,
    pub estimated_case_cost_micros: u64,
    pub note: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveJourneyBudgetSummary {
    pub max_cases: u32,
    pub selected_persona_count: usize,
    pub planned_case_count: usize,
    pub budget_micros: u64,
    pub estimated_case_cost_micros: u64,
    pub estimated_total_cost_micros: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveJourneyRunManifest {
    pub schema_version: String,
    pub source_commit: String,
    pub guard: LiveEvalGuardDecision,
    pub provider_id: Option<String>,
    pub model_id: Option<String>,
    pub persona_library_count: usize,
    pub selected_persona_ids: Vec<String>,
    pub budget: LiveJourneyBudgetSummary,
    pub planned_cases: Vec<PlannedLiveJourneyCase>,
    pub redaction_detectors: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveJourneyRunSummary {
    pub schema_version: String,
    pub status: LiveEvalStatus,
    pub guard: LiveEvalGuardDecision,
    pub provider_id: Option<String>,
    pub model_id: Option<String>,
    pub persona_library_count: usize,
    pub selected_persona_count: usize,
    pub planned_case_count: usize,
    pub budget_micros: Option<u64>,
    pub estimated_total_cost_micros: u64,
    pub manifest_path: Option<String>,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QrToTrialJourneyEvidence {
    pub persona_id: String,
    pub persona_content_hash: String,
    pub case_id: String,
    pub entry_point_id: String,
    pub entry_point_slug: String,
    pub visitor_session_id: String,
    pub conversation_id: String,
    pub visitor_message_id: String,
    pub assistant_message_id: String,
    pub offer_id: String,
    pub offer_slug: String,
    pub acceptance_id: String,
    pub trial_id: String,
    pub trial_status: String,
    pub outcome_ids: Vec<String>,
    pub attribution_count: usize,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QrToTrialJourneyManifest {
    pub schema_version: String,
    pub source_commit: String,
    pub guard: LiveEvalGuardDecision,
    pub provider_mode: String,
    pub network_enabled: bool,
    pub evidence: QrToTrialJourneyEvidence,
    pub packet_path: String,
    pub scorecard_path: String,
    pub manifest_path: String,
    pub redaction_detectors: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QrToTrialJourneyRunSummary {
    pub schema_version: String,
    pub status: LiveEvalStatus,
    pub case_id: String,
    pub persona_id: String,
    pub provider_mode: String,
    pub network_enabled: bool,
    pub entry_point_id: String,
    pub visitor_session_id: String,
    pub conversation_id: String,
    pub offer_id: String,
    pub acceptance_id: String,
    pub trial_id: String,
    pub outcome_count: usize,
    pub attribution_count: usize,
    pub packet_path: String,
    pub scorecard_path: String,
    pub manifest_path: String,
    pub journey_manifest_path: String,
    pub message: String,
}

pub fn plan_live_journey_from_env_map(
    env_values: &BTreeMap<String, String>,
    request: LiveJourneyPlanRequest,
) -> Result<LiveJourneyRunSummary> {
    let personas = load_persona_dir(&request.persona_dir, &request.private_terms)?;
    let (guard, config) = LiveJourneyConfig::from_env_map(env_values);
    let selected_personas = select_personas(&personas, &request.selected_persona_ids)?;

    match config {
        Some(config) => write_live_journey_manifest(LiveJourneyManifestInput {
            guard,
            provider_id: Some(config.provider_id),
            model_id: Some(config.model_id),
            max_cases: config.max_cases,
            budget_micros: config.budget_micros,
            persona_library_count: personas.len(),
            selected_personas,
            request,
        }),
        None => write_live_journey_manifest(LiveJourneyManifestInput {
            guard,
            provider_id: None,
            model_id: None,
            max_cases: DEFAULT_MAX_CASES,
            budget_micros: 0,
            persona_library_count: personas.len(),
            selected_personas,
            request,
        }),
    }
}

pub fn run_qr_to_trial_journey_eval(
    db_path: &Path,
    connection: &Connection,
    persona_dir: &Path,
    selected_persona_id: Option<&str>,
    output_dir: impl Into<PathBuf>,
    source_commit: impl Into<String>,
    private_terms: Vec<String>,
) -> Result<QrToTrialJourneyRunSummary> {
    let personas = load_persona_dir(persona_dir, &private_terms)?;
    let persona = select_qr_to_trial_persona(&personas, selected_persona_id)?;
    let case = qr_to_trial_journey_case(&persona)?;
    let output_dir = output_dir.into();
    let packet_path = output_dir.join(format!("{}-packet.json", case.id));
    let source_commit = source_commit.into();
    let mut harness = DeterministicEvalHarness::new(DeterministicEvalClock::fixed())
        .with_artifact_path(packet_path.to_string_lossy());
    let mut state = QrToTrialJourneyState::new(persona.clone());
    let mut scorecard = harness.run_case(connection, &case, |connection, step| {
        run_qr_to_trial_journey_step(db_path, connection, step, &mut state)
    })?;
    scorecard.provider_mode = "deterministic_live_journey".to_string();
    scorecard.network_enabled = false;
    let mut writer_private_terms = private_terms;
    writer_private_terms.push(persona.narrative_markdown.clone());
    writer_private_terms.push("Project Orchid".to_string());
    writer_private_terms.push("Project".to_string());
    writer_private_terms.push("Orchid".to_string());
    writer_private_terms.push("sk-live-journey-fixture".to_string());
    writer_private_terms.push("alex@example.com".to_string());
    let writer = EvalArtifactWriter::new(&output_dir, &source_commit)
        .with_private_terms(writer_private_terms.clone());
    let artifact_paths = writer.write_packet(connection, &case, &scorecard)?;
    let evidence = state.into_evidence(connection)?;
    ensure_qr_to_trial_evidence(&evidence)?;
    let journey_manifest_path = output_dir.join(format!("{}-journey.json", case.id));
    let manifest = QrToTrialJourneyManifest {
        schema_version: QR_TO_TRIAL_JOURNEY_SCHEMA_VERSION.to_string(),
        source_commit,
        guard: LiveEvalGuardDecision {
            status: LiveEvalStatus::Completed,
            reason: "QR-to-trial journey used deterministic provider path; live network remains guarded for later manual runs.".to_string(),
            network_enabled: false,
        },
        provider_mode: "deterministic_live_journey".to_string(),
        network_enabled: false,
        evidence: evidence.clone(),
        packet_path: artifact_paths.packet_path.to_string_lossy().to_string(),
        scorecard_path: artifact_paths.scorecard_path.to_string_lossy().to_string(),
        manifest_path: artifact_paths.manifest_path.to_string_lossy().to_string(),
        redaction_detectors: vec![
            "email".to_string(),
            "phone".to_string(),
            "auth-token-shaped".to_string(),
            "api-key-shaped".to_string(),
            "private_term".to_string(),
            "persona_narrative".to_string(),
        ],
    };
    ensure_qr_to_trial_manifest_is_safe(&manifest, &writer_private_terms)?;
    write_json(&journey_manifest_path, &manifest)?;

    Ok(QrToTrialJourneyRunSummary {
        schema_version: QR_TO_TRIAL_JOURNEY_SCHEMA_VERSION.to_string(),
        status: if scorecard.passed {
            LiveEvalStatus::Completed
        } else {
            LiveEvalStatus::Failed
        },
        case_id: case.id,
        persona_id: evidence.persona_id,
        provider_mode: "deterministic_live_journey".to_string(),
        network_enabled: false,
        entry_point_id: evidence.entry_point_id,
        visitor_session_id: evidence.visitor_session_id,
        conversation_id: evidence.conversation_id,
        offer_id: evidence.offer_id,
        acceptance_id: evidence.acceptance_id,
        trial_id: evidence.trial_id,
        outcome_count: evidence.outcome_ids.len(),
        attribution_count: evidence.attribution_count,
        packet_path: artifact_paths.packet_path.to_string_lossy().to_string(),
        scorecard_path: artifact_paths.scorecard_path.to_string_lossy().to_string(),
        manifest_path: artifact_paths.manifest_path.to_string_lossy().to_string(),
        journey_manifest_path: journey_manifest_path.to_string_lossy().to_string(),
        message: if scorecard.passed {
            "QR-to-trial journey eval completed without provider network.".to_string()
        } else {
            "QR-to-trial journey eval completed with failed assertions.".to_string()
        },
    })
}

#[derive(Debug, Clone)]
struct QrToTrialJourneyState {
    persona: EvalPersona,
    entry_point_id: Option<String>,
    entry_point_slug: Option<String>,
    visitor_session_id: Option<String>,
    conversation_id: Option<String>,
    visitor_participant_id: Option<String>,
    assistant_participant_id: Option<String>,
    visitor_message_id: Option<String>,
    assistant_message_id: Option<String>,
    offer_id: Option<String>,
    offer_slug: Option<String>,
    acceptance_id: Option<String>,
    trial_id: Option<String>,
    trial_status: Option<String>,
}

impl QrToTrialJourneyState {
    fn new(persona: EvalPersona) -> Self {
        Self {
            persona,
            entry_point_id: None,
            entry_point_slug: None,
            visitor_session_id: None,
            conversation_id: None,
            visitor_participant_id: None,
            assistant_participant_id: None,
            visitor_message_id: None,
            assistant_message_id: None,
            offer_id: None,
            offer_slug: None,
            acceptance_id: None,
            trial_id: None,
            trial_status: None,
        }
    }

    fn into_evidence(self, connection: &Connection) -> Result<QrToTrialJourneyEvidence> {
        let offer_id = required_state(self.offer_id, "offer id")?;
        let outcomes = list_outcomes_by_offer(connection, &offer_id)?;
        let attribution_count = outcomes
            .iter()
            .map(|outcome| list_attributions_for_outcome(connection, &outcome.id))
            .collect::<Result<Vec<_>>>()?
            .into_iter()
            .map(|items| items.len())
            .sum();
        Ok(QrToTrialJourneyEvidence {
            persona_id: self.persona.persona_id.clone(),
            persona_content_hash: self.persona.content_hash,
            case_id: format!(
                "{QR_TO_TRIAL_JOURNEY_CASE_PREFIX}_{}",
                self.persona.persona_id
            ),
            entry_point_id: required_state(self.entry_point_id, "entry point id")?,
            entry_point_slug: required_state(self.entry_point_slug, "entry point slug")?,
            visitor_session_id: required_state(self.visitor_session_id, "visitor session id")?,
            conversation_id: required_state(self.conversation_id, "conversation id")?,
            visitor_message_id: required_state(self.visitor_message_id, "visitor message id")?,
            assistant_message_id: required_state(
                self.assistant_message_id,
                "assistant message id",
            )?,
            offer_id,
            offer_slug: required_state(self.offer_slug, "offer slug")?,
            acceptance_id: required_state(self.acceptance_id, "acceptance id")?,
            trial_id: required_state(self.trial_id, "trial id")?,
            trial_status: required_state(self.trial_status, "trial status")?,
            outcome_ids: outcomes.into_iter().map(|outcome| outcome.id).collect(),
            attribution_count,
            evidence_refs: vec![
                "entry_point".to_string(),
                "visitor_session".to_string(),
                "conversation_message".to_string(),
                "llm_prompt_slot_usage".to_string(),
                "privacy_egress_transform".to_string(),
                "offer_acceptance".to_string(),
                "trial".to_string(),
                "business_outcome".to_string(),
                "business_outcome_attribution".to_string(),
            ],
        })
    }
}

fn select_qr_to_trial_persona(
    personas: &[EvalPersona],
    selected_persona_id: Option<&str>,
) -> Result<EvalPersona> {
    match selected_persona_id {
        Some(id) => personas
            .iter()
            .find(|persona| persona.persona_id == id)
            .cloned()
            .ok_or_else(|| anyhow!("unknown QR-to-trial persona id {id}")),
        None => personas
            .first()
            .cloned()
            .ok_or_else(|| anyhow!("persona library is empty")),
    }
}

fn qr_to_trial_journey_case(persona: &EvalPersona) -> Result<EvalCase> {
    EvalCase::new(
        format!("{QR_TO_TRIAL_JOURNEY_CASE_PREFIX}_{}", persona.persona_id),
        "QR event to 30-day trial journey",
        &json!({
            "fixture": "qr_to_trial_journey",
            "version": 1,
            "personaId": persona.persona_id,
            "personaHash": persona.content_hash,
            "providerMode": "deterministic_live_journey",
            "networkRequired": false,
            "deferredPhases": ["review_return", "affiliate_referral", "admin_staff_handoff", "cross_persona_report"],
        }),
        vec![
            EvalActorRole::AnonymousVisitor,
            EvalActorRole::OrdoAgent,
            EvalActorRole::LlmToolProviderBoundary,
        ],
        vec![EvalStep::new(
            "run_qr_event_to_trial_acceptance",
            EvalActorRole::AnonymousVisitor,
            "live_journey.qr_to_trial",
            vec![
                EvalEvidenceChannel::SqliteRows,
                EvalEvidenceChannel::ConversationEvents,
                EvalEvidenceChannel::RealtimeReplay,
                EvalEvidenceChannel::PolicyDecisions,
                EvalEvidenceChannel::PromptSlotAccounting,
                EvalEvidenceChannel::PrivacyTransforms,
                EvalEvidenceChannel::TokenLedger,
            ],
        )?],
        vec![
            EvalAssertion::minimum_count(
                "durable_sqlite_rows_recorded",
                EvalEvidenceChannel::SqliteRows,
                20,
            )?,
            EvalAssertion::minimum_count(
                "conversation_events_recorded",
                EvalEvidenceChannel::ConversationEvents,
                8,
            )?,
            EvalAssertion::minimum_count(
                "realtime_replay_recorded",
                EvalEvidenceChannel::RealtimeReplay,
                8,
            )?,
            EvalAssertion::minimum_count(
                "llm_policy_decision_recorded",
                EvalEvidenceChannel::PolicyDecisions,
                1,
            )?,
            EvalAssertion::minimum_count(
                "prompt_slot_accounting_recorded",
                EvalEvidenceChannel::PromptSlotAccounting,
                1,
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
        ],
    )
}

fn run_qr_to_trial_journey_step(
    db_path: &Path,
    connection: &Connection,
    step: &EvalStep,
    state: &mut QrToTrialJourneyState,
) -> Result<()> {
    match step.id.as_str() {
        "run_qr_event_to_trial_acceptance" => {
            let offer_slug = format!(
                "ordostudio-30-day-{}",
                slug_fragment(&state.persona.persona_id)
            );
            let (offer, _) = create_offer(
                db_path,
                OfferWriteRequest {
                    slug: offer_slug.clone(),
                    title: "OrdoStudio 30-day trial".to_string(),
                    summary:
                        "A 30-day Studio Ordo trial for relationship-first business operations."
                            .to_string(),
                    status: Some(OfferStatus::Available),
                    visibility: Some(BusinessFactVisibility::Public),
                    publication_state: Some(PublicationState::Published),
                    trial_days: Some(30),
                    source_kind: Some("live_journey_eval".to_string()),
                    source_ref: Some(state.persona.persona_id.clone()),
                    terms: Some(json!({
                        "trialDays": 30,
                        "billing": "manual_follow_up",
                        "claims": "evidence_backed_only",
                        "noFakeScarcity": true,
                    })),
                    metadata: Some(json!({
                        "personaId": state.persona.persona_id,
                        "personaHash": state.persona.content_hash,
                    })),
                },
                None,
            )?;
            state.offer_id = Some(offer.id.clone());
            state.offer_slug = Some(offer.slug.clone());

            let entry_slug = format!("event-qr-{}", slug_fragment(&state.persona.persona_id));
            let (entry_point, _) = create_entry_point(
                db_path,
                EntryPointWriteRequest {
                    slug: entry_slug.clone(),
                    label: "Studio Ordo event QR".to_string(),
                    status: Some(EntryPointStatus::Active),
                    source_kind: "event_qr".to_string(),
                    source_label: Some("Live journey eval event".to_string()),
                    destination_surface: PublicDestinationSurface::Offers,
                    destination_id: Some(offer.id.clone()),
                    attribution: Some(json!({
                        "campaign": "live_product_journey_eval",
                        "personaId": state.persona.persona_id,
                        "source": "event_qr",
                    })),
                    metadata: Some(json!({
                        "evalCase": "qr_to_trial",
                        "personaHash": state.persona.content_hash,
                    })),
                },
                None,
            )?;
            state.entry_point_id = Some(entry_point.id.clone());
            state.entry_point_slug = Some(entry_point.slug.clone());

            let (visitor_session, _) = create_visitor_session(
                db_path,
                VisitorSessionCreateRequest {
                    entry_point_slug: entry_point.slug.clone(),
                    user_agent: Some("Ordo live journey eval mobile browser".to_string()),
                    attribution: Some(json!({
                        "personaId": state.persona.persona_id,
                        "entryPointId": entry_point.id,
                    })),
                },
            )?;
            state.visitor_session_id = Some(visitor_session.id.clone());

            let conversation = find_or_create_canonical_conversation(
                connection,
                &CanonicalConversationRequest {
                    surface: "chat".to_string(),
                    subject_kind: "visitor_session".to_string(),
                    subject_id: visitor_session.id.clone(),
                    connection_id: None,
                    visitor_session_id: Some(visitor_session.id.clone()),
                    created_by_actor_id: None,
                },
            )?;
            state.conversation_id = Some(conversation.id.clone());
            let visitor = create_conversation_participant(
                connection,
                &ConversationParticipantCreateRequest {
                    conversation_id: conversation.id.clone(),
                    participant_kind: "visitor".to_string(),
                    actor_id: None,
                    connection_id: None,
                    visitor_session_id: Some(visitor_session.id.clone()),
                    display_name: state.persona.display_name.clone(),
                    role: "prospective_client".to_string(),
                },
            )?;
            state.visitor_participant_id = Some(visitor.id.clone());
            let assistant = create_conversation_participant(
                connection,
                &ConversationParticipantCreateRequest {
                    conversation_id: conversation.id.clone(),
                    participant_kind: "agent".to_string(),
                    actor_id: None,
                    connection_id: None,
                    visitor_session_id: None,
                    display_name: "Ordo".to_string(),
                    role: "assistant".to_string(),
                },
            )?;
            state.assistant_participant_id = Some(assistant.id.clone());
            let visitor_message = create_conversation_message(
                connection,
                &ConversationMessageCreateRequest {
                    conversation_id: conversation.id.clone(),
                    segment_id: None,
                    participant_id: visitor.id.clone(),
                    message_kind: "message".to_string(),
                    body_markdown: persona_backed_visitor_message(&state.persona),
                    visibility: "participants".to_string(),
                    client_message_id: format!("client-message-{}", state.persona.persona_id),
                    reply_to_message_id: None,
                    undo_expires_at: None,
                },
            )?;
            state.visitor_message_id = Some(visitor_message.id.clone());

            let gateway = LlmGateway::new(DeterministicLlmProvider::new("local_fake", "fake-chat"))
                .with_private_terms(vec![
                    "Project Orchid".to_string(),
                    "Project".to_string(),
                    "Orchid".to_string(),
                    "sk-live-journey-fixture".to_string(),
                    "alex@example.com".to_string(),
                ]);
            let llm_result = gateway.run_completion(
                db_path,
                connection,
                &ActorContext::local_owner("live_journey_qr_to_trial_eval"),
                LlmGatewayRequest {
                    run_id: format!("live_journey_qr_to_trial_{}", state.persona.persona_id),
                    conversation_id: conversation.id.clone(),
                    segment_id: None,
                    assistant_participant_id: assistant.id.clone(),
                    client_id: Some(format!("qr-to-trial-{}", state.persona.persona_id)),
                    provider_id: "local_fake".to_string(),
                    model_id: "fake-chat".to_string(),
                    user_message: visitor_message.body_markdown.clone(),
                    prompt_slots: vec![
                        PromptSlot::new(
                            "ethical_business_persuasion",
                            "Ethical Business Persuasion",
                            "Use evidence-backed reciprocity and commitment only where supported; preserve agency; do not use fake urgency, scarcity, reviews, metrics, shame, fear, or hidden pressure.",
                            vec![
                                format!("entry_point:{}", entry_point.id),
                                format!("offer:{}", offer.id),
                                format!("message:{}", visitor_message.id),
                            ],
                            "The journey evaluates respectful signup guidance for a 30-day trial.",
                            "staff_private",
                        )?,
                        PromptSlot::new(
                            "offer_trial_context",
                            "Offer Trial Context",
                            "Studio Ordo has a public 30-day trial offer; recommend it only as an option the visitor can decline.",
                            vec![
                                format!("offer:{}", offer.id),
                                format!("visitor_session:{}", visitor_session.id),
                            ],
                            "Durable offer and visitor-session evidence for the QR journey.",
                            "participants",
                        )?,
                    ],
                },
            )?;
            let assistant_message = llm_result
                .final_message
                .ok_or_else(|| anyhow!("deterministic QR-to-trial LLM path produced no message"))?;
            state.assistant_message_id = Some(assistant_message.id.clone());

            let (acceptance, trial, _) = accept_public_offer(
                db_path,
                &offer.slug,
                OfferAcceptanceCreateRequest {
                    visitor_session_id: Some(visitor_session.id.clone()),
                    attribution: Some(json!({
                        "personaId": state.persona.persona_id,
                        "conversationId": conversation.id,
                        "visitorMessageId": visitor_message.id,
                        "assistantMessageId": assistant_message.id,
                        "entryPointId": entry_point.id,
                    })),
                    acceptance_context: Some(json!({
                        "decision": "accepted_30_day_trial",
                        "agencyPreserving": true,
                        "evidenceRefs": [
                            format!("entry_point:{}", entry_point.id),
                            format!("visitor_session:{}", visitor_session.id),
                            format!("conversation:{}", conversation.id),
                            format!("message:{}", visitor_message.id),
                            format!("message:{}", assistant_message.id),
                            format!("offer:{}", offer.id)
                        ],
                        "nonGoals": [
                            "no_fake_urgency",
                            "no_fake_scarcity",
                            "no_unsupported_social_proof"
                        ]
                    })),
                },
            )?;
            state.acceptance_id = Some(acceptance.id);
            state.trial_id = Some(trial.id);
            state.trial_status = Some(format!("{:?}", trial.status));
        }
        other => anyhow::bail!("unsupported QR-to-trial journey step: {other}"),
    }
    Ok(())
}

fn ensure_qr_to_trial_evidence(evidence: &QrToTrialJourneyEvidence) -> Result<()> {
    ensure!(
        !evidence.entry_point_id.is_empty(),
        "entry point evidence missing"
    );
    ensure!(
        !evidence.visitor_session_id.is_empty(),
        "visitor session evidence missing"
    );
    ensure!(
        !evidence.conversation_id.is_empty(),
        "conversation evidence missing"
    );
    ensure!(!evidence.offer_id.is_empty(), "offer evidence missing");
    ensure!(
        !evidence.acceptance_id.is_empty(),
        "acceptance evidence missing"
    );
    ensure!(!evidence.trial_id.is_empty(), "trial evidence missing");
    ensure!(
        evidence.trial_status == "Started",
        "QR-to-trial eval should create a started 30-day trial"
    );
    ensure!(
        !evidence.outcome_ids.is_empty(),
        "business outcome evidence missing"
    );
    ensure!(
        evidence.attribution_count >= 3,
        "offer, visitor session, and entry point attribution evidence required"
    );
    Ok(())
}

fn ensure_qr_to_trial_manifest_is_safe(
    manifest: &QrToTrialJourneyManifest,
    private_terms: &[String],
) -> Result<()> {
    let value = serde_json::to_value(manifest)?;
    ensure!(
        !contains_sensitive_value(&value, private_terms),
        "QR-to-trial journey manifest contains raw sensitive value"
    );
    Ok(())
}

fn persona_backed_visitor_message(persona: &EvalPersona) -> String {
    format!(
        "I scanned your event QR code. I run a {} practice and I am considering whether a 30-day OrdoStudio trial fits. My budget sensitivity is {}, my urgency is {}, and I want a plain recommendation without fake scarcity or hype. Please do not repeat Project Orchid, alex@example.com, or sk-live-journey-fixture.",
        persona.person_type, persona.budget_sensitivity, persona.urgency_level
    )
}

fn slug_fragment(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>()
}

fn required_state(value: Option<String>, label: &str) -> Result<String> {
    value.ok_or_else(|| anyhow!("QR-to-trial journey missing {label}"))
}

fn write_json(path: &Path, value: &impl Serialize) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, serde_json::to_string_pretty(value)?)?;
    Ok(())
}

struct LiveJourneyManifestInput {
    guard: LiveEvalGuardDecision,
    provider_id: Option<String>,
    model_id: Option<String>,
    max_cases: u32,
    budget_micros: u64,
    persona_library_count: usize,
    selected_personas: Vec<EvalPersona>,
    request: LiveJourneyPlanRequest,
}

fn write_live_journey_manifest(input: LiveJourneyManifestInput) -> Result<LiveJourneyRunSummary> {
    let capped_personas = input
        .selected_personas
        .iter()
        .take(input.max_cases as usize)
        .cloned()
        .collect::<Vec<_>>();
    let estimated_total_cost_micros =
        ESTIMATED_JOURNEY_CASE_COST_MICROS.saturating_mul(capped_personas.len() as u64);

    let mut manifest_guard = input.guard;
    if manifest_guard.status == LiveEvalStatus::Allowed
        && input.budget_micros < estimated_total_cost_micros
    {
        manifest_guard = blocked(format!(
            "live journey budget would be exceeded before execution: estimated {estimated_total_cost_micros} micros for {} cases with budget {} micros",
            capped_personas.len(),
            input.budget_micros
        ));
    }

    let planned_cases = capped_personas
        .iter()
        .map(|persona| planned_case_for_persona(persona, &manifest_guard))
        .collect::<Vec<_>>();
    let selected_persona_ids = input
        .selected_personas
        .iter()
        .map(|persona| persona.persona_id.clone())
        .collect::<Vec<_>>();

    let manifest = LiveJourneyRunManifest {
        schema_version: LIVE_JOURNEY_RUNNER_SCHEMA_VERSION.to_string(),
        source_commit: input.request.source_commit,
        guard: manifest_guard.clone(),
        provider_id: input.provider_id.clone(),
        model_id: input.model_id.clone(),
        persona_library_count: input.persona_library_count,
        selected_persona_ids,
        budget: LiveJourneyBudgetSummary {
            max_cases: input.max_cases,
            selected_persona_count: input.selected_personas.len(),
            planned_case_count: planned_cases.len(),
            budget_micros: input.budget_micros,
            estimated_case_cost_micros: ESTIMATED_JOURNEY_CASE_COST_MICROS,
            estimated_total_cost_micros,
        },
        planned_cases,
        redaction_detectors: vec![
            "email".to_string(),
            "phone".to_string(),
            "auth-token-shaped".to_string(),
            "api-key-shaped".to_string(),
            "private_term".to_string(),
        ],
    };

    ensure_manifest_is_safe(&manifest, &input.request.private_terms)?;
    fs::create_dir_all(&input.request.output_dir)?;
    let manifest_path = input.request.output_dir.join("live-journey-manifest.json");
    let encoded = serde_json::to_string_pretty(&manifest)?;
    fs::write(&manifest_path, encoded)?;

    Ok(LiveJourneyRunSummary {
        schema_version: LIVE_JOURNEY_RUNNER_SCHEMA_VERSION.to_string(),
        status: manifest_guard.status.clone(),
        guard: manifest_guard,
        provider_id: input.provider_id,
        model_id: input.model_id,
        persona_library_count: input.persona_library_count,
        selected_persona_count: manifest.budget.selected_persona_count,
        planned_case_count: manifest.budget.planned_case_count,
        budget_micros: Some(input.budget_micros),
        estimated_total_cost_micros,
        manifest_path: Some(manifest_path.to_string_lossy().to_string()),
        message: match manifest.status_label() {
            "allowed" => "live journey cases planned; execution remains deferred to later phases",
            "blocked" => "live journey planning blocked before provider execution",
            "skipped" => "live journey planning skipped before provider execution",
            _ => "live journey planning did not execute provider work",
        }
        .to_string(),
    })
}

impl LiveJourneyRunManifest {
    fn status_label(&self) -> &'static str {
        match self.guard.status {
            LiveEvalStatus::Allowed => "allowed",
            LiveEvalStatus::Blocked => "blocked",
            LiveEvalStatus::Skipped => "skipped",
            LiveEvalStatus::Completed | LiveEvalStatus::Failed => "terminal",
        }
    }
}

fn select_personas(personas: &[EvalPersona], selected_ids: &[String]) -> Result<Vec<EvalPersona>> {
    if selected_ids.is_empty() {
        return Ok(personas.to_vec());
    }

    let by_id = personas
        .iter()
        .map(|persona| (persona.persona_id.as_str(), persona))
        .collect::<BTreeMap<_, _>>();
    let mut selected = Vec::new();
    for id in selected_ids {
        let Some(persona) = by_id.get(id.as_str()) else {
            return Err(anyhow!("unknown live journey persona id {id}"));
        };
        selected.push((*persona).clone());
    }
    Ok(selected)
}

fn planned_case_for_persona(
    persona: &EvalPersona,
    guard: &LiveEvalGuardDecision,
) -> PlannedLiveJourneyCase {
    let case_status = match guard.status {
        LiveEvalStatus::Allowed => LiveJourneyCaseStatus::Planned,
        LiveEvalStatus::Skipped => LiveJourneyCaseStatus::Skipped,
        LiveEvalStatus::Blocked | LiveEvalStatus::Completed | LiveEvalStatus::Failed => {
            LiveJourneyCaseStatus::Blocked
        }
    };
    PlannedLiveJourneyCase {
        case_id: format!("live_journey_{}", persona.persona_id),
        persona_id: persona.persona_id.clone(),
        persona_content_hash: persona.content_hash.clone(),
        person_type: persona.person_type.clone(),
        expected_pressure_subsystems: persona
            .expected_eval_pressure_subsystems
            .iter()
            .map(|subsystem| subsystem.as_str().to_string())
            .collect(),
        status: case_status,
        estimated_case_cost_micros: ESTIMATED_JOURNEY_CASE_COST_MICROS,
        note: "Planning only; QR-to-trial execution uses the separate #165 journey runner."
            .to_string(),
    }
}

fn ensure_manifest_is_safe(
    manifest: &LiveJourneyRunManifest,
    private_terms: &[String],
) -> Result<()> {
    let value = serde_json::to_value(manifest)?;
    ensure!(
        !contains_sensitive_value(&value, private_terms),
        "live journey manifest contains raw sensitive value"
    );
    Ok(())
}

pub fn run_live_openai_eval_from_env(
    db_path: &Path,
    connection: &Connection,
    output_dir: impl Into<PathBuf>,
    source_commit: impl Into<String>,
) -> Result<LiveEvalRunSummary> {
    let (guard, config) = LiveEvalConfig::from_env();
    let Some(config) = config else {
        return Ok(LiveEvalRunSummary::skipped_or_blocked(guard));
    };
    run_live_openai_eval_with_transport(
        db_path,
        connection,
        config,
        ReqwestOpenAiTransport,
        output_dir,
        source_commit,
    )
}

pub fn run_live_openai_eval_with_transport<T: OpenAiCompatibleTransport>(
    db_path: &Path,
    connection: &Connection,
    config: LiveEvalConfig,
    transport: T,
    output_dir: impl Into<PathBuf>,
    source_commit: impl Into<String>,
) -> Result<LiveEvalRunSummary> {
    ensure!(
        config.max_cases >= 1,
        "live eval config must allow at least one case"
    );
    ensure!(
        config.budget_micros >= ESTIMATED_CASE_COST_MICROS,
        "live eval budget is below conservative estimate"
    );
    let guard = LiveEvalGuardDecision {
        status: LiveEvalStatus::Allowed,
        reason: "live LLM eval guards satisfied".to_string(),
        network_enabled: true,
    };
    let start = Instant::now();
    let case = live_openai_compatible_smoke_case()?;
    let packet_path = output_dir
        .into()
        .join(format!("{LIVE_OPENAI_SMOKE_CASE_ID}-packet.json"));
    let provider = OpenAiCompatibleProvider::with_transport(config.openai_config()?, transport);
    let mut harness = DeterministicEvalHarness::new(DeterministicEvalClock::fixed())
        .with_artifact_path(packet_path.to_string_lossy());
    let mut scorecard = harness.run_case(connection, &case, |connection, step| {
        run_live_openai_compatible_smoke_step(db_path, connection, step, &provider)
    })?;
    scorecard.provider_mode = "live_openai_compatible".to_string();
    scorecard.network_enabled = true;
    let output_dir = packet_path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    let writer = EvalArtifactWriter::new(output_dir, source_commit).with_private_terms(vec![
        "Project Orchid".to_string(),
        "sk-live-fixture".to_string(),
    ]);
    let artifact_paths = writer.write_packet(connection, &case, &scorecard)?;
    let (input_tokens, output_tokens) =
        token_usage_for_invocation(connection, "live_eval_openai_smoke_run")?;
    Ok(completed_summary(
        guard,
        &config,
        scorecard.passed,
        start.elapsed().as_millis(),
        input_tokens,
        output_tokens,
        artifact_paths,
    ))
}

fn completed_summary(
    guard: LiveEvalGuardDecision,
    config: &LiveEvalConfig,
    passed: bool,
    latency_ms: u128,
    input_tokens: i64,
    output_tokens: i64,
    artifact_paths: EvalArtifactPaths,
) -> LiveEvalRunSummary {
    LiveEvalRunSummary {
        schema_version: LIVE_EVAL_RUNNER_SCHEMA_VERSION.to_string(),
        status: if passed {
            LiveEvalStatus::Completed
        } else {
            LiveEvalStatus::Failed
        },
        guard,
        case_id: Some(LIVE_OPENAI_SMOKE_CASE_ID.to_string()),
        provider_id: Some(config.provider_id.clone()),
        model_id: Some(config.model_id.clone()),
        max_cases: Some(config.max_cases),
        budget_micros: Some(config.budget_micros),
        estimated_case_cost_micros: ESTIMATED_CASE_COST_MICROS,
        attempted_cases: 1,
        completed_cases: if passed { 1 } else { 0 },
        latency_ms: Some(latency_ms),
        input_tokens,
        output_tokens,
        packet_path: Some(artifact_paths.packet_path.to_string_lossy().to_string()),
        scorecard_path: Some(artifact_paths.scorecard_path.to_string_lossy().to_string()),
        manifest_path: Some(artifact_paths.manifest_path.to_string_lossy().to_string()),
        message: if passed {
            "live OpenAI-compatible smoke eval completed".to_string()
        } else {
            "live OpenAI-compatible smoke eval completed with failed assertions".to_string()
        },
    }
}

fn live_openai_compatible_smoke_case() -> Result<EvalCase> {
    EvalCase::new(
        LIVE_OPENAI_SMOKE_CASE_ID,
        "Live OpenAI-compatible smoke",
        &json!({
            "fixture": LIVE_OPENAI_SMOKE_CASE_ID,
            "version": 1,
            "providerMode": "live_openai_compatible",
            "networkRequired": true,
            "estimatedCaseCostMicros": ESTIMATED_CASE_COST_MICROS,
        }),
        vec![
            EvalActorRole::Staff,
            EvalActorRole::OrdoAgent,
            EvalActorRole::LlmToolProviderBoundary,
        ],
        vec![EvalStep::new(
            "run_live_openai_compatible_completion",
            EvalActorRole::LlmToolProviderBoundary,
            "llm.run.request.live_openai_compatible",
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
                1,
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

fn run_live_openai_compatible_smoke_step<T: OpenAiCompatibleTransport>(
    db_path: &Path,
    connection: &Connection,
    step: &EvalStep,
    provider: &OpenAiCompatibleProvider<T>,
) -> Result<()> {
    match step.id.as_str() {
        "run_live_openai_compatible_completion" => {
            let (conversation_id, assistant_id) = live_eval_conversation_and_assistant(connection)?;
            let gateway = LlmGateway::new(provider.clone())
                .with_private_terms(vec!["Project Orchid".to_string()]);
            let result = gateway.run_completion(
                db_path,
                connection,
                &ActorContext::local_owner("live_eval_runner"),
                LlmGatewayRequest {
                    run_id: "live_eval_openai_smoke_run".to_string(),
                    conversation_id,
                    segment_id: None,
                    assistant_participant_id: assistant_id,
                    client_id: Some("live-eval-openai-smoke-1".to_string()),
                    provider_id: provider.provider_id().to_string(),
                    model_id: provider.model_id().to_string(),
                    user_message: "Write one short, respectful next-step candidate for Project Orchid. Contact alex@example.com. sk-live-fixture".to_string(),
                    prompt_slots: vec![PromptSlot::new(
                        "conversation_brief",
                        "Conversation Brief",
                        "Evidence: client asked for a concise next step. Do not invent facts.",
                        vec!["live_eval:evidence:conversation_brief".to_string()],
                        "Tiny live eval smoke evidence.",
                        "participants",
                    )?],
                },
            )?;
            ensure!(
                result.final_message.is_some(),
                "live eval provider did not produce a final assistant candidate"
            );
        }
        other => anyhow::bail!("unsupported live eval step: {other}"),
    }
    Ok(())
}

fn live_eval_conversation_and_assistant(connection: &Connection) -> Result<(String, String)> {
    let conversation = find_or_create_canonical_conversation(
        connection,
        &CanonicalConversationRequest {
            surface: "chat".to_string(),
            subject_kind: "connection".to_string(),
            subject_id: "connection_eval_1".to_string(),
            connection_id: Some("connection_eval_1".to_string()),
            visitor_session_id: Some("visitor_session_eval_1".to_string()),
            created_by_actor_id: Some("actor_staff_eval_1".to_string()),
        },
    )?;
    let assistant = create_conversation_participant(
        connection,
        &ConversationParticipantCreateRequest {
            conversation_id: conversation.id.clone(),
            participant_kind: "agent".to_string(),
            actor_id: None,
            connection_id: None,
            visitor_session_id: None,
            display_name: "Ordo".to_string(),
            role: "assistant".to_string(),
        },
    )?;
    Ok((conversation.id, assistant.id))
}

fn token_usage_for_invocation(connection: &Connection, invocation_id: &str) -> Result<(i64, i64)> {
    let input_tokens = token_usage_for_kind(connection, invocation_id, "provider_input")?;
    let output_tokens = token_usage_for_kind(connection, invocation_id, "provider_output")?;
    Ok((input_tokens, output_tokens))
}

fn token_usage_for_kind(
    connection: &Connection,
    invocation_id: &str,
    usage_kind: &str,
) -> Result<i64> {
    Ok(connection.query_row(
        "SELECT COALESCE(SUM(token_count), 0)
         FROM llm_token_ledger_entries
         WHERE invocation_id = ?1 AND usage_kind = ?2",
        params![invocation_id, usage_kind],
        |row| row.get(0),
    )?)
}

fn skipped(reason: impl Into<String>) -> LiveEvalGuardDecision {
    LiveEvalGuardDecision {
        status: LiveEvalStatus::Skipped,
        reason: reason.into(),
        network_enabled: false,
    }
}

fn blocked(reason: impl Into<String>) -> LiveEvalGuardDecision {
    LiveEvalGuardDecision {
        status: LiveEvalStatus::Blocked,
        reason: reason.into(),
        network_enabled: false,
    }
}

fn env_is_one(values: &BTreeMap<String, String>, key: &str) -> bool {
    values
        .get(key)
        .map(|value| value.trim() == "1")
        .unwrap_or(false)
}

fn env_trimmed(values: &BTreeMap<String, String>, key: &str) -> Option<String> {
    values
        .get(key)
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn parse_optional_u32(values: &BTreeMap<String, String>, key: &str) -> Result<Option<u32>, String> {
    let Some(raw) = env_trimmed(values, key) else {
        return Ok(None);
    };
    raw.parse::<u32>()
        .map(Some)
        .map_err(|_| format!("{key} must be a positive integer"))
}

fn parse_optional_u64(values: &BTreeMap<String, String>, key: &str) -> Result<Option<u64>, String> {
    let Some(raw) = env_trimmed(values, key) else {
        return Ok(None);
    };
    raw.parse::<u64>()
        .map(Some)
        .map_err(|_| format!("{key} must be a positive integer"))
}

fn parse_optional_usd_micros(
    values: &BTreeMap<String, String>,
    key: &str,
) -> Result<Option<u64>, String> {
    let Some(raw) = env_trimmed(values, key) else {
        return Ok(None);
    };
    parse_usd_micros(&raw)
        .map(Some)
        .ok_or_else(|| format!("{key} must be a non-negative decimal USD amount"))
}

fn parse_usd_micros(raw: &str) -> Option<u64> {
    let trimmed = raw.trim();
    if trimmed.is_empty() || trimmed.starts_with('-') {
        return None;
    }
    let (dollars, fraction) = trimmed.split_once('.').unwrap_or((trimmed, ""));
    let dollar_micros = dollars.parse::<u64>().ok()?.checked_mul(1_000_000)?;
    let mut fraction_digits = fraction
        .chars()
        .take(6)
        .filter(|character| character.is_ascii_digit())
        .collect::<String>();
    if fraction_digits.len() != fraction.chars().take(6).count() {
        return None;
    }
    while fraction_digits.len() < 6 {
        fraction_digits.push('0');
    }
    let fraction_micros = if fraction_digits.is_empty() {
        0
    } else {
        fraction_digits.parse::<u64>().ok()?
    };
    dollar_micros.checked_add(fraction_micros)
}

fn contains_sensitive_value(value: &Value, private_terms: &[String]) -> bool {
    match value {
        Value::String(text) => text_contains_sensitive_value(text, private_terms),
        Value::Array(items) => items
            .iter()
            .any(|item| contains_sensitive_value(item, private_terms)),
        Value::Object(map) => map
            .values()
            .any(|item| contains_sensitive_value(item, private_terms)),
        _ => false,
    }
}

fn text_contains_sensitive_value(text: &str, private_terms: &[String]) -> bool {
    let lower = text.to_ascii_lowercase();
    if private_terms.iter().any(|term| {
        let term = term.trim().to_ascii_lowercase();
        !term.is_empty() && lower.contains(&term)
    }) {
        return true;
    }
    text.split_whitespace().any(|token| {
        let trimmed = token.trim_matches(|character: char| {
            matches!(
                character,
                '"' | '\''
                    | ','
                    | '.'
                    | ';'
                    | ':'
                    | '{'
                    | '}'
                    | '['
                    | ']'
                    | '('
                    | ')'
                    | '<'
                    | '>'
                    | '!'
            )
        });
        looks_like_email(trimmed) || looks_like_phone(trimmed) || looks_like_secret(trimmed)
    })
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capabilities::seed_builtin_capabilities;
    use crate::eval_harness::{collect_evidence_snapshot, isolated_eval_connection};
    use crate::llm_gateway::OpenAiTransportResponse;
    use crate::schema::init_schema;
    use serde_json::{json, Value};
    use std::cell::RefCell;
    use std::fs;
    use std::rc::Rc;

    #[derive(Clone)]
    struct CountingOpenAiTransport {
        calls: Rc<RefCell<u32>>,
    }

    impl CountingOpenAiTransport {
        fn new() -> Self {
            Self {
                calls: Rc::new(RefCell::new(0)),
            }
        }

        fn calls(&self) -> u32 {
            *self.calls.borrow()
        }
    }

    impl OpenAiCompatibleTransport for CountingOpenAiTransport {
        fn post_chat_completions(
            &self,
            _endpoint: &str,
            _api_key: &str,
            _timeout_ms: u64,
            _body: &Value,
        ) -> Result<OpenAiTransportResponse> {
            *self.calls.borrow_mut() += 1;
            Ok(OpenAiTransportResponse {
                status: 200,
                body: json!({
                    "choices": [
                        { "message": { "content": "Here is a concise candidate next step." } }
                    ],
                    "usage": {
                        "prompt_tokens": 19,
                        "completion_tokens": 8
                    }
                }),
            })
        }
    }

    fn allowed_env() -> BTreeMap<String, String> {
        BTreeMap::from([
            ("ORDO_LIVE_LLM_EVALS".to_string(), "1".to_string()),
            ("ORDO_LIVE_LLM_ALLOW_NETWORK".to_string(), "1".to_string()),
            ("ORDO_LIVE_LLM_PROVIDER".to_string(), "openai".to_string()),
            ("ORDO_LIVE_LLM_MODEL".to_string(), "gpt-test".to_string()),
            (
                "ORDO_LIVE_LLM_BASE_URL".to_string(),
                "https://api.openai.test/v1".to_string(),
            ),
            (
                "OPENAI_API_KEY".to_string(),
                "sk-live-secret-value".to_string(),
            ),
            ("ORDO_LIVE_LLM_MAX_CASES".to_string(), "1".to_string()),
            ("ORDO_LIVE_LLM_BUDGET_USD".to_string(), "0.01".to_string()),
        ])
    }

    fn personas_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join("docs/evals/personas")
    }

    fn journey_request(
        output_dir: &Path,
        selected_persona_ids: Vec<String>,
    ) -> LiveJourneyPlanRequest {
        LiveJourneyPlanRequest {
            persona_dir: personas_dir(),
            selected_persona_ids,
            output_dir: output_dir.to_path_buf(),
            source_commit: "test-commit".to_string(),
            private_terms: vec![
                "Project Orchid".to_string(),
                "sk-live-secret-value".to_string(),
            ],
        }
    }

    fn file_backed_eval_store() -> (tempfile::NamedTempFile, Connection) {
        let db_file = tempfile::NamedTempFile::new().unwrap();
        let connection = Connection::open(db_file.path()).unwrap();
        init_schema(&connection).unwrap();
        seed_builtin_capabilities(&connection).unwrap();
        (db_file, connection)
    }

    #[test]
    fn live_eval_guards_skip_without_live_or_network_flags() {
        let empty = BTreeMap::new();
        let (decision, config) = LiveEvalConfig::from_env_map(&empty);
        assert_eq!(decision.status, LiveEvalStatus::Skipped);
        assert!(!decision.network_enabled);
        assert!(config.is_none());

        let mut missing_network = allowed_env();
        missing_network.remove("ORDO_LIVE_LLM_ALLOW_NETWORK");
        let (decision, config) = LiveEvalConfig::from_env_map(&missing_network);
        assert_eq!(decision.status, LiveEvalStatus::Skipped);
        assert!(!decision.network_enabled);
        assert!(config.is_none());
    }

    #[test]
    fn live_eval_guards_block_missing_key_malformed_caps_and_budget_overrun() {
        let mut missing_key = allowed_env();
        missing_key.remove("OPENAI_API_KEY");
        let (decision, config) = LiveEvalConfig::from_env_map(&missing_key);
        assert_eq!(decision.status, LiveEvalStatus::Blocked);
        assert!(decision.reason.contains("OPENAI_API_KEY"));
        assert!(config.is_none());

        let mut malformed_cases = allowed_env();
        malformed_cases.insert("ORDO_LIVE_LLM_MAX_CASES".to_string(), "many".to_string());
        let (decision, config) = LiveEvalConfig::from_env_map(&malformed_cases);
        assert_eq!(decision.status, LiveEvalStatus::Blocked);
        assert!(decision.reason.contains("MAX_CASES"));
        assert!(config.is_none());

        let mut too_many_cases = allowed_env();
        too_many_cases.insert("ORDO_LIVE_LLM_MAX_CASES".to_string(), "2".to_string());
        let (decision, config) = LiveEvalConfig::from_env_map(&too_many_cases);
        assert_eq!(decision.status, LiveEvalStatus::Blocked);
        assert!(decision.reason.contains("one smoke case"));
        assert!(config.is_none());

        let mut budget_overrun = allowed_env();
        budget_overrun.insert("ORDO_LIVE_LLM_BUDGET_USD".to_string(), "0.0001".to_string());
        let (decision, config) = LiveEvalConfig::from_env_map(&budget_overrun);
        assert_eq!(decision.status, LiveEvalStatus::Blocked);
        assert!(decision.reason.contains("budget"));
        assert!(config.is_none());
    }

    #[test]
    fn allowed_live_eval_path_runs_with_mock_transport_and_writes_redacted_artifacts() {
        let (decision, config) = LiveEvalConfig::from_env_map(&allowed_env());
        assert_eq!(decision.status, LiveEvalStatus::Allowed);
        let config = config.unwrap();
        assert!(!format!("{config:?}").contains("sk-live-secret-value"));
        assert!(format!("{config:?}").contains("[redacted]"));
        let transport = CountingOpenAiTransport::new();
        let db_path = tempfile::NamedTempFile::new().unwrap();
        let connection = isolated_eval_connection().unwrap();
        let temp_dir = tempfile::tempdir().unwrap();

        let summary = run_live_openai_eval_with_transport(
            db_path.path(),
            &connection,
            config,
            transport.clone(),
            temp_dir.path(),
            "test-commit",
        )
        .unwrap();

        assert_eq!(summary.status, LiveEvalStatus::Completed);
        assert_eq!(summary.case_id.as_deref(), Some(LIVE_OPENAI_SMOKE_CASE_ID));
        assert_eq!(summary.input_tokens, 19);
        assert_eq!(summary.output_tokens, 8);
        assert_eq!(transport.calls(), 1);

        let packet_path = summary.packet_path.as_ref().unwrap();
        let scorecard_path = summary.scorecard_path.as_ref().unwrap();
        let manifest_path = summary.manifest_path.as_ref().unwrap();
        assert!(Path::new(packet_path).exists());
        assert!(Path::new(scorecard_path).exists());
        assert!(Path::new(manifest_path).exists());

        let packet_json = fs::read_to_string(packet_path).unwrap();
        assert!(packet_json.contains("\"networkEnabled\": true"));
        assert!(packet_json.contains("live_openai_compatible"));
        assert!(!packet_json.contains("sk-live-secret-value"));
        assert!(!packet_json.contains("sk-live-fixture"));
        assert!(!packet_json.contains("alex@example.com"));
        assert!(!packet_json.contains("Project Orchid"));
        assert!(packet_json.contains("__ORDO_PRIVATE_"));

        let snapshot = collect_evidence_snapshot(&connection, "now".to_string()).unwrap();
        assert!(snapshot.count_for(EvalEvidenceChannel::TokenLedger) >= 2);
    }

    #[test]
    fn skipped_summary_does_not_attempt_provider_call() {
        let summary = LiveEvalRunSummary::skipped_or_blocked(skipped("missing guard"));
        assert_eq!(summary.status, LiveEvalStatus::Skipped);
        assert_eq!(summary.attempted_cases, 0);
        assert!(!summary.guard.network_enabled);
        assert!(summary.packet_path.is_none());
    }

    #[test]
    fn live_journey_planner_loads_personas_and_limits_by_max_cases() {
        let temp_dir = tempfile::tempdir().unwrap();
        let mut env = allowed_env();
        env.insert("ORDO_LIVE_LLM_MAX_CASES".to_string(), "3".to_string());
        env.insert("ORDO_LIVE_LLM_BUDGET_USD".to_string(), "0.01".to_string());
        let summary =
            plan_live_journey_from_env_map(&env, journey_request(temp_dir.path(), Vec::new()))
                .unwrap();

        assert_eq!(summary.status, LiveEvalStatus::Allowed);
        assert_eq!(summary.persona_library_count, 10);
        assert_eq!(summary.selected_persona_count, 10);
        assert_eq!(summary.planned_case_count, 3);
        assert_eq!(summary.estimated_total_cost_micros, 3_000);

        let manifest_path = summary.manifest_path.as_ref().unwrap();
        let manifest_json = fs::read_to_string(manifest_path).unwrap();
        let manifest: LiveJourneyRunManifest = serde_json::from_str(&manifest_json).unwrap();
        assert_eq!(manifest.planned_cases.len(), 3);
        assert_eq!(
            manifest.planned_cases[0].persona_id,
            "affiliate_referrer_community"
        );
        assert_eq!(
            manifest.planned_cases[0].status,
            LiveJourneyCaseStatus::Planned
        );
        assert!(manifest.planned_cases[0]
            .case_id
            .starts_with("live_journey_"));
        assert!(manifest.planned_cases[0]
            .persona_content_hash
            .starts_with("sha256:"));
        assert_eq!(manifest.provider_id.as_deref(), Some("openai"));
        assert_eq!(manifest.model_id.as_deref(), Some("gpt-test"));
        assert!(!manifest_json.contains("sk-live-secret-value"));
        assert!(!manifest_json.contains("Project Orchid"));
        assert!(!manifest_json.contains("Maya is used to software demos"));
    }

    #[test]
    fn live_journey_planner_skips_without_guards_without_network() {
        let temp_dir = tempfile::tempdir().unwrap();
        let summary = plan_live_journey_from_env_map(
            &BTreeMap::new(),
            journey_request(temp_dir.path(), Vec::new()),
        )
        .unwrap();

        assert_eq!(summary.status, LiveEvalStatus::Skipped);
        assert!(!summary.guard.network_enabled);
        assert_eq!(summary.persona_library_count, 10);
        assert_eq!(summary.planned_case_count, 1);
        let manifest_json = fs::read_to_string(summary.manifest_path.unwrap()).unwrap();
        let manifest: LiveJourneyRunManifest = serde_json::from_str(&manifest_json).unwrap();
        assert_eq!(
            manifest.planned_cases[0].status,
            LiveJourneyCaseStatus::Skipped
        );
        assert!(manifest.provider_id.is_none());
    }

    #[test]
    fn live_journey_planner_rejects_unknown_persona_id() {
        let temp_dir = tempfile::tempdir().unwrap();
        let error = plan_live_journey_from_env_map(
            &allowed_env(),
            journey_request(temp_dir.path(), vec!["missing_persona".to_string()]),
        )
        .unwrap_err();
        assert!(error
            .to_string()
            .contains("unknown live journey persona id missing_persona"));
    }

    #[test]
    fn live_journey_planner_blocks_budget_overrun_before_execution() {
        let temp_dir = tempfile::tempdir().unwrap();
        let mut env = allowed_env();
        env.insert("ORDO_LIVE_LLM_MAX_CASES".to_string(), "3".to_string());
        env.insert("ORDO_LIVE_LLM_BUDGET_USD".to_string(), "0.002".to_string());
        let summary =
            plan_live_journey_from_env_map(&env, journey_request(temp_dir.path(), Vec::new()))
                .unwrap();

        assert_eq!(summary.status, LiveEvalStatus::Blocked);
        assert!(summary.guard.reason.contains("budget would be exceeded"));
        assert_eq!(summary.planned_case_count, 3);
        let manifest_json = fs::read_to_string(summary.manifest_path.unwrap()).unwrap();
        let manifest: LiveJourneyRunManifest = serde_json::from_str(&manifest_json).unwrap();
        assert!(manifest
            .planned_cases
            .iter()
            .all(|case| case.status == LiveJourneyCaseStatus::Blocked));
    }

    #[test]
    fn live_journey_planner_respects_explicit_persona_selection_order() {
        let temp_dir = tempfile::tempdir().unwrap();
        let mut env = allowed_env();
        env.insert("ORDO_LIVE_LLM_MAX_CASES".to_string(), "5".to_string());
        let selected = vec![
            "solo_consultant_followup".to_string(),
            "agency_operator_pipeline".to_string(),
        ];
        let summary = plan_live_journey_from_env_map(
            &env,
            journey_request(temp_dir.path(), selected.clone()),
        )
        .unwrap();

        assert_eq!(summary.status, LiveEvalStatus::Allowed);
        assert_eq!(summary.selected_persona_count, 2);
        assert_eq!(summary.planned_case_count, 2);
        let manifest_json = fs::read_to_string(summary.manifest_path.unwrap()).unwrap();
        let manifest: LiveJourneyRunManifest = serde_json::from_str(&manifest_json).unwrap();
        assert_eq!(manifest.selected_persona_ids, selected);
        assert_eq!(
            manifest.planned_cases[0].persona_id,
            "solo_consultant_followup"
        );
        assert_eq!(
            manifest.planned_cases[1].persona_id,
            "agency_operator_pipeline"
        );
    }

    #[test]
    fn qr_to_trial_journey_eval_creates_entry_session_offer_trial_and_attribution() {
        let (db_file, connection) = file_backed_eval_store();
        let temp_dir = tempfile::tempdir().unwrap();

        let summary = run_qr_to_trial_journey_eval(
            db_file.path(),
            &connection,
            &personas_dir(),
            Some("solo_consultant_followup"),
            temp_dir.path(),
            "test-commit",
            vec![
                "Project Orchid".to_string(),
                "sk-live-journey-fixture".to_string(),
                "alex@example.com".to_string(),
            ],
        )
        .unwrap();

        assert_eq!(summary.status, LiveEvalStatus::Completed);
        assert_eq!(summary.persona_id, "solo_consultant_followup");
        assert_eq!(summary.provider_mode, "deterministic_live_journey");
        assert!(!summary.network_enabled);
        assert!(summary
            .case_id
            .starts_with("live_journey_qr_to_trial_solo_consultant_followup"));
        assert!(!summary.entry_point_id.is_empty());
        assert!(!summary.visitor_session_id.is_empty());
        assert!(!summary.conversation_id.is_empty());
        assert!(!summary.offer_id.is_empty());
        assert!(!summary.acceptance_id.is_empty());
        assert!(!summary.trial_id.is_empty());
        assert_eq!(summary.outcome_count, 1);
        assert!(summary.attribution_count >= 3);
        assert!(Path::new(&summary.packet_path).exists());
        assert!(Path::new(&summary.scorecard_path).exists());
        assert!(Path::new(&summary.manifest_path).exists());
        assert!(Path::new(&summary.journey_manifest_path).exists());

        assert_eq!(table_count(&connection, "tracked_entry_points"), 1);
        assert_eq!(table_count(&connection, "visitor_sessions"), 1);
        assert_eq!(table_count(&connection, "offer_acceptances"), 1);
        assert_eq!(table_count(&connection, "trials"), 1);
        assert_eq!(table_count(&connection, "business_outcomes"), 1);
        assert!(table_count(&connection, "business_outcome_attributions") >= 3);
        assert!(table_count(&connection, "llm_prompt_slot_usage") >= 2);
        assert!(
            connection
                .query_row(
                    "SELECT COUNT(*) FROM conversation_events WHERE event_type = 'privacy.egress.transformed'",
                    [],
                    |row| row.get::<_, i64>(0)
                )
                .unwrap()
                >= 1
        );
        assert!(table_count(&connection, "llm_token_ledger_entries") >= 2);

        let packet_json = fs::read_to_string(&summary.packet_path).unwrap();
        let journey_json = fs::read_to_string(&summary.journey_manifest_path).unwrap();
        assert!(packet_json.contains("ethical_business_persuasion"));
        assert!(packet_json.contains("offer.accepted"));
        assert!(journey_json.contains("\"trialStatus\": \"Started\""));
        assert!(journey_json.contains("\"networkEnabled\": false"));
        assert!(!packet_json.contains("sk-live-journey-fixture"));
        assert!(!packet_json.contains("alex@example.com"));
        assert!(!packet_json.contains("Project Orchid"));
        assert!(!journey_json.contains("sk-live-journey-fixture"));
        assert!(!journey_json.contains("alex@example.com"));
        assert!(!journey_json.contains("Project Orchid"));
    }

    #[test]
    fn qr_to_trial_journey_eval_rejects_unknown_persona_without_provider_or_network() {
        let (db_file, connection) = file_backed_eval_store();
        let temp_dir = tempfile::tempdir().unwrap();

        let error = run_qr_to_trial_journey_eval(
            db_file.path(),
            &connection,
            &personas_dir(),
            Some("not_a_persona"),
            temp_dir.path(),
            "test-commit",
            Vec::new(),
        )
        .unwrap_err();

        assert!(error
            .to_string()
            .contains("unknown QR-to-trial persona id not_a_persona"));
        assert_eq!(table_count(&connection, "tracked_entry_points"), 0);
        assert_eq!(table_count(&connection, "llm_invocations"), 0);
    }

    fn table_count(connection: &Connection, table: &str) -> i64 {
        connection
            .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                row.get(0)
            })
            .unwrap()
    }
}
