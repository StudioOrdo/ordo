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
    Declined,
    ContinueScreening,
    ApprovedLocalOnly,
}

impl DeliveryState {
    fn as_str(self) -> &'static str {
        match self {
            Self::PendingOwnerApproval => "pending_owner_approval",
            Self::Queued => "queued",
            Self::Declined => "declined",
            Self::ContinueScreening => "continue_screening",
            Self::ApprovedLocalOnly => "approved_local_only",
        }
    }
}

impl TryFrom<&str> for DeliveryState {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> Result<Self> {
        match value {
            "pending_owner_approval" => Ok(Self::PendingOwnerApproval),
            "queued" => Ok(Self::Queued),
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

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HandoffInboxItemView {
    pub id: String,
    pub source_kind: String,
    pub source_id: Option<String>,
    pub destination_kind: String,
    pub destination_id: Option<String>,
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
    pub request: Option<Value>,
    pub evidence: Option<Value>,
    pub approval_requirement: Option<ApprovalRequirement>,
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
    let connection = Connection::open(db_path)?;
    let mut statement = connection.prepare(
        "SELECT id, source_kind, source_id, destination_kind, destination_id, request_json,
                evidence_json, approval_requirement, delivery_state, owner_decision,
                decision_reason, created_by_actor_id, created_at, updated_at, resolved_at
         FROM handoff_inbox_items
         ORDER BY updated_at DESC, id DESC",
    )?;
    let items = statement
        .query_map([], handoff_item_from_row)?
        .map(|row| row.map(HandoffInboxItemRecord::into_view))
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(HandoffInboxListResponse { items })
}

pub fn create_handoff_inbox_item(
    db_path: &Path,
    request: HandoffInboxCreateRequest,
    actor_id: Option<&str>,
) -> Result<(HandoffInboxItemView, RealtimeEvent)> {
    let source_kind = require_identifier(&request.source_kind, "Source kind")?;
    let destination_kind = require_identifier(&request.destination_kind, "Destination kind")?;
    let source_id = normalize_identifier_option(request.source_id, "Source id")?;
    let destination_id = normalize_identifier_option(request.destination_id, "Destination id")?;
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
            decision_reason, created_by_actor_id, created_at, updated_at, resolved_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 'pending_owner_approval', NULL, NULL, ?9, ?10, ?10, NULL)",
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
                "externalDelivery": false,
            }),
        ),
    )?;
    transaction.commit()?;
    let item = find_handoff_item_by_id(&connection, item_id)?.expect("handoff item just updated");
    let _ = actor_id;
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
                    decision_reason, created_by_actor_id, created_at, updated_at, resolved_at
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
    Ok(HandoffInboxItemRecord {
        id: row.get(0)?,
        source_kind: row.get(1)?,
        source_id: row.get(2)?,
        destination_kind: row.get(3)?,
        destination_id: row.get(4)?,
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
}
