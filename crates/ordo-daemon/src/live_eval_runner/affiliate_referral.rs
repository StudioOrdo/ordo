use super::*;
use anyhow::{anyhow, ensure, Result};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AffiliateReferralJourneyEvidence {
    pub persona_id: String,
    pub case_id: String,
    pub affiliate_connection_id: String,
    pub affiliate_grant_id: String,
    pub referral_entry_point_id: String,
    pub referral_entry_point_slug: String,
    pub referred_visitor_session_id: String,
    pub conversation_id: String,
    pub referred_message_id: String,
    pub assistant_message_id: String,
    pub offer_id: String,
    pub acceptance_id: String,
    pub trial_id: String,
    pub referral_id: String,
    pub referral_outcome_id: String,
    pub attribution_count: usize,
    pub affiliate_allowed_conversation_read: bool,
    pub affiliate_denied_unrelated_conversation_read: bool,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AffiliateReferralJourneyManifest {
    pub schema_version: String,
    pub source_commit: String,
    pub guard: LiveEvalGuardDecision,
    pub provider_mode: String,
    pub network_enabled: bool,
    pub evidence: AffiliateReferralJourneyEvidence,
    pub packet_path: String,
    pub scorecard_path: String,
    pub manifest_path: String,
    pub redaction_detectors: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AffiliateReferralJourneyRunSummary {
    pub schema_version: String,
    pub status: LiveEvalStatus,
    pub case_id: String,
    pub persona_id: String,
    pub provider_mode: String,
    pub network_enabled: bool,
    pub affiliate_connection_id: String,
    pub referral_entry_point_id: String,
    pub referred_visitor_session_id: String,
    pub conversation_id: String,
    pub offer_id: String,
    pub acceptance_id: String,
    pub trial_id: String,
    pub referral_id: String,
    pub referral_outcome_id: String,
    pub attribution_count: usize,
    pub packet_path: String,
    pub scorecard_path: String,
    pub manifest_path: String,
    pub journey_manifest_path: String,
    pub message: String,
}

pub fn run_affiliate_referral_journey_eval(
    db_path: &Path,
    connection: &Connection,
    persona_dir: &Path,
    selected_persona_id: Option<&str>,
    output_dir: impl Into<PathBuf>,
    source_commit: impl Into<String>,
    private_terms: Vec<String>,
) -> Result<AffiliateReferralJourneyRunSummary> {
    let personas = load_persona_dir(persona_dir, &private_terms)?;
    let persona = select_affiliate_referral_persona(&personas, selected_persona_id)?;
    let case = affiliate_referral_journey_case(&persona)?;
    let output_dir = output_dir.into();
    let packet_path = output_dir.join(format!("{}-packet.json", case.id));
    let source_commit = source_commit.into();
    let mut harness = DeterministicEvalHarness::new(DeterministicEvalClock::fixed())
        .with_artifact_path(packet_path.to_string_lossy());
    let mut state = AffiliateReferralJourneyState::new(persona.clone());
    let mut scorecard = harness.run_case(connection, &case, |connection, step| {
        run_affiliate_referral_journey_step(db_path, connection, step, &mut state)
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
    writer_private_terms.push("affiliate-referral-secret".to_string());
    let writer = EvalArtifactWriter::new(&output_dir, &source_commit)
        .with_private_terms(writer_private_terms.clone());
    let artifact_paths = writer.write_packet(connection, &case, &scorecard)?;
    let evidence = state.into_evidence(connection)?;
    ensure_affiliate_referral_evidence(&evidence)?;
    let journey_manifest_path = output_dir.join(format!("{}-journey.json", case.id));
    let manifest = AffiliateReferralJourneyManifest {
        schema_version: AFFILIATE_REFERRAL_JOURNEY_SCHEMA_VERSION.to_string(),
        source_commit,
        guard: LiveEvalGuardDecision {
            status: LiveEvalStatus::Completed,
            reason: "Affiliate referral journey used deterministic provider path; no provider network ran.".to_string(),
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
    ensure_affiliate_referral_manifest_is_safe(&manifest, &writer_private_terms)?;
    write_json(&journey_manifest_path, &manifest)?;

    Ok(AffiliateReferralJourneyRunSummary {
        schema_version: AFFILIATE_REFERRAL_JOURNEY_SCHEMA_VERSION.to_string(),
        status: if scorecard.passed {
            LiveEvalStatus::Completed
        } else {
            LiveEvalStatus::Failed
        },
        case_id: case.id,
        persona_id: evidence.persona_id,
        provider_mode: "deterministic_live_journey".to_string(),
        network_enabled: false,
        affiliate_connection_id: evidence.affiliate_connection_id,
        referral_entry_point_id: evidence.referral_entry_point_id,
        referred_visitor_session_id: evidence.referred_visitor_session_id,
        conversation_id: evidence.conversation_id,
        offer_id: evidence.offer_id,
        acceptance_id: evidence.acceptance_id,
        trial_id: evidence.trial_id,
        referral_id: evidence.referral_id,
        referral_outcome_id: evidence.referral_outcome_id,
        attribution_count: evidence.attribution_count,
        packet_path: artifact_paths.packet_path.to_string_lossy().to_string(),
        scorecard_path: artifact_paths.scorecard_path.to_string_lossy().to_string(),
        manifest_path: artifact_paths.manifest_path.to_string_lossy().to_string(),
        journey_manifest_path: journey_manifest_path.to_string_lossy().to_string(),
        message: if scorecard.passed {
            "Affiliate referral journey eval completed without provider keys or network."
                .to_string()
        } else {
            "Affiliate referral journey eval completed with failed assertions.".to_string()
        },
    })
}

#[derive(Debug, Clone)]
pub(crate) struct AffiliateReferralJourneyState {
    persona: EvalPersona,
    affiliate_connection_id: Option<String>,
    affiliate_grant_id: Option<String>,
    referral_entry_point_id: Option<String>,
    referral_entry_point_slug: Option<String>,
    referred_visitor_session_id: Option<String>,
    conversation_id: Option<String>,
    referred_message_id: Option<String>,
    assistant_message_id: Option<String>,
    offer_id: Option<String>,
    offer_slug: Option<String>,
    acceptance_id: Option<String>,
    trial_id: Option<String>,
    referral_id: Option<String>,
    referral_outcome_id: Option<String>,
    affiliate_allowed_conversation_read: bool,
    affiliate_denied_unrelated_conversation_read: bool,
}

impl AffiliateReferralJourneyState {
    fn new(persona: EvalPersona) -> Self {
        Self {
            persona,
            affiliate_connection_id: None,
            affiliate_grant_id: None,
            referral_entry_point_id: None,
            referral_entry_point_slug: None,
            referred_visitor_session_id: None,
            conversation_id: None,
            referred_message_id: None,
            assistant_message_id: None,
            offer_id: None,
            offer_slug: None,
            acceptance_id: None,
            trial_id: None,
            referral_id: None,
            referral_outcome_id: None,
            affiliate_allowed_conversation_read: false,
            affiliate_denied_unrelated_conversation_read: false,
        }
    }

    fn into_evidence(self, connection: &Connection) -> Result<AffiliateReferralJourneyEvidence> {
        let offer_id = required_state(self.offer_id, "offer id")?;
        let outcomes = list_outcomes_by_offer(connection, &offer_id)?;
        let attribution_count = outcomes
            .iter()
            .map(|outcome| list_attributions_for_outcome(connection, &outcome.id))
            .collect::<Result<Vec<_>>>()?
            .into_iter()
            .map(|items| items.len())
            .sum();
        Ok(AffiliateReferralJourneyEvidence {
            persona_id: self.persona.persona_id.clone(),
            case_id: format!(
                "{AFFILIATE_REFERRAL_JOURNEY_CASE_PREFIX}_{}",
                self.persona.persona_id
            ),
            affiliate_connection_id: required_state(
                self.affiliate_connection_id,
                "affiliate connection id",
            )?,
            affiliate_grant_id: required_state(self.affiliate_grant_id, "affiliate grant id")?,
            referral_entry_point_id: required_state(
                self.referral_entry_point_id,
                "referral entry point id",
            )?,
            referral_entry_point_slug: required_state(
                self.referral_entry_point_slug,
                "referral entry point slug",
            )?,
            referred_visitor_session_id: required_state(
                self.referred_visitor_session_id,
                "referred visitor session id",
            )?,
            conversation_id: required_state(self.conversation_id, "conversation id")?,
            referred_message_id: required_state(self.referred_message_id, "referred message id")?,
            assistant_message_id: required_state(
                self.assistant_message_id,
                "assistant message id",
            )?,
            offer_id,
            acceptance_id: required_state(self.acceptance_id, "acceptance id")?,
            trial_id: required_state(self.trial_id, "trial id")?,
            referral_id: required_state(self.referral_id, "referral id")?,
            referral_outcome_id: required_state(self.referral_outcome_id, "referral outcome id")?,
            attribution_count,
            affiliate_allowed_conversation_read: self.affiliate_allowed_conversation_read,
            affiliate_denied_unrelated_conversation_read: self
                .affiliate_denied_unrelated_conversation_read,
            evidence_refs: vec![
                "affiliate_connection".to_string(),
                "connection_grant".to_string(),
                "referral_entry_point".to_string(),
                "referred_visitor_session".to_string(),
                "relationship_conversation".to_string(),
                "llm_prompt_slot_usage".to_string(),
                "privacy_egress_transform".to_string(),
                "offer_acceptance".to_string(),
                "trial".to_string(),
                "referral_record".to_string(),
                "business_outcome".to_string(),
                "business_outcome_attribution".to_string(),
                "affiliate_visibility_policy".to_string(),
            ],
        })
    }
}

pub(crate) fn select_affiliate_referral_persona(
    personas: &[EvalPersona],
    selected_persona_id: Option<&str>,
) -> Result<EvalPersona> {
    match selected_persona_id {
        Some(id) => personas
            .iter()
            .find(|persona| persona.persona_id == id)
            .cloned()
            .ok_or_else(|| anyhow!("unknown affiliate referral persona id {id}")),
        None => personas
            .iter()
            .find(|persona| {
                persona.person_type == "affiliate_referrer"
                    || persona.referral_tendency == "high"
                    || persona
                        .expected_eval_pressure_subsystems
                        .iter()
                        .any(|subsystem| subsystem.as_str() == "simulator_fixture")
            })
            .cloned()
            .ok_or_else(|| anyhow!("persona library has no affiliate referral candidate")),
    }
}

pub(crate) fn affiliate_referral_journey_case(persona: &EvalPersona) -> Result<EvalCase> {
    EvalCase::new(
        format!(
            "{AFFILIATE_REFERRAL_JOURNEY_CASE_PREFIX}_{}",
            persona.persona_id
        ),
        "Affiliate referral journey",
        &json!({
            "fixture": "affiliate_referral_journey",
            "version": 1,
            "personaId": persona.persona_id,
            "personaHash": persona.content_hash,
            "providerMode": "deterministic_live_journey",
            "networkRequired": false,
            "deferredPhases": ["admin_staff_handoff", "cross_persona_report", "real_email_adapter_decision"],
        }),
        vec![
            EvalActorRole::Affiliate,
            EvalActorRole::AnonymousVisitor,
            EvalActorRole::OrdoAgent,
            EvalActorRole::LlmToolProviderBoundary,
            EvalActorRole::OwnerSystemAdmin,
        ],
        vec![EvalStep::new(
            "run_affiliate_referral_journey",
            EvalActorRole::Affiliate,
            "live_journey.affiliate_referral",
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
                30,
            )?,
            EvalAssertion::minimum_count(
                "conversation_events_recorded",
                EvalEvidenceChannel::ConversationEvents,
                12,
            )?,
            EvalAssertion::minimum_count(
                "realtime_replay_recorded",
                EvalEvidenceChannel::RealtimeReplay,
                12,
            )?,
            EvalAssertion::minimum_count(
                "policy_decisions_recorded",
                EvalEvidenceChannel::PolicyDecisions,
                3,
            )?,
            EvalAssertion::minimum_count(
                "prompt_slot_accounting_recorded",
                EvalEvidenceChannel::PromptSlotAccounting,
                2,
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

pub(crate) fn run_affiliate_referral_journey_step(
    db_path: &Path,
    connection: &Connection,
    step: &EvalStep,
    state: &mut AffiliateReferralJourneyState,
) -> Result<()> {
    match step.id.as_str() {
        "run_affiliate_referral_journey" => {
            let (affiliate, _) = create_connection(
                db_path,
                ConnectionWriteRequest {
                    connection_type: ConnectionType::Affiliate,
                    display_name: "Community referral partner".to_string(),
                    status: Some(ConnectionStatus::Active),
                    identity: Some(json!({
                        "personaId": state.persona.persona_id,
                        "synthetic": true,
                    })),
                    scope: Some(json!({
                        "affiliateReferralEval": true,
                        "financeAutomation": false,
                    })),
                    metadata: Some(json!({
                        "generator": "live_journey.affiliate_referral",
                        "personaHash": state.persona.content_hash,
                    })),
                },
                None,
            )?;
            state.affiliate_connection_id = Some(affiliate.id.clone());

            let offer_slug = format!(
                "affiliate-ordostudio-30-day-{}",
                slug_fragment(&state.persona.persona_id)
            );
            let (offer, _) = create_offer(
                db_path,
                OfferWriteRequest {
                    slug: offer_slug.clone(),
                    title: "OrdoStudio affiliate referral 30-day trial".to_string(),
                    summary:
                        "A 30-day Studio Ordo trial reached through an evidence-backed affiliate referral."
                            .to_string(),
                    status: Some(OfferStatus::Available),
                    visibility: Some(BusinessFactVisibility::Public),
                    publication_state: Some(PublicationState::Published),
                    trial_days: Some(30),
                    source_kind: Some("live_journey_eval".to_string()),
                    source_ref: Some(state.persona.persona_id.clone()),
                    terms: Some(json!({
                        "trialDays": 30,
                        "affiliateReferral": true,
                        "billing": "manual_follow_up",
                        "noFakeReferralMetrics": true,
                        "noFakeScarcity": true,
                    })),
                    metadata: Some(json!({
                        "personaId": state.persona.persona_id,
                        "personaHash": state.persona.content_hash,
                        "affiliateConnectionId": affiliate.id,
                    })),
                },
                None,
            )?;
            state.offer_id = Some(offer.id.clone());
            state.offer_slug = Some(offer.slug.clone());

            let referral_slug = format!(
                "affiliate-referral-{}",
                slug_fragment(&state.persona.persona_id)
            );
            let (entry_point, _) = create_entry_point(
                db_path,
                EntryPointWriteRequest {
                    slug: referral_slug.clone(),
                    label: "Studio Ordo affiliate referral link".to_string(),
                    status: Some(EntryPointStatus::Active),
                    source_kind: "affiliate_referral".to_string(),
                    source_label: Some("Community referral partner".to_string()),
                    destination_surface: PublicDestinationSurface::Offers,
                    destination_id: Some(offer.id.clone()),
                    attribution: Some(json!({
                        "campaign": "affiliate_referral_eval",
                        "source": "affiliate_referral",
                        "affiliateConnectionId": affiliate.id,
                        "personaId": state.persona.persona_id,
                    })),
                    metadata: Some(json!({
                        "evalCase": "affiliate_referral",
                        "personaHash": state.persona.content_hash,
                    })),
                },
                None,
            )?;
            state.referral_entry_point_id = Some(entry_point.id.clone());
            state.referral_entry_point_slug = Some(entry_point.slug.clone());

            let (visitor_session, _) = create_visitor_session(
                db_path,
                VisitorSessionCreateRequest {
                    entry_point_slug: entry_point.slug.clone(),
                    session_id: None,
                    user_agent: Some("Ordo affiliate referral eval mobile browser".to_string()),
                    attribution: Some(json!({
                        "personaId": state.persona.persona_id,
                        "affiliateConnectionId": affiliate.id,
                        "entryPointId": entry_point.id,
                    })),
                },
            )?;
            state.referred_visitor_session_id = Some(visitor_session.id.clone());

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

            let (grant, _) = create_connection_grant(
                db_path,
                &affiliate.id,
                ConnectionGrantCreateRequest {
                    resource_kind: ResourceKind::Conversation.as_str().to_string(),
                    resource_id: conversation.id.clone(),
                    action: PolicyAction::Read.as_str().to_string(),
                    expires_at: None,
                    grant_reason: Some("Affiliate may inspect only the referred conversation evidence for eval attribution.".to_string()),
                },
                None,
            )?;
            state.affiliate_grant_id = Some(grant.id.clone());
            ensure!(
                list_connection_grants(db_path, &affiliate.id)?
                    .grants
                    .iter()
                    .any(|item| item.id == grant.id
                        && item.resource_id == conversation.id
                        && item.action == PolicyAction::Read.as_str()),
                "affiliate grant must be durable and scoped"
            );

            let allowed = authorize_connection_resource_access(
                connection,
                &affiliate.id,
                PolicyAction::Read,
                ResourceRef::new(ResourceKind::Conversation, &conversation.id),
                Some("affiliate.referral.inspect"),
            );
            state.affiliate_allowed_conversation_read = allowed.outcome == PolicyOutcome::Allowed;
            record_policy_decision(
                connection,
                &allowed,
                PolicyDecisionCorrelation {
                    request_id: Some(format!("affiliate-allowed-{}", state.persona.persona_id)),
                    ..PolicyDecisionCorrelation::default()
                },
            )?;
            let denied = authorize_connection_resource_access(
                connection,
                &affiliate.id,
                PolicyAction::Read,
                ResourceRef::new(ResourceKind::Conversation, "conversation_unrelated_client"),
                Some("affiliate.referral.inspect"),
            );
            state.affiliate_denied_unrelated_conversation_read =
                denied.outcome == PolicyOutcome::Denied;
            record_policy_decision(
                connection,
                &denied,
                PolicyDecisionCorrelation {
                    request_id: Some(format!("affiliate-denied-{}", state.persona.persona_id)),
                    ..PolicyDecisionCorrelation::default()
                },
            )?;

            let visitor = create_conversation_participant(
                connection,
                &ConversationParticipantCreateRequest {
                    conversation_id: conversation.id.clone(),
                    participant_kind: "visitor".to_string(),
                    actor_id: None,
                    connection_id: None,
                    visitor_session_id: Some(visitor_session.id.clone()),
                    display_name: "Referred visitor".to_string(),
                    role: "referred_prospect".to_string(),
                },
            )?;
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
            let referred_message = create_conversation_message(
                connection,
                &ConversationMessageCreateRequest {
                    conversation_id: conversation.id.clone(),
                    segment_id: None,
                    participant_id: visitor.id.clone(),
                    message_kind: "message".to_string(),
                    body_markdown: persona_backed_affiliate_referral_message(&state.persona),
                    visibility: "participants".to_string(),
                    client_message_id: format!(
                        "affiliate-referral-message-{}",
                        state.persona.persona_id
                    ),
                    reply_to_message_id: None,
                    undo_expires_at: None,
                },
            )?;
            state.referred_message_id = Some(referred_message.id.clone());

            let gateway = LlmGateway::new(DeterministicLlmProvider::new("local_fake", "fake-chat"))
                .with_private_terms(vec![
                    "Project Orchid".to_string(),
                    "Project".to_string(),
                    "Orchid".to_string(),
                    "affiliate-referral-secret".to_string(),
                    "alex@example.com".to_string(),
                ]);
            let llm_result = gateway.run_completion(
                db_path,
                connection,
                &ActorContext::local_owner("live_journey_affiliate_referral_eval"),
                LlmGatewayRequest {
                    run_id: format!(
                        "live_journey_affiliate_referral_{}",
                        state.persona.persona_id
                    ),
                    conversation_id: conversation.id.clone(),
                    segment_id: None,
                    assistant_participant_id: assistant.id,
                    client_id: Some(format!("affiliate-referral-{}", state.persona.persona_id)),
                    provider_id: "local_fake".to_string(),
                    model_id: "fake-chat".to_string(),
                    user_message: referred_message.body_markdown.clone(),
                    prompt_slots: vec![
                        PromptSlot::new(
                            "ethical_business_persuasion",
                            "Ethical Business Persuasion",
                            "Use affiliate/referral context only as evidence of how the visitor arrived. Do not invent earnings, fake referral metrics, fake urgency, fake scarcity, reviews, or unsupported social proof.",
                            vec![
                                format!("connection:{}", affiliate.id),
                                format!("entry_point:{}", entry_point.id),
                                format!("message:{}", referred_message.id),
                            ],
                            "Affiliate referral journey evaluates respectful signup guidance with evidence-backed attribution only.",
                            "staff_private",
                        )?,
                        PromptSlot::new(
                            "affiliate_referral_context",
                            "Affiliate Referral Context",
                            "The visitor arrived through a scoped affiliate referral link. Explain the 30-day trial as optional and keep affiliate tracking separate from client-private data.",
                            vec![
                                format!("connection:{}", affiliate.id),
                                format!("visitor_session:{}", visitor_session.id),
                                format!("offer:{}", offer.id),
                            ],
                            "Durable affiliate, visitor-session, and offer evidence for the referral journey.",
                            "participants",
                        )?,
                    ],
                },
            )?;
            let assistant_message = llm_result.final_message.ok_or_else(|| {
                anyhow!("deterministic affiliate-referral LLM path produced no message")
            })?;
            state.assistant_message_id = Some(assistant_message.id.clone());

            let (acceptance, trial, _) = accept_public_offer(
                db_path,
                &offer.slug,
                OfferAcceptanceCreateRequest {
                    visitor_session_id: Some(visitor_session.id.clone()),
                    attribution: Some(json!({
                        "personaId": state.persona.persona_id,
                        "affiliateConnectionId": affiliate.id,
                        "conversationId": conversation.id,
                        "visitorMessageId": referred_message.id,
                        "assistantMessageId": assistant_message.id,
                        "entryPointId": entry_point.id,
                    })),
                    acceptance_context: Some(json!({
                        "decision": "accepted_30_day_trial_from_affiliate_referral",
                        "agencyPreserving": true,
                        "evidenceRefs": [
                            format!("connection:{}", affiliate.id),
                            format!("entry_point:{}", entry_point.id),
                            format!("visitor_session:{}", visitor_session.id),
                            format!("conversation:{}", conversation.id),
                            format!("message:{}", referred_message.id),
                            format!("message:{}", assistant_message.id),
                            format!("offer:{}", offer.id)
                        ],
                        "nonGoals": [
                            "no_fake_referrals",
                            "no_fake_metrics",
                            "no_fake_urgency",
                            "no_fake_scarcity"
                        ]
                    })),
                },
            )?;
            state.acceptance_id = Some(acceptance.id.clone());
            state.trial_id = Some(trial.id.clone());

            let (referral, _) = record_referral(
                connection,
                ReferralRecordInput {
                    status: "captured".to_string(),
                    referrer_connection_id: Some(affiliate.id.clone()),
                    referred_connection_id: None,
                    conversation_id: Some(conversation.id.clone()),
                    entry_point_id: Some(entry_point.id.clone()),
                    visitor_session_id: Some(visitor_session.id.clone()),
                    evidence_refs: vec![
                        format!("connection:{}", affiliate.id),
                        format!("entry_point:{}", entry_point.id),
                        format!("visitor_session:{}", visitor_session.id),
                        format!("conversation:{}", conversation.id),
                        format!("offer_acceptance:{}", acceptance.id),
                        format!("trial:{}", trial.id),
                    ],
                    provenance: json!({
                        "generator": "live_journey.affiliate_referral",
                        "affiliateCredit": "candidate",
                        "financeAutomation": false,
                        "evidenceBackedOnly": true,
                    }),
                },
            )?;
            state.referral_id = Some(referral.id.clone());

            let (referral_outcome, _) = record_outcome(
                connection,
                BusinessOutcomeInput {
                    outcome_kind: "affiliate_referred_trial_started".to_string(),
                    status: "recorded".to_string(),
                    connection_id: Some(affiliate.id.clone()),
                    conversation_id: Some(conversation.id.clone()),
                    segment_id: None,
                    offer_id: Some(offer.id.clone()),
                    ask_id: None,
                    artifact_id: None,
                    entry_point_id: Some(entry_point.id.clone()),
                    visitor_session_id: Some(visitor_session.id.clone()),
                    referral_id: Some(referral.id.clone()),
                    value_micros: None,
                    currency: None,
                    evidence_refs: vec![
                        format!("referral:{}", referral.id),
                        format!("offer_acceptance:{}", acceptance.id),
                        format!("trial:{}", trial.id),
                    ],
                    provenance: json!({
                        "generator": "live_journey.affiliate_referral",
                        "reason": "Concrete referral record existed before referral outcome attribution.",
                    }),
                    occurred_at: None,
                },
            )?;
            state.referral_outcome_id = Some(referral_outcome.id.clone());

            let acceptance_outcome = list_outcomes_by_offer(connection, &offer.id)?
                .into_iter()
                .find(|outcome| outcome.outcome_kind == "offer_acceptance")
                .ok_or_else(|| anyhow!("offer acceptance outcome evidence missing"))?;
            for outcome_id in [&acceptance_outcome.id, &referral_outcome.id] {
                propose_attribution(
                    connection,
                    outcome_id,
                    BusinessOutcomeAttributionInput {
                        attribution_kind: "referral".to_string(),
                        source_id: referral.id.clone(),
                        influence_role: "assisted".to_string(),
                        confidence: 0.9,
                        evidence_refs: vec![
                            format!("referral:{}", referral.id),
                            format!("entry_point:{}", entry_point.id),
                            format!("visitor_session:{}", visitor_session.id),
                        ],
                        provenance: json!({
                            "generator": "live_journey.affiliate_referral",
                            "reason": "Referral id, entry point, and visitor session are all concrete.",
                        }),
                    },
                )?;
                propose_attribution(
                    connection,
                    outcome_id,
                    BusinessOutcomeAttributionInput {
                        attribution_kind: "affiliate_connection".to_string(),
                        source_id: affiliate.id.clone(),
                        influence_role: "assisted".to_string(),
                        confidence: 0.8,
                        evidence_refs: vec![
                            format!("connection:{}", affiliate.id),
                            format!("referral:{}", referral.id),
                        ],
                        provenance: json!({
                            "generator": "live_journey.affiliate_referral",
                            "reason": "Affiliate connection is the referrer attached to the referral record.",
                        }),
                    },
                )?;
            }
        }
        other => anyhow::bail!("unsupported affiliate referral journey step: {other}"),
    }
    Ok(())
}

pub(crate) fn ensure_affiliate_referral_evidence(
    evidence: &AffiliateReferralJourneyEvidence,
) -> Result<()> {
    ensure!(
        !evidence.affiliate_connection_id.is_empty(),
        "affiliate connection evidence missing"
    );
    ensure!(
        !evidence.affiliate_grant_id.is_empty(),
        "affiliate grant evidence missing"
    );
    ensure!(
        !evidence.referral_entry_point_id.is_empty(),
        "referral entry point evidence missing"
    );
    ensure!(
        !evidence.referred_visitor_session_id.is_empty(),
        "referred visitor session evidence missing"
    );
    ensure!(
        !evidence.referral_id.is_empty(),
        "referral record evidence missing"
    );
    ensure!(
        !evidence.referral_outcome_id.is_empty(),
        "referral outcome evidence missing"
    );
    ensure!(
        evidence.attribution_count >= 7,
        "offer, session, entry point, referral, and affiliate attribution evidence required"
    );
    ensure!(
        evidence.affiliate_allowed_conversation_read,
        "affiliate should read only the scoped referred conversation"
    );
    ensure!(
        evidence.affiliate_denied_unrelated_conversation_read,
        "affiliate should be denied unrelated conversation access"
    );
    Ok(())
}

pub(crate) fn ensure_affiliate_referral_manifest_is_safe(
    manifest: &AffiliateReferralJourneyManifest,
    private_terms: &[String],
) -> Result<()> {
    let value = serde_json::to_value(manifest)?;
    ensure!(
        !contains_sensitive_value(&value, private_terms),
        "affiliate referral journey manifest contains raw sensitive value"
    );
    Ok(())
}

pub(crate) fn persona_backed_affiliate_referral_message(persona: &EvalPersona) -> String {
    format!(
        "A trusted community affiliate sent me this link. I am evaluating OrdoStudio as a {} and want the 30-day trial explained plainly. Please track the referral without exposing unrelated client data or inventing earnings, reviews, metrics, scarcity, or urgency. Do not repeat Project Orchid, alex@example.com, or affiliate-referral-secret.",
        persona.person_type
    )
}
