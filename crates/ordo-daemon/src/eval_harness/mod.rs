use crate::artifacts::*;
use crate::conversations::*;
use crate::feedback::*;
use crate::llm_gateway::*;
use crate::policy::*;

pub mod cases;
pub mod core;
pub mod types;

pub use cases::*;
pub use core::*;
pub use types::*;

pub const EVAL_HARNESS_SCHEMA_VERSION: &str = "ordo.eval_harness.v1";
pub const EVAL_ARTIFACT_PACKET_SCHEMA_VERSION: &str = "ordo.eval_artifact_packet.v1";

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, Value};
    use std::fs;

    #[test]
    fn isolated_eval_store_initializes_current_schema() {
        let connection = isolated_eval_connection().unwrap();
        let table_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'conversation_events'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(table_count, 1);
    }

    #[test]
    fn harness_runs_steps_in_stable_order_and_records_scorecard() {
        let connection = isolated_eval_connection().unwrap();
        let mut harness = DeterministicEvalHarness::new(DeterministicEvalClock::fixed())
            .with_artifact_path("tests/evals/backend/scorecards/relationship_message.json");

        let scorecard = harness
            .run_case(
                &connection,
                &relationship_conversation_message_case().unwrap(),
                run_relationship_conversation_step,
            )
            .unwrap();

        assert!(scorecard.passed);
        assert_eq!(scorecard.schema_version, EVAL_HARNESS_SCHEMA_VERSION);
        assert_eq!(scorecard.step_count, 2);
        assert_eq!(scorecard.provider_mode, "deterministic_only");
        assert!(!scorecard.network_enabled);
        assert_eq!(
            scorecard.actor_roles,
            vec![
                EvalActorRole::AnonymousVisitor,
                EvalActorRole::Staff,
                EvalActorRole::LlmToolProviderBoundary,
            ]
        );
        assert_eq!(
            scorecard
                .evidence_after
                .count_for(EvalEvidenceChannel::PromptSlotAccounting),
            0
        );
        assert_eq!(
            scorecard
                .evidence_after
                .count_for(EvalEvidenceChannel::TokenLedger),
            0
        );
        assert!(scorecard
            .evidence_after
            .conversation_event_max_sequence
            .is_some());
        assert!(scorecard
            .evidence_after
            .realtime_replay_max_cursor
            .is_some());
    }

    #[test]
    fn repeated_harness_runs_are_stable_except_durable_ids() {
        let first_connection = isolated_eval_connection().unwrap();
        let second_connection = isolated_eval_connection().unwrap();
        let case = relationship_conversation_message_case().unwrap();

        let mut first = DeterministicEvalHarness::new(DeterministicEvalClock::fixed());
        let mut second = DeterministicEvalHarness::new(DeterministicEvalClock::fixed());
        let first_scorecard = first
            .run_case(&first_connection, &case, run_relationship_conversation_step)
            .unwrap();
        let second_scorecard = second
            .run_case(
                &second_connection,
                &case,
                run_relationship_conversation_step,
            )
            .unwrap();

        assert_eq!(first_scorecard.fixture_hash, second_scorecard.fixture_hash);
        assert_eq!(first_scorecard.actor_roles, second_scorecard.actor_roles);
        assert_eq!(first_scorecard.step_count, second_scorecard.step_count);
        assert_eq!(
            first_scorecard.provider_mode,
            second_scorecard.provider_mode
        );
        assert_eq!(
            first_scorecard.network_enabled,
            second_scorecard.network_enabled
        );
        assert_eq!(
            first_scorecard.assertion_results,
            second_scorecard.assertion_results
        );
        assert_eq!(
            first_scorecard
                .evidence_after
                .count_for(EvalEvidenceChannel::ConversationEvents),
            second_scorecard
                .evidence_after
                .count_for(EvalEvidenceChannel::ConversationEvents)
        );
        assert_eq!(
            first_scorecard
                .evidence_after
                .count_for(EvalEvidenceChannel::RealtimeReplay),
            second_scorecard
                .evidence_after
                .count_for(EvalEvidenceChannel::RealtimeReplay)
        );
    }

    #[test]
    fn artifact_packet_writer_emits_redacted_packet_scorecard_and_manifest() {
        let connection = isolated_eval_connection().unwrap();
        let case = relationship_conversation_message_case().unwrap();
        let temp_dir = tempfile::TempDir::new().unwrap();
        let mut harness = DeterministicEvalHarness::new(DeterministicEvalClock::fixed())
            .with_artifact_path(
                temp_dir
                    .path()
                    .join("relationship_message_foundation-packet.json")
                    .to_string_lossy(),
            );
        let scorecard = harness
            .run_case(&connection, &case, run_relationship_conversation_step)
            .unwrap();
        let writer = EvalArtifactWriter::new(temp_dir.path(), "test-commit")
            .with_private_terms(vec!["package".to_string()]);

        let paths = writer.write_packet(&connection, &case, &scorecard).unwrap();

        assert!(paths.packet_path.exists());
        assert!(paths.scorecard_path.exists());
        assert!(paths.manifest_path.exists());
        let packet_json = fs::read_to_string(&paths.packet_path).unwrap();
        let scorecard_json = fs::read_to_string(&paths.scorecard_path).unwrap();
        let manifest_json = fs::read_to_string(&paths.manifest_path).unwrap();
        assert!(packet_json.contains(EVAL_ARTIFACT_PACKET_SCHEMA_VERSION));
        assert!(packet_json.contains("\"conversationEventLedger\""));
        assert!(packet_json.contains("\"realtimeReplayLedger\""));
        assert!(packet_json.contains("\"policyDecisionLedger\": []"));
        assert!(packet_json.contains("\"promptSlotLedger\": []"));
        assert!(packet_json.contains("\"artifactReview\""));
        assert!(packet_json.contains("[REDACTED:email]"));
        assert!(packet_json.contains("[REDACTED:phone]"));
        assert!(packet_json.contains("[REDACTED:secret]"));
        assert!(packet_json.contains("[REDACTED:private_term]"));
        assert!(!packet_json.contains("alex@example.com"));
        assert!(!packet_json.contains("555-123-4567"));
        assert!(!packet_json.contains("sk-eval-secret"));
        assert!(scorecard_json.contains("\"providerMode\": \"deterministic_only\""));
        assert!(manifest_json.contains("\"sourceCommit\": \"test-commit\""));
    }

    #[test]
    fn normalized_artifact_packets_are_deterministic() {
        let first_connection = isolated_eval_connection().unwrap();
        let second_connection = isolated_eval_connection().unwrap();
        let case = relationship_conversation_message_case().unwrap();
        let mut first_harness = DeterministicEvalHarness::new(DeterministicEvalClock::fixed());
        let mut second_harness = DeterministicEvalHarness::new(DeterministicEvalClock::fixed());
        let first_scorecard = first_harness
            .run_case(&first_connection, &case, run_relationship_conversation_step)
            .unwrap();
        let second_scorecard = second_harness
            .run_case(
                &second_connection,
                &case,
                run_relationship_conversation_step,
            )
            .unwrap();
        let writer = EvalArtifactWriter::new("unused", "test-commit")
            .with_private_terms(vec!["package".to_string()]);

        let first_packet = writer
            .build_packet(&first_connection, &case, &first_scorecard)
            .unwrap();
        let second_packet = writer
            .build_packet(&second_connection, &case, &second_scorecard)
            .unwrap();

        assert_eq!(
            normalized_packet_for_comparison(&first_packet),
            normalized_packet_for_comparison(&second_packet)
        );
    }

    fn normalized_packet_for_comparison(packet: &EvalArtifactPacket) -> Value {
        json!({
            "schemaVersion": packet.schema_version,
            "caseId": packet.case_id,
            "fixtureHash": packet.fixture_hash,
            "actorRoles": packet.actor_roles,
            "stepCount": packet.steps.len(),
            "scorecardPassed": packet.scorecard.passed,
            "transcriptTypes": packet.transcript.iter().map(|entry| entry.entry_type.clone()).collect::<Vec<_>>(),
            "conversationEventTypes": packet.conversation_event_ledger.iter().map(|entry| entry.entry_type.clone()).collect::<Vec<_>>(),
            "realtimeReplayTypes": packet.realtime_replay_ledger.iter().map(|entry| entry.entry_type.clone()).collect::<Vec<_>>(),
            "policyCount": packet.policy_decision_ledger.len(),
            "promptSlotCount": packet.prompt_slot_ledger.len(),
            "redactedCount": packet.redaction_summary.redacted_value_count,
        })
    }

    #[test]
    fn relationship_workflow_eval_writes_artifacts() {
        let connection = isolated_eval_connection().unwrap();
        let temp_dir = tempfile::TempDir::new().unwrap();

        let run =
            run_relationship_conversation_message_eval(&connection, temp_dir.path(), "test-commit")
                .unwrap();

        assert!(run.scorecard.passed);
        assert_eq!(run.case.id, "relationship_conversation_message");
        assert!(run.artifact_paths.packet_path.exists());
        let packet = fs::read_to_string(run.artifact_paths.packet_path).unwrap();
        assert!(packet.contains("\"caseId\": \"relationship_conversation_message\""));
        assert!(packet.contains("\"conversationEventLedger\""));
        assert!(packet.contains("[REDACTED:email]"));
        assert!(!packet.contains("alex@example.com"));
    }

    #[test]
    fn privacy_gateway_roundtrip_eval_records_privacy_accounting_and_artifacts() {
        let connection = isolated_eval_connection().unwrap();
        let temp_dir = tempfile::TempDir::new().unwrap();
        let db_path = temp_dir.path().join("local.db");
        let artifact_dir = temp_dir.path().join("artifacts");

        let run =
            run_privacy_gateway_roundtrip_eval(&db_path, &connection, &artifact_dir, "test-commit")
                .unwrap();

        assert!(run.scorecard.passed);
        assert_eq!(run.case.id, "privacy_gateway_roundtrip");
        assert!(run.artifact_paths.packet_path.exists());
        assert!(
            run.scorecard
                .evidence_after
                .count_for(EvalEvidenceChannel::PolicyDecisions)
                >= 1
        );
        assert!(
            run.scorecard
                .evidence_after
                .count_for(EvalEvidenceChannel::PromptSlotAccounting)
                >= 2
        );
        assert!(
            run.scorecard
                .evidence_after
                .count_for(EvalEvidenceChannel::PrivacyTransforms)
                >= 1
        );
        assert!(
            run.scorecard
                .evidence_after
                .count_for(EvalEvidenceChannel::TokenLedger)
                >= 2
        );
        let packet = fs::read_to_string(run.artifact_paths.packet_path).unwrap();
        assert!(packet.contains("\"caseId\": \"privacy_gateway_roundtrip\""));
        assert!(packet.contains("\"promptSlotLedger\""));
        assert!(packet.contains("\"privacyTransformLedger\""));
        assert!(packet.contains("\"tokenLedger\""));
        assert!(!packet.contains("Project Orchid"));
        assert!(!packet.contains("alex@example.com"));
        assert!(!packet.contains("555-123-4567"));
        assert!(!packet.contains("sk-eval-secret"));
    }

    #[test]
    fn replay_provider_fixture_eval_records_accounting_and_artifacts() {
        let connection = isolated_eval_connection().unwrap();
        let temp_dir = tempfile::TempDir::new().unwrap();
        let db_path = temp_dir.path().join("local.db");
        let artifact_dir = temp_dir.path().join("artifacts");

        let run =
            run_replay_provider_fixture_eval(&db_path, &connection, &artifact_dir, "test-commit")
                .unwrap();

        assert!(run.scorecard.passed);
        assert_eq!(run.case.id, "replay_provider_fixture");
        assert_eq!(run.scorecard.provider_mode, "replay_fixture");
        assert!(run.artifact_paths.packet_path.exists());
        assert!(
            run.scorecard
                .evidence_after
                .count_for(EvalEvidenceChannel::PolicyDecisions)
                >= 1
        );
        assert!(
            run.scorecard
                .evidence_after
                .count_for(EvalEvidenceChannel::PromptSlotAccounting)
                >= 1
        );
        assert!(
            run.scorecard
                .evidence_after
                .count_for(EvalEvidenceChannel::TokenLedger)
                >= 2
        );
        let packet = fs::read_to_string(run.artifact_paths.packet_path).unwrap();
        assert!(packet.contains("\"caseId\": \"replay_provider_fixture\""));
        assert!(packet.contains("\"providerId\": \"replay_fixture\""));
        assert!(packet.contains("\"modelId\": \"replay-chat\""));
        assert!(packet.contains("\"tokenLedger\""));
        assert!(!packet.contains("Project Orchid"));
        assert!(!packet.contains("alex@example.com"));
        assert!(!packet.contains("sk-eval-secret"));
    }

    #[test]
    fn role_lifecycle_anonymous_client_affiliate_eval_writes_boundary_artifacts() {
        let connection = isolated_eval_connection().unwrap();
        let temp_dir = tempfile::TempDir::new().unwrap();

        let run = run_role_lifecycle_anonymous_to_client_eval(
            &connection,
            temp_dir.path(),
            "test-commit",
        )
        .unwrap();

        assert!(run.scorecard.passed);
        assert_eq!(run.case.id, "role_lifecycle_anonymous_to_client");
        assert_eq!(
            run.scorecard.actor_roles,
            vec![
                EvalActorRole::AnonymousVisitor,
                EvalActorRole::ClientMember,
                EvalActorRole::Affiliate,
            ]
        );
        assert!(
            run.scorecard
                .evidence_after
                .count_for(EvalEvidenceChannel::PolicyDecisions)
                >= 2
        );
        assert_eq!(
            connection
                .query_row(
                    "SELECT COUNT(*) FROM conversations
                     WHERE subject_kind = 'connection' AND subject_id = 'connection_eval_1'",
                    [],
                    |row| row.get::<_, i64>(0),
                )
                .unwrap(),
            1
        );
        let packet = fs::read_to_string(run.artifact_paths.packet_path).unwrap();
        assert!(packet.contains("\"caseId\": \"role_lifecycle_anonymous_to_client\""));
        assert!(packet.contains("\"anonymous_visitor\""));
        assert!(packet.contains("\"client_member\""));
        assert!(packet.contains("\"affiliate\""));
        assert!(packet.contains("affiliate_cannot_access_unrelated_customer_conversation"));
        assert!(packet.contains("[REDACTED:email]"));
        assert!(!packet.contains("alex@example.com"));
        assert!(packet.contains("\"handoffLedger\": []"));
        assert!(packet.contains("\"promptSlotLedger\": []"));
        assert!(packet.contains("\"privacyTransformLedger\": []"));
        assert!(packet.contains("\"tokenLedger\": []"));
    }

    #[test]
    fn role_lifecycle_staff_manager_owner_eval_asserts_queue_and_system_boundaries() {
        let connection = isolated_eval_connection().unwrap();
        let temp_dir = tempfile::TempDir::new().unwrap();

        let run = run_role_lifecycle_staff_manager_owner_eval(
            &connection,
            temp_dir.path(),
            "test-commit",
        )
        .unwrap();

        assert!(run.scorecard.passed);
        assert_eq!(run.case.id, "role_lifecycle_staff_manager_owner_boundaries");
        assert!(
            run.scorecard
                .evidence_after
                .count_for(EvalEvidenceChannel::HandoffState)
                >= 1
        );
        assert!(
            run.scorecard
                .evidence_after
                .count_for(EvalEvidenceChannel::PolicyDecisions)
                >= 2
        );
        let packet = fs::read_to_string(run.artifact_paths.packet_path).unwrap();
        assert!(packet.contains("\"staff\""));
        assert!(packet.contains("\"manager_admin\""));
        assert!(packet.contains("\"owner_system_admin\""));
        assert!(packet.contains("\"staffDefault\": \"my_handoffs\""));
        assert!(packet.contains("\"managerAllowed\": \"team_queue\""));
        assert!(packet.contains("\"ownerAllowed\": \"all_conversations\""));
        assert!(packet.contains("Protected daemon route requires loopback access"));
    }

    #[test]
    fn role_lifecycle_agent_silence_eval_records_human_led_boundary() {
        let connection = isolated_eval_connection().unwrap();
        let temp_dir = tempfile::TempDir::new().unwrap();

        let run =
            run_role_lifecycle_agent_silence_eval(&connection, temp_dir.path(), "test-commit")
                .unwrap();

        assert!(run.scorecard.passed);
        assert_eq!(run.case.id, "role_lifecycle_agent_silence_boundary");
        assert!(
            run.scorecard
                .evidence_after
                .count_for(EvalEvidenceChannel::PolicyDecisions)
                >= 1
        );
        let packet = fs::read_to_string(run.artifact_paths.packet_path).unwrap();
        assert!(packet.contains("\"ordo_agent\""));
        assert!(packet.contains("\"human_led_active\""));
        assert!(packet.contains("human_led_active_requires_tag_delegation_or_policy"));
        assert!(packet.contains("\"clientVisibleMechanics\": \"hidden\""));
        assert!(packet.contains("\"promptSlotLedger\": []"));
        assert!(packet.contains("\"privacyTransformLedger\": []"));
        assert!(packet.contains("\"tokenLedger\": []"));
    }

    #[test]
    fn feedback_capture_eval_records_private_feedback_and_tag_candidate() {
        let connection = isolated_eval_connection().unwrap();
        let temp_dir = tempfile::TempDir::new().unwrap();

        let run = run_feedback_capture_private_business_intelligence_eval(
            &connection,
            temp_dir.path(),
            "test-commit",
        )
        .unwrap();

        assert!(run.scorecard.passed);
        assert_eq!(
            run.case.id,
            "feedback_capture_private_business_intelligence"
        );
        assert!(
            run.scorecard
                .evidence_after
                .count_for(EvalEvidenceChannel::FeedbackReviewRecords)
                >= 2
        );
        assert_eq!(list_public_reviews(&connection).unwrap().len(), 0);
        let packet = fs::read_to_string(run.artifact_paths.packet_path).unwrap();
        assert!(packet.contains("\"feedbackLedger\""));
        assert!(packet.contains("\"reviewLedger\": []"));
        assert!(packet.contains("private_business_intelligence"));
        assert!(packet.contains("staff_signal_not_customer_rating"));
        assert!(packet.contains("clear_onboarding"));
        assert!(packet.contains("[REDACTED:email]"));
        assert!(!packet.contains("alex@example.com"));
    }

    #[test]
    fn review_candidate_eval_blocks_publication_until_consent_and_approval() {
        let connection = isolated_eval_connection().unwrap();
        let temp_dir = tempfile::TempDir::new().unwrap();

        let run = run_review_candidate_consent_publication_boundary_eval(
            &connection,
            temp_dir.path(),
            "test-commit",
        )
        .unwrap();

        assert!(run.scorecard.passed);
        assert_eq!(run.case.id, "review_candidate_consent_publication_boundary");
        assert!(
            run.scorecard
                .evidence_after
                .count_for(EvalEvidenceChannel::FeedbackReviewRecords)
                >= 2
        );
        assert_eq!(list_public_reviews(&connection).unwrap().len(), 0);
        let packet = fs::read_to_string(run.artifact_paths.packet_path).unwrap();
        assert!(packet.contains("\"reviewLedger\""));
        assert!(packet.contains("private_until_approved"));
        assert!(packet.contains("public_review"));
        assert!(packet.contains("consentEvidenceRefs"));
        assert!(packet.contains("approvalEvidenceRefs"));
        assert!(packet.contains("\"entryType\": \"retired\""));
        assert!(!packet.contains("fake review"));
    }

    #[test]
    fn home_about_public_narrative_eval_writes_surface_artifacts() {
        let connection = isolated_eval_connection().unwrap();
        let temp_dir = tempfile::TempDir::new().unwrap();

        let run =
            run_home_about_public_narrative_brief_eval(&connection, temp_dir.path(), "test-commit")
                .unwrap();

        assert!(run.scorecard.passed);
        assert_eq!(run.case.id, "home_about_public_narrative_brief");
        assert!(
            run.scorecard
                .evidence_after
                .count_for(EvalEvidenceChannel::ProductSurfaceRecords)
                >= 6
        );
        assert!(
            run.scorecard
                .evidence_after
                .count_for(EvalEvidenceChannel::SurfaceBriefRecords)
                >= 1
        );
        let packet = fs::read_to_string(run.artifact_paths.packet_path).unwrap();
        assert!(packet.contains("\"productSurfaceLedger\""));
        assert!(packet.contains("about.billboards.hero.headline"));
        assert!(packet.contains("reducedMotionFallback"));
        assert!(packet.contains("/chat"));
        assert!(packet.contains("[REDACTED:email]"));
        assert!(!packet.contains("alex@example.com"));
    }

    #[test]
    fn offer_ask_intent_eval_writes_machine_readable_contract_artifacts() {
        let connection = isolated_eval_connection().unwrap();
        let temp_dir = tempfile::TempDir::new().unwrap();

        let run =
            run_offer_ask_machine_readable_intent_eval(&connection, temp_dir.path(), "test-commit")
                .unwrap();

        assert!(run.scorecard.passed);
        assert_eq!(run.case.id, "offer_ask_machine_readable_intent");
        assert!(
            run.scorecard
                .evidence_after
                .count_for(EvalEvidenceChannel::ProductSurfaceRecords)
                >= 8
        );
        let packet = fs::read_to_string(run.artifact_paths.packet_path).unwrap();
        assert!(packet.contains("starter_sprint"));
        assert!(packet.contains("referrals"));
        assert!(packet.contains("futureA2AOnly"));
        assert!(packet.contains("assert_fake_persuasion_rejected"));
        assert!(packet.contains("fake_scarcity"));
        assert!(!packet.contains("fake review"));
    }
}
