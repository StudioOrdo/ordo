use super::*;
use crate::schema::db::ConnectionExt;
use anyhow::bail;
use anyhow::{ensure, Result};
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension, Row, Transaction};
use serde_json::{json, Value};
use uuid::Uuid;

pub struct ConversationService;

pub(crate) struct ConversationMutationPolicyTarget<'a> {
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

    pub fn mark_read(
        connection: &Connection,
        actor: &ConversationMutationActor,
        conversation_id: &str,
        participant_id: &str,
        message_id: &str,
    ) -> Result<ConversationMutationReceipt<ConversationMutationOutcome<ConversationReadStateView>>>
    {
        let policy_decision_id = authorize_participant_mutation(
            connection,
            actor,
            ConversationMutationPolicyTarget {
                conversation_id,
                participant_id,
                action: PolicyAction::Update,
                capability_id: "conversation.receipt.write",
                resource_kind: ResourceKind::ConversationMessage,
                resource_id: message_id,
            },
        )?;
        let value =
            mark_conversation_read(connection, conversation_id, participant_id, message_id)?;
        Ok(ConversationMutationReceipt {
            value,
            policy_decision_id,
        })
    }

    pub fn mark_unread(
        connection: &Connection,
        actor: &ConversationMutationActor,
        conversation_id: &str,
        participant_id: &str,
        message_id: &str,
    ) -> Result<ConversationMutationReceipt<ConversationMutationOutcome<ConversationReadStateView>>>
    {
        let policy_decision_id = authorize_participant_mutation(
            connection,
            actor,
            ConversationMutationPolicyTarget {
                conversation_id,
                participant_id,
                action: PolicyAction::Update,
                capability_id: "conversation.receipt.write",
                resource_kind: ResourceKind::ConversationMessage,
                resource_id: message_id,
            },
        )?;
        let value =
            mark_conversation_unread(connection, conversation_id, participant_id, message_id)?;
        Ok(ConversationMutationReceipt {
            value,
            policy_decision_id,
        })
    }

    pub fn react_to_message(
        connection: &Connection,
        actor: &ConversationMutationActor,
        message_id: &str,
        participant_id: &str,
        reaction_key: &str,
        reaction_kind: &str,
        action: ReactionAction,
    ) -> Result<ConversationMutationReceipt<ConversationMutationOutcome<ConversationReactionView>>>
    {
        let current = load_message(connection, message_id)?;
        let policy_decision_id = authorize_participant_mutation(
            connection,
            actor,
            ConversationMutationPolicyTarget {
                conversation_id: &current.conversation_id,
                participant_id,
                action: PolicyAction::Update,
                capability_id: "conversation.reaction.write",
                resource_kind: ResourceKind::ConversationMessage,
                resource_id: message_id,
            },
        )?;
        let value = react_to_conversation_message(
            connection,
            message_id,
            participant_id,
            reaction_key,
            reaction_kind,
            action,
        )?;
        Ok(ConversationMutationReceipt {
            value,
            policy_decision_id,
        })
    }

    pub fn update_presence(
        connection: &Connection,
        actor: &ConversationMutationActor,
        request: &ConversationPresenceUpdateRequest,
    ) -> Result<ConversationMutationReceipt<ConversationPresenceSnapshotView>> {
        let policy_decision_id = authorize_participant_mutation(
            connection,
            actor,
            ConversationMutationPolicyTarget {
                conversation_id: &request.conversation_id,
                participant_id: &request.participant_id,
                action: PolicyAction::Update,
                capability_id: "conversation.presence.write",
                resource_kind: ResourceKind::ConversationParticipant,
                resource_id: &request.participant_id,
            },
        )?;
        let value = update_conversation_presence(connection, request)?;
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
    connection.query_many(
        "SELECT id, surface, subject_kind, subject_id, connection_id, status, unread_count,
                action_count, last_meaningful_change, updated_at
         FROM conversations
         WHERE subject_kind = ?1 AND subject_id = ?2 AND archived_at IS NULL
         ORDER BY updated_at DESC",
        params![subject_kind, subject_id],
        conversation_summary_from_row,
    )
}

