use super::*;
use anyhow::{ensure, Result};
use rusqlite::{Connection, OptionalExtension};
use std::path::{Path, PathBuf};
use crate::public_surfaces::public_product_surface_contract_connection;
use serde_json::{json, Value};

pub fn run_relationship_conversation_message_eval(
    connection: &Connection,
    output_dir: impl Into<PathBuf>,
    source_commit: impl Into<String>,
) -> Result<EvalWorkflowRun> {
    let case = relationship_conversation_message_case()?;
    let packet_path = output_dir
        .into()
        .join("relationship_conversation_message-packet.json");
    let mut harness = DeterministicEvalHarness::new(DeterministicEvalClock::fixed())
        .with_artifact_path(packet_path.to_string_lossy());
    let scorecard = harness.run_case(connection, &case, run_relationship_conversation_step)?;
    let output_dir = packet_path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    let writer = EvalArtifactWriter::new(output_dir, source_commit)
        .with_private_terms(vec!["package".to_string(), "Project Orchid".to_string()]);
    let artifact_paths = writer.write_packet(connection, &case, &scorecard)?;
    Ok(EvalWorkflowRun {
        case,
        scorecard,
        artifact_paths,
    })
}

pub fn run_privacy_gateway_roundtrip_eval(
    db_path: &Path,
    connection: &Connection,
    output_dir: impl Into<PathBuf>,
    source_commit: impl Into<String>,
) -> Result<EvalWorkflowRun> {
    let case = privacy_gateway_roundtrip_case()?;
    let packet_path = output_dir
        .into()
        .join("privacy_gateway_roundtrip-packet.json");
    let mut harness = DeterministicEvalHarness::new(DeterministicEvalClock::fixed())
        .with_artifact_path(packet_path.to_string_lossy());
    let scorecard = harness.run_case(connection, &case, |connection, step| {
        run_privacy_gateway_roundtrip_step(db_path, connection, step)
    })?;
    let output_dir = packet_path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    let writer = EvalArtifactWriter::new(output_dir, source_commit)
        .with_private_terms(vec!["Project Orchid".to_string()]);
    let artifact_paths = writer.write_packet(connection, &case, &scorecard)?;
    Ok(EvalWorkflowRun {
        case,
        scorecard,
        artifact_paths,
    })
}

pub fn run_replay_provider_fixture_eval(
    db_path: &Path,
    connection: &Connection,
    output_dir: impl Into<PathBuf>,
    source_commit: impl Into<String>,
) -> Result<EvalWorkflowRun> {
    let case = replay_provider_fixture_case()?;
    let packet_path = output_dir
        .into()
        .join("replay_provider_fixture-packet.json");
    let mut harness = DeterministicEvalHarness::new(DeterministicEvalClock::fixed())
        .with_artifact_path(packet_path.to_string_lossy());
    let scorecard = harness.run_case(connection, &case, |connection, step| {
        run_replay_provider_fixture_step(db_path, connection, step)
    })?;
    let output_dir = packet_path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    let writer = EvalArtifactWriter::new(output_dir, source_commit)
        .with_private_terms(vec!["Project Orchid".to_string()]);
    let artifact_paths = writer.write_packet(connection, &case, &scorecard)?;
    Ok(EvalWorkflowRun {
        case,
        scorecard,
        artifact_paths,
    })
}

pub fn run_role_lifecycle_anonymous_to_client_eval(
    connection: &Connection,
    output_dir: impl Into<PathBuf>,
    source_commit: impl Into<String>,
) -> Result<EvalWorkflowRun> {
    run_role_lifecycle_eval(
        connection,
        role_lifecycle_anonymous_to_client_case()?,
        output_dir,
        source_commit,
        run_role_lifecycle_anonymous_to_client_step,
    )
}

pub fn run_role_lifecycle_staff_manager_owner_eval(
    connection: &Connection,
    output_dir: impl Into<PathBuf>,
    source_commit: impl Into<String>,
) -> Result<EvalWorkflowRun> {
    run_role_lifecycle_eval(
        connection,
        role_lifecycle_staff_manager_owner_case()?,
        output_dir,
        source_commit,
        run_role_lifecycle_staff_manager_owner_step,
    )
}

pub fn run_role_lifecycle_agent_silence_eval(
    connection: &Connection,
    output_dir: impl Into<PathBuf>,
    source_commit: impl Into<String>,
) -> Result<EvalWorkflowRun> {
    run_role_lifecycle_eval(
        connection,
        role_lifecycle_agent_silence_case()?,
        output_dir,
        source_commit,
        run_role_lifecycle_agent_silence_step,
    )
}

pub fn run_feedback_capture_private_business_intelligence_eval(
    connection: &Connection,
    output_dir: impl Into<PathBuf>,
    source_commit: impl Into<String>,
) -> Result<EvalWorkflowRun> {
    run_role_lifecycle_eval(
        connection,
        feedback_capture_private_business_intelligence_case()?,
        output_dir,
        source_commit,
        run_feedback_capture_private_business_intelligence_step,
    )
}

pub fn run_review_candidate_consent_publication_boundary_eval(
    connection: &Connection,
    output_dir: impl Into<PathBuf>,
    source_commit: impl Into<String>,
) -> Result<EvalWorkflowRun> {
    run_role_lifecycle_eval(
        connection,
        review_candidate_consent_publication_boundary_case()?,
        output_dir,
        source_commit,
        run_review_candidate_consent_publication_boundary_step,
    )
}

pub fn run_home_about_public_narrative_brief_eval(
    connection: &Connection,
    output_dir: impl Into<PathBuf>,
    source_commit: impl Into<String>,
) -> Result<EvalWorkflowRun> {
    run_role_lifecycle_eval(
        connection,
        home_about_public_narrative_brief_case()?,
        output_dir,
        source_commit,
        run_home_about_public_narrative_brief_step,
    )
}

pub fn run_offer_ask_machine_readable_intent_eval(
    connection: &Connection,
    output_dir: impl Into<PathBuf>,
    source_commit: impl Into<String>,
) -> Result<EvalWorkflowRun> {
    run_role_lifecycle_eval(
        connection,
        offer_ask_machine_readable_intent_case()?,
        output_dir,
        source_commit,
        run_offer_ask_machine_readable_intent_step,
    )
}

pub(crate) fn run_role_lifecycle_eval<F>(
    connection: &Connection,
    case: EvalCase,
    output_dir: impl Into<PathBuf>,
    source_commit: impl Into<String>,
    step_runner: F,
) -> Result<EvalWorkflowRun>
where
    F: FnMut(&Connection, &EvalStep) -> Result<()>,
{
    let packet_path = output_dir.into().join(format!("{}-packet.json", case.id));
    let mut harness = DeterministicEvalHarness::new(DeterministicEvalClock::fixed())
        .with_artifact_path(packet_path.to_string_lossy());
    let scorecard = harness.run_case(connection, &case, step_runner)?;
    let output_dir = packet_path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    let writer = EvalArtifactWriter::new(output_dir, source_commit)
        .with_private_terms(vec!["package".to_string(), "Project Orchid".to_string()]);
    let artifact_paths = writer.write_packet(connection, &case, &scorecard)?;
    Ok(EvalWorkflowRun {
        case,
        scorecard,
        artifact_paths,
    })
}

