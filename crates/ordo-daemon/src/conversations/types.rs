use super::*;
use anyhow::bail;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CandidateState {
    Proposed,
    Confirmed,
    Rejected,
    Superseded,
}

impl CandidateState {
    pub(crate) fn as_str(self) -> &'static str {
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
    pub(crate) fn as_str(self) -> &'static str {
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
    pub(crate) fn as_str(self) -> &'static str {
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

    pub(crate) fn is_terminal(self) -> bool {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReactionAction {
    Add,
    Remove,
    Toggle,
}

impl TryFrom<&str> for ReactionAction {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> Result<Self> {
        match value {
            "add" => Ok(Self::Add),
            "remove" => Ok(Self::Remove),
            "toggle" => Ok(Self::Toggle),
            other => bail!("Unsupported reaction action: {other}"),
        }
    }
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConversationReadStateView {
    pub conversation_id: String,
    pub participant_id: String,
    pub last_read_message_id: Option<String>,
    pub last_read_event_cursor: Option<i64>,
    pub last_read_at: Option<String>,
    pub manual_unread_from_message_id: Option<String>,
    pub unread_count: i64,
    pub unread_mentions_count: i64,
    pub unread_action_count: i64,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConversationReactionView {
    pub id: String,
    pub message_id: String,
    pub participant_id: String,
    pub reaction_key: String,
    pub reaction_kind: String,
    pub metadata: Value,
    pub created_at: String,
    pub removed_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConversationPresenceSnapshotView {
    pub participant_id: String,
    pub conversation_id: String,
    pub status: String,
    pub visibility: String,
    pub status_message: Option<String>,
    pub device_class: Option<String>,
    pub metadata: Value,
    pub updated_at: String,
    pub expires_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConversationPresenceUpdateRequest {
    pub conversation_id: String,
    pub participant_id: String,
    pub status: String,
    pub visibility: String,
    pub status_message: Option<String>,
    pub device_class: Option<String>,
    pub expires_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConversationListReadModel {
    pub conversation: ConversationSummary,
    pub participant_id: String,
    pub last_message: Option<ConversationMessageView>,
    pub read_state: ConversationReadStateView,
    pub presence: Vec<ConversationPresenceSnapshotView>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConversationMutationOutcome<T> {
    pub value: T,
    pub event_type: Option<String>,
    pub changed: bool,
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
