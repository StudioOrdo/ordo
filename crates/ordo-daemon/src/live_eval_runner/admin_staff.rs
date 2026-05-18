use super::*;
use anyhow::{anyhow, ensure, Result};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdminStaffJourneyEvidence {
    pub persona_id: String,
    pub case_id: String,
    pub conversation_id: String,
    pub visitor_message_id: String,
    pub handoff_id: String,
    pub final_handoff_status: String,
    pub human_led_blocked_public_agent_post: bool,
    pub delegated_allows_public_agent_post: bool,
    pub returned_mode_allows_public_agent_post: bool,
    pub review_id: String,
    pub review_public_count_before_approval: usize,
    pub review_public_count_after_publish: usize,
    pub affiliate_connection_id: String,
    pub affiliate_grant_id: String,
    pub affiliate_allowed_before_revoke: bool,
    pub affiliate_denied_after_revoke: bool,
    pub staff_queue_count: usize,
    pub manager_queue_count: usize,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdminStaffJourneyManifest {
    pub schema_version: String,
    pub source_commit: String,
    pub guard: LiveEvalGuardDecision,
    pub provider_mode: String,
    pub network_enabled: bool,
    pub evidence: AdminStaffJourneyEvidence,
    pub packet_path: String,
    pub scorecard_path: String,
    pub manifest_path: String,
    pub redaction_detectors: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdminStaffJourneyRunSummary {
    pub schema_version: String,
    pub status: LiveEvalStatus,
    pub case_id: String,
    pub persona_id: String,
    pub provider_mode: String,
    pub network_enabled: bool,
    pub conversation_id: String,
    pub handoff_id: String,
    pub final_handoff_status: String,
    pub review_id: String,
    pub affiliate_connection_id: String,
    pub packet_path: String,
    pub scorecard_path: String,
    pub manifest_path: String,
    pub journey_manifest_path: String,
    pub message: String,
}

pub fn run_admin_staff_journey_eval(
    db_path: &Path,
    connection: &Connection,
    persona_dir: &Path,
    selected_persona_id: Option<&str>,
    output_dir: impl Into<PathBuf>,
    source_commit: impl Into<String>,
    private_terms: Vec<String>,
) -> Result<AdminStaffJourneyRunSummary> {
    let personas = load_persona_dir(persona_dir, &private_terms)?;
    let persona = select_admin_staff_persona(&personas, selected_persona_id)?;
    let case = admin_staff_journey_case(&persona)?;
    let output_dir = output_dir.into();
    let packet_path = output_dir.join(format!("{}-packet.json", case.id));
    let source_commit = source_commit.into();
    let mut harness = DeterministicEvalHarness::new(DeterministicEvalClock::fixed())
        .with_artifact_path(packet_path.to_string_lossy());
    let mut state = AdminStaffJourneyState::new(persona.clone());
    let mut scorecard = harness.run_case(connection, &case, |connection, step| {
        run_admin_staff_journey_step(db_path, connection, step, &mut state)
    })?;
    scorecard.provider_mode = "deterministic_live_journey".to_string();
    scorecard.network_enabled = false;

    let mut writer_private_terms = private_terms;
    writer_private_terms.push(persona.narrative_markdown.clone());
    writer_private_terms.push("Project Orchid".to_string());
    writer_private_terms.push("Project".to_string());
    writer_private_terms.push("Orchid".to_string());
    writer_private_terms.push("sk-live-journey-fixture".to_string());
    writer_private_terms.push("alex@example.com".to_string());
    writer_private_terms.push("admin-staff-secret".to_string());
    let writer = EvalArtifactWriter::new(&output_dir, &source_commit)
        .with_private_terms(writer_private_terms.clone());
    let artifact_paths = writer.write_packet(connection, &case, &scorecard)?;
    let evidence = state.into_evidence()?;
    ensure_admin_staff_evidence(&evidence)?;
    let journey_manifest_path = output_dir.join(format!("{}-journey.json", case.id));
    let manifest = AdminStaffJourneyManifest {
        schema_version: ADMIN_STAFF_JOURNEY_SCHEMA_VERSION.to_string(),
        source_commit,
        guard: LiveEvalGuardDecision {
            status: LiveEvalStatus::Completed,
            reason:
                "Admin/staff journey used deterministic domain helpers; no provider network ran."
                    .to_string(),
            network_enabled: false,
        },
        provider_mode: "deterministic_live_journey".to_string(),
        network_enabled: false,
        evidence: evidence.clone(),
        packet_path: artifact_paths.packet_path.to_string_lossy().to_string(),
        scorecard_path: artifact_paths.scorecard_path.to_string_lossy().to_string(),
        manifest_path: artifact_paths.manifest_path.to_string_lossy().to_string(),
        redaction_detectors: vec![
            "email".to_string(),
            "phone".to_string(),
            "auth-token-shaped".to_string(),
            "api-key-shaped".to_string(),
            "private_term".to_string(),
            "persona_narrative".to_string(),
            "staff_internal".to_string(),
        ],
    };
    ensure_admin_staff_manifest_is_safe(&manifest, &writer_private_terms)?;
    write_json(&journey_manifest_path, &manifest)?;

    Ok(AdminStaffJourneyRunSummary {
        schema_version: ADMIN_STAFF_JOURNEY_SCHEMA_VERSION.to_string(),
        status: if scorecard.passed {
            LiveEvalStatus::Completed
        } else {
            LiveEvalStatus::Failed
        },
        case_id: case.id,
        persona_id: evidence.persona_id,
        provider_mode: "deterministic_live_journey".to_string(),
        network_enabled: false,
        conversation_id: evidence.conversation_id,
        handoff_id: evidence.handoff_id,
        final_handoff_status: evidence.final_handoff_status,
        review_id: evidence.review_id,
        affiliate_connection_id: evidence.affiliate_connection_id,
        packet_path: artifact_paths.packet_path.to_string_lossy().to_string(),
        scorecard_path: artifact_paths.scorecard_path.to_string_lossy().to_string(),
        manifest_path: artifact_paths.manifest_path.to_string_lossy().to_string(),
        journey_manifest_path: journey_manifest_path.to_string_lossy().to_string(),
        message: if scorecard.passed {
            "Admin/staff handoff and moderation journey eval completed without provider keys or network."
                .to_string()
        } else {
            "Admin/staff handoff and moderation journey eval completed with failed assertions."
                .to_string()
        },
    })
}

#[derive(Debug, Clone)]
pub(crate) struct AdminStaffJourneyState {
    persona: EvalPersona,
    conversation_id: Option<String>,
    visitor_message_id: Option<String>,
    handoff_id: Option<String>,
    final_handoff_status: Option<String>,
    human_led_blocked_public_agent_post: bool,
    delegated_allows_public_agent_post: bool,
    returned_mode_allows_public_agent_post: bool,
    review_id: Option<String>,
    review_public_count_before_approval: usize,
    review_public_count_after_publish: usize,
    affiliate_connection_id: Option<String>,
    affiliate_grant_id: Option<String>,
    affiliate_allowed_before_revoke: bool,
    affiliate_denied_after_revoke: bool,
    staff_queue_count: usize,
    manager_queue_count: usize,
}

impl AdminStaffJourneyState {
    fn new(persona: EvalPersona) -> Self {
        Self {
            persona,
            conversation_id: None,
            visitor_message_id: None,
            handoff_id: None,
            final_handoff_status: None,
            human_led_blocked_public_agent_post: false,
            delegated_allows_public_agent_post: false,
            returned_mode_allows_public_agent_post: false,
            review_id: None,
            review_public_count_before_approval: 0,
            review_public_count_after_publish: 0,
            affiliate_connection_id: None,
            affiliate_grant_id: None,
            affiliate_allowed_before_revoke: false,
            affiliate_denied_after_revoke: false,
            staff_queue_count: 0,
            manager_queue_count: 0,
        }
    }

    fn into_evidence(self) -> Result<AdminStaffJourneyEvidence> {
        Ok(AdminStaffJourneyEvidence {
            persona_id: self.persona.persona_id.clone(),
            case_id: format!(
                "{ADMIN_STAFF_JOURNEY_CASE_PREFIX}_{}",
                self.persona.persona_id
            ),
            conversation_id: required_state(self.conversation_id, "conversation id")?,
            visitor_message_id: required_state(self.visitor_message_id, "visitor message id")?,
            handoff_id: required_state(self.handoff_id, "handoff id")?,
            final_handoff_status: required_state(
                self.final_handoff_status,
                "final handoff status",
            )?,
            human_led_blocked_public_agent_post: self.human_led_blocked_public_agent_post,
            delegated_allows_public_agent_post: self.delegated_allows_public_agent_post,
            returned_mode_allows_public_agent_post: self.returned_mode_allows_public_agent_post,
            review_id: required_state(self.review_id, "review id")?,
            review_public_count_before_approval: self.review_public_count_before_approval,
            review_public_count_after_publish: self.review_public_count_after_publish,
            affiliate_connection_id: required_state(
                self.affiliate_connection_id,
                "affiliate connection id",
            )?,
            affiliate_grant_id: required_state(self.affiliate_grant_id, "affiliate grant id")?,
            affiliate_allowed_before_revoke: self.affiliate_allowed_before_revoke,
            affiliate_denied_after_revoke: self.affiliate_denied_after_revoke,
            staff_queue_count: self.staff_queue_count,
            manager_queue_count: self.manager_queue_count,
            evidence_refs: vec![
                "relationship_conversation".to_string(),
                "conversation_handoff".to_string(),
                "conversation_mode".to_string(),
                "agent_silence_boundary".to_string(),
                "review_moderation".to_string(),
                "affiliate_connection".to_string(),
                "connection_grant".to_string(),
                "policy_decision".to_string(),
            ],
        })
    }
}

pub(crate) fn select_admin_staff_persona(
    personas: &[EvalPersona],
    selected_persona_id: Option<&str>,
) -> Result<EvalPersona> {
    match selected_persona_id {
        Some(id) => personas
            .iter()
            .find(|persona| persona.persona_id == id)
            .cloned()
            .ok_or_else(|| anyhow!("unknown admin/staff persona id {id}")),
        None => personas
            .iter()
            .find(|persona| persona.handoff_likelihood == "high")
            .cloned()
            .or_else(|| {
                personas
                    .iter()
                    .find(|persona| persona.persona_id == "dissatisfied_trial_user")
                    .cloned()
            })
            .ok_or_else(|| anyhow!("persona library has no admin/staff handoff candidate")),
    }
}

pub(crate) fn admin_staff_journey_case(persona: &EvalPersona) -> Result<EvalCase> {
    EvalCase::new(
        format!("{ADMIN_STAFF_JOURNEY_CASE_PREFIX}_{}", persona.persona_id),
        "Admin/staff handoff and moderation journey",
        &json!({
            "fixture": "admin_staff_journey",
            "version": 1,
            "personaId": persona.persona_id,
            "personaHash": persona.content_hash,
            "providerMode": "deterministic_live_journey",
            "networkRequired": false,
            "deferredPhases": ["cross_persona_report", "real_email_adapter_decision"],
        }),
        vec![
            EvalActorRole::AnonymousVisitor,
            EvalActorRole::Staff,
            EvalActorRole::ManagerAdmin,
            EvalActorRole::OwnerSystemAdmin,
            EvalActorRole::Affiliate,
            EvalActorRole::OrdoAgent,
        ],
        vec![EvalStep::new(
            "run_admin_staff_handoff_and_moderation_journey",
            EvalActorRole::Staff,
            "live_journey.admin_staff",
            vec![
                EvalEvidenceChannel::SqliteRows,
                EvalEvidenceChannel::ConversationEvents,
                EvalEvidenceChannel::RealtimeReplay,
                EvalEvidenceChannel::PolicyDecisions,
                EvalEvidenceChannel::HandoffState,
                EvalEvidenceChannel::FeedbackReviewRecords,
            ],
        )?],
        vec![
            EvalAssertion::minimum_count(
                "durable_sqlite_rows_recorded",
                EvalEvidenceChannel::SqliteRows,
                30,
            )?,
            EvalAssertion::minimum_count(
                "conversation_events_recorded",
                EvalEvidenceChannel::ConversationEvents,
                14,
            )?,
            EvalAssertion::minimum_count(
                "realtime_replay_recorded",
                EvalEvidenceChannel::RealtimeReplay,
                14,
            )?,
            EvalAssertion::minimum_count(
                "policy_decisions_recorded",
                EvalEvidenceChannel::PolicyDecisions,
                3,
            )?,
            EvalAssertion::minimum_count(
                "handoff_state_recorded",
                EvalEvidenceChannel::HandoffState,
                1,
            )?,
            EvalAssertion::minimum_count(
                "feedback_review_records_recorded",
                EvalEvidenceChannel::FeedbackReviewRecords,
                2,
            )?,
        ],
    )
}

pub(crate) fn run_admin_staff_journey_step(
    db_path: &Path,
    connection: &Connection,
    step: &EvalStep,
    state: &mut AdminStaffJourneyState,
) -> Result<()> {
    match step.id.as_str() {
        "run_admin_staff_handoff_and_moderation_journey" => {
            let conversation = find_or_create_canonical_conversation(
                connection,
                &CanonicalConversationRequest {
                    surface: "chat".to_string(),
                    subject_kind: "admin_staff_eval".to_string(),
                    subject_id: state.persona.persona_id.clone(),
                    connection_id: None,
                    visitor_session_id: None,
                    created_by_actor_id: None,
                },
            )?;
            state.conversation_id = Some(conversation.id.clone());
            let visitor = create_conversation_participant(
                connection,
                &ConversationParticipantCreateRequest {
                    conversation_id: conversation.id.clone(),
                    participant_kind: "visitor".to_string(),
                    actor_id: None,
                    connection_id: None,
                    visitor_session_id: None,
                    display_name: state.persona.display_name.clone(),
                    role: "moderation_requester".to_string(),
                },
            )?;
            let message = create_conversation_message(
                connection,
                &ConversationMessageCreateRequest {
                    conversation_id: conversation.id.clone(),
                    segment_id: None,
                    participant_id: visitor.id,
                    message_kind: "message".to_string(),
                    body_markdown: persona_backed_admin_staff_message(&state.persona),
                    visibility: "participants".to_string(),
                    client_message_id: format!("admin-staff-message-{}", state.persona.persona_id),
                    reply_to_message_id: None,
                    undo_expires_at: None,
                },
            )?;
            state.visitor_message_id = Some(message.id.clone());

            let handoff_policy = crate::policy::PolicyDecision {
                outcome: PolicyOutcome::ReviewRequired,
                actor: ActorContext::local_owner("live_journey_admin_staff_eval"),
                action: PolicyAction::Approve,
                resource: ResourceRef::new(ResourceKind::Conversation, &conversation.id),
                capability_id: Some("conversation.handoff.manage".to_string()),
                reason: "Staff handoff requires visible moderation evidence.".to_string(),
            };
            let handoff_policy_id = record_policy_decision(
                connection,
                &handoff_policy,
                PolicyDecisionCorrelation {
                    request_id: Some(format!("handoff-policy-{}", state.persona.persona_id)),
                    ..PolicyDecisionCorrelation::default()
                },
            )?;
            let handoff = create_conversation_handoff(
                connection,
                &ConversationHandoffCreateRequest {
                    conversation_id: conversation.id.clone(),
                    segment_id: None,
                    connection_id: None,
                    requested_by_actor_id: None,
                    assigned_to_actor_id: Some(LOCAL_OWNER_ACTOR_ID.to_string()),
                    reason: "Persona asked for staff review and moderation.".to_string(),
                    urgency: "normal".to_string(),
                    required_capability_id: "conversation.handoff.manage".to_string(),
                    evidence_summary:
                        "Visitor requested staff review; only brief context is allowed.".to_string(),
                    allowed_context: vec![
                        format!("message:{}", message.id),
                        "review_status_only".to_string(),
                    ],
                    policy_decision_id: Some(handoff_policy_id),
                },
            )?;
            state.handoff_id = Some(handoff.id.clone());

            let staff_queue = conversation_queue(
                connection,
                ConversationRole::Staff,
                Some(LOCAL_OWNER_ACTOR_ID),
                Some(QueueScope::MyHandoffs),
            )?;
            let manager_queue = conversation_queue(
                connection,
                ConversationRole::Manager,
                None,
                Some(QueueScope::TeamQueue),
            )?;
            state.staff_queue_count = staff_queue.len();
            state.manager_queue_count = manager_queue.len();

            let accepted = transition_conversation_handoff(
                connection,
                &handoff.id,
                HandoffStatus::Accepted,
                Some(LOCAL_OWNER_ACTOR_ID),
                "staff accepted moderation handoff",
            )?;
            let assigned = transition_conversation_handoff(
                connection,
                &accepted.id,
                HandoffStatus::Assigned,
                Some(LOCAL_OWNER_ACTOR_ID),
                "assigned to owner/admin for review",
            )?;
            let in_progress = transition_conversation_handoff(
                connection,
                &assigned.id,
                HandoffStatus::InProgress,
                Some(LOCAL_OWNER_ACTOR_ID),
                "staff is actively moderating",
            )?;
            let returned = transition_conversation_handoff(
                connection,
                &in_progress.id,
                HandoffStatus::ReturnedToAgent,
                Some(LOCAL_OWNER_ACTOR_ID),
                "agent may resume after staff review",
            )?;
            let closed = transition_conversation_handoff(
                connection,
                &returned.id,
                HandoffStatus::Closed,
                Some(LOCAL_OWNER_ACTOR_ID),
                "handoff complete",
            )?;
            state.final_handoff_status = Some(handoff_status_label(closed.status).to_string());

            let human_led = record_staff_activity_sets_human_led(
                connection,
                &conversation.id,
                LOCAL_OWNER_ACTOR_ID,
            )?;
            let blocked = may_agent_post_publicly(human_led.mode, &PublicPostContext::default());
            state.human_led_blocked_public_agent_post = !blocked.allowed;
            let delegated = upsert_conversation_mode(
                connection,
                &conversation.id,
                ConversationMode::HumanLedActive,
                Some(LOCAL_OWNER_ACTOR_ID),
                true,
                vec!["review_follow_up_only".to_string()],
                None,
            )?;
            state.delegated_allows_public_agent_post = may_agent_post_publicly(
                delegated.mode,
                &PublicPostContext {
                    delegated: delegated.delegated_to_agent,
                    ..Default::default()
                },
            )
            .allowed;
            let returned_mode = upsert_conversation_mode(
                connection,
                &conversation.id,
                ConversationMode::ReturnedToAgent,
                None,
                false,
                vec![],
                None,
            )?;
            state.returned_mode_allows_public_agent_post =
                may_agent_post_publicly(returned_mode.mode, &PublicPostContext::default()).allowed;

            let (feedback, _) = capture_feedback(
                connection,
                CustomerFeedbackInput {
                    connection_id: None,
                    conversation_id: conversation.id.clone(),
                    segment_id: None,
                    message_id: Some(message.id.clone()),
                    feedback_kind: "moderation_review".to_string(),
                    body_summary:
                        "Visitor feedback is useful but requires staff approval before publication."
                            .to_string(),
                    source_refs: vec![format!("message:{}", message.id)],
                    evidence_refs: vec![format!("message:{}", message.id)],
                    provenance: json!({
                        "generator": "live_journey.admin_staff",
                        "privateBusinessIntelligence": true,
                        "requiresModeration": true,
                    }),
                },
            )?;
            let (review, _) = create_review_candidate(
                connection,
                &feedback.id,
                ReviewCandidateInput {
                    review_body:
                        "Studio Ordo made the review and handoff path feel more controlled."
                            .to_string(),
                    evidence_refs: vec![format!("feedback:{}", feedback.id)],
                    provenance: json!({
                        "generator": "live_journey.admin_staff",
                        "candidateOnly": true,
                    }),
                },
            )?;
            state.review_public_count_before_approval = list_public_reviews(connection)?.len();
            let requested = transition_review(
                connection,
                &review.id,
                ReviewStatus::Requested,
                vec![format!("handoff:{}", closed.id)],
                "staff requested review approval evidence",
            )?
            .0;
            let received = transition_review(
                connection,
                &requested.id,
                ReviewStatus::Received,
                vec![format!("message:{}", message.id)],
                "review text received for moderation",
            )?
            .0;
            let consented = transition_review(
                connection,
                &received.id,
                ReviewStatus::ConsentConfirmed,
                vec![format!("message:{}", message.id)],
                "consent confirmed before staff approval",
            )?
            .0;
            let approved = transition_review(
                connection,
                &consented.id,
                ReviewStatus::Approved,
                vec![format!("handoff:{}", closed.id)],
                "staff/admin approved publication",
            )?
            .0;
            let published = transition_review(
                connection,
                &approved.id,
                ReviewStatus::Published,
                vec![format!("review:{}", approved.id)],
                "published after consent and approval",
            )?
            .0;
            state.review_id = Some(published.id.clone());
            state.review_public_count_after_publish = list_public_reviews(connection)?.len();

            let (affiliate, _) = create_connection(
                db_path,
                ConnectionWriteRequest {
                    connection_type: ConnectionType::Affiliate,
                    display_name: "Moderated affiliate".to_string(),
                    status: Some(ConnectionStatus::Active),
                    identity: Some(json!({ "synthetic": true })),
                    scope: Some(json!({ "adminStaffEval": true })),
                    metadata: Some(json!({
                        "generator": "live_journey.admin_staff",
                        "governedByStaff": true,
                    })),
                },
                None,
            )?;
            state.affiliate_connection_id = Some(affiliate.id.clone());
            let (grant, _) = create_connection_grant(
                db_path,
                &affiliate.id,
                ConnectionGrantCreateRequest {
                    resource_kind: ResourceKind::Conversation.as_str().to_string(),
                    resource_id: conversation.id.clone(),
                    action: PolicyAction::Read.as_str().to_string(),
                    expires_at: None,
                    grant_reason: Some("temporary affiliate management inspection".to_string()),
                },
                None,
            )?;
            state.affiliate_grant_id = Some(grant.id.clone());
            let allowed = authorize_connection_resource_access(
                connection,
                &affiliate.id,
                PolicyAction::Read,
                ResourceRef::new(ResourceKind::Conversation, &conversation.id),
                Some("affiliate.management.inspect"),
            );
            state.affiliate_allowed_before_revoke = allowed.outcome == PolicyOutcome::Allowed;
            record_policy_decision(
                connection,
                &allowed,
                PolicyDecisionCorrelation {
                    request_id: Some(format!(
                        "affiliate-management-allowed-{}",
                        state.persona.persona_id
                    )),
                    ..PolicyDecisionCorrelation::default()
                },
            )?;
            revoke_connection_grant(
                db_path,
                &grant.id,
                ConnectionGrantRevokeRequest {
                    revocation_reason: Some(
                        "admin/staff eval revokes temporary access".to_string(),
                    ),
                },
                None,
            )?;
            let denied = authorize_connection_resource_access(
                connection,
                &affiliate.id,
                PolicyAction::Read,
                ResourceRef::new(ResourceKind::Conversation, &conversation.id),
                Some("affiliate.management.inspect"),
            );
            state.affiliate_denied_after_revoke = denied.outcome == PolicyOutcome::Denied;
            record_policy_decision(
                connection,
                &denied,
                PolicyDecisionCorrelation {
                    request_id: Some(format!(
                        "affiliate-management-denied-{}",
                        state.persona.persona_id
                    )),
                    ..PolicyDecisionCorrelation::default()
                },
            )?;
        }
        other => anyhow::bail!("unsupported admin/staff journey step: {other}"),
    }
    Ok(())
}

pub(crate) fn ensure_admin_staff_evidence(evidence: &AdminStaffJourneyEvidence) -> Result<()> {
    ensure!(!evidence.handoff_id.is_empty(), "handoff evidence missing");
    ensure!(
        evidence.final_handoff_status == handoff_status_label(HandoffStatus::Closed),
        "handoff should close after staff moderation"
    );
    ensure!(
        evidence.human_led_blocked_public_agent_post,
        "human-led active mode should block untagged public agent post"
    );
    ensure!(
        evidence.delegated_allows_public_agent_post,
        "delegation should allow agent public post"
    );
    ensure!(
        evidence.returned_mode_allows_public_agent_post,
        "returned-to-agent mode should allow public agent post"
    );
    ensure!(
        evidence.review_public_count_before_approval == 0,
        "review should not be public before approval"
    );
    ensure!(
        evidence.review_public_count_after_publish >= 1,
        "review should be public after consent and approval"
    );
    ensure!(
        evidence.affiliate_allowed_before_revoke,
        "affiliate grant should allow scoped access before revocation"
    );
    ensure!(
        evidence.affiliate_denied_after_revoke,
        "revoked affiliate grant should deny access"
    );
    ensure!(
        evidence.staff_queue_count >= 1,
        "staff queue should include assigned handoff"
    );
    ensure!(
        evidence.manager_queue_count >= 1,
        "manager/team queue should include active handoff"
    );
    Ok(())
}

pub(crate) fn ensure_admin_staff_manifest_is_safe(
    manifest: &AdminStaffJourneyManifest,
    private_terms: &[String],
) -> Result<()> {
    let value = serde_json::to_value(manifest)?;
    ensure!(
        !contains_sensitive_value(&value, private_terms),
        "admin/staff journey manifest contains raw sensitive value"
    );
    Ok(())
}

pub(crate) fn persona_backed_admin_staff_message(persona: &EvalPersona) -> String {
    format!(
        "I am a {} trial user asking for staff review before anything public happens. Please keep the review and handoff details private, avoid exposing policy or provider mechanics, and do not repeat Project Orchid, alex@example.com, or admin-staff-secret.",
        persona.person_type
    )
}

pub(crate) fn handoff_status_label(status: HandoffStatus) -> &'static str {
    match status {
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
