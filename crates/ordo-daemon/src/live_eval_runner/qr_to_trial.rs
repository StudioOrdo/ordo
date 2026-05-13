use super::*;
use anyhow::{anyhow, ensure, Result};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QrToTrialJourneyEvidence {
    pub persona_id: String,
    pub persona_content_hash: String,
    pub case_id: String,
    pub entry_point_id: String,
    pub entry_point_slug: String,
    pub visitor_session_id: String,
    pub conversation_id: String,
    pub visitor_message_id: String,
    pub assistant_message_id: String,
    pub offer_id: String,
    pub offer_slug: String,
    pub acceptance_id: String,
    pub trial_id: String,
    pub trial_status: String,
    pub outcome_ids: Vec<String>,
    pub attribution_count: usize,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QrToTrialJourneyManifest {
    pub schema_version: String,
    pub source_commit: String,
    pub guard: LiveEvalGuardDecision,
    pub provider_mode: String,
    pub network_enabled: bool,
    pub evidence: QrToTrialJourneyEvidence,
    pub packet_path: String,
    pub scorecard_path: String,
    pub manifest_path: String,
    pub redaction_detectors: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QrToTrialJourneyRunSummary {
    pub schema_version: String,
    pub status: LiveEvalStatus,
    pub case_id: String,
    pub persona_id: String,
    pub provider_mode: String,
    pub network_enabled: bool,
    pub entry_point_id: String,
    pub visitor_session_id: String,
    pub conversation_id: String,
    pub offer_id: String,
    pub acceptance_id: String,
    pub trial_id: String,
    pub outcome_count: usize,
    pub attribution_count: usize,
    pub packet_path: String,
    pub scorecard_path: String,
    pub manifest_path: String,
    pub journey_manifest_path: String,
    pub message: String,
}

pub fn run_qr_to_trial_journey_eval(
    db_path: &Path,
    connection: &Connection,
    persona_dir: &Path,
    selected_persona_id: Option<&str>,
    output_dir: impl Into<PathBuf>,
    source_commit: impl Into<String>,
    private_terms: Vec<String>,
) -> Result<QrToTrialJourneyRunSummary> {
    let personas = load_persona_dir(persona_dir, &private_terms)?;
    let persona = select_qr_to_trial_persona(&personas, selected_persona_id)?;
    let case = qr_to_trial_journey_case(&persona)?;
    let output_dir = output_dir.into();
    let packet_path = output_dir.join(format!("{}-packet.json", case.id));
    let source_commit = source_commit.into();
    let mut harness = DeterministicEvalHarness::new(DeterministicEvalClock::fixed())
        .with_artifact_path(packet_path.to_string_lossy());
    let mut state = QrToTrialJourneyState::new(persona.clone());
    let mut scorecard = harness.run_case(connection, &case, |connection, step| {
        run_qr_to_trial_journey_step(db_path, connection, step, &mut state)
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
    let writer = EvalArtifactWriter::new(&output_dir, &source_commit)
        .with_private_terms(writer_private_terms.clone());
    let artifact_paths = writer.write_packet(connection, &case, &scorecard)?;
    let evidence = state.into_evidence(connection)?;
    ensure_qr_to_trial_evidence(&evidence)?;
    let journey_manifest_path = output_dir.join(format!("{}-journey.json", case.id));
    let manifest = QrToTrialJourneyManifest {
        schema_version: QR_TO_TRIAL_JOURNEY_SCHEMA_VERSION.to_string(),
        source_commit,
        guard: LiveEvalGuardDecision {
            status: LiveEvalStatus::Completed,
            reason: "QR-to-trial journey used deterministic provider path; live network remains guarded for later manual runs.".to_string(),
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
        ],
    };
    ensure_qr_to_trial_manifest_is_safe(&manifest, &writer_private_terms)?;
    write_json(&journey_manifest_path, &manifest)?;

    Ok(QrToTrialJourneyRunSummary {
        schema_version: QR_TO_TRIAL_JOURNEY_SCHEMA_VERSION.to_string(),
        status: if scorecard.passed {
            LiveEvalStatus::Completed
        } else {
            LiveEvalStatus::Failed
        },
        case_id: case.id,
        persona_id: evidence.persona_id,
        provider_mode: "deterministic_live_journey".to_string(),
        network_enabled: false,
        entry_point_id: evidence.entry_point_id,
        visitor_session_id: evidence.visitor_session_id,
        conversation_id: evidence.conversation_id,
        offer_id: evidence.offer_id,
        acceptance_id: evidence.acceptance_id,
        trial_id: evidence.trial_id,
        outcome_count: evidence.outcome_ids.len(),
        attribution_count: evidence.attribution_count,
        packet_path: artifact_paths.packet_path.to_string_lossy().to_string(),
        scorecard_path: artifact_paths.scorecard_path.to_string_lossy().to_string(),
        manifest_path: artifact_paths.manifest_path.to_string_lossy().to_string(),
        journey_manifest_path: journey_manifest_path.to_string_lossy().to_string(),
        message: if scorecard.passed {
            "QR-to-trial journey eval completed without provider network.".to_string()
        } else {
            "QR-to-trial journey eval completed with failed assertions.".to_string()
        },
    })
}

#[derive(Debug, Clone)]
pub(crate) struct QrToTrialJourneyState {
    persona: EvalPersona,
    entry_point_id: Option<String>,
    entry_point_slug: Option<String>,
    visitor_session_id: Option<String>,
    conversation_id: Option<String>,
    visitor_participant_id: Option<String>,
    assistant_participant_id: Option<String>,
    visitor_message_id: Option<String>,
    assistant_message_id: Option<String>,
    offer_id: Option<String>,
    offer_slug: Option<String>,
    acceptance_id: Option<String>,
    trial_id: Option<String>,
    trial_status: Option<String>,
}

impl QrToTrialJourneyState {
    fn new(persona: EvalPersona) -> Self {
        Self {
            persona,
            entry_point_id: None,
            entry_point_slug: None,
            visitor_session_id: None,
            conversation_id: None,
            visitor_participant_id: None,
            assistant_participant_id: None,
            visitor_message_id: None,
            assistant_message_id: None,
            offer_id: None,
            offer_slug: None,
            acceptance_id: None,
            trial_id: None,
            trial_status: None,
        }
    }

    fn into_evidence(self, connection: &Connection) -> Result<QrToTrialJourneyEvidence> {
        let offer_id = required_state(self.offer_id, "offer id")?;
        let outcomes = list_outcomes_by_offer(connection, &offer_id)?;
        let attribution_count = outcomes
            .iter()
            .map(|outcome| list_attributions_for_outcome(connection, &outcome.id))
            .collect::<Result<Vec<_>>>()?
            .into_iter()
            .map(|items| items.len())
            .sum();
        Ok(QrToTrialJourneyEvidence {
            persona_id: self.persona.persona_id.clone(),
            persona_content_hash: self.persona.content_hash,
            case_id: format!(
                "{QR_TO_TRIAL_JOURNEY_CASE_PREFIX}_{}",
                self.persona.persona_id
            ),
            entry_point_id: required_state(self.entry_point_id, "entry point id")?,
            entry_point_slug: required_state(self.entry_point_slug, "entry point slug")?,
            visitor_session_id: required_state(self.visitor_session_id, "visitor session id")?,
            conversation_id: required_state(self.conversation_id, "conversation id")?,
            visitor_message_id: required_state(self.visitor_message_id, "visitor message id")?,
            assistant_message_id: required_state(
                self.assistant_message_id,
                "assistant message id",
            )?,
            offer_id,
            offer_slug: required_state(self.offer_slug, "offer slug")?,
            acceptance_id: required_state(self.acceptance_id, "acceptance id")?,
            trial_id: required_state(self.trial_id, "trial id")?,
            trial_status: required_state(self.trial_status, "trial status")?,
            outcome_ids: outcomes.into_iter().map(|outcome| outcome.id).collect(),
            attribution_count,
            evidence_refs: vec![
                "entry_point".to_string(),
                "visitor_session".to_string(),
                "conversation_message".to_string(),
                "llm_prompt_slot_usage".to_string(),
                "privacy_egress_transform".to_string(),
                "offer_acceptance".to_string(),
                "trial".to_string(),
                "business_outcome".to_string(),
                "business_outcome_attribution".to_string(),
            ],
        })
    }
}

pub(crate) fn select_qr_to_trial_persona(
    personas: &[EvalPersona],
    selected_persona_id: Option<&str>,
) -> Result<EvalPersona> {
    match selected_persona_id {
        Some(id) => personas
            .iter()
            .find(|persona| persona.persona_id == id)
            .cloned()
            .ok_or_else(|| anyhow!("unknown QR-to-trial persona id {id}")),
        None => personas
            .first()
            .cloned()
            .ok_or_else(|| anyhow!("persona library is empty")),
    }
}

pub(crate) fn qr_to_trial_journey_case(persona: &EvalPersona) -> Result<EvalCase> {
    EvalCase::new(
        format!("{QR_TO_TRIAL_JOURNEY_CASE_PREFIX}_{}", persona.persona_id),
        "QR event to 30-day trial journey",
        &json!({
            "fixture": "qr_to_trial_journey",
            "version": 1,
            "personaId": persona.persona_id,
            "personaHash": persona.content_hash,
            "providerMode": "deterministic_live_journey",
            "networkRequired": false,
            "deferredPhases": ["review_return", "affiliate_referral", "admin_staff_handoff", "cross_persona_report"],
        }),
        vec![
            EvalActorRole::AnonymousVisitor,
            EvalActorRole::OrdoAgent,
            EvalActorRole::LlmToolProviderBoundary,
        ],
        vec![EvalStep::new(
            "run_qr_event_to_trial_acceptance",
            EvalActorRole::AnonymousVisitor,
            "live_journey.qr_to_trial",
            vec![
                EvalEvidenceChannel::SqliteRows,
                EvalEvidenceChannel::ConversationEvents,
                EvalEvidenceChannel::RealtimeReplay,
                EvalEvidenceChannel::PolicyDecisions,
                EvalEvidenceChannel::PromptSlotAccounting,
                EvalEvidenceChannel::PrivacyTransforms,
                EvalEvidenceChannel::TokenLedger,
            ],
        )?],
        vec![
            EvalAssertion::minimum_count(
                "durable_sqlite_rows_recorded",
                EvalEvidenceChannel::SqliteRows,
                20,
            )?,
            EvalAssertion::minimum_count(
                "conversation_events_recorded",
                EvalEvidenceChannel::ConversationEvents,
                8,
            )?,
            EvalAssertion::minimum_count(
                "realtime_replay_recorded",
                EvalEvidenceChannel::RealtimeReplay,
                8,
            )?,
            EvalAssertion::minimum_count(
                "llm_policy_decision_recorded",
                EvalEvidenceChannel::PolicyDecisions,
                1,
            )?,
            EvalAssertion::minimum_count(
                "prompt_slot_accounting_recorded",
                EvalEvidenceChannel::PromptSlotAccounting,
                1,
            )?,
            EvalAssertion::minimum_count(
                "privacy_transform_recorded",
                EvalEvidenceChannel::PrivacyTransforms,
                1,
            )?,
            EvalAssertion::minimum_count(
                "token_ledger_recorded",
                EvalEvidenceChannel::TokenLedger,
                2,
            )?,
        ],
    )
}

pub(crate) fn run_qr_to_trial_journey_step(
    db_path: &Path,
    connection: &Connection,
    step: &EvalStep,
    state: &mut QrToTrialJourneyState,
) -> Result<()> {
    match step.id.as_str() {
        "run_qr_event_to_trial_acceptance" => {
            let offer_slug = format!(
                "ordostudio-30-day-{}",
                slug_fragment(&state.persona.persona_id)
            );
            let (offer, _) = create_offer(
                db_path,
                OfferWriteRequest {
                    slug: offer_slug.clone(),
                    title: "OrdoStudio 30-day trial".to_string(),
                    summary:
                        "A 30-day Studio Ordo trial for relationship-first business operations."
                            .to_string(),
                    status: Some(OfferStatus::Available),
                    visibility: Some(BusinessFactVisibility::Public),
                    publication_state: Some(PublicationState::Published),
                    trial_days: Some(30),
                    source_kind: Some("live_journey_eval".to_string()),
                    source_ref: Some(state.persona.persona_id.clone()),
                    terms: Some(json!({
                        "trialDays": 30,
                        "billing": "manual_follow_up",
                        "claims": "evidence_backed_only",
                        "noFakeScarcity": true,
                    })),
                    metadata: Some(json!({
                        "personaId": state.persona.persona_id,
                        "personaHash": state.persona.content_hash,
                    })),
                },
                None,
            )?;
            state.offer_id = Some(offer.id.clone());
            state.offer_slug = Some(offer.slug.clone());

            let entry_slug = format!("event-qr-{}", slug_fragment(&state.persona.persona_id));
            let (entry_point, _) = create_entry_point(
                db_path,
                EntryPointWriteRequest {
                    slug: entry_slug.clone(),
                    label: "Studio Ordo event QR".to_string(),
                    status: Some(EntryPointStatus::Active),
                    source_kind: "event_qr".to_string(),
                    source_label: Some("Live journey eval event".to_string()),
                    destination_surface: PublicDestinationSurface::Offers,
                    destination_id: Some(offer.id.clone()),
                    attribution: Some(json!({
                        "campaign": "live_product_journey_eval",
                        "personaId": state.persona.persona_id,
                        "source": "event_qr",
                    })),
                    metadata: Some(json!({
                        "evalCase": "qr_to_trial",
                        "personaHash": state.persona.content_hash,
                    })),
                },
                None,
            )?;
            state.entry_point_id = Some(entry_point.id.clone());
            state.entry_point_slug = Some(entry_point.slug.clone());

            let (visitor_session, _) = create_visitor_session(
                db_path,
                VisitorSessionCreateRequest {
                    entry_point_slug: entry_point.slug.clone(),
                    user_agent: Some("Ordo live journey eval mobile browser".to_string()),
                    attribution: Some(json!({
                        "personaId": state.persona.persona_id,
                        "entryPointId": entry_point.id,
                    })),
                },
            )?;
            state.visitor_session_id = Some(visitor_session.id.clone());

            let conversation = find_or_create_canonical_conversation(
                connection,
                &CanonicalConversationRequest {
                    surface: "chat".to_string(),
                    subject_kind: "visitor_session".to_string(),
                    subject_id: visitor_session.id.clone(),
                    connection_id: None,
                    visitor_session_id: Some(visitor_session.id.clone()),
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
                    visitor_session_id: Some(visitor_session.id.clone()),
                    display_name: state.persona.display_name.clone(),
                    role: "prospective_client".to_string(),
                },
            )?;
            state.visitor_participant_id = Some(visitor.id.clone());
            let assistant = create_conversation_participant(
                connection,
                &ConversationParticipantCreateRequest {
                    conversation_id: conversation.id.clone(),
                    participant_kind: "agent".to_string(),
                    actor_id: None,
                    connection_id: None,
                    visitor_session_id: None,
                    display_name: "Ordo".to_string(),
                    role: "assistant".to_string(),
                },
            )?;
            state.assistant_participant_id = Some(assistant.id.clone());
            let visitor_message = create_conversation_message(
                connection,
                &ConversationMessageCreateRequest {
                    conversation_id: conversation.id.clone(),
                    segment_id: None,
                    participant_id: visitor.id.clone(),
                    message_kind: "message".to_string(),
                    body_markdown: persona_backed_visitor_message(&state.persona),
                    visibility: "participants".to_string(),
                    client_message_id: format!("client-message-{}", state.persona.persona_id),
                    reply_to_message_id: None,
                    undo_expires_at: None,
                },
            )?;
            state.visitor_message_id = Some(visitor_message.id.clone());

            let gateway = LlmGateway::new(DeterministicLlmProvider::new("local_fake", "fake-chat"))
                .with_private_terms(vec![
                    "Project Orchid".to_string(),
                    "Project".to_string(),
                    "Orchid".to_string(),
                    "sk-live-journey-fixture".to_string(),
                    "alex@example.com".to_string(),
                ]);
            let llm_result = gateway.run_completion(
                db_path,
                connection,
                &ActorContext::local_owner("live_journey_qr_to_trial_eval"),
                LlmGatewayRequest {
                    run_id: format!("live_journey_qr_to_trial_{}", state.persona.persona_id),
                    conversation_id: conversation.id.clone(),
                    segment_id: None,
                    assistant_participant_id: assistant.id.clone(),
                    client_id: Some(format!("qr-to-trial-{}", state.persona.persona_id)),
                    provider_id: "local_fake".to_string(),
                    model_id: "fake-chat".to_string(),
                    user_message: visitor_message.body_markdown.clone(),
                    prompt_slots: vec![
                        PromptSlot::new(
                            "ethical_business_persuasion",
                            "Ethical Business Persuasion",
                            "Use evidence-backed reciprocity and commitment only where supported; preserve agency; do not use fake urgency, scarcity, reviews, metrics, shame, fear, or hidden pressure.",
                            vec![
                                format!("entry_point:{}", entry_point.id),
                                format!("offer:{}", offer.id),
                                format!("message:{}", visitor_message.id),
                            ],
                            "The journey evaluates respectful signup guidance for a 30-day trial.",
                            "staff_private",
                        )?,
                        PromptSlot::new(
                            "offer_trial_context",
                            "Offer Trial Context",
                            "Studio Ordo has a public 30-day trial offer; recommend it only as an option the visitor can decline.",
                            vec![
                                format!("offer:{}", offer.id),
                                format!("visitor_session:{}", visitor_session.id),
                            ],
                            "Durable offer and visitor-session evidence for the QR journey.",
                            "participants",
                        )?,
                    ],
                },
            )?;
            let assistant_message = llm_result
                .final_message
                .ok_or_else(|| anyhow!("deterministic QR-to-trial LLM path produced no message"))?;
            state.assistant_message_id = Some(assistant_message.id.clone());

            let (acceptance, trial, _) = accept_public_offer(
                db_path,
                &offer.slug,
                OfferAcceptanceCreateRequest {
                    visitor_session_id: Some(visitor_session.id.clone()),
                    attribution: Some(json!({
                        "personaId": state.persona.persona_id,
                        "conversationId": conversation.id,
                        "visitorMessageId": visitor_message.id,
                        "assistantMessageId": assistant_message.id,
                        "entryPointId": entry_point.id,
                    })),
                    acceptance_context: Some(json!({
                        "decision": "accepted_30_day_trial",
                        "agencyPreserving": true,
                        "evidenceRefs": [
                            format!("entry_point:{}", entry_point.id),
                            format!("visitor_session:{}", visitor_session.id),
                            format!("conversation:{}", conversation.id),
                            format!("message:{}", visitor_message.id),
                            format!("message:{}", assistant_message.id),
                            format!("offer:{}", offer.id)
                        ],
                        "nonGoals": [
                            "no_fake_urgency",
                            "no_fake_scarcity",
                            "no_unsupported_social_proof"
                        ]
                    })),
                },
            )?;
            state.acceptance_id = Some(acceptance.id);
            state.trial_id = Some(trial.id);
            state.trial_status = Some(format!("{:?}", trial.status));
        }
        other => anyhow::bail!("unsupported QR-to-trial journey step: {other}"),
    }
    Ok(())
}

pub(crate) fn ensure_qr_to_trial_evidence(evidence: &QrToTrialJourneyEvidence) -> Result<()> {
    ensure!(
        !evidence.entry_point_id.is_empty(),
        "entry point evidence missing"
    );
    ensure!(
        !evidence.visitor_session_id.is_empty(),
        "visitor session evidence missing"
    );
    ensure!(
        !evidence.conversation_id.is_empty(),
        "conversation evidence missing"
    );
    ensure!(!evidence.offer_id.is_empty(), "offer evidence missing");
    ensure!(
        !evidence.acceptance_id.is_empty(),
        "acceptance evidence missing"
    );
    ensure!(!evidence.trial_id.is_empty(), "trial evidence missing");
    ensure!(
        evidence.trial_status == "Started",
        "QR-to-trial eval should create a started 30-day trial"
    );
    ensure!(
        !evidence.outcome_ids.is_empty(),
        "business outcome evidence missing"
    );
    ensure!(
        evidence.attribution_count >= 3,
        "offer, visitor session, and entry point attribution evidence required"
    );
    Ok(())
}

pub(crate) fn ensure_qr_to_trial_manifest_is_safe(
    manifest: &QrToTrialJourneyManifest,
    private_terms: &[String],
) -> Result<()> {
    let value = serde_json::to_value(manifest)?;
    ensure!(
        !contains_sensitive_value(&value, private_terms),
        "QR-to-trial journey manifest contains raw sensitive value"
    );
    Ok(())
}

pub(crate) fn persona_backed_visitor_message(persona: &EvalPersona) -> String {
    format!(
        "I scanned your event QR code. I run a {} practice and I am considering whether a 30-day OrdoStudio trial fits. My budget sensitivity is {}, my urgency is {}, and I want a plain recommendation without fake scarcity or hype. Please do not repeat Project Orchid, alex@example.com, or sk-live-journey-fixture.",
        persona.person_type, persona.budget_sensitivity, persona.urgency_level
    )
}

pub(crate) fn slug_fragment(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>()
}

pub(crate) fn required_state(value: Option<String>, label: &str) -> Result<String> {
    value.ok_or_else(|| anyhow!("QR-to-trial journey missing {label}"))
}


