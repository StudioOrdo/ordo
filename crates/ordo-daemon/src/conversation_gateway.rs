use anyhow::{ensure, Result};
use axum::extract::ws::{Message, WebSocket};
use chrono::{DateTime, Duration as ChronoDuration, Utc};
use rusqlite::{params, Connection, OptionalExtension};
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::broadcast;
use uuid::Uuid;

use crate::conversation_protocol::{
    ack_envelope, command_rejected_error, dispatch_envelope, ConversationGatewayDurability,
    ConversationGatewayEnvelope, ConversationGatewayOp, ConversationGatewayScope,
    CONVERSATION_GATEWAY_SCHEMA_VERSION,
};
use crate::conversations::{
    create_conversation_handoff, transition_conversation_handoff, upsert_conversation_mode,
    ConversationHandoffCreateRequest, ConversationMessageCreateRequest, ConversationMode,
    ConversationMutationActor, ConversationPresenceUpdateRequest, ConversationService,
    HandoffStatus, ReactionAction,
};
use crate::policy::{ActorContext, ActorKind};

const DEFAULT_REPLAY_LIMIT: usize = 100;
const MAX_REPLAY_LIMIT: usize = 500;
const MAX_TEXT_FRAME_BYTES: usize = 64 * 1024;
const COMMAND_RATE_LIMIT: usize = 30;
const COMMAND_RATE_WINDOW_SECONDS: i64 = 60;

#[derive(Debug, Clone)]
pub struct ConversationGatewaySession {
    pub session_id: String,
    pub actor_id: Option<String>,
    pub participant_id: Option<String>,
    pub subscriptions: BTreeSet<String>,
    typing_by_conversation: BTreeMap<String, BTreeSet<String>>,
    recent_message_commands: VecDeque<DateTime<Utc>>,
}

