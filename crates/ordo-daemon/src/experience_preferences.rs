use anyhow::{bail, Context, Result};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

pub const EXPERIENCE_PREFERENCES_SCHEMA_VERSION: &str = "ordo.experience_preferences.v1";

const ALLOWED_SETTING_KEYS: &[&str] = &[
    "theme",
    "density",
    "motion",
    "typeScale",
    "contrast",
    "evidenceDetail",
    "privacyDisplay",
    "performanceMode",
    "locale",
    "colorBlindMode",
    "localComputeEnabled",
    "gpuVisualsEnabled",
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActorExperiencePreferenceRecord {
    pub actor_id: String,
    pub schema_version: String,
    pub requested_settings_json: String,
    pub created_at: String,
    pub updated_at: String,
}

pub fn save_actor_experience_preferences(
    connection: &Connection,
    actor_id: &str,
    requested_settings_json: &str,
    now: &str,
) -> Result<ActorExperiencePreferenceRecord> {
    let actor_id = actor_id.trim();
    if actor_id.is_empty() {
        bail!("actor experience preferences require a non-empty actor id");
    }
    if now.trim().is_empty() {
        bail!("actor experience preferences require a non-empty timestamp");
    }

    let canonical_requested_settings_json =
        canonicalize_requested_settings_json(requested_settings_json)?;

    connection.execute(
        "INSERT INTO actor_experience_preferences (
            actor_id, schema_version, requested_settings_json, created_at, updated_at
         ) VALUES (?1, ?2, ?3, ?4, ?4)
         ON CONFLICT(actor_id) DO UPDATE SET
            schema_version = excluded.schema_version,
            requested_settings_json = excluded.requested_settings_json,
            updated_at = excluded.updated_at",
        params![
            actor_id,
            EXPERIENCE_PREFERENCES_SCHEMA_VERSION,
            canonical_requested_settings_json,
            now
        ],
    )?;

    load_actor_experience_preferences(connection, actor_id)?
        .context("actor experience preferences should exist after save")
}

pub fn load_actor_experience_preferences(
    connection: &Connection,
    actor_id: &str,
) -> Result<Option<ActorExperiencePreferenceRecord>> {
    let actor_id = actor_id.trim();
    if actor_id.is_empty() {
        return Ok(None);
    }

    connection
        .query_row(
            "SELECT actor_id, schema_version, requested_settings_json, created_at, updated_at
             FROM actor_experience_preferences
             WHERE actor_id = ?1",
            [actor_id],
            |row| {
                Ok(ActorExperiencePreferenceRecord {
                    actor_id: row.get(0)?,
                    schema_version: row.get(1)?,
                    requested_settings_json: row.get(2)?,
                    created_at: row.get(3)?,
                    updated_at: row.get(4)?,
                })
            },
        )
        .optional()
        .context("load actor experience preferences")
}

pub fn canonicalize_requested_settings_json(raw_json: &str) -> Result<String> {
    let value: Value = serde_json::from_str(raw_json).context("parse requested settings JSON")?;
    let object = value
        .as_object()
        .context("requested settings must be a JSON object")?;

    let mut canonical = Map::new();
    for key in ALLOWED_SETTING_KEYS {
        if let Some(value) = object.get(*key) {
            validate_setting_value(key, value)?;
            canonical.insert((*key).to_string(), value.clone());
        }
    }

    for key in object.keys() {
        if !ALLOWED_SETTING_KEYS.contains(&key.as_str()) {
            bail!("unknown experience preference setting `{key}`");
        }
    }

    Ok(serde_json::to_string(&Value::Object(canonical))?)
}

fn validate_setting_value(key: &str, value: &Value) -> Result<()> {
    match value {
        Value::String(value) => {
            reject_sensitive_text(value)?;
            if value.trim().is_empty() {
                bail!("experience preference setting `{key}` cannot be empty");
            }
        }
        Value::Bool(_) => {}
        Value::Null => bail!("experience preference setting `{key}` cannot be null"),
        Value::Number(_) | Value::Array(_) | Value::Object(_) => {
            bail!("experience preference setting `{key}` must be a string or boolean")
        }
    }
    Ok(())
}

