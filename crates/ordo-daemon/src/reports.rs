use anyhow::{bail, Result};
use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::path::Path;
use uuid::Uuid;

use crate::diagnostics::{
    diagnostic_log, insert_diagnostic_log_connection, query_diagnostic_logs, DiagnosticLogQuery,
};
use crate::health::{build_health_report, build_readiness_report};
use crate::kernel::{append_job_event, create_job_from_template};
use crate::policy::{
    provenance_metadata, ActorContext, ActorKind, PolicyAction, ResourceClassification,
    ResourceKind, ResourceRef,
};
use crate::templates::require_builtin_template;

pub const ISSUE_REPORT_TEMPLATE_ID: &str = "issue.report.prepare";

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IssueSeverity {
    Low,
    #[default]
    Medium,
    High,
    Blocker,
}

impl IssueSeverity {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
            Self::Blocker => "blocker",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IssueReportStatus {
    Draft,
    ReadyForReview,
    Exported,
    Submitted,
    Dismissed,
}

impl IssueReportStatus {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::ReadyForReview => "ready_for_review",
            Self::Exported => "exported",
            Self::Submitted => "submitted",
            Self::Dismissed => "dismissed",
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IssueReportPrepareRequest {
    pub title: Option<String>,
    pub severity: Option<IssueSeverity>,
    pub description: String,
    pub expected_behavior: Option<String>,
    pub actual_behavior: Option<String>,
    pub steps: Option<Vec<String>>,
    pub source_route: Option<String>,
    pub include_health_snapshot: Option<bool>,
    pub include_recent_events: Option<bool>,
    pub include_recent_jobs: Option<bool>,
    pub include_diagnostic_logs: Option<bool>,
    pub include_browser_context: Option<bool>,
    pub browser_context: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EvidenceEnvelope {
    pub source: String,
    pub collected_at: String,
    pub status: String,
    pub summary: String,
    pub payload: Value,
    pub redactions: Vec<String>,
    pub limits: Value,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IssueReportArtifact {
    pub id: String,
    pub job_id: Option<String>,
    pub status: String,
    pub severity: String,
    pub title: String,
    pub summary: String,
    pub description: String,
    pub source_route: Option<String>,
    pub markdown_body: String,
    pub diagnostics: Value,
    pub evidence: Vec<EvidenceEnvelope>,
    pub redactions: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
    pub exported_at: Option<String>,
    pub submitted_at: Option<String>,
    pub external_url: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IssueReportsResponse {
    pub reports: Vec<IssueReportSummary>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IssueReportSummary {
    pub id: String,
    pub job_id: Option<String>,
    pub status: String,
    pub severity: String,
    pub title: String,
    pub summary: String,
    pub source_route: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub exported_at: Option<String>,
    pub submitted_at: Option<String>,
    pub external_url: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IssueReportDetailResponse {
    pub report: IssueReportArtifact,
    pub exports: Vec<IssueReportExportView>,
    pub status_events: Vec<IssueReportStatusEventView>,
    pub support_packets: Vec<SupportPacketView>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IssueReportExportView {
    pub id: String,
    pub report_id: String,
    pub export_format: String,
    pub content_hash: String,
    pub content_bytes: i64,
    pub content_text: String,
    pub created_by_actor_id: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IssueReportStatusEventView {
    pub id: String,
    pub report_id: String,
    pub from_status: Option<String>,
    pub to_status: String,
    pub reason: Option<String>,
    pub created_by_actor_id: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IssueReportStatusUpdateRequest {
    pub status: IssueReportStatus,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IssueReportExportRequest {
    pub export_format: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IssueReportExportResponse {
    pub report: IssueReportArtifact,
    pub export: IssueReportExportView,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SupportPacketDraftRequest {
    pub report_id: String,
    pub destination_kind: Option<String>,
    pub destination_id: Option<String>,
    pub destination_label: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SupportPacketApprovalRequest {
    pub approval_note: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SupportPacketView {
    pub id: String,
    pub report_id: String,
    pub status: String,
    pub destination_kind: String,
    pub destination_id: Option<String>,
    pub destination_label: Option<String>,
    pub payload: Value,
    pub payload_hash: String,
    pub approval_required: bool,
    pub approved_by_actor_id: Option<String>,
    pub approved_at: Option<String>,
    pub created_by_actor_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SupportPacketReceiptView {
    pub id: String,
    pub packet_id: String,
    pub receipt_kind: String,
    pub payload: Value,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SupportPacketListResponse {
    pub packets: Vec<SupportPacketView>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SupportPacketReceiptListResponse {
    pub receipts: Vec<SupportPacketReceiptView>,
}

struct NormalizedIssueReportRequest {
    title: String,
    severity: IssueSeverity,
    description: String,
    expected_behavior: Option<String>,
    actual_behavior: Option<String>,
    steps: Vec<String>,
    source_route: Option<String>,
    include_health_snapshot: bool,
    include_recent_events: bool,
    include_recent_jobs: bool,
    include_diagnostic_logs: bool,
    include_browser_context: bool,
    browser_context: Option<Value>,
}

pub fn list_issue_reports(db_path: &Path) -> Result<IssueReportsResponse> {
    let connection = Connection::open(db_path)?;
    Ok(IssueReportsResponse {
        reports: load_issue_report_summaries(&connection)?,
    })
}

pub fn read_issue_report(db_path: &Path, report_id: &str) -> Result<IssueReportDetailResponse> {
    let connection = Connection::open(db_path)?;
    let report = require_issue_report(&connection, report_id)?;
    Ok(IssueReportDetailResponse {
        exports: load_issue_report_exports(&connection, report_id)?,
        status_events: load_issue_report_status_events(&connection, report_id)?,
        support_packets: load_support_packets_for_report(&connection, report_id)?,
        report,
    })
}

pub fn update_issue_report_status(
    db_path: &Path,
    report_id: &str,
    request: IssueReportStatusUpdateRequest,
    actor_id: Option<&str>,
) -> Result<IssueReportDetailResponse> {
    let connection = Connection::open(db_path)?;
    let report = require_issue_report(&connection, report_id)?;
    let new_status = request.status.as_str();
    let now = Utc::now().to_rfc3339();
    connection.execute(
        "UPDATE issue_report_artifacts SET status = ?1, updated_at = ?2 WHERE id = ?3",
        params![new_status, now, report_id],
    )?;
    insert_report_status_event(
        &connection,
        report_id,
        Some(&report.status),
        new_status,
        non_empty_optional(request.reason),
        actor_id,
    )?;
    read_issue_report(db_path, report_id)
}

pub fn export_issue_report(
    db_path: &Path,
    report_id: &str,
    request: IssueReportExportRequest,
    actor_id: Option<&str>,
) -> Result<IssueReportExportResponse> {
    let connection = Connection::open(db_path)?;
    let report = require_issue_report(&connection, report_id)?;
    let export_format = request
        .export_format
        .as_deref()
        .map(str::trim)
        .filter(|format| !format.is_empty())
        .unwrap_or("markdown")
        .to_string();
    if export_format != "markdown" {
        bail!("Only markdown issue report exports are supported");
    }
    let now = Utc::now().to_rfc3339();
    let export_id = format!("report_export_{}", Uuid::new_v4());
    let content_hash = content_hash(&report.markdown_body);
    connection.execute(
        "INSERT INTO issue_report_exports (
            id, report_id, export_format, content_hash, content_bytes, content_text,
            created_by_actor_id, created_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            export_id,
            report_id,
            export_format,
            content_hash,
            report.markdown_body.len() as i64,
            &report.markdown_body,
            actor_id,
            now,
        ],
    )?;
    connection.execute(
        "UPDATE issue_report_artifacts
         SET status = 'exported', exported_at = COALESCE(exported_at, ?1), updated_at = ?1
         WHERE id = ?2",
        params![now, report_id],
    )?;
    insert_report_status_event(
        &connection,
        report_id,
        Some(&report.status),
        IssueReportStatus::Exported.as_str(),
        Some("local markdown export prepared".to_string()),
        actor_id,
    )?;
    let export = load_issue_report_export(&connection, &export_id)?
        .ok_or_else(|| anyhow::anyhow!("Inserted report export was not found"))?;
    let report = require_issue_report(&connection, report_id)?;
    Ok(IssueReportExportResponse { report, export })
}

pub fn list_support_packets(db_path: &Path) -> Result<SupportPacketListResponse> {
    let connection = Connection::open(db_path)?;
    Ok(SupportPacketListResponse {
        packets: load_support_packets(&connection)?,
    })
}

pub fn draft_support_packet(
    db_path: &Path,
    request: SupportPacketDraftRequest,
    actor_id: Option<&str>,
) -> Result<SupportPacketView> {
    let connection = Connection::open(db_path)?;
    let report = require_issue_report(&connection, &request.report_id)?;
    let payload_markdown = redact_support_packet_markdown(&report.markdown_body);
    let destination_kind = request
        .destination_kind
        .as_deref()
        .map(str::trim)
        .filter(|kind| !kind.is_empty())
        .unwrap_or("studio_ordo_support")
        .to_string();
    let destination_id = non_empty_optional(request.destination_id);
    let destination_label = non_empty_optional(request.destination_label)
        .or_else(|| Some("Studio Ordo Support".to_string()));
    let payload = json!({
        "schemaVersion": 1,
        "reportId": report.id,
        "reportTitle": report.title,
        "reportStatus": report.status,
        "contentFormat": "markdown",
        "content": payload_markdown,
        "redactions": report.redactions,
        "localOnly": true,
        "externalDelivery": false,
        "approvalRequirement": "explicit_owner_approval_before_egress",
    });
    let packet_id = format!("support_packet_{}", Uuid::new_v4());
    let now = Utc::now().to_rfc3339();
    let payload_hash = content_hash(&payload.to_string());
    connection.execute(
        "INSERT INTO support_packets (
            id, report_id, status, destination_kind, destination_id, destination_label,
            payload_json, payload_hash, approval_required, approved_by_actor_id, approved_at,
            created_by_actor_id, created_at, updated_at
         ) VALUES (?1, ?2, 'draft', ?3, ?4, ?5, ?6, ?7, 1, NULL, NULL, ?8, ?9, ?9)",
        params![
            packet_id,
            report.id,
            destination_kind,
            destination_id,
            destination_label,
            payload.to_string(),
            payload_hash,
            actor_id,
            now,
        ],
    )?;
    insert_support_packet_receipt(
        &connection,
        &packet_id,
        "draft_prepared",
        json!({
            "reportId": request.report_id,
            "externalDelivery": false,
            "approvalRequired": true,
        }),
    )?;
    load_support_packet(&connection, &packet_id)?
        .ok_or_else(|| anyhow::anyhow!("Inserted support packet was not found"))
}

pub fn approve_support_packet(
    db_path: &Path,
    packet_id: &str,
    request: SupportPacketApprovalRequest,
    actor_id: Option<&str>,
) -> Result<SupportPacketView> {
    let connection = Connection::open(db_path)?;
    let packet = require_support_packet(&connection, packet_id)?;
    if packet.status != "draft" {
        bail!("Only draft support packets can be approved");
    }
    let now = Utc::now().to_rfc3339();
    connection.execute(
        "UPDATE support_packets
         SET status = 'approved_local_only', approved_by_actor_id = ?1, approved_at = ?2, updated_at = ?2
         WHERE id = ?3",
        params![actor_id, now, packet_id],
    )?;
    insert_support_packet_receipt(
        &connection,
        packet_id,
        "owner_approved_local_only",
        json!({
            "approvalNote": non_empty_optional(request.approval_note),
            "approvedByActorId": actor_id,
            "payloadHash": packet.payload_hash,
            "externalDelivery": false,
            "deliveryState": "not_sent",
        }),
    )?;
    load_support_packet(&connection, packet_id)?
        .ok_or_else(|| anyhow::anyhow!("Approved support packet was not found"))
}

pub fn list_support_packet_receipts(
    db_path: &Path,
    packet_id: &str,
) -> Result<SupportPacketReceiptListResponse> {
    let connection = Connection::open(db_path)?;
    require_support_packet(&connection, packet_id)?;
    Ok(SupportPacketReceiptListResponse {
        receipts: load_support_packet_receipts(&connection, packet_id)?,
    })
}

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

fn complete_issue_report_job(
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

fn normalize_request(request: IssueReportPrepareRequest) -> Result<NormalizedIssueReportRequest> {
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

fn non_empty_optional(value: Option<String>) -> Option<String> {
    value
        .as_deref()
        .map(str::trim)
        .filter(|entry| !entry.is_empty())
        .map(ToString::to_string)
}

fn selected_sources(request: &NormalizedIssueReportRequest) -> Vec<String> {
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

fn envelope(
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

fn health_evidence(_db_path: &Path) -> EvidenceEnvelope {
    let health = build_health_report();
    envelope(
        "health",
        "succeeded",
        format!("Daemon health is {}.", health.status),
        serde_json::to_value(health).unwrap_or_else(|_| json!({})),
    )
}

fn readiness_evidence(db_path: &Path) -> EvidenceEnvelope {
    let readiness = build_readiness_report(db_path);
    envelope(
        "readiness",
        "succeeded",
        format!("Daemon readiness is {}.", readiness.status),
        serde_json::to_value(readiness).unwrap_or_else(|_| json!({})),
    )
}

fn events_evidence(connection: &Connection) -> Result<EvidenceEnvelope> {
    let events = query_json_rows(
        connection,
        "SELECT cursor, family, event_type, job_id, task_key, occurred_at, payload_json
         FROM realtime_events ORDER BY cursor DESC LIMIT 25",
        |row| {
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

fn jobs_evidence(connection: &Connection) -> Result<EvidenceEnvelope> {
    let jobs = query_json_rows(
        connection,
        "SELECT id, template_id, capability_id, kind, status, current_task_key, created_at, updated_at, failure_message
         FROM jobs ORDER BY updated_at DESC LIMIT 25",
        |row| {
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

fn logs_evidence(connection: &Connection) -> Result<EvidenceEnvelope> {
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

fn browser_context_evidence(browser_context: Option<Value>) -> EvidenceEnvelope {
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

fn query_json_rows(
    connection: &Connection,
    sql: &str,
    map: impl Fn(&rusqlite::Row<'_>) -> rusqlite::Result<Value>,
) -> Result<Vec<Value>> {
    let mut statement = connection.prepare(sql)?;
    let rows = statement.query_map([], map)?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(Into::into)
}

fn parse_json_column(value: String) -> Value {
    serde_json::from_str(&value).unwrap_or_else(|_| json!({}))
}

fn render_report_markdown(
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

fn insert_issue_report_artifact(
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

fn report_provenance_metadata(
    job_id: &str,
    report_id: &str,
    origin: &str,
    actor_id: Option<&str>,
) -> Value {
    let mut metadata = provenance_metadata(
        actor_context_for_origin(origin, actor_id),
        PolicyAction::Prepare,
        ResourceRef::new(ResourceKind::IssueReport, report_id),
        Some("issue.report.prepare"),
        ResourceClassification::local_operations_ready_for_review(),
    );
    if let Some(object) = metadata.as_object_mut() {
        object.insert("jobId".to_string(), json!(job_id));
        object.insert(
            "processTemplateId".to_string(),
            json!(ISSUE_REPORT_TEMPLATE_ID),
        );
    }
    metadata
}

fn actor_context_for_origin(origin: &str, actor_id: Option<&str>) -> ActorContext {
    let kind = match origin {
        "mcp" => ActorKind::McpClient,
        "scheduler" => ActorKind::Scheduler,
        "system" => ActorKind::System,
        _ => ActorKind::BrowserOperator,
    };
    ActorContext::new(kind, origin, actor_id.map(ToString::to_string))
}

fn load_issue_report_summaries(connection: &Connection) -> Result<Vec<IssueReportSummary>> {
    let mut statement = connection.prepare(
        "SELECT id, job_id, status, severity, title, summary, source_route, created_at,
                updated_at, exported_at, submitted_at, external_url
         FROM issue_report_artifacts
         ORDER BY updated_at DESC",
    )?;
    let rows = statement.query_map([], issue_report_summary_from_row)?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(Into::into)
}

fn load_issue_report(connection: &Connection, id: &str) -> Result<Option<IssueReportArtifact>> {
    connection
        .query_row(
            "SELECT id, job_id, status, severity, title, summary, description, source_route,
                    markdown_body, diagnostics_json, evidence_json, redactions_json, created_at,
                    updated_at, exported_at, submitted_at, external_url
             FROM issue_report_artifacts WHERE id = ?1",
            [id],
            issue_report_from_row,
        )
        .optional()
        .map_err(Into::into)
}

fn require_issue_report(connection: &Connection, id: &str) -> Result<IssueReportArtifact> {
    load_issue_report(connection, id)?.ok_or_else(|| anyhow::anyhow!("Issue report not found"))
}

fn issue_report_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<IssueReportArtifact> {
    let diagnostics_json: String = row.get(9)?;
    let evidence_json: String = row.get(10)?;
    let redactions_json: String = row.get(11)?;
    Ok(IssueReportArtifact {
        id: row.get(0)?,
        job_id: row.get(1)?,
        status: row.get(2)?,
        severity: row.get(3)?,
        title: row.get(4)?,
        summary: row.get(5)?,
        description: row.get(6)?,
        source_route: row.get(7)?,
        markdown_body: row.get(8)?,
        diagnostics: serde_json::from_str(&diagnostics_json).unwrap_or_else(|_| json!({})),
        evidence: serde_json::from_str(&evidence_json).unwrap_or_default(),
        redactions: serde_json::from_str(&redactions_json).unwrap_or_default(),
        created_at: row.get(12)?,
        updated_at: row.get(13)?,
        exported_at: row.get(14)?,
        submitted_at: row.get(15)?,
        external_url: row.get(16)?,
    })
}

fn issue_report_summary_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<IssueReportSummary> {
    Ok(IssueReportSummary {
        id: row.get(0)?,
        job_id: row.get(1)?,
        status: row.get(2)?,
        severity: row.get(3)?,
        title: row.get(4)?,
        summary: row.get(5)?,
        source_route: row.get(6)?,
        created_at: row.get(7)?,
        updated_at: row.get(8)?,
        exported_at: row.get(9)?,
        submitted_at: row.get(10)?,
        external_url: row.get(11)?,
    })
}

fn insert_report_status_event(
    connection: &Connection,
    report_id: &str,
    from_status: Option<&str>,
    to_status: &str,
    reason: Option<String>,
    actor_id: Option<&str>,
) -> Result<()> {
    connection.execute(
        "INSERT INTO issue_report_status_events (
            id, report_id, from_status, to_status, reason, created_by_actor_id, created_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            format!("report_status_event_{}", Uuid::new_v4()),
            report_id,
            from_status,
            to_status,
            reason,
            actor_id,
            Utc::now().to_rfc3339(),
        ],
    )?;
    Ok(())
}

fn load_issue_report_exports(
    connection: &Connection,
    report_id: &str,
) -> Result<Vec<IssueReportExportView>> {
    let mut statement = connection.prepare(
        "SELECT id, report_id, export_format, content_hash, content_bytes, content_text,
                created_by_actor_id, created_at
         FROM issue_report_exports WHERE report_id = ?1 ORDER BY created_at DESC",
    )?;
    let rows = statement.query_map([report_id], issue_report_export_from_row)?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(Into::into)
}

fn load_issue_report_export(
    connection: &Connection,
    export_id: &str,
) -> Result<Option<IssueReportExportView>> {
    connection
        .query_row(
            "SELECT id, report_id, export_format, content_hash, content_bytes, content_text,
                    created_by_actor_id, created_at
             FROM issue_report_exports WHERE id = ?1",
            [export_id],
            issue_report_export_from_row,
        )
        .optional()
        .map_err(Into::into)
}

fn issue_report_export_from_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<IssueReportExportView> {
    Ok(IssueReportExportView {
        id: row.get(0)?,
        report_id: row.get(1)?,
        export_format: row.get(2)?,
        content_hash: row.get(3)?,
        content_bytes: row.get(4)?,
        content_text: row.get(5)?,
        created_by_actor_id: row.get(6)?,
        created_at: row.get(7)?,
    })
}

fn load_issue_report_status_events(
    connection: &Connection,
    report_id: &str,
) -> Result<Vec<IssueReportStatusEventView>> {
    let mut statement = connection.prepare(
        "SELECT id, report_id, from_status, to_status, reason, created_by_actor_id, created_at
         FROM issue_report_status_events WHERE report_id = ?1 ORDER BY created_at DESC",
    )?;
    let rows = statement.query_map([report_id], |row| {
        Ok(IssueReportStatusEventView {
            id: row.get(0)?,
            report_id: row.get(1)?,
            from_status: row.get(2)?,
            to_status: row.get(3)?,
            reason: row.get(4)?,
            created_by_actor_id: row.get(5)?,
            created_at: row.get(6)?,
        })
    })?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(Into::into)
}

fn load_support_packets(connection: &Connection) -> Result<Vec<SupportPacketView>> {
    let mut statement = connection.prepare(
        "SELECT id, report_id, status, destination_kind, destination_id, destination_label,
                payload_json, payload_hash, approval_required, approved_by_actor_id, approved_at,
                created_by_actor_id, created_at, updated_at
         FROM support_packets ORDER BY updated_at DESC",
    )?;
    let rows = statement.query_map([], support_packet_from_row)?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(Into::into)
}

fn load_support_packets_for_report(
    connection: &Connection,
    report_id: &str,
) -> Result<Vec<SupportPacketView>> {
    let mut statement = connection.prepare(
        "SELECT id, report_id, status, destination_kind, destination_id, destination_label,
                payload_json, payload_hash, approval_required, approved_by_actor_id, approved_at,
                created_by_actor_id, created_at, updated_at
         FROM support_packets WHERE report_id = ?1 ORDER BY updated_at DESC",
    )?;
    let rows = statement.query_map([report_id], support_packet_from_row)?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(Into::into)
}

fn load_support_packet(
    connection: &Connection,
    packet_id: &str,
) -> Result<Option<SupportPacketView>> {
    connection
        .query_row(
            "SELECT id, report_id, status, destination_kind, destination_id, destination_label,
                    payload_json, payload_hash, approval_required, approved_by_actor_id, approved_at,
                    created_by_actor_id, created_at, updated_at
             FROM support_packets WHERE id = ?1",
            [packet_id],
            support_packet_from_row,
        )
        .optional()
        .map_err(Into::into)
}

fn require_support_packet(connection: &Connection, packet_id: &str) -> Result<SupportPacketView> {
    load_support_packet(connection, packet_id)?
        .ok_or_else(|| anyhow::anyhow!("Support packet not found"))
}

fn support_packet_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<SupportPacketView> {
    let payload_json: String = row.get(6)?;
    let approval_required: i64 = row.get(8)?;
    Ok(SupportPacketView {
        id: row.get(0)?,
        report_id: row.get(1)?,
        status: row.get(2)?,
        destination_kind: row.get(3)?,
        destination_id: row.get(4)?,
        destination_label: row.get(5)?,
        payload: serde_json::from_str(&payload_json).unwrap_or_else(|_| json!({})),
        payload_hash: row.get(7)?,
        approval_required: approval_required == 1,
        approved_by_actor_id: row.get(9)?,
        approved_at: row.get(10)?,
        created_by_actor_id: row.get(11)?,
        created_at: row.get(12)?,
        updated_at: row.get(13)?,
    })
}

fn insert_support_packet_receipt(
    connection: &Connection,
    packet_id: &str,
    receipt_kind: &str,
    payload: Value,
) -> Result<()> {
    connection.execute(
        "INSERT INTO support_packet_receipts (id, packet_id, receipt_kind, payload_json, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            format!("support_packet_receipt_{}", Uuid::new_v4()),
            packet_id,
            receipt_kind,
            payload.to_string(),
            Utc::now().to_rfc3339(),
        ],
    )?;
    Ok(())
}

fn load_support_packet_receipts(
    connection: &Connection,
    packet_id: &str,
) -> Result<Vec<SupportPacketReceiptView>> {
    let mut statement = connection.prepare(
        "SELECT id, packet_id, receipt_kind, payload_json, created_at
         FROM support_packet_receipts WHERE packet_id = ?1 ORDER BY created_at DESC",
    )?;
    let rows = statement.query_map([packet_id], |row| {
        let payload_json: String = row.get(3)?;
        Ok(SupportPacketReceiptView {
            id: row.get(0)?,
            packet_id: row.get(1)?,
            receipt_kind: row.get(2)?,
            payload: serde_json::from_str(&payload_json).unwrap_or_else(|_| json!({})),
            created_at: row.get(4)?,
        })
    })?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(Into::into)
}

fn content_hash(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    format!("sha256:{:x}", hasher.finalize())
}

fn redact_support_packet_markdown(markdown: &str) -> String {
    markdown
        .lines()
        .map(|line| {
            if contains_secret_indicator(line) {
                "[redacted support packet line]".to_string()
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn contains_secret_indicator(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    [
        "api_key",
        "apikey",
        "token",
        "password",
        "secret",
        "vault key",
        "bearer ",
    ]
    .iter()
    .any(|needle| lower.contains(needle))
}

fn set_job_running(connection: &Connection, job_id: &str) -> Result<()> {
    let now = Utc::now().to_rfc3339();
    connection.execute(
        "UPDATE jobs SET status = 'running', started_at = COALESCE(started_at, ?1), updated_at = ?1 WHERE id = ?2",
        params![now, job_id],
    )?;
    append_job_event(connection, job_id, None, "job.started", json!({}))?;
    Ok(())
}

fn run_task(connection: &Connection, job_id: &str, task_key: &str, output: Value) -> Result<()> {
    mark_task_running(connection, job_id, task_key)?;
    mark_task_succeeded(connection, job_id, task_key, output)
}

fn mark_task_running(connection: &Connection, job_id: &str, task_key: &str) -> Result<()> {
    let now = Utc::now().to_rfc3339();
    let status: String = connection.query_row(
        "SELECT status FROM job_tasks WHERE job_id = ?1 AND task_key = ?2",
        params![job_id, task_key],
        |row| row.get(0),
    )?;
    if status == "pending" {
        connection.execute(
            "UPDATE job_tasks SET status = 'ready', updated_at = ?1 WHERE job_id = ?2 AND task_key = ?3",
            params![now, job_id, task_key],
        )?;
        append_job_event(
            connection,
            job_id,
            Some(task_key),
            "task.ready",
            json!({ "taskKey": task_key }),
        )?;
    }
    connection.execute(
        "UPDATE job_tasks
         SET status = 'running', attempt_count = attempt_count + 1,
             started_at = COALESCE(started_at, ?1), updated_at = ?1
         WHERE job_id = ?2 AND task_key = ?3",
        params![now, job_id, task_key],
    )?;
    connection.execute(
        "UPDATE jobs SET status = 'running', current_task_key = ?1, updated_at = ?2 WHERE id = ?3",
        params![task_key, now, job_id],
    )?;
    append_job_event(
        connection,
        job_id,
        Some(task_key),
        "task.started",
        json!({ "taskKey": task_key }),
    )?;
    Ok(())
}

fn mark_task_succeeded(
    connection: &Connection,
    job_id: &str,
    task_key: &str,
    output: Value,
) -> Result<()> {
    let now = Utc::now().to_rfc3339();
    connection.execute(
        "UPDATE job_tasks
         SET status = 'succeeded', output_json = ?1, completed_at = ?2, updated_at = ?2
         WHERE job_id = ?3 AND task_key = ?4",
        params![output.to_string(), now, job_id, task_key],
    )?;
    update_completed_required_count(connection, job_id)?;
    append_job_event(
        connection,
        job_id,
        Some(task_key),
        "task.succeeded",
        json!({ "taskKey": task_key, "output": output }),
    )?;
    Ok(())
}

fn update_completed_required_count(connection: &Connection, job_id: &str) -> Result<()> {
    let completed_required_count: i64 = connection.query_row(
        "SELECT COUNT(*) FROM job_tasks WHERE job_id = ?1 AND required = 1 AND status IN ('succeeded', 'skipped')",
        [job_id],
        |row| row.get(0),
    )?;
    connection.execute(
        "UPDATE jobs SET completed_required_task_count = ?1, updated_at = ?2 WHERE id = ?3",
        params![completed_required_count, Utc::now().to_rfc3339(), job_id],
    )?;
    Ok(())
}

fn mark_job_succeeded(connection: &Connection, job_id: &str, payload: Value) -> Result<()> {
    let now = Utc::now().to_rfc3339();
    connection.execute(
        "UPDATE jobs
         SET status = 'succeeded', current_task_key = NULL, completed_at = ?1, updated_at = ?1,
             completed_required_task_count = required_task_count
         WHERE id = ?2",
        params![now, job_id],
    )?;
    append_job_event(connection, job_id, None, "job.succeeded", payload)?;
    Ok(())
}

fn mark_job_failed(connection: &Connection, job_id: &str, message: &str) -> Result<()> {
    let now = Utc::now().to_rfc3339();
    connection.execute(
        "UPDATE jobs
         SET status = 'failed', current_task_key = NULL, completed_at = ?1, updated_at = ?1, failure_message = ?2
         WHERE id = ?3",
        params![now, message, job_id],
    )?;
    append_job_event(
        connection,
        job_id,
        None,
        "job.failed",
        json!({ "message": message }),
    )?;
    Ok(())
}

fn insert_job_artifact(
    connection: &Connection,
    job_id: &str,
    task_key: Option<&str>,
    artifact_kind: &str,
    uri: &str,
    label: &str,
    metadata: Value,
) -> Result<String> {
    let artifact_id = format!("artifact_{}", Uuid::new_v4());
    connection.execute(
        "INSERT INTO job_artifacts (id, job_id, task_key, artifact_kind, uri, label, metadata_json, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            artifact_id,
            job_id,
            task_key,
            artifact_kind,
            uri,
            label,
            metadata.to_string(),
            Utc::now().to_rfc3339(),
        ],
    )?;
    Ok(artifact_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capabilities::seed_builtin_capabilities;
    use crate::schema::init_schema;
    use crate::templates::seed_builtin_templates;
    use tempfile::NamedTempFile;

    fn setup_db() -> NamedTempFile {
        let db = NamedTempFile::new().unwrap();
        let connection = Connection::open(db.path()).unwrap();
        init_schema(&connection).unwrap();
        seed_builtin_capabilities(&connection).unwrap();
        seed_builtin_templates(&connection).unwrap();
        db
    }

    #[test]
    fn issue_report_preparation_creates_job_and_artifact() {
        let db = setup_db();
        let report = prepare_issue_report(
            db.path(),
            IssueReportPrepareRequest {
                title: Some("Smoke issue".to_string()),
                severity: Some(IssueSeverity::High),
                description: "Something observable happened.".to_string(),
                expected_behavior: Some("Expected stable operation.".to_string()),
                actual_behavior: Some("Saw an error.".to_string()),
                steps: Some(vec!["Open System".to_string()]),
                source_route: Some("/backup-restore".to_string()),
                include_health_snapshot: Some(true),
                include_recent_events: Some(true),
                include_recent_jobs: Some(true),
                include_diagnostic_logs: Some(true),
                include_browser_context: Some(false),
                browser_context: None,
            },
            "test",
            None,
        )
        .unwrap();

        assert_eq!(report.status, "ready_for_review");
        assert_eq!(report.severity, "high");
        assert!(report.markdown_body.contains("## Evidence"));
        assert!(report
            .evidence
            .iter()
            .any(|entry| entry.source == "diagnostic_logs"));
        assert_eq!(
            report.diagnostics["classification"]["visibility"],
            "owner_system"
        );

        let reports = list_issue_reports(db.path()).unwrap();
        assert_eq!(reports.reports.len(), 1);
        let summary_json = serde_json::to_value(&reports.reports[0]).unwrap();
        assert_eq!(summary_json["id"], report.id);
        assert!(summary_json.get("markdownBody").is_none());
        assert!(summary_json.get("diagnostics").is_none());
        assert!(summary_json.get("evidence").is_none());
        assert!(summary_json.get("redactions").is_none());
        assert!(summary_json.get("description").is_none());

        let detail = read_issue_report(db.path(), &report.id).unwrap();
        assert_eq!(detail.report.markdown_body, report.markdown_body);
        assert_eq!(detail.report.evidence.len(), report.evidence.len());

        let connection = Connection::open(db.path()).unwrap();
        let metadata_json: String = connection
            .query_row(
                "SELECT metadata_json FROM job_artifacts WHERE artifact_kind = 'issue.report'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let metadata: Value = serde_json::from_str(&metadata_json).unwrap();
        assert_eq!(metadata["provenance"]["actor"]["kind"], "browser_operator");
        assert_eq!(metadata["provenance"]["action"], "prepare");
        assert_eq!(metadata["provenance"]["resource"]["kind"], "issue_report");
        assert_eq!(
            metadata["provenance"]["jobId"],
            json!(report.job_id.unwrap())
        );
    }

    #[test]
    fn report_detail_status_and_export_contracts_are_durable() {
        let db = setup_db();
        let report = prepare_issue_report(
            db.path(),
            IssueReportPrepareRequest {
                title: Some("Export issue".to_string()),
                severity: Some(IssueSeverity::Medium),
                description: "Needs a local export.".to_string(),
                expected_behavior: None,
                actual_behavior: None,
                steps: None,
                source_route: None,
                include_health_snapshot: Some(false),
                include_recent_events: Some(false),
                include_recent_jobs: Some(false),
                include_diagnostic_logs: Some(false),
                include_browser_context: Some(false),
                browser_context: None,
            },
            "test",
            Some("actor_local_owner"),
        )
        .unwrap();

        let status = update_issue_report_status(
            db.path(),
            &report.id,
            IssueReportStatusUpdateRequest {
                status: IssueReportStatus::Draft,
                reason: Some("operator is still reviewing".to_string()),
            },
            Some("actor_local_owner"),
        )
        .unwrap();
        assert_eq!(status.report.status, "draft");
        assert_eq!(status.status_events.len(), 1);
        assert_eq!(status.status_events[0].to_status, "draft");

        let exported = export_issue_report(
            db.path(),
            &report.id,
            IssueReportExportRequest {
                export_format: None,
            },
            Some("actor_local_owner"),
        )
        .unwrap();
        assert_eq!(exported.report.status, "exported");
        assert_eq!(exported.export.export_format, "markdown");
        assert_eq!(exported.export.content_text, report.markdown_body);
        assert!(exported.export.content_hash.starts_with("sha256:"));

        let detail = read_issue_report(db.path(), &report.id).unwrap();
        assert_eq!(detail.exports.len(), 1);
        assert_eq!(detail.status_events.len(), 2);
    }

    #[test]
    fn support_packet_draft_redacts_and_requires_approval_before_local_receipt() {
        let db = setup_db();
        let report = prepare_issue_report(
            db.path(),
            IssueReportPrepareRequest {
                title: Some("Support handoff".to_string()),
                severity: Some(IssueSeverity::High),
                description: "Provider api_key = sk-live-secret should never leave.".to_string(),
                expected_behavior: None,
                actual_behavior: None,
                steps: None,
                source_route: None,
                include_health_snapshot: Some(false),
                include_recent_events: Some(false),
                include_recent_jobs: Some(false),
                include_diagnostic_logs: Some(false),
                include_browser_context: Some(false),
                browser_context: None,
            },
            "test",
            Some("actor_local_owner"),
        )
        .unwrap();

        let packet = draft_support_packet(
            db.path(),
            SupportPacketDraftRequest {
                report_id: report.id.clone(),
                destination_kind: None,
                destination_id: None,
                destination_label: None,
            },
            Some("actor_local_owner"),
        )
        .unwrap();
        assert_eq!(packet.status, "draft");
        assert!(packet.approval_required);
        assert_eq!(packet.payload["externalDelivery"], false);
        let content = packet.payload["content"].as_str().unwrap();
        assert!(!content.contains("sk-live-secret"));
        assert!(content.contains("[redacted support packet line]"));

        let receipts = list_support_packet_receipts(db.path(), &packet.id).unwrap();
        assert_eq!(receipts.receipts.len(), 1);
        assert_eq!(receipts.receipts[0].receipt_kind, "draft_prepared");
        assert_eq!(receipts.receipts[0].payload["externalDelivery"], false);

        let approved = approve_support_packet(
            db.path(),
            &packet.id,
            SupportPacketApprovalRequest {
                approval_note: Some("Reviewed locally".to_string()),
            },
            Some("actor_local_owner"),
        )
        .unwrap();
        assert_eq!(approved.status, "approved_local_only");
        assert!(approved.approved_at.is_some());

        let receipts = list_support_packet_receipts(db.path(), &packet.id).unwrap();
        assert_eq!(receipts.receipts.len(), 2);
        assert!(receipts.receipts.iter().any(|receipt| receipt.receipt_kind
            == "owner_approved_local_only"
            && receipt.payload["externalDelivery"] == false
            && receipt.payload["deliveryState"] == "not_sent"));
        assert!(approve_support_packet(
            db.path(),
            &packet.id,
            SupportPacketApprovalRequest {
                approval_note: None
            },
            Some("actor_local_owner"),
        )
        .is_err());
    }
}