impl ConversationGatewaySession {
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

#[derive(Debug, Clone)]
pub struct ConversationGatewayOutput {
    pub frames: Vec<ConversationGatewayEnvelope>,
    pub broadcast: Vec<ConversationGatewayEnvelope>,
}

pub fn hello_frame(session_id: &str) -> ConversationGatewayEnvelope {
    ConversationGatewayEnvelope {
        schema_version: CONVERSATION_GATEWAY_SCHEMA_VERSION.to_string(),
        op: ConversationGatewayOp::Hello,
        frame_type: "gateway.hello".to_string(),
        client_id: None,
        server_id: Some(session_id.to_string()),
        conversation_id: None,
        segment_id: None,
        sequence: None,
        cursor: None,
        durability: ConversationGatewayDurability::Ephemeral,
        scope: ConversationGatewayScope::System,
        payload: json!({
            "sessionId": session_id,
            "heartbeatIntervalMs": 30000,
            "resumeSupported": true,
            "route": crate::conversation_protocol::CONVERSATION_GATEWAY_ROUTE,
        }),
        occurred_at: Utc::now().to_rfc3339(),
    }
}

pub async fn handle_conversation_socket(
    mut socket: WebSocket,
    db_path: Arc<PathBuf>,
    conversation_sender: broadcast::Sender<ConversationGatewayEnvelope>,
) {
    let session_id = format!("conversation_session_{}", Uuid::new_v4());
    let mut session = ConversationGatewaySession::new(session_id.clone());
    if send_gateway_frame(&mut socket, &hello_frame(&session_id))
        .await
        .is_err()
    {
        return;
    }

    let mut conversation_receiver = conversation_sender.subscribe();
    loop {
        tokio::select! {
            incoming = socket.recv() => {
                let Some(Ok(message)) = incoming else {
                    return;
                };
                match message {
                    Message::Text(text) => {
                        let output = handle_gateway_text_frame(db_path.as_ref(), &mut session, &text);
                        for frame in output.frames {
                            if send_gateway_frame(&mut socket, &frame).await.is_err() {
                                return;
                            }
                        }
                        for frame in output.broadcast {
                            let _ = conversation_sender.send(frame);
                        }
                    }
                    Message::Close(_) => return,
                    Message::Ping(bytes) => {
                        if socket.send(Message::Pong(bytes)).await.is_err() {
                            return;
                        }
                    }
                    _ => {}
                }
            }
            outbound = conversation_receiver.recv() => {
                match outbound {
                    Ok(frame) => {
                        if frame.conversation_id.as_ref().is_some_and(|conversation_id| session.subscriptions.contains(conversation_id))
                            && send_gateway_frame(&mut socket, &frame).await.is_err()
                        {
                            return;
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(skipped)) => {
                        let frame = lagged_client_frame(skipped, &Utc::now().to_rfc3339());
                        if send_gateway_frame(&mut socket, &frame).await.is_err() {
                            return;
                        }
                    }
                    Err(broadcast::error::RecvError::Closed) => return,
                }
            }
        }
    }
}

async fn send_gateway_frame(
    socket: &mut WebSocket,
    frame: &ConversationGatewayEnvelope,
) -> std::result::Result<(), axum::Error> {
    socket
        .send(Message::Text(
            serde_json::to_string(frame).unwrap_or_else(|_| "{}".to_string()),
        ))
        .await
}

pub fn handle_gateway_text_frame(
    db_path: &Path,
    session: &mut ConversationGatewaySession,
    text: &str,
) -> ConversationGatewayOutput {
    let now = Utc::now().to_rfc3339();
    if text.len() > MAX_TEXT_FRAME_BYTES {
        return error_output(command_rejected_error(
            None,
            None,
            "frame_too_large",
            "Conversation gateway frame exceeds the maximum accepted size.",
            false,
            &now,
        ));
    }
    let Ok(envelope) = serde_json::from_str::<ConversationGatewayEnvelope>(text) else {
        return error_output(command_rejected_error(
            None,
            None,
            "invalid_envelope",
            "Frame must be a valid conversation gateway envelope.",
            false,
            &now,
        ));
    };
    if envelope.schema_version != CONVERSATION_GATEWAY_SCHEMA_VERSION {
        return error_output(command_rejected_error(
            envelope.client_id.as_deref(),
            envelope.conversation_id.as_deref(),
            "unsupported_protocol_version",
            "Unsupported conversation gateway protocol version.",
            false,
            &now,
        ));
    }

    match handle_gateway_envelope(db_path, session, envelope) {
        Ok(output) => output,
        Err(error) => error_output(command_rejected_error(
            None,
            None,
            "command_failed",
            &error.to_string(),
            false,
            &now,
        )),
    }
}

pub fn handle_gateway_envelope(
    db_path: &Path,
    session: &mut ConversationGatewaySession,
    envelope: ConversationGatewayEnvelope,
) -> Result<ConversationGatewayOutput> {
    match envelope.op {
        ConversationGatewayOp::Identify => identify(session, envelope),
        ConversationGatewayOp::Subscribe => subscribe(db_path, session, envelope),
        ConversationGatewayOp::Unsubscribe => unsubscribe(session, envelope),
        ConversationGatewayOp::Resume | ConversationGatewayOp::Replay => replay(db_path, envelope),
        ConversationGatewayOp::Heartbeat => heartbeat(envelope),
        ConversationGatewayOp::Command => command(db_path, session, envelope),
        _ => Ok(error_output(command_rejected_error(
            envelope.client_id.as_deref(),
            envelope.conversation_id.as_deref(),
            "unsupported_operation",
            "Operation is not supported by this gateway slice.",
            false,
            &Utc::now().to_rfc3339(),
        ))),
    }
}

fn identify(
    session: &mut ConversationGatewaySession,
    envelope: ConversationGatewayEnvelope,
) -> Result<ConversationGatewayOutput> {
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct IdentifyPayload {
        actor_id: Option<String>,
        participant_id: Option<String>,
    }
    let payload: IdentifyPayload = serde_json::from_value(envelope.payload.clone())?;
    session.actor_id = payload.actor_id;
    session.participant_id = payload.participant_id;
    Ok(single_frame(ack_envelope(
        required_client_id(&envelope)?,
        envelope.conversation_id.as_deref(),
        "identify.ack",
        json!({
            "actorId": session.actor_id,
            "participantId": session.participant_id,
        }),
        &Utc::now().to_rfc3339(),
    )))
}

fn subscribe(
    db_path: &Path,
    session: &mut ConversationGatewaySession,
    envelope: ConversationGatewayEnvelope,
) -> Result<ConversationGatewayOutput> {
    let conversation_id = required_conversation_id(&envelope)?.to_string();
    session.subscriptions.insert(conversation_id.clone());
    let replay = replay_conversation_events(
        db_path,
        &conversation_id,
        after_sequence(&envelope),
        limit(&envelope),
    )?;
    let mut frames = vec![ack_envelope(
        required_client_id(&envelope)?,
        Some(&conversation_id),
        "conversation.subscribe.ack",
        json!({ "conversationId": conversation_id }),
        &Utc::now().to_rfc3339(),
    )];
    frames.extend(replay);
    Ok(ConversationGatewayOutput {
        frames,
        broadcast: vec![],
    })
}

fn unsubscribe(
    session: &mut ConversationGatewaySession,
    envelope: ConversationGatewayEnvelope,
) -> Result<ConversationGatewayOutput> {
    let conversation_id = required_conversation_id(&envelope)?.to_string();
    session.subscriptions.remove(&conversation_id);
    Ok(single_frame(ack_envelope(
        required_client_id(&envelope)?,
        Some(&conversation_id),
        "conversation.unsubscribe.ack",
        json!({ "conversationId": conversation_id }),
        &Utc::now().to_rfc3339(),
    )))
}

fn replay(
    db_path: &Path,
    envelope: ConversationGatewayEnvelope,
) -> Result<ConversationGatewayOutput> {
    let conversation_id = required_conversation_id(&envelope)?;
    let replay = replay_conversation_events(
        db_path,
        conversation_id,
        after_sequence(&envelope),
        limit(&envelope),
    )?;
    Ok(ConversationGatewayOutput {
        frames: replay,
        broadcast: vec![],
    })
}

fn heartbeat(envelope: ConversationGatewayEnvelope) -> Result<ConversationGatewayOutput> {
    Ok(single_frame(ack_envelope(
        envelope.client_id.as_deref().unwrap_or("heartbeat"),
        envelope.conversation_id.as_deref(),
        "heartbeat.ack",
        json!({ "receivedAt": Utc::now().to_rfc3339() }),
        &Utc::now().to_rfc3339(),
    )))
}

fn command(
    db_path: &Path,
    session: &mut ConversationGatewaySession,
    envelope: ConversationGatewayEnvelope,
) -> Result<ConversationGatewayOutput> {
    let command_type = envelope.frame_type.as_str();
    match command_type {
        "conversation.subscribe" => subscribe(db_path, session, envelope),
        "conversation.replay_after_cursor" => replay(db_path, envelope),
        "message.submit" => {
            if let Some(output) = enforce_message_command_rate_limit(session, &envelope)? {
                return Ok(output);
            }
            message_submit(db_path, session, envelope)
        }
        "message.edit" => {
            if let Some(output) = enforce_message_command_rate_limit(session, &envelope)? {
                return Ok(output);
            }
            message_edit(db_path, session, envelope)
        }
        "message.delete" => {
            if let Some(output) = enforce_message_command_rate_limit(session, &envelope)? {
                return Ok(output);
            }
            message_delete(db_path, session, envelope)
        }
        "message.undo" => {
            if let Some(output) = enforce_message_command_rate_limit(session, &envelope)? {
                return Ok(output);
            }
            message_undo(db_path, session, envelope)
        }
        "message.mark_read" => message_mark_read(db_path, session, envelope),
        "message.mark_unread" => message_mark_unread(db_path, session, envelope),
        "message.react" => message_react(db_path, session, envelope),
        "presence.update" => presence_update(db_path, session, envelope),
        "typing.start" | "typing.stop" => typing(session, envelope),
        "conversation.handoff.create" => handoff_create(db_path, session, envelope),
        "conversation.handoff.accept" | "handoff.accept" => handoff_transition(
            db_path,
            session,
            envelope,
            HandoffStatus::Accepted,
            "conversation.handoff.accept.ack",
        ),
        "conversation.handoff.decline" | "handoff.decline" => handoff_transition(
            db_path,
            session,
            envelope,
            HandoffStatus::Declined,
            "conversation.handoff.decline.ack",
        ),
        "conversation.handoff.assign" | "handoff.assign" => handoff_transition(
            db_path,
            session,
            envelope,
            HandoffStatus::Assigned,
            "conversation.handoff.assign.ack",
        ),
        "conversation.handoff.return_to_agent" | "handoff.return_to_agent" => handoff_transition(
            db_path,
            session,
            envelope,
            HandoffStatus::ReturnedToAgent,
            "conversation.handoff.return_to_agent.ack",
        ),
        "conversation.handoff.close" => handoff_transition(
            db_path,
            session,
            envelope,
            HandoffStatus::Closed,
            "conversation.handoff.close.ack",
        ),
        "conversation.mode.set" => conversation_mode_set(db_path, session, envelope),
        "conversation.mode.human_led_active" | "agent.takeover" => conversation_mode_fixed(
            db_path,
            session,
            envelope,
            ConversationMode::HumanLedActive,
            false,
            "conversation.mode.human_led_active.ack",
        ),
        "conversation.mode.return_to_agent" => conversation_mode_fixed(
            db_path,
            session,
            envelope,
            ConversationMode::ReturnedToAgent,
            false,
            "conversation.mode.return_to_agent.ack",
        ),
        "conversation.agent.delegate" | "agent.delegate" => {
            agent_delegation(db_path, session, envelope, true)
        }
        "conversation.agent.delegation_revoke" => {
            agent_delegation(db_path, session, envelope, false)
        }
        _ => Ok(error_output(command_rejected_error(
            envelope.client_id.as_deref(),
            envelope.conversation_id.as_deref(),
            "unsupported_command",
            "Command is not implemented by this gateway slice.",
            false,
            &Utc::now().to_rfc3339(),
        ))),
    }
}

fn message_submit(
    db_path: &Path,
    session: &ConversationGatewaySession,
    envelope: ConversationGatewayEnvelope,
) -> Result<ConversationGatewayOutput> {
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct SubmitPayload {
        participant_id: Option<String>,
        body_markdown: String,
        client_message_id: String,
        message_kind: Option<String>,
        visibility: Option<String>,
        undo_expires_at: Option<String>,
    }
    let conversation_id = required_conversation_id(&envelope)?.to_string();
    let payload: SubmitPayload = serde_json::from_value(envelope.payload.clone())?;
    let participant_id = payload
        .participant_id
        .or_else(|| session.participant_id.clone())
        .ok_or_else(|| anyhow::anyhow!("participantId is required"))?;
    let actor = mutation_actor(session, envelope.client_id.clone());
    let connection = Connection::open(db_path)?;
    let receipt = ConversationService::submit_message(
        &connection,
        &actor,
        &ConversationMessageCreateRequest {
            conversation_id: conversation_id.clone(),
            segment_id: None,
            participant_id,
            message_kind: payload.message_kind.unwrap_or_else(|| "human".to_string()),
            body_markdown: payload.body_markdown,
            visibility: payload
                .visibility
                .unwrap_or_else(|| "participants".to_string()),
            client_message_id: payload.client_message_id,
            reply_to_message_id: None,
            undo_expires_at: payload.undo_expires_at,
        },
    )?;
    command_ack_and_dispatch(
        &envelope,
        "message.submit.ack",
        &receipt.value.id,
        "message.created",
        db_path,
    )
}

fn message_edit(
    db_path: &Path,
    session: &ConversationGatewaySession,
    envelope: ConversationGatewayEnvelope,
) -> Result<ConversationGatewayOutput> {
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct EditPayload {
        participant_id: Option<String>,
        message_id: String,
        body_markdown: String,
        reason: Option<String>,
    }
    let payload: EditPayload = serde_json::from_value(envelope.payload.clone())?;
    let participant_id = payload
        .participant_id
        .or_else(|| session.participant_id.clone())
        .ok_or_else(|| anyhow::anyhow!("participantId is required"))?;
    let actor = mutation_actor(session, envelope.client_id.clone());
    let connection = Connection::open(db_path)?;
    let receipt = ConversationService::edit_message(
        &connection,
        &actor,
        &payload.message_id,
        &participant_id,
        &payload.body_markdown,
        payload.reason.as_deref(),
    )?;
    command_ack_and_dispatch(
        &envelope,
        "message.edit.ack",
        &receipt.value.id,
        "message.edited",
        db_path,
    )
}

fn message_delete(
    db_path: &Path,
    session: &ConversationGatewaySession,
    envelope: ConversationGatewayEnvelope,
) -> Result<ConversationGatewayOutput> {
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct DeletePayload {
        participant_id: Option<String>,
        message_id: String,
        reason: String,
    }
    let payload: DeletePayload = serde_json::from_value(envelope.payload.clone())?;
    let participant_id = payload
        .participant_id
        .or_else(|| session.participant_id.clone())
        .ok_or_else(|| anyhow::anyhow!("participantId is required"))?;
    let actor = mutation_actor(session, envelope.client_id.clone());
    let connection = Connection::open(db_path)?;
    let receipt = ConversationService::delete_message(
        &connection,
        &actor,
        &payload.message_id,
        &participant_id,
        &payload.reason,
    )?;
    command_ack_and_dispatch(
        &envelope,
        "message.delete.ack",
        &receipt.value.id,
        "message.tombstoned",
        db_path,
    )
}

fn message_undo(
    db_path: &Path,
    session: &ConversationGatewaySession,
    envelope: ConversationGatewayEnvelope,
) -> Result<ConversationGatewayOutput> {
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct UndoPayload {
        participant_id: Option<String>,
        message_id: String,
    }
    let payload: UndoPayload = serde_json::from_value(envelope.payload.clone())?;
    let participant_id = payload
        .participant_id
        .or_else(|| session.participant_id.clone())
        .ok_or_else(|| anyhow::anyhow!("participantId is required"))?;
    let actor = mutation_actor(session, envelope.client_id.clone());
    let connection = Connection::open(db_path)?;
    let receipt = ConversationService::undo_message(
        &connection,
        &actor,
        &payload.message_id,
        &participant_id,
    )?;
    command_ack_and_dispatch(
        &envelope,
        "message.undo.ack",
        &receipt.value.id,
        "message.undo.cancelled",
        db_path,
    )
}

fn message_mark_read(
    db_path: &Path,
    session: &ConversationGatewaySession,
    envelope: ConversationGatewayEnvelope,
) -> Result<ConversationGatewayOutput> {
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct MarkReadPayload {
        participant_id: Option<String>,
        message_id: String,
    }
    let conversation_id = required_conversation_id(&envelope)?.to_string();
    let payload: MarkReadPayload = serde_json::from_value(envelope.payload.clone())?;
    let participant_id = payload
        .participant_id
        .or_else(|| session.participant_id.clone())
        .ok_or_else(|| anyhow::anyhow!("participantId is required"))?;
    let actor = mutation_actor(session, envelope.client_id.clone());
    let connection = Connection::open(db_path)?;
    let receipt = ConversationService::mark_read(
        &connection,
        &actor,
        &conversation_id,
        &participant_id,
        &payload.message_id,
    )?;
    ack_with_optional_message_dispatch(
        &envelope,
        "message.mark_read.ack",
        json!({
            "messageId": payload.message_id,
            "readState": receipt.value.value,
            "changed": receipt.value.changed,
        }),
        receipt.value.event_type.as_deref(),
        receipt.value.changed,
        db_path,
        &payload.message_id,
    )
}

fn message_mark_unread(
    db_path: &Path,
    session: &ConversationGatewaySession,
    envelope: ConversationGatewayEnvelope,
) -> Result<ConversationGatewayOutput> {
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct MarkUnreadPayload {
        participant_id: Option<String>,
        message_id: String,
    }
    let conversation_id = required_conversation_id(&envelope)?.to_string();
    let payload: MarkUnreadPayload = serde_json::from_value(envelope.payload.clone())?;
    let participant_id = payload
        .participant_id
        .or_else(|| session.participant_id.clone())
        .ok_or_else(|| anyhow::anyhow!("participantId is required"))?;
    let actor = mutation_actor(session, envelope.client_id.clone());
    let connection = Connection::open(db_path)?;
    let receipt = ConversationService::mark_unread(
        &connection,
        &actor,
        &conversation_id,
        &participant_id,
        &payload.message_id,
    )?;
    ack_with_optional_message_dispatch(
        &envelope,
        "message.mark_unread.ack",
        json!({
            "messageId": payload.message_id,
            "readState": receipt.value.value,
            "changed": receipt.value.changed,
        }),
        receipt.value.event_type.as_deref(),
        receipt.value.changed,
        db_path,
        &payload.message_id,
    )
}

fn message_react(
    db_path: &Path,
    session: &ConversationGatewaySession,
    envelope: ConversationGatewayEnvelope,
) -> Result<ConversationGatewayOutput> {
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct ReactPayload {
        participant_id: Option<String>,
        message_id: String,
        reaction_key: String,
        reaction_kind: Option<String>,
        action: Option<String>,
    }
    let payload: ReactPayload = serde_json::from_value(envelope.payload.clone())?;
    let participant_id = payload
        .participant_id
        .or_else(|| session.participant_id.clone())
        .ok_or_else(|| anyhow::anyhow!("participantId is required"))?;
    let action = ReactionAction::try_from(payload.action.as_deref().unwrap_or("toggle"))?;
    let actor = mutation_actor(session, envelope.client_id.clone());
    let connection = Connection::open(db_path)?;
    let receipt = ConversationService::react_to_message(
        &connection,
        &actor,
        &payload.message_id,
        &participant_id,
        &payload.reaction_key,
        payload.reaction_kind.as_deref().unwrap_or("emoji"),
        action,
    )?;
    ack_with_optional_message_dispatch(
        &envelope,
        "message.react.ack",
        json!({
            "messageId": payload.message_id,
            "reaction": receipt.value.value,
            "changed": receipt.value.changed,
        }),
        receipt.value.event_type.as_deref(),
        receipt.value.changed,
        db_path,
        &payload.message_id,
    )
}

fn presence_update(
    db_path: &Path,
    session: &ConversationGatewaySession,
    envelope: ConversationGatewayEnvelope,
) -> Result<ConversationGatewayOutput> {
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct PresencePayload {
        participant_id: Option<String>,
        status: String,
        visibility: Option<String>,
        status_message: Option<String>,
        device_class: Option<String>,
        expires_at: Option<String>,
    }
    let conversation_id = required_conversation_id(&envelope)?.to_string();
    let payload: PresencePayload = serde_json::from_value(envelope.payload.clone())?;
    let participant_id = payload
        .participant_id
        .or_else(|| session.participant_id.clone())
        .ok_or_else(|| anyhow::anyhow!("participantId is required"))?;
    let actor = mutation_actor(session, envelope.client_id.clone());
    let connection = Connection::open(db_path)?;
    let receipt = ConversationService::update_presence(
        &connection,
        &actor,
        &ConversationPresenceUpdateRequest {
            conversation_id: conversation_id.clone(),
            participant_id: participant_id.clone(),
            status: payload.status,
            visibility: payload
                .visibility
                .unwrap_or_else(|| "participants".to_string()),
            status_message: payload.status_message,
            device_class: payload.device_class,
            expires_at: payload.expires_at,
        },
    )?;
    let dispatch = ConversationGatewayEnvelope {
        schema_version: CONVERSATION_GATEWAY_SCHEMA_VERSION.to_string(),
        op: ConversationGatewayOp::Dispatch,
        frame_type: "presence.changed".to_string(),
        client_id: envelope.client_id.clone(),
        server_id: None,
        conversation_id: Some(conversation_id.clone()),
        segment_id: None,
        sequence: None,
        cursor: None,
        durability: ConversationGatewayDurability::Ephemeral,
        scope: ConversationGatewayScope::Conversation,
        payload: json!({ "presence": receipt.value }),
        occurred_at: Utc::now().to_rfc3339(),
    };
    Ok(ConversationGatewayOutput {
        frames: vec![ack_envelope(
            required_client_id(&envelope)?,
            Some(&conversation_id),
            "presence.update.ack",
            json!({ "participantId": participant_id }),
            &Utc::now().to_rfc3339(),
        )],
        broadcast: vec![dispatch],
    })
}

fn typing(
    session: &mut ConversationGatewaySession,
    envelope: ConversationGatewayEnvelope,
) -> Result<ConversationGatewayOutput> {
    let conversation_id = required_conversation_id(&envelope)?.to_string();
    let participant_id = session
        .participant_id
        .clone()
        .ok_or_else(|| anyhow::anyhow!("participantId is required before typing"))?;
    let participants = session
        .typing_by_conversation
        .entry(conversation_id.clone())
        .or_default();
    if envelope.frame_type == "typing.start" {
        participants.insert(participant_id.clone());
    } else {
        participants.remove(&participant_id);
    }
    let event_type = if envelope.frame_type == "typing.start" {
        "typing.started"
    } else {
        "typing.stopped"
    };
    let dispatch = ConversationGatewayEnvelope {
        schema_version: CONVERSATION_GATEWAY_SCHEMA_VERSION.to_string(),
        op: ConversationGatewayOp::Dispatch,
        frame_type: event_type.to_string(),
        client_id: envelope.client_id.clone(),
        server_id: None,
        conversation_id: Some(conversation_id.clone()),
        segment_id: None,
        sequence: None,
        cursor: None,
        durability: ConversationGatewayDurability::Ephemeral,
        scope: ConversationGatewayScope::Conversation,
        payload: json!({
            "conversationId": conversation_id,
            "participantId": participant_id,
            "expiresAt": if event_type == "typing.started" {
                Some((Utc::now() + chrono::Duration::seconds(5)).to_rfc3339())
            } else {
                None
            },
        }),
        occurred_at: Utc::now().to_rfc3339(),
    };
    Ok(ConversationGatewayOutput {
        frames: vec![ack_envelope(
            required_client_id(&envelope)?,
            dispatch.conversation_id.as_deref(),
            "typing.ack",
            json!({ "eventType": event_type }),
            &Utc::now().to_rfc3339(),
        )],
        broadcast: vec![dispatch],
    })
}

fn handoff_create(
    db_path: &Path,
    session: &ConversationGatewaySession,
    envelope: ConversationGatewayEnvelope,
) -> Result<ConversationGatewayOutput> {
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct HandoffCreatePayload {
        segment_id: Option<String>,
        connection_id: Option<String>,
        requested_by_actor_id: Option<String>,
        assigned_to_actor_id: Option<String>,
        reason: String,
        urgency: Option<String>,
        required_capability_id: Option<String>,
        evidence_summary: String,
        allowed_context: Option<Vec<String>>,
        policy_decision_id: Option<String>,
    }

    let conversation_id = required_conversation_id(&envelope)?.to_string();
    let payload: HandoffCreatePayload = serde_json::from_value(envelope.payload.clone())?;
    let connection = Connection::open(db_path)?;
    let handoff = create_conversation_handoff(
        &connection,
        &ConversationHandoffCreateRequest {
            conversation_id: conversation_id.clone(),
            segment_id: payload.segment_id,
            connection_id: payload.connection_id,
            requested_by_actor_id: payload
                .requested_by_actor_id
                .or_else(|| session.actor_id.clone()),
            assigned_to_actor_id: payload.assigned_to_actor_id,
            reason: payload.reason,
            urgency: payload.urgency.unwrap_or_else(|| "normal".to_string()),
            required_capability_id: payload
                .required_capability_id
                .unwrap_or_else(|| "conversation.handoff.manage".to_string()),
            evidence_summary: payload.evidence_summary,
            allowed_context: payload.allowed_context.unwrap_or_default(),
            policy_decision_id: payload.policy_decision_id,
        },
    )?;
    ack_and_latest_conversation_event(
        &envelope,
        "conversation.handoff.create.ack",
        json!({ "handoffId": handoff.id, "status": "requested" }),
        "conversation.handoff.requested",
        Some(&handoff.id),
        db_path,
    )
}

fn handoff_transition(
    db_path: &Path,
    session: &ConversationGatewaySession,
    envelope: ConversationGatewayEnvelope,
    next_status: HandoffStatus,
    ack_type: &str,
) -> Result<ConversationGatewayOutput> {
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct HandoffTransitionPayload {
        handoff_id: String,
        actor_id: Option<String>,
        assigned_to_actor_id: Option<String>,
        reason: Option<String>,
    }

    let payload: HandoffTransitionPayload = serde_json::from_value(envelope.payload.clone())?;
    let actor_id = payload
        .actor_id
        .or(payload.assigned_to_actor_id)
        .or_else(|| session.actor_id.clone());
    let reason = payload.reason.unwrap_or_else(|| {
        format!(
            "gateway command {}",
            envelope.frame_type.trim_start_matches("conversation.")
        )
    });
    let connection = Connection::open(db_path)?;
    let handoff = transition_conversation_handoff(
        &connection,
        &payload.handoff_id,
        next_status,
        actor_id.as_deref(),
        &reason,
    )?;
    let event_type = handoff_status_event_type(next_status);
    ack_and_latest_conversation_event(
        &envelope,
        ack_type,
        json!({ "handoffId": handoff.id, "status": next_status }),
        event_type,
        Some(&handoff.id),
        db_path,
    )
}

fn conversation_mode_set(
    db_path: &Path,
    session: &ConversationGatewaySession,
    envelope: ConversationGatewayEnvelope,
) -> Result<ConversationGatewayOutput> {
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct ModePayload {
        mode: String,
        led_by_actor_id: Option<String>,
        delegated_to_agent: Option<bool>,
        delegation_scope: Option<Vec<String>>,
        idle_after: Option<String>,
    }

    let conversation_id = required_conversation_id(&envelope)?.to_string();
    let payload: ModePayload = serde_json::from_value(envelope.payload.clone())?;
    let mode = ConversationMode::try_from(payload.mode.as_str())?;
    let led_by_actor_id = payload.led_by_actor_id.or_else(|| session.actor_id.clone());
    let connection = Connection::open(db_path)?;
    let mode_view = upsert_conversation_mode(
        &connection,
        &conversation_id,
        mode,
        led_by_actor_id.as_deref(),
        payload.delegated_to_agent.unwrap_or(false),
        payload.delegation_scope.unwrap_or_default(),
        payload.idle_after.as_deref(),
    )?;
    ack_and_latest_conversation_event(
        &envelope,
        "conversation.mode.set.ack",
        json!({ "mode": mode_view }),
        "conversation.mode.changed",
        None,
        db_path,
    )
}

fn conversation_mode_fixed(
    db_path: &Path,
    session: &ConversationGatewaySession,
    envelope: ConversationGatewayEnvelope,
    mode: ConversationMode,
    delegated_to_agent: bool,
    ack_type: &str,
) -> Result<ConversationGatewayOutput> {
    let conversation_id = required_conversation_id(&envelope)?.to_string();
    let connection = Connection::open(db_path)?;
    let mode_view = upsert_conversation_mode(
        &connection,
        &conversation_id,
        mode,
        session.actor_id.as_deref(),
        delegated_to_agent,
        vec![],
        None,
    )?;
    ack_and_latest_conversation_event(
        &envelope,
        ack_type,
        json!({ "mode": mode_view }),
        "conversation.mode.changed",
        None,
        db_path,
    )
}

fn agent_delegation(
    db_path: &Path,
    session: &ConversationGatewaySession,
    envelope: ConversationGatewayEnvelope,
    delegated_to_agent: bool,
) -> Result<ConversationGatewayOutput> {
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct DelegationPayload {
        delegation_scope: Option<Vec<String>>,
        reason: Option<String>,
    }

    let conversation_id = required_conversation_id(&envelope)?.to_string();
    let payload: DelegationPayload = serde_json::from_value(envelope.payload.clone())?;
    if delegated_to_agent {
        ensure!(
            payload
                .delegation_scope
                .as_ref()
                .is_some_and(|scope| !scope.is_empty()),
            "delegationScope is required for agent delegation"
        );
    }
    let _reason = payload.reason;
    let scope = if delegated_to_agent {
        payload.delegation_scope.unwrap_or_default()
    } else {
        vec![]
    };
    let connection = Connection::open(db_path)?;
    let mode_view = upsert_conversation_mode(
        &connection,
        &conversation_id,
        ConversationMode::HumanLedActive,
        session.actor_id.as_deref(),
        delegated_to_agent,
        scope,
        None,
    )?;
    let ack_type = if delegated_to_agent {
        "conversation.agent.delegate.ack"
    } else {
        "conversation.agent.delegation_revoke.ack"
    };
    ack_and_latest_conversation_event(
        &envelope,
        ack_type,
        json!({ "mode": mode_view }),
        "conversation.mode.changed",
        None,
        db_path,
    )
}

fn command_ack_and_dispatch(
    envelope: &ConversationGatewayEnvelope,
    ack_type: &str,
    message_id: &str,
    event_type: &str,
    db_path: &Path,
) -> Result<ConversationGatewayOutput> {
    let conversation_id = required_conversation_id(envelope)?.to_string();
    let dispatch = latest_message_event(db_path, &conversation_id, message_id, event_type)?;
    Ok(ConversationGatewayOutput {
        frames: vec![ack_envelope(
            required_client_id(envelope)?,
            Some(&conversation_id),
            ack_type,
            json!({ "messageId": message_id }),
            &Utc::now().to_rfc3339(),
        )],
        broadcast: vec![dispatch],
    })
}

fn ack_with_optional_message_dispatch(
    envelope: &ConversationGatewayEnvelope,
    ack_type: &str,
    payload: Value,
    event_type: Option<&str>,
    changed: bool,
    db_path: &Path,
    message_id: &str,
) -> Result<ConversationGatewayOutput> {
    let conversation_id = required_conversation_id(envelope)?.to_string();
    let broadcast = if changed {
        if let Some(event_type) = event_type {
            vec![latest_message_event(
                db_path,
                &conversation_id,
                message_id,
                event_type,
            )?]
        } else {
            vec![]
        }
    } else {
        vec![]
    };
    Ok(ConversationGatewayOutput {
        frames: vec![ack_envelope(
            required_client_id(envelope)?,
            Some(&conversation_id),
            ack_type,
            payload,
            &Utc::now().to_rfc3339(),
        )],
        broadcast,
    })
}

fn ack_and_latest_conversation_event(
    envelope: &ConversationGatewayEnvelope,
    ack_type: &str,
    payload: Value,
    event_type: &str,
    payload_marker: Option<&str>,
    db_path: &Path,
) -> Result<ConversationGatewayOutput> {
    let conversation_id = required_conversation_id(envelope)?.to_string();
    let dispatch =
        latest_conversation_event(db_path, &conversation_id, event_type, payload_marker)?;
    Ok(ConversationGatewayOutput {
        frames: vec![ack_envelope(
            required_client_id(envelope)?,
            Some(&conversation_id),
            ack_type,
            payload,
            &Utc::now().to_rfc3339(),
        )],
        broadcast: vec![dispatch],
    })
}

fn latest_message_event(
    db_path: &Path,
    conversation_id: &str,
    message_id: &str,
    event_type: &str,
) -> Result<ConversationGatewayEnvelope> {
    latest_conversation_event(db_path, conversation_id, event_type, Some(message_id))
}

fn latest_conversation_event(
    db_path: &Path,
    conversation_id: &str,
    event_type: &str,
    payload_marker: Option<&str>,
) -> Result<ConversationGatewayEnvelope> {
    let connection = Connection::open(db_path)?;
    let mut sql = String::from(
        "SELECT sequence, event_type, payload_json, realtime_cursor, occurred_at
         FROM conversation_events
         WHERE conversation_id = ?1
           AND event_type = ?2",
    );
    let marker = payload_marker.map(|marker| format!("%{marker}%"));
    if marker.is_some() {
        sql.push_str(" AND payload_json LIKE ?3");
    }
    sql.push_str(" ORDER BY sequence DESC LIMIT 1");
    let mut statement = connection.prepare(&sql)?;
    let map_row = |row: &rusqlite::Row<'_>| {
        let sequence: i64 = row.get(0)?;
        let event_type: String = row.get(1)?;
        let payload_json: String = row.get(2)?;
        let cursor: Option<i64> = row.get(3)?;
        let occurred_at: String = row.get(4)?;
        let payload = serde_json::from_str(&payload_json).unwrap_or_else(|_| json!({}));
        Ok(dispatch_envelope(
            &event_type,
            conversation_id,
            sequence,
            cursor,
            payload,
            &occurred_at,
        ))
    };
    let event = if let Some(marker) = marker {
        statement
            .query_row(params![conversation_id, event_type, marker], map_row)
            .optional()?
    } else {
        statement
            .query_row(params![conversation_id, event_type], map_row)
            .optional()?
    };
    event.ok_or_else(|| {
        anyhow::anyhow!("durable command completed without replayable {event_type} event")
    })
}

fn handoff_status_event_type(status: HandoffStatus) -> &'static str {
    match status {
        HandoffStatus::Suggested => "conversation.handoff.suggested",
        HandoffStatus::Requested => "conversation.handoff.requested",
        HandoffStatus::Accepted => "conversation.handoff.accepted",
        HandoffStatus::Declined => "conversation.handoff.declined",
        HandoffStatus::Assigned => "conversation.handoff.assigned",
        HandoffStatus::InProgress => "conversation.handoff.in_progress",
        HandoffStatus::ReturnedToAgent => "conversation.handoff.returned_to_agent",
        HandoffStatus::Closed => "conversation.handoff.closed",
    }
}

