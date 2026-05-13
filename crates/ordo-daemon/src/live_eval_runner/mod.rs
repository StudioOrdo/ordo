use crate::artifacts::*;
use crate::attribution::*;
use crate::business::*;
use crate::connections::*;
use crate::conversations::*;
use crate::entry_points::*;
use crate::eval_harness::*;
use crate::eval_personas::*;
use crate::feedback::*;
use crate::llm_gateway::*;
use crate::offers::*;
use crate::policy::*;
use crate::secrets::{normalize_secret, OrdoSecretString};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;
use std::path::PathBuf;

pub mod admin_staff;
pub mod affiliate_referral;
pub mod planning;
pub mod qr_to_trial;
pub mod review_return;
pub mod utils;

pub use admin_staff::*;
pub use affiliate_referral::*;
pub use planning::*;
pub use qr_to_trial::*;
pub use review_return::*;
pub(crate) use utils::*;

pub const LIVE_EVAL_RUNNER_SCHEMA_VERSION: &str = "ordo.live_eval_runner.v1";
pub const LIVE_OPENAI_SMOKE_CASE_ID: &str = "live_openai_compatible_smoke";
pub const LIVE_JOURNEY_RUNNER_SCHEMA_VERSION: &str = "ordo.live_journey_runner.v1";
pub const QR_TO_TRIAL_JOURNEY_SCHEMA_VERSION: &str = "ordo.qr_to_trial_journey_eval.v1";
pub const QR_TO_TRIAL_JOURNEY_CASE_PREFIX: &str = "live_journey_qr_to_trial";
pub const REVIEW_RETURN_JOURNEY_SCHEMA_VERSION: &str = "ordo.review_return_journey_eval.v1";
pub const REVIEW_RETURN_JOURNEY_CASE_PREFIX: &str = "live_journey_review_return";
pub const AFFILIATE_REFERRAL_JOURNEY_SCHEMA_VERSION: &str =
    "ordo.affiliate_referral_journey_eval.v1";
pub const AFFILIATE_REFERRAL_JOURNEY_CASE_PREFIX: &str = "live_journey_affiliate_referral";
pub const ADMIN_STAFF_JOURNEY_SCHEMA_VERSION: &str = "ordo.admin_staff_journey_eval.v1";
pub const ADMIN_STAFF_JOURNEY_CASE_PREFIX: &str = "live_journey_admin_staff";

const DEFAULT_MAX_CASES: u32 = 1;
const DEFAULT_BUDGET_MICROS: u64 = 10_000;
const ESTIMATED_CASE_COST_MICROS: u64 = 1_000;
const ESTIMATED_JOURNEY_CASE_COST_MICROS: u64 = 1_000;
const DEFAULT_OPENAI_BASE_URL: &str = "https://api.openai.com/v1";
const REVIEW_RETURN_ELAPSED_DAYS: i64 = 4;

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

