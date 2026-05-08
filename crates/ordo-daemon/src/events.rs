use anyhow::Result;
use chrono::Utc;
use rusqlite::{params, Connection, Transaction};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::Path;

const DEFAULT_EVENT_REPLAY_LIMIT: usize = 100;
const MAX_EVENT_REPLAY_LIMIT: usize = 500;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RealtimeEvent {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor: Option<i64>,
    pub schema_version: String,
    pub family: String,
    pub event_type: String,
    pub job_id: Option<String>,
    pub task_key: Option<String>,
    pub sequence: Option<i64>,
    pub payload: Value,
    pub occurred_at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EventReplayResponse {
    pub events: Vec<RealtimeEvent>,
    pub next_cursor: Option<i64>,
}

pub fn system_event(event_type: &str, payload: Value) -> RealtimeEvent {
    RealtimeEvent {
        cursor: None,
        schema_version: "1".to_string(),
        family: "system".to_string(),
        event_type: event_type.to_string(),
        job_id: None,
        task_key: None,
        sequence: None,
        payload,
        occurred_at: Utc::now().to_rfc3339(),
    }
}

pub fn job_event(
    event_type: &str,
    job_id: &str,
    task_key: Option<&str>,
    sequence: i64,
    payload: Value,
) -> RealtimeEvent {
    RealtimeEvent {
        cursor: None,
        schema_version: "1".to_string(),
        family: "job".to_string(),
        event_type: event_type.to_string(),
        job_id: Some(job_id.to_string()),
        task_key: task_key.map(str::to_string),
        sequence: Some(sequence),
        payload,
        occurred_at: Utc::now().to_rfc3339(),
    }
}

pub fn append_realtime_event(
    connection: &Connection,
    event: &RealtimeEvent,
) -> Result<RealtimeEvent> {
    connection.execute(
        "INSERT INTO realtime_events (
            schema_version, family, event_type, job_id, task_key, job_sequence, payload_json, occurred_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            event.schema_version,
            event.family,
            event.event_type,
            event.job_id,
            event.task_key,
            event.sequence,
            event.payload.to_string(),
            event.occurred_at,
        ],
    )?;
    Ok(event_with_cursor(event, connection.last_insert_rowid()))
}

pub fn append_realtime_event_tx(
    transaction: &Transaction,
    event: &RealtimeEvent,
) -> Result<RealtimeEvent> {
    transaction.execute(
        "INSERT INTO realtime_events (
            schema_version, family, event_type, job_id, task_key, job_sequence, payload_json, occurred_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            event.schema_version,
            event.family,
            event.event_type,
            event.job_id,
            event.task_key,
            event.sequence,
            event.payload.to_string(),
            event.occurred_at,
        ],
    )?;
    Ok(event_with_cursor(event, transaction.last_insert_rowid()))
}

pub fn append_system_event(
    db_path: &Path,
    event_type: &str,
    payload: Value,
) -> Result<RealtimeEvent> {
    let connection = Connection::open(db_path)?;
    append_realtime_event(&connection, &system_event(event_type, payload))
}

pub fn replay_events(
    db_path: &Path,
    after: Option<i64>,
    limit: Option<usize>,
) -> Result<EventReplayResponse> {
    let connection = Connection::open(db_path)?;
    let after_cursor = after.unwrap_or(0).max(0);
    let event_limit = limit
        .unwrap_or(DEFAULT_EVENT_REPLAY_LIMIT)
        .clamp(1, MAX_EVENT_REPLAY_LIMIT);
    let mut statement = connection.prepare(
        "SELECT cursor, schema_version, family, event_type, job_id, task_key, job_sequence, payload_json, occurred_at
         FROM realtime_events
         WHERE cursor > ?1
         ORDER BY cursor ASC
         LIMIT ?2",
    )?;
    let events = statement
        .query_map(params![after_cursor, event_limit as i64], event_from_row)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let next_cursor = events.last().and_then(|event| event.cursor);
    Ok(EventReplayResponse {
        events,
        next_cursor,
    })
}

fn event_with_cursor(event: &RealtimeEvent, cursor: i64) -> RealtimeEvent {
    let mut persisted = event.clone();
    persisted.cursor = Some(cursor);
    persisted
}

fn event_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<RealtimeEvent> {
    let payload_json: String = row.get(7)?;
    Ok(RealtimeEvent {
        cursor: Some(row.get(0)?),
        schema_version: row.get(1)?,
        family: row.get(2)?,
        event_type: row.get(3)?,
        job_id: row.get(4)?,
        task_key: row.get(5)?,
        sequence: row.get(6)?,
        payload: serde_json::from_str(&payload_json).unwrap_or_else(|_| json!({})),
        occurred_at: row.get(8)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn serializes_job_event_shape() {
        let event = job_event(
            "task.succeeded",
            "job_1",
            Some("health.probe"),
            4,
            json!({ "ok": true }),
        );
        let serialized = serde_json::to_value(event).unwrap();

        assert_eq!(serialized["schemaVersion"], "1");
        assert!(serialized.get("cursor").is_none());
        assert_eq!(serialized["family"], "job");
        assert_eq!(serialized["eventType"], "task.succeeded");
        assert_eq!(serialized["jobId"], "job_1");
        assert_eq!(serialized["taskKey"], "health.probe");
        assert_eq!(serialized["sequence"], 4);
    }

    #[test]
    fn replay_after_cursor_returns_reconnect_gap() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let db_path = temp_dir.path().join("local.db");
        let connection = Connection::open(&db_path).unwrap();
        crate::schema::init_schema(&connection).unwrap();

        let first =
            append_realtime_event(&connection, &system_event("daemon.started", json!({}))).unwrap();
        let second = append_realtime_event(
            &connection,
            &job_event("job.created", "job_1", None, 1, json!({ "ok": true })),
        )
        .unwrap();

        assert_eq!(first.cursor, Some(1));
        assert_eq!(second.cursor, Some(2));

        let replay = replay_events(&db_path, first.cursor, Some(10)).unwrap();

        assert_eq!(replay.events.len(), 1);
        assert_eq!(replay.next_cursor, Some(2));
        assert_eq!(replay.events[0].cursor, Some(2));
        assert_eq!(replay.events[0].event_type, "job.created");
    }
}
