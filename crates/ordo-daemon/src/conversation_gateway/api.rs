use anyhow::Result;
use axum::extract::ws::{Message, WebSocket};
use chrono::Utc;
use serde_json::json;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::broadcast;
use uuid::Uuid;

use crate::conversation_protocol::{
    command_rejected_error, ConversationGatewayDurability, ConversationGatewayEnvelope,
    ConversationGatewayOp, ConversationGatewayScope, CONVERSATION_GATEWAY_SCHEMA_VERSION,
};

pub(crate) const MAX_TEXT_FRAME_BYTES: usize = 64 * 1024;

use super::handlers::*;
use super::types::*;

/// Generates the initial HELLO frame containing schema version,
/// heartbeat interval, and connectivity routing info.
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

/// Main entry point for a conversation gateway WebSocket connection.
/// Handles connection lifecycle, heartbeat management, message broadcasting,
/// and sequential frame routing to internal handlers.
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
                        let db_path_for_task = Arc::clone(&db_path);
                        let mut session_for_task = session.clone();
                        let output = match tokio::task::spawn_blocking(move || {
                            let output = handle_gateway_text_frame(
                                db_path_for_task.as_ref(),
                                &mut session_for_task,
                                &text,
                            );
                            (output, session_for_task)
                        })
                        .await
                        {
                            Ok((output, updated_session)) => {
                                session = updated_session;
                                output
                            }
                            Err(error) => error_output(command_rejected_error(
                                None,
                                None,
                                "command_failed",
                                &error.to_string(),
                                false,
                                &Utc::now().to_rfc3339(),
                            )),
                        };
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

/// Entry point for parsing and dispatching a raw JSON text frame
/// received over the WebSocket. Verifies schema and payload limits.
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

/// Routes a fully parsed `ConversationGatewayEnvelope` to the appropriate
/// capability or command handler based on its operation opcode (`op`).
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