fn reject_sensitive_text(value: &str) -> Result<()> {
    let lower = value.to_ascii_lowercase();
    if lower.contains("bearer ")
        || lower.contains("api_key")
        || lower.contains("api key")
        || lower.contains("private_fixture_term")
        || lower.contains("configured_private_term")
        || lower.contains("sk-")
    {
        bail!("experience preferences cannot store secrets or private fixture terms");
    }
    if looks_like_email(value) {
        bail!("experience preferences cannot store raw email addresses");
    }
    if looks_like_phone(value) {
        bail!("experience preferences cannot store raw phone numbers");
    }
    Ok(())
}

fn looks_like_email(value: &str) -> bool {
    let Some((local, domain)) = value.split_once('@') else {
        return false;
    };
    !local.is_empty() && domain.contains('.') && !domain.ends_with('.')
}

fn looks_like_phone(value: &str) -> bool {
    value
        .chars()
        .filter(|character| character.is_ascii_digit())
        .count()
        >= 10
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::init_schema;

    fn connection() -> Connection {
        let connection = Connection::open_in_memory().unwrap();
        init_schema(&connection).unwrap();
        connection
    }

    #[test]
    fn saves_and_loads_actor_experience_preferences_deterministically() {
        let connection = connection();
        let record = save_actor_experience_preferences(
            &connection,
            "actor_local_owner",
            r#"{
                "typeScale":"lg",
                "contrast":"high",
                "motion":"off",
                "colorBlindMode":"deuteranopia",
                "density":"relaxed",
                "theme":"high_contrast",
                "locale":"en-US",
                "performanceMode":"economy"
            }"#,
            "2026-05-10T00:00:00Z",
        )
        .unwrap();

        assert_eq!(record.actor_id, "actor_local_owner");
        assert_eq!(record.schema_version, EXPERIENCE_PREFERENCES_SCHEMA_VERSION);
        assert_eq!(record.created_at, "2026-05-10T00:00:00Z");
        assert_eq!(record.updated_at, "2026-05-10T00:00:00Z");
        assert_eq!(
            record.requested_settings_json,
            r#"{"colorBlindMode":"deuteranopia","contrast":"high","density":"relaxed","locale":"en-US","motion":"off","performanceMode":"economy","theme":"high_contrast","typeScale":"lg"}"#
        );

        let loaded = load_actor_experience_preferences(&connection, "actor_local_owner")
            .unwrap()
            .unwrap();
        assert_eq!(loaded, record);
    }

    #[test]
    fn updates_requested_settings_without_losing_created_at() {
        let connection = connection();
        save_actor_experience_preferences(
            &connection,
            "actor_local_owner",
            r#"{"theme":"minimal"}"#,
            "2026-05-10T00:00:00Z",
        )
        .unwrap();
        let updated = save_actor_experience_preferences(
            &connection,
            "actor_local_owner",
            r#"{"theme":"high_contrast","motion":"off"}"#,
            "2026-05-11T00:00:00Z",
        )
        .unwrap();

        assert_eq!(updated.created_at, "2026-05-10T00:00:00Z");
        assert_eq!(updated.updated_at, "2026-05-11T00:00:00Z");
        assert_eq!(
            updated.requested_settings_json,
            r#"{"motion":"off","theme":"high_contrast"}"#
        );
    }

    #[test]
    fn rejects_malformed_or_private_preference_payloads() {
        assert!(canonicalize_requested_settings_json(r#"{"unknown":"value"}"#).is_err());
        assert!(canonicalize_requested_settings_json(r#"{"theme":null}"#).is_err());
        assert!(canonicalize_requested_settings_json(r#"{"theme":"user@example.com"}"#).is_err());
        assert!(canonicalize_requested_settings_json(r#"{"theme":"+1 555 123 4567"}"#).is_err());
        assert!(canonicalize_requested_settings_json(r#"{"theme":"Bearer abc"}"#).is_err());
        assert!(
            canonicalize_requested_settings_json(r#"{"theme":"private_fixture_term"}"#).is_err()
        );
    }

    #[test]
    fn empty_actor_load_is_anonymous_default_case() {
        let connection = connection();
        let loaded = load_actor_experience_preferences(&connection, "").unwrap();

        assert_eq!(loaded, None);
    }
}