pub(crate) fn relationship_conversation_message_case() -> Result<EvalCase> {
    EvalCase::new(
        "relationship_conversation_message",
        "Relationship conversation message",
        &json!({
            "fixture": "relationship_conversation_message",
            "version": 1,
            "sensitiveFixtureKinds": ["email", "phone", "api_key", "private_term"],
        }),
        vec![
            EvalActorRole::AnonymousVisitor,
            EvalActorRole::Staff,
            EvalActorRole::LlmToolProviderBoundary,
        ],
        vec![
            EvalStep::new(
                "create_canonical_conversation",
                EvalActorRole::AnonymousVisitor,
                "create_or_find_conversation",
                vec![
                    EvalEvidenceChannel::SqliteRows,
                    EvalEvidenceChannel::ConversationEvents,
                ],
            )?,
            EvalStep::new(
                "submit_message",
                EvalActorRole::AnonymousVisitor,
                "message.submit",
                vec![
                    EvalEvidenceChannel::SqliteRows,
                    EvalEvidenceChannel::ConversationEvents,
                    EvalEvidenceChannel::RealtimeReplay,
                ],
            )?,
        ],
        vec![
            EvalAssertion::minimum_count(
                "conversation_events_exist",
                EvalEvidenceChannel::ConversationEvents,
                2,
            )?,
            EvalAssertion::minimum_count(
                "realtime_replay_exists",
                EvalEvidenceChannel::RealtimeReplay,
                2,
            )?,
        ],
    )
}