fn replay_conversation_events(
    db_path: &Path,
    conversation_id: &str,
    after_sequence: i64,
    limit: usize,
) -> Result<Vec<ConversationGatewayEnvelope>> {
    ensure!(
        (1..=MAX_REPLAY_LIMIT).contains(&limit),
        "invalid replay limit"
    );
    let connection = Connection::open(db_path)?;
    let mut statement = connection.prepare(
        "SELECT sequence, event_type, payload_json, realtime_cursor, occurred_at
         FROM conversation_events
         WHERE conversation_id = ?1 AND sequence > ?2
         ORDER BY sequence ASC
         LIMIT ?3",
    )?;
    let rows = statement.query_map(
        params![conversation_id, after_sequence, limit as i64],
        |row| {
            let sequence: i64 = row.get(0)?;
            let event_type: String = row.get(1)?;
            let payload_json: String = row.get(2)?;
            let cursor: Option<i64> = row.get(3)?;
            let occurred_at: String = row.get(4)?;
            let payload = serde_json::from_str(&payload_json).unwrap_or_else(|_| json!({}));
            Ok(dispatch_envelope(
                &event_type,
                conversation_id,
                sequence,
                cursor,
                payload,
                &occurred_at,
            ))
        },
    )?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(Into::into)
}

fn enforce_message_command_rate_limit(
    session: &mut ConversationGatewaySession,
    envelope: &ConversationGatewayEnvelope,
) -> Result<Option<ConversationGatewayOutput>> {
    let now = Utc::now();
    let floor = now - ChronoDuration::seconds(COMMAND_RATE_WINDOW_SECONDS);
    while session
        .recent_message_commands
        .front()
        .is_some_and(|timestamp| *timestamp < floor)
    {
        session.recent_message_commands.pop_front();
    }
    if session.recent_message_commands.len() >= COMMAND_RATE_LIMIT {
        return Ok(Some(error_output(command_rejected_error(
            envelope.client_id.as_deref(),
            envelope.conversation_id.as_deref(),
            "rate_limited",
            "Message command rate limit exceeded.",
            true,
            &now.to_rfc3339(),
        ))));
    }
    session.recent_message_commands.push_back(now);
    Ok(None)
}

