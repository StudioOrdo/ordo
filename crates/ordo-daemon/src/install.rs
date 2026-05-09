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
    pub providers: Vec<ProviderConfigView>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderConfigView {
    pub provider_id: String,
    pub provider_name: String,
    pub enabled: bool,
    pub default_provider: bool,
    pub model: Option<String>,
    pub base_url: Option<String>,
    pub non_secret_config: Value,
    pub api_key: RedactedSecretField,
    pub created_at: String,
    pub updated_at: String,
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
    default_base_url: Option<&'static str>,
}

const PROVIDER_CATALOG: &[ProviderCatalogEntry] = &[
    ProviderCatalogEntry {
        provider_id: "anthropic",
        provider_name: "Anthropic",
        api_key_env_keys: &["ANTHROPIC_API_KEY", "API__ANTHROPIC_API_KEY"],
        default_model: Some("claude-haiku-4-5"),
        default_base_url: None,
    },
    ProviderCatalogEntry {
        provider_id: "openai",
        provider_name: "OpenAI",
        api_key_env_keys: &["OPENAI_API_KEY", "API__OPENAI_API_KEY"],
        default_model: Some("gpt-5"),
        default_base_url: None,
    },
    ProviderCatalogEntry {
        provider_id: "deepseek",
        provider_name: "DeepSeek",
        api_key_env_keys: &["DEEPSEEK_API_KEY"],
        default_model: Some("deepseek-v4-flash"),
        default_base_url: Some("https://api.deepseek.com/anthropic"),
    },
    ProviderCatalogEntry {
        provider_id: "local",
        provider_name: "Local Model Provider",
        api_key_env_keys: &[],
        default_model: None,
        default_base_url: None,
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
    Ok((provider_record_to_view(record, catalog), event))
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
    let mut providers = Vec::new();
    for catalog in PROVIDER_CATALOG {
        let record = find_provider_record(connection, catalog.provider_id)?
            .unwrap_or_else(|| default_provider_record(catalog));
        providers.push(provider_record_to_view(record, catalog));
    }
    Ok(ProviderListResponse { providers })
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
) -> ProviderConfigView {
    let env = std::env::vars().collect();
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
    ProviderConfigView {
        provider_id: record.provider_id,
        provider_name: record.provider_name,
        enabled: record.enabled,
        default_provider: record.default_provider,
        model: record.model,
        base_url: record.base_url,
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