pub(crate) fn privacy_gateway_roundtrip_case() -> Result<EvalCase> {
    EvalCase::new(
        "privacy_gateway_roundtrip",
        "Privacy gateway roundtrip",
        &json!({
            "fixture": "privacy_gateway_roundtrip",
            "version": 1,
            "providerMode": "deterministic_only",
        }),
        vec![
            EvalActorRole::Staff,
            EvalActorRole::OrdoAgent,
            EvalActorRole::LlmToolProviderBoundary,
        ],
        vec![EvalStep::new(
            "run_deterministic_llm_completion",
            EvalActorRole::LlmToolProviderBoundary,
            "llm.run.request",
            vec![
                EvalEvidenceChannel::ConversationEvents,
                EvalEvidenceChannel::PolicyDecisions,
                EvalEvidenceChannel::PromptSlotAccounting,
                EvalEvidenceChannel::PrivacyTransforms,
                EvalEvidenceChannel::TokenLedger,
                EvalEvidenceChannel::RealtimeReplay,
            ],
        )?],
        vec![
            EvalAssertion::minimum_count(
                "policy_decision_recorded",
                EvalEvidenceChannel::PolicyDecisions,
                1,
            )?,
            EvalAssertion::minimum_count(
                "prompt_slots_accounted",
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
            EvalAssertion::minimum_count(
                "conversation_events_recorded",
                EvalEvidenceChannel::ConversationEvents,
                7,
            )?,
        ],
    )
}

pub(crate) fn replay_provider_fixture_case() -> Result<EvalCase> {
    EvalCase::new(
        "replay_provider_fixture",
        "Replay provider fixture roundtrip",
        &json!({
            "fixture": "tiny-success",
            "version": 1,
            "providerMode": "replay_fixture",
            "fixtureSchema": crate::llm_gateway::LLM_REPLAY_FIXTURE_SCHEMA_VERSION,
        }),
        vec![
            EvalActorRole::Staff,
            EvalActorRole::OrdoAgent,
            EvalActorRole::LlmToolProviderBoundary,
        ],
        vec![EvalStep::new(
            "run_replay_llm_completion",
            EvalActorRole::LlmToolProviderBoundary,
            "llm.run.request.replay_fixture",
            vec![
                EvalEvidenceChannel::ConversationEvents,
                EvalEvidenceChannel::PolicyDecisions,
                EvalEvidenceChannel::PromptSlotAccounting,
                EvalEvidenceChannel::PrivacyTransforms,
                EvalEvidenceChannel::TokenLedger,
                EvalEvidenceChannel::RealtimeReplay,
            ],
        )?],
        vec![
            EvalAssertion::minimum_count(
                "policy_decision_recorded",
                EvalEvidenceChannel::PolicyDecisions,
                1,
            )?,
            EvalAssertion::minimum_count(
                "prompt_slots_accounted",
                EvalEvidenceChannel::PromptSlotAccounting,
                1,
            )?,
            EvalAssertion::minimum_count(
                "token_ledger_recorded",
                EvalEvidenceChannel::TokenLedger,
                2,
            )?,
            EvalAssertion::minimum_count(
                "conversation_events_recorded",
                EvalEvidenceChannel::ConversationEvents,
                7,
            )?,
        ],
    )
}

pub(crate) fn role_lifecycle_anonymous_to_client_case() -> Result<EvalCase> {
    EvalCase::new(
        "role_lifecycle_anonymous_to_client",
        "Role lifecycle anonymous visitor, client member, and affiliate boundaries",
        &json!({
            "fixture": "role_lifecycle_anonymous_to_client",
            "version": 1,
            "roles": ["anonymous_visitor", "client_member", "affiliate"],
            "providerMode": "deterministic_only",
        }),
        vec![
            EvalActorRole::AnonymousVisitor,
            EvalActorRole::ClientMember,
            EvalActorRole::Affiliate,
        ],
        vec![
            eval_step_with_metadata(
                "anonymous_visitor_relationship_message",
                EvalActorRole::AnonymousVisitor,
                "conversation.message.submit",
                vec![
                    EvalEvidenceChannel::SqliteRows,
                    EvalEvidenceChannel::ConversationEvents,
                    EvalEvidenceChannel::RealtimeReplay,
                ],
                json!({
                    "surface": "chat",
                    "subjectKind": "visitor_session",
                    "visibilityExpectation": "relationship_conversation_without_staff_admin_internals",
                }),
            )?,
            eval_step_with_metadata(
                "authenticated_client_relationship_continuity",
                EvalActorRole::ClientMember,
                "conversation.relationship.attach",
                vec![
                    EvalEvidenceChannel::SqliteRows,
                    EvalEvidenceChannel::ConversationEvents,
                    EvalEvidenceChannel::RealtimeReplay,
                    EvalEvidenceChannel::PolicyDecisions,
                ],
                json!({
                    "surface": "client_portal",
                    "subjectKind": "connection",
                    "visibilityExpectation": "one_client_visible_relationship_conversation",
                }),
            )?,
            eval_step_with_metadata(
                "affiliate_unrelated_customer_denied",
                EvalActorRole::Affiliate,
                "conversation.boundary.denied",
                vec![EvalEvidenceChannel::PolicyDecisions],
                json!({
                    "visibilityExpectation": "affiliate_cannot_access_unrelated_customer_conversation",
                }),
            )?,
        ],
        vec![
            EvalAssertion::minimum_count(
                "conversation_events_recorded",
                EvalEvidenceChannel::ConversationEvents,
                4,
            )?,
            EvalAssertion::minimum_count(
                "realtime_replay_recorded",
                EvalEvidenceChannel::RealtimeReplay,
                4,
            )?,
            EvalAssertion::minimum_count(
                "role_boundary_policy_decisions_recorded",
                EvalEvidenceChannel::PolicyDecisions,
                2,
            )?,
        ],
    )
}

pub(crate) fn role_lifecycle_staff_manager_owner_case() -> Result<EvalCase> {
    EvalCase::new(
        "role_lifecycle_staff_manager_owner_boundaries",
        "Role lifecycle staff, manager, owner, and system internals boundaries",
        &json!({
            "fixture": "role_lifecycle_staff_manager_owner_boundaries",
            "version": 1,
            "roles": ["staff", "manager_admin", "owner_system_admin"],
        }),
        vec![
            EvalActorRole::Staff,
            EvalActorRole::ManagerAdmin,
            EvalActorRole::OwnerSystemAdmin,
        ],
        vec![
            eval_step_with_metadata(
                "seed_staff_handoff",
                EvalActorRole::Staff,
                "handoff.create",
                vec![
                    EvalEvidenceChannel::SqliteRows,
                    EvalEvidenceChannel::ConversationEvents,
                    EvalEvidenceChannel::RealtimeReplay,
                    EvalEvidenceChannel::HandoffState,
                ],
                json!({
                    "defaultQueue": "my_handoffs",
                    "assignedActorId": "actor_staff_eval_1",
                }),
            )?,
            eval_step_with_metadata(
                "assert_queue_role_boundaries",
                EvalActorRole::ManagerAdmin,
                "conversation.queue.read",
                vec![
                    EvalEvidenceChannel::HandoffState,
                    EvalEvidenceChannel::PolicyDecisions,
                ],
                json!({
                    "staffDefault": "my_handoffs",
                    "managerAllowed": "team_queue",
                    "ownerAllowed": "all_conversations",
                    "staffDenied": "all_conversations",
                }),
            )?,
            eval_step_with_metadata(
                "assert_owner_system_boundary",
                EvalActorRole::OwnerSystemAdmin,
                "daemon.system.route.boundary",
                vec![EvalEvidenceChannel::PolicyDecisions],
                json!({
                    "ordinaryBrowser": "denied_without_loopback_or_token",
                    "ownerSystem": "allowed_with_loopback",
                }),
            )?,
        ],
        vec![
            EvalAssertion::minimum_count(
                "handoff_state_recorded",
                EvalEvidenceChannel::HandoffState,
                1,
            )?,
            EvalAssertion::minimum_count(
                "queue_policy_decisions_recorded",
                EvalEvidenceChannel::PolicyDecisions,
                2,
            )?,
            EvalAssertion::minimum_count(
                "conversation_events_recorded",
                EvalEvidenceChannel::ConversationEvents,
                2,
            )?,
        ],
    )
}

pub(crate) fn role_lifecycle_agent_silence_case() -> Result<EvalCase> {
    EvalCase::new(
        "role_lifecycle_agent_silence_boundary",
        "Role lifecycle Ordo agent silence during human-led active mode",
        &json!({
            "fixture": "role_lifecycle_agent_silence_boundary",
            "version": 1,
            "mode": "human_led_active",
        }),
        vec![
            EvalActorRole::Staff,
            EvalActorRole::OrdoAgent,
            EvalActorRole::ClientMember,
        ],
        vec![
            eval_step_with_metadata(
                "staff_reply_sets_human_led_active",
                EvalActorRole::Staff,
                "conversation.mode.human_led_active",
                vec![
                    EvalEvidenceChannel::SqliteRows,
                    EvalEvidenceChannel::ConversationEvents,
                    EvalEvidenceChannel::RealtimeReplay,
                ],
                json!({
                    "mode": "human_led_active",
                    "ledByActorId": "actor_staff_eval_1",
                }),
            )?,
            eval_step_with_metadata(
                "agent_public_post_blocked_without_delegation",
                EvalActorRole::OrdoAgent,
                "agent.public_post.denied",
                vec![EvalEvidenceChannel::PolicyDecisions],
                json!({
                    "expectedDecision": "human_led_active_requires_tag_delegation_or_policy",
                    "clientVisibleMechanics": "hidden",
                }),
            )?,
        ],
        vec![
            EvalAssertion::minimum_count(
                "mode_events_recorded",
                EvalEvidenceChannel::ConversationEvents,
                2,
            )?,
            EvalAssertion::minimum_count(
                "agent_silence_policy_evidence_recorded",
                EvalEvidenceChannel::PolicyDecisions,
                1,
            )?,
        ],
    )
}

pub(crate) fn feedback_capture_private_business_intelligence_case() -> Result<EvalCase> {
    EvalCase::new(
        "feedback_capture_private_business_intelligence",
        "Customer feedback capture as private business intelligence",
        &json!({
            "fixture": "feedback_capture_private_business_intelligence",
            "version": 1,
            "feedbackContract": "private_business_intelligence",
        }),
        vec![
            EvalActorRole::ClientMember,
            EvalActorRole::Staff,
            EvalActorRole::ManagerAdmin,
        ],
        vec![
            eval_step_with_metadata(
                "seed_feedback_source_message",
                EvalActorRole::ClientMember,
                "conversation.message.submit",
                vec![
                    EvalEvidenceChannel::ConversationEvents,
                    EvalEvidenceChannel::RealtimeReplay,
                ],
                json!({
                    "source": "durable_message",
                    "containsPrivateContactFixture": true,
                }),
            )?,
            eval_step_with_metadata(
                "capture_private_feedback",
                EvalActorRole::Staff,
                "feedback.capture",
                vec![
                    EvalEvidenceChannel::FeedbackReviewRecords,
                    EvalEvidenceChannel::RealtimeReplay,
                ],
                json!({
                    "visibility": "private_business_intelligence",
                    "notReview": true,
                    "notTestimonial": true,
                }),
            )?,
            eval_step_with_metadata(
                "star_and_tag_feedback_candidate",
                EvalActorRole::Staff,
                "feedback.star_and_tag",
                vec![
                    EvalEvidenceChannel::FeedbackReviewRecords,
                    EvalEvidenceChannel::RealtimeReplay,
                ],
                json!({
                    "starMeans": "staff_signal_not_customer_rating",
                    "tagCandidateState": "proposed",
                }),
            )?,
        ],
        vec![
            EvalAssertion::minimum_count(
                "feedback_records_created",
                EvalEvidenceChannel::FeedbackReviewRecords,
                2,
            )?,
            EvalAssertion::minimum_count(
                "feedback_events_recorded",
                EvalEvidenceChannel::RealtimeReplay,
                4,
            )?,
            EvalAssertion::minimum_count(
                "conversation_source_evidence_recorded",
                EvalEvidenceChannel::ConversationEvents,
                2,
            )?,
        ],
    )
}

pub(crate) fn review_candidate_consent_publication_boundary_case() -> Result<EvalCase> {
    EvalCase::new(
        "review_candidate_consent_publication_boundary",
        "Review candidate consent and publication boundary",
        &json!({
            "fixture": "review_candidate_consent_publication_boundary",
            "version": 1,
            "reviewLifecycle": [
                "candidate",
                "requested",
                "received",
                "consent_confirmed",
                "approved",
                "published",
                "featured",
                "retired"
            ],
        }),
        vec![
            EvalActorRole::ClientMember,
            EvalActorRole::Staff,
            EvalActorRole::ManagerAdmin,
        ],
        vec![
            eval_step_with_metadata(
                "create_review_candidate",
                EvalActorRole::Staff,
                "review.candidate.create",
                vec![
                    EvalEvidenceChannel::FeedbackReviewRecords,
                    EvalEvidenceChannel::RealtimeReplay,
                ],
                json!({
                    "source": "private_feedback",
                    "initialVisibility": "private_until_approved",
                }),
            )?,
            eval_step_with_metadata(
                "assert_publish_blocked_before_consent_approval",
                EvalActorRole::Staff,
                "review.publish.denied",
                vec![EvalEvidenceChannel::FeedbackReviewRecords],
                json!({
                    "requiredBeforePublish": ["consent_evidence", "approval_evidence"],
                }),
            )?,
            eval_step_with_metadata(
                "complete_review_consent_approval_publication_lifecycle",
                EvalActorRole::ManagerAdmin,
                "review.lifecycle.transition",
                vec![
                    EvalEvidenceChannel::FeedbackReviewRecords,
                    EvalEvidenceChannel::RealtimeReplay,
                ],
                json!({
                    "terminalState": "retired",
                    "publishedOnlyAfter": ["consent_confirmed", "approved"],
                }),
            )?,
        ],
        vec![
            EvalAssertion::minimum_count(
                "review_records_created",
                EvalEvidenceChannel::FeedbackReviewRecords,
                2,
            )?,
            EvalAssertion::minimum_count(
                "review_events_recorded",
                EvalEvidenceChannel::RealtimeReplay,
                8,
            )?,
        ],
    )
}

pub(crate) fn home_about_public_narrative_brief_case() -> Result<EvalCase> {
    EvalCase::new(
        "home_about_public_narrative_brief",
        "Home/About public narrative brief",
        &json!({
            "fixture": "home_about_public_narrative_brief",
            "version": 1,
            "surface": "home_about",
            "reducedMotionFallbackRequired": true,
        }),
        vec![
            EvalActorRole::AnonymousVisitor,
            EvalActorRole::ClientMember,
            EvalActorRole::Staff,
        ],
        vec![
            eval_step_with_metadata(
                "seed_home_about_surface_evidence",
                EvalActorRole::Staff,
                "home_about.evidence.seed",
                vec![
                    EvalEvidenceChannel::ProductSurfaceRecords,
                    EvalEvidenceChannel::SurfaceBriefRecords,
                    EvalEvidenceChannel::ArtifactRecords,
                    EvalEvidenceChannel::FeedbackReviewRecords,
                ],
                json!({
                    "surface": "home_about",
                    "publicClaims": "evidence_backed_only",
                    "includesPrivateDraftFixture": true,
                }),
            )?,
            eval_step_with_metadata(
                "assert_home_about_public_contract",
                EvalActorRole::AnonymousVisitor,
                "home_about.public_contract.read",
                vec![EvalEvidenceChannel::ProductSurfaceRecords],
                json!({
                    "billboardStatesAllowed": ["pinned", "dynamic", "published"],
                    "reducedMotionFallback": "required",
                    "links": ["/offers/starter-sprint", "/asks/referrals", "/latest", "/chat"],
                }),
            )?,
        ],
        vec![
            EvalAssertion::minimum_count(
                "product_surface_records_created",
                EvalEvidenceChannel::ProductSurfaceRecords,
                6,
            )?,
            EvalAssertion::minimum_count(
                "surface_brief_records_created",
                EvalEvidenceChannel::SurfaceBriefRecords,
                1,
            )?,
            EvalAssertion::minimum_count(
                "artifact_records_created",
                EvalEvidenceChannel::ArtifactRecords,
                1,
            )?,
        ],
    )
}

pub(crate) fn offer_ask_machine_readable_intent_case() -> Result<EvalCase> {
    EvalCase::new(
        "offer_ask_machine_readable_intent",
        "Offer and Ask machine-readable business intent",
        &json!({
            "fixture": "offer_ask_machine_readable_intent",
            "version": 1,
            "a2aStatus": "future_contract",
            "decisionBoundary": "human_or_policy_decides",
        }),
        vec![
            EvalActorRole::AnonymousVisitor,
            EvalActorRole::ClientMember,
            EvalActorRole::Staff,
        ],
        vec![
            eval_step_with_metadata(
                "seed_offer_ask_intent_evidence",
                EvalActorRole::Staff,
                "offer_ask.intent.seed",
                vec![EvalEvidenceChannel::ProductSurfaceRecords],
                json!({
                    "offerIntent": "starter_sprint",
                    "askIntent": "referrals",
                    "fakeProofPolicy": "rejected_or_absent",
                }),
            )?,
            eval_step_with_metadata(
                "assert_offer_ask_intent_contract",
                EvalActorRole::AnonymousVisitor,
                "offer_ask.intent.read",
                vec![EvalEvidenceChannel::ProductSurfaceRecords],
                json!({
                    "requiresHumanReadable": true,
                    "requiresMachineReadable": true,
                    "futureA2AOnly": true,
                }),
            )?,
            eval_step_with_metadata(
                "assert_fake_persuasion_rejected",
                EvalActorRole::Staff,
                "offer_ask.intent.guardrail",
                vec![EvalEvidenceChannel::ProductSurfaceRecords],
                json!({
                    "rejects": ["fake_scarcity", "unsupported_social_proof", "fake_metrics"],
                }),
            )?,
        ],
        vec![EvalAssertion::minimum_count(
            "offer_ask_intent_records_created",
            EvalEvidenceChannel::ProductSurfaceRecords,
            8,
        )?],
    )
}

pub(crate) fn eval_step_with_metadata(
    id: impl Into<String>,
    actor_role: EvalActorRole,
    action: impl Into<String>,
    expected_evidence: Vec<EvalEvidenceChannel>,
    metadata: Value,
) -> Result<EvalStep> {
    let mut step = EvalStep::new(id, actor_role, action, expected_evidence)?;
    step.metadata = metadata;
    Ok(step)
}

pub(crate) fn run_relationship_conversation_step(connection: &Connection, step: &EvalStep) -> Result<()> {
    match step.id.as_str() {
        "create_canonical_conversation" => {
            find_or_create_canonical_conversation(connection, &visitor_conversation_request())?;
        }
        "submit_message" => {
            let conversation =
                find_or_create_canonical_conversation(connection, &visitor_conversation_request())?;
            let participant = create_visitor_participant(connection, &conversation.id)?;
            create_conversation_message(
                connection,
                &ConversationMessageCreateRequest {
                    conversation_id: conversation.id,
                    segment_id: None,
                    participant_id: participant.id,
                    message_kind: "user".to_string(),
                    body_markdown:
                        "I need help choosing a package. Email me at alex@example.com or 555-123-4567. sk-eval-secret"
                            .to_string(),
                    visibility: "participants".to_string(),
                    client_message_id: "eval-client-message-1".to_string(),
                    reply_to_message_id: None,
                    undo_expires_at: None,
                },
            )?;
        }
        other => anyhow::bail!("unsupported relationship workflow eval step: {other}"),
    }
    Ok(())
}

pub(crate) fn run_privacy_gateway_roundtrip_step(
    db_path: &Path,
    connection: &Connection,
    step: &EvalStep,
) -> Result<()> {
    match step.id.as_str() {
        "run_deterministic_llm_completion" => {
            let (conversation_id, assistant_id) = conversation_and_assistant(connection)?;
            let gateway = LlmGateway::new(DeterministicLlmProvider::new("local_fake", "fake-chat"))
                .with_private_terms(vec!["Project Orchid".to_string()]);
            gateway.run_completion(
                db_path,
                connection,
                &ActorContext::local_owner("eval_privacy_gateway_roundtrip"),
                LlmGatewayRequest {
                    run_id: "eval_llm_run_privacy_roundtrip".to_string(),
                    conversation_id,
                    segment_id: None,
                    assistant_participant_id: assistant_id,
                    client_id: Some("eval-client-llm-1".to_string()),
                    provider_id: "local_fake".to_string(),
                    model_id: "fake-chat".to_string(),
                    user_message: "Draft a reply for Project Orchid. Contact alex@example.com, 555-123-4567, sk-eval-secret.".to_string(),
                    prompt_slots: vec![
                        PromptSlot::new(
                            "ethical_business_persuasion",
                            "Ethical Business Persuasion",
                            "Use verified evidence only; preserve client agency.",
                            vec![
                                "docs/architecture/conversation-realtime/product-doctrine.md"
                                    .to_string(),
                            ],
                            "Business communication lens required by product doctrine.",
                            "staff_private",
                        )?,
                        PromptSlot::new(
                            "conversation_brief",
                            "Conversation Brief",
                            "Project Orchid needs a grounded next step.",
                            vec!["conversation_event_eval_1".to_string()],
                            "Current conversation evidence.",
                            "participants",
                        )?,
                    ],
                },
            )?;
        }
        other => anyhow::bail!("unsupported privacy workflow eval step: {other}"),
    }
    Ok(())
}

pub(crate) fn run_replay_provider_fixture_step(
    db_path: &Path,
    connection: &Connection,
    step: &EvalStep,
) -> Result<()> {
    match step.id.as_str() {
        "run_replay_llm_completion" => {
            let (conversation_id, assistant_id) = conversation_and_assistant(connection)?;
            let fixture_path =
                Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/llm-replay/tiny-success.json");
            let gateway = LlmGateway::new(ReplayLlmProvider::from_fixture_file(&fixture_path)?);
            gateway.run_completion(
                db_path,
                connection,
                &ActorContext::local_owner("eval_replay_provider_fixture"),
                LlmGatewayRequest {
                    run_id: "eval_llm_run_replay_fixture".to_string(),
                    conversation_id,
                    segment_id: None,
                    assistant_participant_id: assistant_id,
                    client_id: Some("eval-client-replay-1".to_string()),
                    provider_id: "replay_fixture".to_string(),
                    model_id: "replay-chat".to_string(),
                    user_message: "Please draft the next step.".to_string(),
                    prompt_slots: vec![PromptSlot::new(
                        "conversation_brief",
                        "Conversation Brief",
                        "Client needs a concise next step.",
                        vec!["conversation_event_replay_1".to_string()],
                        "Replay fixture request evidence.",
                        "participants",
                    )?],
                },
            )?;
        }
        other => anyhow::bail!("unsupported replay provider fixture eval step: {other}"),
    }
    Ok(())
}

pub(crate) fn run_role_lifecycle_anonymous_to_client_step(
    connection: &Connection,
    step: &EvalStep,
) -> Result<()> {
    match step.id.as_str() {
        "anonymous_visitor_relationship_message" => {
            run_relationship_conversation_step(
                connection,
                &EvalStep::new(
                    "submit_message",
                    EvalActorRole::AnonymousVisitor,
                    "message.submit",
                    vec![],
                )?,
            )?;
        }
        "authenticated_client_relationship_continuity" => {
            let conversation = find_or_create_canonical_conversation(
                connection,
                &CanonicalConversationRequest {
                    surface: "client_portal".to_string(),
                    subject_kind: "connection".to_string(),
                    subject_id: "connection_eval_1".to_string(),
                    connection_id: Some("connection_eval_1".to_string()),
                    visitor_session_id: None,
                    created_by_actor_id: Some("actor_client_eval_1".to_string()),
                },
            )?;
            let repeated = find_or_create_canonical_conversation(
                connection,
                &CanonicalConversationRequest {
                    surface: "client_portal".to_string(),
                    subject_kind: "connection".to_string(),
                    subject_id: "connection_eval_1".to_string(),
                    connection_id: Some("connection_eval_1".to_string()),
                    visitor_session_id: None,
                    created_by_actor_id: Some("actor_client_eval_1".to_string()),
                },
            )?;
            ensure!(
                conversation.id == repeated.id,
                "client/member lifecycle must preserve one relationship conversation"
            );
            let participant = create_conversation_participant(
                connection,
                &ConversationParticipantCreateRequest {
                    conversation_id: conversation.id.clone(),
                    participant_kind: "connection".to_string(),
                    actor_id: Some("actor_client_eval_1".to_string()),
                    connection_id: Some("connection_eval_1".to_string()),
                    visitor_session_id: None,
                    display_name: "Eval Client".to_string(),
                    role: "client".to_string(),
                },
            )?;
            create_conversation_message(
                connection,
                &ConversationMessageCreateRequest {
                    conversation_id: conversation.id.clone(),
                    segment_id: None,
                    participant_id: participant.id,
                    message_kind: "user".to_string(),
                    body_markdown:
                        "Authenticated client asks for the next step without staff internals."
                            .to_string(),
                    visibility: "participants".to_string(),
                    client_message_id: "eval-client-member-message-1".to_string(),
                    reply_to_message_id: None,
                    undo_expires_at: None,
                },
            )?;
            let denied = authorize_protected_daemon_action(
                ActorContext::new(
                    ActorKind::BrowserOperator,
                    "client_member_eval",
                    Some("actor_client_eval_1".to_string()),
                ),
                PolicyAction::Read,
                ResourceRef::new(ResourceKind::DaemonRoute, "/admin/policy-decisions"),
                Some("policy.decisions.query"),
                ProtectedAccessEvidence {
                    loopback: false,
                    token: false,
                },
            );
            ensure!(
                !denied.allowed(),
                "client/member should not satisfy protected admin route access"
            );
            record_policy_decision(
                connection,
                &denied,
                PolicyDecisionCorrelation {
                    request_id: Some("eval_client_admin_boundary".to_string()),
                    ..Default::default()
                },
            )?;
        }
        "affiliate_unrelated_customer_denied" => {
            let denied = authorize_resource_access(
                connection,
                ActorContext::new(
                    ActorKind::BrowserOperator,
                    "affiliate_eval",
                    Some("actor_affiliate_eval_1".to_string()),
                ),
                PolicyAction::Read,
                ResourceRef::new(
                    ResourceKind::Conversation,
                    "conversation_unrelated_customer",
                ),
                Some("conversation.read"),
            );
            ensure!(
                !denied.allowed(),
                "affiliate should not access unrelated customer conversation"
            );
            record_policy_decision(
                connection,
                &denied,
                PolicyDecisionCorrelation {
                    request_id: Some("eval_affiliate_boundary".to_string()),
                    ..Default::default()
                },
            )?;
        }
        other => anyhow::bail!("unsupported anonymous/client role lifecycle eval step: {other}"),
    }
    Ok(())
}

pub(crate) fn run_role_lifecycle_staff_manager_owner_step(
    connection: &Connection,
    step: &EvalStep,
) -> Result<()> {
    match step.id.as_str() {
        "seed_staff_handoff" => {
            let conversation = find_or_create_canonical_conversation(
                connection,
                &CanonicalConversationRequest {
                    surface: "client_portal".to_string(),
                    subject_kind: "connection".to_string(),
                    subject_id: "connection_eval_1".to_string(),
                    connection_id: Some("connection_eval_1".to_string()),
                    visitor_session_id: None,
                    created_by_actor_id: Some("actor_staff_eval_1".to_string()),
                },
            )?;
            create_conversation_handoff(
                connection,
                &ConversationHandoffCreateRequest {
                    conversation_id: conversation.id,
                    segment_id: None,
                    connection_id: Some("connection_eval_1".to_string()),
                    requested_by_actor_id: Some("actor_client_eval_1".to_string()),
                    assigned_to_actor_id: Some("actor_staff_eval_1".to_string()),
                    reason: "Client needs staff follow-up".to_string(),
                    urgency: "high".to_string(),
                    required_capability_id: "conversation.handoff.manage".to_string(),
                    evidence_summary: "Client asked for a human review of the next step."
                        .to_string(),
                    allowed_context: vec![
                        "conversation_summary".to_string(),
                        "client_request".to_string(),
                    ],
                    policy_decision_id: None,
                },
            )?;
        }
        "assert_queue_role_boundaries" => {
            let staff_rows = conversation_queue(
                connection,
                ConversationRole::Staff,
                Some("actor_staff_eval_1"),
                None,
            )?;
            let manager_rows = conversation_queue(
                connection,
                ConversationRole::Manager,
                None,
                Some(QueueScope::TeamQueue),
            )?;
            let owner_rows = conversation_queue(
                connection,
                ConversationRole::Owner,
                None,
                Some(QueueScope::AllConversations),
            )?;
            let staff_all = conversation_queue(
                connection,
                ConversationRole::Staff,
                Some("actor_staff_eval_1"),
                Some(QueueScope::AllConversations),
            );
            ensure!(staff_rows.len() == 1, "staff should default to My Handoffs");
            ensure!(
                manager_rows.len() == 1,
                "manager/admin should access Team Queue"
            );
            ensure!(
                owner_rows.len() == 1,
                "owner/system admin should access All Conversations"
            );
            ensure!(
                staff_all.is_err(),
                "ordinary staff should not access All Conversations"
            );
        }
        "assert_owner_system_boundary" => {
            let denied = authorize_protected_daemon_action(
                ActorContext::browser_operator(),
                PolicyAction::Read,
                ResourceRef::new(ResourceKind::DaemonRoute, "/diagnostic-logs"),
                Some("diagnostic_logs.read"),
                ProtectedAccessEvidence {
                    loopback: false,
                    token: false,
                },
            );
            let allowed = authorize_protected_daemon_action(
                ActorContext::local_owner("eval_role_lifecycle"),
                PolicyAction::Read,
                ResourceRef::new(ResourceKind::DaemonRoute, "/diagnostic-logs"),
                Some("diagnostic_logs.read"),
                ProtectedAccessEvidence {
                    loopback: true,
                    token: false,
                },
            );
            ensure!(!denied.allowed(), "non-owner browser should be denied");
            ensure!(allowed.allowed(), "owner/system loopback should be allowed");
            record_policy_decision(
                connection,
                &denied,
                PolicyDecisionCorrelation {
                    request_id: Some("eval_staff_system_boundary_denied".to_string()),
                    ..Default::default()
                },
            )?;
            record_policy_decision(
                connection,
                &allowed,
                PolicyDecisionCorrelation {
                    request_id: Some("eval_owner_system_boundary_allowed".to_string()),
                    ..Default::default()
                },
            )?;
        }
        other => anyhow::bail!("unsupported staff/manager/owner role lifecycle eval step: {other}"),
    }
    Ok(())
}

pub(crate) fn run_role_lifecycle_agent_silence_step(connection: &Connection, step: &EvalStep) -> Result<()> {
    match step.id.as_str() {
        "staff_reply_sets_human_led_active" => {
            let conversation = find_or_create_canonical_conversation(
                connection,
                &CanonicalConversationRequest {
                    surface: "client_portal".to_string(),
                    subject_kind: "connection".to_string(),
                    subject_id: "connection_eval_1".to_string(),
                    connection_id: Some("connection_eval_1".to_string()),
                    visitor_session_id: None,
                    created_by_actor_id: Some("actor_staff_eval_1".to_string()),
                },
            )?;
            record_staff_activity_sets_human_led(
                connection,
                &conversation.id,
                "actor_staff_eval_1",
            )?;
        }
        "agent_public_post_blocked_without_delegation" => {
            let decision = may_agent_post_publicly(
                ConversationMode::HumanLedActive,
                &PublicPostContext::default(),
            );
            ensure!(
                !decision.allowed,
                "agent should stay silent publicly during human-led active mode"
            );
            ensure!(
                decision.reason == "human_led_active_requires_tag_delegation_or_policy",
                "agent silence decision should cite the human-led boundary"
            );
            let policy_decision = authorize_resource_access(
                connection,
                ActorContext::new(
                    ActorKind::System,
                    "ordo_agent_eval",
                    Some("actor_system".to_string()),
                ),
                PolicyAction::Create,
                ResourceRef::new(ResourceKind::ConversationMessage, "public_agent_message"),
                Some("conversation.message.create"),
            );
            record_policy_decision(
                connection,
                &policy_decision,
                PolicyDecisionCorrelation {
                    request_id: Some("eval_agent_public_post_blocked".to_string()),
                    ..Default::default()
                },
            )?;
        }
        other => anyhow::bail!("unsupported agent silence role lifecycle eval step: {other}"),
    }
    Ok(())
}

pub(crate) fn run_feedback_capture_private_business_intelligence_step(
    connection: &Connection,
    step: &EvalStep,
) -> Result<()> {
    match step.id.as_str() {
        "seed_feedback_source_message" => {
            seed_feedback_source_message(connection)?;
        }
        "capture_private_feedback" => {
            let (conversation_id, message_id) = seed_feedback_source_message(connection)?;
            let (feedback, _) = capture_feedback(
                connection,
                CustomerFeedbackInput {
                    connection_id: Some("connection_eval_1".to_string()),
                    conversation_id,
                    segment_id: None,
                    message_id: Some(message_id.clone()),
                    feedback_kind: "praise".to_string(),
                    body_summary: "Client says the onboarding was clear and useful.".to_string(),
                    source_refs: vec![message_id.clone()],
                    evidence_refs: vec![message_id],
                    provenance: json!({
                        "workflow": "feedback_capture_private_business_intelligence",
                        "source": "conversation_message",
                    }),
                },
            )?;
            ensure!(
                feedback.visibility == "private_business_intelligence",
                "feedback must remain private business intelligence"
            );
            ensure!(
                list_public_reviews(connection)?.is_empty(),
                "private feedback must not create a public review"
            );
        }
        "star_and_tag_feedback_candidate" => {
            let feedback_id = latest_feedback_id(connection)?;
            let (starred, _) = set_feedback_starred(
                connection,
                &feedback_id,
                true,
                vec!["staff_review_1".to_string()],
            )?;
            ensure!(
                starred.is_starred,
                "starred feedback should persist as staff signal"
            );
            let (tag, _) = propose_feedback_tag(
                connection,
                &feedback_id,
                FeedbackTagInput {
                    tag: "clear_onboarding".to_string(),
                    confidence: 0.82,
                    evidence_refs: starred.evidence_refs.clone(),
                    provenance: json!({
                        "workflow": "feedback_capture_private_business_intelligence",
                        "candidate": true,
                    }),
                },
            )?;
            ensure!(
                tag.candidate_state == "proposed",
                "feedback tags should default to proposed candidates"
            );
            let private_rows = list_private_feedback(connection, &starred.conversation_id)?;
            ensure!(
                private_rows.len() == 1,
                "feedback should be visible in private staff intelligence list"
            );
        }
        other => anyhow::bail!("unsupported feedback capture workflow eval step: {other}"),
    }
    Ok(())
}

pub(crate) fn run_review_candidate_consent_publication_boundary_step(
    connection: &Connection,
    step: &EvalStep,
) -> Result<()> {
    match step.id.as_str() {
        "create_review_candidate" => {
            let feedback_id = ensure_review_feedback(connection)?;
            let (review, _) = create_review_candidate(
                connection,
                &feedback_id,
                ReviewCandidateInput {
                    review_body: "The onboarding made the next decision easy.".to_string(),
                    evidence_refs: vec![feedback_id.clone()],
                    provenance: json!({
                        "workflow": "review_candidate_consent_publication_boundary",
                        "source": "private_feedback",
                    }),
                },
            )?;
            ensure!(
                review.status == ReviewStatus::Candidate,
                "review should start as candidate"
            );
            ensure!(
                review.publication_visibility == "private_until_approved",
                "review candidate must remain private before consent and approval"
            );
        }
        "assert_publish_blocked_before_consent_approval" => {
            let review_id = latest_review_id(connection)?;
            let blocked = transition_review(
                connection,
                &review_id,
                ReviewStatus::Published,
                vec!["publish_attempt_1".to_string()],
                "attempted early publication",
            );
            ensure!(
                blocked.is_err(),
                "review publication should fail closed before consent and approval"
            );
            ensure!(
                list_public_reviews(connection)?.is_empty(),
                "early publication failure should not expose public reviews"
            );
        }
        "complete_review_consent_approval_publication_lifecycle" => {
            let review_id = latest_review_id(connection)?;
            let (requested, _) = transition_review(
                connection,
                &review_id,
                ReviewStatus::Requested,
                vec!["review_request_1".to_string()],
                "request review",
            )?;
            let (received, _) = transition_review(
                connection,
                &requested.id,
                ReviewStatus::Received,
                vec!["review_received_1".to_string()],
                "review received",
            )?;
            let (consented, _) = transition_review(
                connection,
                &received.id,
                ReviewStatus::ConsentConfirmed,
                vec!["review_consent_1".to_string()],
                "customer consent confirmed",
            )?;
            let (approved, _) = transition_review(
                connection,
                &consented.id,
                ReviewStatus::Approved,
                vec!["review_approval_1".to_string()],
                "staff approved publication",
            )?;
            let (published, _) = transition_review(
                connection,
                &approved.id,
                ReviewStatus::Published,
                vec!["review_publish_1".to_string()],
                "publish public review",
            )?;
            ensure!(
                list_public_reviews(connection)?.len() == 1,
                "published review should become public only after consent and approval"
            );
            let (featured, _) = transition_review(
                connection,
                &published.id,
                ReviewStatus::Featured,
                vec!["review_feature_1".to_string()],
                "feature review",
            )?;
            let (retired, _) = transition_review(
                connection,
                &featured.id,
                ReviewStatus::Retired,
                vec!["review_retire_1".to_string()],
                "retire review",
            )?;
            ensure!(
                retired.status == ReviewStatus::Retired,
                "review lifecycle should support retired state"
            );
            ensure!(
                list_public_reviews(connection)?.is_empty(),
                "retired reviews should leave the public review list"
            );
        }
        other => anyhow::bail!("unsupported review workflow eval step: {other}"),
    }
    Ok(())
}

pub(crate) fn run_home_about_public_narrative_brief_step(
    connection: &Connection,
    step: &EvalStep,
) -> Result<()> {
    match step.id.as_str() {
        "seed_home_about_surface_evidence" => {
            seed_home_about_surface_evidence(connection)?;
        }
        "assert_home_about_public_contract" => {
            seed_home_about_surface_evidence(connection)?;
            let contract = public_product_surface_contract_connection(connection)?;
            ensure!(
                contract.home_about.billboards.len() == 1,
                "Home/About should expose one public billboard from seeded evidence"
            );
            let billboard = &contract.home_about.billboards[0];
            ensure!(
                billboard.status == "pinned",
                "Home/About billboard should preserve the public state contract"
            );
            ensure!(
                !billboard.evidence.is_empty(),
                "Home/About billboard claims require durable evidence"
            );
            ensure!(
                !billboard.reduced_motion_fallback.trim().is_empty(),
                "Home/About billboard requires a reduced-motion fallback"
            );
            ensure!(
                billboard.links.contains(&"/chat".to_string()),
                "Home/About should keep Chat reachable instead of replacing source surfaces"
            );
            let public_contract = serde_json::to_string(&contract)?;
            ensure!(
                !public_contract.contains("alex@example.com"),
                "public Home/About contract must not expose private fixture text"
            );
            ensure!(
                !public_contract.contains("Private operator-only positioning"),
                "public Home/About contract must not expose staff/private facts"
            );
            ensure!(
                !public_contract.contains("Draft billboard"),
                "public Home/About contract must not expose draft billboards"
            );
        }
        other => anyhow::bail!("unsupported Home/About product surface eval step: {other}"),
    }
    Ok(())
}

pub(crate) fn run_offer_ask_machine_readable_intent_step(
    connection: &Connection,
    step: &EvalStep,
) -> Result<()> {
    match step.id.as_str() {
        "seed_offer_ask_intent_evidence" => {
            seed_offer_ask_intent_evidence(connection)?;
        }
        "assert_offer_ask_intent_contract" => {
            seed_offer_ask_intent_evidence(connection)?;
            let contract = public_product_surface_contract_connection(connection)?;
            ensure!(
                contract.offer_intents.len() == 1,
                "offer intent contract should expose one seeded public offer"
            );
            ensure!(
                contract.ask_intents.len() == 1,
                "ask intent contract should expose one seeded public ask"
            );
            let offer = &contract.offer_intents[0];
            ensure!(
                offer.human_readable.contains("Starter Sprint"),
                "offer intent must keep human-readable page copy"
            );
            ensure!(
                offer.machine_readable["intentKind"] == "offer",
                "offer intent must include machine-readable intent kind"
            );
            ensure!(
                offer.machine_readable["a2aStatus"] == "future_contract",
                "offer intent should not claim external A2A implementation"
            );
            ensure!(
                !offer.evidence.is_empty(),
                "offer intent requires evidence refs"
            );
            let ask = &contract.ask_intents[0];
            ensure!(
                ask.machine_readable["decisionBoundary"]
                    == "human_or_policy_decides_what_becomes_real",
                "ask intent must preserve human/policy decision boundary"
            );
        }
        "assert_fake_persuasion_rejected" => {
            seed_offer_ask_intent_evidence(connection)?;
            insert_eval_business_fact(
                connection,
                "business_fact_eval_offer_fake_title",
                "offers.rush.title",
                json!("Rush Offer"),
                "public",
                "published",
                "eval_fixture",
            )?;
            insert_eval_business_fact(
                connection,
                "business_fact_eval_offer_fake_summary",
                "offers.rush.summary",
                json!("Act now before it disappears."),
                "public",
                "published",
                "eval_fixture",
            )?;
            insert_eval_business_fact(
                connection,
                "business_fact_eval_offer_fake_scarcity",
                "offers.rush.scarcity",
                json!("Only two spots left."),
                "public",
                "published",
                "eval_fixture",
            )?;
            let error = public_product_surface_contract_connection(connection)
                .expect_err("unsupported public scarcity should be rejected");
            ensure!(
                error
                    .to_string()
                    .contains("unsupported public persuasion proof"),
                "fake scarcity rejection should be structured and inspectable"
            );
        }
        other => anyhow::bail!("unsupported Offer/Ask product surface eval step: {other}"),
    }
    Ok(())
}

pub(crate) fn seed_home_about_surface_evidence(connection: &Connection) -> Result<()> {
    for (id, fact_key, value, visibility, publication_state) in [
        (
            "business_fact_eval_home_status",
            "about.billboards.hero.status",
            json!("pinned"),
            "public",
            "published",
        ),
        (
            "business_fact_eval_home_headline",
            "about.billboards.hero.headline",
            json!("Proof-backed client operations"),
            "public",
            "published",
        ),
        (
            "business_fact_eval_home_body",
            "about.billboards.hero.body",
            json!("Ordo turns relationship evidence into a usable next action."),
            "public",
            "published",
        ),
        (
            "business_fact_eval_home_motion",
            "about.billboards.hero.reducedMotionFallback",
            json!("Static proof-backed narrative with the same claims."),
            "public",
            "published",
        ),
        (
            "business_fact_eval_home_links",
            "about.billboards.hero.links",
            json!([
                "/offers/starter-sprint",
                "/asks/referrals",
                "/latest",
                "/chat"
            ]),
            "public",
            "published",
        ),
        (
            "business_fact_eval_home_draft",
            "about.billboards.draft.body",
            json!("Draft billboard"),
            "public",
            "draft",
        ),
        (
            "business_fact_eval_home_private",
            "about.billboards.private.body",
            json!("Private operator-only positioning for alex@example.com"),
            "staff",
            "published",
        ),
    ] {
        insert_eval_business_fact(
            connection,
            id,
            fact_key,
            value,
            visibility,
            publication_state,
            "eval_fixture",
        )?;
    }

    record_artifact(
        connection,
        ArtifactInput {
            artifact_kind: "home_about.narrative".to_string(),
            title: "Home/About narrative evidence".to_string(),
            status: "published".to_string(),
            visibility_ceiling: "public".to_string(),
            summary: "Evidence packet for the public Home/About narrative.".to_string(),
            source_kind: Some("public_surface".to_string()),
            source_id: Some("home_about".to_string()),
            evidence_refs: vec![
                "business_fact_eval_home_headline".to_string(),
                "business_fact_eval_home_body".to_string(),
            ],
            provenance: json!({
                "workflow": "home_about_public_narrative_brief",
                "mode": "deterministic_eval",
            }),
            content_hash: stable_text_hash("home_about_public_narrative_brief"),
            storage_uri: None,
            health_status: Some("available".to_string()),
            created_by_job_id: None,
        },
    )?;
    connection.execute(
        "INSERT OR IGNORE INTO surface_briefs (
            id, surface_kind, subject_kind, subject_id, status, artifact_id, title,
            brief_markdown, evidence_refs_json, limitations_json, created_by_job_id,
            generated_at, created_at, updated_at, completed_at
         ) VALUES (
            'surface_brief_eval_home_about', 'home_about', NULL, NULL, 'completed',
            (SELECT id FROM artifacts WHERE artifact_kind = 'home_about.narrative' ORDER BY created_at DESC LIMIT 1),
            'Home/About public narrative',
            'Proof-backed public narrative. Limitation: deterministic eval fixture only.',
            '[\"business_fact_eval_home_headline\",\"business_fact_eval_home_body\"]',
            '[\"No frontend scrollytelling is implemented in this eval slice.\"]',
            NULL, '2026-05-09T00:00:00Z', '2026-05-09T00:00:00Z',
            '2026-05-09T00:00:00Z', '2026-05-09T00:00:00Z'
         )",
        [],
    )?;
    Ok(())
}

