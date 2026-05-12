use anyhow::{bail, ensure, Result};
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::path::Path;

use crate::conversation_protocol::{
    CONVERSATION_GATEWAY_ROUTE, CONVERSATION_GATEWAY_SCHEMA_VERSION,
};
use crate::conversations::core::load_participant;
use crate::conversations::{
    create_conversation_participant, find_or_create_canonical_conversation,
    CanonicalConversationRequest, ConversationParticipantCreateRequest,
};
use crate::events::{system_event, RealtimeEvent};

const CHAT_BOOTSTRAP_SCHEMA_VERSION: &str = "ordo.chat-bootstrap.v1";

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatBootstrapRequest {
    pub session_id: String,
    pub actor_id: String,
}

#[derive(Debug, Clone)]
struct LocalSessionRecord {
    actor_id: String,
    display_name: String,
    role: String,
    expires_at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatBootstrapTransport {
    pub route: String,
    pub protocol: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatBootstrapView {
    pub schema_version: String,
    pub actor_id: String,
    pub conversation_id: String,
    pub participant_id: String,
    pub assistant_participant_id: String,
    pub transport: ChatBootstrapTransport,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatBootstrapResponse {
    pub bootstrap: ChatBootstrapView,
}

pub fn bootstrap_local_chat(
    db_path: &Path,
    request: ChatBootstrapRequest,
) -> Result<(ChatBootstrapResponse, RealtimeEvent)> {
    validate_request_id("sessionId", &request.session_id, "local_session_")?;
    validate_request_id("actorId", &request.actor_id, "actor_local_member_")?;

    let connection = Connection::open(db_path)?;
    let local_session = read_active_local_session(&connection, &request)?;
    ensure!(
        local_session.role == "client",
        "Local session is not available for member chat."
    );

    let conversation = find_or_create_canonical_conversation(
        &connection,
        &CanonicalConversationRequest {
            surface: "member_ordo".to_string(),
            subject_kind: "local_actor".to_string(),
            subject_id: local_session.actor_id.clone(),
            connection_id: None,
            visitor_session_id: None,
            created_by_actor_id: Some(local_session.actor_id.clone()),
        },
    )?;
    let participant =
        find_or_create_actor_participant(&connection, &conversation.id, &local_session)?;
    let assistant = find_or_create_assistant_participant(&connection, &conversation.id)?;

    let bootstrap = ChatBootstrapView {
        schema_version: CHAT_BOOTSTRAP_SCHEMA_VERSION.to_string(),
        actor_id: local_session.actor_id,
        conversation_id: conversation.id,
        participant_id: participant.id,
        assistant_participant_id: assistant.id,
        transport: ChatBootstrapTransport {
            route: CONVERSATION_GATEWAY_ROUTE.to_string(),
            protocol: CONVERSATION_GATEWAY_SCHEMA_VERSION.to_string(),
        },
    };
    let event = system_event(
        "local_chat.bootstrap.established",
        json!({
            "actorId": bootstrap.actor_id,
            "conversationId": bootstrap.conversation_id,
            "participantId": bootstrap.participant_id,
            "assistantParticipantId": bootstrap.assistant_participant_id,
            "transportRoute": bootstrap.transport.route,
        }),
    );

    Ok((ChatBootstrapResponse { bootstrap }, event))
}

fn validate_request_id(field: &str, value: &str, expected_prefix: &str) -> Result<()> {
    if value.len() > 96
        || !value.starts_with(expected_prefix)
        || value.chars().any(char::is_whitespace)
    {
        bail!("Invalid {field} for chat bootstrap.");
    }
    Ok(())
}

fn read_active_local_session(
    connection: &Connection,
    request: &ChatBootstrapRequest,
) -> Result<LocalSessionRecord> {
    let Some(record) = connection
        .query_row(
            "SELECT actor_id, display_name, role, expires_at
             FROM local_account_sessions
             WHERE session_id = ?1 AND actor_id = ?2",
            params![request.session_id, request.actor_id],
            |row| {
                Ok(LocalSessionRecord {
                    actor_id: row.get(0)?,
                    display_name: row.get(1)?,
                    role: row.get(2)?,
                    expires_at: row.get(3)?,
                })
            },
        )
        .optional()?
    else {
        bail!("Local session is not available for chat bootstrap.");
    };
    let expires_at = DateTime::parse_from_rfc3339(&record.expires_at)
        .map_err(|_| anyhow::anyhow!("Local session is not available for chat bootstrap."))?
        .with_timezone(&Utc);
    if expires_at <= Utc::now() {
        bail!("Local session is not available for chat bootstrap.");
    }
    Ok(record)
}

fn find_or_create_actor_participant(
    connection: &Connection,
    conversation_id: &str,
    local_session: &LocalSessionRecord,
) -> Result<crate::conversations::ConversationParticipantView> {
    if let Some(participant_id) = connection
        .query_row(
            "SELECT id
             FROM conversation_participants
             WHERE conversation_id = ?1 AND actor_id = ?2 AND role = 'client' AND status = 'active'
             ORDER BY joined_at DESC
             LIMIT 1",
            params![conversation_id, local_session.actor_id],
            |row| row.get::<_, String>(0),
        )
        .optional()?
    {
        return load_participant(connection, &participant_id);
    }

    create_conversation_participant(
        connection,
        &ConversationParticipantCreateRequest {
            conversation_id: conversation_id.to_string(),
            participant_kind: "client".to_string(),
            actor_id: Some(local_session.actor_id.clone()),
            connection_id: None,
            visitor_session_id: None,
            display_name: local_session.display_name.clone(),
            role: "client".to_string(),
        },
    )
}

fn find_or_create_assistant_participant(
    connection: &Connection,
    conversation_id: &str,
) -> Result<crate::conversations::ConversationParticipantView> {
    if let Some(participant_id) = connection
        .query_row(
            "SELECT id
             FROM conversation_participants
             WHERE conversation_id = ?1 AND participant_kind = 'assistant' AND role = 'assistant' AND status = 'active'
             ORDER BY joined_at DESC
             LIMIT 1",
            [conversation_id],
            |row| row.get::<_, String>(0),
        )
        .optional()?
    {
        return load_participant(connection, &participant_id);
    }

    create_conversation_participant(
        connection,
        &ConversationParticipantCreateRequest {
            conversation_id: conversation_id.to_string(),
            participant_kind: "assistant".to_string(),
            actor_id: None,
            connection_id: None,
            visitor_session_id: None,
            display_name: "Ordo".to_string(),
            role: "assistant".to_string(),
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::local_sessions::{create_or_restore_local_session, LocalSessionCreateRequest};
    use crate::schema::init_database;
    use tempfile::tempdir;

    fn test_session(db_path: &Path) -> ChatBootstrapRequest {
        let (response, _) = create_or_restore_local_session(
            db_path,
            LocalSessionCreateRequest {
                mode: "register".to_string(),
                name: Some("Ava Client".to_string()),
                email: "ava@example.com".to_string(),
                password: "local-only-pass".to_string(),
            },
        )
        .unwrap();
        ChatBootstrapRequest {
            session_id: response.session.session_id,
            actor_id: response.session.actor_id,
        }
    }

    #[test]
    fn chat_bootstrap_creates_stable_conversation_and_participants() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("ordo.sqlite3");
        init_database(&db_path).unwrap();
        let request = test_session(&db_path);

        let (first, event) = bootstrap_local_chat(&db_path, request.clone()).unwrap();
        let (second, _) = bootstrap_local_chat(&db_path, request).unwrap();

        assert_eq!(event.event_type, "local_chat.bootstrap.established");
        assert_eq!(first.bootstrap.actor_id, second.bootstrap.actor_id);
        assert_eq!(
            first.bootstrap.conversation_id,
            second.bootstrap.conversation_id
        );
        assert_eq!(
            first.bootstrap.participant_id,
            second.bootstrap.participant_id
        );
        assert_eq!(
            first.bootstrap.assistant_participant_id,
            second.bootstrap.assistant_participant_id
        );
        assert_eq!(first.bootstrap.transport.route, "/chat/ws");

        let connection = Connection::open(&db_path).unwrap();
        let conversation_count: i64 = connection
            .query_row("SELECT COUNT(*) FROM conversations", [], |row| row.get(0))
            .unwrap();
        let participant_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM conversation_participants WHERE conversation_id = ?1",
                [&first.bootstrap.conversation_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(conversation_count, 1);
        assert_eq!(participant_count, 2);
    }

    #[test]
    fn chat_bootstrap_recreates_missing_assistant_without_duplicate_conversation() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("ordo.sqlite3");
        init_database(&db_path).unwrap();
        let request = test_session(&db_path);

        let (first, _) = bootstrap_local_chat(&db_path, request.clone()).unwrap();
        let connection = Connection::open(&db_path).unwrap();
        connection
            .execute(
                "DELETE FROM conversation_participants WHERE id = ?1",
                [&first.bootstrap.assistant_participant_id],
            )
            .unwrap();

        let (second, _) = bootstrap_local_chat(&db_path, request).unwrap();
        assert_eq!(
            first.bootstrap.conversation_id,
            second.bootstrap.conversation_id
        );
        assert_ne!(
            first.bootstrap.assistant_participant_id,
            second.bootstrap.assistant_participant_id
        );

        let conversation_count: i64 = connection
            .query_row("SELECT COUNT(*) FROM conversations", [], |row| row.get(0))
            .unwrap();
        let assistant_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM conversation_participants WHERE conversation_id = ?1 AND participant_kind = 'assistant'",
                [&second.bootstrap.conversation_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(conversation_count, 1);
        assert_eq!(assistant_count, 1);
    }

    #[test]
    fn chat_bootstrap_rejects_unknown_or_malformed_session_safely() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("ordo.sqlite3");
        init_database(&db_path).unwrap();

        let malformed = bootstrap_local_chat(
            &db_path,
            ChatBootstrapRequest {
                session_id: "bad session id".to_string(),
                actor_id: "actor_local_member_secret".to_string(),
            },
        )
        .unwrap_err()
        .to_string();
        assert!(!malformed.contains("secret"));

        let unknown = bootstrap_local_chat(
            &db_path,
            ChatBootstrapRequest {
                session_id: "local_session_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string(),
                actor_id: "actor_local_member_bbbbbbbbbbbbbbbb".to_string(),
            },
        )
        .unwrap_err()
        .to_string();
        assert!(!unknown.contains("aaaaaaaa"));
        assert!(!unknown.contains("bbbbbbbb"));
    }
}
