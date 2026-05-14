use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const ISSUE_REPORT_TEMPLATE_ID: &str = "issue.report.prepare";

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum IssueSeverity {
    Low,
    #[default]
    Medium,
    High,
    Blocker,
}

impl IssueSeverity {
    pub(crate) fn as_str(&self) -> &'static str {
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
    pub(crate) fn as_str(&self) -> &'static str {
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

pub(crate) struct NormalizedIssueReportRequest {
    pub(crate) title: String,
    pub(crate) severity: IssueSeverity,
    pub(crate) description: String,
    pub(crate) expected_behavior: Option<String>,
    pub(crate) actual_behavior: Option<String>,
    pub(crate) steps: Vec<String>,
    pub(crate) source_route: Option<String>,
    pub(crate) include_health_snapshot: bool,
    pub(crate) include_recent_events: bool,
    pub(crate) include_recent_jobs: bool,
    pub(crate) include_diagnostic_logs: bool,
    pub(crate) include_browser_context: bool,
    pub(crate) browser_context: Option<Value>,
}
