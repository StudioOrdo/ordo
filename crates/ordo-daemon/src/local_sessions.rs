use anyhow::{bail, Result};
use chrono::{Duration, Utc};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::path::Path;

use crate::events::{system_event, RealtimeEvent};

const LOCAL_SESSION_SCHEMA_VERSION: &str = "ordo.local-session.v1";
const LOCAL_SESSION_TTL_DAYS: i64 = 30;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalSessionCreateRequest {
    pub mode: String,
    pub name: Option<String>,
    pub email: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalSessionView {
    pub schema_version: String,
    pub session_kind: String,
    pub session_id: String,
    pub actor_id: String,
    pub role: String,
    pub display_name: String,
    pub email_hash: String,
    pub issued_at: String,
    pub expires_at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalSessionResponse {
    pub session: LocalSessionView,
}

pub fn create_or_restore_local_session(
    db_path: &Path,
    request: LocalSessionCreateRequest,
) -> Result<(LocalSessionResponse, RealtimeEvent)> {
    let normalized_email = normalize_email(&request.email)?;
    validate_password(&request.password)?;
    let display_name = if request.mode == "register" {
        normalize_display_name(request.name.as_deref())?
    } else if request.mode == "login" {
        display_name_from_email(&normalized_email)
    } else {
        bail!("unsupported local session mode");
    };

    let email_hash = hash_hex(&normalized_email);
    let session_id = format!("local_session_{}", &email_hash[..32]);
    let actor_id = format!("actor_local_member_{}", &email_hash[..16]);
    let now = Utc::now();
    let issued_at = now.to_rfc3339();
    let expires_at = (now + Duration::days(LOCAL_SESSION_TTL_DAYS)).to_rfc3339();

    let connection = Connection::open(db_path)?;
    connection.execute(
        "INSERT INTO actors (id, actor_kind, display_name, status, metadata_json, created_at, updated_at)
         VALUES (?1, 'browser_operator', ?2, 'active', ?3, ?4, ?4)
         ON CONFLICT(id) DO UPDATE SET display_name = excluded.display_name, updated_at = excluded.updated_at",
        params![
            actor_id,
            display_name,
            json!({ "source": "local_account_session", "emailHash": email_hash }).to_string(),
            issued_at,
        ],
    )?;
    connection.execute(
        "INSERT INTO local_account_sessions (
            session_id, actor_id, email_hash, display_name, role, issued_at, expires_at, last_seen_at, metadata_json
         ) VALUES (?1, ?2, ?3, ?4, 'client', ?5, ?6, ?5, ?7)
         ON CONFLICT(email_hash) DO UPDATE SET
            session_id = excluded.session_id,
            actor_id = excluded.actor_id,
            display_name = excluded.display_name,
            role = excluded.role,
            issued_at = excluded.issued_at,
            expires_at = excluded.expires_at,
            last_seen_at = excluded.last_seen_at,
            metadata_json = excluded.metadata_json",
        params![
            session_id,
            actor_id,
            email_hash,
            display_name,
            issued_at,
            expires_at,
            json!({ "mode": request.mode, "secretHandling": "password_not_stored" }).to_string(),
        ],
    )?;

    let session = read_local_session_by_email_hash(&connection, &email_hash)?
        .expect("local session just inserted");
    let event = system_event(
        "local_session.established",
        json!({
            "sessionId": session.session_id,
            "actorId": session.actor_id,
            "role": session.role,
            "emailHash": session.email_hash,
        }),
    );

    Ok((LocalSessionResponse { session }, event))
}

fn read_local_session_by_email_hash(
    connection: &Connection,
    email_hash: &str,
) -> Result<Option<LocalSessionView>> {
    connection
        .query_row(
            "SELECT session_id, actor_id, email_hash, display_name, role, issued_at, expires_at
             FROM local_account_sessions
             WHERE email_hash = ?1",
            [email_hash],
            |row| {
                Ok(LocalSessionView {
                    schema_version: LOCAL_SESSION_SCHEMA_VERSION.to_string(),
                    session_kind: "local_appliance_session".to_string(),
                    session_id: row.get(0)?,
                    actor_id: row.get(1)?,
                    email_hash: row.get(2)?,
                    display_name: row.get(3)?,
                    role: row.get(4)?,
                    issued_at: row.get(5)?,
                    expires_at: row.get(6)?,
                })
            },
        )
        .optional()
        .map_err(Into::into)
}

fn normalize_email(value: &str) -> Result<String> {
    let email = value.trim().to_lowercase();
    if email.len() < 3 || email.len() > 254 || email.chars().any(char::is_whitespace) {
        bail!("Enter a valid email address.");
    }
    let Some((local, domain)) = email.split_once('@') else {
        bail!("Enter a valid email address.");
    };
    if local.is_empty() || !domain.contains('.') || domain.ends_with('.') {
        bail!("Enter a valid email address.");
    }
    Ok(email)
}

fn validate_password(value: &str) -> Result<()> {
    let password = value.trim();
    if password.len() < 8 || password.len() > 128 {
        bail!("Enter a local session password with at least 8 characters.");
    }
    Ok(())
}

fn normalize_display_name(value: Option<&str>) -> Result<String> {
    let name = value
        .unwrap_or_default()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    if name.is_empty() || name.len() > 80 {
        bail!("Enter a display name for this local appliance session.");
    }
    Ok(name)
}

fn display_name_from_email(email: &str) -> String {
    email
        .split('@')
        .next()
        .unwrap_or("Local member")
        .replace(['.', '_', '-'], " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn hash_hex(value: &str) -> String {
    let digest = Sha256::digest(value.as_bytes());
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::init_database;
    use tempfile::tempdir;

    #[test]
    fn local_session_persists_without_storing_password() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("ordo.sqlite3");
        init_database(&db_path).unwrap();

        let (response, event) = create_or_restore_local_session(
            &db_path,
            LocalSessionCreateRequest {
                mode: "register".to_string(),
                name: Some("Ava Client".to_string()),
                email: "AVA@example.com".to_string(),
                password: "local-only-pass".to_string(),
            },
        )
        .unwrap();

        assert_eq!(response.session.display_name, "Ava Client");
        assert_eq!(response.session.role, "client");
        assert_eq!(event.event_type, "local_session.established");

        let connection = Connection::open(&db_path).unwrap();
        let stored_metadata: String = connection
            .query_row(
                "SELECT metadata_json FROM local_account_sessions WHERE session_id = ?1",
                [&response.session.session_id],
                |row| row.get(0),
            )
            .unwrap();
        assert!(!stored_metadata.contains("local-only-pass"));
        assert!(!stored_metadata.contains("AVA@example.com"));
    }

    #[test]
    fn repeated_login_restores_same_local_actor() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("ordo.sqlite3");
        init_database(&db_path).unwrap();

        let request = LocalSessionCreateRequest {
            mode: "login".to_string(),
            name: None,
            email: "ava@example.com".to_string(),
            password: "local-only-pass".to_string(),
        };
        let (first, _) = create_or_restore_local_session(&db_path, request.clone()).unwrap();
        let (second, _) = create_or_restore_local_session(&db_path, request).unwrap();

        assert_eq!(first.session.actor_id, second.session.actor_id);
        assert_eq!(first.session.session_id, second.session.session_id);
    }

    #[test]
    fn invalid_local_session_input_returns_safe_error() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("ordo.sqlite3");
        init_database(&db_path).unwrap();

        let error = create_or_restore_local_session(
            &db_path,
            LocalSessionCreateRequest {
                mode: "register".to_string(),
                name: Some(" ".to_string()),
                email: "not-an-email".to_string(),
                password: "secret-value".to_string(),
            },
        )
        .unwrap_err()
        .to_string();

        assert!(!error.contains("secret-value"));
        assert!(!error.contains("not-an-email"));
    }
}
