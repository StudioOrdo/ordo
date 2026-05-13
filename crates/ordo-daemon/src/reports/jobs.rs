use anyhow::{bail, Context, Result};
use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use serde_json::{json, Value};
use super::types::*;
use super::evidence::*;
use super::api::*;
use crate::schema::db::ConnectionExt;
use crate::templates::require_builtin_template;
use crate::kernel::{append_job_event, create_job_from_template};
use crate::diagnostics::{insert_diagnostic_log_connection, diagnostic_log};
use crate::policy::ResourceClassification;
use uuid::Uuid;
use sha2::{Digest, Sha256};

pub fn prepare_issue_report(
    db_path: &Path,
    request: IssueReportPrepareRequest,
    origin: &str,
    actor_id: Option<&str>,
) -> Result<IssueReportArtifact> {
    let mut connection = Connection::open(db_path)?;
    let normalized = normalize_request(request)?;
    let template = require_builtin_template(ISSUE_REPORT_TEMPLATE_ID)?;
    let job_id = create_job_from_template(
        &mut connection,
        &template,
        origin,
        actor_id,
        json!({
            "title": normalized.title,
            "severity": normalized.severity.as_str(),
            "sourceRoute": normalized.source_route,
            "evidenceSources": selected_sources(&normalized),
        }),
    )?;

    match complete_issue_report_job(&connection, db_path, &job_id, normalized, origin, actor_id) {
        Ok(report) => Ok(report),
        Err(error) => {
            mark_job_failed(&connection, &job_id, &error.to_string())?;
            Err(error)
        }
    }
}

pub(crate) fn complete_issue_report_job(
    connection: &Connection,
    db_path: &Path,
    job_id: &str,
    request: NormalizedIssueReportRequest,
    origin: &str,
    actor_id: Option<&str>,
) -> Result<IssueReportArtifact> {
    set_job_running(connection, job_id)?;
    run_task(
        connection,
        job_id,
        "scope.validate",
        json!({ "valid": true, "evidenceSources": selected_sources(&request) }),
    )?;
    run_task(
        connection,
        job_id,
        "narrative.capture",
        json!({
            "title": request.title,
            "severity": request.severity.as_str(),
            "descriptionLength": request.description.len(),
            "steps": request.steps.len(),
        }),
    )?;

    let mut evidence = Vec::new();
    if request.include_health_snapshot {
        evidence.push(health_evidence(db_path));
        evidence.push(readiness_evidence(db_path));
    }
    if request.include_recent_events {
        evidence.push(events_evidence(connection)?);
    }
    if request.include_recent_jobs {
        evidence.push(jobs_evidence(connection)?);
    }
    if request.include_diagnostic_logs {
        evidence.push(logs_evidence(connection)?);
    }
    if request.include_browser_context {
        evidence.push(browser_context_evidence(request.browser_context.clone()));
    }

    run_task(
        connection,
        job_id,
        "diagnostics.collect",
        json!({ "sources": evidence.iter().map(|entry| entry.source.clone()).collect::<Vec<_>>() }),
    )?;
    run_task(
        connection,
        job_id,
        "events.collect",
        json!({ "included": request.include_recent_events }),
    )?;
    run_task(
        connection,
        job_id,
        "jobs.collect",
        json!({ "included": request.include_recent_jobs }),
    )?;

    let redactions = vec![
        "Secrets, tokens, passwords, and unsafe diagnostic fields are redacted before storage."
            .to_string(),
        "External submission transports are not enabled in Reports 1.0.".to_string(),
    ];
    run_task(
        connection,
        job_id,
        "redactions.apply",
        json!({ "redactions": redactions }),
    )?;

    let diagnostics = json!({
        "schemaVersion": 1,
        "evidenceSources": selected_sources(&request),
        "evidenceCount": evidence.len(),
        "localOnly": true,
        "externalSubmission": "not_implemented",
        "classification": ResourceClassification::local_operations_ready_for_review(),
    });
    let markdown = render_report_markdown(&request, &evidence, &redactions);
    run_task(
        connection,
        job_id,
        "draft.generate",
        json!({ "markdownBytes": markdown.len(), "evidenceCount": evidence.len() }),
    )?;

    mark_task_running(connection, job_id, "artifact.save")?;
    let report = insert_issue_report_artifact(
        connection,
        job_id,
        &request,
        markdown,
        diagnostics,
        evidence,
        redactions,
    )?;
    insert_job_artifact(
        connection,
        job_id,
        Some("artifact.save"),
        "issue.report",
        &format!("ordo://reports/issues/{}", report.id),
        "Issue report",
        json!({
            "reportId": report.id,
            "severity": report.severity,
            "status": report.status,
            "provenance": report_provenance_metadata(job_id, &report.id, origin, actor_id),
        }),
    )?;
    append_job_event(
        connection,
        job_id,
        Some("artifact.save"),
        "issue.report.artifact.created",
        json!({ "reportId": report.id, "severity": report.severity }),
    )?;
    mark_task_succeeded(
        connection,
        job_id,
        "artifact.save",
        json!({ "reportId": report.id, "status": report.status }),
    )?;
    mark_job_succeeded(
        connection,
        job_id,
        json!({ "reportId": report.id, "status": report.status }),
    )?;
    insert_diagnostic_log_connection(
        connection,
        NewReportLog::prepared(&report.id, job_id, &report.severity),
    )?;
    Ok(report)
}