pub fn staff_episode_details(
    connection: &Connection,
    conversation_id: &str,
) -> Result<Vec<ConversationSegmentView>> {
    require_text("conversation_id", conversation_id)?;
    connection.query_many(
        "SELECT id, conversation_id, title, segment_kind, status, candidate_state, confidence,
                evidence_refs_json, provenance_json, created_by_job_id, source_kind, source_id,
                created_at, updated_at
         FROM conversation_segments
         WHERE conversation_id = ?1
         ORDER BY started_at DESC",
        [conversation_id],
        segment_from_row,
    )
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
    connection.execute(
        "INSERT INTO conversation_read_states (
            conversation_id, participant_id, unread_count, unread_mentions_count,
            unread_action_count, updated_at
         ) VALUES (?1, ?2, 0, 0, 0, ?3)
         ON CONFLICT(conversation_id, participant_id) DO NOTHING",
        params![request.conversation_id, participant_id, now],
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
    transaction.execute(
        "UPDATE conversation_read_states
         SET unread_count = unread_count + 1,
             updated_at = ?1
         WHERE conversation_id = ?2
           AND participant_id != ?3",
        params![now, request.conversation_id, request.participant_id],
    )?;
    update_conversation_unread_count_tx(&transaction, &request.conversation_id)?;
    transaction.commit()?;

    let message = load_message(connection, &message_id)?;
    let _ = queue_analysis_for_message(connection, &message)?;
    Ok(message)
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
    update_read_counts_for_conversation_tx(&transaction, &current.conversation_id)?;
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
    update_read_counts_for_conversation_tx(&transaction, &current.conversation_id)?;
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
    update_read_counts_for_conversation_tx(&transaction, &current.conversation_id)?;
    transaction.commit()?;

    load_message(connection, message_id)
}

pub fn mark_conversation_read(
    connection: &Connection,
    conversation_id: &str,
    participant_id: &str,
    message_id: &str,
) -> Result<ConversationMutationOutcome<ConversationReadStateView>> {
    require_text("conversation_id", conversation_id)?;
    require_text("participant_id", participant_id)?;
    require_text("message_id", message_id)?;
    let participant = load_participant(connection, participant_id)?;
    ensure!(
        participant.conversation_id == conversation_id,
        "participant does not belong to conversation"
    );
    let message = load_message(connection, message_id)?;
    ensure!(
        message.conversation_id == conversation_id,
        "message does not belong to conversation"
    );
    let previous = optional_read_state(connection, conversation_id, participant_id)?;
    if previous.as_ref().is_some_and(|state| {
        state.last_read_event_cursor.unwrap_or(0) >= message.event_cursor.unwrap_or(0)
            && state.manual_unread_from_message_id.is_none()
    }) {
        return Ok(ConversationMutationOutcome {
            value: previous.unwrap(),
            event_type: None,
            changed: false,
        });
    }

    let unread_count = unread_count_after_sequence(
        connection,
        conversation_id,
        participant_id,
        message.sequence,
    )?;
    let now = Utc::now().to_rfc3339();
    let transaction = connection.unchecked_transaction()?;
    transaction.execute(
        "INSERT INTO conversation_receipts (
            id, conversation_id, message_id, participant_id, receipt_kind, event_cursor,
            sequence, payload_json, created_at
         ) VALUES (?1, ?2, ?3, ?4, 'read', ?5, ?6, ?7, ?8)",
        params![
            format!("receipt_{}", Uuid::new_v4()),
            conversation_id,
            message_id,
            participant_id,
            message.event_cursor,
            message.sequence,
            json!({ "unreadCount": unread_count }).to_string(),
            now
        ],
    )?;
    transaction.execute(
        "INSERT INTO conversation_read_states (
            conversation_id, participant_id, last_read_message_id, last_read_event_cursor,
            last_read_at, manual_unread_from_message_id, unread_count, unread_mentions_count,
            unread_action_count, updated_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, NULL, ?6, 0, 0, ?5)
         ON CONFLICT(conversation_id, participant_id) DO UPDATE SET
            last_read_message_id = excluded.last_read_message_id,
            last_read_event_cursor = excluded.last_read_event_cursor,
            last_read_at = excluded.last_read_at,
            manual_unread_from_message_id = NULL,
            unread_count = excluded.unread_count,
            unread_mentions_count = 0,
            unread_action_count = 0,
            updated_at = excluded.updated_at",
        params![
            conversation_id,
            participant_id,
            message_id,
            message.event_cursor,
            now,
            unread_count
        ],
    )?;
    append_conversation_event_tx(
        &transaction,
        conversation_id,
        message.segment_id.as_deref(),
        None,
        "message.read",
        json!({
            "messageId": message_id,
            "participantId": participant_id,
            "unreadCount": unread_count,
        }),
        None,
    )?;
    update_conversation_unread_count_tx(&transaction, conversation_id)?;
    transaction.commit()?;

    Ok(ConversationMutationOutcome {
        value: load_read_state(connection, conversation_id, participant_id)?,
        event_type: Some("message.read".to_string()),
        changed: true,
    })
}

