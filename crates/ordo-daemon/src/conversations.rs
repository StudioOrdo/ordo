use anyhow::{bail, ensure, Result};
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension, Row, Transaction};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::events::{append_realtime_event, append_realtime_event_tx, RealtimeEvent};
use crate::policy::{
    record_policy_decision, ActorContext, ActorKind, PolicyAction, PolicyDecision,
    PolicyDecisionCorrelation, PolicyOutcome, ResourceKind, ResourceRef, LOCAL_OWNER_ACTOR_ID,
    SYSTEM_ACTOR_ID,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CandidateState {
    Proposed,
    Confirmed,
    Rejected,
    Superseded,
}

impl CandidateState {
    fn as_str(self) -> &'static str {
        match self {
            CandidateState::Proposed => "proposed",
            CandidateState::Confirmed => "confirmed",
            CandidateState::Rejected => "rejected",
            CandidateState::Superseded => "superseded",
        }
    }
}

impl TryFrom<&str> for CandidateState {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> Result<Self> {
        match value {
            "proposed" => Ok(CandidateState::Proposed),
            "confirmed" => Ok(CandidateState::Confirmed),
            "rejected" => Ok(CandidateState::Rejected),
            "superseded" => Ok(CandidateState::Superseded),
            other => bail!("Unsupported candidate state: {other}"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConversationMode {
    AgentLed,
    HumanLedActive,
    HumanLedIdle,
    AssistivePrivate,
    NeedsHandoff,
    ReturnedToAgent,
}

impl ConversationMode {
    fn as_str(self) -> &'static str {
        match self {
            ConversationMode::AgentLed => "agent_led",
            ConversationMode::HumanLedActive => "human_led_active",
            ConversationMode::HumanLedIdle => "human_led_idle",
            ConversationMode::AssistivePrivate => "assistive_private",
            ConversationMode::NeedsHandoff => "needs_handoff",
            ConversationMode::ReturnedToAgent => "returned_to_agent",
        }
    }
}

impl TryFrom<&str> for ConversationMode {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> Result<Self> {
        match value {
            "agent_led" => Ok(ConversationMode::AgentLed),
            "human_led_active" => Ok(ConversationMode::HumanLedActive),
            "human_led_idle" => Ok(ConversationMode::HumanLedIdle),
            "assistive_private" => Ok(ConversationMode::AssistivePrivate),
            "needs_handoff" => Ok(ConversationMode::NeedsHandoff),
            "returned_to_agent" => Ok(ConversationMode::ReturnedToAgent),
            other => bail!("Unsupported conversation mode: {other}"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HandoffStatus {
    Suggested,
    Requested,
    Accepted,
    Declined,
    Assigned,
    InProgress,
    ReturnedToAgent,
    Closed,
}

impl HandoffStatus {
    fn as_str(self) -> &'static str {
        match self {
            HandoffStatus::Suggested => "suggested",
            HandoffStatus::Requested => "requested",
            HandoffStatus::Accepted => "accepted",
            HandoffStatus::Declined => "declined",
            HandoffStatus::Assigned => "assigned",
            HandoffStatus::InProgress => "in_progress",
            HandoffStatus::ReturnedToAgent => "returned_to_agent",
            HandoffStatus::Closed => "closed",
        }
    }

    fn is_terminal(self) -> bool {
        matches!(self, HandoffStatus::Declined | HandoffStatus::Closed)
    }
}

impl TryFrom<&str> for HandoffStatus {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> Result<Self> {
        match value {
            "suggested" => Ok(HandoffStatus::Suggested),
            "requested" => Ok(HandoffStatus::Requested),
            "accepted" => Ok(HandoffStatus::Accepted),
            "declined" => Ok(HandoffStatus::Declined),
            "assigned" => Ok(HandoffStatus::Assigned),
            "in_progress" => Ok(HandoffStatus::InProgress),
            "returned_to_agent" => Ok(HandoffStatus::ReturnedToAgent),
            "closed" => Ok(HandoffStatus::Closed),
            other => bail!("Unsupported handoff status: {other}"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConversationRole {
    Client,
    Staff,
    Manager,
    Admin,
    Owner,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QueueScope {
    MyHandoffs,
    TeamQueue,
    AllConversations,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CanonicalConversationRequest {
    pub surface: String,
    pub subject_kind: String,
    pub subject_id: String,
    pub connection_id: Option<String>,
    pub visitor_session_id: Option<String>,
    pub created_by_actor_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConversationSummary {
    pub id: String,
    pub surface: String,
    pub subject_kind: String,
    pub subject_id: String,
    pub connection_id: Option<String>,
    pub status: String,
    pub unread_count: i64,
    pub action_count: i64,
    pub last_meaningful_change: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EpisodeCandidateRequest {
    pub conversation_id: String,
    pub title: String,
    pub segment_kind: String,
    pub evidence_refs: Vec<String>,
    pub confidence: f64,
    pub provenance: Value,
    pub created_by_job_id: Option<String>,
    pub source_kind: Option<String>,
    pub source_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConversationSegmentView {
    pub id: String,
    pub conversation_id: String,
    pub title: String,
    pub segment_kind: String,
    pub status: String,
    pub candidate_state: CandidateState,
    pub confidence: f64,
    pub evidence_refs: Vec<String>,
    pub provenance: Value,
    pub created_by_job_id: Option<String>,
    pub source_kind: String,
    pub source_id: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConversationHandoffCreateRequest {
    pub conversation_id: String,
    pub segment_id: Option<String>,
    pub connection_id: Option<String>,
    pub requested_by_actor_id: Option<String>,
    pub assigned_to_actor_id: Option<String>,
    pub reason: String,
    pub urgency: String,
    pub required_capability_id: String,
    pub evidence_summary: String,
    pub allowed_context: Vec<String>,
    pub policy_decision_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConversationHandoffView {
    pub id: String,
    pub conversation_id: String,
    pub segment_id: Option<String>,
    pub connection_id: Option<String>,
    pub requested_by_actor_id: Option<String>,
    pub assigned_to_actor_id: Option<String>,
    pub reason: String,
    pub urgency: String,
    pub required_capability_id: String,
    pub evidence_summary: String,
    pub allowed_context: Vec<String>,
    pub status: HandoffStatus,
    pub policy_decision_id: Option<String>,
    pub receipt: Value,
    pub created_at: String,
    pub updated_at: String,
    pub closed_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HandoffBriefView {
    pub handoff_id: String,
    pub conversation_id: String,
    pub reason: String,
    pub urgency: String,
    pub status: HandoffStatus,
    pub assigned_to_actor_id: Option<String>,
    pub required_capability_id: String,
    pub evidence_summary: String,
    pub allowed_context: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConversationModeView {
    pub conversation_id: String,
    pub mode: ConversationMode,
    pub led_by_actor_id: Option<String>,
    pub delegated_to_agent: bool,
    pub delegation_scope: Vec<String>,
    pub idle_after: Option<String>,
    pub private_reminder_sent_at: Option<String>,
    pub updated_at: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PublicPostContext {
    pub tagged: bool,
    pub delegated: bool,
    pub policy_required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PublicPostDecision {
    pub allowed: bool,
    pub reason: String,
    pub private_reminder_required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConversationQueueRow {
    pub conversation_id: String,
    pub handoff_id: Option<String>,
    pub why: String,
    pub urgency: String,
    pub handoff_status: Option<HandoffStatus>,
    pub connection_id: Option<String>,
    pub assigned_actor_id: Option<String>,
    pub last_meaningful_change: String,
    pub unread_count: i64,
    pub action_count: i64,
    pub evidence_summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConversationParticipantCreateRequest {
    pub conversation_id: String,
    pub participant_kind: String,
    pub actor_id: Option<String>,
    pub connection_id: Option<String>,
    pub visitor_session_id: Option<String>,
    pub display_name: String,
    pub role: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConversationParticipantView {
    pub id: String,
    pub conversation_id: String,
    pub participant_kind: String,
    pub actor_id: Option<String>,
    pub connection_id: Option<String>,
    pub visitor_session_id: Option<String>,
    pub display_name: String,
    pub role: String,
    pub status: String,
    pub joined_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConversationMessageCreateRequest {
    pub conversation_id: String,
    pub segment_id: Option<String>,
    pub participant_id: String,
    pub message_kind: String,
    pub body_markdown: String,
    pub visibility: String,
    pub client_message_id: String,
    pub reply_to_message_id: Option<String>,
    pub undo_expires_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConversationMessageView {
    pub id: String,
    pub conversation_id: String,
    pub segment_id: Option<String>,
    pub participant_id: String,
    pub message_kind: String,
    pub status: String,
    pub body_markdown: String,
    pub visibility: String,
    pub client_message_id: Option<String>,
    pub sequence: i64,
    pub event_cursor: Option<i64>,
    pub undo_expires_at: Option<String>,
    pub undo_cancelled_at: Option<String>,
    pub created_at: String,
    pub edited_at: Option<String>,
    pub deleted_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConversationMessageRevisionView {
    pub id: String,
    pub message_id: String,
    pub revision_number: i64,
    pub body_markdown: String,
    pub edited_by_participant_id: String,
    pub reason: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone)]
pub struct ConversationMutationActor {
    pub actor: ActorContext,
    pub request_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConversationMutationReceipt<T> {
    pub value: T,
    pub policy_decision_id: String,
}

pub struct ConversationService;

struct ConversationMutationPolicyTarget<'a> {
    conversation_id: &'a str,
    participant_id: &'a str,
    action: PolicyAction,
    capability_id: &'a str,
    resource_kind: ResourceKind,
    resource_id: &'a str,
}

impl ConversationService {
    pub fn submit_message(
        connection: &Connection,
        actor: &ConversationMutationActor,
        request: &ConversationMessageCreateRequest,
    ) -> Result<ConversationMutationReceipt<ConversationMessageView>> {
        let policy_decision_id = authorize_participant_mutation(
            connection,
            actor,
            ConversationMutationPolicyTarget {
                conversation_id: &request.conversation_id,
                participant_id: &request.participant_id,
                action: PolicyAction::Create,
                capability_id: "conversation.message.create",
                resource_kind: ResourceKind::Conversation,
                resource_id: &request.conversation_id,
            },
        )?;
        let value = create_conversation_message(connection, request)?;
        Ok(ConversationMutationReceipt {
            value,
            policy_decision_id,
        })
    }

    pub fn edit_message(
        connection: &Connection,
        actor: &ConversationMutationActor,
        message_id: &str,
        edited_by_participant_id: &str,
        body_markdown: &str,
        reason: Option<&str>,
    ) -> Result<ConversationMutationReceipt<ConversationMessageView>> {
        let current = load_message(connection, message_id)?;
        let policy_decision_id = authorize_participant_mutation(
            connection,
            actor,
            ConversationMutationPolicyTarget {
                conversation_id: &current.conversation_id,
                participant_id: edited_by_participant_id,
                action: PolicyAction::Update,
                capability_id: "conversation.message.edit",
                resource_kind: ResourceKind::ConversationMessage,
                resource_id: message_id,
            },
        )?;
        let value = edit_conversation_message(
            connection,
            message_id,
            edited_by_participant_id,
            body_markdown,
            reason,
        )?;
        Ok(ConversationMutationReceipt {
            value,
            policy_decision_id,
        })
    }

    pub fn delete_message(
        connection: &Connection,
        actor: &ConversationMutationActor,
        message_id: &str,
        deleted_by_participant_id: &str,
        reason: &str,
    ) -> Result<ConversationMutationReceipt<ConversationMessageView>> {
        let current = load_message(connection, message_id)?;
        let policy_decision_id = authorize_participant_mutation(
            connection,
            actor,
            ConversationMutationPolicyTarget {
                conversation_id: &current.conversation_id,
                participant_id: deleted_by_participant_id,
                action: PolicyAction::Update,
                capability_id: "conversation.message.delete",
                resource_kind: ResourceKind::ConversationMessage,
                resource_id: message_id,
            },
        )?;
        let value =
            delete_conversation_message(connection, message_id, deleted_by_participant_id, reason)?;
        Ok(ConversationMutationReceipt {
            value,
            policy_decision_id,
        })
    }

    pub fn undo_message(
        connection: &Connection,
        actor: &ConversationMutationActor,
        message_id: &str,
        participant_id: &str,
    ) -> Result<ConversationMutationReceipt<ConversationMessageView>> {
        let current = load_message(connection, message_id)?;
        let policy_decision_id = authorize_participant_mutation(
            connection,
            actor,
            ConversationMutationPolicyTarget {
                conversation_id: &current.conversation_id,
                participant_id,
                action: PolicyAction::Update,
                capability_id: "conversation.message.delete",
                resource_kind: ResourceKind::ConversationMessage,
                resource_id: message_id,
            },
        )?;
        let value = undo_conversation_message(connection, message_id, participant_id)?;
        Ok(ConversationMutationReceipt {
            value,
            policy_decision_id,
        })
    }
}

pub fn find_or_create_canonical_conversation(
    connection: &Connection,
    request: &CanonicalConversationRequest,
) -> Result<ConversationSummary> {
    require_text("surface", &request.surface)?;
    require_text("subject_kind", &request.subject_kind)?;
    require_text("subject_id", &request.subject_id)?;

    if let Some(existing) = connection
        .query_row(
            "SELECT id, surface, subject_kind, subject_id, connection_id, status, unread_count,
                    action_count, last_meaningful_change, updated_at
             FROM conversations
             WHERE surface = ?1 AND subject_kind = ?2 AND subject_id = ?3 AND archived_at IS NULL
             ORDER BY updated_at DESC
             LIMIT 1",
            params![request.surface, request.subject_kind, request.subject_id],
            conversation_summary_from_row,
        )
        .optional()?
    {
        return Ok(existing);
    }

    let now = Utc::now().to_rfc3339();
    let conversation_id = format!("conversation_{}", Uuid::new_v4());
    connection.execute(
        "INSERT INTO conversations (
            id, surface, subject_kind, subject_id, connection_id, visitor_session_id, status,
            visibility, privacy_scope, last_meaningful_change, created_by_actor_id, created_at, updated_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'active', 'participant', 'relationship',
                   'conversation.created', ?7, ?8, ?8)",
        params![
            conversation_id,
            request.surface,
            request.subject_kind,
            request.subject_id,
            request.connection_id,
            request.visitor_session_id,
            request.created_by_actor_id,
            now
        ],
    )?;
    upsert_conversation_mode(
        connection,
        &conversation_id,
        ConversationMode::AgentLed,
        None,
        false,
        vec![],
        None,
    )?;
    append_conversation_event(
        connection,
        &conversation_id,
        None,
        None,
        "conversation.created",
        json!({
            "surface": request.surface,
            "subjectKind": request.subject_kind,
            "subjectId": request.subject_id,
        }),
        None,
    )?;

    load_conversation_summary(connection, &conversation_id)
}

pub fn add_episode_candidate(
    connection: &Connection,
    request: &EpisodeCandidateRequest,
) -> Result<ConversationSegmentView> {
    require_text("conversation_id", &request.conversation_id)?;
    require_text("title", &request.title)?;
    require_text("segment_kind", &request.segment_kind)?;
    ensure!(
        !request.evidence_refs.is_empty(),
        "episode candidate requires evidence refs"
    );
    ensure!(
        (0.0..=1.0).contains(&request.confidence),
        "episode candidate confidence must be between 0 and 1"
    );
    ensure!(
        !request.provenance.is_null() && request.provenance != json!({}),
        "episode candidate requires provenance"
    );

    let source_kind = request.source_kind.clone().unwrap_or_default();
    let source_id = request.source_id.clone().unwrap_or_default();
    if !source_kind.is_empty() && !source_id.is_empty() {
        if let Some(existing_id) = connection
            .query_row(
                "SELECT id FROM conversation_segments
                 WHERE conversation_id = ?1 AND segment_kind = ?2
                   AND source_kind = ?3 AND source_id = ?4
                   AND ((created_by_job_id IS NULL AND ?5 IS NULL) OR created_by_job_id = ?5)
                 LIMIT 1",
                params![
                    request.conversation_id,
                    request.segment_kind,
                    source_kind,
                    source_id,
                    request.created_by_job_id
                ],
                |row| row.get::<_, String>(0),
            )
            .optional()?
        {
            return load_segment(connection, &existing_id);
        }
    }

    let now = Utc::now().to_rfc3339();
    let segment_id = format!("segment_{}", Uuid::new_v4());
    connection.execute(
        "INSERT INTO conversation_segments (
            id, conversation_id, segment_kind, title, status, candidate_state, confidence,
            evidence_refs_json, provenance_json, created_by_job_id, source_kind, source_id,
            started_at, created_at, updated_at
         ) VALUES (?1, ?2, ?3, ?4, 'active', 'proposed', ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?11, ?11)",
        params![
            segment_id,
            request.conversation_id,
            request.segment_kind,
            request.title,
            request.confidence,
            serde_json::to_string(&request.evidence_refs)?,
            request.provenance.to_string(),
            request.created_by_job_id,
            source_kind,
            source_id,
            now
        ],
    )?;
    connection.execute(
        "UPDATE conversations
         SET current_segment_id = ?1, last_meaningful_change = 'conversation.episode.proposed', updated_at = ?2
         WHERE id = ?3",
        params![segment_id, now, request.conversation_id],
    )?;
    append_conversation_event(
        connection,
        &request.conversation_id,
        Some(&segment_id),
        None,
        "conversation.episode.proposed",
        json!({
            "segmentId": segment_id,
            "candidateState": CandidateState::Proposed.as_str(),
            "evidenceRefs": request.evidence_refs,
            "provenance": request.provenance,
        }),
        None,
    )?;

    load_segment(connection, &segment_id)
}

pub fn client_conversation_summaries(
    connection: &Connection,
    subject_kind: &str,
    subject_id: &str,
) -> Result<Vec<ConversationSummary>> {
    require_text("subject_kind", subject_kind)?;
    require_text("subject_id", subject_id)?;
    let mut statement = connection.prepare(
        "SELECT id, surface, subject_kind, subject_id, connection_id, status, unread_count,
                action_count, last_meaningful_change, updated_at
         FROM conversations
         WHERE subject_kind = ?1 AND subject_id = ?2 AND archived_at IS NULL
         ORDER BY updated_at DESC",
    )?;
    let rows = statement.query_map(
        params![subject_kind, subject_id],
        conversation_summary_from_row,
    )?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(Into::into)
}

pub fn staff_episode_details(
    connection: &Connection,
    conversation_id: &str,
) -> Result<Vec<ConversationSegmentView>> {
    require_text("conversation_id", conversation_id)?;
    let mut statement = connection.prepare(
        "SELECT id, conversation_id, title, segment_kind, status, candidate_state, confidence,
                evidence_refs_json, provenance_json, created_by_job_id, source_kind, source_id,
                created_at, updated_at
         FROM conversation_segments
         WHERE conversation_id = ?1
         ORDER BY started_at DESC",
    )?;
    let rows = statement.query_map([conversation_id], segment_from_row)?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(Into::into)
}

pub fn create_conversation_handoff(
    connection: &Connection,
    request: &ConversationHandoffCreateRequest,
) -> Result<ConversationHandoffView> {
    validate_handoff_request(request)?;
    let now = Utc::now().to_rfc3339();
    let handoff_id = format!("handoff_{}", Uuid::new_v4());
    let receipt = json!({
        "receiptKind": "conversation_handoff_created",
        "createdAt": now,
        "policyDecisionId": request.policy_decision_id,
    });

    connection.execute(
        "INSERT INTO conversation_handoffs (
            id, conversation_id, segment_id, connection_id, requested_by_actor_id, assigned_to_actor_id,
            reason, urgency, required_capability_id, evidence_summary, allowed_context_json,
            status, policy_decision_id, receipt_json, created_at, updated_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, 'requested', ?12, ?13, ?14, ?14)",
        params![
            handoff_id,
            request.conversation_id,
            request.segment_id,
            request.connection_id,
            request.requested_by_actor_id,
            request.assigned_to_actor_id,
            request.reason,
            request.urgency,
            request.required_capability_id,
            request.evidence_summary,
            serde_json::to_string(&request.allowed_context)?,
            request.policy_decision_id,
            receipt.to_string(),
            now
        ],
    )?;
    connection.execute(
        "UPDATE conversations
         SET last_meaningful_change = 'conversation.handoff.requested', action_count = action_count + 1, updated_at = ?1
         WHERE id = ?2",
        params![now, request.conversation_id],
    )?;
    append_conversation_event(
        connection,
        &request.conversation_id,
        request.segment_id.as_deref(),
        Some(&handoff_id),
        "conversation.handoff.requested",
        json!({
            "handoffId": handoff_id,
            "reason": request.reason,
            "urgency": request.urgency,
            "requiredCapability": request.required_capability_id,
            "evidenceSummary": request.evidence_summary,
        }),
        request.policy_decision_id.as_deref(),
    )?;

    load_handoff(connection, &handoff_id)
}

pub fn transition_conversation_handoff(
    connection: &Connection,
    handoff_id: &str,
    next_status: HandoffStatus,
    actor_id: Option<&str>,
    reason: &str,
) -> Result<ConversationHandoffView> {
    require_text("handoff_id", handoff_id)?;
    require_text("reason", reason)?;
    let current = load_handoff(connection, handoff_id)?;
    ensure!(
        valid_handoff_transition(current.status, next_status),
        "Invalid handoff transition from {} to {}",
        current.status.as_str(),
        next_status.as_str()
    );

    let now = Utc::now().to_rfc3339();
    let closed_at = if next_status.is_terminal() {
        Some(now.clone())
    } else {
        current.closed_at.clone()
    };
    let assigned_to_actor_id = actor_id
        .map(ToString::to_string)
        .or(current.assigned_to_actor_id.clone());
    connection.execute(
        "UPDATE conversation_handoffs
         SET status = ?1, assigned_to_actor_id = ?2, updated_at = ?3, closed_at = ?4
         WHERE id = ?5",
        params![
            next_status.as_str(),
            assigned_to_actor_id,
            now,
            closed_at,
            handoff_id
        ],
    )?;
    connection.execute(
        "UPDATE conversations
         SET last_meaningful_change = ?1, updated_at = ?2
         WHERE id = ?3",
        params![
            format!("conversation.handoff.{}", next_status.as_str()),
            now,
            current.conversation_id
        ],
    )?;
    append_conversation_event(
        connection,
        &current.conversation_id,
        current.segment_id.as_deref(),
        Some(handoff_id),
        &format!("conversation.handoff.{}", next_status.as_str()),
        json!({
            "handoffId": handoff_id,
            "fromStatus": current.status.as_str(),
            "toStatus": next_status.as_str(),
            "actorId": actor_id,
            "reason": reason,
        }),
        current.policy_decision_id.as_deref(),
    )?;

    load_handoff(connection, handoff_id)
}

pub fn handoff_brief(connection: &Connection, handoff_id: &str) -> Result<HandoffBriefView> {
    let handoff = load_handoff(connection, handoff_id)?;
    Ok(HandoffBriefView {
        handoff_id: handoff.id,
        conversation_id: handoff.conversation_id,
        reason: handoff.reason,
        urgency: handoff.urgency,
        status: handoff.status,
        assigned_to_actor_id: handoff.assigned_to_actor_id,
        required_capability_id: handoff.required_capability_id,
        evidence_summary: handoff.evidence_summary,
        allowed_context: handoff.allowed_context,
    })
}

pub fn upsert_conversation_mode(
    connection: &Connection,
    conversation_id: &str,
    mode: ConversationMode,
    led_by_actor_id: Option<&str>,
    delegated_to_agent: bool,
    delegation_scope: Vec<String>,
    idle_after: Option<&str>,
) -> Result<ConversationModeView> {
    require_text("conversation_id", conversation_id)?;
    let now = Utc::now().to_rfc3339();
    connection.execute(
        "INSERT INTO conversation_modes (
            conversation_id, mode, led_by_actor_id, delegated_to_agent, delegation_scope_json,
            idle_after, updated_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
         ON CONFLICT(conversation_id) DO UPDATE SET
            mode = excluded.mode,
            led_by_actor_id = excluded.led_by_actor_id,
            delegated_to_agent = excluded.delegated_to_agent,
            delegation_scope_json = excluded.delegation_scope_json,
            idle_after = excluded.idle_after,
            updated_at = excluded.updated_at",
        params![
            conversation_id,
            mode.as_str(),
            led_by_actor_id,
            if delegated_to_agent { 1 } else { 0 },
            serde_json::to_string(&delegation_scope)?,
            idle_after,
            now
        ],
    )?;
    append_conversation_event(
        connection,
        conversation_id,
        None,
        None,
        "conversation.mode.changed",
        json!({
            "mode": mode.as_str(),
            "ledByActorId": led_by_actor_id,
            "delegatedToAgent": delegated_to_agent,
            "delegationScope": delegation_scope,
        }),
        None,
    )?;
    load_conversation_mode(connection, conversation_id)
}

pub fn record_staff_activity_sets_human_led(
    connection: &Connection,
    conversation_id: &str,
    staff_actor_id: &str,
) -> Result<ConversationModeView> {
    upsert_conversation_mode(
        connection,
        conversation_id,
        ConversationMode::HumanLedActive,
        Some(staff_actor_id),
        false,
        vec![],
        None,
    )
}

pub fn mark_human_led_idle_private_reminder(
    connection: &Connection,
    conversation_id: &str,
) -> Result<ConversationModeView> {
    require_text("conversation_id", conversation_id)?;
    let now = Utc::now().to_rfc3339();
    connection.execute(
        "UPDATE conversation_modes
         SET mode = 'human_led_idle', private_reminder_sent_at = ?1, updated_at = ?1
         WHERE conversation_id = ?2",
        params![now, conversation_id],
    )?;
    append_conversation_event(
        connection,
        conversation_id,
        None,
        None,
        "conversation.human_idle.private_reminder",
        json!({ "privateReminderSentAt": now }),
        None,
    )?;
    load_conversation_mode(connection, conversation_id)
}

pub fn may_agent_post_publicly(
    mode: ConversationMode,
    context: &PublicPostContext,
) -> PublicPostDecision {
    if context.policy_required {
        return PublicPostDecision {
            allowed: true,
            reason: "policy_required".to_string(),
            private_reminder_required: false,
        };
    }
    if context.tagged || context.delegated {
        return PublicPostDecision {
            allowed: true,
            reason: "tagged_or_delegated".to_string(),
            private_reminder_required: false,
        };
    }

    match mode {
        ConversationMode::AgentLed | ConversationMode::ReturnedToAgent => PublicPostDecision {
            allowed: true,
            reason: "agent_led".to_string(),
            private_reminder_required: false,
        },
        ConversationMode::HumanLedActive => PublicPostDecision {
            allowed: false,
            reason: "human_led_active_requires_tag_delegation_or_policy".to_string(),
            private_reminder_required: false,
        },
        ConversationMode::HumanLedIdle => PublicPostDecision {
            allowed: false,
            reason: "human_led_idle_requires_private_reminder_first".to_string(),
            private_reminder_required: true,
        },
        ConversationMode::AssistivePrivate | ConversationMode::NeedsHandoff => PublicPostDecision {
            allowed: false,
            reason: "private_or_handoff_mode".to_string(),
            private_reminder_required: false,
        },
    }
}

pub fn default_queue_for_role(role: ConversationRole) -> QueueScope {
    match role {
        ConversationRole::Staff => QueueScope::MyHandoffs,
        ConversationRole::Manager => QueueScope::TeamQueue,
        ConversationRole::Admin | ConversationRole::Owner => QueueScope::AllConversations,
        ConversationRole::Client => QueueScope::MyHandoffs,
    }
}

pub fn can_access_queue(role: ConversationRole, scope: QueueScope) -> bool {
    match role {
        ConversationRole::Client => false,
        ConversationRole::Staff => matches!(scope, QueueScope::MyHandoffs),
        ConversationRole::Manager => {
            matches!(scope, QueueScope::MyHandoffs | QueueScope::TeamQueue)
        }
        ConversationRole::Admin | ConversationRole::Owner => true,
    }
}

pub fn conversation_queue(
    connection: &Connection,
    role: ConversationRole,
    actor_id: Option<&str>,
    scope: Option<QueueScope>,
) -> Result<Vec<ConversationQueueRow>> {
    let scope = scope.unwrap_or_else(|| default_queue_for_role(role));
    ensure!(
        can_access_queue(role, scope),
        "role cannot access requested conversation queue"
    );
    if matches!(scope, QueueScope::MyHandoffs) {
        require_text("actor_id", actor_id.unwrap_or_default())?;
    }

    let mut statement = match scope {
        QueueScope::MyHandoffs => connection.prepare(
            "SELECT c.id, h.id, h.reason, h.urgency, h.status, h.connection_id,
                    h.assigned_to_actor_id, c.last_meaningful_change, c.unread_count,
                    c.action_count, h.evidence_summary
             FROM conversation_handoffs h
             JOIN conversations c ON c.id = h.conversation_id
             WHERE h.assigned_to_actor_id = ?1
               AND h.status NOT IN ('declined', 'closed')
             ORDER BY h.updated_at DESC",
        )?,
        QueueScope::TeamQueue => connection.prepare(
            "SELECT c.id, h.id, h.reason, h.urgency, h.status, h.connection_id,
                    h.assigned_to_actor_id, c.last_meaningful_change, c.unread_count,
                    c.action_count, h.evidence_summary
             FROM conversation_handoffs h
             JOIN conversations c ON c.id = h.conversation_id
             WHERE h.status NOT IN ('declined', 'closed')
             ORDER BY h.updated_at DESC",
        )?,
        QueueScope::AllConversations => connection.prepare(
            "SELECT c.id, h.id, COALESCE(h.reason, 'Conversation requires review'), COALESCE(h.urgency, 'normal'),
                    h.status, COALESCE(h.connection_id, c.connection_id), h.assigned_to_actor_id,
                    c.last_meaningful_change, c.unread_count, c.action_count,
                    COALESCE(h.evidence_summary, c.summary_json)
             FROM conversations c
             LEFT JOIN conversation_handoffs h ON h.conversation_id = c.id AND h.status NOT IN ('declined', 'closed')
             WHERE c.archived_at IS NULL
             ORDER BY c.updated_at DESC",
        )?,
    };
    let rows = if matches!(scope, QueueScope::MyHandoffs) {
        statement.query_map([actor_id.unwrap_or_default()], queue_row_from_row)?
    } else {
        statement.query_map([], queue_row_from_row)?
    };
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(Into::into)
}

pub fn create_conversation_participant(
    connection: &Connection,
    request: &ConversationParticipantCreateRequest,
) -> Result<ConversationParticipantView> {
    require_text("conversation_id", &request.conversation_id)?;
    require_text("participant_kind", &request.participant_kind)?;
    require_text("display_name", &request.display_name)?;
    require_text("role", &request.role)?;

    let now = Utc::now().to_rfc3339();
    let participant_id = format!("participant_{}", Uuid::new_v4());
    connection.execute(
        "INSERT INTO conversation_participants (
            id, conversation_id, participant_kind, actor_id, connection_id, visitor_session_id,
            display_name, role, status, joined_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 'active', ?9)",
        params![
            participant_id,
            request.conversation_id,
            request.participant_kind,
            request.actor_id,
            request.connection_id,
            request.visitor_session_id,
            request.display_name,
            request.role,
            now
        ],
    )?;
    append_conversation_event(
        connection,
        &request.conversation_id,
        None,
        None,
        "participant.joined",
        json!({
            "participantId": participant_id,
            "participantKind": request.participant_kind,
            "role": request.role,
        }),
        None,
    )?;

    load_participant(connection, &participant_id)
}

pub fn create_conversation_message(
    connection: &Connection,
    request: &ConversationMessageCreateRequest,
) -> Result<ConversationMessageView> {
    require_text("conversation_id", &request.conversation_id)?;
    require_text("participant_id", &request.participant_id)?;
    require_text("message_kind", &request.message_kind)?;
    require_text("body_markdown", &request.body_markdown)?;
    require_text("visibility", &request.visibility)?;
    require_text("client_message_id", &request.client_message_id)?;

    if let Some(existing_id) = connection
        .query_row(
            "SELECT id FROM conversation_messages
             WHERE conversation_id = ?1 AND participant_id = ?2 AND client_message_id = ?3",
            params![
                request.conversation_id,
                request.participant_id,
                request.client_message_id
            ],
            |row| row.get::<_, String>(0),
        )
        .optional()?
    {
        return load_message(connection, &existing_id);
    }

    let sequence = next_conversation_sequence(connection, &request.conversation_id)?;
    let now = Utc::now().to_rfc3339();
    let message_id = format!("message_{}", Uuid::new_v4());
    let transaction = connection.unchecked_transaction()?;
    let realtime = append_realtime_event_tx(
        &transaction,
        &RealtimeEvent {
            cursor: None,
            schema_version: "conversation.gateway.v1".to_string(),
            family: "conversation".to_string(),
            event_type: "message.created".to_string(),
            job_id: None,
            task_key: None,
            sequence: Some(sequence),
            payload: json!({
                "conversationId": request.conversation_id,
                "messageId": message_id,
                "participantId": request.participant_id,
                "clientMessageId": request.client_message_id,
            }),
            occurred_at: now.clone(),
        },
    )?;
    transaction.execute(
        "INSERT INTO conversation_messages (
            id, conversation_id, segment_id, participant_id, message_kind, status,
            body_markdown, redaction_state, visibility, reply_to_message_id,
            client_message_id, sequence, event_cursor, undo_expires_at, created_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, 'sent', ?6, 'none', ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
        params![
            message_id,
            request.conversation_id,
            request.segment_id,
            request.participant_id,
            request.message_kind,
            request.body_markdown,
            request.visibility,
            request.reply_to_message_id,
            request.client_message_id,
            sequence,
            realtime.cursor,
            request.undo_expires_at,
            now
        ],
    )?;
    transaction.execute(
        "INSERT INTO conversation_receipts (
            id, conversation_id, message_id, participant_id, receipt_kind, event_cursor, sequence, created_at
         ) VALUES (?1, ?2, ?3, ?4, 'persisted', ?5, ?6, ?7)",
        params![
            format!("receipt_{}", Uuid::new_v4()),
            request.conversation_id,
            message_id,
            request.participant_id,
            realtime.cursor,
            sequence,
            now
        ],
    )?;
    transaction.execute(
        "INSERT INTO conversation_events (
            id, conversation_id, segment_id, sequence, event_type, payload_json, realtime_cursor, occurred_at
         ) VALUES (?1, ?2, ?3, ?4, 'message.created', ?5, ?6, ?7)",
        params![
            format!("conversation_event_{}", Uuid::new_v4()),
            request.conversation_id,
            request.segment_id,
            sequence,
            json!({
                "messageId": message_id,
                "participantId": request.participant_id,
                "clientMessageId": request.client_message_id,
            })
            .to_string(),
            realtime.cursor,
            now
        ],
    )?;
    transaction.execute(
        "UPDATE conversations
         SET last_meaningful_change = 'message.created', updated_at = ?1
         WHERE id = ?2",
        params![now, request.conversation_id],
    )?;
    transaction.commit()?;

    load_message(connection, &message_id)
}

pub fn edit_conversation_message(
    connection: &Connection,
    message_id: &str,
    edited_by_participant_id: &str,
    body_markdown: &str,
    reason: Option<&str>,
) -> Result<ConversationMessageView> {
    require_text("message_id", message_id)?;
    require_text("edited_by_participant_id", edited_by_participant_id)?;
    require_text("body_markdown", body_markdown)?;
    let current = load_message(connection, message_id)?;
    let revision_number: i64 = connection.query_row(
        "SELECT COALESCE(MAX(revision_number), 0) + 1
         FROM conversation_message_revisions
         WHERE message_id = ?1",
        [message_id],
        |row| row.get(0),
    )?;
    let now = Utc::now().to_rfc3339();
    let transaction = connection.unchecked_transaction()?;
    transaction.execute(
        "INSERT INTO conversation_message_revisions (
            id, message_id, revision_number, body_markdown, edited_by_participant_id, reason, created_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            format!("revision_{}", Uuid::new_v4()),
            message_id,
            revision_number,
            current.body_markdown,
            edited_by_participant_id,
            reason,
            now
        ],
    )?;
    transaction.execute(
        "UPDATE conversation_messages
         SET body_markdown = ?1, edited_at = ?2
         WHERE id = ?3",
        params![body_markdown, now, message_id],
    )?;
    append_conversation_event_tx(
        &transaction,
        &current.conversation_id,
        current.segment_id.as_deref(),
        None,
        "message.edited",
        json!({
            "messageId": message_id,
            "revisionNumber": revision_number,
            "editedByParticipantId": edited_by_participant_id,
        }),
        None,
    )?;
    transaction.commit()?;

    load_message(connection, message_id)
}

pub fn undo_conversation_message(
    connection: &Connection,
    message_id: &str,
    participant_id: &str,
) -> Result<ConversationMessageView> {
    undo_conversation_message_at(connection, message_id, participant_id, Utc::now())
}

pub fn undo_conversation_message_at(
    connection: &Connection,
    message_id: &str,
    participant_id: &str,
    now: DateTime<Utc>,
) -> Result<ConversationMessageView> {
    require_text("message_id", message_id)?;
    require_text("participant_id", participant_id)?;
    let current = load_message(connection, message_id)?;
    ensure!(
        current.participant_id == participant_id,
        "only the author participant can undo this message"
    );
    ensure!(
        current.deleted_at.is_none(),
        "message is already deleted or cancelled"
    );
    let Some(undo_expires_at) = current.undo_expires_at.as_deref() else {
        bail!("message does not have an undo grace window");
    };
    let undo_expires_at = DateTime::parse_from_rfc3339(undo_expires_at)?.with_timezone(&Utc);
    ensure!(now <= undo_expires_at, "message undo grace window expired");

    let now = now.to_rfc3339();
    let transaction = connection.unchecked_transaction()?;
    transaction.execute(
        "UPDATE conversation_messages
         SET status = 'cancelled', body_markdown = '', undo_cancelled_at = ?1, deleted_at = ?1
         WHERE id = ?2",
        params![now, message_id],
    )?;
    append_conversation_event_tx(
        &transaction,
        &current.conversation_id,
        current.segment_id.as_deref(),
        None,
        "message.undo.cancelled",
        json!({
            "messageId": message_id,
            "participantId": participant_id,
        }),
        None,
    )?;
    transaction.commit()?;

    load_message(connection, message_id)
}

pub fn delete_conversation_message(
    connection: &Connection,
    message_id: &str,
    participant_id: &str,
    reason: &str,
) -> Result<ConversationMessageView> {
    require_text("message_id", message_id)?;
    require_text("participant_id", participant_id)?;
    require_text("reason", reason)?;
    let current = load_message(connection, message_id)?;
    ensure!(
        current.deleted_at.is_none(),
        "message is already deleted or cancelled"
    );

    let revision_number: i64 = connection.query_row(
        "SELECT COALESCE(MAX(revision_number), 0) + 1
         FROM conversation_message_revisions
         WHERE message_id = ?1",
        [message_id],
        |row| row.get(0),
    )?;
    let now = Utc::now().to_rfc3339();
    let transaction = connection.unchecked_transaction()?;
    transaction.execute(
        "INSERT INTO conversation_message_revisions (
            id, message_id, revision_number, body_markdown, edited_by_participant_id, reason, created_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            format!("revision_{}", Uuid::new_v4()),
            message_id,
            revision_number,
            current.body_markdown,
            participant_id,
            reason,
            now
        ],
    )?;
    transaction.execute(
        "UPDATE conversation_messages
         SET status = 'tombstoned', body_markdown = '', deleted_at = ?1
         WHERE id = ?2",
        params![now, message_id],
    )?;
    append_conversation_event_tx(
        &transaction,
        &current.conversation_id,
        current.segment_id.as_deref(),
        None,
        "message.tombstoned",
        json!({
            "messageId": message_id,
            "participantId": participant_id,
            "reason": reason,
            "revisionNumber": revision_number,
        }),
        None,
    )?;
    transaction.commit()?;

    load_message(connection, message_id)
}

fn load_conversation_summary(
    connection: &Connection,
    conversation_id: &str,
) -> Result<ConversationSummary> {
    connection
        .query_row(
            "SELECT id, surface, subject_kind, subject_id, connection_id, status, unread_count,
                    action_count, last_meaningful_change, updated_at
             FROM conversations
             WHERE id = ?1",
            [conversation_id],
            conversation_summary_from_row,
        )
        .map_err(Into::into)
}

fn load_segment(connection: &Connection, segment_id: &str) -> Result<ConversationSegmentView> {
    connection
        .query_row(
            "SELECT id, conversation_id, title, segment_kind, status, candidate_state, confidence,
                    evidence_refs_json, provenance_json, created_by_job_id, source_kind, source_id,
                    created_at, updated_at
             FROM conversation_segments
             WHERE id = ?1",
            [segment_id],
            segment_from_row,
        )
        .map_err(Into::into)
}

fn load_handoff(connection: &Connection, handoff_id: &str) -> Result<ConversationHandoffView> {
    connection
        .query_row(
            "SELECT id, conversation_id, segment_id, connection_id, requested_by_actor_id,
                    assigned_to_actor_id, reason, urgency, required_capability_id,
                    evidence_summary, allowed_context_json, status, policy_decision_id,
                    receipt_json, created_at, updated_at, closed_at
             FROM conversation_handoffs
             WHERE id = ?1",
            [handoff_id],
            handoff_from_row,
        )
        .map_err(Into::into)
}

fn load_conversation_mode(
    connection: &Connection,
    conversation_id: &str,
) -> Result<ConversationModeView> {
    connection
        .query_row(
            "SELECT conversation_id, mode, led_by_actor_id, delegated_to_agent, delegation_scope_json,
                    idle_after, private_reminder_sent_at, updated_at
             FROM conversation_modes
             WHERE conversation_id = ?1",
            [conversation_id],
            mode_from_row,
        )
        .map_err(Into::into)
}

fn load_participant(
    connection: &Connection,
    participant_id: &str,
) -> Result<ConversationParticipantView> {
    connection
        .query_row(
            "SELECT id, conversation_id, participant_kind, actor_id, connection_id, visitor_session_id,
                    display_name, role, status, joined_at
             FROM conversation_participants
             WHERE id = ?1",
            [participant_id],
            participant_from_row,
        )
        .map_err(Into::into)
}

fn load_message(connection: &Connection, message_id: &str) -> Result<ConversationMessageView> {
    connection
        .query_row(
            "SELECT id, conversation_id, segment_id, participant_id, message_kind, status,
                    body_markdown, visibility, client_message_id, sequence, event_cursor,
                    undo_expires_at, undo_cancelled_at, created_at, edited_at, deleted_at
             FROM conversation_messages
             WHERE id = ?1",
            [message_id],
            message_from_row,
        )
        .map_err(Into::into)
}

fn append_conversation_event(
    connection: &Connection,
    conversation_id: &str,
    segment_id: Option<&str>,
    handoff_id: Option<&str>,
    event_type: &str,
    payload: Value,
    policy_decision_id: Option<&str>,
) -> Result<()> {
    let sequence = next_conversation_sequence(connection, conversation_id)?;
    let occurred_at = Utc::now().to_rfc3339();
    let realtime = append_realtime_event(
        connection,
        &RealtimeEvent {
            cursor: None,
            schema_version: "conversation.product.v1".to_string(),
            family: "conversation".to_string(),
            event_type: event_type.to_string(),
            job_id: None,
            task_key: None,
            sequence: Some(sequence),
            payload: payload.clone(),
            occurred_at: occurred_at.clone(),
        },
    )?;
    connection.execute(
        "INSERT INTO conversation_events (
            id, conversation_id, segment_id, handoff_id, sequence, event_type, payload_json,
            policy_decision_id, realtime_cursor, occurred_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        params![
            format!("conversation_event_{}", Uuid::new_v4()),
            conversation_id,
            segment_id,
            handoff_id,
            sequence,
            event_type,
            payload.to_string(),
            policy_decision_id,
            realtime.cursor,
            occurred_at
        ],
    )?;
    Ok(())
}

fn append_conversation_event_tx(
    transaction: &Transaction<'_>,
    conversation_id: &str,
    segment_id: Option<&str>,
    handoff_id: Option<&str>,
    event_type: &str,
    payload: Value,
    policy_decision_id: Option<&str>,
) -> Result<()> {
    let sequence = next_conversation_sequence_tx(transaction, conversation_id)?;
    let occurred_at = Utc::now().to_rfc3339();
    let realtime = append_realtime_event_tx(
        transaction,
        &RealtimeEvent {
            cursor: None,
            schema_version: "conversation.product.v1".to_string(),
            family: "conversation".to_string(),
            event_type: event_type.to_string(),
            job_id: None,
            task_key: None,
            sequence: Some(sequence),
            payload: payload.clone(),
            occurred_at: occurred_at.clone(),
        },
    )?;
    transaction.execute(
        "INSERT INTO conversation_events (
            id, conversation_id, segment_id, handoff_id, sequence, event_type, payload_json,
            policy_decision_id, realtime_cursor, occurred_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        params![
            format!("conversation_event_{}", Uuid::new_v4()),
            conversation_id,
            segment_id,
            handoff_id,
            sequence,
            event_type,
            payload.to_string(),
            policy_decision_id,
            realtime.cursor,
            occurred_at
        ],
    )?;
    Ok(())
}

fn next_conversation_sequence(connection: &Connection, conversation_id: &str) -> Result<i64> {
    let current: i64 = connection.query_row(
        "SELECT COALESCE(MAX(sequence), 0) FROM conversation_events WHERE conversation_id = ?1",
        [conversation_id],
        |row| row.get(0),
    )?;
    Ok(current + 1)
}

fn next_conversation_sequence_tx(
    transaction: &Transaction<'_>,
    conversation_id: &str,
) -> Result<i64> {
    let current: i64 = transaction.query_row(
        "SELECT COALESCE(MAX(sequence), 0) FROM conversation_events WHERE conversation_id = ?1",
        [conversation_id],
        |row| row.get(0),
    )?;
    Ok(current + 1)
}

fn conversation_summary_from_row(row: &Row<'_>) -> rusqlite::Result<ConversationSummary> {
    Ok(ConversationSummary {
        id: row.get(0)?,
        surface: row.get(1)?,
        subject_kind: row.get(2)?,
        subject_id: row.get(3)?,
        connection_id: row.get(4)?,
        status: row.get(5)?,
        unread_count: row.get(6)?,
        action_count: row.get(7)?,
        last_meaningful_change: row.get(8)?,
        updated_at: row.get(9)?,
    })
}

fn segment_from_row(row: &Row<'_>) -> rusqlite::Result<ConversationSegmentView> {
    let evidence_refs_json: String = row.get(7)?;
    let provenance_json: String = row.get(8)?;
    let candidate_state_raw: String = row.get(5)?;
    Ok(ConversationSegmentView {
        id: row.get(0)?,
        conversation_id: row.get(1)?,
        title: row.get(2)?,
        segment_kind: row.get(3)?,
        status: row.get(4)?,
        candidate_state: CandidateState::try_from(candidate_state_raw.as_str())
            .map_err(to_sql_error)?,
        confidence: row.get(6)?,
        evidence_refs: serde_json::from_str(&evidence_refs_json).unwrap_or_default(),
        provenance: serde_json::from_str(&provenance_json).unwrap_or_else(|_| json!({})),
        created_by_job_id: row.get(9)?,
        source_kind: row.get(10)?,
        source_id: row.get(11)?,
        created_at: row.get(12)?,
        updated_at: row.get(13)?,
    })
}

fn handoff_from_row(row: &Row<'_>) -> rusqlite::Result<ConversationHandoffView> {
    let allowed_context_json: String = row.get(10)?;
    let status_raw: String = row.get(11)?;
    let receipt_json: String = row.get(13)?;
    Ok(ConversationHandoffView {
        id: row.get(0)?,
        conversation_id: row.get(1)?,
        segment_id: row.get(2)?,
        connection_id: row.get(3)?,
        requested_by_actor_id: row.get(4)?,
        assigned_to_actor_id: row.get(5)?,
        reason: row.get(6)?,
        urgency: row.get(7)?,
        required_capability_id: row.get(8)?,
        evidence_summary: row.get(9)?,
        allowed_context: serde_json::from_str(&allowed_context_json).unwrap_or_default(),
        status: HandoffStatus::try_from(status_raw.as_str()).map_err(to_sql_error)?,
        policy_decision_id: row.get(12)?,
        receipt: serde_json::from_str(&receipt_json).unwrap_or_else(|_| json!({})),
        created_at: row.get(14)?,
        updated_at: row.get(15)?,
        closed_at: row.get(16)?,
    })
}

fn mode_from_row(row: &Row<'_>) -> rusqlite::Result<ConversationModeView> {
    let mode_raw: String = row.get(1)?;
    let delegation_scope_json: String = row.get(4)?;
    Ok(ConversationModeView {
        conversation_id: row.get(0)?,
        mode: ConversationMode::try_from(mode_raw.as_str()).map_err(to_sql_error)?,
        led_by_actor_id: row.get(2)?,
        delegated_to_agent: row.get::<_, i64>(3)? == 1,
        delegation_scope: serde_json::from_str(&delegation_scope_json).unwrap_or_default(),
        idle_after: row.get(5)?,
        private_reminder_sent_at: row.get(6)?,
        updated_at: row.get(7)?,
    })
}

fn queue_row_from_row(row: &Row<'_>) -> rusqlite::Result<ConversationQueueRow> {
    let status_raw: Option<String> = row.get(4)?;
    Ok(ConversationQueueRow {
        conversation_id: row.get(0)?,
        handoff_id: row.get(1)?,
        why: row.get(2)?,
        urgency: row.get(3)?,
        handoff_status: status_raw
            .as_deref()
            .map(HandoffStatus::try_from)
            .transpose()
            .map_err(to_sql_error)?,
        connection_id: row.get(5)?,
        assigned_actor_id: row.get(6)?,
        last_meaningful_change: row.get(7)?,
        unread_count: row.get(8)?,
        action_count: row.get(9)?,
        evidence_summary: row.get(10)?,
    })
}

fn authorize_participant_mutation(
    connection: &Connection,
    actor: &ConversationMutationActor,
    target: ConversationMutationPolicyTarget<'_>,
) -> Result<String> {
    let participant = load_participant(connection, target.participant_id)?;
    let allowed = participant.conversation_id == target.conversation_id
        && actor_can_act_for_participant(&actor.actor, &participant);
    let decision = PolicyDecision {
        outcome: if allowed {
            PolicyOutcome::Allowed
        } else {
            PolicyOutcome::Denied
        },
        actor: actor.actor.clone(),
        action: target.action,
        resource: ResourceRef::new(target.resource_kind, target.resource_id),
        capability_id: Some(target.capability_id.to_string()),
        reason: if allowed {
            "Conversation participant and actor context allow this mutation.".to_string()
        } else {
            "Conversation mutation requires an actor bound to the participant, local owner, or system."
                .to_string()
        },
    };
    let policy_decision_id = record_policy_decision(
        connection,
        &decision,
        PolicyDecisionCorrelation {
            request_id: actor.request_id.clone(),
            ..PolicyDecisionCorrelation::default()
        },
    )?;
    ensure!(
        allowed,
        "conversation mutation denied by policy decision {policy_decision_id}"
    );
    Ok(policy_decision_id)
}

fn actor_can_act_for_participant(
    actor: &ActorContext,
    participant: &ConversationParticipantView,
) -> bool {
    if matches!(actor.kind, ActorKind::System) && actor.id.as_deref() == Some(SYSTEM_ACTOR_ID) {
        return true;
    }
    if actor.id.as_deref() == Some(LOCAL_OWNER_ACTOR_ID) {
        return true;
    }
    participant.actor_id.as_deref().is_some()
        && participant.actor_id.as_deref() == actor.id.as_deref()
}

fn participant_from_row(row: &Row<'_>) -> rusqlite::Result<ConversationParticipantView> {
    Ok(ConversationParticipantView {
        id: row.get(0)?,
        conversation_id: row.get(1)?,
        participant_kind: row.get(2)?,
        actor_id: row.get(3)?,
        connection_id: row.get(4)?,
        visitor_session_id: row.get(5)?,
        display_name: row.get(6)?,
        role: row.get(7)?,
        status: row.get(8)?,
        joined_at: row.get(9)?,
    })
}

fn message_from_row(row: &Row<'_>) -> rusqlite::Result<ConversationMessageView> {
    Ok(ConversationMessageView {
        id: row.get(0)?,
        conversation_id: row.get(1)?,
        segment_id: row.get(2)?,
        participant_id: row.get(3)?,
        message_kind: row.get(4)?,
        status: row.get(5)?,
        body_markdown: row.get(6)?,
        visibility: row.get(7)?,
        client_message_id: row.get(8)?,
        sequence: row.get(9)?,
        event_cursor: row.get(10)?,
        undo_expires_at: row.get(11)?,
        undo_cancelled_at: row.get(12)?,
        created_at: row.get(13)?,
        edited_at: row.get(14)?,
        deleted_at: row.get(15)?,
    })
}

fn validate_handoff_request(request: &ConversationHandoffCreateRequest) -> Result<()> {
    require_text("conversation_id", &request.conversation_id)?;
    require_text("reason", &request.reason)?;
    require_text("urgency", &request.urgency)?;
    require_text("required_capability_id", &request.required_capability_id)?;
    require_text("evidence_summary", &request.evidence_summary)?;
    ensure!(
        !request.allowed_context.is_empty(),
        "handoff requires allowed context"
    );
    Ok(())
}

fn valid_handoff_transition(from: HandoffStatus, to: HandoffStatus) -> bool {
    matches!(
        (from, to),
        (HandoffStatus::Suggested, HandoffStatus::Requested)
            | (HandoffStatus::Suggested, HandoffStatus::Declined)
            | (HandoffStatus::Requested, HandoffStatus::Accepted)
            | (HandoffStatus::Requested, HandoffStatus::Declined)
            | (HandoffStatus::Requested, HandoffStatus::Assigned)
            | (HandoffStatus::Accepted, HandoffStatus::Assigned)
            | (HandoffStatus::Accepted, HandoffStatus::InProgress)
            | (HandoffStatus::Accepted, HandoffStatus::Closed)
            | (HandoffStatus::Assigned, HandoffStatus::InProgress)
            | (HandoffStatus::Assigned, HandoffStatus::ReturnedToAgent)
            | (HandoffStatus::Assigned, HandoffStatus::Closed)
            | (HandoffStatus::InProgress, HandoffStatus::ReturnedToAgent)
            | (HandoffStatus::InProgress, HandoffStatus::Closed)
            | (HandoffStatus::ReturnedToAgent, HandoffStatus::Requested)
            | (HandoffStatus::ReturnedToAgent, HandoffStatus::Closed)
    )
}

fn require_text(field_name: &str, value: &str) -> Result<()> {
    ensure!(!value.trim().is_empty(), "{field_name} is required");
    Ok(())
}

fn to_sql_error(error: anyhow::Error) -> rusqlite::Error {
    rusqlite::Error::FromSqlConversionFailure(
        0,
        rusqlite::types::Type::Text,
        Box::new(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            error.to_string(),
        )),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capabilities::seed_builtin_capabilities;
    use crate::schema::init_schema;

    fn test_connection() -> Connection {
        let connection = Connection::open_in_memory().unwrap();
        init_schema(&connection).unwrap();
        seed_builtin_capabilities(&connection).unwrap();
        connection
            .execute(
                "INSERT INTO actors (id, actor_kind, display_name, status, metadata_json, created_at, updated_at)
                 VALUES
                    ('actor_staff', 'staff', 'Staff', 'active', '{}', 'now', 'now'),
                    ('actor_client', 'client', 'Client', 'active', '{}', 'now', 'now')",
                [],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO connections (
                    id, connection_type, display_name, status, identity_json, scope_json, metadata_json, created_at, updated_at
                 ) VALUES ('connection_1', 'client', 'Client', 'active', '{}', '{}', '{}', 'now', 'now')",
                [],
            )
            .unwrap();
        connection
    }

    fn canonical_request() -> CanonicalConversationRequest {
        CanonicalConversationRequest {
            surface: "client_portal".to_string(),
            subject_kind: "connection".to_string(),
            subject_id: "connection_1".to_string(),
            connection_id: None,
            visitor_session_id: None,
            created_by_actor_id: None,
        }
    }

    fn create_conversation(connection: &Connection) -> ConversationSummary {
        find_or_create_canonical_conversation(connection, &canonical_request()).unwrap()
    }

    fn create_participant(
        connection: &Connection,
        conversation_id: &str,
    ) -> ConversationParticipantView {
        create_conversation_participant(
            connection,
            &ConversationParticipantCreateRequest {
                conversation_id: conversation_id.to_string(),
                participant_kind: "staff".to_string(),
                actor_id: Some("actor_staff".to_string()),
                connection_id: None,
                visitor_session_id: None,
                display_name: "Staff".to_string(),
                role: "staff".to_string(),
            },
        )
        .unwrap()
    }

    fn staff_mutation_actor() -> ConversationMutationActor {
        ConversationMutationActor {
            actor: ActorContext::new(
                ActorKind::BrowserOperator,
                "test",
                Some("actor_staff".to_string()),
            ),
            request_id: Some("request_1".to_string()),
        }
    }

    #[test]
    fn canonical_conversation_keeps_client_visible_relationship_unfragmented() {
        let connection = test_connection();
        let first = create_conversation(&connection);
        let second =
            find_or_create_canonical_conversation(&connection, &canonical_request()).unwrap();

        assert_eq!(first.id, second.id);
        let summaries =
            client_conversation_summaries(&connection, "connection", "connection_1").unwrap();
        assert_eq!(summaries.len(), 1);
        assert_eq!(summaries[0].id, first.id);
    }

    #[test]
    fn episode_candidates_require_evidence_and_are_idempotent_by_source_job() {
        let connection = test_connection();
        let conversation = create_conversation(&connection);
        let request = EpisodeCandidateRequest {
            conversation_id: conversation.id.clone(),
            title: "Pricing follow-up".to_string(),
            segment_kind: "episode".to_string(),
            evidence_refs: vec!["message_1".to_string()],
            confidence: 0.82,
            provenance: json!({ "jobId": "job_external" }),
            created_by_job_id: None,
            source_kind: Some("message_window".to_string()),
            source_id: Some("window_1".to_string()),
        };

        let first = add_episode_candidate(&connection, &request).unwrap();
        let second = add_episode_candidate(&connection, &request).unwrap();

        assert_eq!(first.id, second.id);
        assert_eq!(first.candidate_state, CandidateState::Proposed);
        assert_eq!(first.evidence_refs, vec!["message_1"]);
        assert_eq!(
            staff_episode_details(&connection, &conversation.id)
                .unwrap()
                .len(),
            1
        );
    }

    #[test]
    fn episode_candidate_rejects_missing_evidence_or_provenance() {
        let connection = test_connection();
        let conversation = create_conversation(&connection);
        let err = add_episode_candidate(
            &connection,
            &EpisodeCandidateRequest {
                conversation_id: conversation.id,
                title: "No evidence".to_string(),
                segment_kind: "episode".to_string(),
                evidence_refs: vec![],
                confidence: 0.5,
                provenance: json!({ "jobId": "job_external" }),
                created_by_job_id: None,
                source_kind: None,
                source_id: None,
            },
        )
        .unwrap_err();

        assert!(err.to_string().contains("evidence refs"));
    }

    #[test]
    fn handoff_requires_governed_fields_and_returns_brief_before_transcript() {
        let connection = test_connection();
        let conversation = create_conversation(&connection);
        let handoff = create_conversation_handoff(
            &connection,
            &ConversationHandoffCreateRequest {
                conversation_id: conversation.id.clone(),
                segment_id: None,
                connection_id: None,
                requested_by_actor_id: Some("actor_client".to_string()),
                assigned_to_actor_id: Some("actor_staff".to_string()),
                reason: "Client asked for contract terms".to_string(),
                urgency: "high".to_string(),
                required_capability_id: "conversation.handoff.manage".to_string(),
                evidence_summary: "Client asked whether the terms can be revised.".to_string(),
                allowed_context: vec![
                    "conversation_summary".to_string(),
                    "offer_terms".to_string(),
                ],
                policy_decision_id: None,
            },
        )
        .unwrap();

        assert_eq!(handoff.status, HandoffStatus::Requested);
        assert_eq!(
            handoff.allowed_context,
            vec!["conversation_summary", "offer_terms"]
        );
        let brief = handoff_brief(&connection, &handoff.id).unwrap();
        assert_eq!(brief.handoff_id, handoff.id);
        assert_eq!(
            brief.evidence_summary,
            "Client asked whether the terms can be revised."
        );
        assert!(!brief.allowed_context.is_empty());
    }

    #[test]
    fn handoff_lifecycle_transitions_are_durable() {
        let connection = test_connection();
        let conversation = create_conversation(&connection);
        let handoff = create_conversation_handoff(
            &connection,
            &ConversationHandoffCreateRequest {
                conversation_id: conversation.id,
                segment_id: None,
                connection_id: None,
                requested_by_actor_id: None,
                assigned_to_actor_id: Some("actor_staff".to_string()),
                reason: "Needs human quote".to_string(),
                urgency: "normal".to_string(),
                required_capability_id: "conversation.handoff.manage".to_string(),
                evidence_summary: "The client asked for a custom quote.".to_string(),
                allowed_context: vec!["quote_request".to_string()],
                policy_decision_id: None,
            },
        )
        .unwrap();

        let accepted = transition_conversation_handoff(
            &connection,
            &handoff.id,
            HandoffStatus::Accepted,
            Some("actor_staff"),
            "Taking ownership",
        )
        .unwrap();
        let closed = transition_conversation_handoff(
            &connection,
            &handoff.id,
            HandoffStatus::Closed,
            Some("actor_staff"),
            "Resolved",
        )
        .unwrap();

        assert_eq!(accepted.status, HandoffStatus::Accepted);
        assert_eq!(closed.status, HandoffStatus::Closed);
        assert!(closed.closed_at.is_some());
    }

    #[test]
    fn human_led_active_blocks_public_agent_post_without_delegation() {
        let blocked = may_agent_post_publicly(
            ConversationMode::HumanLedActive,
            &PublicPostContext::default(),
        );
        let delegated = may_agent_post_publicly(
            ConversationMode::HumanLedActive,
            &PublicPostContext {
                delegated: true,
                ..Default::default()
            },
        );
        let tagged = may_agent_post_publicly(
            ConversationMode::HumanLedActive,
            &PublicPostContext {
                tagged: true,
                ..Default::default()
            },
        );

        assert!(!blocked.allowed);
        assert_eq!(
            blocked.reason,
            "human_led_active_requires_tag_delegation_or_policy"
        );
        assert!(delegated.allowed);
        assert!(tagged.allowed);
    }

    #[test]
    fn idle_recovery_records_private_reminder_before_public_return() {
        let connection = test_connection();
        let conversation = create_conversation(&connection);
        record_staff_activity_sets_human_led(&connection, &conversation.id, "actor_staff").unwrap();
        let idle = mark_human_led_idle_private_reminder(&connection, &conversation.id).unwrap();
        let decision = may_agent_post_publicly(
            idle.mode,
            &PublicPostContext {
                delegated: false,
                tagged: false,
                policy_required: false,
            },
        );

        assert_eq!(idle.mode, ConversationMode::HumanLedIdle);
        assert!(idle.private_reminder_sent_at.is_some());
        assert!(!decision.allowed);
        assert!(decision.private_reminder_required);
    }

    #[test]
    fn conversation_queues_are_role_scoped() {
        let connection = test_connection();
        let conversation = create_conversation(&connection);
        let handoff = create_conversation_handoff(
            &connection,
            &ConversationHandoffCreateRequest {
                conversation_id: conversation.id.clone(),
                segment_id: None,
                connection_id: Some("connection_1".to_string()),
                requested_by_actor_id: None,
                assigned_to_actor_id: Some("actor_staff".to_string()),
                reason: "Needs owner reply".to_string(),
                urgency: "high".to_string(),
                required_capability_id: "conversation.handoff.manage".to_string(),
                evidence_summary: "Client requested owner confirmation.".to_string(),
                allowed_context: vec!["conversation_summary".to_string()],
                policy_decision_id: None,
            },
        )
        .unwrap();

        let staff_rows = conversation_queue(
            &connection,
            ConversationRole::Staff,
            Some("actor_staff"),
            None,
        )
        .unwrap();
        let team_rows = conversation_queue(
            &connection,
            ConversationRole::Manager,
            None,
            Some(QueueScope::TeamQueue),
        )
        .unwrap();
        let all_rows = conversation_queue(
            &connection,
            ConversationRole::Admin,
            None,
            Some(QueueScope::AllConversations),
        )
        .unwrap();
        let denied = conversation_queue(
            &connection,
            ConversationRole::Staff,
            Some("actor_staff"),
            Some(QueueScope::AllConversations),
        );

        assert_eq!(staff_rows.len(), 1);
        assert_eq!(staff_rows[0].handoff_id, Some(handoff.id.clone()));
        assert_eq!(staff_rows[0].urgency, "high");
        assert_eq!(
            staff_rows[0].evidence_summary,
            "Client requested owner confirmation."
        );
        assert_eq!(team_rows.len(), 1);
        assert_eq!(all_rows.len(), 1);
        assert!(denied.is_err());
    }

    #[test]
    fn message_create_is_durable_sequenced_and_idempotent_by_client_message_id() {
        let connection = test_connection();
        let conversation = create_conversation(&connection);
        let participant = create_participant(&connection, &conversation.id);
        let request = ConversationMessageCreateRequest {
            conversation_id: conversation.id.clone(),
            segment_id: None,
            participant_id: participant.id.clone(),
            message_kind: "human".to_string(),
            body_markdown: "First durable message".to_string(),
            visibility: "participants".to_string(),
            client_message_id: "client_msg_1".to_string(),
            reply_to_message_id: None,
            undo_expires_at: Some("2099-05-09T00:00:30Z".to_string()),
        };

        let first = create_conversation_message(&connection, &request).unwrap();
        let second = create_conversation_message(&connection, &request).unwrap();

        assert_eq!(first.id, second.id);
        assert_eq!(first.sequence, 4);
        assert!(first.event_cursor.is_some());
        assert_eq!(
            first.undo_expires_at.as_deref(),
            Some("2099-05-09T00:00:30Z")
        );

        let receipt_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM conversation_receipts WHERE message_id = ?1 AND receipt_kind = 'persisted'",
                [&first.id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(receipt_count, 1);
    }

    #[test]
    fn message_edit_preserves_revision_history() {
        let connection = test_connection();
        let conversation = create_conversation(&connection);
        let participant = create_participant(&connection, &conversation.id);
        let message = create_conversation_message(
            &connection,
            &ConversationMessageCreateRequest {
                conversation_id: conversation.id.clone(),
                segment_id: None,
                participant_id: participant.id.clone(),
                message_kind: "human".to_string(),
                body_markdown: "Original".to_string(),
                visibility: "participants".to_string(),
                client_message_id: "client_msg_edit".to_string(),
                reply_to_message_id: None,
                undo_expires_at: None,
            },
        )
        .unwrap();

        let edited = edit_conversation_message(
            &connection,
            &message.id,
            &participant.id,
            "Edited",
            Some("clarity"),
        )
        .unwrap();

        assert_eq!(edited.body_markdown, "Edited");
        assert!(edited.edited_at.is_some());
        let original_revision: String = connection
            .query_row(
                "SELECT body_markdown FROM conversation_message_revisions WHERE message_id = ?1 AND revision_number = 1",
                [&message.id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(original_revision, "Original");
    }

    #[test]
    fn message_undo_records_cancellation_without_losing_event_history() {
        let connection = test_connection();
        let conversation = create_conversation(&connection);
        let participant = create_participant(&connection, &conversation.id);
        let message = create_conversation_message(
            &connection,
            &ConversationMessageCreateRequest {
                conversation_id: conversation.id.clone(),
                segment_id: None,
                participant_id: participant.id.clone(),
                message_kind: "human".to_string(),
                body_markdown: "Undo me".to_string(),
                visibility: "participants".to_string(),
                client_message_id: "client_msg_undo".to_string(),
                reply_to_message_id: None,
                undo_expires_at: Some("2099-05-09T00:00:30Z".to_string()),
            },
        )
        .unwrap();

        let cancelled =
            undo_conversation_message(&connection, &message.id, &participant.id).unwrap();

        assert_eq!(cancelled.status, "cancelled");
        assert_eq!(cancelled.body_markdown, "");
        assert!(cancelled.undo_cancelled_at.is_some());
        assert!(cancelled.deleted_at.is_some());
        let event_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM conversation_events WHERE conversation_id = ?1",
                [&conversation.id],
                |row| row.get(0),
            )
            .unwrap();
        assert!(event_count >= 4);
    }

    #[test]
    fn service_submit_records_policy_and_preserves_message_event_atomicity() {
        let connection = test_connection();
        let conversation = create_conversation(&connection);
        let participant = create_participant(&connection, &conversation.id);

        let result = ConversationService::submit_message(
            &connection,
            &staff_mutation_actor(),
            &ConversationMessageCreateRequest {
                conversation_id: conversation.id.clone(),
                segment_id: None,
                participant_id: participant.id,
                message_kind: "human".to_string(),
                body_markdown: "Service message".to_string(),
                visibility: "participants".to_string(),
                client_message_id: "client_msg_service".to_string(),
                reply_to_message_id: None,
                undo_expires_at: Some("2099-05-09T00:00:30Z".to_string()),
            },
        )
        .unwrap();

        assert!(result.policy_decision_id.starts_with("policy_decision_"));
        let message_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM conversation_messages WHERE id = ?1",
                [&result.value.id],
                |row| row.get(0),
            )
            .unwrap();
        let event_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM conversation_events WHERE event_type = 'message.created' AND payload_json LIKE ?1",
                [format!("%{}%", result.value.id)],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(message_count, 1);
        assert_eq!(event_count, 1);
    }

    #[test]
    fn service_denial_records_policy_without_mutating_message() {
        let connection = test_connection();
        let conversation = create_conversation(&connection);
        let participant = create_participant(&connection, &conversation.id);
        let denied_actor = ConversationMutationActor {
            actor: ActorContext::new(
                ActorKind::BrowserOperator,
                "test",
                Some("actor_client".to_string()),
            ),
            request_id: Some("request_denied".to_string()),
        };

        let denied = ConversationService::submit_message(
            &connection,
            &denied_actor,
            &ConversationMessageCreateRequest {
                conversation_id: conversation.id,
                segment_id: None,
                participant_id: participant.id,
                message_kind: "human".to_string(),
                body_markdown: "Should not persist".to_string(),
                visibility: "participants".to_string(),
                client_message_id: "client_msg_denied".to_string(),
                reply_to_message_id: None,
                undo_expires_at: None,
            },
        );

        assert!(denied.is_err());
        let message_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM conversation_messages WHERE client_message_id = 'client_msg_denied'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let denied_decisions: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM policy_decisions WHERE outcome = 'denied' AND request_id = 'request_denied'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(message_count, 0);
        assert_eq!(denied_decisions, 1);
    }

    #[test]
    fn message_delete_tombstones_and_preserves_prior_body_in_revision() {
        let connection = test_connection();
        let conversation = create_conversation(&connection);
        let participant = create_participant(&connection, &conversation.id);
        let message = create_conversation_message(
            &connection,
            &ConversationMessageCreateRequest {
                conversation_id: conversation.id,
                segment_id: None,
                participant_id: participant.id.clone(),
                message_kind: "human".to_string(),
                body_markdown: "Remove me".to_string(),
                visibility: "participants".to_string(),
                client_message_id: "client_msg_delete".to_string(),
                reply_to_message_id: None,
                undo_expires_at: None,
            },
        )
        .unwrap();

        let deleted = ConversationService::delete_message(
            &connection,
            &staff_mutation_actor(),
            &message.id,
            &participant.id,
            "moderation",
        )
        .unwrap();

        assert_eq!(deleted.value.status, "tombstoned");
        assert_eq!(deleted.value.body_markdown, "");
        let revision_body: String = connection
            .query_row(
                "SELECT body_markdown FROM conversation_message_revisions WHERE message_id = ?1",
                [&message.id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(revision_body, "Remove me");
    }

    #[test]
    fn undo_outside_grace_window_fails_with_structured_reason() {
        let connection = test_connection();
        let conversation = create_conversation(&connection);
        let participant = create_participant(&connection, &conversation.id);
        let message = create_conversation_message(
            &connection,
            &ConversationMessageCreateRequest {
                conversation_id: conversation.id,
                segment_id: None,
                participant_id: participant.id.clone(),
                message_kind: "human".to_string(),
                body_markdown: "Too late".to_string(),
                visibility: "participants".to_string(),
                client_message_id: "client_msg_undo_expired".to_string(),
                reply_to_message_id: None,
                undo_expires_at: Some("2026-05-09T00:00:30Z".to_string()),
            },
        )
        .unwrap();

        let expired = undo_conversation_message_at(
            &connection,
            &message.id,
            &participant.id,
            DateTime::parse_from_rfc3339("2026-05-09T00:00:31Z")
                .unwrap()
                .with_timezone(&Utc),
        );

        assert!(expired
            .unwrap_err()
            .to_string()
            .contains("undo grace window expired"));
    }
}