struct NewReportLog;

impl NewReportLog {
    fn prepared(
        report_id: &str,
        job_id: &str,
        severity: &str,
    ) -> crate::diagnostics::NewDiagnosticLogEntry {
        crate::diagnostics::NewDiagnosticLogEntry {
            job_id: Some(job_id.to_string()),
            capability_id: Some("issue.report.prepare".to_string()),
            event_type: Some("issue.report.prepared".to_string()),
            ..diagnostic_log(
                "info",
                "reports",
                "Issue report prepared.",
                json!({ "reportId": report_id, "severity": severity }),
            )
        }
    }
}

pub(crate) fn normalize_request(request: IssueReportPrepareRequest) -> Result<NormalizedIssueReportRequest> {
    let description = request.description.trim().to_string();
    if description.is_empty() {
        bail!("Issue report description is required");
    }
    let title = request
        .title
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("Local diagnostic report")
        .to_string();
    Ok(NormalizedIssueReportRequest {
        title,
        severity: request.severity.unwrap_or_default(),
        description,
        expected_behavior: non_empty_optional(request.expected_behavior),
        actual_behavior: non_empty_optional(request.actual_behavior),
        steps: request
            .steps
            .unwrap_or_default()
            .into_iter()
            .map(|step| step.trim().to_string())
            .filter(|step| !step.is_empty())
            .collect(),
        source_route: non_empty_optional(request.source_route),
        include_health_snapshot: request.include_health_snapshot.unwrap_or(true),
        include_recent_events: request.include_recent_events.unwrap_or(true),
        include_recent_jobs: request.include_recent_jobs.unwrap_or(true),
        include_diagnostic_logs: request.include_diagnostic_logs.unwrap_or(true),
        include_browser_context: request.include_browser_context.unwrap_or(false),
        browser_context: request.browser_context,
    })
}

pub(crate) fn non_empty_optional(value: Option<String>) -> Option<String> {
    value
        .as_deref()
        .map(str::trim)
        .filter(|entry| !entry.is_empty())
        .map(ToString::to_string)
}

pub(crate) fn selected_sources(request: &NormalizedIssueReportRequest) -> Vec<String> {
    let mut sources = Vec::new();
    if request.include_health_snapshot {
        sources.extend(["health".to_string(), "readiness".to_string()]);
    }
    if request.include_recent_events {
        sources.push("recent_events".to_string());
    }
    if request.include_recent_jobs {
        sources.push("recent_jobs".to_string());
    }
    if request.include_diagnostic_logs {
        sources.push("diagnostic_logs".to_string());
    }
    if request.include_browser_context {
        sources.push("browser_runtime".to_string());
    }
    sources
}