fn mutation_actor(
    session: &ConversationGatewaySession,
    request_id: Option<String>,
) -> ConversationMutationActor {
    ConversationMutationActor {
        actor: ActorContext::new(
            ActorKind::BrowserOperator,
            "conversation_gateway",
            session.actor_id.clone(),
        ),
        request_id,
    }
}

fn required_client_id(envelope: &ConversationGatewayEnvelope) -> Result<&str> {
    envelope
        .client_id
        .as_deref()
        .filter(|client_id| !client_id.trim().is_empty())
        .ok_or_else(|| anyhow::anyhow!("clientId is required"))
}

fn required_conversation_id(envelope: &ConversationGatewayEnvelope) -> Result<&str> {
    envelope
        .conversation_id
        .as_deref()
        .filter(|conversation_id| !conversation_id.trim().is_empty())
        .ok_or_else(|| anyhow::anyhow!("conversationId is required"))
}

fn after_sequence(envelope: &ConversationGatewayEnvelope) -> i64 {
    envelope
        .payload
        .get("afterSequence")
        .and_then(Value::as_i64)
        .unwrap_or(0)
        .max(0)
}

fn limit(envelope: &ConversationGatewayEnvelope) -> usize {
    envelope
        .payload
        .get("limit")
        .and_then(Value::as_u64)
        .map(|limit| limit as usize)
        .unwrap_or(DEFAULT_REPLAY_LIMIT)
        .clamp(1, MAX_REPLAY_LIMIT)
}