pub fn mark_conversation_unread(
    connection: &Connection,
    conversation_id: &str,
    participant_id: &str,
    message_id: &str,
) -> Result<ConversationMutationOutcome<ConversationReadStateView>> {
    require_text("conversation_id", conversation_id)?;
    require_text("participant_id", participant_id)?;
    require_text("message_id", message_id)?;
    let participant = load_participant(connection, participant_id)?;
    ensure!(
        participant.conversation_id == conversation_id,
        "participant does not belong to conversation"
    );
    let message = load_message(connection, message_id)?;
    ensure!(
        message.conversation_id == conversation_id,
        "message does not belong to conversation"
    );
    let previous = optional_read_state(connection, conversation_id, participant_id)?;
    if previous
        .as_ref()
        .is_some_and(|state| state.manual_unread_from_message_id.as_deref() == Some(message_id))
    {
        return Ok(ConversationMutationOutcome {
            value: previous.unwrap(),
            event_type: None,
            changed: false,
        });
    }

    let unread_count = unread_count_from_sequence(
        connection,
        conversation_id,
        participant_id,
        message.sequence,
    )?;
    let now = Utc::now().to_rfc3339();
    let transaction = connection.unchecked_transaction()?;
    transaction.execute(
        "INSERT INTO conversation_receipts (
            id, conversation_id, message_id, participant_id, receipt_kind, event_cursor,
            sequence, payload_json, created_at
         ) VALUES (?1, ?2, ?3, ?4, 'unread', ?5, ?6, ?7, ?8)",
        params![
            format!("receipt_{}", Uuid::new_v4()),
            conversation_id,
            message_id,
            participant_id,
            message.event_cursor,
            message.sequence,
            json!({ "unreadCount": unread_count }).to_string(),
            now
        ],
    )?;
    transaction.execute(
        "INSERT INTO conversation_read_states (
            conversation_id, participant_id, manual_unread_from_message_id, unread_count,
            unread_mentions_count, unread_action_count, updated_at
         ) VALUES (?1, ?2, ?3, ?4, 0, 0, ?5)
         ON CONFLICT(conversation_id, participant_id) DO UPDATE SET
            manual_unread_from_message_id = excluded.manual_unread_from_message_id,
            unread_count = excluded.unread_count,
            unread_mentions_count = 0,
            unread_action_count = 0,
            updated_at = excluded.updated_at",
        params![
            conversation_id,
            participant_id,
            message_id,
            unread_count,
            now
        ],
    )?;
    append_conversation_event_tx(
        &transaction,
        conversation_id,
        message.segment_id.as_deref(),
        None,
        "message.marked_unread",
        json!({
            "messageId": message_id,
            "participantId": participant_id,
            "unreadCount": unread_count,
        }),
        None,
    )?;
    update_conversation_unread_count_tx(&transaction, conversation_id)?;
    transaction.commit()?;

    Ok(ConversationMutationOutcome {
        value: load_read_state(connection, conversation_id, participant_id)?,
        event_type: Some("message.marked_unread".to_string()),
        changed: true,
    })
}

