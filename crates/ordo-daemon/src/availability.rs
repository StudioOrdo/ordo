use anyhow::{bail, Result};
use chrono::{DateTime, Datelike, Timelike, Utc};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::Path;
use uuid::Uuid;

use crate::events::{append_realtime_event_tx, system_event, RealtimeEvent};

const DEFAULT_AVAILABILITY_SCHEDULE_ID: &str = "availability_schedule_default";
const DEFAULT_OPERATOR_PRESENCE_ID: &str = "operator_presence_default";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AvailabilityScheduleStatus {
    Active,
    Paused,
}

impl AvailabilityScheduleStatus {
    fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Paused => "paused",
        }
    }
}

impl TryFrom<&str> for AvailabilityScheduleStatus {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> Result<Self> {
        match value {
            "active" => Ok(Self::Active),
            "paused" => Ok(Self::Paused),
            _ => bail!("Unsupported availability schedule status: {value}"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OperatorPresenceStatus {
    Available,
    Away,
    Focused,
    Offline,
    Paused,
}

impl OperatorPresenceStatus {
    fn as_str(self) -> &'static str {
        match self {
            Self::Available => "available",
            Self::Away => "away",
            Self::Focused => "focused",
            Self::Offline => "offline",
            Self::Paused => "paused",
        }
    }
}

impl TryFrom<&str> for OperatorPresenceStatus {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> Result<Self> {
        match value {
            "available" => Ok(Self::Available),
            "away" => Ok(Self::Away),
            "focused" => Ok(Self::Focused),
            "offline" => Ok(Self::Offline),
            "paused" => Ok(Self::Paused),
            _ => bail!("Unsupported operator presence status: {value}"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InterruptionThreshold {
    Open,
    Selective,
    MoneyOnly,
    UrgentOnly,
    Paused,
}

impl InterruptionThreshold {
    fn as_str(self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::Selective => "selective",
            Self::MoneyOnly => "money_only",
            Self::UrgentOnly => "urgent_only",
            Self::Paused => "paused",
        }
    }
}

impl TryFrom<&str> for InterruptionThreshold {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> Result<Self> {
        match value {
            "open" => Ok(Self::Open),
            "selective" => Ok(Self::Selective),
            "money_only" => Ok(Self::MoneyOnly),
            "urgent_only" => Ok(Self::UrgentOnly),
            "paused" => Ok(Self::Paused),
            _ => bail!("Unsupported interruption threshold: {value}"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HandoffIntent {
    General,
    Sales,
    Money,
    Urgent,
    Support,
}

impl HandoffIntent {
    fn as_str(self) -> &'static str {
        match self {
            Self::General => "general",
            Self::Sales => "sales",
            Self::Money => "money",
            Self::Urgent => "urgent",
            Self::Support => "support",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConnectionTrustLevel {
    Unknown,
    Untrusted,
    Trusted,
}

impl ConnectionTrustLevel {
    fn as_str(self) -> &'static str {
        match self {
            Self::Unknown => "unknown",
            Self::Untrusted => "untrusted",
            Self::Trusted => "trusted",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalRequirement {
    OwnerApprovalRequired,
    OwnerReviewOnly,
}

impl ApprovalRequirement {
    fn as_str(self) -> &'static str {
        match self {
            Self::OwnerApprovalRequired => "owner_approval_required",
            Self::OwnerReviewOnly => "owner_review_only",
        }
    }
}

impl TryFrom<&str> for ApprovalRequirement {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> Result<Self> {
        match value {
            "owner_approval_required" => Ok(Self::OwnerApprovalRequired),
            "owner_review_only" => Ok(Self::OwnerReviewOnly),
            _ => bail!("Unsupported approval requirement: {value}"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeliveryState {
    PendingOwnerApproval,
    Queued,
    Assigned,
    Declined,
    ContinueScreening,
    ApprovedLocalOnly,
}

impl DeliveryState {
    fn as_str(self) -> &'static str {
        match self {
            Self::PendingOwnerApproval => "pending_owner_approval",
            Self::Queued => "queued",
            Self::Assigned => "assigned",
            Self::Declined => "declined",
            Self::ContinueScreening => "continue_screening",
            Self::ApprovedLocalOnly => "approved_local_only",
        }
    }

    fn is_terminal(self) -> bool {
        matches!(self, Self::Declined | Self::ApprovedLocalOnly)
    }
}

impl TryFrom<&str> for DeliveryState {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> Result<Self> {
        match value {
            "pending_owner_approval" => Ok(Self::PendingOwnerApproval),
            "queued" => Ok(Self::Queued),
            "assigned" => Ok(Self::Assigned),
            "declined" => Ok(Self::Declined),
            "continue_screening" => Ok(Self::ContinueScreening),
            "approved_local_only" => Ok(Self::ApprovedLocalOnly),
            _ => bail!("Unsupported delivery state: {value}"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OwnerDecision {
    Accept,
    Decline,
    Queue,
    ContinueScreening,
}

impl OwnerDecision {
    fn as_str(self) -> &'static str {
        match self {
            Self::Accept => "accept",
            Self::Decline => "decline",
            Self::Queue => "queue",
            Self::ContinueScreening => "continue_screening",
        }
    }

    fn delivery_state(self) -> DeliveryState {
        match self {
            Self::Accept => DeliveryState::ApprovedLocalOnly,
            Self::Decline => DeliveryState::Declined,
            Self::Queue => DeliveryState::Queued,
            Self::ContinueScreening => DeliveryState::ContinueScreening,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AvailabilityWindow {
    pub day_of_week: u32,
    pub start_minute: u32,
    pub end_minute: u32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AvailabilityStateResponse {
    pub schedule: AvailabilityScheduleView,
    pub presence: OperatorPresenceView,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AvailabilityScheduleView {
    pub id: String,
    pub label: String,
    pub timezone: String,
    pub status: AvailabilityScheduleStatus,
    pub windows: Vec<AvailabilityWindow>,
    pub metadata: Value,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OperatorPresenceView {
    pub id: String,
    pub status: OperatorPresenceStatus,
    pub threshold: InterruptionThreshold,
    pub status_message: Option<String>,
    pub metadata: Value,
    pub updated_by_actor_id: Option<String>,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HandoffEligibilityView {
    pub id: String,
    pub allowed: bool,
    pub reason: String,
    pub evidence: Value,
    pub intent: HandoffIntent,
    pub connection_id: Option<String>,
    pub connection_trust: ConnectionTrustLevel,
    pub decided_at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HandoffInboxListResponse {
    pub items: Vec<HandoffInboxItemView>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct HandoffInboxListQuery {
    pub delivery_state: Option<DeliveryState>,
    pub assignee_actor_id: Option<String>,
    pub source_kind: Option<String>,
    pub source_id: Option<String>,
    pub visibility: Option<String>,
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HandoffInboxItemView {
    pub id: String,
    pub source_kind: String,
    pub source_id: Option<String>,
    pub destination_kind: String,
    pub destination_id: Option<String>,
    pub reason: String,
    pub requested_action: String,
    pub urgency: String,
    pub assignee_actor_id: Option<String>,
    pub due_at: Option<String>,
    pub next_action_hint: Option<String>,
    pub evidence_refs: Vec<String>,
    pub visibility: String,
    pub request: Value,
    pub evidence: Value,
    pub approval_requirement: ApprovalRequirement,
    pub delivery_state: DeliveryState,
    pub owner_decision: Option<OwnerDecision>,
    pub decision_reason: Option<String>,
    pub created_by_actor_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub resolved_at: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HandoffReceiptListResponse {
    pub receipts: Vec<HandoffReceiptView>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HandoffReceiptView {
    pub id: String,
    pub handoff_item_id: String,
    pub receipt_kind: String,
    pub payload: Value,
    pub created_at: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AvailabilityScheduleWriteRequest {
    pub label: Option<String>,
    pub timezone: Option<String>,
    pub status: Option<AvailabilityScheduleStatus>,
    pub windows: Vec<AvailabilityWindow>,
    pub metadata: Option<Value>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OperatorPresenceWriteRequest {
    pub status: OperatorPresenceStatus,
    pub threshold: InterruptionThreshold,
    pub status_message: Option<String>,
    pub metadata: Option<Value>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HandoffEligibilityRequest {
    pub intent: HandoffIntent,
    pub connection_id: Option<String>,
    pub connection_trust: Option<ConnectionTrustLevel>,
    pub evaluated_at: Option<String>,
    pub evidence: Option<Value>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HandoffInboxCreateRequest {
    pub source_kind: String,
    pub source_id: Option<String>,
    pub destination_kind: String,
    pub destination_id: Option<String>,
    pub reason: Option<String>,
    pub requested_action: Option<String>,
    pub urgency: Option<String>,
    pub assignee_actor_id: Option<String>,
    pub due_at: Option<String>,
    pub next_action_hint: Option<String>,
    pub evidence_refs: Option<Vec<String>>,
    pub visibility: Option<String>,
    pub request: Option<Value>,
    pub evidence: Option<Value>,
    pub approval_requirement: Option<ApprovalRequirement>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct HandoffInboxUpdateRequest {
    pub reason: Option<String>,
    pub requested_action: Option<String>,
    pub urgency: Option<String>,
    pub assignee_actor_id: Option<String>,
    pub clear_assignee: Option<bool>,
    pub due_at: Option<String>,
    pub clear_due_at: Option<bool>,
    pub next_action_hint: Option<String>,
    pub evidence_refs: Option<Vec<String>>,
    pub visibility: Option<String>,
    pub delivery_state: Option<DeliveryState>,
    pub evidence: Option<Value>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HandoffInboxResolveRequest {
    pub decision: OwnerDecision,
    pub decision_reason: Option<String>,
    pub evidence: Option<Value>,
}

#[derive(Debug, Clone)]
struct AvailabilityScheduleRecord {
    id: String,
    label: String,
    timezone: String,
    status: AvailabilityScheduleStatus,
    windows: Vec<AvailabilityWindow>,
    metadata: Value,
    created_at: String,
    updated_at: String,
}

#[derive(Debug, Clone)]
struct OperatorPresenceRecord {
    id: String,
    status: OperatorPresenceStatus,
    threshold: InterruptionThreshold,
    status_message: Option<String>,
    metadata: Value,
    updated_by_actor_id: Option<String>,
    updated_at: String,
}

#[derive(Debug, Clone)]
struct HandoffInboxItemRecord {
    id: String,
    source_kind: String,
    source_id: Option<String>,
    destination_kind: String,
    destination_id: Option<String>,
    reason: String,
    requested_action: String,
    urgency: String,
    assignee_actor_id: Option<String>,
    due_at: Option<String>,
    next_action_hint: Option<String>,
    evidence_refs: Vec<String>,
    visibility: String,
    request: Value,
    evidence: Value,
    approval_requirement: ApprovalRequirement,
    delivery_state: DeliveryState,
    owner_decision: Option<OwnerDecision>,
    decision_reason: Option<String>,
    created_by_actor_id: Option<String>,
    created_at: String,
    updated_at: String,
    resolved_at: Option<String>,
}

pub fn read_availability_state(db_path: &Path) -> Result<AvailabilityStateResponse> {
    let connection = Connection::open(db_path)?;
    ensure_default_availability_state(&connection)?;
    let schedule = find_default_schedule(&connection)?.expect("default schedule exists");
    let presence = find_default_presence(&connection)?.expect("default presence exists");
    Ok(AvailabilityStateResponse {
        schedule: schedule.into_view(),
        presence: presence.into_view(),
    })
}

pub fn update_availability_schedule(
    db_path: &Path,
    request: AvailabilityScheduleWriteRequest,
) -> Result<(AvailabilityScheduleView, RealtimeEvent)> {
    validate_windows(&request.windows)?;
    let mut connection = Connection::open(db_path)?;
    ensure_default_availability_state(&connection)?;
    let transaction = connection.transaction()?;
    let existing = find_default_schedule(&transaction)?.expect("default schedule exists");
    let now = Utc::now().to_rfc3339();
    let label = normalize_optional_string(request.label).unwrap_or(existing.label);
    let timezone = normalize_optional_string(request.timezone).unwrap_or(existing.timezone);
    let status = request.status.unwrap_or(existing.status);
    let windows_json = serde_json::to_string(&request.windows)?;
    let metadata = request.metadata.unwrap_or(existing.metadata);
    transaction.execute(
        "UPDATE availability_schedules
         SET label = ?1, timezone = ?2, status = ?3, windows_json = ?4,
             metadata_json = ?5, updated_at = ?6
         WHERE id = ?7",
        params![
            label,
            timezone,
            status.as_str(),
            windows_json,
            metadata.to_string(),
            now,
            DEFAULT_AVAILABILITY_SCHEDULE_ID,
        ],
    )?;
    let event = append_realtime_event_tx(
        &transaction,
        &system_event(
            "availability.schedule.updated",
            json!({ "scheduleId": DEFAULT_AVAILABILITY_SCHEDULE_ID, "status": status.as_str() }),
        ),
    )?;
    transaction.commit()?;
    let schedule = find_default_schedule(&connection)?.expect("default schedule exists");
    Ok((schedule.into_view(), event))
}

pub fn update_operator_presence(
    db_path: &Path,
    request: OperatorPresenceWriteRequest,
    actor_id: Option<&str>,
) -> Result<(OperatorPresenceView, RealtimeEvent)> {
    let mut connection = Connection::open(db_path)?;
    ensure_default_availability_state(&connection)?;
    let transaction = connection.transaction()?;
    let now = Utc::now().to_rfc3339();
    let status_message = normalize_optional_string(request.status_message);
    transaction.execute(
        "UPDATE operator_presence
         SET status = ?1, threshold = ?2, status_message = ?3, metadata_json = ?4,
             updated_by_actor_id = ?5, updated_at = ?6
         WHERE id = ?7",
        params![
            request.status.as_str(),
            request.threshold.as_str(),
            status_message,
            request.metadata.unwrap_or_else(|| json!({})).to_string(),
            actor_id,
            now,
            DEFAULT_OPERATOR_PRESENCE_ID,
        ],
    )?;
    let event = append_realtime_event_tx(
        &transaction,
        &system_event(
            "operator.presence.updated",
            json!({
                "presenceId": DEFAULT_OPERATOR_PRESENCE_ID,
                "status": request.status.as_str(),
                "threshold": request.threshold.as_str(),
            }),
        ),
    )?;
    transaction.commit()?;
    let presence = find_default_presence(&connection)?.expect("default presence exists");
    Ok((presence.into_view(), event))
}

pub fn evaluate_handoff_eligibility(
    db_path: &Path,
    request: HandoffEligibilityRequest,
) -> Result<HandoffEligibilityView> {
    let connection = Connection::open(db_path)?;
    ensure_default_availability_state(&connection)?;
    let schedule = find_default_schedule(&connection)?.expect("default schedule exists");
    let presence = find_default_presence(&connection)?.expect("default presence exists");
    let evaluated_at = parse_optional_datetime(request.evaluated_at.as_deref())?;
    let connection_id = request.connection_id.clone();
    let connection_trust = request
        .connection_trust
        .unwrap_or(ConnectionTrustLevel::Unknown);
    let schedule_allows = schedule_allows_time(&schedule, evaluated_at);
    let (threshold_allows, threshold_reason) =
        threshold_allows_intent(presence.threshold, request.intent, connection_trust);
    let presence_allows = presence.status == OperatorPresenceStatus::Available;
    let allowed = schedule.status == AvailabilityScheduleStatus::Active
        && schedule_allows
        && presence_allows
        && threshold_allows;
    let reason = if allowed {
        "handoff_allowed".to_string()
    } else if schedule.status == AvailabilityScheduleStatus::Paused {
        "availability_schedule_paused".to_string()
    } else if !schedule_allows {
        "outside_availability_schedule".to_string()
    } else if !presence_allows {
        "operator_presence_blocks_handoff".to_string()
    } else {
        threshold_reason
    };
    let id = format!("handoff_eligibility_{}", Uuid::new_v4());
    let decided_at = Utc::now().to_rfc3339();
    let evidence = json!({
        "request": request.evidence.unwrap_or_else(|| json!({})),
        "schedule": {
            "id": schedule.id,
            "status": schedule.status.as_str(),
            "timezone": schedule.timezone,
            "windowMatched": schedule_allows,
        },
        "presence": {
            "id": presence.id,
            "status": presence.status.as_str(),
            "threshold": presence.threshold.as_str(),
        },
        "intent": request.intent.as_str(),
        "connectionId": connection_id,
        "connectionTrust": connection_trust.as_str(),
        "evaluatedAt": evaluated_at.to_rfc3339(),
    });
    connection.execute(
        "INSERT INTO handoff_eligibility_decisions (
            id, intent, connection_id, connection_trust, allowed, reason, evidence_json, decided_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            id,
            request.intent.as_str(),
            connection_id.clone(),
            connection_trust.as_str(),
            if allowed { 1 } else { 0 },
            reason,
            evidence.to_string(),
            decided_at,
        ],
    )?;
    Ok(HandoffEligibilityView {
        id,
        allowed,
        reason,
        evidence,
        intent: request.intent,
        connection_id,
        connection_trust,
        decided_at,
    })
}

pub fn list_handoff_inbox(db_path: &Path) -> Result<HandoffInboxListResponse> {
    list_handoff_inbox_with_query(db_path, HandoffInboxListQuery::default())
}

pub fn list_handoff_inbox_with_query(
    db_path: &Path,
    query: HandoffInboxListQuery,
) -> Result<HandoffInboxListResponse> {
    let connection = Connection::open(db_path)?;
    let mut statement = connection.prepare(
        "SELECT id, source_kind, source_id, destination_kind, destination_id, request_json,
                evidence_json, approval_requirement, delivery_state, owner_decision,
                decision_reason, created_by_actor_id, created_at, updated_at, resolved_at,
                reason, requested_action, urgency, assignee_actor_id, due_at, next_action_hint,
                evidence_refs_json, visibility
         FROM handoff_inbox_items
         ORDER BY updated_at DESC, id DESC",
    )?;
    let items = statement
        .query_map([], handoff_item_from_row)?
        .map(|row| row.map(HandoffInboxItemRecord::into_view))
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let limit = query.limit.unwrap_or(100).min(500);
    Ok(HandoffInboxListResponse {
        items: items
            .into_iter()
            .filter(|item| handoff_query_matches_item(&query, item))
            .take(limit)
            .collect(),
    })
}

pub fn read_handoff_inbox_item(db_path: &Path, item_id: &str) -> Result<HandoffInboxItemView> {
    let connection = Connection::open(db_path)?;
    let item_id = require_identifier(item_id, "Handoff item id")?;
    find_handoff_item_by_id(&connection, &item_id)?
        .map(HandoffInboxItemRecord::into_view)
        .ok_or_else(|| anyhow::anyhow!("Handoff inbox item was not found: {item_id}"))
}

pub fn create_handoff_inbox_item(
    db_path: &Path,
    request: HandoffInboxCreateRequest,
    actor_id: Option<&str>,
) -> Result<(HandoffInboxItemView, RealtimeEvent)> {
    let source_kind = require_support_source_kind(&request.source_kind)?;
    let destination_kind = require_identifier(&request.destination_kind, "Destination kind")?;
    let source_id = normalize_identifier_option(request.source_id, "Source id")?;
    let destination_id = normalize_identifier_option(request.destination_id, "Destination id")?;
    let reason = normalize_required_text(request.reason, "Reason", "Support review requested")?;
    let requested_action =
        normalize_required_text(request.requested_action, "Requested action", "review")?;
    let urgency = normalize_urgency(request.urgency)?;
    let assignee_actor_id =
        normalize_identifier_option(request.assignee_actor_id, "Assignee actor id")?;
    let due_at = normalize_optional_due_at(request.due_at)?;
    let next_action_hint = normalize_optional_string(request.next_action_hint);
    let evidence_refs = normalize_evidence_refs(request.evidence_refs)?;
    let visibility = normalize_handoff_visibility(request.visibility)?;
    let approval_requirement = request
        .approval_requirement
        .unwrap_or(ApprovalRequirement::OwnerApprovalRequired);
    let mut connection = Connection::open(db_path)?;
    let transaction = connection.transaction()?;
    let id = format!("handoff_item_{}", Uuid::new_v4());
    let now = Utc::now().to_rfc3339();
    transaction.execute(
        "INSERT INTO handoff_inbox_items (
            id, source_kind, source_id, destination_kind, destination_id, request_json,
            evidence_json, approval_requirement, delivery_state, owner_decision,
            decision_reason, created_by_actor_id, created_at, updated_at, resolved_at,
            reason, requested_action, urgency, assignee_actor_id, due_at, next_action_hint,
            evidence_refs_json, visibility
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 'pending_owner_approval', NULL, NULL, ?9, ?10, ?10, NULL,
                   ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18)",
        params![
            id,
            source_kind,
            source_id,
            destination_kind,
            destination_id,
            request.request.unwrap_or_else(|| json!({})).to_string(),
            request.evidence.unwrap_or_else(|| json!({})).to_string(),
            approval_requirement.as_str(),
            actor_id,
            now,
            reason,
            requested_action,
            urgency,
            assignee_actor_id,
            due_at,
            next_action_hint,
            serde_json::to_string(&evidence_refs)?,
            visibility,
        ],
    )?;
    append_handoff_event_tx(
        &transaction,
        &id,
        "handoff.inbox.created",
        json!({
            "handoffItemId": id,
            "approvalRequirement": approval_requirement.as_str(),
            "deliveryState": DeliveryState::PendingOwnerApproval.as_str(),
            "sourceKind": source_kind,
            "urgency": urgency,
            "assigneeActorId": assignee_actor_id,
            "visibility": visibility,
        }),
        &now,
    )?;
    let event = append_realtime_event_tx(
        &transaction,
        &system_event(
            "handoff.inbox.created",
            json!({
                "handoffItemId": id,
                "approvalRequirement": approval_requirement.as_str(),
                "deliveryState": DeliveryState::PendingOwnerApproval.as_str(),
                "sourceKind": source_kind,
                "urgency": urgency,
                "assigneeActorId": assignee_actor_id,
                "visibility": visibility,
            }),
        ),
    )?;
    transaction.commit()?;
    let item = find_handoff_item_by_id(&connection, &id)?.expect("handoff item just inserted");
    Ok((item.into_view(), event))
}

pub fn resolve_handoff_inbox_item(
    db_path: &Path,
    item_id: &str,
    request: HandoffInboxResolveRequest,
    actor_id: Option<&str>,
) -> Result<(HandoffInboxItemView, RealtimeEvent)> {
    let mut connection = Connection::open(db_path)?;
    let transaction = connection.transaction()?;
    let existing = find_handoff_item_by_id(&transaction, item_id)?
        .ok_or_else(|| anyhow::anyhow!("Handoff inbox item was not found: {item_id}"))?;
    ensure_handoff_mutable(existing.delivery_state)?;
    let now = Utc::now().to_rfc3339();
    let delivery_state = request.decision.delivery_state();
    let decision_reason = normalize_optional_string(request.decision_reason);
    let merged_evidence = merge_evidence(existing.evidence, request.evidence);
    transaction.execute(
        "UPDATE handoff_inbox_items
         SET delivery_state = ?1, owner_decision = ?2, decision_reason = ?3,
             evidence_json = ?4, updated_at = ?5, resolved_at = ?5
         WHERE id = ?6",
        params![
            delivery_state.as_str(),
            request.decision.as_str(),
            decision_reason,
            merged_evidence.to_string(),
            now,
            item_id,
        ],
    )?;
    append_handoff_event_tx(
        &transaction,
        item_id,
        "handoff.inbox.resolved",
        json!({
            "handoffItemId": item_id,
            "ownerDecision": request.decision.as_str(),
            "deliveryState": delivery_state.as_str(),
            "decidedByActorId": actor_id,
            "externalDelivery": false,
        }),
        &now,
    )?;
    let event = append_realtime_event_tx(
        &transaction,
        &system_event(
            "handoff.inbox.resolved",
            json!({
                "handoffItemId": item_id,
                "ownerDecision": request.decision.as_str(),
                "deliveryState": delivery_state.as_str(),
                "decidedByActorId": actor_id,
                "externalDelivery": false,
            }),
        ),
    )?;
    transaction.commit()?;
    let item = find_handoff_item_by_id(&connection, item_id)?.expect("handoff item just updated");
    Ok((item.into_view(), event))
}

pub fn update_handoff_inbox_item(
    db_path: &Path,
    item_id: &str,
    request: HandoffInboxUpdateRequest,
    actor_id: Option<&str>,
) -> Result<(HandoffInboxItemView, RealtimeEvent)> {
    let mut connection = Connection::open(db_path)?;
    let transaction = connection.transaction()?;
    let existing = find_handoff_item_by_id(&transaction, item_id)?
        .ok_or_else(|| anyhow::anyhow!("Handoff inbox item was not found: {item_id}"))?;
    ensure_handoff_mutable(existing.delivery_state)?;

    let reason = request
        .reason
        .map(|value| normalize_required_text(Some(value), "Reason", "Support review requested"))
        .transpose()?
        .unwrap_or(existing.reason);
    let requested_action = request
        .requested_action
        .map(|value| normalize_required_text(Some(value), "Requested action", "review"))
        .transpose()?
        .unwrap_or(existing.requested_action);
    let urgency = request
        .urgency
        .map(|value| normalize_urgency(Some(value)))
        .transpose()?
        .unwrap_or(existing.urgency);
    let assignee_actor_id = if request.clear_assignee.unwrap_or(false) {
        None
    } else {
        normalize_identifier_option(request.assignee_actor_id, "Assignee actor id")?
            .or(existing.assignee_actor_id)
    };
    let due_at = if request.clear_due_at.unwrap_or(false) {
        None
    } else {
        normalize_optional_due_at(request.due_at)?.or(existing.due_at)
    };
    let next_action_hint = request
        .next_action_hint
        .and_then(|value| normalize_optional_string(Some(value)))
        .or(existing.next_action_hint);
    let evidence_refs = request
        .evidence_refs
        .map(Some)
        .map(normalize_evidence_refs)
        .transpose()?
        .unwrap_or(existing.evidence_refs);
    let visibility = request
        .visibility
        .map(|value| normalize_handoff_visibility(Some(value)))
        .transpose()?
        .unwrap_or(existing.visibility);
    let delivery_state = request.delivery_state.unwrap_or_else(|| {
        if assignee_actor_id.is_some()
            && matches!(
                existing.delivery_state,
                DeliveryState::PendingOwnerApproval | DeliveryState::Queued
            )
        {
            DeliveryState::Assigned
        } else {
            existing.delivery_state
        }
    });
    ensure_update_delivery_state(delivery_state)?;
    let merged_evidence = merge_evidence(existing.evidence, request.evidence);
    let now = Utc::now().to_rfc3339();
    transaction.execute(
        "UPDATE handoff_inbox_items
         SET reason = ?1, requested_action = ?2, urgency = ?3,
             assignee_actor_id = ?4, due_at = ?5, next_action_hint = ?6,
             evidence_refs_json = ?7, visibility = ?8, delivery_state = ?9,
             evidence_json = ?10, updated_at = ?11,
             resolved_at = CASE WHEN ?9 IN ('approved_local_only', 'declined') THEN ?11 ELSE resolved_at END
         WHERE id = ?12",
        params![
            reason,
            requested_action,
            urgency,
            assignee_actor_id,
            due_at,
            next_action_hint,
            serde_json::to_string(&evidence_refs)?,
            visibility,
            delivery_state.as_str(),
            merged_evidence.to_string(),
            now,
            item_id,
        ],
    )?;
    let event_type =
        handoff_update_event_type(existing.delivery_state, delivery_state, &assignee_actor_id);
    append_handoff_event_tx(
        &transaction,
        item_id,
        event_type,
        json!({
            "handoffItemId": item_id,
            "deliveryState": delivery_state.as_str(),
            "assigneeActorId": assignee_actor_id,
            "updatedByActorId": actor_id,
            "externalDelivery": false,
        }),
        &now,
    )?;
    let event = append_realtime_event_tx(
        &transaction,
        &system_event(
            event_type,
            json!({
                "handoffItemId": item_id,
                "deliveryState": delivery_state.as_str(),
                "assigneeActorId": assignee_actor_id,
                "updatedByActorId": actor_id,
                "externalDelivery": false,
            }),
        ),
    )?;
    transaction.commit()?;
    let item = find_handoff_item_by_id(&connection, item_id)?.expect("handoff item just updated");
    Ok((item.into_view(), event))
}

pub fn list_handoff_receipts(db_path: &Path, item_id: &str) -> Result<HandoffReceiptListResponse> {
    let connection = Connection::open(db_path)?;
    let item_id = require_identifier(item_id, "Handoff item id")?;
    let mut statement = connection.prepare(
        "SELECT id, handoff_item_id, receipt_kind, payload_json, created_at
         FROM handoff_receipts
         WHERE handoff_item_id = ?1
         ORDER BY created_at DESC, id DESC",
    )?;
    let receipts = statement
        .query_map([item_id], handoff_receipt_from_row)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(HandoffReceiptListResponse { receipts })
}

fn ensure_default_availability_state(connection: &Connection) -> Result<()> {
    let now = Utc::now().to_rfc3339();
    connection.execute(
        "INSERT OR IGNORE INTO availability_schedules (
            id, label, timezone, status, windows_json, metadata_json, created_at, updated_at
         ) VALUES (?1, 'Default Handoff Hours', 'UTC', 'active', '[]', '{}', ?2, ?2)",
        params![DEFAULT_AVAILABILITY_SCHEDULE_ID, now],
    )?;
    connection.execute(
        "INSERT OR IGNORE INTO operator_presence (
            id, status, threshold, status_message, metadata_json, updated_by_actor_id, updated_at
         ) VALUES (?1, 'offline', 'paused', NULL, '{}', NULL, ?2)",
        params![DEFAULT_OPERATOR_PRESENCE_ID, now],
    )?;
    Ok(())
}

fn append_handoff_event_tx(
    transaction: &rusqlite::Transaction<'_>,
    item_id: &str,
    event_type: &str,
    payload: Value,
    occurred_at: &str,
) -> Result<String> {
    let event_id = format!("handoff_event_{}", Uuid::new_v4());
    transaction.execute(
        "INSERT INTO handoff_events (id, handoff_item_id, event_type, payload_json, occurred_at)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            event_id,
            item_id,
            event_type,
            payload.to_string(),
            occurred_at
        ],
    )?;
    transaction.execute(
        "INSERT INTO handoff_receipts (id, handoff_item_id, event_id, receipt_kind, payload_json, created_at)
         VALUES (?1, ?2, ?3, 'local_recorded', ?4, ?5)",
        params![
            format!("handoff_receipt_{}", Uuid::new_v4()),
            item_id,
            event_id,
            json!({ "eventType": event_type, "recorded": true, "externalDelivery": false }).to_string(),
            occurred_at,
        ],
    )?;
    Ok(event_id)
}

fn find_default_schedule(
    connection: &Connection,
) -> rusqlite::Result<Option<AvailabilityScheduleRecord>> {
    connection
        .query_row(
            "SELECT id, label, timezone, status, windows_json, metadata_json, created_at, updated_at
             FROM availability_schedules WHERE id = ?1",
            [DEFAULT_AVAILABILITY_SCHEDULE_ID],
            availability_schedule_from_row,
        )
        .optional()
}

fn find_default_presence(
    connection: &Connection,
) -> rusqlite::Result<Option<OperatorPresenceRecord>> {
    connection
        .query_row(
            "SELECT id, status, threshold, status_message, metadata_json, updated_by_actor_id, updated_at
             FROM operator_presence WHERE id = ?1",
            [DEFAULT_OPERATOR_PRESENCE_ID],
            operator_presence_from_row,
        )
        .optional()
}

fn find_handoff_item_by_id(
    connection: &Connection,
    item_id: &str,
) -> rusqlite::Result<Option<HandoffInboxItemRecord>> {
    connection
        .query_row(
            "SELECT id, source_kind, source_id, destination_kind, destination_id, request_json,
                    evidence_json, approval_requirement, delivery_state, owner_decision,
                    decision_reason, created_by_actor_id, created_at, updated_at, resolved_at,
                    reason, requested_action, urgency, assignee_actor_id, due_at, next_action_hint,
                    evidence_refs_json, visibility
             FROM handoff_inbox_items WHERE id = ?1",
            [item_id],
            handoff_item_from_row,
        )
        .optional()
}

fn availability_schedule_from_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<AvailabilityScheduleRecord> {
    let status: String = row.get(3)?;
    let windows_json: String = row.get(4)?;
    let metadata_json: String = row.get(5)?;
    Ok(AvailabilityScheduleRecord {
        id: row.get(0)?,
        label: row.get(1)?,
        timezone: row.get(2)?,
        status: AvailabilityScheduleStatus::try_from(status.as_str()).map_err(|error| {
            rusqlite::Error::FromSqlConversionFailure(3, rusqlite::types::Type::Text, error.into())
        })?,
        windows: serde_json::from_str(&windows_json).unwrap_or_default(),
        metadata: serde_json::from_str(&metadata_json).unwrap_or_else(|_| json!({})),
        created_at: row.get(6)?,
        updated_at: row.get(7)?,
    })
}

fn operator_presence_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<OperatorPresenceRecord> {
    let status: String = row.get(1)?;
    let threshold: String = row.get(2)?;
    let metadata_json: String = row.get(4)?;
    Ok(OperatorPresenceRecord {
        id: row.get(0)?,
        status: OperatorPresenceStatus::try_from(status.as_str()).map_err(|error| {
            rusqlite::Error::FromSqlConversionFailure(1, rusqlite::types::Type::Text, error.into())
        })?,
        threshold: InterruptionThreshold::try_from(threshold.as_str()).map_err(|error| {
            rusqlite::Error::FromSqlConversionFailure(2, rusqlite::types::Type::Text, error.into())
        })?,
        status_message: row.get(3)?,
        metadata: serde_json::from_str(&metadata_json).unwrap_or_else(|_| json!({})),
        updated_by_actor_id: row.get(5)?,
        updated_at: row.get(6)?,
    })
}

fn handoff_item_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<HandoffInboxItemRecord> {
    let request_json: String = row.get(5)?;
    let evidence_json: String = row.get(6)?;
    let approval_requirement: String = row.get(7)?;
    let delivery_state: String = row.get(8)?;
    let owner_decision: Option<String> = row.get(9)?;
    let evidence_refs_json: String = row.get(21)?;
    Ok(HandoffInboxItemRecord {
        id: row.get(0)?,
        source_kind: row.get(1)?,
        source_id: row.get(2)?,
        destination_kind: row.get(3)?,
        destination_id: row.get(4)?,
        reason: row.get(15)?,
        requested_action: row.get(16)?,
        urgency: row.get(17)?,
        assignee_actor_id: row.get(18)?,
        due_at: row.get(19)?,
        next_action_hint: row.get(20)?,
        evidence_refs: serde_json::from_str(&evidence_refs_json).unwrap_or_default(),
        visibility: row.get(22)?,
        request: serde_json::from_str(&request_json).unwrap_or_else(|_| json!({})),
        evidence: serde_json::from_str(&evidence_json).unwrap_or_else(|_| json!({})),
        approval_requirement: ApprovalRequirement::try_from(approval_requirement.as_str())
            .map_err(|error| {
                rusqlite::Error::FromSqlConversionFailure(
                    7,
                    rusqlite::types::Type::Text,
                    error.into(),
                )
            })?,
        delivery_state: DeliveryState::try_from(delivery_state.as_str()).map_err(|error| {
            rusqlite::Error::FromSqlConversionFailure(8, rusqlite::types::Type::Text, error.into())
        })?,
        owner_decision: owner_decision
            .as_deref()
            .map(owner_decision_from_str)
            .transpose()
            .map_err(|error| {
                rusqlite::Error::FromSqlConversionFailure(
                    9,
                    rusqlite::types::Type::Text,
                    error.into(),
                )
            })?,
        decision_reason: row.get(10)?,
        created_by_actor_id: row.get(11)?,
        created_at: row.get(12)?,
        updated_at: row.get(13)?,
        resolved_at: row.get(14)?,
    })
}

fn handoff_receipt_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<HandoffReceiptView> {
    let payload_json: String = row.get(3)?;
    Ok(HandoffReceiptView {
        id: row.get(0)?,
        handoff_item_id: row.get(1)?,
        receipt_kind: row.get(2)?,
        payload: serde_json::from_str(&payload_json).unwrap_or_else(|_| json!({})),
        created_at: row.get(4)?,
    })
}

impl AvailabilityScheduleRecord {
    fn into_view(self) -> AvailabilityScheduleView {
        AvailabilityScheduleView {
            id: self.id,
            label: self.label,
            timezone: self.timezone,
            status: self.status,
            windows: self.windows,
            metadata: self.metadata,
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }
}

impl OperatorPresenceRecord {
    fn into_view(self) -> OperatorPresenceView {
        OperatorPresenceView {
            id: self.id,
            status: self.status,
            threshold: self.threshold,
            status_message: self.status_message,
            metadata: self.metadata,
            updated_by_actor_id: self.updated_by_actor_id,
            updated_at: self.updated_at,
        }
    }
}

impl HandoffInboxItemRecord {
    fn into_view(self) -> HandoffInboxItemView {
        HandoffInboxItemView {
            id: self.id,
            source_kind: self.source_kind,
            source_id: self.source_id,
            destination_kind: self.destination_kind,
            destination_id: self.destination_id,
            reason: self.reason,
            requested_action: self.requested_action,
            urgency: self.urgency,
            assignee_actor_id: self.assignee_actor_id,
            due_at: self.due_at,
            next_action_hint: self.next_action_hint,
            evidence_refs: self.evidence_refs,
            visibility: self.visibility,
            request: self.request,
            evidence: self.evidence,
            approval_requirement: self.approval_requirement,
            delivery_state: self.delivery_state,
            owner_decision: self.owner_decision,
            decision_reason: self.decision_reason,
            created_by_actor_id: self.created_by_actor_id,
            created_at: self.created_at,
            updated_at: self.updated_at,
            resolved_at: self.resolved_at,
        }
    }
}

fn schedule_allows_time(
    schedule: &AvailabilityScheduleRecord,
    evaluated_at: DateTime<Utc>,
) -> bool {
    if schedule.windows.is_empty() {
        return true;
    }
    let day_of_week = evaluated_at.weekday().number_from_monday();
    let minute = evaluated_at.hour() * 60 + evaluated_at.minute();
    schedule.windows.iter().any(|window| {
        window.day_of_week == day_of_week
            && window.start_minute <= minute
            && minute < window.end_minute
    })
}

fn threshold_allows_intent(
    threshold: InterruptionThreshold,
    intent: HandoffIntent,
    trust: ConnectionTrustLevel,
) -> (bool, String) {
    match threshold {
        InterruptionThreshold::Open => (true, "threshold_open".to_string()),
        InterruptionThreshold::Selective => (
            trust == ConnectionTrustLevel::Trusted
                || matches!(intent, HandoffIntent::Urgent | HandoffIntent::Money),
            "selective_threshold_requires_trust_money_or_urgency".to_string(),
        ),
        InterruptionThreshold::MoneyOnly => (
            intent == HandoffIntent::Money,
            "money_only_threshold_blocks_non_money_intent".to_string(),
        ),
        InterruptionThreshold::UrgentOnly => (
            intent == HandoffIntent::Urgent,
            "urgent_only_threshold_blocks_non_urgent_intent".to_string(),
        ),
        InterruptionThreshold::Paused => (false, "interruption_threshold_paused".to_string()),
    }
}

fn handoff_query_matches_item(query: &HandoffInboxListQuery, item: &HandoffInboxItemView) -> bool {
    query
        .delivery_state
        .is_none_or(|state| item.delivery_state == state)
        && query
            .assignee_actor_id
            .as_deref()
            .is_none_or(|assignee| item.assignee_actor_id.as_deref() == Some(assignee))
        && query
            .source_kind
            .as_deref()
            .is_none_or(|source_kind| item.source_kind == source_kind)
        && query
            .source_id
            .as_deref()
            .is_none_or(|source_id| item.source_id.as_deref() == Some(source_id))
        && query
            .visibility
            .as_deref()
            .is_none_or(|visibility| item.visibility == visibility)
}

fn validate_windows(windows: &[AvailabilityWindow]) -> Result<()> {
    for window in windows {
        if !(1..=7).contains(&window.day_of_week) {
            bail!("Availability window day_of_week must be 1 through 7.");
        }
        if window.start_minute >= window.end_minute || window.end_minute > 24 * 60 {
            bail!("Availability window minutes must be within one day and start before end.");
        }
    }
    Ok(())
}

fn require_support_source_kind(value: &str) -> Result<String> {
    let source_kind = require_identifier(value, "Source kind")?;
    match source_kind.as_str() {
        "account" | "member" | "actor" | "visitor" | "visitor_session" | "trial" | "connection"
        | "conversation" | "job" | "artifact" | "request" | "offer_acceptance" | "feedback" => {
            Ok(source_kind)
        }
        _ => bail!("Source kind is not supported for the Support handoff queue: {source_kind}"),
    }
}

fn normalize_required_text(
    value: Option<String>,
    label: &str,
    default_value: &str,
) -> Result<String> {
    let normalized = normalize_optional_string(value).unwrap_or_else(|| default_value.to_string());
    if normalized.len() > 500 {
        bail!("{label} must be 500 characters or fewer.");
    }
    Ok(normalized)
}

fn normalize_urgency(value: Option<String>) -> Result<String> {
    let urgency = normalize_optional_string(value).unwrap_or_else(|| "normal".to_string());
    match urgency.as_str() {
        "low" | "normal" | "high" | "urgent" => Ok(urgency),
        _ => bail!("Urgency must be low, normal, high, or urgent."),
    }
}

fn normalize_handoff_visibility(value: Option<String>) -> Result<String> {
    let visibility = normalize_optional_string(value).unwrap_or_else(|| "staff".to_string());
    match visibility.as_str() {
        "staff" | "owner" | "system" => Ok(visibility),
        _ => bail!("Support handoff visibility must be staff, owner, or system."),
    }
}

fn normalize_optional_due_at(value: Option<String>) -> Result<Option<String>> {
    value
        .map(|value| {
            let value = require_identifier(&value, "Due at").or_else(|_| {
                normalize_optional_string(Some(value))
                    .ok_or_else(|| anyhow::anyhow!("Due at is required."))
            })?;
            Ok(DateTime::parse_from_rfc3339(&value)?
                .with_timezone(&Utc)
                .to_rfc3339())
        })
        .transpose()
}

fn normalize_evidence_refs(value: Option<Vec<String>>) -> Result<Vec<String>> {
    value
        .unwrap_or_default()
        .into_iter()
        .map(|evidence_ref| require_identifier(&evidence_ref, "Evidence ref"))
        .collect()
}

fn ensure_handoff_mutable(delivery_state: DeliveryState) -> Result<()> {
    if delivery_state.is_terminal() {
        bail!(
            "Handoff inbox item is already terminal and cannot be mutated: {}",
            delivery_state.as_str()
        );
    }
    Ok(())
}

fn ensure_update_delivery_state(delivery_state: DeliveryState) -> Result<()> {
    match delivery_state {
        DeliveryState::PendingOwnerApproval
        | DeliveryState::Queued
        | DeliveryState::Assigned
        | DeliveryState::ContinueScreening => Ok(()),
        DeliveryState::Declined | DeliveryState::ApprovedLocalOnly => {
            bail!("Use the resolve route for terminal handoff decisions.")
        }
    }
}

fn handoff_update_event_type(
    previous_state: DeliveryState,
    delivery_state: DeliveryState,
    assignee_actor_id: &Option<String>,
) -> &'static str {
    if delivery_state == DeliveryState::ContinueScreening {
        "handoff.inbox.returned_to_ordo"
    } else if delivery_state == DeliveryState::Assigned
        || (assignee_actor_id.is_some() && previous_state != DeliveryState::Assigned)
    {
        "handoff.inbox.assigned"
    } else {
        "handoff.inbox.updated"
    }
}

fn parse_optional_datetime(value: Option<&str>) -> Result<DateTime<Utc>> {
    match value {
        Some(value) => Ok(DateTime::parse_from_rfc3339(value)?.with_timezone(&Utc)),
        None => Ok(Utc::now()),
    }
}

fn merge_evidence(existing: Value, addition: Option<Value>) -> Value {
    match addition {
        Some(addition) => json!({ "existing": existing, "resolution": addition }),
        None => existing,
    }
}

fn owner_decision_from_str(value: &str) -> Result<OwnerDecision> {
    match value {
        "accept" => Ok(OwnerDecision::Accept),
        "decline" => Ok(OwnerDecision::Decline),
        "queue" => Ok(OwnerDecision::Queue),
        "continue_screening" => Ok(OwnerDecision::ContinueScreening),
        _ => bail!("Unsupported owner decision: {value}"),
    }
}

fn normalize_identifier_option(value: Option<String>, label: &str) -> Result<Option<String>> {
    value
        .map(|value| require_identifier(&value, label))
        .transpose()
}

fn require_identifier(value: &str, label: &str) -> Result<String> {
    let normalized = normalize_optional_string(Some(value.to_string()))
        .ok_or_else(|| anyhow::anyhow!("{label} is required."))?;
    if normalized.len() > 160
        || !normalized.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '_' | '-' | '.' | ':' | '/')
        })
    {
        bail!("{label} must be a stable identifier.");
    }
    Ok(normalized)
}

fn normalize_optional_string(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().replace(char::is_whitespace, " "))
        .map(|value| value.split_whitespace().collect::<Vec<_>>().join(" "))
        .filter(|value| !value.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::policy::LOCAL_OWNER_ACTOR_ID;
    use crate::schema::init_database;
    use tempfile::TempDir;

    #[test]
    fn paused_presence_blocks_handoff_with_evidence() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("local.db");
        init_database(&db_path).unwrap();
        update_availability_schedule(
            &db_path,
            AvailabilityScheduleWriteRequest {
                label: None,
                timezone: Some("UTC".to_string()),
                status: Some(AvailabilityScheduleStatus::Active),
                windows: vec![AvailabilityWindow {
                    day_of_week: 1,
                    start_minute: 0,
                    end_minute: 1440,
                }],
                metadata: None,
            },
        )
        .unwrap();
        update_operator_presence(
            &db_path,
            OperatorPresenceWriteRequest {
                status: OperatorPresenceStatus::Paused,
                threshold: InterruptionThreshold::Open,
                status_message: None,
                metadata: None,
            },
            Some(LOCAL_OWNER_ACTOR_ID),
        )
        .unwrap();

        let decision = evaluate_handoff_eligibility(
            &db_path,
            HandoffEligibilityRequest {
                intent: HandoffIntent::Urgent,
                connection_id: Some("connection_1".to_string()),
                connection_trust: Some(ConnectionTrustLevel::Trusted),
                evaluated_at: Some("2026-05-04T12:00:00Z".to_string()),
                evidence: Some(json!({ "source": "test" })),
            },
        )
        .unwrap();

        assert!(!decision.allowed);
        assert_eq!(decision.reason, "operator_presence_blocks_handoff");
        assert_eq!(decision.evidence["presence"]["status"], "paused");
    }

    #[test]
    fn schedule_presence_threshold_and_trust_allow_handoff() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("local.db");
        init_database(&db_path).unwrap();
        update_availability_schedule(
            &db_path,
            AvailabilityScheduleWriteRequest {
                label: Some("Weekdays".to_string()),
                timezone: Some("UTC".to_string()),
                status: Some(AvailabilityScheduleStatus::Active),
                windows: vec![AvailabilityWindow {
                    day_of_week: 5,
                    start_minute: 9 * 60,
                    end_minute: 17 * 60,
                }],
                metadata: None,
            },
        )
        .unwrap();
        update_operator_presence(
            &db_path,
            OperatorPresenceWriteRequest {
                status: OperatorPresenceStatus::Available,
                threshold: InterruptionThreshold::Selective,
                status_message: None,
                metadata: None,
            },
            Some(LOCAL_OWNER_ACTOR_ID),
        )
        .unwrap();

        let decision = evaluate_handoff_eligibility(
            &db_path,
            HandoffEligibilityRequest {
                intent: HandoffIntent::Support,
                connection_id: Some("connection_trusted".to_string()),
                connection_trust: Some(ConnectionTrustLevel::Trusted),
                evaluated_at: Some("2026-05-08T12:00:00Z".to_string()),
                evidence: None,
            },
        )
        .unwrap();

        assert!(decision.allowed);
        assert_eq!(decision.reason, "handoff_allowed");
    }

    #[test]
    fn handoff_inbox_requires_owner_decision_and_never_external_delivery() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("local.db");
        init_database(&db_path).unwrap();
        let (item, event) = create_handoff_inbox_item(
            &db_path,
            HandoffInboxCreateRequest {
                source_kind: "visitor_session".to_string(),
                source_id: Some("session_1".to_string()),
                destination_kind: "owner".to_string(),
                destination_id: Some("actor_local_owner".to_string()),
                reason: None,
                requested_action: None,
                urgency: None,
                assignee_actor_id: None,
                due_at: None,
                next_action_hint: None,
                evidence_refs: None,
                visibility: None,
                request: Some(json!({ "summary": "needs review" })),
                evidence: Some(json!({ "intent": "urgent" })),
                approval_requirement: None,
            },
            Some(LOCAL_OWNER_ACTOR_ID),
        )
        .unwrap();

        assert_eq!(event.event_type, "handoff.inbox.created");
        assert_eq!(item.delivery_state, DeliveryState::PendingOwnerApproval);
        assert_eq!(
            item.approval_requirement,
            ApprovalRequirement::OwnerApprovalRequired
        );
        let (resolved, resolved_event) = resolve_handoff_inbox_item(
            &db_path,
            &item.id,
            HandoffInboxResolveRequest {
                decision: OwnerDecision::Accept,
                decision_reason: Some("owner will handle locally".to_string()),
                evidence: Some(json!({ "ownerReviewed": true })),
            },
            Some(LOCAL_OWNER_ACTOR_ID),
        )
        .unwrap();

        assert_eq!(resolved_event.event_type, "handoff.inbox.resolved");
        assert_eq!(resolved.delivery_state, DeliveryState::ApprovedLocalOnly);
        let receipts = list_handoff_receipts(&db_path, &item.id).unwrap();
        assert!(receipts
            .receipts
            .iter()
            .any(|receipt| receipt.payload["externalDelivery"] == false));
    }

    #[test]
    fn inbox_items_can_be_listed_and_declined() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("local.db");
        init_database(&db_path).unwrap();
        let (item, _) = create_handoff_inbox_item(
            &db_path,
            HandoffInboxCreateRequest {
                source_kind: "connection".to_string(),
                source_id: Some("connection_1".to_string()),
                destination_kind: "owner".to_string(),
                destination_id: None,
                reason: None,
                requested_action: None,
                urgency: None,
                assignee_actor_id: None,
                due_at: None,
                next_action_hint: None,
                evidence_refs: None,
                visibility: None,
                request: None,
                evidence: None,
                approval_requirement: Some(ApprovalRequirement::OwnerApprovalRequired),
            },
            Some(LOCAL_OWNER_ACTOR_ID),
        )
        .unwrap();
        resolve_handoff_inbox_item(
            &db_path,
            &item.id,
            HandoffInboxResolveRequest {
                decision: OwnerDecision::Decline,
                decision_reason: Some("not needed".to_string()),
                evidence: None,
            },
            Some(LOCAL_OWNER_ACTOR_ID),
        )
        .unwrap();

        let items = list_handoff_inbox(&db_path).unwrap();
        assert_eq!(items.items.len(), 1);
        assert_eq!(items.items[0].delivery_state, DeliveryState::Declined);
        assert_eq!(items.items[0].owner_decision, Some(OwnerDecision::Decline));
    }

    #[test]
    fn support_queue_fields_filter_assignment_and_return_are_durable() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("local.db");
        init_database(&db_path).unwrap();
        let (item, _) = create_handoff_inbox_item(
            &db_path,
            HandoffInboxCreateRequest {
                source_kind: "conversation".to_string(),
                source_id: Some("conversation_1".to_string()),
                destination_kind: "support".to_string(),
                destination_id: Some("support_queue".to_string()),
                reason: Some("Trial user asked for strategy help".to_string()),
                requested_action: Some("Schedule Keith review".to_string()),
                urgency: Some("high".to_string()),
                assignee_actor_id: Some("actor_keith".to_string()),
                due_at: Some("2026-05-14T15:00:00Z".to_string()),
                next_action_hint: Some("Review conversation brief first".to_string()),
                evidence_refs: Some(vec![
                    "conversation:conversation_1".to_string(),
                    "message:message_1".to_string(),
                ]),
                visibility: Some("staff".to_string()),
                request: Some(json!({ "summary": "needs strategy follow-up" })),
                evidence: Some(json!({ "intent": "strategy_session" })),
                approval_requirement: None,
            },
            Some(LOCAL_OWNER_ACTOR_ID),
        )
        .unwrap();

        assert_eq!(item.reason, "Trial user asked for strategy help");
        assert_eq!(item.requested_action, "Schedule Keith review");
        assert_eq!(item.urgency, "high");
        assert_eq!(item.assignee_actor_id.as_deref(), Some("actor_keith"));
        assert_eq!(item.due_at.as_deref(), Some("2026-05-14T15:00:00+00:00"));
        assert_eq!(
            item.next_action_hint.as_deref(),
            Some("Review conversation brief first")
        );
        assert_eq!(
            item.evidence_refs,
            vec![
                "conversation:conversation_1".to_string(),
                "message:message_1".to_string()
            ]
        );
        assert_eq!(item.visibility, "staff");

        let assigned_items = list_handoff_inbox_with_query(
            &db_path,
            HandoffInboxListQuery {
                assignee_actor_id: Some("actor_keith".to_string()),
                source_kind: Some("conversation".to_string()),
                delivery_state: Some(DeliveryState::PendingOwnerApproval),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(assigned_items.items.len(), 1);
        assert_eq!(assigned_items.items[0].id, item.id);

        let (assigned, assigned_event) = update_handoff_inbox_item(
            &db_path,
            &item.id,
            HandoffInboxUpdateRequest {
                assignee_actor_id: Some("actor_staff".to_string()),
                delivery_state: Some(DeliveryState::Assigned),
                evidence: Some(json!({ "assignmentReason": "staff owns NYC pilot" })),
                ..Default::default()
            },
            Some("actor_staff"),
        )
        .unwrap();
        assert_eq!(assigned.delivery_state, DeliveryState::Assigned);
        assert_eq!(assigned.assignee_actor_id.as_deref(), Some("actor_staff"));
        assert_eq!(assigned_event.event_type, "handoff.inbox.assigned");

        let (returned, returned_event) = update_handoff_inbox_item(
            &db_path,
            &item.id,
            HandoffInboxUpdateRequest {
                delivery_state: Some(DeliveryState::ContinueScreening),
                next_action_hint: Some("Ask one more qualifying question".to_string()),
                evidence: Some(json!({ "returnReason": "needs more context" })),
                ..Default::default()
            },
            Some("actor_staff"),
        )
        .unwrap();
        assert_eq!(returned.delivery_state, DeliveryState::ContinueScreening);
        assert_eq!(
            returned.next_action_hint.as_deref(),
            Some("Ask one more qualifying question")
        );
        assert_eq!(returned_event.event_type, "handoff.inbox.returned_to_ordo");
    }

    #[test]
    fn support_queue_rejects_public_visibility_and_unsupported_sources() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("local.db");
        init_database(&db_path).unwrap();

        let public_visibility = create_handoff_inbox_item(
            &db_path,
            HandoffInboxCreateRequest {
                source_kind: "conversation".to_string(),
                source_id: Some("conversation_1".to_string()),
                destination_kind: "support".to_string(),
                destination_id: None,
                reason: Some("needs help".to_string()),
                requested_action: None,
                urgency: None,
                assignee_actor_id: None,
                due_at: None,
                next_action_hint: None,
                evidence_refs: None,
                visibility: Some("public".to_string()),
                request: None,
                evidence: None,
                approval_requirement: None,
            },
            Some(LOCAL_OWNER_ACTOR_ID),
        );
        assert!(public_visibility.is_err());

        let provider_source = create_handoff_inbox_item(
            &db_path,
            HandoffInboxCreateRequest {
                source_kind: "provider_secret".to_string(),
                source_id: Some("secret_1".to_string()),
                destination_kind: "support".to_string(),
                destination_id: None,
                reason: Some("needs help".to_string()),
                requested_action: None,
                urgency: None,
                assignee_actor_id: None,
                due_at: None,
                next_action_hint: None,
                evidence_refs: None,
                visibility: None,
                request: None,
                evidence: None,
                approval_requirement: None,
            },
            Some(LOCAL_OWNER_ACTOR_ID),
        );
        assert!(provider_source.is_err());
    }

    #[test]
    fn terminal_support_queue_items_cannot_be_mutated_again() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("local.db");
        init_database(&db_path).unwrap();
        let (item, _) = create_handoff_inbox_item(
            &db_path,
            HandoffInboxCreateRequest {
                source_kind: "connection".to_string(),
                source_id: Some("connection_1".to_string()),
                destination_kind: "support".to_string(),
                destination_id: None,
                reason: Some("needs review".to_string()),
                requested_action: None,
                urgency: None,
                assignee_actor_id: None,
                due_at: None,
                next_action_hint: None,
                evidence_refs: None,
                visibility: None,
                request: None,
                evidence: None,
                approval_requirement: None,
            },
            Some(LOCAL_OWNER_ACTOR_ID),
        )
        .unwrap();

        resolve_handoff_inbox_item(
            &db_path,
            &item.id,
            HandoffInboxResolveRequest {
                decision: OwnerDecision::Decline,
                decision_reason: Some("not a fit".to_string()),
                evidence: None,
            },
            Some(LOCAL_OWNER_ACTOR_ID),
        )
        .unwrap();

        let update = update_handoff_inbox_item(
            &db_path,
            &item.id,
            HandoffInboxUpdateRequest {
                assignee_actor_id: Some("actor_staff".to_string()),
                ..Default::default()
            },
            Some("actor_staff"),
        );
        assert!(update.is_err());

        let resolve_again = resolve_handoff_inbox_item(
            &db_path,
            &item.id,
            HandoffInboxResolveRequest {
                decision: OwnerDecision::Accept,
                decision_reason: Some("changed mind".to_string()),
                evidence: None,
            },
            Some(LOCAL_OWNER_ACTOR_ID),
        );
        assert!(resolve_again.is_err());
    }
}