pub(crate) fn seed_offer_ask_intent_evidence(connection: &Connection) -> Result<()> {
    for (id, fact_key, value) in [
        (
            "business_fact_eval_offer_title",
            "offers.starter_sprint.title",
            json!("Starter Sprint"),
        ),
        (
            "business_fact_eval_offer_summary",
            "offers.starter_sprint.summary",
            json!("A focused implementation sprint with evidence-backed scope."),
        ),
        (
            "business_fact_eval_offer_intent",
            "offers.starter_sprint.intent",
            json!({
                "object": "offer",
                "outputArtifact": "implementation_plan",
                "approvalRequired": true,
                "startPath": "/chat",
                "outcomeRefs": ["business_outcome_eval_1"]
            }),
        ),
        (
            "business_fact_eval_offer_terms",
            "offers.starter_sprint.terms",
            json!({
                "humanReadable": "Start in chat; staff confirms fit before anything becomes real.",
                "policyDecides": true
            }),
        ),
        (
            "business_fact_eval_ask_title",
            "asks.referrals.title",
            json!("Referral fit"),
        ),
        (
            "business_fact_eval_ask_summary",
            "asks.referrals.summary",
            json!("Introduce teams that need proof-backed operations."),
        ),
        (
            "business_fact_eval_ask_intent",
            "asks.referrals.intent",
            json!({
                "object": "ask",
                "requestedInput": "warm_intro",
                "respondPath": "/chat",
                "referralRefs": ["referral_eval_1"]
            }),
        ),
        (
            "business_fact_eval_ask_terms",
            "asks.referrals.terms",
            json!({
                "humanReadable": "Only make intros with consent.",
                "approvalRequired": true
            }),
        ),
    ] {
        insert_eval_business_fact(
            connection,
            id,
            fact_key,
            value,
            "public",
            "published",
            "eval_fixture",
        )?;
    }
    Ok(())
}