pub fn react_to_conversation_message(
    connection: &Connection,
    message_id: &str,
    participant_id: &str,
    reaction_key: &str,
    reaction_kind: &str,
    action: ReactionAction,
) -> Result<ConversationMutationOutcome<ConversationReactionView>> {
    require_text("message_id", message_id)?;
    require_text("participant_id", participant_id)?;
    require_text("reaction_key", reaction_key)?;
    require_text("reaction_kind", reaction_kind)?;
    let message = load_message(connection, message_id)?;
    let participant = load_participant(connection, participant_id)?;
    ensure!(
        participant.conversation_id == message.conversation_id,
        "participant does not belong to message conversation"
    );
    let active = active_reaction(connection, message_id, participant_id, reaction_key)?;
    match (action, active) {
        (ReactionAction::Add, Some(reaction)) => Ok(ConversationMutationOutcome {
            value: reaction,
            event_type: None,
            changed: false,
        }),
        (ReactionAction::Remove, None) => Ok(ConversationMutationOutcome {
            value: removed_reaction_placeholder(
                message_id,
                participant_id,
                reaction_key,
                reaction_kind,
            ),
            event_type: None,
            changed: false,
        }),
        (ReactionAction::Toggle, Some(reaction)) | (ReactionAction::Remove, Some(reaction)) => {
            let now = Utc::now().to_rfc3339();
            let transaction = connection.unchecked_transaction()?;
            transaction.execute(
                "UPDATE conversation_reactions SET removed_at = ?1 WHERE id = ?2",
                params![now, reaction.id],
            )?;
            append_conversation_event_tx(
                &transaction,
                &message.conversation_id,
                message.segment_id.as_deref(),
                None,
                "message.reaction.removed",
                json!({
                    "messageId": message_id,
                    "participantId": participant_id,
                    "reactionKey": reaction_key,
                    "reactionKind": reaction.reaction_kind,
                }),
                None,
            )?;
            transaction.commit()?;
            Ok(ConversationMutationOutcome {
                value: load_reaction(connection, &reaction.id)?,
                event_type: Some("message.reaction.removed".to_string()),
                changed: true,
            })
        }
        (ReactionAction::Add, None) | (ReactionAction::Toggle, None) => {
            let now = Utc::now().to_rfc3339();
            let reaction_id = format!("reaction_{}", Uuid::new_v4());
            let transaction = connection.unchecked_transaction()?;
            transaction.execute(
                "INSERT INTO conversation_reactions (
                    id, message_id, participant_id, reaction_key, reaction_kind, metadata_json, created_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5, '{}', ?6)",
                params![
                    reaction_id,
                    message_id,
                    participant_id,
                    reaction_key,
                    reaction_kind,
                    now
                ],
            )?;
            append_conversation_event_tx(
                &transaction,
                &message.conversation_id,
                message.segment_id.as_deref(),
                None,
                "message.reaction.added",
                json!({
                    "messageId": message_id,
                    "participantId": participant_id,
                    "reactionKey": reaction_key,
                    "reactionKind": reaction_kind,
                }),
                None,
            )?;
            transaction.commit()?;
            Ok(ConversationMutationOutcome {
                value: load_reaction(connection, &reaction_id)?,
                event_type: Some("message.reaction.added".to_string()),
                changed: true,
            })
        }
    }
}