#[derive(Clone, Serialize, Deserialize)]
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
    api_key: OrdoSecretString,
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
            .and_then(normalize_secret)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capabilities::seed_builtin_capabilities;
    use crate::eval_harness::{collect_evidence_snapshot, isolated_eval_connection};
    use crate::llm_gateway::OpenAiTransportResponse;
    use crate::schema::init_schema;
    use rusqlite::Connection;
    use serde_json::{json, Value};
    use std::cell::RefCell;
    use std::fs;
    use std::path::Path;
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
        assert!(!format!("{:?}", config.api_key).contains("sk-live-secret-value"));
        assert!(format!("{:?}", config.api_key).contains("[REDACTED]"));
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

    #[test]
    fn review_return_journey_eval_records_simulated_email_feedback_and_review_lifecycle() {
        let (db_file, connection) = file_backed_eval_store();
        let temp_dir = tempfile::tempdir().unwrap();

        let summary = run_review_return_journey_eval(
            db_file.path(),
            &connection,
            &personas_dir(),
            Some("solo_consultant_followup"),
            temp_dir.path(),
            "test-commit",
            vec![
                "Project Orchid".to_string(),
                "sk-live-journey-fixture".to_string(),
                "review-return-secret".to_string(),
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
            .starts_with("live_journey_review_return_solo_consultant_followup"));
        assert!(!summary.simulated_email_artifact_id.is_empty());
        assert!(!summary.return_entry_point_id.is_empty());
        assert!(!summary.return_visitor_session_id.is_empty());
        assert!(!summary.conversation_id.is_empty());
        assert!(!summary.feedback_id.is_empty());
        assert!(!summary.review_id.is_empty());
        assert_eq!(summary.final_review_status, "retired");
        assert!(Path::new(&summary.packet_path).exists());
        assert!(Path::new(&summary.scorecard_path).exists());
        assert!(Path::new(&summary.manifest_path).exists());
        assert!(Path::new(&summary.journey_manifest_path).exists());

        assert_eq!(table_count(&connection, "artifacts"), 1);
        assert_eq!(table_count(&connection, "customer_feedback"), 1);
        assert_eq!(table_count(&connection, "customer_reviews"), 1);
        assert_eq!(table_count(&connection, "tracked_entry_points"), 2);
        assert_eq!(table_count(&connection, "visitor_sessions"), 2);
        assert_eq!(table_count(&connection, "offer_acceptances"), 1);
        assert_eq!(table_count(&connection, "trials"), 1);
        assert!(table_count(&connection, "llm_prompt_slot_usage") >= 3);
        assert!(table_count(&connection, "llm_token_ledger_entries") >= 4);
        assert_eq!(list_public_reviews(&connection).unwrap().len(), 0);

        let journey_json = fs::read_to_string(&summary.journey_manifest_path).unwrap();
        assert!(journey_json.contains("\"schemaVersion\": \"ordo.review_return_journey_eval.v1\""));
        assert!(journey_json.contains("\"networkEnabled\": false"));
        assert!(journey_json.contains("\"elapsedDaysSimulated\": 4"));
        assert!(journey_json.contains("\"blockedPublishWithoutConsentOrApproval\": true"));
        assert!(journey_json.contains("\"publicReviewCountBeforePublish\": 0"));
        assert!(journey_json.contains("\"publicReviewCountAfterPublish\": 1"));
        assert!(journey_json.contains("\"publicReviewCountAfterRetire\": 0"));
        assert!(!journey_json.contains("sk-live-journey-fixture"));
        assert!(!journey_json.contains("review-return-secret"));
        assert!(!journey_json.contains("alex@example.com"));
        assert!(!journey_json.contains("Project Orchid"));

        let packet_json = fs::read_to_string(&summary.packet_path).unwrap();
        assert!(packet_json.contains("simulated_review_request_email"));
        assert!(packet_json.contains("simulated_not_delivered"));
        assert!(packet_json.contains("feedback.item.created"));
        assert!(packet_json.contains("review.published"));
        assert!(packet_json.contains("review.retired"));
        assert!(!packet_json.contains("sk-live-journey-fixture"));
        assert!(!packet_json.contains("review-return-secret"));
        assert!(!packet_json.contains("alex@example.com"));
        assert!(!packet_json.contains("Project Orchid"));
    }

    #[test]
    fn review_return_journey_eval_rejects_unknown_persona_without_provider_or_network() {
        let (db_file, connection) = file_backed_eval_store();
        let temp_dir = tempfile::tempdir().unwrap();

        let error = run_review_return_journey_eval(
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
        assert_eq!(table_count(&connection, "artifacts"), 0);
        assert_eq!(table_count(&connection, "customer_feedback"), 0);
        assert_eq!(table_count(&connection, "llm_invocations"), 0);
    }

    #[test]
    fn affiliate_referral_journey_eval_records_scoped_referral_attribution_and_boundaries() {
        let (db_file, connection) = file_backed_eval_store();
        let temp_dir = tempfile::tempdir().unwrap();

        let summary = run_affiliate_referral_journey_eval(
            db_file.path(),
            &connection,
            &personas_dir(),
            Some("affiliate_referrer_community"),
            temp_dir.path(),
            "test-commit",
            vec![
                "Project Orchid".to_string(),
                "sk-live-journey-fixture".to_string(),
                "affiliate-referral-secret".to_string(),
                "alex@example.com".to_string(),
            ],
        )
        .unwrap();

        assert_eq!(summary.status, LiveEvalStatus::Completed);
        assert_eq!(summary.persona_id, "affiliate_referrer_community");
        assert_eq!(summary.provider_mode, "deterministic_live_journey");
        assert!(!summary.network_enabled);
        assert!(summary
            .case_id
            .starts_with("live_journey_affiliate_referral_affiliate_referrer_community"));
        assert!(!summary.affiliate_connection_id.is_empty());
        assert!(!summary.referral_entry_point_id.is_empty());
        assert!(!summary.referred_visitor_session_id.is_empty());
        assert!(!summary.conversation_id.is_empty());
        assert!(!summary.offer_id.is_empty());
        assert!(!summary.acceptance_id.is_empty());
        assert!(!summary.trial_id.is_empty());
        assert!(!summary.referral_id.is_empty());
        assert!(!summary.referral_outcome_id.is_empty());
        assert!(summary.attribution_count >= 7);
        assert!(Path::new(&summary.packet_path).exists());
        assert!(Path::new(&summary.scorecard_path).exists());
        assert!(Path::new(&summary.manifest_path).exists());
        assert!(Path::new(&summary.journey_manifest_path).exists());

        assert_eq!(table_count(&connection, "connections"), 1);
        assert_eq!(table_count(&connection, "connection_grants"), 1);
        assert_eq!(table_count(&connection, "tracked_entry_points"), 1);
        assert_eq!(table_count(&connection, "visitor_sessions"), 1);
        assert_eq!(table_count(&connection, "offer_acceptances"), 1);
        assert_eq!(table_count(&connection, "trials"), 1);
        assert_eq!(table_count(&connection, "referral_records"), 1);
        assert_eq!(table_count(&connection, "business_outcomes"), 2);
        assert!(table_count(&connection, "business_outcome_attributions") >= 7);
        assert!(table_count(&connection, "policy_decisions") >= 3);
        assert!(table_count(&connection, "llm_prompt_slot_usage") >= 2);
        assert!(table_count(&connection, "llm_token_ledger_entries") >= 2);

        let referral_outcome_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM business_outcomes
                 WHERE referral_id = ?1 AND connection_id = ?2",
                [&summary.referral_id, &summary.affiliate_connection_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(referral_outcome_count, 1);
        let referral_attribution_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM business_outcome_attributions
                 WHERE attribution_kind = 'referral' AND source_id = ?1",
                [&summary.referral_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(referral_attribution_count, 2);
        let denied_policy_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM policy_decisions
                 WHERE outcome = 'denied' AND resource_id = 'conversation_unrelated_client'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(denied_policy_count, 1);

        let journey_json = fs::read_to_string(&summary.journey_manifest_path).unwrap();
        assert!(
            journey_json.contains("\"schemaVersion\": \"ordo.affiliate_referral_journey_eval.v1\"")
        );
        assert!(journey_json.contains("\"networkEnabled\": false"));
        assert!(journey_json.contains("\"affiliateAllowedConversationRead\": true"));
        assert!(journey_json.contains("\"affiliateDeniedUnrelatedConversationRead\": true"));
        assert!(journey_json.contains("referral_record"));
        assert!(!journey_json.contains("sk-live-journey-fixture"));
        assert!(!journey_json.contains("affiliate-referral-secret"));
        assert!(!journey_json.contains("alex@example.com"));
        assert!(!journey_json.contains("Project Orchid"));

        let packet_json = fs::read_to_string(&summary.packet_path).unwrap();
        assert!(packet_json.contains("affiliate_referral"));
        assert!(packet_json.contains("referral.captured"));
        assert!(packet_json.contains("business.attribution.proposed"));
        assert!(packet_json.contains("connection.grant.created"));
        assert!(!packet_json.contains("sk-live-journey-fixture"));
        assert!(!packet_json.contains("affiliate-referral-secret"));
        assert!(!packet_json.contains("alex@example.com"));
        assert!(!packet_json.contains("Project Orchid"));
    }

    #[test]
    fn affiliate_referral_journey_eval_rejects_unknown_persona_without_provider_or_network() {
        let (db_file, connection) = file_backed_eval_store();
        let temp_dir = tempfile::tempdir().unwrap();

        let error = run_affiliate_referral_journey_eval(
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
            .contains("unknown affiliate referral persona id not_a_persona"));
        assert_eq!(table_count(&connection, "connections"), 0);
        assert_eq!(table_count(&connection, "referral_records"), 0);
        assert_eq!(table_count(&connection, "llm_invocations"), 0);
    }

    #[test]
    fn admin_staff_journey_eval_records_handoff_moderation_and_affiliate_boundaries() {
        let (db_file, connection) = file_backed_eval_store();
        let temp_dir = tempfile::tempdir().unwrap();

        let summary = run_admin_staff_journey_eval(
            db_file.path(),
            &connection,
            &personas_dir(),
            Some("dissatisfied_trial_user"),
            temp_dir.path(),
            "test-commit",
            vec![
                "Project Orchid".to_string(),
                "sk-live-journey-fixture".to_string(),
                "admin-staff-secret".to_string(),
                "alex@example.com".to_string(),
            ],
        )
        .unwrap();

        assert_eq!(summary.status, LiveEvalStatus::Completed);
        assert_eq!(summary.persona_id, "dissatisfied_trial_user");
        assert_eq!(summary.provider_mode, "deterministic_live_journey");
        assert!(!summary.network_enabled);
        assert!(summary
            .case_id
            .starts_with("live_journey_admin_staff_dissatisfied_trial_user"));
        assert_eq!(summary.final_handoff_status, "closed");
        assert!(!summary.conversation_id.is_empty());
        assert!(!summary.handoff_id.is_empty());
        assert!(!summary.review_id.is_empty());
        assert!(!summary.affiliate_connection_id.is_empty());
        assert!(Path::new(&summary.packet_path).exists());
        assert!(Path::new(&summary.scorecard_path).exists());
        assert!(Path::new(&summary.manifest_path).exists());
        assert!(Path::new(&summary.journey_manifest_path).exists());

        assert_eq!(table_count(&connection, "conversation_handoffs"), 1);
        assert_eq!(table_count(&connection, "conversation_modes"), 1);
        assert_eq!(table_count(&connection, "customer_feedback"), 1);
        assert_eq!(table_count(&connection, "customer_reviews"), 1);
        assert_eq!(table_count(&connection, "connections"), 1);
        assert_eq!(table_count(&connection, "connection_grants"), 1);
        assert!(table_count(&connection, "policy_decisions") >= 3);
        assert!(table_count(&connection, "conversation_events") >= 14);
        assert!(table_count(&connection, "realtime_events") >= 14);

        let revoked_grants: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM connection_grants
                 WHERE status = 'revoked' AND connection_id = ?1",
                [&summary.affiliate_connection_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(revoked_grants, 1);
        let denied_after_revoke: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM policy_decisions
                 WHERE outcome = 'denied' AND request_id LIKE 'affiliate-management-denied-%'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(denied_after_revoke, 1);

        let journey_json = fs::read_to_string(&summary.journey_manifest_path).unwrap();
        assert!(journey_json.contains("\"schemaVersion\": \"ordo.admin_staff_journey_eval.v1\""));
        assert!(journey_json.contains("\"networkEnabled\": false"));
        assert!(journey_json.contains("\"humanLedBlockedPublicAgentPost\": true"));
        assert!(journey_json.contains("\"delegatedAllowsPublicAgentPost\": true"));
        assert!(journey_json.contains("\"returnedModeAllowsPublicAgentPost\": true"));
        assert!(journey_json.contains("\"reviewPublicCountBeforeApproval\": 0"));
        assert!(journey_json.contains("\"affiliateAllowedBeforeRevoke\": true"));
        assert!(journey_json.contains("\"affiliateDeniedAfterRevoke\": true"));
        assert!(journey_json.contains("review_moderation"));
        assert!(journey_json.contains("affiliate_connection"));
        assert!(!journey_json.contains("sk-live-journey-fixture"));
        assert!(!journey_json.contains("admin-staff-secret"));
        assert!(!journey_json.contains("alex@example.com"));
        assert!(!journey_json.contains("Project Orchid"));

        let packet_json = fs::read_to_string(&summary.packet_path).unwrap();
        assert!(packet_json.contains("conversation.handoff.requested"));
        assert!(packet_json.contains("conversation.handoff.closed"));
        assert!(packet_json.contains("conversation.mode.changed"));
        assert!(packet_json.contains("review.published"));
        assert!(packet_json.contains("connection.grant.revoked"));
        assert!(!packet_json.contains("sk-live-journey-fixture"));
        assert!(!packet_json.contains("admin-staff-secret"));
        assert!(!packet_json.contains("alex@example.com"));
        assert!(!packet_json.contains("Project Orchid"));
    }

    #[test]
    fn admin_staff_journey_eval_rejects_unknown_persona_without_provider_or_network() {
        let (db_file, connection) = file_backed_eval_store();
        let temp_dir = tempfile::tempdir().unwrap();

        let error = run_admin_staff_journey_eval(
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
            .contains("unknown admin/staff persona id not_a_persona"));
        assert_eq!(table_count(&connection, "conversation_handoffs"), 0);
        assert_eq!(table_count(&connection, "customer_feedback"), 0);
        assert_eq!(table_count(&connection, "connections"), 0);
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