pub(crate) fn insert_eval_business_fact(
    connection: &Connection,
    id: &str,
    fact_key: &str,
    value: serde_json::Value,
    visibility: &str,
    publication_state: &str,
    source_kind: &str,
) -> Result<()> {
    connection.execute(
        "INSERT OR IGNORE INTO business_facts (
            id, subject_type, subject_id, fact_key, value_json, source_kind, source_label,
            source_uri, provenance_json, visibility, publication_state, created_by_actor_id,
            created_at, updated_at, published_at, archived_at
         ) VALUES (
            ?1, 'public_surface', 'eval_business', ?2, ?3, ?4, 'Product surface eval',
            NULL, ?5, ?6, ?7, 'actor_staff_eval_1',
            '2026-05-09T00:00:00Z', '2026-05-09T00:00:00Z',
            CASE WHEN ?7 = 'published' THEN '2026-05-09T00:00:00Z' ELSE NULL END,
            CASE WHEN ?7 IN ('archived', 'revoked') THEN '2026-05-09T00:00:00Z' ELSE NULL END
         )",
        rusqlite::params![
            id,
            fact_key,
            value.to_string(),
            source_kind,
            json!({
                "workflow": "product_surface_workflow_eval",
                "evidenceBacked": true,
            })
            .to_string(),
            visibility,
            publication_state,
        ],
    )?;
    Ok(())
}

