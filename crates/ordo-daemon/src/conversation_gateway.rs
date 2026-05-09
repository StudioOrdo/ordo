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
    ConversationMessageCreateRequest, ConversationMutationActor, ConversationService,
};
use crate::policy::{ActorContext, ActorKind};

const DEFAULT_REPLAY_LIMIT: usize = 100;
const MAX_REPLAY_LIMIT: usize = 500;
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
                        let frame = command_rejected_error(
                            None,
                            None,
                            "client_lagged",
                            &format!("Conversation gateway client skipped {skipped} frame(s)."),
                            true,
                            &Utc::now().to_rfc3339(),
                        );
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
            enforce_message_command_rate_limit(session)?;
            message_submit(db_path, session, envelope)
        }
        "message.edit" => {
            enforce_message_command_rate_limit(session)?;
            message_edit(db_path, session, envelope)
        }
        "message.delete" => {
            enforce_message_command_rate_limit(session)?;
            message_delete(db_path, session, envelope)
        }
        "message.undo" => {
            enforce_message_command_rate_limit(session)?;
            message_undo(db_path, session, envelope)
        }
        "typing.start" | "typing.stop" => typing(session, envelope),
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

fn latest_message_event(
    db_path: &Path,
    conversation_id: &str,
    message_id: &str,
    event_type: &str,
) -> Result<ConversationGatewayEnvelope> {
    let connection = Connection::open(db_path)?;
    let like_message = format!("%{message_id}%");
    connection
        .query_row(
            "SELECT sequence, event_type, payload_json, realtime_cursor, occurred_at
             FROM conversation_events
             WHERE conversation_id = ?1
               AND event_type = ?2
               AND payload_json LIKE ?3
             ORDER BY sequence DESC
             LIMIT 1",
            params![conversation_id, event_type, like_message],
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
        )
        .optional()?
        .ok_or_else(|| {
            anyhow::anyhow!(
                "durable command completed without replayable {event_type} event for {message_id}"
            )
        })
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

fn enforce_message_command_rate_limit(session: &mut ConversationGatewaySession) -> Result<()> {
    let now = Utc::now();
    let floor = now - ChronoDuration::seconds(COMMAND_RATE_WINDOW_SECONDS);
    while session
        .recent_message_commands
        .front()
        .is_some_and(|timestamp| *timestamp < floor)
    {
        session.recent_message_commands.pop_front();
    }
    ensure!(
        session.recent_message_commands.len() < COMMAND_RATE_LIMIT,
        "message command rate limit exceeded"
    );
    session.recent_message_commands.push_back(now);
    Ok(())
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
}
