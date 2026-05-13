use std::collections::{BTreeSet, BTreeMap, VecDeque};
use chrono::{DateTime, Utc};
use crate::conversation_protocol::ConversationGatewayEnvelope;

/// Represents the state and subscriptions of an active WebSocket session
/// connected to the conversation gateway.
#[derive(Debug, Clone)]
pub struct ConversationGatewaySession {
    pub session_id: String,
    pub actor_id: Option<String>,
    pub participant_id: Option<String>,
    pub subscriptions: BTreeSet<String>,
    pub(crate) typing_by_conversation: BTreeMap<String, BTreeSet<String>>,
    pub(crate) recent_message_commands: VecDeque<DateTime<Utc>>,
}

impl ConversationGatewaySession {
    /// Instantiates a new session state with the provided ID.
    pub fn new(session_id: impl Into<String>) -> Self {
        Self {
            session_id: session_id.into(),
            actor_id: None,
            participant_id: None,
            subscriptions: BTreeSet::new(),
            typing_by_conversation: BTreeMap::new(),
            recent_message_commands: VecDeque::new(),
        }
    }
}

/// Encapsulates the raw message frames and broadcast envelopes
/// resulting from a gateway command execution.
#[derive(Debug, Clone)]
pub struct ConversationGatewayOutput {
    pub frames: Vec<ConversationGatewayEnvelope>,
    pub broadcast: Vec<ConversationGatewayEnvelope>,
}