pub(crate) fn seed_feedback_source_message(connection: &Connection) -> Result<(String, String)> {
    let conversation =
        find_or_create_canonical_conversation(connection, &feedback_conversation_request())?;
    let participant = create_conversation_participant(
        connection,
        &ConversationParticipantCreateRequest {
            conversation_id: conversation.id.clone(),
            participant_kind: "connection".to_string(),
            actor_id: Some("actor_client_eval_1".to_string()),
            connection_id: Some("connection_eval_1".to_string()),
            visitor_session_id: None,
            display_name: "Eval Client".to_string(),
            role: "client".to_string(),
        },
    )?;
    let message = create_conversation_message(
        connection,
        &ConversationMessageCreateRequest {
            conversation_id: conversation.id.clone(),
            segment_id: None,
            participant_id: participant.id,
            message_kind: "user".to_string(),
            body_markdown:
                "The onboarding was clear. Please keep my email alex@example.com private."
                    .to_string(),
            visibility: "participants".to_string(),
            client_message_id: "eval-feedback-source-message-1".to_string(),
            reply_to_message_id: None,
            undo_expires_at: None,
        },
    )?;
    Ok((conversation.id, message.id))
}

pub(crate) fn ensure_review_feedback(connection: &Connection) -> Result<String> {
    if let Some(id) = connection
        .query_row(
            "SELECT id FROM customer_feedback ORDER BY created_at DESC LIMIT 1",
            [],
            |row| row.get(0),
        )
        .optional()?
    {
        return Ok(id);
    }

    let (conversation_id, message_id) = seed_feedback_source_message(connection)?;
    let (feedback, _) = capture_feedback(
        connection,
        CustomerFeedbackInput {
            connection_id: Some("connection_eval_1".to_string()),
            conversation_id,
            segment_id: None,
            message_id: Some(message_id.clone()),
            feedback_kind: "praise".to_string(),
            body_summary: "Client says onboarding made the next decision easy.".to_string(),
            source_refs: vec![message_id.clone()],
            evidence_refs: vec![message_id],
            provenance: json!({
                "workflow": "review_candidate_consent_publication_boundary",
                "source": "conversation_message",
            }),
        },
    )?;
    Ok(feedback.id)
}