pub fn update_conversation_presence(
    connection: &Connection,
    request: &ConversationPresenceUpdateRequest,
) -> Result<ConversationPresenceSnapshotView> {
    require_text("conversation_id", &request.conversation_id)?;
    require_text("participant_id", &request.participant_id)?;
    require_text("status", &request.status)?;
    require_text("visibility", &request.visibility)?;
    let participant = load_participant(connection, &request.participant_id)?;
    ensure!(
        participant.conversation_id == request.conversation_id,
        "participant does not belong to conversation"
    );
    ensure!(
        matches!(
            request.visibility.as_str(),
            "public" | "participants" | "private"
        ),
        "unsupported presence visibility"
    );
    let now = Utc::now().to_rfc3339();
    connection.execute(
        "INSERT INTO conversation_presence_snapshots (
            participant_id, conversation_id, status, visibility, status_message, device_class,
            metadata_json, updated_at, expires_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, '{}', ?7, ?8)
         ON CONFLICT(participant_id) DO UPDATE SET
            conversation_id = excluded.conversation_id,
            status = excluded.status,
            visibility = excluded.visibility,
            status_message = excluded.status_message,
            device_class = excluded.device_class,
            updated_at = excluded.updated_at,
            expires_at = excluded.expires_at",
        params![
            request.participant_id,
            request.conversation_id,
            request.status,
            request.visibility,
            request.status_message,
            request.device_class,
            now,
            request.expires_at
        ],
    )?;
    connection.execute(
        "UPDATE conversation_participants SET last_seen_at = ?1 WHERE id = ?2",
        params![now, request.participant_id],
    )?;
    load_presence_snapshot(connection, &request.participant_id)
}

pub fn conversation_list_read_model(
    connection: &Connection,
    conversation_id: &str,
    participant_id: &str,
) -> Result<ConversationListReadModel> {
    let conversation = load_conversation_summary(connection, conversation_id)?;
    let read_state = load_or_create_read_state(connection, conversation_id, participant_id)?;
    let last_message = latest_visible_message(connection, conversation_id)?;
    let presence = list_presence_snapshots(connection, conversation_id, participant_id)?;
    Ok(ConversationListReadModel {
        conversation,
        participant_id: participant_id.to_string(),
        last_message,
        read_state,
        presence,
    })
}

