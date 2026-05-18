use crate::schema::db::ConnectionExt;
use anyhow::Result;
use chrono::Utc;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::Path;
use uuid::Uuid;

const DEFAULT_LOG_LIMIT: usize = 100;
const MAX_LOG_LIMIT: usize = 500;
const RETAIN_LOG_ROWS: i64 = 2_000;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticLogEntry {
    pub id: String,
    pub timestamp: String,
    pub level: String,
    pub source: String,
    pub message: String,
    pub request_id: Option<String>,
    pub job_id: Option<String>,
    pub task_key: Option<String>,
    pub capability_id: Option<String>,
    pub event_type: Option<String>,
    pub error_code: Option<String>,
    pub duration_ms: Option<i64>,
    pub payload: Value,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticLogQuery {
    pub level: Option<String>,
    pub source: Option<String>,
    pub job_id: Option<String>,
    pub task_key: Option<String>,
    pub capability_id: Option<String>,
    pub since: Option<String>,
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticLogsResponse {
    pub logs: Vec<DiagnosticLogEntry>,
}

#[derive(Debug, Clone)]
pub struct NewDiagnosticLogEntry {
    pub level: String,
    pub source: String,
    pub message: String,
    pub request_id: Option<String>,
    pub job_id: Option<String>,
    pub task_key: Option<String>,
    pub capability_id: Option<String>,
    pub event_type: Option<String>,
    pub error_code: Option<String>,
    pub duration_ms: Option<i64>,
    pub payload: Value,
}

pub fn diagnostic_log(
    level: &str,
    source: &str,
    message: impl Into<String>,
    payload: Value,
) -> NewDiagnosticLogEntry {
    NewDiagnosticLogEntry {
        level: level.to_string(),
        source: source.to_string(),
        message: message.into(),
        request_id: None,
        job_id: None,
        task_key: None,
        capability_id: None,
        event_type: None,
        error_code: None,
        duration_ms: None,
        payload,
    }
}

pub fn record_diagnostic_log(db_path: &Path, entry: NewDiagnosticLogEntry) -> Result<String> {
    let connection = Connection::open(db_path)?;
    insert_diagnostic_log_connection(&connection, entry)
}

pub fn insert_diagnostic_log_connection(
    connection: &Connection,
    entry: NewDiagnosticLogEntry,
) -> Result<String> {
    let id = format!("log_{}", Uuid::new_v4());
    connection.execute(
        "INSERT INTO diagnostic_logs (
            id, timestamp, level, source, message, request_id, job_id, task_key,
            capability_id, event_type, error_code, duration_ms, payload_json
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
        params![
            id,
            Utc::now().to_rfc3339(),
            normalize_level(&entry.level),
            entry.source,
            entry.message,
            entry.request_id,
            entry.job_id,
            entry.task_key,
            entry.capability_id,
            entry.event_type,
            entry.error_code,
            entry.duration_ms,
            sanitize_payload(entry.payload).to_string(),
        ],
    )?;
    cap_log_retention(connection)?;
    Ok(id)
}

pub fn list_diagnostic_logs(
    db_path: &Path,
    query: DiagnosticLogQuery,
) -> Result<DiagnosticLogsResponse> {
    let connection = Connection::open(db_path)?;
    Ok(DiagnosticLogsResponse {
        logs: query_diagnostic_logs(&connection, &query)?,
    })
}

pub fn query_diagnostic_logs(
    connection: &Connection,
    query: &DiagnosticLogQuery,
) -> Result<Vec<DiagnosticLogEntry>> {
    let limit = query.limit.unwrap_or(DEFAULT_LOG_LIMIT).min(MAX_LOG_LIMIT);
    connection.query_many(
        "SELECT id, timestamp, level, source, message, request_id, job_id, task_key,
                capability_id, event_type, error_code, duration_ms, payload_json
         FROM diagnostic_logs
         WHERE (?1 IS NULL OR level = ?1)
           AND (?2 IS NULL OR source = ?2)
           AND (?3 IS NULL OR job_id = ?3)
           AND (?4 IS NULL OR task_key = ?4)
           AND (?5 IS NULL OR capability_id = ?5)
           AND (?6 IS NULL OR timestamp >= ?6)
         ORDER BY timestamp DESC, id DESC
         LIMIT ?7",
        params![
            query.level.as_deref().map(normalize_level),
            query.source,
            query.job_id,
            query.task_key,
            query.capability_id,
            query.since,
            limit as i64,
        ],
        diagnostic_log_from_row,
    )
}

fn diagnostic_log_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<DiagnosticLogEntry> {
    let payload_json: String = row.get(12)?;
    Ok(DiagnosticLogEntry {
        id: row.get(0)?,
        timestamp: row.get(1)?,
        level: row.get(2)?,
        source: row.get(3)?,
        message: row.get(4)?,
        request_id: row.get(5)?,
        job_id: row.get(6)?,
        task_key: row.get(7)?,
        capability_id: row.get(8)?,
        event_type: row.get(9)?,
        error_code: row.get(10)?,
        duration_ms: row.get(11)?,
        payload: serde_json::from_str(&payload_json).unwrap_or_else(|_| json!({})),
    })
}

fn normalize_level(level: &str) -> String {
    match level.to_ascii_lowercase().as_str() {
        "error" => "error".to_string(),
        "warn" | "warning" => "warn".to_string(),
        "debug" => "debug".to_string(),
        _ => "info".to_string(),
    }
}

fn sanitize_payload(payload: Value) -> Value {
    match payload {
        Value::Object(map) => Value::Object(
            map.into_iter()
                .map(|(key, value)| {
                    if is_sensitive_key(&key) {
                        (key, Value::String("[redacted]".to_string()))
                    } else {
                        (key, sanitize_payload(value))
                    }
                })
                .collect(),
        ),
        Value::Array(values) => Value::Array(values.into_iter().map(sanitize_payload).collect()),
        other => other,
    }
}

fn is_sensitive_key(key: &str) -> bool {
    let lower = key.to_ascii_lowercase();
    lower.contains("token")
        || lower.contains("secret")
        || lower.contains("password")
        || lower.contains("apikey")
        || lower.contains("api_key")
        || lower.contains("api-key")
        || lower.contains("vaultkey")
        || lower.contains("vault_key")
}

fn cap_log_retention(connection: &Connection) -> Result<()> {
    connection.execute(
        "DELETE FROM diagnostic_logs
         WHERE id IN (
            SELECT id FROM diagnostic_logs
            ORDER BY timestamp DESC, id DESC
            LIMIT -1 OFFSET ?1
         )",
        [RETAIN_LOG_ROWS],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::init_schema;

    #[test]
    fn diagnostic_logs_are_persisted_and_queryable() {
        let connection = Connection::open_in_memory().unwrap();
        init_schema(&connection).unwrap();
        insert_diagnostic_log_connection(
            &connection,
            NewDiagnosticLogEntry {
                capability_id: Some("backup.create".to_string()),
                ..diagnostic_log(
                    "warning",
                    "backup",
                    "Backup warning",
                    json!({
                        "token": "secret",
                        "apiKey": "secret-api-key",
                        "vaultKey": "secret-vault-key",
                        "count": 1
                    }),
                )
            },
        )
        .unwrap();

        let logs = query_diagnostic_logs(
            &connection,
            &DiagnosticLogQuery {
                level: Some("warn".to_string()),
                source: None,
                job_id: None,
                task_key: None,
                capability_id: Some("backup.create".to_string()),
                since: None,
                limit: Some(10),
            },
        )
        .unwrap();

        assert_eq!(logs.len(), 1);
        assert_eq!(logs[0].level, "warn");
        assert_eq!(logs[0].payload["token"], "[redacted]");
        assert_eq!(logs[0].payload["apiKey"], "[redacted]");
        assert_eq!(logs[0].payload["vaultKey"], "[redacted]");
    }
}