pub(crate) fn latest_feedback_id(connection: &Connection) -> Result<String> {
    connection
        .query_row(
            "SELECT id FROM customer_feedback ORDER BY created_at DESC LIMIT 1",
            [],
            |row| row.get(0),
        )
        .optional()?
        .ok_or_else(|| anyhow::anyhow!("expected feedback row"))
}

pub(crate) fn latest_review_id(connection: &Connection) -> Result<String> {
    connection
        .query_row(
            "SELECT id FROM customer_reviews ORDER BY created_at DESC LIMIT 1",
            [],
            |row| row.get(0),
        )
        .optional()?
        .ok_or_else(|| anyhow::anyhow!("expected review row"))
}

pub(crate) fn feedback_conversation_request() -> CanonicalConversationRequest {
    CanonicalConversationRequest {
        surface: "client_portal".to_string(),
        subject_kind: "connection".to_string(),
        subject_id: "connection_eval_1".to_string(),
        connection_id: Some("connection_eval_1".to_string()),
        visitor_session_id: None,
        created_by_actor_id: Some("actor_client_eval_1".to_string()),
    }
}

pub(crate) fn visitor_conversation_request() -> CanonicalConversationRequest {
    CanonicalConversationRequest {
        surface: "chat".to_string(),
        subject_kind: "visitor_session".to_string(),
        subject_id: "visitor_session_eval_1".to_string(),
        connection_id: None,
        visitor_session_id: Some("visitor_session_eval_1".to_string()),
        created_by_actor_id: None,
    }
}

