use anyhow::{ensure, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::conversations::CanonicalConversationRequest;

pub const CONVERSATION_GATEWAY_SCHEMA_VERSION: &str = "conversation.gateway.v1";
pub const CONVERSATION_GATEWAY_ROUTE: &str = "/chat/ws";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConversationGatewayOp {
    Hello,
    Identify,
    Subscribe,
    Unsubscribe,
    Command,
    Dispatch,
    Ack,
    Heartbeat,
    Resume,
    Replay,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConversationGatewayDurability {
    Durable,
    Ephemeral,
    ReadModel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConversationGatewayScope {
    Connection,
    User,
    Conversation,
    System,
    Run,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConversationCommandType {
    ConversationSubscribe,
    ConversationReplayAfterCursor,
    MessageSubmit,
    MessageEdit,
    MessageDelete,
    MessageUndo,
    MessageReact,
    MessageMarkRead,
    MessageMarkUnread,
    TypingStart,
    TypingStop,
    PresenceUpdate,
    HandoffAccept,
    HandoffDecline,
    HandoffAssign,
    HandoffReturnToAgent,
    AgentDelegate,
    AgentTakeover,
    LlmRunRequest,
    LlmRunCancel,
    LlmToolApprove,
    LlmToolReject,
    LlmToolExecute,
}

impl ConversationCommandType {
    pub fn as_wire_type(&self) -> &'static str {
        match self {
            ConversationCommandType::ConversationSubscribe => "conversation.subscribe",
            ConversationCommandType::ConversationReplayAfterCursor => {
                "conversation.replay_after_cursor"
            }
            ConversationCommandType::MessageSubmit => "message.submit",
            ConversationCommandType::MessageEdit => "message.edit",
            ConversationCommandType::MessageDelete => "message.delete",
            ConversationCommandType::MessageUndo => "message.undo",
            ConversationCommandType::MessageReact => "message.react",
            ConversationCommandType::MessageMarkRead => "message.mark_read",
            ConversationCommandType::MessageMarkUnread => "message.mark_unread",
            ConversationCommandType::TypingStart => "typing.start",
            ConversationCommandType::TypingStop => "typing.stop",
            ConversationCommandType::PresenceUpdate => "presence.update",
            ConversationCommandType::HandoffAccept => "handoff.accept",
            ConversationCommandType::HandoffDecline => "handoff.decline",
            ConversationCommandType::HandoffAssign => "handoff.assign",
            ConversationCommandType::HandoffReturnToAgent => "handoff.return_to_agent",
            ConversationCommandType::AgentDelegate => "agent.delegate",
            ConversationCommandType::AgentTakeover => "agent.takeover",
            ConversationCommandType::LlmRunRequest => "llm.run.request",
            ConversationCommandType::LlmRunCancel => "llm.run.cancel",
            ConversationCommandType::LlmToolApprove => "tool.approve",
            ConversationCommandType::LlmToolReject => "tool.reject",
            ConversationCommandType::LlmToolExecute => "tool.execute",
        }
    }

    pub fn required_capability_id(&self) -> &'static str {
        match self {
            ConversationCommandType::ConversationSubscribe
            | ConversationCommandType::ConversationReplayAfterCursor => "conversation.read",
            ConversationCommandType::MessageSubmit => "conversation.message.create",
            ConversationCommandType::MessageEdit => "conversation.message.edit",
            ConversationCommandType::MessageDelete | ConversationCommandType::MessageUndo => {
                "conversation.message.delete"
            }
            ConversationCommandType::MessageReact => "conversation.reaction.write",
            ConversationCommandType::MessageMarkRead
            | ConversationCommandType::MessageMarkUnread => "conversation.receipt.write",
            ConversationCommandType::TypingStart
            | ConversationCommandType::TypingStop
            | ConversationCommandType::PresenceUpdate => "conversation.presence.write",
            ConversationCommandType::HandoffAccept
            | ConversationCommandType::HandoffDecline
            | ConversationCommandType::HandoffAssign
            | ConversationCommandType::HandoffReturnToAgent => "conversation.handoff.manage",
            ConversationCommandType::AgentDelegate | ConversationCommandType::AgentTakeover => {
                "conversation.agent.delegate"
            }
            ConversationCommandType::LlmRunRequest => "llm.invoke",
            ConversationCommandType::LlmRunCancel => "llm.cancel",
            ConversationCommandType::LlmToolApprove => "llm.tool.approve",
            ConversationCommandType::LlmToolReject => "llm.tool.reject",
            ConversationCommandType::LlmToolExecute => "llm.tool.execute",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConversationGatewayEnvelope {
    pub schema_version: String,
    pub op: ConversationGatewayOp,
    #[serde(rename = "type")]
    pub frame_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub server_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conversation_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub segment_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sequence: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor: Option<i64>,
    pub durability: ConversationGatewayDurability,
    pub scope: ConversationGatewayScope,
    pub payload: Value,
    pub occurred_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConversationGatewayErrorPayload {
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub policy_decision_id: Option<String>,
    pub retryable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConversationAuthorizationContext {
    pub actor_id: String,
    pub participant_id: Option<String>,
    pub conversation_id: Option<String>,
    pub capability_id: String,
    pub resource_kind: String,
    pub resource_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConversationSubscriptionIntent {
    pub conversation: CanonicalConversationRequest,
    pub after_sequence: Option<i64>,
    pub after_cursor: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConversationReplayCursor {
    pub conversation_id: String,
    pub after_sequence: i64,
    pub after_cursor: Option<i64>,
    pub limit: usize,
}

impl ConversationReplayCursor {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            !self.conversation_id.trim().is_empty(),
            "conversation_id is required"
        );
        ensure!(
            self.after_sequence >= 0,
            "after_sequence cannot be negative"
        );
        ensure!(
            (1..=500).contains(&self.limit),
            "replay limit must be 1..=500"
        );
        Ok(())
    }
}

pub trait ConversationDurableProjection {
    fn replay_after(
        &self,
        cursor: &ConversationReplayCursor,
    ) -> Result<Vec<ConversationGatewayEnvelope>>;
}

pub fn command_envelope(
    command: ConversationCommandType,
    client_id: &str,
    conversation_id: Option<&str>,
    payload: Value,
    occurred_at: &str,
) -> ConversationGatewayEnvelope {
    ConversationGatewayEnvelope {
        schema_version: CONVERSATION_GATEWAY_SCHEMA_VERSION.to_string(),
        op: ConversationGatewayOp::Command,
        frame_type: command.as_wire_type().to_string(),
        client_id: Some(client_id.to_string()),
        server_id: None,
        conversation_id: conversation_id.map(ToString::to_string),
        segment_id: None,
        sequence: None,
        cursor: None,
        durability: ConversationGatewayDurability::Durable,
        scope: ConversationGatewayScope::Conversation,
        payload,
        occurred_at: occurred_at.to_string(),
    }
}

pub fn dispatch_envelope(
    event_type: &str,
    conversation_id: &str,
    sequence: i64,
    cursor: Option<i64>,
    payload: Value,
    occurred_at: &str,
) -> ConversationGatewayEnvelope {
    ConversationGatewayEnvelope {
        schema_version: CONVERSATION_GATEWAY_SCHEMA_VERSION.to_string(),
        op: ConversationGatewayOp::Dispatch,
        frame_type: event_type.to_string(),
        client_id: None,
        server_id: Some(format!("{conversation_id}:{sequence}")),
        conversation_id: Some(conversation_id.to_string()),
        segment_id: None,
        sequence: Some(sequence),
        cursor,
        durability: ConversationGatewayDurability::Durable,
        scope: ConversationGatewayScope::Conversation,
        payload,
        occurred_at: occurred_at.to_string(),
    }
}

pub fn policy_denied_error(
    client_id: &str,
    policy_decision_id: Option<&str>,
    message: &str,
    occurred_at: &str,
) -> ConversationGatewayEnvelope {
    ConversationGatewayEnvelope {
        schema_version: CONVERSATION_GATEWAY_SCHEMA_VERSION.to_string(),
        op: ConversationGatewayOp::Error,
        frame_type: "command.rejected".to_string(),
        client_id: Some(client_id.to_string()),
        server_id: None,
        conversation_id: None,
        segment_id: None,
        sequence: None,
        cursor: None,
        durability: ConversationGatewayDurability::Ephemeral,
        scope: ConversationGatewayScope::User,
        payload: json!(ConversationGatewayErrorPayload {
            code: "policy_denied".to_string(),
            message: message.to_string(),
            policy_decision_id: policy_decision_id.map(ToString::to_string),
            retryable: false,
        }),
        occurred_at: occurred_at.to_string(),
    }
}

pub fn command_rejected_error(
    client_id: Option<&str>,
    conversation_id: Option<&str>,
    code: &str,
    message: &str,
    retryable: bool,
    occurred_at: &str,
) -> ConversationGatewayEnvelope {
    ConversationGatewayEnvelope {
        schema_version: CONVERSATION_GATEWAY_SCHEMA_VERSION.to_string(),
        op: ConversationGatewayOp::Error,
        frame_type: "command.rejected".to_string(),
        client_id: client_id.map(ToString::to_string),
        server_id: None,
        conversation_id: conversation_id.map(ToString::to_string),
        segment_id: None,
        sequence: None,
        cursor: None,
        durability: ConversationGatewayDurability::Ephemeral,
        scope: ConversationGatewayScope::User,
        payload: json!(ConversationGatewayErrorPayload {
            code: code.to_string(),
            message: message.to_string(),
            policy_decision_id: None,
            retryable,
        }),
        occurred_at: occurred_at.to_string(),
    }
}

pub fn ack_envelope(
    client_id: &str,
    conversation_id: Option<&str>,
    ack_type: &str,
    payload: Value,
    occurred_at: &str,
) -> ConversationGatewayEnvelope {
    ConversationGatewayEnvelope {
        schema_version: CONVERSATION_GATEWAY_SCHEMA_VERSION.to_string(),
        op: ConversationGatewayOp::Ack,
        frame_type: ack_type.to_string(),
        client_id: Some(client_id.to_string()),
        server_id: None,
        conversation_id: conversation_id.map(ToString::to_string),
        segment_id: None,
        sequence: None,
        cursor: None,
        durability: ConversationGatewayDurability::Ephemeral,
        scope: ConversationGatewayScope::User,
        payload,
        occurred_at: occurred_at.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: &str = "2026-05-09T00:00:00Z";

    #[test]
    fn command_serialization_preserves_wire_contract_and_capability() {
        let envelope = command_envelope(
            ConversationCommandType::MessageSubmit,
            "client_1",
            Some("conversation_1"),
            json!({ "bodyMarkdown": "hello" }),
            NOW,
        );
        let serialized = serde_json::to_value(&envelope).unwrap();

        assert_eq!(
            serialized["schemaVersion"],
            CONVERSATION_GATEWAY_SCHEMA_VERSION
        );
        assert_eq!(serialized["op"], "command");
        assert_eq!(serialized["type"], "message.submit");
        assert_eq!(serialized["clientId"], "client_1");
        assert_eq!(serialized["conversationId"], "conversation_1");
        assert_eq!(
            ConversationCommandType::MessageSubmit.required_capability_id(),
            "conversation.message.create"
        );
    }

    #[test]
    fn replay_cursor_is_bounded_and_non_negative() {
        assert!(ConversationReplayCursor {
            conversation_id: "conversation_1".to_string(),
            after_sequence: 4,
            after_cursor: Some(10),
            limit: 100,
        }
        .validate()
        .is_ok());
        assert!(ConversationReplayCursor {
            conversation_id: "conversation_1".to_string(),
            after_sequence: -1,
            after_cursor: None,
            limit: 100,
        }
        .validate()
        .is_err());
        assert!(ConversationReplayCursor {
            conversation_id: "conversation_1".to_string(),
            after_sequence: 0,
            after_cursor: None,
            limit: 501,
        }
        .validate()
        .is_err());
    }

    #[test]
    fn dispatch_envelope_has_stable_server_id_sequence_and_cursor() {
        let envelope = dispatch_envelope(
            "message.created",
            "conversation_1",
            42,
            Some(1884),
            json!({ "messageId": "message_1" }),
            NOW,
        );
        let serialized = serde_json::to_value(&envelope).unwrap();

        assert_eq!(serialized["op"], "dispatch");
        assert_eq!(serialized["type"], "message.created");
        assert_eq!(serialized["serverId"], "conversation_1:42");
        assert_eq!(serialized["sequence"], 42);
        assert_eq!(serialized["cursor"], 1884);
        assert_eq!(serialized["durability"], "durable");
    }

    #[test]
    fn policy_denial_error_is_correlated_and_inspectable() {
        let envelope = policy_denied_error(
            "client_1",
            Some("policy_decision_1"),
            "Message submission is not allowed for this conversation.",
            NOW,
        );
        let serialized = serde_json::to_value(&envelope).unwrap();

        assert_eq!(serialized["op"], "error");
        assert_eq!(serialized["type"], "command.rejected");
        assert_eq!(serialized["clientId"], "client_1");
        assert_eq!(serialized["payload"]["code"], "policy_denied");
        assert_eq!(
            serialized["payload"]["policyDecisionId"],
            "policy_decision_1"
        );
        assert_eq!(serialized["payload"]["retryable"], false);
    }
}
