use anyhow::{bail, Result};
use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

use crate::events::{append_realtime_event_tx, system_event, RealtimeEvent};
use crate::vault::{ensure_vault_key, store_secret};

const INSTALL_STATE_ID: &str = "local";
const OWNER_ID: &str = "owner_local";
const BUSINESS_PROFILE_ID: &str = "business_local";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallStateResponse {
    pub installed: bool,
    pub completed_at: Option<String>,
    pub owner: Option<OwnerProfile>,
    pub business: Option<BusinessProfile>,
    pub provider_boundary: ProviderBoundarySummary,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OwnerProfile {
    pub id: String,
    pub display_name: String,
    pub email: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BusinessProfile {
    pub id: String,
    pub name: String,
    pub workspace_label: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderBoundarySummary {
    pub configured: bool,
    pub default_provider_id: Option<String>,
    pub enabled_provider_count: usize,
    pub missing_secret_provider_ids: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompleteInstallRequest {
    pub owner_display_name: String,
    pub owner_email: Option<String>,
    pub business_name: String,
    pub workspace_label: Option<String>,
    pub default_provider_id: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderListResponse {
    pub readiness: ProviderReadinessSummary,
    pub providers: Vec<ProviderConfigView>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderReadinessSummary {
    pub configured_provider_mode: String,
    pub requested_provider_id: Option<String>,
    pub default_provider_id: Option<String>,
    pub live_mode_requested: bool,
    pub live_invocation_enabled: bool,
    pub live_invocation_guard: String,
    pub credentials_present: bool,
    pub credential_source: String,
    pub missing_credential_provider_ids: Vec<String>,
    pub openai: OpenAiProviderReadiness,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenAiProviderReadiness {
    pub provider_id: String,
    pub decision: String,
    pub model_id: Option<String>,
    pub model_source: String,
    pub base_url: String,
    pub base_url_source: String,
    pub timeout_ms: Option<u64>,
    pub timeout_guard: String,
    pub budget_micros: Option<u64>,
    pub budget_guard: String,
    pub max_cases: Option<u32>,
    pub api_key_configured: bool,
    pub api_key_source: String,
    pub live_eval_guard: String,
    pub network_guard: String,
    pub live_invocation_guard: String,
    pub ready_for_guarded_smoke: bool,
    pub reasons: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderConfigView {
    pub provider_id: String,
    pub provider_name: String,
    pub enabled: bool,
    pub default_provider: bool,
    pub model: Option<String>,
    pub available_models: Vec<ProviderModelOption>,
    pub base_url: Option<String>,
    pub non_secret_config: Value,
    pub api_key: RedactedSecretField,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderModelOption {
    pub id: String,
    pub label: String,
    pub default: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RedactedSecretField {
    pub configured: bool,
    pub source: String,
    pub locked: bool,
    pub redacted: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderUpdateRequest {
    pub provider_name: Option<String>,
    pub enabled: Option<bool>,
    pub default_provider: Option<bool>,
    pub model: Option<String>,
    pub base_url: Option<String>,
    pub api_key: Option<String>,
    pub non_secret_config: Option<Value>,
}

#[derive(Debug, Clone)]
struct ProviderCatalogEntry {
    provider_id: &'static str,
    provider_name: &'static str,
    api_key_env_keys: &'static [&'static str],
    default_model: Option<&'static str>,
    available_models: &'static [&'static str],
    default_base_url: Option<&'static str>,
}

const PROVIDER_CATALOG: &[ProviderCatalogEntry] = &[
    ProviderCatalogEntry {
        provider_id: "anthropic",
        provider_name: "Anthropic",
        api_key_env_keys: &["ANTHROPIC_API_KEY", "API__ANTHROPIC_API_KEY"],
        default_model: Some("claude-haiku-4-5"),
        available_models: &["claude-haiku-4-5", "claude-sonnet-4-6"],
        default_base_url: Some("https://api.anthropic.com/v1"),
    },
    ProviderCatalogEntry {
        provider_id: "openai",
        provider_name: "OpenAI",
        api_key_env_keys: &["OPENAI_API_KEY", "API__OPENAI_API_KEY"],
        default_model: Some("gpt-5"),
        available_models: &["gpt-5"],
        default_base_url: Some("https://api.openai.com/v1"),
    },
    ProviderCatalogEntry {
        provider_id: "deepseek",
        provider_name: "DeepSeek",
        api_key_env_keys: &["DEEPSEEK_API_KEY", "API__DEEPSEEK_API_KEY", "deepseek"],
        default_model: Some("deepseek-v4-flash"),
        available_models: &["deepseek-v4-flash", "deepseek-v4-pro"],
        default_base_url: Some("https://api.deepseek.com/v1"),
    },
    ProviderCatalogEntry {
        provider_id: "local",
        provider_name: "Local Ollama",
        api_key_env_keys: &[],
        default_model: Some("qwen2.5-coder:7b"),
        available_models: &["qwen2.5-coder:7b", "gemma3:12b"],
        default_base_url: Some("http://127.0.0.1:11434/api"),
    },
];

#[derive(Debug, Clone)]
struct ProviderRecord {
    provider_id: String,
    provider_name: String,
    enabled: bool,
    default_provider: bool,
    model: Option<String>,
    base_url: Option<String>,
    secret_ref: Option<String>,
    non_secret_config: Value,
    created_at: String,
    updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ResolvedSecret {
    configured: bool,
    source: &'static str,
    locked: bool,
}

pub fn read_install_state(db_path: &Path) -> Result<InstallStateResponse> {
    let connection = Connection::open(db_path)?;
    read_install_state_connection(&connection)
}

pub fn complete_local_install(
    db_path: &Path,
    request: CompleteInstallRequest,
) -> Result<(InstallStateResponse, RealtimeEvent)> {
    ensure_vault_key(db_path)?;
    let mut connection = Connection::open(db_path)?;
    let transaction = connection.transaction()?;
    if install_completed_at(&transaction)?.is_some() {
        bail!("Local install is already completed.");
    }

    let owner_display_name = require_label(&request.owner_display_name, "Owner display name")?;
    let business_name = require_label(&request.business_name, "Business name")?;
    let owner_email = normalize_optional_string(request.owner_email);
    let workspace_label = normalize_optional_string(request.workspace_label);
    let default_provider_id = normalize_optional_string(request.default_provider_id);
    if let Some(provider_id) = default_provider_id.as_deref() {
        require_provider_entry(provider_id)?;
    }

    let now = Utc::now().to_rfc3339();
    transaction.execute(
        "INSERT INTO appliance_owner (id, display_name, email, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?4)
         ON CONFLICT(id) DO UPDATE SET display_name = excluded.display_name, email = excluded.email, updated_at = excluded.updated_at",
        params![OWNER_ID, owner_display_name, owner_email, now],
    )?;
    transaction.execute(
        "INSERT INTO business_profile (id, name, workspace_label, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?4)
         ON CONFLICT(id) DO UPDATE SET name = excluded.name, workspace_label = excluded.workspace_label, updated_at = excluded.updated_at",
        params![BUSINESS_PROFILE_ID, business_name, workspace_label, now],
    )?;
    transaction.execute(
        "INSERT INTO install_state (
            id, installed, completed_at, owner_id, business_profile_id, default_provider_id, created_at, updated_at
         ) VALUES (?1, 1, ?2, ?3, ?4, ?5, ?2, ?2)",
        params![INSTALL_STATE_ID, now, OWNER_ID, BUSINESS_PROFILE_ID, default_provider_id],
    )?;
    let event = append_realtime_event_tx(
        &transaction,
        &system_event(
            "install.completed",
            json!({
                "ownerId": OWNER_ID,
                "businessProfileId": BUSINESS_PROFILE_ID,
                "defaultProviderId": default_provider_id,
            }),
        ),
    )?;
    transaction.commit()?;
    Ok((read_install_state_connection(&connection)?, event))
}

pub fn list_provider_configs(db_path: &Path) -> Result<ProviderListResponse> {
    let connection = Connection::open(db_path)?;
    list_provider_configs_connection(&connection)
}

pub fn update_provider_config(
    db_path: &Path,
    provider_id: &str,
    request: ProviderUpdateRequest,
) -> Result<(ProviderConfigView, RealtimeEvent)> {
    update_provider_config_with_env(db_path, provider_id, request, &std::env::vars().collect())
}

fn update_provider_config_with_env(
    db_path: &Path,
    provider_id: &str,
    request: ProviderUpdateRequest,
    env: &HashMap<String, String>,
) -> Result<(ProviderConfigView, RealtimeEvent)> {
    let mut connection = Connection::open(db_path)?;
    let transaction = connection.transaction()?;
    let provider_id = normalize_provider_id(provider_id)?;
    let catalog = require_provider_entry(&provider_id)?;
    let existing = find_provider_record(&transaction, &provider_id)?;
    let now = Utc::now().to_rfc3339();
    let secret = resolve_secret_with_env(catalog, env);
    let submitted_api_key = normalize_optional_string(request.api_key);
    if submitted_api_key.is_some() && secret.locked {
        let _event = append_realtime_event_tx(
            &transaction,
            &system_event(
                "provider.settings.rejected_locked",
                json!({ "providerId": provider_id, "lockedSource": secret.source }),
            ),
        )?;
        transaction.commit()?;
        bail!(
            "Provider API key for {provider_id} is locked by {} configuration.",
            secret.source
        );
    }

    let provider_name = normalize_optional_string(request.provider_name)
        .or_else(|| existing.as_ref().map(|record| record.provider_name.clone()))
        .unwrap_or_else(|| catalog.provider_name.to_string());
    let enabled = request
        .enabled
        .or_else(|| existing.as_ref().map(|record| record.enabled))
        .unwrap_or(false);
    let default_provider = request
        .default_provider
        .or_else(|| existing.as_ref().map(|record| record.default_provider))
        .unwrap_or(false);
    let model = normalize_optional_string(request.model)
        .or_else(|| existing.as_ref().and_then(|record| record.model.clone()))
        .or_else(|| catalog.default_model.map(str::to_string));
    if let Some(model) = model.as_deref() {
        if !catalog.available_models.is_empty() && !catalog.available_models.contains(&model) {
            bail!("Unsupported model for provider.");
        }
    }
    let base_url = normalize_optional_string(request.base_url)
        .or_else(|| existing.as_ref().and_then(|record| record.base_url.clone()))
        .or_else(|| catalog.default_base_url.map(str::to_string));
    let existing_secret_ref = existing
        .as_ref()
        .and_then(|record| record.secret_ref.clone());
    let secret_ref = match submitted_api_key {
        Some(api_key) => Some(
            store_secret(
                db_path,
                &transaction,
                "provider_api_key",
                &format!("{} API key", catalog.provider_name),
                &api_key,
                existing_secret_ref.as_deref(),
                json!({ "providerId": provider_id }),
            )?
            .id,
        ),
        None => existing_secret_ref,
    };
    let non_secret_config = request
        .non_secret_config
        .filter(Value::is_object)
        .or_else(|| {
            existing
                .as_ref()
                .map(|record| record.non_secret_config.clone())
        })
        .map(sanitize_non_secret_config)
        .unwrap_or_else(|| json!({}));
    let created_at = existing
        .as_ref()
        .map(|record| record.created_at.clone())
        .unwrap_or_else(|| now.clone());

    if default_provider {
        transaction.execute(
            "UPDATE provider_configs SET default_provider = 0 WHERE provider_id <> ?1",
            [provider_id.as_str()],
        )?;
    }
    transaction.execute(
        "INSERT INTO provider_configs (
            provider_id, provider_name, enabled, default_provider, model, base_url,
                secret_ref, non_secret_config_json, created_at, updated_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
         ON CONFLICT(provider_id) DO UPDATE SET
            provider_name = excluded.provider_name,
            enabled = excluded.enabled,
            default_provider = excluded.default_provider,
            model = excluded.model,
            base_url = excluded.base_url,
                secret_ref = excluded.secret_ref,
            non_secret_config_json = excluded.non_secret_config_json,
            updated_at = excluded.updated_at",
        params![
            provider_id,
            provider_name,
            enabled,
            default_provider,
            model,
            base_url,
            secret_ref,
            non_secret_config.to_string(),
            created_at,
            now,
        ],
    )?;
    let event = append_realtime_event_tx(
        &transaction,
        &system_event(
            "provider.settings.updated",
            json!({
                "providerId": provider_id,
                "enabled": enabled,
                "defaultProvider": default_provider,
                "apiKeyConfigured": secret.configured || secret_ref.is_some(),
                "apiKeySource": if secret.configured { secret.source } else if secret_ref.is_some() { "vault" } else { "missing" },
            }),
        ),
    )?;
    transaction.commit()?;

    let refreshed = Connection::open(db_path)?;
    let record = find_provider_record(&refreshed, &provider_id)?.expect("provider just inserted");
    Ok((provider_record_to_view(record, catalog, &env), event))
}

pub fn read_install_state_connection(connection: &Connection) -> Result<InstallStateResponse> {
    let completed_at = install_completed_at(connection)?;
    let owner = find_owner(connection)?;
    let business = find_business(connection)?;
    let providers = list_provider_configs_connection(connection)?.providers;
    let default_provider_id = providers
        .iter()
        .find(|provider| provider.default_provider)
        .map(|provider| provider.provider_id.clone())
        .or_else(|| install_default_provider_id(connection).ok().flatten());
    let enabled_provider_count = providers.iter().filter(|provider| provider.enabled).count();
    let missing_secret_provider_ids = providers
        .iter()
        .filter(|provider| {
            provider.enabled && !provider.api_key.configured && provider.provider_id != "local"
        })
        .map(|provider| provider.provider_id.clone())
        .collect::<Vec<_>>();

    Ok(InstallStateResponse {
        installed: completed_at.is_some(),
        completed_at,
        owner,
        business,
        provider_boundary: ProviderBoundarySummary {
            configured: enabled_provider_count > 0 && missing_secret_provider_ids.is_empty(),
            default_provider_id,
            enabled_provider_count,
            missing_secret_provider_ids,
        },
    })
}

pub fn list_provider_configs_connection(connection: &Connection) -> Result<ProviderListResponse> {
    list_provider_configs_connection_with_env(connection, &std::env::vars().collect())
}

pub(crate) fn list_provider_configs_connection_with_env(
    connection: &Connection,
    env: &HashMap<String, String>,
) -> Result<ProviderListResponse> {
    let mut providers = Vec::new();
    for catalog in PROVIDER_CATALOG {
        let record = find_provider_record(connection, catalog.provider_id)?
            .unwrap_or_else(|| default_provider_record(catalog));
        providers.push(provider_record_to_view(record, catalog, env));
    }
    let readiness = provider_readiness_summary(connection, &providers, env)?;
    Ok(ProviderListResponse {
        readiness,
        providers,
    })
}

fn install_completed_at(connection: &Connection) -> rusqlite::Result<Option<String>> {
    connection
        .query_row(
            "SELECT completed_at FROM install_state WHERE id = ?1 AND installed = 1",
            [INSTALL_STATE_ID],
            |row| row.get(0),
        )
        .optional()
}

fn install_default_provider_id(connection: &Connection) -> rusqlite::Result<Option<String>> {
    connection
        .query_row(
            "SELECT default_provider_id FROM install_state WHERE id = ?1",
            [INSTALL_STATE_ID],
            |row| row.get(0),
        )
        .optional()
        .map(|value| value.flatten())
}

fn find_owner(connection: &Connection) -> rusqlite::Result<Option<OwnerProfile>> {
    connection
        .query_row(
            "SELECT id, display_name, email, created_at, updated_at FROM appliance_owner WHERE id = ?1",
            [OWNER_ID],
            |row| {
                Ok(OwnerProfile {
                    id: row.get(0)?,
                    display_name: row.get(1)?,
                    email: row.get(2)?,
                    created_at: row.get(3)?,
                    updated_at: row.get(4)?,
                })
            },
        )
        .optional()
}

fn find_business(connection: &Connection) -> rusqlite::Result<Option<BusinessProfile>> {
    connection
        .query_row(
            "SELECT id, name, workspace_label, created_at, updated_at FROM business_profile WHERE id = ?1",
            [BUSINESS_PROFILE_ID],
            |row| {
                Ok(BusinessProfile {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    workspace_label: row.get(2)?,
                    created_at: row.get(3)?,
                    updated_at: row.get(4)?,
                })
            },
        )
        .optional()
}

fn find_provider_record(
    connection: &Connection,
    provider_id: &str,
) -> rusqlite::Result<Option<ProviderRecord>> {
    connection
        .query_row(
            "SELECT provider_id, provider_name, enabled, default_provider, model, base_url,
                    secret_ref, non_secret_config_json, created_at, updated_at
             FROM provider_configs
             WHERE provider_id = ?1",
            [provider_id],
            provider_record_from_row,
        )
        .optional()
}

fn provider_record_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<ProviderRecord> {
    let non_secret_config_json: String = row.get(7)?;
    Ok(ProviderRecord {
        provider_id: row.get(0)?,
        provider_name: row.get(1)?,
        enabled: row.get::<_, i64>(2)? == 1,
        default_provider: row.get::<_, i64>(3)? == 1,
        model: row.get(4)?,
        base_url: row.get(5)?,
        secret_ref: row.get(6)?,
        non_secret_config: sanitize_non_secret_config(
            serde_json::from_str(&non_secret_config_json).unwrap_or_else(|_| json!({})),
        ),
        created_at: row.get(8)?,
        updated_at: row.get(9)?,
    })
}

fn default_provider_record(catalog: &ProviderCatalogEntry) -> ProviderRecord {
    let now = "".to_string();
    ProviderRecord {
        provider_id: catalog.provider_id.to_string(),
        provider_name: catalog.provider_name.to_string(),
        enabled: false,
        default_provider: false,
        model: catalog.default_model.map(str::to_string),
        base_url: catalog.default_base_url.map(str::to_string),
        secret_ref: None,
        non_secret_config: json!({}),
        created_at: now.clone(),
        updated_at: now,
    }
}

fn provider_record_to_view(
    record: ProviderRecord,
    catalog: &ProviderCatalogEntry,
    env: &HashMap<String, String>,
) -> ProviderConfigView {
    let resolved = resolve_secret_with_env(catalog, &env);
    let local_configured = record.secret_ref.is_some();
    let configured = resolved.configured || local_configured;
    let source = if resolved.configured {
        resolved.source.to_string()
    } else if local_configured {
        "vault".to_string()
    } else {
        "missing".to_string()
    };
    let base_url = if record.provider_id == "local" {
        normalize_optional_string(env.get("ORDO_OLLAMA_BASE_URL").cloned())
            .or_else(|| normalize_optional_string(env.get("OLLAMA_BASE_URL").cloned()))
            .or(record.base_url)
    } else {
        record.base_url
    };

    ProviderConfigView {
        provider_id: record.provider_id,
        provider_name: record.provider_name,
        enabled: record.enabled,
        default_provider: record.default_provider,
        model: record.model,
        available_models: catalog
            .available_models
            .iter()
            .map(|model| ProviderModelOption {
                id: (*model).to_string(),
                label: (*model).to_string(),
                default: Some(*model) == catalog.default_model,
            })
            .collect(),
        base_url,
        non_secret_config: record.non_secret_config,
        api_key: RedactedSecretField {
            configured,
            source,
            locked: resolved.locked,
            redacted: configured.then(|| "[redacted]".to_string()),
        },
        created_at: record.created_at,
        updated_at: record.updated_at,
    }
}

fn provider_readiness_summary(
    connection: &Connection,
    providers: &[ProviderConfigView],
    env: &HashMap<String, String>,
) -> Result<ProviderReadinessSummary> {
    let configured_provider_mode =
        normalize_optional_string(env.get("ORDO_LIVE_LLM_PROVIDER").cloned())
            .unwrap_or_else(|| "deterministic_local".to_string());
    let requested_provider_id = if configured_provider_mode == "deterministic_local" {
        None
    } else {
        Some(configured_provider_mode.to_ascii_lowercase())
    };
    let default_provider_id = providers
        .iter()
        .find(|provider| provider.default_provider)
        .map(|provider| provider.provider_id.clone())
        .or_else(|| install_default_provider_id(connection).ok().flatten());
    let live_mode_requested = requested_provider_id.is_some();
    let credential_provider_id = requested_provider_id
        .as_deref()
        .or(default_provider_id.as_deref())
        .unwrap_or("local");
    let credential_provider = providers
        .iter()
        .find(|provider| provider.provider_id == credential_provider_id);
    let credentials_present = credential_provider
        .map(|provider| provider.provider_id == "local" || provider.api_key.configured)
        .unwrap_or(false);
    let credential_source = credential_provider
        .map(|provider| provider.api_key.source.clone())
        .unwrap_or_else(|| "unsupported_provider".to_string());
    let missing_credential_provider_ids = providers
        .iter()
        .filter(|provider| {
            provider.enabled && provider.provider_id != "local" && !provider.api_key.configured
        })
        .map(|provider| provider.provider_id.clone())
        .collect::<Vec<_>>();
    let openai = openai_provider_readiness(requested_provider_id.as_deref(), providers, env);

    Ok(ProviderReadinessSummary {
        configured_provider_mode,
        requested_provider_id,
        default_provider_id,
        live_mode_requested,
        live_invocation_enabled: false,
        live_invocation_guard: "conversation_gateway_local_only".to_string(),
        credentials_present,
        credential_source,
        missing_credential_provider_ids,
        openai,
    })
}

fn openai_provider_readiness(
    requested_provider_id: Option<&str>,
    providers: &[ProviderConfigView],
    env: &HashMap<String, String>,
) -> OpenAiProviderReadiness {
    const DEFAULT_TIMEOUT_MS: u64 = 30_000;
    const DEFAULT_MAX_CASES: u32 = 1;
    const DEFAULT_BUDGET_MICROS: u64 = 10_000;
    const ESTIMATED_CASE_COST_MICROS: u64 = 1_000;

    let openai = providers
        .iter()
        .find(|provider| provider.provider_id == "openai");
    let mut reasons = Vec::new();
    let live_invocation_guard = "conversation_gateway_local_only".to_string();
    let mode_targets_openai = requested_provider_id == Some("openai");
    if !mode_targets_openai {
        reasons.push("ORDO_LIVE_LLM_PROVIDER is not set to openai.".to_string());
    }

    let (model_id, model_source) =
        match normalize_optional_string(env.get("ORDO_LIVE_LLM_MODEL").cloned()) {
            Some(model) => (Some(model), "env".to_string()),
            None => (
                openai.and_then(|provider| provider.model.clone()),
                "provider_config".to_string(),
            ),
        };
    let model_supported = model_id.as_deref().is_some_and(|model| {
        openai
            .map(|provider| {
                provider
                    .available_models
                    .iter()
                    .any(|option| option.id == model)
            })
            .unwrap_or(false)
    });
    if model_id.is_none() {
        reasons.push("OpenAI model is missing.".to_string());
    } else if !model_supported {
        reasons.push("OpenAI model is not in the daemon catalog.".to_string());
    }

    let (base_url, base_url_source) =
        match normalize_optional_string(env.get("ORDO_LIVE_LLM_BASE_URL").cloned()) {
            Some(base_url) => (base_url, "env".to_string()),
            None => (
                openai
                    .and_then(|provider| provider.base_url.clone())
                    .unwrap_or_else(|| "https://api.openai.com/v1".to_string()),
                "provider_config".to_string(),
            ),
        };

    let api_key_configured = openai
        .map(|provider| provider.api_key.configured)
        .unwrap_or(false);
    let api_key_source = openai
        .map(|provider| provider.api_key.source.clone())
        .unwrap_or_else(|| "missing".to_string());
    if !api_key_configured {
        reasons.push("OpenAI API key is missing.".to_string());
    }

    let (timeout_ms, timeout_guard) =
        match optional_positive_u64(env, "ORDO_LIVE_LLM_TIMEOUT_MS", DEFAULT_TIMEOUT_MS) {
            ParsedGuardValue::Ready(value) => (Some(value), "ready".to_string()),
            ParsedGuardValue::Invalid(reason) => {
                reasons.push(reason);
                (None, "missing_timeout".to_string())
            }
        };
    let (max_cases, max_cases_guard) =
        match optional_positive_u32(env, "ORDO_LIVE_LLM_MAX_CASES", DEFAULT_MAX_CASES) {
            ParsedGuardValue::Ready(value) if value == 1 => (Some(value), "ready".to_string()),
            ParsedGuardValue::Ready(_) => {
                reasons.push("ORDO_LIVE_LLM_MAX_CASES must be 1 for guarded smoke.".to_string());
                (None, "invalid_max_cases".to_string())
            }
            ParsedGuardValue::Invalid(reason) => {
                reasons.push(reason);
                (None, "invalid_max_cases".to_string())
            }
        };
    let (budget_micros, budget_guard) =
        match optional_usd_micros(env, "ORDO_LIVE_LLM_BUDGET_USD", DEFAULT_BUDGET_MICROS) {
            ParsedGuardValue::Ready(value) if value >= ESTIMATED_CASE_COST_MICROS => {
                (Some(value), "ready".to_string())
            }
            ParsedGuardValue::Ready(_) => {
                reasons.push(
                    "Live LLM budget is below the conservative per-case estimate.".to_string(),
                );
                (None, "missing_budget".to_string())
            }
            ParsedGuardValue::Invalid(reason) => {
                reasons.push(reason);
                (None, "missing_budget".to_string())
            }
        };

    let live_eval_guard = if env_is_one(env, "ORDO_LIVE_LLM_EVALS") {
        "enabled".to_string()
    } else {
        reasons.push("ORDO_LIVE_LLM_EVALS=1 is required for guarded smoke.".to_string());
        "missing_live_guard".to_string()
    };
    let network_guard = if env_is_one(env, "ORDO_LIVE_LLM_ALLOW_NETWORK") {
        "enabled".to_string()
    } else {
        reasons.push("ORDO_LIVE_LLM_ALLOW_NETWORK=1 is required for guarded smoke.".to_string());
        "missing_network_guard".to_string()
    };

    let ready_for_guarded_smoke = mode_targets_openai
        && model_supported
        && api_key_configured
        && timeout_guard == "ready"
        && max_cases_guard == "ready"
        && budget_guard == "ready"
        && live_eval_guard == "enabled"
        && network_guard == "enabled";
    let decision = if !mode_targets_openai {
        if requested_provider_id.is_some() {
            "unsupported_mode"
        } else {
            "disabled"
        }
    } else if !api_key_configured {
        "missing_key"
    } else if model_id.is_none() {
        "missing_model"
    } else if !model_supported {
        "unsupported_model"
    } else if timeout_guard != "ready" {
        "missing_timeout"
    } else if budget_guard != "ready" {
        "missing_budget"
    } else if ready_for_guarded_smoke {
        "ready_for_guarded_smoke"
    } else {
        "ready_but_live_disabled"
    }
    .to_string();

    OpenAiProviderReadiness {
        provider_id: "openai".to_string(),
        decision,
        model_id,
        model_source,
        base_url,
        base_url_source,
        timeout_ms,
        timeout_guard,
        budget_micros,
        budget_guard,
        max_cases,
        api_key_configured,
        api_key_source,
        live_eval_guard,
        network_guard,
        live_invocation_guard,
        ready_for_guarded_smoke,
        reasons,
    }
}

enum ParsedGuardValue<T> {
    Ready(T),
    Invalid(String),
}

fn optional_positive_u64(
    env: &HashMap<String, String>,
    key: &str,
    default_value: u64,
) -> ParsedGuardValue<u64> {
    match normalize_optional_string(env.get(key).cloned()) {
        Some(value) => match value.parse::<u64>() {
            Ok(parsed) if parsed > 0 => ParsedGuardValue::Ready(parsed),
            _ => ParsedGuardValue::Invalid(format!("{key} must be a positive integer.")),
        },
        None => ParsedGuardValue::Ready(default_value),
    }
}

fn optional_positive_u32(
    env: &HashMap<String, String>,
    key: &str,
    default_value: u32,
) -> ParsedGuardValue<u32> {
    match normalize_optional_string(env.get(key).cloned()) {
        Some(value) => match value.parse::<u32>() {
            Ok(parsed) if parsed > 0 => ParsedGuardValue::Ready(parsed),
            _ => ParsedGuardValue::Invalid(format!("{key} must be a positive integer.")),
        },
        None => ParsedGuardValue::Ready(default_value),
    }
}

fn optional_usd_micros(
    env: &HashMap<String, String>,
    key: &str,
    default_value: u64,
) -> ParsedGuardValue<u64> {
    match normalize_optional_string(env.get(key).cloned()) {
        Some(value) => match value.parse::<f64>() {
            Ok(parsed) if parsed.is_finite() && parsed >= 0.0 => {
                ParsedGuardValue::Ready((parsed * 1_000_000.0).round() as u64)
            }
            _ => ParsedGuardValue::Invalid(format!("{key} must be a non-negative USD amount.")),
        },
        None => ParsedGuardValue::Ready(default_value),
    }
}

fn env_is_one(env: &HashMap<String, String>, key: &str) -> bool {
    normalize_optional_string(env.get(key).cloned()).as_deref() == Some("1")
}

fn resolve_secret_with_env(
    catalog: &ProviderCatalogEntry,
    env: &HashMap<String, String>,
) -> ResolvedSecret {
    for key in catalog.api_key_env_keys {
        if normalize_optional_string(env.get(*key).cloned()).is_some() {
            return ResolvedSecret {
                configured: true,
                source: "env",
                locked: true,
            };
        }
        let file_key = format!("{key}_FILE");
        if let Some(path) = normalize_optional_string(env.get(&file_key).cloned()) {
            if fs::read_to_string(path)
                .ok()
                .and_then(|value| normalize_optional_string(Some(value)))
                .is_some()
            {
                return ResolvedSecret {
                    configured: true,
                    source: "file",
                    locked: true,
                };
            }
        }
    }
    ResolvedSecret {
        configured: false,
        source: "missing",
        locked: false,
    }
}

fn require_provider_entry(provider_id: &str) -> Result<&'static ProviderCatalogEntry> {
    PROVIDER_CATALOG
        .iter()
        .find(|entry| entry.provider_id == provider_id)
        .ok_or_else(|| anyhow::anyhow!("Unsupported provider id: {provider_id}"))
}

fn normalize_provider_id(value: &str) -> Result<String> {
    let normalized = value.trim().to_ascii_lowercase();
    if normalized.is_empty() || normalized.len() > 64 || !is_safe_identifier(&normalized) {
        bail!("Invalid provider id.");
    }
    Ok(normalized)
}

fn require_label(value: &str, label: &str) -> Result<String> {
    normalize_optional_string(Some(value.to_string()))
        .filter(|value| value.len() <= 160)
        .ok_or_else(|| anyhow::anyhow!("{label} is required."))
}

fn normalize_optional_string(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().replace(char::is_whitespace, " "))
        .map(|value| value.split_whitespace().collect::<Vec<_>>().join(" "))
        .filter(|value| !value.is_empty())
}

fn sanitize_non_secret_config(value: Value) -> Value {
    match value {
        Value::Object(map) => Value::Object(
            map.into_iter()
                .map(|(key, value)| {
                    if is_sensitive_config_key(&key) {
                        (key, Value::String("[redacted]".to_string()))
                    } else {
                        (key, sanitize_non_secret_config(value))
                    }
                })
                .collect(),
        ),
        Value::Array(values) => {
            Value::Array(values.into_iter().map(sanitize_non_secret_config).collect())
        }
        other => other,
    }
}

fn is_sensitive_config_key(key: &str) -> bool {
    let lower = key.to_ascii_lowercase();
    lower.contains("token")
        || lower.contains("secret")
        || lower.contains("password")
        || lower.contains("apikey")
        || lower.contains("api_key")
        || lower.contains("api-key")
        || lower == "key"
        || lower.ends_with("key")
}

fn is_safe_identifier(value: &str) -> bool {
    value.chars().all(|character| {
        character.is_ascii_lowercase() || character.is_ascii_digit() || character == '_'
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostics::{diagnostic_log, insert_diagnostic_log_connection};
    use crate::reports::{prepare_issue_report, IssueReportPrepareRequest};
    use crate::schema::init_schema;
    use crate::vault::{decrypt_secret, get_vault_item};

    fn connection() -> Connection {
        let connection = Connection::open_in_memory().unwrap();
        init_schema(&connection).unwrap();
        connection
    }

    #[test]
    fn fresh_database_reports_not_installed() {
        let connection = connection();

        let state = read_install_state_connection(&connection).unwrap();

        assert!(!state.installed);
        assert!(state.completed_at.is_none());
        assert!(state.owner.is_none());
        assert!(state.business.is_none());
    }

    #[test]
    fn install_completion_persists_owner_and_business() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let db_path = temp_dir.path().join("local.db");
        crate::schema::init_database(&db_path).unwrap();

        let (state, event) = complete_local_install(
            &db_path,
            CompleteInstallRequest {
                owner_display_name: "Kenny".to_string(),
                owner_email: Some("kenny@example.test".to_string()),
                business_name: "Studio Ordo".to_string(),
                workspace_label: Some("Local appliance".to_string()),
                default_provider_id: Some("anthropic".to_string()),
            },
        )
        .unwrap();

        assert!(state.installed);
        assert_eq!(state.owner.unwrap().display_name, "Kenny");
        assert_eq!(state.business.unwrap().name, "Studio Ordo");
        assert_eq!(
            state.provider_boundary.default_provider_id.as_deref(),
            Some("anthropic")
        );
        assert_eq!(event.event_type, "install.completed");
    }

    #[test]
    fn repeated_install_completion_rejects() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let db_path = temp_dir.path().join("local.db");
        crate::schema::init_database(&db_path).unwrap();
        let request = CompleteInstallRequest {
            owner_display_name: "Kenny".to_string(),
            owner_email: None,
            business_name: "Studio Ordo".to_string(),
            workspace_label: None,
            default_provider_id: None,
        };

        complete_local_install(&db_path, request.clone()).unwrap();
        let second = complete_local_install(&db_path, request).unwrap_err();

        assert!(second.to_string().contains("already completed"));
    }

    #[test]
    fn provider_secret_is_redacted_on_read() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let db_path = temp_dir.path().join("local.db");
        crate::schema::init_database(&db_path).unwrap();

        let (provider, event) = update_provider_config(
            &db_path,
            "anthropic",
            ProviderUpdateRequest {
                provider_name: None,
                enabled: Some(true),
                default_provider: Some(true),
                model: Some("claude-sonnet-4-6".to_string()),
                base_url: None,
                api_key: Some("sk-test-secret".to_string()),
                non_secret_config: Some(json!({ "timeoutMs": 45000 })),
            },
        )
        .unwrap();

        assert!(provider.api_key.configured);
        assert_eq!(provider.api_key.redacted.as_deref(), Some("[redacted]"));
        assert_eq!(provider.api_key.source, "vault");
        let serialized = serde_json::to_string(&provider).unwrap();
        assert!(!serialized.contains("sk-test-secret"));
        assert!(!event.payload.to_string().contains("sk-test-secret"));

        let connection = Connection::open(&db_path).unwrap();
        let secret_ref: String = connection
            .query_row(
                "SELECT secret_ref FROM provider_configs WHERE provider_id = 'anthropic'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let item = get_vault_item(&connection, &secret_ref).unwrap().unwrap();
        assert!(!item.encrypted_value.contains("sk-test-secret"));
        assert_eq!(
            decrypt_secret(&db_path, &connection, &secret_ref).unwrap(),
            "sk-test-secret"
        );
    }

    #[test]
    fn provider_secret_lock_detects_env_source_without_revealing_value() {
        let catalog = require_provider_entry("anthropic").unwrap();
        let env = HashMap::from([("ANTHROPIC_API_KEY".to_string(), "sk-env-secret".to_string())]);

        let resolved = resolve_secret_with_env(catalog, &env);

        assert_eq!(resolved.source, "env");
        assert!(resolved.configured);
        assert!(resolved.locked);
    }

    #[test]
    fn provider_readiness_reports_live_mode_without_exposing_env_secret() {
        let connection = connection();
        let env = HashMap::from([
            ("ORDO_LIVE_LLM_PROVIDER".to_string(), "openai".to_string()),
            ("OPENAI_API_KEY".to_string(), "sk-live-secret".to_string()),
        ]);

        let response = list_provider_configs_connection_with_env(&connection, &env).unwrap();

        assert_eq!(response.readiness.configured_provider_mode, "openai");
        assert_eq!(
            response.readiness.requested_provider_id.as_deref(),
            Some("openai")
        );
        assert!(response.readiness.live_mode_requested);
        assert!(!response.readiness.live_invocation_enabled);
        assert_eq!(
            response.readiness.live_invocation_guard,
            "conversation_gateway_local_only"
        );
        assert!(response.readiness.credentials_present);
        assert_eq!(response.readiness.credential_source, "env");

        let serialized = serde_json::to_string(&response).unwrap();
        assert!(!serialized.contains("sk-live-secret"));
        assert!(serialized.contains("[redacted]"));
    }

    #[test]
    fn openai_readiness_reports_env_key_without_leaking_value() {
        let connection = connection();
        let env = HashMap::from([
            ("ORDO_LIVE_LLM_PROVIDER".to_string(), "openai".to_string()),
            ("ORDO_LIVE_LLM_MODEL".to_string(), "gpt-5".to_string()),
            ("OPENAI_API_KEY".to_string(), "sk-openai-secret".to_string()),
        ]);

        let response = list_provider_configs_connection_with_env(&connection, &env).unwrap();

        assert_eq!(response.readiness.openai.provider_id, "openai");
        assert_eq!(response.readiness.openai.api_key_source, "env");
        assert!(response.readiness.openai.api_key_configured);
        assert_eq!(
            response.readiness.openai.decision,
            "ready_but_live_disabled"
        );
        assert!(!response.readiness.openai.ready_for_guarded_smoke);
        let serialized = serde_json::to_string(&response).unwrap();
        assert!(!serialized.contains("sk-openai-secret"));
        assert!(serialized.contains("[redacted]"));
    }

    #[test]
    fn openai_readiness_fails_closed_when_key_is_missing() {
        let connection = connection();
        let env = HashMap::from([
            ("ORDO_LIVE_LLM_PROVIDER".to_string(), "openai".to_string()),
            ("ORDO_LIVE_LLM_MODEL".to_string(), "gpt-5".to_string()),
        ]);

        let response = list_provider_configs_connection_with_env(&connection, &env).unwrap();

        assert_eq!(response.readiness.openai.decision, "missing_key");
        assert!(!response.readiness.openai.api_key_configured);
        assert_eq!(response.readiness.openai.api_key_source, "missing");
        assert!(!response.readiness.openai.ready_for_guarded_smoke);
    }

    #[test]
    fn openai_readiness_rejects_unsupported_env_model() {
        let connection = connection();
        let env = HashMap::from([
            ("ORDO_LIVE_LLM_PROVIDER".to_string(), "openai".to_string()),
            (
                "ORDO_LIVE_LLM_MODEL".to_string(),
                "not-a-catalog-model".to_string(),
            ),
            ("OPENAI_API_KEY".to_string(), "sk-openai-secret".to_string()),
        ]);

        let response = list_provider_configs_connection_with_env(&connection, &env).unwrap();

        assert_eq!(response.readiness.openai.decision, "unsupported_model");
        assert_eq!(
            response.readiness.openai.model_id.as_deref(),
            Some("not-a-catalog-model")
        );
        assert!(!response.readiness.openai.ready_for_guarded_smoke);
        assert!(!serde_json::to_string(&response)
            .unwrap()
            .contains("sk-openai-secret"));
    }

    #[test]
    fn openai_readiness_reports_missing_live_guard_without_network() {
        let connection = connection();
        let env = HashMap::from([
            ("ORDO_LIVE_LLM_PROVIDER".to_string(), "openai".to_string()),
            ("ORDO_LIVE_LLM_MODEL".to_string(), "gpt-5".to_string()),
            ("OPENAI_API_KEY".to_string(), "sk-openai-secret".to_string()),
        ]);

        let response = list_provider_configs_connection_with_env(&connection, &env).unwrap();

        assert_eq!(
            response.readiness.openai.decision,
            "ready_but_live_disabled"
        );
        assert_eq!(
            response.readiness.openai.live_eval_guard,
            "missing_live_guard"
        );
        assert_eq!(
            response.readiness.openai.network_guard,
            "missing_network_guard"
        );
        assert_eq!(
            response.readiness.openai.live_invocation_guard,
            "conversation_gateway_local_only"
        );
        assert!(!response.readiness.openai.ready_for_guarded_smoke);
    }

    #[test]
    fn openai_readiness_reports_ready_for_guarded_smoke_when_all_guards_are_set() {
        let connection = connection();
        let env = HashMap::from([
            ("ORDO_LIVE_LLM_PROVIDER".to_string(), "openai".to_string()),
            ("ORDO_LIVE_LLM_MODEL".to_string(), "gpt-5".to_string()),
            ("OPENAI_API_KEY".to_string(), "sk-openai-secret".to_string()),
            ("ORDO_LIVE_LLM_EVALS".to_string(), "1".to_string()),
            ("ORDO_LIVE_LLM_ALLOW_NETWORK".to_string(), "1".to_string()),
            ("ORDO_LIVE_LLM_BUDGET_USD".to_string(), "0.01".to_string()),
            ("ORDO_LIVE_LLM_TIMEOUT_MS".to_string(), "30000".to_string()),
            ("ORDO_LIVE_LLM_MAX_CASES".to_string(), "1".to_string()),
        ]);

        let response = list_provider_configs_connection_with_env(&connection, &env).unwrap();

        assert_eq!(
            response.readiness.openai.decision,
            "ready_for_guarded_smoke"
        );
        assert!(response.readiness.openai.ready_for_guarded_smoke);
        assert_eq!(response.readiness.openai.live_eval_guard, "enabled");
        assert_eq!(response.readiness.openai.network_guard, "enabled");
        assert_eq!(response.readiness.openai.budget_micros, Some(10_000));
        assert_eq!(response.readiness.openai.timeout_ms, Some(30_000));
        assert!(!serde_json::to_string(&response)
            .unwrap()
            .contains("sk-openai-secret"));
    }

    #[test]
    fn provider_readiness_reports_missing_credentials_for_enabled_provider() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let db_path = temp_dir.path().join("local.db");
        crate::schema::init_database(&db_path).unwrap();

        update_provider_config_with_env(
            &db_path,
            "openai",
            ProviderUpdateRequest {
                provider_name: None,
                enabled: Some(true),
                default_provider: Some(true),
                model: None,
                base_url: None,
                api_key: None,
                non_secret_config: None,
            },
            &HashMap::new(),
        )
        .unwrap();
        let connection = Connection::open(&db_path).unwrap();
        let env = HashMap::from([("ORDO_LIVE_LLM_PROVIDER".to_string(), "openai".to_string())]);

        let response = list_provider_configs_connection_with_env(&connection, &env).unwrap();

        assert!(!response.readiness.credentials_present);
        assert_eq!(response.readiness.credential_source, "missing");
        assert_eq!(
            response.readiness.missing_credential_provider_ids,
            vec!["openai".to_string()]
        );
        assert!(!response.readiness.live_invocation_enabled);
    }

    #[test]
    fn provider_read_model_includes_catalog_model_options() {
        let connection = connection();

        let response =
            list_provider_configs_connection_with_env(&connection, &HashMap::new()).unwrap();
        let anthropic = response
            .providers
            .iter()
            .find(|provider| provider.provider_id == "anthropic")
            .unwrap();

        assert_eq!(anthropic.model.as_deref(), Some("claude-haiku-4-5"));
        assert_eq!(
            anthropic
                .available_models
                .iter()
                .map(|model| model.id.as_str())
                .collect::<Vec<_>>(),
            vec!["claude-haiku-4-5", "claude-sonnet-4-6"]
        );
        assert!(anthropic
            .available_models
            .iter()
            .any(|model| model.id == "claude-haiku-4-5" && model.default));
    }

    #[test]
    fn provider_update_rejects_unknown_catalog_model() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let db_path = temp_dir.path().join("local.db");
        crate::schema::init_database(&db_path).unwrap();

        let result = update_provider_config_with_env(
            &db_path,
            "openai",
            ProviderUpdateRequest {
                provider_name: None,
                enabled: Some(true),
                default_provider: None,
                model: Some("not-a-real-option".to_string()),
                base_url: None,
                api_key: None,
                non_secret_config: None,
            },
            &HashMap::new(),
        );

        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Unsupported model for provider"));
    }

    #[test]
    fn locked_provider_secret_update_records_redacted_rejection_event() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let db_path = temp_dir.path().join("local.db");
        crate::schema::init_database(&db_path).unwrap();
        let env = HashMap::from([("ANTHROPIC_API_KEY".to_string(), "sk-env-secret".to_string())]);

        let result = update_provider_config_with_env(
            &db_path,
            "anthropic",
            ProviderUpdateRequest {
                provider_name: None,
                enabled: Some(true),
                default_provider: None,
                model: None,
                base_url: None,
                api_key: Some("sk-local-secret".to_string()),
                non_secret_config: None,
            },
            &env,
        );

        assert!(result.unwrap_err().to_string().contains("locked by env"));
        let connection = Connection::open(&db_path).unwrap();
        let payload_json: String = connection
            .query_row(
                "SELECT payload_json FROM realtime_events WHERE event_type = 'provider.settings.rejected_locked'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(payload_json.contains("anthropic"));
        assert!(!payload_json.contains("sk-env-secret"));
        assert!(!payload_json.contains("sk-local-secret"));
    }

    #[test]
    fn provider_events_do_not_include_secret_value() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let db_path = temp_dir.path().join("local.db");
        crate::schema::init_database(&db_path).unwrap();

        let (_, event) = update_provider_config(
            &db_path,
            "openai",
            ProviderUpdateRequest {
                provider_name: None,
                enabled: Some(true),
                default_provider: None,
                model: None,
                base_url: None,
                api_key: Some("secret-openai-key".to_string()),
                non_secret_config: None,
            },
        )
        .unwrap();

        assert_eq!(event.event_type, "provider.settings.updated");
        assert!(!event.payload.to_string().contains("secret-openai-key"));
    }

    #[test]
    fn provider_non_secret_config_redacts_secret_shaped_keys() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let db_path = temp_dir.path().join("local.db");
        crate::schema::init_database(&db_path).unwrap();

        let (provider, _) = update_provider_config(
            &db_path,
            "anthropic",
            ProviderUpdateRequest {
                provider_name: None,
                enabled: Some(true),
                default_provider: None,
                model: None,
                base_url: None,
                api_key: None,
                non_secret_config: Some(json!({
                    "timeoutMs": 45000,
                    "fallbackApiKey": "secret-in-wrong-place",
                    "nested": { "accessToken": "nested-secret" }
                })),
            },
        )
        .unwrap();

        assert_eq!(provider.non_secret_config["timeoutMs"], 45000);
        assert_eq!(provider.non_secret_config["fallbackApiKey"], "[redacted]");
        assert_eq!(
            provider.non_secret_config["nested"]["accessToken"],
            "[redacted]"
        );
        let serialized = serde_json::to_string(&provider).unwrap();
        assert!(!serialized.contains("secret-in-wrong-place"));
        assert!(!serialized.contains("nested-secret"));
    }

    #[test]
    fn diagnostic_logs_and_reports_do_not_include_provider_secret_values() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let db_path = temp_dir.path().join("local.db");
        crate::schema::init_database(&db_path).unwrap();
        let connection = Connection::open(&db_path).unwrap();
        insert_diagnostic_log_connection(
            &connection,
            diagnostic_log(
                "info",
                "providers",
                "Provider settings updated.",
                json!({ "providerId": "anthropic", "apiKey": "secret-report-key" }),
            ),
        )
        .unwrap();

        let report = prepare_issue_report(
            &db_path,
            IssueReportPrepareRequest {
                title: Some("Provider secret redaction check".to_string()),
                severity: None,
                description: "Check provider logs do not leak secrets.".to_string(),
                expected_behavior: None,
                actual_behavior: None,
                steps: None,
                source_route: None,
                include_health_snapshot: Some(false),
                include_recent_events: Some(false),
                include_recent_jobs: Some(false),
                include_diagnostic_logs: Some(true),
                include_browser_context: Some(false),
                browser_context: None,
            },
            "test",
            Some("actor_local_owner"),
        )
        .unwrap();
        let serialized = serde_json::to_string(&report).unwrap();

        assert!(!serialized.contains("secret-report-key"));
        assert!(serialized.contains("[redacted]"));
    }
}
