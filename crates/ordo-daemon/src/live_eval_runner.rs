use anyhow::{ensure, Result};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::BTreeMap;
use std::fmt;
use std::path::{Path, PathBuf};
use std::time::Instant;

use crate::conversations::{
    create_conversation_participant, find_or_create_canonical_conversation,
    CanonicalConversationRequest, ConversationParticipantCreateRequest,
};
use crate::eval_harness::{
    DeterministicEvalClock, DeterministicEvalHarness, EvalActorRole, EvalArtifactPaths,
    EvalArtifactWriter, EvalAssertion, EvalCase, EvalEvidenceChannel, EvalStep,
};
use crate::llm_gateway::{
    LlmGateway, LlmGatewayRequest, LlmProviderAdapter, OpenAiCompatibleConfig,
    OpenAiCompatibleProvider, OpenAiCompatibleTransport, PromptSlot, ReqwestOpenAiTransport,
};
use crate::policy::ActorContext;

pub const LIVE_EVAL_RUNNER_SCHEMA_VERSION: &str = "ordo.live_eval_runner.v1";
pub const LIVE_OPENAI_SMOKE_CASE_ID: &str = "live_openai_compatible_smoke";

const DEFAULT_MAX_CASES: u32 = 1;
const DEFAULT_BUDGET_MICROS: u64 = 10_000;
const ESTIMATED_CASE_COST_MICROS: u64 = 1_000;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::eval_harness::{collect_evidence_snapshot, isolated_eval_connection};
    use crate::llm_gateway::OpenAiTransportResponse;
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
}