pub(crate) fn envelope(
    source: &str,
    status: &str,
    summary: impl Into<String>,
    payload: Value,
) -> EvidenceEnvelope {
    EvidenceEnvelope {
        source: source.to_string(),
        collected_at: Utc::now().to_rfc3339(),
        status: status.to_string(),
        summary: summary.into(),
        payload,
        redactions: Vec::new(),
        limits: json!({}),
        errors: Vec::new(),
    }
}

pub(crate) fn query_json_rows(
    connection: &Connection,
    sql: &str,
    map: impl Fn(&rusqlite::Row<'_>) -> rusqlite::Result<Value>,
) -> Result<Vec<Value>> {
    connection.query_many(sql, [], map)
}

pub(crate) fn parse_json_column(value: String) -> Value {
    serde_json::from_str(&value).unwrap_or_else(|_| json!({}))
}

pub(crate) fn render_report_markdown(
    request: &NormalizedIssueReportRequest,
    evidence: &[EvidenceEnvelope],
    redactions: &[String],
) -> String {
    let mut body = String::new();
    body.push_str(&format!("# {}\n\n", request.title));
    body.push_str(&format!("- Severity: {}\n", request.severity.as_str()));
    body.push_str("- Status: ready_for_review\n");
    body.push_str(
        "- Submission: local only; external transports are not implemented in Reports 1.0\n",
    );
    if let Some(route) = &request.source_route {
        body.push_str(&format!("- Source route: {route}\n"));
    }
    body.push_str("\n## Description\n\n");
    body.push_str(&request.description);
    body.push_str("\n\n## Expected Behavior\n\n");
    body.push_str(
        request
            .expected_behavior
            .as_deref()
            .unwrap_or("Not provided."),
    );
    body.push_str("\n\n## Actual Behavior\n\n");
    body.push_str(
        request
            .actual_behavior
            .as_deref()
            .unwrap_or("Not provided."),
    );
    body.push_str("\n\n## Steps To Reproduce\n\n");
    if request.steps.is_empty() {
        body.push_str("Not provided.\n");
    } else {
        for (index, step) in request.steps.iter().enumerate() {
            body.push_str(&format!("{}. {}\n", index + 1, step));
        }
    }
    body.push_str("\n## Diagnostics Summary\n\n");
    for entry in evidence {
        body.push_str(&format!("- {}: {}\n", entry.source, entry.summary));
    }
    body.push_str("\n## Evidence\n\n");
    for entry in evidence {
        body.push_str(&format!("### {}\n\n", entry.source));
        body.push_str(&format!(
            "Status: {}\n\n{}\n\n",
            entry.status, entry.summary
        ));
    }
    body.push_str("## Redaction Notes\n\n");
    for note in redactions {
        body.push_str(&format!("- {note}\n"));
    }
    body.push_str("\n## Limitations\n\n- This report is prepared and stored locally. External submission transports are future operator-confirmed actions.\n");
    body
}

pub(crate) fn insert_issue_report_artifact(
    connection: &Connection,
    job_id: &str,
    request: &NormalizedIssueReportRequest,
    markdown_body: String,
    diagnostics: Value,
    evidence: Vec<EvidenceEnvelope>,
    redactions: Vec<String>,
) -> Result<IssueReportArtifact> {
    let id = format!("report_{}", Uuid::new_v4());
    let now = Utc::now().to_rfc3339();
    let summary = format!(
        "{} evidence sources collected for a {} severity local report.",
        evidence.len(),
        request.severity.as_str()
    );
    connection.execute(
        "INSERT INTO issue_report_artifacts (
            id, job_id, status, severity, title, summary, description, source_route,
            markdown_body, diagnostics_json, evidence_json, redactions_json, created_at, updated_at,
            exported_at, submitted_at, external_url
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?13, NULL, NULL, NULL)",
        params![
            id,
            job_id,
            IssueReportStatus::ReadyForReview.as_str(),
            request.severity.as_str(),
            request.title,
            summary,
            request.description,
            request.source_route,
            markdown_body,
            diagnostics.to_string(),
            serde_json::to_string(&evidence)?,
            serde_json::to_string(&redactions)?,
            now,
        ],
    )?;
    load_issue_report(connection, &id)?
        .ok_or_else(|| anyhow::anyhow!("Inserted report was not found"))
}