pub(crate) fn load_conversation_summary(
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

pub(crate) fn load_segment(
    connection: &Connection,
    segment_id: &str,
) -> Result<ConversationSegmentView> {
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

pub(crate) fn load_handoff(
    connection: &Connection,
    handoff_id: &str,
) -> Result<ConversationHandoffView> {
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

pub(crate) fn load_conversation_mode(
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

pub(crate) fn load_participant(
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

pub(crate) fn load_message(
    connection: &Connection,
    message_id: &str,
) -> Result<ConversationMessageView> {
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

pub(crate) fn load_or_create_read_state(
    connection: &Connection,
    conversation_id: &str,
    participant_id: &str,
) -> Result<ConversationReadStateView> {
    match optional_read_state(connection, conversation_id, participant_id)? {
        Some(state) => Ok(state),
        None => {
            let now = Utc::now().to_rfc3339();
            connection.execute(
                "INSERT INTO conversation_read_states (
                    conversation_id, participant_id, unread_count, unread_mentions_count,
                    unread_action_count, updated_at
                 ) VALUES (?1, ?2, 0, 0, 0, ?3)",
                params![conversation_id, participant_id, now],
            )?;
            load_read_state(connection, conversation_id, participant_id)
        }
    }
}

pub(crate) fn load_read_state(
    connection: &Connection,
    conversation_id: &str,
    participant_id: &str,
) -> Result<ConversationReadStateView> {
    connection
        .query_row(
            "SELECT conversation_id, participant_id, last_read_message_id,
                    last_read_event_cursor, last_read_at, manual_unread_from_message_id,
                    unread_count, unread_mentions_count, unread_action_count, updated_at
             FROM conversation_read_states
             WHERE conversation_id = ?1 AND participant_id = ?2",
            params![conversation_id, participant_id],
            read_state_from_row,
        )
        .map_err(Into::into)
}

pub(crate) fn optional_read_state(
    connection: &Connection,
    conversation_id: &str,
    participant_id: &str,
) -> Result<Option<ConversationReadStateView>> {
    connection
        .query_row(
            "SELECT conversation_id, participant_id, last_read_message_id,
                    last_read_event_cursor, last_read_at, manual_unread_from_message_id,
                    unread_count, unread_mentions_count, unread_action_count, updated_at
             FROM conversation_read_states
             WHERE conversation_id = ?1 AND participant_id = ?2",
            params![conversation_id, participant_id],
            read_state_from_row,
        )
        .optional()
        .map_err(Into::into)
}

pub(crate) fn unread_count_after_sequence(
    connection: &Connection,
    conversation_id: &str,
    participant_id: &str,
    sequence: i64,
) -> Result<i64> {
    unread_count_with_predicate(
        connection,
        conversation_id,
        participant_id,
        "sequence > ?3",
        sequence,
    )
}

pub(crate) fn unread_count_from_sequence(
    connection: &Connection,
    conversation_id: &str,
    participant_id: &str,
    sequence: i64,
) -> Result<i64> {
    unread_count_with_predicate(
        connection,
        conversation_id,
        participant_id,
        "sequence >= ?3",
        sequence,
    )
}

pub(crate) fn unread_count_with_predicate(
    connection: &Connection,
    conversation_id: &str,
    participant_id: &str,
    predicate: &str,
    sequence: i64,
) -> Result<i64> {
    let sql = format!(
        "SELECT COUNT(*)
         FROM conversation_messages
         WHERE conversation_id = ?1
           AND participant_id != ?2
           AND status NOT IN ('cancelled', 'tombstoned')
           AND {predicate}"
    );
    connection
        .query_row(
            &sql,
            params![conversation_id, participant_id, sequence],
            |row| row.get(0),
        )
        .map_err(Into::into)
}

pub(crate) fn update_conversation_unread_count_tx(
    transaction: &Transaction<'_>,
    conversation_id: &str,
) -> Result<()> {
    let unread_count: i64 = transaction.query_row(
        "SELECT COALESCE(MAX(unread_count), 0)
         FROM conversation_read_states
         WHERE conversation_id = ?1",
        [conversation_id],
        |row| row.get(0),
    )?;
    transaction.execute(
        "UPDATE conversations
         SET unread_count = ?1, updated_at = ?2
         WHERE id = ?3",
        params![unread_count, Utc::now().to_rfc3339(), conversation_id],
    )?;
    Ok(())
}

pub(crate) fn update_read_counts_for_conversation_tx(
    transaction: &Transaction<'_>,
    conversation_id: &str,
) -> Result<()> {
    let states = {
        transaction.query_many(
            "SELECT participant_id, last_read_message_id, manual_unread_from_message_id
             FROM conversation_read_states
             WHERE conversation_id = ?1",
            [conversation_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, Option<String>>(1)?,
                    row.get::<_, Option<String>>(2)?,
                ))
            },
        )?
    };

    for (participant_id, last_read_message_id, manual_unread_from_message_id) in states {
        let unread_count = if let Some(message_id) = manual_unread_from_message_id {
            let sequence = message_sequence_tx(transaction, &message_id)?;
            unread_count_for_state_tx(
                transaction,
                conversation_id,
                &participant_id,
                "sequence >= ?3",
                sequence,
            )?
        } else if let Some(message_id) = last_read_message_id {
            let sequence = message_sequence_tx(transaction, &message_id)?;
            unread_count_for_state_tx(
                transaction,
                conversation_id,
                &participant_id,
                "sequence > ?3",
                sequence,
            )?
        } else {
            unread_count_for_state_tx(
                transaction,
                conversation_id,
                &participant_id,
                "sequence >= ?3",
                0,
            )?
        };
        transaction.execute(
            "UPDATE conversation_read_states
             SET unread_count = ?1, unread_mentions_count = 0, unread_action_count = 0,
                 updated_at = ?2
             WHERE conversation_id = ?3 AND participant_id = ?4",
            params![
                unread_count,
                Utc::now().to_rfc3339(),
                conversation_id,
                participant_id
            ],
        )?;
    }
    update_conversation_unread_count_tx(transaction, conversation_id)
}

pub(crate) fn message_sequence_tx(transaction: &Transaction<'_>, message_id: &str) -> Result<i64> {
    transaction
        .query_row(
            "SELECT sequence FROM conversation_messages WHERE id = ?1",
            [message_id],
            |row| row.get(0),
        )
        .map_err(Into::into)
}

pub(crate) fn unread_count_for_state_tx(
    transaction: &Transaction<'_>,
    conversation_id: &str,
    participant_id: &str,
    predicate: &str,
    sequence: i64,
) -> Result<i64> {
    let sql = format!(
        "SELECT COUNT(*)
         FROM conversation_messages
         WHERE conversation_id = ?1
           AND participant_id != ?2
           AND status NOT IN ('cancelled', 'tombstoned')
           AND {predicate}"
    );
    transaction
        .query_row(
            &sql,
            params![conversation_id, participant_id, sequence],
            |row| row.get(0),
        )
        .map_err(Into::into)
}

pub(crate) fn active_reaction(
    connection: &Connection,
    message_id: &str,
    participant_id: &str,
    reaction_key: &str,
) -> Result<Option<ConversationReactionView>> {
    connection
        .query_row(
            "SELECT id, message_id, participant_id, reaction_key, reaction_kind,
                    metadata_json, created_at, removed_at
             FROM conversation_reactions
             WHERE message_id = ?1
               AND participant_id = ?2
               AND reaction_key = ?3
               AND removed_at IS NULL",
            params![message_id, participant_id, reaction_key],
            reaction_from_row,
        )
        .optional()
        .map_err(Into::into)
}

pub(crate) fn load_reaction(
    connection: &Connection,
    reaction_id: &str,
) -> Result<ConversationReactionView> {
    connection
        .query_row(
            "SELECT id, message_id, participant_id, reaction_key, reaction_kind,
                    metadata_json, created_at, removed_at
             FROM conversation_reactions
             WHERE id = ?1",
            [reaction_id],
            reaction_from_row,
        )
        .map_err(Into::into)
}

pub(crate) fn removed_reaction_placeholder(
    message_id: &str,
    participant_id: &str,
    reaction_key: &str,
    reaction_kind: &str,
) -> ConversationReactionView {
    ConversationReactionView {
        id: String::new(),
        message_id: message_id.to_string(),
        participant_id: participant_id.to_string(),
        reaction_key: reaction_key.to_string(),
        reaction_kind: reaction_kind.to_string(),
        metadata: json!({}),
        created_at: String::new(),
        removed_at: Some(Utc::now().to_rfc3339()),
    }
}

pub(crate) fn load_presence_snapshot(
    connection: &Connection,
    participant_id: &str,
) -> Result<ConversationPresenceSnapshotView> {
    connection
        .query_row(
            "SELECT participant_id, conversation_id, status, visibility, status_message,
                    device_class, metadata_json, updated_at, expires_at
             FROM conversation_presence_snapshots
             WHERE participant_id = ?1",
            [participant_id],
            presence_from_row,
        )
        .map_err(Into::into)
}

pub(crate) fn list_presence_snapshots(
    connection: &Connection,
    conversation_id: &str,
    requesting_participant_id: &str,
) -> Result<Vec<ConversationPresenceSnapshotView>> {
    connection.query_many(
        "SELECT participant_id, conversation_id, status, visibility, status_message,
                device_class, metadata_json, updated_at, expires_at
         FROM conversation_presence_snapshots
         WHERE conversation_id = ?1
           AND status != 'offline'
           AND (
                visibility = 'public'
                OR visibility = 'participants'
                OR participant_id = ?2
           )
         ORDER BY updated_at DESC",
        params![conversation_id, requesting_participant_id],
        presence_from_row,
    )
}

pub(crate) fn latest_visible_message(
    connection: &Connection,
    conversation_id: &str,
) -> Result<Option<ConversationMessageView>> {
    connection
        .query_row(
            "SELECT id, conversation_id, segment_id, participant_id, message_kind, status,
                    body_markdown, visibility, client_message_id, sequence, event_cursor,
                    undo_expires_at, undo_cancelled_at, created_at, edited_at, deleted_at
             FROM conversation_messages
             WHERE conversation_id = ?1
               AND status NOT IN ('cancelled', 'tombstoned')
             ORDER BY sequence DESC
             LIMIT 1",
            [conversation_id],
            message_from_row,
        )
        .optional()
        .map_err(Into::into)
}

pub fn append_conversation_event(
    connection: &Connection,
    conversation_id: &str,
    segment_id: Option<&str>,
    handoff_id: Option<&str>,
    event_type: &str,
    payload: Value,
    policy_decision_id: Option<&str>,
) -> Result<RealtimeEvent> {
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
    Ok(realtime)
}

pub(crate) fn append_conversation_event_tx(
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

pub(crate) fn next_conversation_sequence(
    connection: &Connection,
    conversation_id: &str,
) -> Result<i64> {
    let current: i64 = connection.query_row(
        "SELECT COALESCE(MAX(sequence), 0) FROM conversation_events WHERE conversation_id = ?1",
        [conversation_id],
        |row| row.get(0),
    )?;
    Ok(current + 1)
}

pub(crate) fn next_conversation_sequence_tx(
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

pub(crate) fn conversation_summary_from_row(
    row: &Row<'_>,
) -> rusqlite::Result<ConversationSummary> {
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

pub(crate) fn segment_from_row(row: &Row<'_>) -> rusqlite::Result<ConversationSegmentView> {
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

pub(crate) fn handoff_from_row(row: &Row<'_>) -> rusqlite::Result<ConversationHandoffView> {
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

pub(crate) fn mode_from_row(row: &Row<'_>) -> rusqlite::Result<ConversationModeView> {
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

pub(crate) fn queue_row_from_row(row: &Row<'_>) -> rusqlite::Result<ConversationQueueRow> {
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

pub(crate) fn authorize_participant_mutation(
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

pub(crate) fn actor_can_act_for_participant(
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

pub(crate) fn participant_from_row(row: &Row<'_>) -> rusqlite::Result<ConversationParticipantView> {
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

pub(crate) fn message_from_row(row: &Row<'_>) -> rusqlite::Result<ConversationMessageView> {
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

pub(crate) fn read_state_from_row(row: &Row<'_>) -> rusqlite::Result<ConversationReadStateView> {
    Ok(ConversationReadStateView {
        conversation_id: row.get(0)?,
        participant_id: row.get(1)?,
        last_read_message_id: row.get(2)?,
        last_read_event_cursor: row.get(3)?,
        last_read_at: row.get(4)?,
        manual_unread_from_message_id: row.get(5)?,
        unread_count: row.get(6)?,
        unread_mentions_count: row.get(7)?,
        unread_action_count: row.get(8)?,
        updated_at: row.get(9)?,
    })
}

pub(crate) fn reaction_from_row(row: &Row<'_>) -> rusqlite::Result<ConversationReactionView> {
    let metadata_json: String = row.get(5)?;
    Ok(ConversationReactionView {
        id: row.get(0)?,
        message_id: row.get(1)?,
        participant_id: row.get(2)?,
        reaction_key: row.get(3)?,
        reaction_kind: row.get(4)?,
        metadata: serde_json::from_str(&metadata_json).unwrap_or_else(|_| json!({})),
        created_at: row.get(6)?,
        removed_at: row.get(7)?,
    })
}

pub(crate) fn presence_from_row(
    row: &Row<'_>,
) -> rusqlite::Result<ConversationPresenceSnapshotView> {
    let metadata_json: String = row.get(6)?;
    Ok(ConversationPresenceSnapshotView {
        participant_id: row.get(0)?,
        conversation_id: row.get(1)?,
        status: row.get(2)?,
        visibility: row.get(3)?,
        status_message: row.get(4)?,
        device_class: row.get(5)?,
        metadata: serde_json::from_str(&metadata_json).unwrap_or_else(|_| json!({})),
        updated_at: row.get(7)?,
        expires_at: row.get(8)?,
    })
}

pub(crate) fn validate_handoff_request(request: &ConversationHandoffCreateRequest) -> Result<()> {
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

pub(crate) fn valid_handoff_transition(from: HandoffStatus, to: HandoffStatus) -> bool {
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

pub(crate) fn require_text(field_name: &str, value: &str) -> Result<()> {
    ensure!(!value.trim().is_empty(), "{field_name} is required");
    Ok(())
}

pub(crate) fn to_sql_error(error: anyhow::Error) -> rusqlite::Error {
    rusqlite::Error::FromSqlConversionFailure(
        0,
        rusqlite::types::Type::Text,
        Box::new(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            error.to_string(),
        )),
    )
}