fn single_frame(frame: ConversationGatewayEnvelope) -> ConversationGatewayOutput {
    ConversationGatewayOutput {
        frames: vec![frame],
        broadcast: vec![],
    }
}

fn error_output(frame: ConversationGatewayEnvelope) -> ConversationGatewayOutput {
    single_frame(frame)
}

fn lagged_client_frame(skipped: u64, occurred_at: &str) -> ConversationGatewayEnvelope {
    command_rejected_error(
        None,
        None,
        "client_lagged",
        &format!(
            "Conversation gateway client skipped {skipped} frame(s); replay from the latest durable cursor."
        ),
        true,
        occurred_at,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::conversations::{
        create_conversation_participant, find_or_create_canonical_conversation,
        CanonicalConversationRequest, ConversationParticipantCreateRequest,
    };
    use crate::schema::init_database;

    fn seeded_db() -> (tempfile::TempDir, std::path::PathBuf, String, String) {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let db_path = temp_dir.path().join("local.db");
        init_database(&db_path).unwrap();
        let connection = Connection::open(&db_path).unwrap();
        connection
            .execute(
                "INSERT INTO actors (id, actor_kind, display_name, status, metadata_json, created_at, updated_at)
                 VALUES ('actor_staff', 'staff', 'Staff', 'active', '{}', 'now', 'now')",
                [],
            )
            .unwrap();
        let conversation = find_or_create_canonical_conversation(
            &connection,
            &CanonicalConversationRequest {
                surface: "client_portal".to_string(),
                subject_kind: "connection".to_string(),
                subject_id: "connection_1".to_string(),
                connection_id: None,
                visitor_session_id: None,
                created_by_actor_id: None,
            },
        )
        .unwrap();
        let participant = create_conversation_participant(
            &connection,
            &ConversationParticipantCreateRequest {
                conversation_id: conversation.id.clone(),
                participant_kind: "staff".to_string(),
                actor_id: Some("actor_staff".to_string()),
                connection_id: None,
                visitor_session_id: None,
                display_name: "Staff".to_string(),
                role: "staff".to_string(),
            },
        )
        .unwrap();
        (temp_dir, db_path, conversation.id, participant.id)
    }

    fn session(participant_id: String) -> ConversationGatewaySession {
        let mut session = ConversationGatewaySession::new("session_1");
        session.actor_id = Some("actor_staff".to_string());
        session.participant_id = Some(participant_id);
        session
    }

    fn command(
        frame_type: &str,
        client_id: &str,
        conversation_id: &str,
        payload: Value,
    ) -> ConversationGatewayEnvelope {
        ConversationGatewayEnvelope {
            schema_version: CONVERSATION_GATEWAY_SCHEMA_VERSION.to_string(),
            op: ConversationGatewayOp::Command,
            frame_type: frame_type.to_string(),
            client_id: Some(client_id.to_string()),
            server_id: None,
            conversation_id: Some(conversation_id.to_string()),
            segment_id: None,
            sequence: None,
            cursor: None,
            durability: ConversationGatewayDurability::Durable,
            scope: ConversationGatewayScope::Conversation,
            payload,
            occurred_at: "2026-05-09T00:00:00Z".to_string(),
        }
    }

    #[test]
    fn hello_frame_shape_is_stable() {
        let hello = hello_frame("session_1");

        assert_eq!(hello.schema_version, CONVERSATION_GATEWAY_SCHEMA_VERSION);
        assert_eq!(hello.op, ConversationGatewayOp::Hello);
        assert_eq!(hello.frame_type, "gateway.hello");
        assert_eq!(hello.payload["resumeSupported"], true);
    }

    #[test]
    fn malformed_and_unsupported_frames_return_structured_errors() {
        let (_temp_dir, db_path, _conversation_id, participant_id) = seeded_db();
        let mut session = session(participant_id);

        let malformed = handle_gateway_text_frame(&db_path, &mut session, "{");
        assert_eq!(malformed.frames[0].op, ConversationGatewayOp::Error);
        assert_eq!(malformed.frames[0].payload["code"], "invalid_envelope");

        let unsupported = handle_gateway_text_frame(
            &db_path,
            &mut session,
            r#"{"schemaVersion":"bad","op":"command","type":"message.submit","durability":"durable","scope":"conversation","payload":{},"occurredAt":"now"}"#,
        );
        assert_eq!(
            unsupported.frames[0].payload["code"],
            "unsupported_protocol_version"
        );

        let oversized = handle_gateway_text_frame(
            &db_path,
            &mut session,
            &"x".repeat(MAX_TEXT_FRAME_BYTES + 1),
        );
        assert_eq!(oversized.frames[0].op, ConversationGatewayOp::Error);
        assert_eq!(oversized.frames[0].payload["code"], "frame_too_large");
        assert_eq!(oversized.frames[0].payload["retryable"], false);
    }

    #[test]
    fn lagged_client_error_is_retryable_and_points_to_replay() {
        let frame = lagged_client_frame(7, "2026-05-09T00:00:00Z");

        assert_eq!(frame.op, ConversationGatewayOp::Error);
        assert_eq!(frame.frame_type, "command.rejected");
        assert_eq!(frame.payload["code"], "client_lagged");
        assert_eq!(frame.payload["retryable"], true);
        assert!(frame.payload["message"]
            .as_str()
            .unwrap()
            .contains("replay"));
    }

    #[test]
    fn submit_edit_delete_and_undo_return_ack_and_canonical_dispatch() {
        let (_temp_dir, db_path, conversation_id, participant_id) = seeded_db();
        let mut session = session(participant_id.clone());

        let submit = handle_gateway_envelope(
            &db_path,
            &mut session,
            command(
                "message.submit",
                "client_submit_1",
                &conversation_id,
                json!({
                    "bodyMarkdown": "hello",
                    "clientMessageId": "client_msg_1",
                    "undoExpiresAt": "2099-05-09T00:00:30Z"
                }),
            ),
        )
        .unwrap();
        assert_eq!(submit.frames[0].op, ConversationGatewayOp::Ack);
        assert_eq!(
            submit.frames[0].client_id.as_deref(),
            Some("client_submit_1")
        );
        assert_eq!(submit.broadcast[0].frame_type, "message.created");
        assert!(submit.broadcast[0].sequence.is_some());
        assert!(submit.broadcast[0].cursor.is_some());
        let message_id = submit.frames[0].payload["messageId"].as_str().unwrap();

        let edit = handle_gateway_envelope(
            &db_path,
            &mut session,
            command(
                "message.edit",
                "client_edit_1",
                &conversation_id,
                json!({
                    "messageId": message_id,
                    "bodyMarkdown": "edited",
                    "reason": "clarity"
                }),
            ),
        )
        .unwrap();
        assert_eq!(edit.broadcast[0].frame_type, "message.edited");

        let undo = handle_gateway_envelope(
            &db_path,
            &mut session,
            command(
                "message.undo",
                "client_undo_1",
                &conversation_id,
                json!({ "messageId": message_id }),
            ),
        )
        .unwrap();
        assert_eq!(undo.broadcast[0].frame_type, "message.undo.cancelled");

        let submit_delete = handle_gateway_envelope(
            &db_path,
            &mut session,
            command(
                "message.submit",
                "client_submit_2",
                &conversation_id,
                json!({
                    "bodyMarkdown": "delete me",
                    "clientMessageId": "client_msg_2"
                }),
            ),
        )
        .unwrap();
        let delete_message_id = submit_delete.frames[0].payload["messageId"]
            .as_str()
            .unwrap();
        let delete = handle_gateway_envelope(
            &db_path,
            &mut session,
            command(
                "message.delete",
                "client_delete_1",
                &conversation_id,
                json!({ "messageId": delete_message_id, "reason": "test" }),
            ),
        )
        .unwrap();
        assert_eq!(delete.broadcast[0].frame_type, "message.tombstoned");
    }

    #[test]
    fn subscribe_and_replay_return_ordered_events() {
        let (_temp_dir, db_path, conversation_id, participant_id) = seeded_db();
        let mut session = session(participant_id);
        let _submit = handle_gateway_envelope(
            &db_path,
            &mut session,
            command(
                "message.submit",
                "client_submit_1",
                &conversation_id,
                json!({ "bodyMarkdown": "hello", "clientMessageId": "client_msg_1" }),
            ),
        )
        .unwrap();

        let replay = handle_gateway_envelope(
            &db_path,
            &mut session,
            command(
                "conversation.replay_after_cursor",
                "client_replay_1",
                &conversation_id,
                json!({ "afterSequence": 0, "limit": 20 }),
            ),
        )
        .unwrap();

        assert!(!replay.frames.is_empty());
        assert!(replay
            .frames
            .windows(2)
            .all(|window| window[0].sequence <= window[1].sequence));

        let latest_sequence = replay
            .frames
            .last()
            .and_then(|frame| frame.sequence)
            .unwrap();
        let stale_replay = handle_gateway_envelope(
            &db_path,
            &mut session,
            command(
                "conversation.replay_after_cursor",
                "client_replay_stale",
                &conversation_id,
                json!({ "afterSequence": latest_sequence + 100, "limit": 20 }),
            ),
        )
        .unwrap();
        assert!(stale_replay.frames.is_empty());
    }

    #[test]
    fn duplicate_submit_retry_uses_canonical_message_without_duplicate_events() {
        let (_temp_dir, db_path, conversation_id, participant_id) = seeded_db();
        let mut session = session(participant_id);

        let first = handle_gateway_envelope(
            &db_path,
            &mut session,
            command(
                "message.submit",
                "client_submit_first",
                &conversation_id,
                json!({
                    "bodyMarkdown": "hello",
                    "clientMessageId": "client_msg_retry"
                }),
            ),
        )
        .unwrap();
        let retry = handle_gateway_envelope(
            &db_path,
            &mut session,
            command(
                "message.submit",
                "client_submit_retry",
                &conversation_id,
                json!({
                    "bodyMarkdown": "hello",
                    "clientMessageId": "client_msg_retry"
                }),
            ),
        )
        .unwrap();

        assert_eq!(
            first.frames[0].payload["messageId"],
            retry.frames[0].payload["messageId"]
        );
        assert_eq!(
            retry.frames[0].client_id.as_deref(),
            Some("client_submit_retry")
        );

        let connection = Connection::open(&db_path).unwrap();
        let message_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM conversation_messages WHERE client_message_id = 'client_msg_retry'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let created_event_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM conversation_events
                 WHERE conversation_id = ?1
                   AND event_type = 'message.created'
                   AND payload_json LIKE '%client_msg_retry%'",
                params![conversation_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(message_count, 1);
        assert_eq!(created_event_count, 1);
    }

    #[test]
    fn message_command_flood_returns_structured_rejection() {
        let (_temp_dir, db_path, conversation_id, participant_id) = seeded_db();
        let mut session = session(participant_id);

        for index in 0..COMMAND_RATE_LIMIT {
            let accepted = handle_gateway_envelope(
                &db_path,
                &mut session,
                command(
                    "message.submit",
                    &format!("client_submit_{index}"),
                    &conversation_id,
                    json!({
                        "bodyMarkdown": format!("message {index}"),
                        "clientMessageId": format!("client_msg_{index}")
                    }),
                ),
            )
            .unwrap();
            assert_eq!(accepted.frames[0].op, ConversationGatewayOp::Ack);
        }

        let rejected = handle_gateway_text_frame(
            &db_path,
            &mut session,
            &serde_json::to_string(&command(
                "message.submit",
                "client_submit_over_limit",
                &conversation_id,
                json!({
                    "bodyMarkdown": "too many",
                    "clientMessageId": "client_msg_over_limit"
                }),
            ))
            .unwrap(),
        );
        assert_eq!(rejected.frames[0].op, ConversationGatewayOp::Error);
        assert_eq!(rejected.frames[0].payload["code"], "rate_limited");
        assert_eq!(rejected.frames[0].payload["retryable"], true);
        assert_eq!(
            rejected.frames[0].client_id.as_deref(),
            Some("client_submit_over_limit")
        );
        assert!(rejected.frames[0].payload["message"]
            .as_str()
            .unwrap()
            .contains("rate limit"));
    }

    #[test]
    fn unsupported_command_returns_structured_rejection_and_typing_is_ephemeral() {
        let (_temp_dir, db_path, conversation_id, participant_id) = seeded_db();
        let mut session = session(participant_id);

        let unsupported = handle_gateway_envelope(
            &db_path,
            &mut session,
            command(
                "llm.run.request",
                "client_llm_1",
                &conversation_id,
                json!({}),
            ),
        )
        .unwrap();
        assert_eq!(unsupported.frames[0].payload["code"], "unsupported_command");

        let typing = handle_gateway_envelope(
            &db_path,
            &mut session,
            command(
                "typing.start",
                "client_typing_1",
                &conversation_id,
                json!({}),
            ),
        )
        .unwrap();
        assert_eq!(typing.frames[0].op, ConversationGatewayOp::Ack);
        assert_eq!(typing.broadcast[0].frame_type, "typing.started");
        assert_eq!(
            typing.broadcast[0].durability,
            ConversationGatewayDurability::Ephemeral
        );
    }

    #[test]
    fn handoff_lifecycle_commands_ack_persist_and_replay_in_order() {
        let (_temp_dir, db_path, conversation_id, participant_id) = seeded_db();
        let mut session = session(participant_id);

        let create = handle_gateway_envelope(
            &db_path,
            &mut session,
            command(
                "conversation.handoff.create",
                "client_handoff_create",
                &conversation_id,
                json!({
                    "reason": "client needs a human decision",
                    "urgency": "high",
                    "evidenceSummary": "message msg_1 asked for staff follow-up",
                    "allowedContext": ["conversation", "latest_message"]
                }),
            ),
        )
        .unwrap();
        assert_eq!(create.frames[0].op, ConversationGatewayOp::Ack);
        assert_eq!(
            create.frames[0].client_id.as_deref(),
            Some("client_handoff_create")
        );
        assert_eq!(
            create.broadcast[0].frame_type,
            "conversation.handoff.requested"
        );
        let handoff_id = create.frames[0].payload["handoffId"].as_str().unwrap();

        let accept = handle_gateway_envelope(
            &db_path,
            &mut session,
            command(
                "conversation.handoff.accept",
                "client_handoff_accept",
                &conversation_id,
                json!({ "handoffId": handoff_id, "reason": "staff accepted" }),
            ),
        )
        .unwrap();
        assert_eq!(
            accept.frames[0].client_id.as_deref(),
            Some("client_handoff_accept")
        );
        assert_eq!(
            accept.broadcast[0].frame_type,
            "conversation.handoff.accepted"
        );

        let assign = handle_gateway_envelope(
            &db_path,
            &mut session,
            command(
                "handoff.assign",
                "client_handoff_assign",
                &conversation_id,
                json!({
                    "handoffId": handoff_id,
                    "assignedToActorId": "actor_staff",
                    "reason": "assign to current staff"
                }),
            ),
        )
        .unwrap();
        assert_eq!(
            assign.broadcast[0].frame_type,
            "conversation.handoff.assigned"
        );

        let returned = handle_gateway_envelope(
            &db_path,
            &mut session,
            command(
                "conversation.handoff.return_to_agent",
                "client_handoff_return",
                &conversation_id,
                json!({ "handoffId": handoff_id, "reason": "agent may resume" }),
            ),
        )
        .unwrap();
        assert_eq!(
            returned.broadcast[0].frame_type,
            "conversation.handoff.returned_to_agent"
        );

        let close = handle_gateway_envelope(
            &db_path,
            &mut session,
            command(
                "conversation.handoff.close",
                "client_handoff_close",
                &conversation_id,
                json!({ "handoffId": handoff_id, "reason": "complete" }),
            ),
        )
        .unwrap();
        assert_eq!(close.broadcast[0].frame_type, "conversation.handoff.closed");

        let replay = handle_gateway_envelope(
            &db_path,
            &mut session,
            command(
                "conversation.replay_after_cursor",
                "client_replay_handoff",
                &conversation_id,
                json!({ "afterSequence": 0, "limit": 50 }),
            ),
        )
        .unwrap();
        let durable_types = replay
            .frames
            .iter()
            .map(|frame| frame.frame_type.as_str())
            .collect::<Vec<_>>();
        let requested_index = durable_types
            .iter()
            .position(|event_type| *event_type == "conversation.handoff.requested")
            .unwrap();
        let closed_index = durable_types
            .iter()
            .position(|event_type| *event_type == "conversation.handoff.closed")
            .unwrap();
        assert!(requested_index < closed_index);
        assert!(durable_types.contains(&"conversation.handoff.accepted"));
        assert!(durable_types.contains(&"conversation.handoff.assigned"));
        assert!(durable_types.contains(&"conversation.handoff.returned_to_agent"));
    }

    #[test]
    fn mode_and_delegation_commands_are_durable_scoped_and_non_leaking() {
        let (_temp_dir, db_path, conversation_id, participant_id) = seeded_db();
        let mut session = session(participant_id);

        let human_led = handle_gateway_envelope(
            &db_path,
            &mut session,
            command(
                "conversation.mode.human_led_active",
                "client_mode_human",
                &conversation_id,
                json!({}),
            ),
        )
        .unwrap();
        assert_eq!(
            human_led.frames[0].client_id.as_deref(),
            Some("client_mode_human")
        );
        assert_eq!(
            human_led.broadcast[0].frame_type,
            "conversation.mode.changed"
        );
        assert_eq!(human_led.broadcast[0].payload["delegatedToAgent"], false);

        let delegate = handle_gateway_envelope(
            &db_path,
            &mut session,
            command(
                "conversation.agent.delegate",
                "client_agent_delegate",
                &conversation_id,
                json!({ "delegationScope": ["draft_reply"], "reason": "staff tagged Ordo" }),
            ),
        )
        .unwrap();
        assert_eq!(
            delegate.frames[0].client_id.as_deref(),
            Some("client_agent_delegate")
        );
        assert_eq!(delegate.broadcast[0].payload["delegatedToAgent"], true);
        assert_eq!(
            delegate.broadcast[0].payload["delegationScope"][0],
            "draft_reply"
        );
        let dispatch_json = serde_json::to_string(&delegate.broadcast[0].payload).unwrap();
        assert!(!dispatch_json.contains("provider"));
        assert!(!dispatch_json.contains("privacyTransform"));
        assert!(!dispatch_json.contains("policyDecision"));

        let revoke = handle_gateway_envelope(
            &db_path,
            &mut session,
            command(
                "conversation.agent.delegation_revoke",
                "client_agent_revoke",
                &conversation_id,
                json!({ "reason": "staff resumed manually" }),
            ),
        )
        .unwrap();
        assert_eq!(revoke.broadcast[0].payload["delegatedToAgent"], false);

        let returned = handle_gateway_envelope(
            &db_path,
            &mut session,
            command(
                "conversation.mode.return_to_agent",
                "client_mode_return",
                &conversation_id,
                json!({}),
            ),
        )
        .unwrap();
        assert_eq!(returned.broadcast[0].payload["mode"], "returned_to_agent");

        let rejected = handle_gateway_text_frame(
            &db_path,
            &mut session,
            &serde_json::to_string(&command(
                "conversation.agent.delegate",
                "client_agent_delegate_bad",
                &conversation_id,
                json!({}),
            ))
            .unwrap(),
        );
        assert_eq!(rejected.frames[0].op, ConversationGatewayOp::Error);
        assert_eq!(rejected.frames[0].payload["code"], "command_failed");
        assert!(rejected.frames[0].payload["message"]
            .as_str()
            .unwrap()
            .contains("delegationScope"));

        let replay = handle_gateway_envelope(
            &db_path,
            &mut session,
            command(
                "conversation.replay_after_cursor",
                "client_replay_mode",
                &conversation_id,
                json!({ "afterSequence": 0, "limit": 50 }),
            ),
        )
        .unwrap();
        assert!(
            replay
                .frames
                .iter()
                .filter(|frame| frame.frame_type == "conversation.mode.changed")
                .count()
                >= 4
        );
    }

    #[test]
    fn receipt_reaction_and_presence_commands_return_ack_and_expected_dispatch() {
        let (_temp_dir, db_path, conversation_id, participant_id) = seeded_db();
        let mut session = session(participant_id);

        let submit = handle_gateway_envelope(
            &db_path,
            &mut session,
            command(
                "message.submit",
                "client_submit_1",
                &conversation_id,
                json!({ "bodyMarkdown": "hello", "clientMessageId": "client_msg_1" }),
            ),
        )
        .unwrap();
        let message_id = submit.frames[0].payload["messageId"].as_str().unwrap();

        let read = handle_gateway_envelope(
            &db_path,
            &mut session,
            command(
                "message.mark_read",
                "client_read_1",
                &conversation_id,
                json!({ "messageId": message_id }),
            ),
        )
        .unwrap();
        assert_eq!(read.frames[0].client_id.as_deref(), Some("client_read_1"));
        assert_eq!(read.broadcast[0].frame_type, "message.read");

        let reaction = handle_gateway_envelope(
            &db_path,
            &mut session,
            command(
                "message.react",
                "client_react_1",
                &conversation_id,
                json!({
                    "messageId": message_id,
                    "reactionKey": "heart",
                    "reactionKind": "emoji",
                    "action": "add"
                }),
            ),
        )
        .unwrap();
        assert_eq!(
            reaction.frames[0].client_id.as_deref(),
            Some("client_react_1")
        );
        assert_eq!(reaction.broadcast[0].frame_type, "message.reaction.added");

        let before_messages: i64 = Connection::open(&db_path)
            .unwrap()
            .query_row("SELECT COUNT(*) FROM conversation_messages", [], |row| {
                row.get(0)
            })
            .unwrap();
        let presence = handle_gateway_envelope(
            &db_path,
            &mut session,
            command(
                "presence.update",
                "client_presence_1",
                &conversation_id,
                json!({ "status": "online", "visibility": "participants" }),
            ),
        )
        .unwrap();
        assert_eq!(
            presence.frames[0].client_id.as_deref(),
            Some("client_presence_1")
        );
        assert_eq!(presence.broadcast[0].frame_type, "presence.changed");
        assert_eq!(
            presence.broadcast[0].durability,
            ConversationGatewayDurability::Ephemeral
        );
        let after_messages: i64 = Connection::open(&db_path)
            .unwrap()
            .query_row("SELECT COUNT(*) FROM conversation_messages", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(before_messages, after_messages);

        let replay = handle_gateway_envelope(
            &db_path,
            &mut session,
            command(
                "conversation.replay_after_cursor",
                "client_replay_2",
                &conversation_id,
                json!({ "afterSequence": 0, "limit": 20 }),
            ),
        )
        .unwrap();
        let durable_types = replay
            .frames
            .iter()
            .map(|frame| frame.frame_type.as_str())
            .collect::<Vec<_>>();
        assert!(durable_types.contains(&"message.read"));
        assert!(durable_types.contains(&"message.reaction.added"));
        assert!(!durable_types.contains(&"presence.changed"));
    }
}