pub(crate) fn create_visitor_participant(
    connection: &Connection,
    conversation_id: &str,
) -> Result<crate::conversations::ConversationParticipantView> {
    create_conversation_participant(
        connection,
        &ConversationParticipantCreateRequest {
            conversation_id: conversation_id.to_string(),
            participant_kind: "visitor".to_string(),
            actor_id: None,
            connection_id: None,
            visitor_session_id: Some("visitor_session_eval_1".to_string()),
            display_name: "Visitor".to_string(),
            role: "client".to_string(),
        },
    )
}

pub(crate) fn conversation_and_assistant(connection: &Connection) -> Result<(String, String)> {
    let conversation = find_or_create_canonical_conversation(
        connection,
        &CanonicalConversationRequest {
            surface: "client_portal".to_string(),
            subject_kind: "connection".to_string(),
            subject_id: "connection_eval_1".to_string(),
            connection_id: Some("connection_eval_1".to_string()),
            visitor_session_id: None,
            created_by_actor_id: Some("actor_staff_eval_1".to_string()),
        },
    )?;
    let assistant = create_conversation_participant(
        connection,
        &ConversationParticipantCreateRequest {
            conversation_id: conversation.id.clone(),
            participant_kind: "assistant".to_string(),
            actor_id: None,
            connection_id: None,
            visitor_session_id: None,
            display_name: "Ordo".to_string(),
            role: "assistant".to_string(),
        },
    )?;
    Ok((conversation.id, assistant.id))
}

