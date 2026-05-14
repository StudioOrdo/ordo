use super::jobs::*;
use super::types::*;
use crate::diagnostics::{query_diagnostic_logs, DiagnosticLogQuery};
use crate::health::{build_health_report, build_readiness_report};
use anyhow::Result;
use rusqlite::Connection;
use serde_json::{json, Value};
use std::path::Path;

pub(crate) fn health_evidence(_db_path: &Path) -> EvidenceEnvelope {
    let health = build_health_report();
    envelope(
        "health",
        "succeeded",
        format!("Daemon health is {}.", health.status),
        serde_json::to_value(health).unwrap_or_else(|_| json!({})),
    )
}

pub(crate) fn readiness_evidence(db_path: &Path) -> EvidenceEnvelope {
    let readiness = build_readiness_report(db_path);
    envelope(
        "readiness",
        "succeeded",
        format!("Daemon readiness is {}.", readiness.status),
        serde_json::to_value(readiness).unwrap_or_else(|_| json!({})),
    )
}

pub(crate) fn events_evidence(connection: &Connection) -> Result<EvidenceEnvelope> {
    let events = query_json_rows(
        connection,
        "SELECT cursor, family, event_type, job_id, task_key, occurred_at, payload_json
         FROM realtime_events ORDER BY cursor DESC LIMIT 25",
        |row: &rusqlite::Row| {
            Ok(json!({
                "cursor": row.get::<_, i64>(0)?,
                "family": row.get::<_, String>(1)?,
                "eventType": row.get::<_, String>(2)?,
                "jobId": row.get::<_, Option<String>>(3)?,
                "taskKey": row.get::<_, Option<String>>(4)?,
                "occurredAt": row.get::<_, String>(5)?,
                "payload": parse_json_column(row.get::<_, String>(6)?),
            }))
        },
    )?;
    let mut entry = envelope(
        "recent_events",
        "succeeded",
        format!("Collected {} recent persisted events.", events.len()),
        json!({ "events": events }),
    );
    entry.limits = json!({ "maxEvents": 25 });
    Ok(entry)
}

pub(crate) fn jobs_evidence(connection: &Connection) -> Result<EvidenceEnvelope> {
    let jobs = query_json_rows(
        connection,
        "SELECT id, template_id, capability_id, kind, status, current_task_key, created_at, updated_at, failure_message
         FROM jobs ORDER BY updated_at DESC LIMIT 25",
        |row: &rusqlite::Row| {
            Ok(json!({
                "id": row.get::<_, String>(0)?,
                "templateId": row.get::<_, String>(1)?,
                "capabilityId": row.get::<_, String>(2)?,
                "kind": row.get::<_, String>(3)?,
                "status": row.get::<_, String>(4)?,
                "currentTaskKey": row.get::<_, Option<String>>(5)?,
                "createdAt": row.get::<_, String>(6)?,
                "updatedAt": row.get::<_, String>(7)?,
                "failureMessage": row.get::<_, Option<String>>(8)?,
            }))
        },
    )?;
    let mut entry = envelope(
        "recent_jobs",
        "succeeded",
        format!("Collected {} recent jobs.", jobs.len()),
        json!({ "jobs": jobs }),
    );
    entry.limits = json!({ "maxJobs": 25 });
    Ok(entry)
}

pub(crate) fn logs_evidence(connection: &Connection) -> Result<EvidenceEnvelope> {
    let logs = query_diagnostic_logs(
        connection,
        &DiagnosticLogQuery {
            level: None,
            source: None,
            job_id: None,
            task_key: None,
            capability_id: None,
            since: None,
            limit: Some(50),
        },
    )?;
    let mut entry = envelope(
        "diagnostic_logs",
        "succeeded",
        format!("Collected {} recent structured logs.", logs.len()),
        json!({ "logs": logs }),
    );
    entry.limits = json!({ "maxLogs": 50 });
    entry
        .redactions
        .push("Sensitive payload keys are redacted during log capture.".to_string());
    Ok(entry)
}

pub(crate) fn browser_context_evidence(browser_context: Option<Value>) -> EvidenceEnvelope {
    match browser_context {
        Some(payload) => envelope(
            "browser_runtime",
            "succeeded",
            "Browser diagnostic context was attached by the UI.",
            payload,
        ),
        None => {
            let mut entry = envelope(
                "browser_runtime",
                "skipped",
                "Browser diagnostic context was requested but no envelope was submitted.",
                json!({}),
            );
            entry
                .errors
                .push("No browser diagnostic envelope provided.".to_string());
            entry
        }
    }
}
