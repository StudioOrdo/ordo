pub mod core;
pub mod types;

pub use core::*;
pub use types::*;

pub const LLM_INVOKE_CAPABILITY_ID: &str = "llm.invoke";
pub const LLM_CANCEL_CAPABILITY_ID: &str = "llm.cancel";
pub const LLM_TOOL_REQUEST_CAPABILITY_ID: &str = "llm.tool.request";
pub const LLM_TOOL_APPROVE_CAPABILITY_ID: &str = "llm.tool.approve";
pub const LLM_TOOL_REJECT_CAPABILITY_ID: &str = "llm.tool.reject";
pub const LLM_TOOL_EXECUTE_CAPABILITY_ID: &str = "llm.tool.execute";
pub const LLM_REPLAY_FIXTURE_SCHEMA_VERSION: &str = "ordo.llm_replay_fixture.v1";

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capabilities::seed_builtin_capabilities;
    use crate::conversation_protocol::{command_rejected_error, ConversationGatewayDurability};
    use crate::conversations::{
        create_conversation_participant, find_or_create_canonical_conversation,
        CanonicalConversationRequest, ConversationParticipantCreateRequest,
    };
    use crate::policy::ActorContext;
    use crate::schema::init_schema;
    use anyhow::Result;
    use rusqlite::Connection;
    use serde_json::{json, Value};
    use std::cell::{Cell, RefCell};
    use std::path::{Path, PathBuf};
    use std::rc::Rc;

    fn test_connection() -> Connection {
        let connection = Connection::open_in_memory().unwrap();
        init_schema(&connection).unwrap();
        seed_builtin_capabilities(&connection).unwrap();
        connection
            .execute(
                "INSERT INTO actors (id, actor_kind, display_name, status, metadata_json, created_at, updated_at)
                 VALUES ('actor_staff', 'staff', 'Staff', 'active', '{}', 'now', 'now')",
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

    fn test_db_path() -> (tempfile::TempDir, PathBuf) {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let db_path = temp_dir.path().join("local.db");
        (temp_dir, db_path)
    }

    fn conversation_and_assistant(connection: &Connection) -> (String, String) {
        let conversation = find_or_create_canonical_conversation(
            connection,
            &CanonicalConversationRequest {
                surface: "client_portal".to_string(),
                subject_kind: "connection".to_string(),
                subject_id: "connection_1".to_string(),
                connection_id: Some("connection_1".to_string()),
                visitor_session_id: None,
                created_by_actor_id: Some("actor_staff".to_string()),
            },
        )
        .unwrap();
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
        )
        .unwrap();
        (conversation.id, assistant.id)
    }

    fn prompt_slots() -> Vec<PromptSlot> {
        vec![
            PromptSlot::new(
                "ethical_business_persuasion",
                "Ethical Business Persuasion",
                "Use verified evidence only; preserve client agency.",
                vec!["docs/architecture/conversation-realtime/product-doctrine.md".to_string()],
                "Business communication lens required by product doctrine.",
                "staff_private",
            )
            .unwrap(),
            PromptSlot::new(
                "conversation_brief",
                "Conversation Brief",
                "Client asked about next steps.",
                vec!["conversation_event_1".to_string()],
                "Current conversation evidence.",
                "participants",
            )
            .unwrap(),
        ]
    }

    fn llm_request(conversation_id: &str, assistant_id: &str) -> LlmGatewayRequest {
        LlmGatewayRequest {
            run_id: "llm_run_1".to_string(),
            conversation_id: conversation_id.to_string(),
            segment_id: None,
            assistant_participant_id: assistant_id.to_string(),
            client_id: Some("client_llm_1".to_string()),
            provider_id: "local_fake".to_string(),
            model_id: "fake-chat".to_string(),
            user_message: "What should we say next?".to_string(),
            prompt_slots: prompt_slots(),
        }
    }

    fn replay_file_request(conversation_id: &str, assistant_id: &str) -> LlmGatewayRequest {
        LlmGatewayRequest {
            run_id: "llm_run_replay_fixture".to_string(),
            conversation_id: conversation_id.to_string(),
            segment_id: None,
            assistant_participant_id: assistant_id.to_string(),
            client_id: Some("client_llm_replay_1".to_string()),
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
            )
            .unwrap()],
        }
    }

    fn replay_fixture_for_request(request: &LlmGatewayRequest) -> ReplayLlmFixture {
        let prompt = compile_prompt(&request.prompt_slots).unwrap();
        ReplayLlmFixture {
            schema_version: LLM_REPLAY_FIXTURE_SCHEMA_VERSION.to_string(),
            fixture_id: "replay_success_fixture".to_string(),
            provider_id: request.provider_id.clone(),
            model_id: request.model_id.clone(),
            request_fingerprint: replay_request_fingerprint(&LlmProviderRequest {
                run_id: request.run_id.clone(),
                provider_id: request.provider_id.clone(),
                model_id: request.model_id.clone(),
                prompt: prompt.clone(),
                user_message: request.user_message.clone(),
            }),
            prompt_hash: prompt.prompt_hash,
            expected_prompt_slot_ids: request
                .prompt_slots
                .iter()
                .map(|slot| slot.id.clone())
                .collect(),
            events: vec![
                ReplayLlmFixtureEvent::TextDelta {
                    delta: "Replay ".to_string(),
                },
                ReplayLlmFixtureEvent::TextDelta {
                    delta: "answer".to_string(),
                },
                ReplayLlmFixtureEvent::Completed {
                    text: "Replay answer".to_string(),
                    usage: LlmUsageMetadata {
                        input_tokens: 21,
                        output_tokens: 2,
                    },
                },
            ],
            redaction_summary: ReplayFixtureRedactionSummary {
                redacted_value_count: 0,
                detectors: vec![
                    "email".to_string(),
                    "phone".to_string(),
                    "secret".to_string(),
                ],
            },
            provenance_refs: vec!["eval_artifact_packet:replay_success_fixture".to_string()],
            created_at: "2026-05-09T00:00:00Z".to_string(),
            updated_at: "2026-05-09T00:00:00Z".to_string(),
        }
    }

    type MockOpenAiCallLog = Rc<RefCell<Vec<(String, String, u64, Value)>>>;

    #[derive(Clone)]
    struct MockOpenAiTransport {
        response: OpenAiTransportResponse,
        seen: MockOpenAiCallLog,
    }

    impl MockOpenAiTransport {
        fn success(text: &str) -> Self {
            Self {
                response: OpenAiTransportResponse {
                    status: 200,
                    body: json!({
                        "choices": [
                            { "message": { "content": text } }
                        ],
                        "usage": {
                            "prompt_tokens": 17,
                            "completion_tokens": text.split_whitespace().count() as i64,
                        }
                    }),
                },
                seen: Rc::new(RefCell::new(Vec::new())),
            }
        }

        fn error(status: u16, code: &str, message: &str) -> Self {
            Self {
                response: OpenAiTransportResponse {
                    status,
                    body: json!({
                        "error": {
                            "code": code,
                            "message": message,
                        }
                    }),
                },
                seen: Rc::new(RefCell::new(Vec::new())),
            }
        }
    }

    impl OpenAiCompatibleTransport for MockOpenAiTransport {
        fn post_chat_completions(
            &self,
            endpoint: &str,
            api_key: &str,
            timeout_ms: u64,
            body: &Value,
        ) -> Result<OpenAiTransportResponse> {
            self.seen.borrow_mut().push((
                endpoint.to_string(),
                api_key.to_string(),
                timeout_ms,
                body.clone(),
            ));
            Ok(self.response.clone())
        }
    }

    fn tool_request(conversation_id: &str, capability_id: &str) -> LlmToolRequestCreateRequest {
        LlmToolRequestCreateRequest {
            run_id: "llm_run_1".to_string(),
            conversation_id: conversation_id.to_string(),
            requested_capability_id: capability_id.to_string(),
            requested_by: "llm_run_1".to_string(),
            reason: "Need governed local evidence.".to_string(),
            evidence_refs: vec!["conversation_event_1".to_string()],
            input_summary: "Read current daemon status.".to_string(),
            visibility_ceiling: "staff_private".to_string(),
            client_id: Some("client_tool_1".to_string()),
        }
    }

    #[test]
    fn openai_compatible_request_uses_transformed_payload_and_redacts_config_debug() {
        let connection = test_connection();
        let (_temp_dir, db_path) = test_db_path();
        let (conversation_id, assistant_id) = conversation_and_assistant(&connection);
        let mut request = llm_request(&conversation_id, &assistant_id);
        request.provider_id = "openai".to_string();
        request.model_id = "gpt-test".to_string();
        request.user_message =
            "Draft for Project Orchid using ada@example.com and sk-test-secret-value.".to_string();
        request.prompt_slots = vec![PromptSlot::new(
            "conversation_brief",
            "Conversation Brief",
            "Project Orchid client asked for a private next step.",
            vec!["conversation_event_1".to_string()],
            "Current conversation evidence.",
            "participants",
        )
        .unwrap()];
        let config = OpenAiCompatibleConfig::new(
            "openai",
            "gpt-test",
            "https://api.openai.test/v1",
            "sk-live-secret",
        )
        .unwrap()
        .with_timeout_ms(12_345)
        .unwrap();
        assert!(!format!("{config:?}").contains("sk-live-secret"));
        let transport = MockOpenAiTransport::success("Mocked provider answer");
        let seen = transport.seen.clone();
        let gateway = LlmGateway::new(OpenAiCompatibleProvider::with_transport(config, transport))
            .with_private_terms(vec!["Project Orchid".to_string()]);

        let result = gateway
            .run_completion(
                &db_path,
                &connection,
                &ActorContext::local_owner("test"),
                request,
            )
            .unwrap();

        assert_eq!(
            result
                .final_message
                .as_ref()
                .map(|message| message.body_markdown.as_str()),
            Some("Mocked provider answer")
        );
        let seen = seen.borrow();
        assert_eq!(seen.len(), 1);
        assert_eq!(seen[0].0, "https://api.openai.test/v1/chat/completions");
        assert_eq!(seen[0].1, "sk-live-secret");
        assert_eq!(seen[0].2, 12_345);
        let request_json = serde_json::to_string(&seen[0].3).unwrap();
        assert!(request_json.contains("\"model\":\"gpt-test\""));
        assert!(request_json.contains("__ORDO_PRIVATE_EMAIL_"));
        assert!(request_json.contains("__ORDO_PRIVATE_API_KEY_"));
        assert!(!request_json.contains("Project Orchid"));
        assert!(!request_json.contains("ada@example.com"));
        assert!(!request_json.contains("sk-test-secret-value"));
    }

    #[test]
    fn openai_compatible_response_normalization_handles_success_error_and_bad_shape() {
        let success = normalize_openai_response(OpenAiTransportResponse {
            status: 200,
            body: json!({
                "choices": [{ "message": { "content": "Provider answer" } }],
                "usage": { "prompt_tokens": 11, "completion_tokens": 2 }
            }),
        });
        assert!(matches!(
            success.first(),
            Some(LlmProviderStreamEvent::TextDelta(delta)) if delta == "Provider answer"
        ));
        assert_eq!(
            success.last(),
            Some(&LlmProviderStreamEvent::Completed {
                text: "Provider answer".to_string(),
                usage: LlmUsageMetadata {
                    input_tokens: 11,
                    output_tokens: 2,
                },
            })
        );

        let failed = normalize_openai_response(OpenAiTransportResponse {
            status: 401,
            body: json!({
                "error": {
                    "code": "invalid_api_key",
                    "message": "Invalid API key sk-test-secret-value"
                }
            }),
        });
        assert!(matches!(
            failed.first(),
            Some(LlmProviderStreamEvent::Failed { code, message })
                if code == "invalid_api_key"
                    && message.contains("Provider error redacted")
                    && !message.contains("sk-test-secret-value")
        ));

        let bad_shape = normalize_openai_response(OpenAiTransportResponse {
            status: 200,
            body: json!({ "choices": [] }),
        });
        assert!(matches!(
            bad_shape.first(),
            Some(LlmProviderStreamEvent::Failed { code, .. })
                if code == "unsupported_provider_response"
        ));
    }

    #[test]
    fn openai_compatible_config_fails_closed_without_key() {
        let error =
            OpenAiCompatibleConfig::new("openai", "gpt-test", "https://api.openai.test/v1", "")
                .unwrap_err();
        assert!(error.to_string().contains("api_key is required"));
        assert!(!error.to_string().contains("sk-"));
    }

    #[test]
    fn openai_compatible_gateway_records_usage_and_avoids_sensitive_persistence() {
        let connection = test_connection();
        let (_temp_dir, db_path) = test_db_path();
        let (conversation_id, assistant_id) = conversation_and_assistant(&connection);
        let mut request = llm_request(&conversation_id, &assistant_id);
        request.run_id = "llm_run_openai_mock".to_string();
        request.provider_id = "openai".to_string();
        request.model_id = "gpt-test".to_string();
        request.user_message =
            "Draft for Project Orchid using ada@example.com and sk-test-secret-value.".to_string();
        request.prompt_slots = vec![PromptSlot::new(
            "conversation_brief",
            "Conversation Brief",
            "Project Orchid needs a concise next step.",
            vec!["conversation_event_1".to_string()],
            "Current conversation evidence.",
            "participants",
        )
        .unwrap()];
        let config = OpenAiCompatibleConfig::new(
            "openai",
            "gpt-test",
            "https://api.openai.test/v1",
            "sk-live-secret",
        )
        .unwrap();
        let gateway = LlmGateway::new(OpenAiCompatibleProvider::with_transport(
            config,
            MockOpenAiTransport::success("Safe mocked answer"),
        ))
        .with_private_terms(vec!["Project Orchid".to_string()]);

        let result = gateway
            .run_completion(
                &db_path,
                &connection,
                &ActorContext::local_owner("test"),
                request,
            )
            .unwrap();

        assert_eq!(
            result
                .final_message
                .as_ref()
                .map(|message| message.body_markdown.as_str()),
            Some("Safe mocked answer")
        );
        assert!(result
            .frames
            .iter()
            .any(|frame| frame.frame_type == "llm.usage.recorded"));
        let usage_kinds = connection
            .prepare(
                "SELECT usage_kind FROM llm_token_ledger_entries
                 WHERE invocation_id = 'llm_run_openai_mock'
                 ORDER BY usage_kind",
            )
            .unwrap()
            .query_map([], |row| row.get::<_, String>(0))
            .unwrap()
            .collect::<rusqlite::Result<Vec<_>>>()
            .unwrap();
        assert!(usage_kinds.contains(&"provider_input".to_string()));
        assert!(usage_kinds.contains(&"provider_output".to_string()));
        let sensitive_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM conversation_events
                 WHERE payload_json LIKE '%Project Orchid%'
                    OR payload_json LIKE '%ada@example.com%'
                    OR payload_json LIKE '%sk-test-secret-value%'
                    OR payload_json LIKE '%sk-live-secret%'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(sensitive_count, 0);
    }

    #[test]
    fn openai_compatible_provider_error_does_not_create_final_message() {
        let connection = test_connection();
        let (_temp_dir, db_path) = test_db_path();
        let (conversation_id, assistant_id) = conversation_and_assistant(&connection);
        let mut request = llm_request(&conversation_id, &assistant_id);
        request.provider_id = "openai".to_string();
        request.model_id = "gpt-test".to_string();
        let config = OpenAiCompatibleConfig::new(
            "openai",
            "gpt-test",
            "https://api.openai.test/v1",
            "sk-live-secret",
        )
        .unwrap();
        let gateway = LlmGateway::new(OpenAiCompatibleProvider::with_transport(
            config,
            MockOpenAiTransport::error(404, "model_not_found", "No such model"),
        ));

        let result = gateway
            .run_completion(
                &db_path,
                &connection,
                &ActorContext::local_owner("test"),
                request,
            )
            .unwrap();

        assert!(result.final_message.is_none());
        assert!(result.frames.iter().any(|frame| {
            frame.frame_type == "llm.run.failed" && frame.payload["code"] == "model_not_found"
        }));
    }

    #[test]
    fn provider_stream_normalizes_ephemeral_deltas_and_durable_completion() {
        let connection = test_connection();
        let (_temp_dir, db_path) = test_db_path();
        let (conversation_id, assistant_id) = conversation_and_assistant(&connection);
        let gateway = LlmGateway::new(DeterministicLlmProvider::new("local_fake", "fake-chat"));

        let result = gateway
            .run_completion(
                &db_path,
                &connection,
                &ActorContext::local_owner("test"),
                llm_request(&conversation_id, &assistant_id),
            )
            .unwrap();

        assert!(result
            .frames
            .iter()
            .any(|frame| frame.frame_type == "llm.text.delta"
                && frame.durability == ConversationGatewayDurability::Ephemeral
                && frame.cursor.is_none()));
        assert!(result
            .frames
            .iter()
            .any(|frame| frame.frame_type == "llm.text.completed"
                && frame.durability == ConversationGatewayDurability::Durable
                && frame.cursor.is_some()));
        assert_eq!(
            result.final_message.as_ref().unwrap().body_markdown,
            "Drafting answer"
        );

        let persisted_deltas: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM conversation_events WHERE event_type = 'llm.text.delta'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let completed_events: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM conversation_events WHERE event_type = 'llm.text.completed'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(persisted_deltas, 0);
        assert_eq!(completed_events, 1);
    }

    #[test]
    fn prompt_slots_record_evidence_metadata_and_hashes() {
        let connection = test_connection();
        let (_temp_dir, db_path) = test_db_path();
        let (conversation_id, assistant_id) = conversation_and_assistant(&connection);
        let gateway = LlmGateway::new(DeterministicLlmProvider::new("local_fake", "fake-chat"));

        let result = gateway
            .run_completion(
                &db_path,
                &connection,
                &ActorContext::local_owner("test"),
                llm_request(&conversation_id, &assistant_id),
            )
            .unwrap();

        let prompt = result.prompt.unwrap();
        assert_eq!(prompt.slots.len(), 2);
        assert!(prompt.prompt_hash.starts_with("sha256:"));
        assert!(prompt
            .slots
            .iter()
            .all(|slot| slot.content_hash.starts_with("sha256:")
                && !slot.source_refs.is_empty()
                && !slot.inclusion_reason.is_empty()
                && !slot.visibility_ceiling.is_empty()));
        let slot_events: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM conversation_events WHERE event_type = 'llm.prompt.slot.included'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(slot_events, 2);
    }

    #[test]
    fn cancellation_records_canonical_cancel_state() {
        let connection = test_connection();
        let (conversation_id, _) = conversation_and_assistant(&connection);
        let gateway = LlmGateway::new(DeterministicLlmProvider::new("local_fake", "fake-chat"));

        let result = gateway
            .cancel_run(
                &connection,
                &ActorContext::local_owner("test"),
                &conversation_id,
                "llm_run_cancel",
                Some("client_cancel_1"),
            )
            .unwrap();

        assert_eq!(result.frames[0].frame_type, "llm.run.cancelled");
        let cancelled_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM conversation_events WHERE event_type = 'llm.run.cancelled'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(cancelled_count, 1);
    }

    #[test]
    fn provider_failure_records_failed_state_without_final_message() {
        let connection = test_connection();
        let (_temp_dir, db_path) = test_db_path();
        let (conversation_id, assistant_id) = conversation_and_assistant(&connection);
        let gateway = LlmGateway::new(DeterministicLlmProvider::failing(
            "local_fake",
            "fake-chat",
            "provider_unavailable",
            "provider offline",
        ));

        let result = gateway
            .run_completion(
                &db_path,
                &connection,
                &ActorContext::local_owner("test"),
                llm_request(&conversation_id, &assistant_id),
            )
            .unwrap();

        assert!(result.final_message.is_none());
        assert!(result
            .frames
            .iter()
            .any(|frame| frame.frame_type == "llm.run.failed"));
    }

    #[test]
    fn replay_provider_fixture_runs_through_gateway_accounting_and_events() {
        let connection = test_connection();
        let (_temp_dir, db_path) = test_db_path();
        let (conversation_id, assistant_id) = conversation_and_assistant(&connection);
        let request = replay_file_request(&conversation_id, &assistant_id);
        let provider = ReplayLlmProvider::new(vec![replay_fixture_for_request(&request)]).unwrap();
        let gateway = LlmGateway::new(provider);

        let result = gateway
            .run_completion(
                &db_path,
                &connection,
                &ActorContext::local_owner("test"),
                request,
            )
            .unwrap();

        assert_eq!(
            result
                .final_message
                .as_ref()
                .map(|message| message.body_markdown.as_str()),
            Some("Replay answer")
        );
        assert!(result.frames.iter().any(
            |frame| frame.frame_type == "llm.text.delta" && frame.payload["delta"] == "Replay "
        ));
        assert!(result
            .frames
            .iter()
            .any(|frame| frame.frame_type == "llm.usage.recorded"));

        let usage_kinds = connection
            .prepare(
                "SELECT usage_kind FROM llm_token_ledger_entries
                 WHERE invocation_id = 'llm_run_replay_fixture'
                 ORDER BY usage_kind",
            )
            .unwrap()
            .query_map([], |row| row.get::<_, String>(0))
            .unwrap()
            .collect::<rusqlite::Result<Vec<_>>>()
            .unwrap();
        assert!(usage_kinds.contains(&"provider_input".to_string()));
        assert!(usage_kinds.contains(&"provider_output".to_string()));
    }

    #[test]
    fn replay_provider_loads_committed_fixture_and_matches_stable_fingerprint() {
        let connection = test_connection();
        let (_temp_dir, db_path) = test_db_path();
        let (conversation_id, assistant_id) = conversation_and_assistant(&connection);
        let request = replay_file_request(&conversation_id, &assistant_id);
        let fixture_path =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/llm-replay/tiny-success.json");
        let gateway = LlmGateway::new(ReplayLlmProvider::from_fixture_file(&fixture_path).unwrap());

        let result = gateway
            .run_completion(
                &db_path,
                &connection,
                &ActorContext::local_owner("test"),
                request,
            )
            .unwrap();

        assert_eq!(
            result
                .final_message
                .as_ref()
                .map(|message| message.body_markdown.as_str()),
            Some("Replay fixture answer")
        );
    }

    #[test]
    fn replay_provider_missing_fixture_records_canonical_failure_without_network() {
        let connection = test_connection();
        let (_temp_dir, db_path) = test_db_path();
        let (conversation_id, assistant_id) = conversation_and_assistant(&connection);
        let mut request = replay_file_request(&conversation_id, &assistant_id);
        let fixture = replay_fixture_for_request(&request);
        request.user_message = "A different replay request.".to_string();
        let gateway = LlmGateway::new(ReplayLlmProvider::new(vec![fixture]).unwrap());

        let result = gateway
            .run_completion(
                &db_path,
                &connection,
                &ActorContext::local_owner("test"),
                request,
            )
            .unwrap();

        assert!(result.final_message.is_none());
        assert!(result.frames.iter().any(|frame| {
            frame.frame_type == "llm.run.failed"
                && frame.payload["code"] == "replay_fixture_not_found"
        }));
        let failure_code: String = connection
            .query_row(
                "SELECT failure_code FROM llm_invocations WHERE id = 'llm_run_replay_fixture'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(failure_code, "replay_fixture_not_found");
    }

    #[test]
    fn replay_fixture_validation_rejects_secret_shaped_content() {
        let request = replay_file_request("conversation_1", "participant_assistant");
        let mut fixture = replay_fixture_for_request(&request);
        fixture.events = vec![ReplayLlmFixtureEvent::Completed {
            text: "Email ada@example.com with sk-test-secret-value".to_string(),
            usage: LlmUsageMetadata {
                input_tokens: 1,
                output_tokens: 1,
            },
        }];

        let error = ReplayLlmProvider::new(vec![fixture]).unwrap_err();
        assert!(error
            .to_string()
            .contains("replay fixture contains raw sensitive values"));
    }

    struct CountingProvider {
        called: Cell<bool>,
    }

    impl LlmProviderAdapter for CountingProvider {
        fn provider_id(&self) -> &str {
            "local_fake"
        }

        fn model_id(&self) -> &str {
            "fake-chat"
        }

        fn stream(&self, _request: &LlmProviderRequest) -> Result<Vec<LlmProviderStreamEvent>> {
            self.called.set(true);
            Ok(vec![])
        }

        fn cancel(&self, _run_id: &str) -> Result<()> {
            self.called.set(true);
            Ok(())
        }
    }

    struct RecordingProvider {
        seen_request: RefCell<Option<LlmProviderRequest>>,
        echo_user_message: bool,
    }

    impl LlmProviderAdapter for RecordingProvider {
        fn provider_id(&self) -> &str {
            "local_fake"
        }

        fn model_id(&self) -> &str {
            "fake-chat"
        }

        fn stream(&self, request: &LlmProviderRequest) -> Result<Vec<LlmProviderStreamEvent>> {
            self.seen_request.replace(Some(request.clone()));
            Ok(vec![LlmProviderStreamEvent::Completed {
                text: if self.echo_user_message {
                    request.user_message.clone()
                } else {
                    "ok".to_string()
                },
                usage: LlmUsageMetadata {
                    input_tokens: 1,
                    output_tokens: 1,
                },
            }])
        }

        fn cancel(&self, _run_id: &str) -> Result<()> {
            Ok(())
        }
    }

    #[test]
    fn policy_denial_records_evidence_and_does_not_invoke_provider() {
        let connection = test_connection();
        let (_temp_dir, db_path) = test_db_path();
        let (conversation_id, assistant_id) = conversation_and_assistant(&connection);
        let provider = CountingProvider {
            called: Cell::new(false),
        };
        let gateway = LlmGateway::with_policies(
            provider,
            LlmPolicy::deny("LLM invocation denied by test policy."),
            LlmPolicy::allow("cancel allowed"),
        );

        let result = gateway
            .run_completion(
                &db_path,
                &connection,
                &ActorContext::local_owner("test"),
                llm_request(&conversation_id, &assistant_id),
            )
            .unwrap();

        assert_eq!(result.frames[0].frame_type, "command.rejected");
        assert!(result.final_message.is_none());
        assert!(!gateway.provider.called.get());
        let policy_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM policy_decisions WHERE capability_id = 'llm.invoke' AND outcome = 'denied'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(policy_count, 1);
    }

    #[test]
    fn llm_capabilities_are_required_and_provider_keys_never_enter_events() {
        let connection = test_connection();
        let (_temp_dir, db_path) = test_db_path();
        let (conversation_id, assistant_id) = conversation_and_assistant(&connection);
        let gateway = LlmGateway::new(DeterministicLlmProvider::new("local_fake", "fake-chat"));

        gateway
            .run_completion(
                &db_path,
                &connection,
                &ActorContext::local_owner("test"),
                llm_request(&conversation_id, &assistant_id),
            )
            .unwrap();

        let capability_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM capabilities WHERE id IN ('llm.invoke', 'llm.cancel')",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let leaked_secret_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM conversation_events WHERE payload_json LIKE '%sk-test%'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(capability_count, 2);
        assert_eq!(leaked_secret_count, 0);
    }

    #[test]
    fn privacy_firewall_transforms_provider_bound_payloads_and_reconstructs_locally() {
        let connection = test_connection();
        let (_temp_dir, db_path) = test_db_path();
        let (conversation_id, assistant_id) = conversation_and_assistant(&connection);
        let provider = RecordingProvider {
            seen_request: RefCell::new(None),
            echo_user_message: true,
        };
        let gateway =
            LlmGateway::new(provider).with_private_terms(vec!["Project Orchid".to_string()]);
        let mut request = llm_request(&conversation_id, &assistant_id);
        request.user_message =
            "Email ada@example.com or call +1-212-555-0101 with key sk-test-123456 and Bearer tok_abcdef123456 about Project Orchid.".to_string();
        request.prompt_slots.push(
            PromptSlot::new(
                "private_fixture",
                "Private Fixture",
                "Provider must not see ada@example.com or Project Orchid.",
                vec!["fixture".to_string()],
                "Privacy regression fixture.",
                "staff_private",
            )
            .unwrap(),
        );

        let result = gateway
            .run_completion(
                &db_path,
                &connection,
                &ActorContext::local_owner("test"),
                request,
            )
            .unwrap();

        let seen = gateway.provider.seen_request.borrow();
        let provider_request = seen.as_ref().unwrap();
        let provider_payload = serde_json::to_string(provider_request).unwrap();
        for raw in [
            "ada@example.com",
            "+1-212-555-0101",
            "sk-test-123456",
            "tok_abcdef123456",
            "Project Orchid",
        ] {
            assert!(
                !provider_payload.contains(raw),
                "provider payload leaked {raw}"
            );
        }
        assert!(provider_payload.contains("__ORDO_PRIVATE_EMAIL_"));
        assert!(provider_payload.contains("__ORDO_PRIVATE_API_KEY_"));
        assert!(result
            .final_message
            .as_ref()
            .unwrap()
            .body_markdown
            .contains("ada@example.com"));

        let raw_event_leaks: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM conversation_events
                 WHERE payload_json LIKE '%ada@example.com%'
                    OR payload_json LIKE '%sk-test-123456%'
                    OR payload_json LIKE '%Project Orchid%'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let raw_realtime_leaks: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM realtime_events
                 WHERE payload_json LIKE '%ada@example.com%'
                    OR payload_json LIKE '%sk-test-123456%'
                    OR payload_json LIKE '%Project Orchid%'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let raw_policy_leaks: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM policy_decisions
                 WHERE metadata_json LIKE '%ada@example.com%'
                    OR metadata_json LIKE '%sk-test-123456%'
                    OR metadata_json LIKE '%Project Orchid%'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(raw_event_leaks, 0);
        assert_eq!(raw_realtime_leaks, 0);
        assert_eq!(raw_policy_leaks, 0);
    }

    #[test]
    fn privacy_transform_events_are_durable_and_metadata_only() {
        let connection = test_connection();
        let (_temp_dir, db_path) = test_db_path();
        let (conversation_id, assistant_id) = conversation_and_assistant(&connection);
        let gateway = LlmGateway::new(DeterministicLlmProvider::new("local_fake", "fake-chat"));
        let mut request = llm_request(&conversation_id, &assistant_id);
        request.user_message = "Use sk-test-secret-value for ada@example.com".to_string();

        gateway
            .run_completion(
                &db_path,
                &connection,
                &ActorContext::local_owner("test"),
                request,
            )
            .unwrap();

        let transform_payload: String = connection
            .query_row(
                "SELECT payload_json FROM conversation_events
                 WHERE event_type = 'privacy.egress.transformed'
                   AND payload_json LIKE '%api_key%'
                 ORDER BY sequence ASC
                 LIMIT 1",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(transform_payload.contains("sourcePayloadHash"));
        assert!(transform_payload.contains("__ORDO_PRIVATE_API_KEY_"));
        assert!(!transform_payload.contains("sk-test-secret-value"));
        assert!(!transform_payload.contains("ada@example.com"));

        let sequence_order = connection
            .prepare(
                "SELECT event_type FROM conversation_events
                 WHERE event_type IN ('privacy.egress.transformed', 'llm.provider.started')
                 ORDER BY sequence ASC",
            )
            .unwrap()
            .query_map([], |row| row.get::<_, String>(0))
            .unwrap()
            .collect::<rusqlite::Result<Vec<_>>>()
            .unwrap();
        let first_provider_index = sequence_order
            .iter()
            .position(|event| event == "llm.provider.started")
            .unwrap();
        assert!(sequence_order[..first_provider_index]
            .iter()
            .any(|event| event == "privacy.egress.transformed"));
    }

    #[test]
    fn untransformable_provider_payload_blocks_provider_invocation() {
        let connection = test_connection();
        let (_temp_dir, db_path) = test_db_path();
        let (conversation_id, assistant_id) = conversation_and_assistant(&connection);
        let provider = CountingProvider {
            called: Cell::new(false),
        };
        let gateway = LlmGateway::new(provider);
        let mut request = llm_request(&conversation_id, &assistant_id);
        request.user_message = "Already has __ORDO_PRIVATE_EMAIL_1__".to_string();

        let result = gateway
            .run_completion(
                &db_path,
                &connection,
                &ActorContext::local_owner("test"),
                request,
            )
            .unwrap();

        assert!(!gateway.provider.called.get());
        assert!(result.final_message.is_none());
        assert!(result
            .frames
            .iter()
            .any(|frame| frame.frame_type == "privacy.egress.blocked"));
        assert!(result
            .frames
            .iter()
            .any(|frame| frame.payload["code"] == "privacy_transform_failed"));
    }

    #[test]
    fn llm_command_types_map_to_gateway_capabilities() {
        assert_eq!(
            crate::conversation_protocol::ConversationCommandType::LlmRunRequest
                .required_capability_id(),
            LLM_INVOKE_CAPABILITY_ID
        );
        assert_eq!(
            crate::conversation_protocol::ConversationCommandType::LlmRunCancel
                .required_capability_id(),
            LLM_CANCEL_CAPABILITY_ID
        );
    }

    #[test]
    fn unsupported_external_behavior_uses_structured_rejection_shape() {
        let frame = command_rejected_error(
            Some("client_llm_tool_1"),
            Some("conversation_1"),
            "unsupported_command",
            "Provider tool execution is not implemented in this slice.",
            false,
            "2026-05-09T00:00:00Z",
        );
        assert_eq!(frame.frame_type, "command.rejected");
        assert_eq!(frame.payload["code"], "unsupported_command");
    }

    #[test]
    fn tool_request_records_evidence_and_required_capability() {
        let connection = test_connection();
        let (conversation_id, _) = conversation_and_assistant(&connection);
        let gateway = LlmGateway::new(DeterministicLlmProvider::new("local_fake", "fake-chat"));

        let receipt = gateway
            .request_tool(
                &connection,
                &ActorContext::local_owner("test"),
                tool_request(&conversation_id, "system.status.read"),
            )
            .unwrap();

        let request = receipt.tool_request.unwrap();
        assert_eq!(request.status, LlmToolRequestStatus::Requested);
        assert_eq!(request.requested_capability_id, "system.status.read");
        assert_eq!(request.evidence_refs, vec!["conversation_event_1"]);
        assert_eq!(receipt.frames[0].frame_type, "llm.tool.requested");
    }

    #[test]
    fn tool_approval_and_rejection_record_policy_and_durable_events() {
        let connection = test_connection();
        let (conversation_id, _) = conversation_and_assistant(&connection);
        let gateway = LlmGateway::new(DeterministicLlmProvider::new("local_fake", "fake-chat"));
        let approved_request = gateway
            .request_tool(
                &connection,
                &ActorContext::local_owner("test"),
                tool_request(&conversation_id, "system.status.read"),
            )
            .unwrap()
            .tool_request
            .unwrap();

        let approved = gateway
            .approve_tool_request(
                &connection,
                &ActorContext::local_owner("test"),
                &conversation_id,
                &approved_request.tool_request_id,
                "Owner approved read-only evidence retrieval.",
            )
            .unwrap();

        let rejected_request = gateway
            .request_tool(
                &connection,
                &ActorContext::local_owner("test"),
                tool_request(&conversation_id, "system.status.read"),
            )
            .unwrap()
            .tool_request
            .unwrap();
        let rejected = gateway
            .reject_tool_request(
                &connection,
                &ActorContext::local_owner("test"),
                &conversation_id,
                &rejected_request.tool_request_id,
                "Evidence is not needed.",
            )
            .unwrap();

        assert_eq!(
            approved.tool_request.unwrap().status,
            LlmToolRequestStatus::Approved
        );
        assert_eq!(
            rejected.tool_request.unwrap().status,
            LlmToolRequestStatus::Rejected
        );
        let policy_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM policy_decisions WHERE capability_id IN ('llm.tool.approve', 'llm.tool.reject')",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let event_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM conversation_events WHERE event_type IN ('llm.tool.approved', 'llm.tool.rejected')",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(policy_count, 2);
        assert_eq!(event_count, 2);
    }

    #[test]
    fn tool_execution_requires_approval_and_registered_exported_capability() {
        let connection = test_connection();
        let (conversation_id, _) = conversation_and_assistant(&connection);
        let gateway = LlmGateway::new(DeterministicLlmProvider::new("local_fake", "fake-chat"));
        let pending = gateway
            .request_tool(
                &connection,
                &ActorContext::local_owner("test"),
                tool_request(&conversation_id, "system.status.read"),
            )
            .unwrap()
            .tool_request
            .unwrap();

        let blocked = gateway
            .execute_approved_tool_request(
                &connection,
                &ActorContext::local_owner("test"),
                &conversation_id,
                &pending.tool_request_id,
                "ok",
            )
            .unwrap();

        assert_eq!(blocked.frames[0].frame_type, "command.rejected");
        assert_eq!(blocked.frames[0].payload["code"], "review_required");

        gateway
            .approve_tool_request(
                &connection,
                &ActorContext::local_owner("test"),
                &conversation_id,
                &pending.tool_request_id,
                "Approved.",
            )
            .unwrap();
        let completed = gateway
            .execute_approved_tool_request(
                &connection,
                &ActorContext::local_owner("test"),
                &conversation_id,
                &pending.tool_request_id,
                "Daemon is ready.",
            )
            .unwrap();

        assert_eq!(completed.frames[0].frame_type, "llm.tool.executing");
        assert_eq!(completed.frames[1].frame_type, "llm.tool.completed");
        assert_eq!(
            completed.tool_request.unwrap().status,
            LlmToolRequestStatus::Completed
        );
    }

    #[test]
    fn unknown_and_non_exported_tool_capabilities_are_rejected_before_execution() {
        let connection = test_connection();
        let (conversation_id, _) = conversation_and_assistant(&connection);
        let gateway = LlmGateway::new(DeterministicLlmProvider::new("local_fake", "fake-chat"));

        let unknown = gateway
            .request_tool(
                &connection,
                &ActorContext::local_owner("test"),
                tool_request(&conversation_id, "missing.capability"),
            )
            .unwrap();
        let non_exported = gateway
            .request_tool(
                &connection,
                &ActorContext::local_owner("test"),
                tool_request(&conversation_id, "llm.invoke"),
            )
            .unwrap();

        assert!(unknown.tool_request.is_none());
        assert_eq!(unknown.frames[0].payload["code"], "unsupported_command");
        assert!(non_exported.tool_request.is_none());
        assert_eq!(
            non_exported.frames[0].payload["code"],
            "unsupported_command"
        );
        let requested_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM conversation_events WHERE event_type = 'llm.tool.requested'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(requested_count, 0);
    }

    #[test]
    fn approved_tool_failure_records_deterministic_failed_state() {
        let connection = test_connection();
        let (conversation_id, _) = conversation_and_assistant(&connection);
        let gateway = LlmGateway::new(DeterministicLlmProvider::new("local_fake", "fake-chat"));
        let request = gateway
            .request_tool(
                &connection,
                &ActorContext::local_owner("test"),
                tool_request(&conversation_id, "system.status.read"),
            )
            .unwrap()
            .tool_request
            .unwrap();
        gateway
            .approve_tool_request(
                &connection,
                &ActorContext::local_owner("test"),
                &conversation_id,
                &request.tool_request_id,
                "Approved.",
            )
            .unwrap();

        let failed = gateway
            .fail_approved_tool_request(
                &connection,
                &ActorContext::local_owner("test"),
                &conversation_id,
                &request.tool_request_id,
                "tool_failed",
                "Deterministic failure.",
            )
            .unwrap();

        assert_eq!(failed.frames[0].frame_type, "llm.tool.failed");
        assert_eq!(
            failed.tool_request.unwrap().status,
            LlmToolRequestStatus::Failed
        );
    }

    #[test]
    fn tool_request_events_replay_in_conversation_sequence_order() {
        let connection = test_connection();
        let (conversation_id, _) = conversation_and_assistant(&connection);
        let gateway = LlmGateway::new(DeterministicLlmProvider::new("local_fake", "fake-chat"));
        let request = gateway
            .request_tool(
                &connection,
                &ActorContext::local_owner("test"),
                tool_request(&conversation_id, "system.status.read"),
            )
            .unwrap()
            .tool_request
            .unwrap();
        gateway
            .approve_tool_request(
                &connection,
                &ActorContext::local_owner("test"),
                &conversation_id,
                &request.tool_request_id,
                "Approved.",
            )
            .unwrap();
        gateway
            .execute_approved_tool_request(
                &connection,
                &ActorContext::local_owner("test"),
                &conversation_id,
                &request.tool_request_id,
                "ok",
            )
            .unwrap();

        let events = connection
            .prepare(
                "SELECT event_type FROM conversation_events
                 WHERE conversation_id = ?1 AND event_type LIKE 'llm.tool.%'
                 ORDER BY sequence ASC",
            )
            .unwrap()
            .query_map([conversation_id], |row| row.get::<_, String>(0))
            .unwrap()
            .collect::<rusqlite::Result<Vec<_>>>()
            .unwrap();
        assert_eq!(
            events,
            vec![
                "llm.tool.requested",
                "llm.tool.approved",
                "llm.tool.executing",
                "llm.tool.completed"
            ]
        );
    }

    #[test]
    fn tool_capabilities_are_registered_and_do_not_leak_provider_secrets() {
        let connection = test_connection();
        let (conversation_id, _) = conversation_and_assistant(&connection);
        let gateway = LlmGateway::new(DeterministicLlmProvider::new("local_fake", "fake-chat"));

        gateway
            .request_tool(
                &connection,
                &ActorContext::local_owner("test"),
                tool_request(&conversation_id, "system.status.read"),
            )
            .unwrap();

        let capability_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM capabilities WHERE id IN ('llm.tool.request', 'llm.tool.approve', 'llm.tool.reject', 'llm.tool.execute')",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let leaked_secret_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM conversation_events WHERE payload_json LIKE '%sk-test%'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(capability_count, 4);
        assert_eq!(leaked_secret_count, 0);
    }

    #[test]
    fn completed_llm_run_records_invocation_slots_and_ledger_entries() {
        let connection = test_connection();
        let (_temp_dir, db_path) = test_db_path();
        let (conversation_id, assistant_id) = conversation_and_assistant(&connection);
        let gateway = LlmGateway::new(DeterministicLlmProvider::new("local_fake", "fake-chat"));

        let result = gateway
            .run_completion(
                &db_path,
                &connection,
                &ActorContext::local_owner("test"),
                llm_request(&conversation_id, &assistant_id),
            )
            .unwrap();

        assert!(result
            .frames
            .iter()
            .any(|frame| frame.frame_type == "llm.prompt.slot.accounted"));
        assert!(result
            .frames
            .iter()
            .any(|frame| frame.frame_type == "llm.ledger.entry.recorded"));
        let invocation: (String, String, String, String) = connection
            .query_row(
                "SELECT status, provider_id, model_id, capability_id
                 FROM llm_invocations
                 WHERE id = 'llm_run_1'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .unwrap();
        assert_eq!(
            invocation,
            (
                "completed".to_string(),
                "local_fake".to_string(),
                "fake-chat".to_string(),
                "llm.invoke".to_string()
            )
        );
        let slot_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM llm_prompt_slot_usage WHERE invocation_id = 'llm_run_1'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let (ledger_total, usage_kinds): (i64, String) = connection
            .query_row(
                "SELECT SUM(token_count), group_concat(usage_kind, ',')
                 FROM llm_token_ledger_entries
                 WHERE invocation_id = 'llm_run_1'
                 ORDER BY usage_kind",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(slot_count, 2);
        assert!(ledger_total > 0);
        assert!(usage_kinds.contains("provider_input"));
        assert!(usage_kinds.contains("provider_output"));
    }

    #[test]
    fn provider_failure_updates_invocation_without_ledger_entries() {
        let connection = test_connection();
        let (_temp_dir, db_path) = test_db_path();
        let (conversation_id, assistant_id) = conversation_and_assistant(&connection);
        let gateway = LlmGateway::new(DeterministicLlmProvider::failing(
            "local_fake",
            "fake-chat",
            "provider_unavailable",
            "provider offline",
        ));

        gateway
            .run_completion(
                &db_path,
                &connection,
                &ActorContext::local_owner("test"),
                llm_request(&conversation_id, &assistant_id),
            )
            .unwrap();

        let (status, failure_code, failure_hash): (String, String, String) = connection
            .query_row(
                "SELECT status, failure_code, failure_message_hash
                 FROM llm_invocations
                 WHERE id = 'llm_run_1'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap();
        assert_eq!(status, "failed");
        assert_eq!(failure_code, "provider_unavailable");
        assert!(failure_hash.starts_with("sha256:"));
        let ledger_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM llm_token_ledger_entries WHERE invocation_id = 'llm_run_1'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(ledger_count, 0);
    }

    #[test]
    fn token_ledger_does_not_store_raw_sensitive_prompt_or_user_text() {
        let connection = test_connection();
        let (_temp_dir, db_path) = test_db_path();
        let (conversation_id, assistant_id) = conversation_and_assistant(&connection);
        let provider = RecordingProvider {
            seen_request: RefCell::new(None),
            echo_user_message: false,
        };
        let gateway =
            LlmGateway::new(provider).with_private_terms(vec!["Project Orchid".to_string()]);
        let mut request = llm_request(&conversation_id, &assistant_id);
        request.user_message =
            "Reach ada@example.com with Bearer tok_abcdef123456 about Project Orchid.".to_string();
        request.prompt_slots.push(
            PromptSlot::new(
                "private_fixture",
                "Private Fixture",
                "Do not leak ada@example.com or Project Orchid.",
                vec!["fixture".to_string()],
                "Privacy regression fixture.",
                "staff_private",
            )
            .unwrap(),
        );

        gateway
            .run_completion(
                &db_path,
                &connection,
                &ActorContext::local_owner("test"),
                request,
            )
            .unwrap();

        for raw in [
            "ada@example.com",
            "tok_abcdef123456",
            "Project Orchid",
            "Do not leak ada@example.com",
        ] {
            for (table, columns) in [
                (
                    "llm_invocations",
                    "metadata_json || prompt_hash || privacy_transform_run_ids_json",
                ),
                (
                    "llm_prompt_slot_usage",
                    "slot_id || source_refs_json || visibility || content_hash",
                ),
                (
                    "llm_token_ledger_entries",
                    "usage_kind || pricing_snapshot_json || metadata_json",
                ),
                ("conversation_events", "event_type || payload_json"),
                ("realtime_events", "event_type || payload_json"),
                ("policy_decisions", "reason || metadata_json"),
            ] {
                let leaked_count: i64 = connection
                    .query_row(
                        &format!("SELECT COUNT(*) FROM {table} WHERE {columns} LIKE ?1"),
                        [format!("%{raw}%")],
                        |row| row.get(0),
                    )
                    .unwrap();
                assert_eq!(leaked_count, 0, "{table} leaked {raw}");
            }
        }
    }
}
