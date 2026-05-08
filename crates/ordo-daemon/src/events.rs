use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RealtimeEvent {
    pub schema_version: String,
    pub family: String,
    pub event_type: String,
    pub job_id: Option<String>,
    pub task_key: Option<String>,
    pub sequence: Option<i64>,
    pub payload: Value,
    pub occurred_at: String,
}

pub fn system_event(event_type: &str, payload: Value) -> RealtimeEvent {
    RealtimeEvent {
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
        assert_eq!(serialized["family"], "job");
        assert_eq!(serialized["eventType"], "task.succeeded");
        assert_eq!(serialized["jobId"], "job_1");
        assert_eq!(serialized["taskKey"], "health.probe");
        assert_eq!(serialized["sequence"], 4);
    }
}
