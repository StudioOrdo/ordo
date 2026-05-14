use crate::conversation_analysis::*;
use crate::events::*;
use crate::policy::*;

pub mod core;
pub mod types;

pub use core::*;
pub use types::*;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capabilities::seed_builtin_capabilities;
    use crate::schema::init_schema;
    use chrono::{DateTime, Utc};
    use rusqlite::{Connection, Row};
    use serde_json::json;

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

    fn create_client_participant(
        connection: &Connection,
        conversation_id: &str,
    ) -> ConversationParticipantView {
        create_conversation_participant(
            connection,
            &ConversationParticipantCreateRequest {
                conversation_id: conversation_id.to_string(),
                participant_kind: "client".to_string(),
                actor_id: Some("actor_client".to_string()),
                connection_id: Some("connection_1".to_string()),
                visitor_session_id: None,
                display_name: "Client".to_string(),
                role: "client".to_string(),
            },
        )
        .unwrap()
    }

    fn create_message_from(
        connection: &Connection,
        conversation_id: &str,
        participant_id: &str,
        client_message_id: &str,
    ) -> ConversationMessageView {
        create_conversation_message(
            connection,
            &ConversationMessageCreateRequest {
                conversation_id: conversation_id.to_string(),
                segment_id: None,
                participant_id: participant_id.to_string(),
                message_kind: "human".to_string(),
                body_markdown: "hello".to_string(),
                visibility: "participants".to_string(),
                client_message_id: client_message_id.to_string(),
                reply_to_message_id: None,
                undo_expires_at: None,
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

    fn client_mutation_actor() -> ConversationMutationActor {
        ConversationMutationActor {
            actor: ActorContext::new(
                ActorKind::BrowserOperator,
                "test",
                Some("actor_client".to_string()),
            ),
            request_id: Some("request_client_1".to_string()),
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
                |row: &Row| row.get(0),
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
                |row: &Row| row.get(0),
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
                |row: &Row| row.get(0),
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
                |row: &Row| row.get(0),
            )
            .unwrap();
        let event_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM conversation_events WHERE event_type = 'message.created' AND payload_json LIKE ?1",
                [format!("%{}%", result.value.id)],
                |row: &Row| row.get(0),
            )
            .unwrap();
        assert_eq!(message_count, 1);
        assert_eq!(event_count, 1);
    }

    #[test]
    fn read_and_unread_state_updates_counts_and_events_idempotently() {
        let connection = test_connection();
        let conversation = create_conversation(&connection);
        let staff = create_participant(&connection, &conversation.id);
        let client = create_client_participant(&connection, &conversation.id);
        let message =
            create_message_from(&connection, &conversation.id, &client.id, "client_msg_1");

        let before =
            conversation_list_read_model(&connection, &conversation.id, &staff.id).unwrap();
        assert_eq!(before.read_state.unread_count, 1);
        assert_eq!(before.conversation.unread_count, 1);
        assert_eq!(before.last_message.as_ref().unwrap().id, message.id);

        let read = ConversationService::mark_read(
            &connection,
            &staff_mutation_actor(),
            &conversation.id,
            &staff.id,
            &message.id,
        )
        .unwrap();
        assert!(read.value.changed);
        assert_eq!(read.value.value.unread_count, 0);
        assert_eq!(read.value.event_type.as_deref(), Some("message.read"));

        let repeated = ConversationService::mark_read(
            &connection,
            &staff_mutation_actor(),
            &conversation.id,
            &staff.id,
            &message.id,
        )
        .unwrap();
        assert!(!repeated.value.changed);
        let read_events: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM conversation_events WHERE event_type = 'message.read'",
                [],
                |row: &Row| row.get(0),
            )
            .unwrap();
        assert_eq!(read_events, 1);

        let unread = ConversationService::mark_unread(
            &connection,
            &staff_mutation_actor(),
            &conversation.id,
            &staff.id,
            &message.id,
        )
        .unwrap();
        assert!(unread.value.changed);
        assert_eq!(unread.value.value.unread_count, 1);
        assert_eq!(
            unread.value.value.manual_unread_from_message_id.as_deref(),
            Some(message.id.as_str())
        );
        let after = conversation_list_read_model(&connection, &conversation.id, &staff.id).unwrap();
        assert_eq!(after.read_state.unread_count, 1);
        assert_eq!(after.conversation.unread_count, 1);

        let deleted = ConversationService::delete_message(
            &connection,
            &staff_mutation_actor(),
            &message.id,
            &staff.id,
            "remove stale unread fixture",
        )
        .unwrap();
        assert_eq!(deleted.value.status, "tombstoned");
        let after_delete =
            conversation_list_read_model(&connection, &conversation.id, &staff.id).unwrap();
        assert_eq!(after_delete.read_state.unread_count, 0);
        assert_eq!(after_delete.conversation.unread_count, 0);
    }

    #[test]
    fn reactions_are_idempotent_and_preserve_event_history() {
        let connection = test_connection();
        let conversation = create_conversation(&connection);
        let staff = create_participant(&connection, &conversation.id);
        let client = create_client_participant(&connection, &conversation.id);
        let message =
            create_message_from(&connection, &conversation.id, &client.id, "client_msg_1");

        let added = ConversationService::react_to_message(
            &connection,
            &staff_mutation_actor(),
            &message.id,
            &staff.id,
            "heart",
            "emoji",
            ReactionAction::Add,
        )
        .unwrap();
        assert!(added.value.changed);
        assert_eq!(
            added.value.event_type.as_deref(),
            Some("message.reaction.added")
        );

        let repeated = ConversationService::react_to_message(
            &connection,
            &staff_mutation_actor(),
            &message.id,
            &staff.id,
            "heart",
            "emoji",
            ReactionAction::Add,
        )
        .unwrap();
        assert!(!repeated.value.changed);

        let removed = ConversationService::react_to_message(
            &connection,
            &staff_mutation_actor(),
            &message.id,
            &staff.id,
            "heart",
            "emoji",
            ReactionAction::Toggle,
        )
        .unwrap();
        assert!(removed.value.changed);
        assert!(removed.value.value.removed_at.is_some());
        assert_eq!(
            removed.value.event_type.as_deref(),
            Some("message.reaction.removed")
        );

        let event_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM conversation_events
                 WHERE event_type IN ('message.reaction.added', 'message.reaction.removed')",
                [],
                |row: &Row| row.get(0),
            )
            .unwrap();
        assert_eq!(event_count, 2);
    }

    #[test]
    fn presence_snapshots_are_policy_filtered_and_do_not_create_messages() {
        let connection = test_connection();
        let conversation = create_conversation(&connection);
        let staff = create_participant(&connection, &conversation.id);
        let client = create_client_participant(&connection, &conversation.id);
        create_message_from(&connection, &conversation.id, &client.id, "client_msg_1");
        let before_messages: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM conversation_messages",
                [],
                |row: &Row| row.get(0),
            )
            .unwrap();

        let presence = ConversationService::update_presence(
            &connection,
            &client_mutation_actor(),
            &ConversationPresenceUpdateRequest {
                conversation_id: conversation.id.clone(),
                participant_id: client.id.clone(),
                status: "online".to_string(),
                visibility: "private".to_string(),
                status_message: Some("Working".to_string()),
                device_class: Some("phone".to_string()),
                expires_at: None,
            },
        )
        .unwrap();
        assert_eq!(presence.value.status, "online");
        assert_eq!(presence.value.visibility, "private");

        let staff_view =
            conversation_list_read_model(&connection, &conversation.id, &staff.id).unwrap();
        assert!(staff_view.presence.is_empty());
        let client_view =
            conversation_list_read_model(&connection, &conversation.id, &client.id).unwrap();
        assert_eq!(client_view.presence.len(), 1);
        assert_eq!(client_view.presence[0].participant_id, client.id);

        let after_messages: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM conversation_messages",
                [],
                |row: &Row| row.get(0),
            )
            .unwrap();
        assert_eq!(before_messages, after_messages);
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
                |row: &Row| row.get(0),
            )
            .unwrap();
        let denied_decisions: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM policy_decisions WHERE outcome = 'denied' AND request_id = 'request_denied'",
                [],
                |row: &Row| row.get(0),
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
                |row: &Row| row.get(0),
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
