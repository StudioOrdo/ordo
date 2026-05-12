use super::*;
use anyhow::{anyhow, ensure, Result};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::path::{Path, PathBuf};
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewReturnJourneyEvidence {
    pub persona_id: String,
    pub case_id: String,
    pub qr_case_id: String,
    pub simulated_email_artifact_id: String,
    pub return_entry_point_id: String,
    pub return_entry_point_slug: String,
    pub return_visitor_session_id: String,
    pub conversation_id: String,
    pub return_message_id: String,
    pub assistant_message_id: String,
    pub feedback_id: String,
    pub review_id: String,
    pub final_review_status: String,
    pub public_review_count_before_publish: usize,
    pub public_review_count_after_publish: usize,
    pub public_review_count_after_retire: usize,
    pub blocked_publish_without_consent_or_approval: bool,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewReturnJourneyManifest {
    pub schema_version: String,
    pub source_commit: String,
    pub guard: LiveEvalGuardDecision,
    pub provider_mode: String,
    pub network_enabled: bool,
    pub elapsed_days_simulated: i64,
    pub evidence: ReviewReturnJourneyEvidence,
    pub qr_packet_path: String,
    pub qr_journey_manifest_path: String,
    pub packet_path: String,
    pub scorecard_path: String,
    pub manifest_path: String,
    pub redaction_detectors: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewReturnJourneyRunSummary {
    pub schema_version: String,
    pub status: LiveEvalStatus,
    pub case_id: String,
    pub persona_id: String,
    pub provider_mode: String,
    pub network_enabled: bool,
    pub simulated_email_artifact_id: String,
    pub return_entry_point_id: String,
    pub return_visitor_session_id: String,
    pub conversation_id: String,
    pub feedback_id: String,
    pub review_id: String,
    pub final_review_status: String,
    pub packet_path: String,
    pub scorecard_path: String,
    pub manifest_path: String,
    pub journey_manifest_path: String,
    pub message: String,
}

pub fn run_review_return_journey_eval(
    db_path: &Path,
    connection: &Connection,
    persona_dir: &Path,
    selected_persona_id: Option<&str>,
    output_dir: impl Into<PathBuf>,
    source_commit: impl Into<String>,
    private_terms: Vec<String>,
) -> Result<ReviewReturnJourneyRunSummary> {
    let output_dir = output_dir.into();
    let source_commit = source_commit.into();
    let qr_output_dir = output_dir.join("qr-to-trial-setup");
    let qr_summary = run_qr_to_trial_journey_eval(
        db_path,
        connection,
        persona_dir,
        selected_persona_id,
        &qr_output_dir,
        source_commit.clone(),
        private_terms.clone(),
    )?;
    ensure!(
        qr_summary.status == LiveEvalStatus::Completed,
        "review-return journey requires completed QR-to-trial setup"
    );

    let personas = load_persona_dir(persona_dir, &private_terms)?;
    let persona = select_qr_to_trial_persona(&personas, Some(&qr_summary.persona_id))?;
    let case = review_return_journey_case(&persona)?;
    let packet_path = output_dir.join(format!("{}-packet.json", case.id));
    let mut harness = DeterministicEvalHarness::new(DeterministicEvalClock::fixed())
        .with_artifact_path(packet_path.to_string_lossy());
    let mut state = ReviewReturnJourneyState::new(persona.clone(), qr_summary.clone());
    let mut scorecard = harness.run_case(connection, &case, |connection, step| {
        run_review_return_journey_step(db_path, connection, step, &mut state)
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
    writer_private_terms.push("review-return-secret".to_string());
    let writer = EvalArtifactWriter::new(&output_dir, &source_commit)
        .with_private_terms(writer_private_terms.clone());
    let artifact_paths = writer.write_packet(connection, &case, &scorecard)?;
    let evidence = state.into_evidence()?;
    ensure_review_return_evidence(&evidence)?;
    let journey_manifest_path = output_dir.join(format!("{}-journey.json", case.id));
    let manifest = ReviewReturnJourneyManifest {
        schema_version: REVIEW_RETURN_JOURNEY_SCHEMA_VERSION.to_string(),
        source_commit,
        guard: LiveEvalGuardDecision {
            status: LiveEvalStatus::Completed,
            reason: "Review-return journey used deterministic provider path and simulated email artifact; no real outbound email or provider network ran.".to_string(),
            network_enabled: false,
        },
        provider_mode: "deterministic_live_journey".to_string(),
        network_enabled: false,
        elapsed_days_simulated: REVIEW_RETURN_ELAPSED_DAYS,
        evidence: evidence.clone(),
        qr_packet_path: qr_summary.packet_path.clone(),
        qr_journey_manifest_path: qr_summary.journey_manifest_path.clone(),
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
            "simulated_email_body".to_string(),
        ],
    };
    ensure_review_return_manifest_is_safe(&manifest, &writer_private_terms)?;
    write_json(&journey_manifest_path, &manifest)?;

    Ok(ReviewReturnJourneyRunSummary {
        schema_version: REVIEW_RETURN_JOURNEY_SCHEMA_VERSION.to_string(),
        status: if scorecard.passed {
            LiveEvalStatus::Completed
        } else {
            LiveEvalStatus::Failed
        },
        case_id: case.id,
        persona_id: evidence.persona_id,
        provider_mode: "deterministic_live_journey".to_string(),
        network_enabled: false,
        simulated_email_artifact_id: evidence.simulated_email_artifact_id,
        return_entry_point_id: evidence.return_entry_point_id,
        return_visitor_session_id: evidence.return_visitor_session_id,
        conversation_id: evidence.conversation_id,
        feedback_id: evidence.feedback_id,
        review_id: evidence.review_id,
        final_review_status: evidence.final_review_status,
        packet_path: artifact_paths.packet_path.to_string_lossy().to_string(),
        scorecard_path: artifact_paths.scorecard_path.to_string_lossy().to_string(),
        manifest_path: artifact_paths.manifest_path.to_string_lossy().to_string(),
        journey_manifest_path: journey_manifest_path.to_string_lossy().to_string(),
        message: if scorecard.passed {
            "Review-return journey eval completed without real email, provider keys, or network."
                .to_string()
        } else {
            "Review-return journey eval completed with failed assertions.".to_string()
        },
    })
}

#[derive(Debug, Clone)]
pub(crate) struct ReviewReturnJourneyState {
    persona: EvalPersona,
    qr_summary: QrToTrialJourneyRunSummary,
    simulated_email_artifact_id: Option<String>,
    return_entry_point_id: Option<String>,
    return_entry_point_slug: Option<String>,
    return_visitor_session_id: Option<String>,
    return_message_id: Option<String>,
    assistant_message_id: Option<String>,
    feedback_id: Option<String>,
    review_id: Option<String>,
    final_review_status: Option<String>,
    public_review_count_before_publish: usize,
    public_review_count_after_publish: usize,
    public_review_count_after_retire: usize,
    blocked_publish_without_consent_or_approval: bool,
}

impl ReviewReturnJourneyState {
    fn new(persona: EvalPersona, qr_summary: QrToTrialJourneyRunSummary) -> Self {
        Self {
            persona,
            qr_summary,
            simulated_email_artifact_id: None,
            return_entry_point_id: None,
            return_entry_point_slug: None,
            return_visitor_session_id: None,
            return_message_id: None,
            assistant_message_id: None,
            feedback_id: None,
            review_id: None,
            final_review_status: None,
            public_review_count_before_publish: 0,
            public_review_count_after_publish: 0,
            public_review_count_after_retire: 0,
            blocked_publish_without_consent_or_approval: false,
        }
    }

    fn into_evidence(self) -> Result<ReviewReturnJourneyEvidence> {
        Ok(ReviewReturnJourneyEvidence {
            persona_id: self.persona.persona_id.clone(),
            case_id: format!(
                "{REVIEW_RETURN_JOURNEY_CASE_PREFIX}_{}",
                self.persona.persona_id
            ),
            qr_case_id: self.qr_summary.case_id,
            simulated_email_artifact_id: required_state(
                self.simulated_email_artifact_id,
                "simulated email artifact id",
            )?,
            return_entry_point_id: required_state(
                self.return_entry_point_id,
                "return entry point id",
            )?,
            return_entry_point_slug: required_state(
                self.return_entry_point_slug,
                "return entry point slug",
            )?,
            return_visitor_session_id: required_state(
                self.return_visitor_session_id,
                "return visitor session id",
            )?,
            conversation_id: self.qr_summary.conversation_id,
            return_message_id: required_state(self.return_message_id, "return message id")?,
            assistant_message_id: required_state(
                self.assistant_message_id,
                "assistant message id",
            )?,
            feedback_id: required_state(self.feedback_id, "feedback id")?,
            review_id: required_state(self.review_id, "review id")?,
            final_review_status: required_state(self.final_review_status, "final review status")?,
            public_review_count_before_publish: self.public_review_count_before_publish,
            public_review_count_after_publish: self.public_review_count_after_publish,
            public_review_count_after_retire: self.public_review_count_after_retire,
            blocked_publish_without_consent_or_approval: self
                .blocked_publish_without_consent_or_approval,
            evidence_refs: vec![
                "simulated_review_request_email_artifact".to_string(),
                "return_entry_point".to_string(),
                "return_visitor_session".to_string(),
                "relationship_conversation".to_string(),
                "return_message".to_string(),
                "llm_prompt_slot_usage".to_string(),
                "privacy_egress_transform".to_string(),
                "customer_feedback".to_string(),
                "customer_review".to_string(),
                "review_consent_evidence".to_string(),
                "review_approval_evidence".to_string(),
            ],
        })
    }
}

pub(crate) fn review_return_journey_case(persona: &EvalPersona) -> Result<EvalCase> {
    EvalCase::new(
        format!("{REVIEW_RETURN_JOURNEY_CASE_PREFIX}_{}", persona.persona_id),
        "Review-request return journey",
        &json!({
            "fixture": "review_return_journey",
            "version": 1,
            "personaId": persona.persona_id,
            "personaHash": persona.content_hash,
            "providerMode": "deterministic_live_journey",
            "networkRequired": false,
            "simulatedEmailOnly": true,
            "elapsedDaysSimulated": REVIEW_RETURN_ELAPSED_DAYS,
            "deferredPhases": ["affiliate_referral", "admin_staff_handoff", "cross_persona_report", "real_email_adapter_decision"],
        }),
        vec![
            EvalActorRole::AnonymousVisitor,
            EvalActorRole::OrdoAgent,
            EvalActorRole::LlmToolProviderBoundary,
            EvalActorRole::Staff,
        ],
        vec![EvalStep::new(
            "run_review_request_return_journey",
            EvalActorRole::AnonymousVisitor,
            "live_journey.review_return",
            vec![
                EvalEvidenceChannel::SqliteRows,
                EvalEvidenceChannel::ConversationEvents,
                EvalEvidenceChannel::RealtimeReplay,
                EvalEvidenceChannel::PolicyDecisions,
                EvalEvidenceChannel::PromptSlotAccounting,
                EvalEvidenceChannel::PrivacyTransforms,
                EvalEvidenceChannel::TokenLedger,
                EvalEvidenceChannel::ArtifactRecords,
                EvalEvidenceChannel::FeedbackReviewRecords,
            ],
        )?],
        vec![
            EvalAssertion::minimum_count(
                "durable_sqlite_rows_recorded",
                EvalEvidenceChannel::SqliteRows,
                35,
            )?,
            EvalAssertion::minimum_count(
                "conversation_events_recorded",
                EvalEvidenceChannel::ConversationEvents,
                18,
            )?,
            EvalAssertion::minimum_count(
                "realtime_replay_recorded",
                EvalEvidenceChannel::RealtimeReplay,
                18,
            )?,
            EvalAssertion::minimum_count(
                "policy_decision_recorded",
                EvalEvidenceChannel::PolicyDecisions,
                2,
            )?,
            EvalAssertion::minimum_count(
                "prompt_slot_accounting_recorded",
                EvalEvidenceChannel::PromptSlotAccounting,
                3,
            )?,
            EvalAssertion::minimum_count(
                "privacy_transform_recorded",
                EvalEvidenceChannel::PrivacyTransforms,
                2,
            )?,
            EvalAssertion::minimum_count(
                "token_ledger_recorded",
                EvalEvidenceChannel::TokenLedger,
                4,
            )?,
            EvalAssertion::minimum_count(
                "simulated_email_artifact_recorded",
                EvalEvidenceChannel::ArtifactRecords,
                1,
            )?,
            EvalAssertion::minimum_count(
                "private_feedback_recorded",
                EvalEvidenceChannel::FeedbackReviewRecords,
                2,
            )?,
        ],
    )
}

pub(crate) fn run_review_return_journey_step(
    db_path: &Path,
    connection: &Connection,
    step: &EvalStep,
    state: &mut ReviewReturnJourneyState,
) -> Result<()> {
    match step.id.as_str() {
        "run_review_request_return_journey" => {
            let simulated_email_summary = format!(
                "Simulated review request for {} after {REVIEW_RETURN_ELAPSED_DAYS} days. Not delivered.",
                state.persona.person_type
            );
            let simulated_email_body = format!(
                "This is a simulated review-request email artifact for trial {}. It invites the visitor back through a review return link without sending real email or creating public proof.",
                state.qr_summary.trial_id
            );
            let (email_artifact, _) = record_artifact(
                connection,
                ArtifactInput {
                    artifact_kind: "simulated_review_request_email".to_string(),
                    title: "Simulated review request email".to_string(),
                    status: "simulated_not_delivered".to_string(),
                    visibility_ceiling: "staff_private".to_string(),
                    summary: simulated_email_summary,
                    source_kind: Some("trial".to_string()),
                    source_id: Some(state.qr_summary.trial_id.clone()),
                    evidence_refs: vec![
                        format!("trial:{}", state.qr_summary.trial_id),
                        format!("offer:{}", state.qr_summary.offer_id),
                        format!("conversation:{}", state.qr_summary.conversation_id),
                    ],
                    provenance: json!({
                        "generator": "live_journey.review_return",
                        "simulated": true,
                        "delivered": false,
                        "elapsedDays": REVIEW_RETURN_ELAPSED_DAYS,
                        "emailAdapter": "not_implemented",
                        "issueOwner": "#170",
                    }),
                    content_hash: stable_eval_content_hash(&simulated_email_body),
                    storage_uri: None,
                    health_status: Some("simulated_not_delivered".to_string()),
                    created_by_job_id: None,
                },
            )?;
            state.simulated_email_artifact_id = Some(email_artifact.id.clone());

            let return_slug = format!("review-return-{}", slug_fragment(&state.persona.persona_id));
            let (return_entry, _) = create_entry_point(
                db_path,
                EntryPointWriteRequest {
                    slug: return_slug.clone(),
                    label: "Studio Ordo review return link".to_string(),
                    status: Some(EntryPointStatus::Active),
                    source_kind: "simulated_review_request_link".to_string(),
                    source_label: Some("Simulated review request email".to_string()),
                    destination_surface: PublicDestinationSurface::Offers,
                    destination_id: Some(state.qr_summary.offer_id.clone()),
                    attribution: Some(json!({
                        "campaign": "review_return_eval",
                        "personaId": state.persona.persona_id,
                        "source": "simulated_review_request_email",
                        "simulatedEmailArtifactId": email_artifact.id,
                        "delivered": false,
                    })),
                    metadata: Some(json!({
                        "evalCase": "review_return",
                        "trialId": state.qr_summary.trial_id,
                        "conversationId": state.qr_summary.conversation_id,
                    })),
                },
                None,
            )?;
            state.return_entry_point_id = Some(return_entry.id.clone());
            state.return_entry_point_slug = Some(return_entry.slug.clone());

            let (return_session, _) = create_visitor_session(
                db_path,
                VisitorSessionCreateRequest {
                    entry_point_slug: return_entry.slug.clone(),
                    user_agent: Some("Ordo review return eval mobile browser".to_string()),
                    attribution: Some(json!({
                        "personaId": state.persona.persona_id,
                        "trialId": state.qr_summary.trial_id,
                        "simulatedEmailArtifactId": email_artifact.id,
                    })),
                },
            )?;
            state.return_visitor_session_id = Some(return_session.id.clone());

            let return_visitor = create_conversation_participant(
                connection,
                &ConversationParticipantCreateRequest {
                    conversation_id: state.qr_summary.conversation_id.clone(),
                    participant_kind: "visitor".to_string(),
                    actor_id: None,
                    connection_id: None,
                    visitor_session_id: Some(return_session.id.clone()),
                    display_name: state.persona.display_name.clone(),
                    role: "trial_reviewer".to_string(),
                },
            )?;
            let return_message = create_conversation_message(
                connection,
                &ConversationMessageCreateRequest {
                    conversation_id: state.qr_summary.conversation_id.clone(),
                    segment_id: None,
                    participant_id: return_visitor.id.clone(),
                    message_kind: "message".to_string(),
                    body_markdown: persona_backed_review_return_message(&state.persona),
                    visibility: "participants".to_string(),
                    client_message_id: format!(
                        "review-return-message-{}",
                        state.persona.persona_id
                    ),
                    reply_to_message_id: None,
                    undo_expires_at: None,
                },
            )?;
            state.return_message_id = Some(return_message.id.clone());

            let assistant = create_conversation_participant(
                connection,
                &ConversationParticipantCreateRequest {
                    conversation_id: state.qr_summary.conversation_id.clone(),
                    participant_kind: "agent".to_string(),
                    actor_id: None,
                    connection_id: None,
                    visitor_session_id: None,
                    display_name: "Ordo".to_string(),
                    role: "assistant".to_string(),
                },
            )?;
            let gateway = LlmGateway::new(DeterministicLlmProvider::new("local_fake", "fake-chat"))
                .with_private_terms(vec![
                    "Project Orchid".to_string(),
                    "Project".to_string(),
                    "Orchid".to_string(),
                    "review-return-secret".to_string(),
                    "alex@example.com".to_string(),
                ]);
            let llm_result = gateway.run_completion(
                db_path,
                connection,
                &ActorContext::local_owner("live_journey_review_return_eval"),
                LlmGatewayRequest {
                    run_id: format!("live_journey_review_return_{}", state.persona.persona_id),
                    conversation_id: state.qr_summary.conversation_id.clone(),
                    segment_id: None,
                    assistant_participant_id: assistant.id,
                    client_id: Some(format!("review-return-{}", state.persona.persona_id)),
                    provider_id: "local_fake".to_string(),
                    model_id: "fake-chat".to_string(),
                    user_message: return_message.body_markdown.clone(),
                    prompt_slots: vec![PromptSlot::new(
                        "review_return_context",
                        "Review Return Context",
                        "Ask for private feedback first. Explain that a public review requires explicit consent and approval. Do not invent proof or treat feedback as a review.",
                        vec![
                            format!("artifact:{}", email_artifact.id),
                            format!("message:{}", return_message.id),
                            format!("trial:{}", state.qr_summary.trial_id),
                        ],
                        "Review-return journey evidence.",
                        "participants",
                    )?],
                },
            )?;
            let assistant_message = llm_result.final_message.ok_or_else(|| {
                anyhow!("deterministic review-return LLM path produced no message")
            })?;
            state.assistant_message_id = Some(assistant_message.id.clone());

            let (feedback, _) = capture_feedback(
                connection,
                CustomerFeedbackInput {
                    connection_id: None,
                    conversation_id: state.qr_summary.conversation_id.clone(),
                    segment_id: None,
                    message_id: Some(return_message.id.clone()),
                    feedback_kind: "trial_experience".to_string(),
                    body_summary: "Trial user reported that OrdoStudio made follow-up feel clearer while still wanting proof before committing.".to_string(),
                    source_refs: vec![
                        format!("message:{}", return_message.id),
                        format!("trial:{}", state.qr_summary.trial_id),
                    ],
                    evidence_refs: vec![
                        format!("message:{}", return_message.id),
                        format!("artifact:{}", email_artifact.id),
                    ],
                    provenance: json!({
                        "generator": "live_journey.review_return",
                        "privateBusinessIntelligence": true,
                        "notPublicReview": true,
                    }),
                },
            )?;
            state.feedback_id = Some(feedback.id.clone());
            ensure!(
                list_private_feedback(connection, &state.qr_summary.conversation_id)?
                    .iter()
                    .any(|item| item.id == feedback.id),
                "review-return feedback must remain private"
            );

            let (review, _) = create_review_candidate(
                connection,
                &feedback.id,
                ReviewCandidateInput {
                    review_body: "OrdoStudio helped me see the next relationship follow-up more clearly during the trial.".to_string(),
                    evidence_refs: vec![
                        format!("feedback:{}", feedback.id),
                        format!("message:{}", return_message.id),
                    ],
                    provenance: json!({
                        "generator": "live_journey.review_return",
                        "candidateOnly": true,
                    }),
                },
            )?;
            state.review_id = Some(review.id.clone());
            state.public_review_count_before_publish = list_public_reviews(connection)?.len();
            state.blocked_publish_without_consent_or_approval = transition_review(
                connection,
                &review.id,
                ReviewStatus::Published,
                vec![format!("message:{}", return_message.id)],
                "blocked early publish attempt",
            )
            .is_err();
            ensure!(
                state.blocked_publish_without_consent_or_approval,
                "review publication must be blocked before consent and approval"
            );

            let (requested, _) = transition_review(
                connection,
                &review.id,
                ReviewStatus::Requested,
                vec![format!("artifact:{}", email_artifact.id)],
                "simulated review request sent as local artifact only",
            )?;
            let (received, _) = transition_review(
                connection,
                &requested.id,
                ReviewStatus::Received,
                vec![format!("message:{}", return_message.id)],
                "visitor returned with trial feedback",
            )?;
            let (consented, _) = transition_review(
                connection,
                &received.id,
                ReviewStatus::ConsentConfirmed,
                vec![format!("message:{}", return_message.id)],
                "visitor explicitly consented in eval fixture",
            )?;
            let (approved, _) = transition_review(
                connection,
                &consented.id,
                ReviewStatus::Approved,
                vec![format!("message:{}", assistant_message.id)],
                "operator approval represented by deterministic eval evidence",
            )?;
            let (published, _) = transition_review(
                connection,
                &approved.id,
                ReviewStatus::Published,
                vec![
                    format!("message:{}", return_message.id),
                    format!("message:{}", assistant_message.id),
                ],
                "publish after consent and approval",
            )?;
            state.public_review_count_after_publish = list_public_reviews(connection)?.len();
            let (featured, _) = transition_review(
                connection,
                &published.id,
                ReviewStatus::Featured,
                vec![format!("review:{}", published.id)],
                "feature published review in eval lifecycle",
            )?;
            let (retired, _) = transition_review(
                connection,
                &featured.id,
                ReviewStatus::Retired,
                vec![format!("review:{}", featured.id)],
                "retire review in eval lifecycle",
            )?;
            state.final_review_status = Some(retired.status.as_str().to_string());
            state.public_review_count_after_retire = list_public_reviews(connection)?.len();
        }
        other => anyhow::bail!("unsupported review-return journey step: {other}"),
    }
    Ok(())
}

pub(crate) fn ensure_review_return_evidence(evidence: &ReviewReturnJourneyEvidence) -> Result<()> {
    ensure!(
        !evidence.simulated_email_artifact_id.is_empty(),
        "simulated email artifact evidence missing"
    );
    ensure!(
        !evidence.return_entry_point_id.is_empty(),
        "return entry point evidence missing"
    );
    ensure!(
        !evidence.return_visitor_session_id.is_empty(),
        "return visitor session evidence missing"
    );
    ensure!(
        evidence.blocked_publish_without_consent_or_approval,
        "review publication must be blocked before consent and approval"
    );
    ensure!(
        evidence.public_review_count_before_publish == 0,
        "candidate review should not be public before consent and approval"
    );
    ensure!(
        evidence.public_review_count_after_publish >= 1,
        "approved published review should become public"
    );
    ensure!(
        evidence.public_review_count_after_retire == 0,
        "retired review should leave public review listing"
    );
    ensure!(
        evidence.final_review_status == ReviewStatus::Retired.as_str(),
        "review lifecycle should finish retired in the eval"
    );
    Ok(())
}

pub(crate) fn ensure_review_return_manifest_is_safe(
    manifest: &ReviewReturnJourneyManifest,
    private_terms: &[String],
) -> Result<()> {
    let value = serde_json::to_value(manifest)?;
    ensure!(
        !contains_sensitive_value(&value, private_terms),
        "review-return journey manifest contains raw sensitive value"
    );
    Ok(())
}

pub(crate) fn persona_backed_review_return_message(persona: &EvalPersona) -> String {
    format!(
        "I came back from the review request link after trying the 30-day trial. As a {} user, the follow-up flow felt clearer and I consent to discussing whether one quote can become a public review after approval. Keep this private unless I consent. Do not repeat Project Orchid, alex@example.com, or review-return-secret.",
        persona.person_type
    )
}

pub(crate) fn stable_eval_content_hash(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    format!("sha256:{:x}", hasher.finalize())
}

