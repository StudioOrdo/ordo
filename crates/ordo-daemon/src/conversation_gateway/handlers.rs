use anyhow::{ensure, Result};
use chrono::{Duration as ChronoDuration, Utc};
use rusqlite::{params, Connection, OptionalExtension};
use serde::Deserialize;
use serde_json::{json, Value};
use std::fs;
use std::path::Path;
use std::time::Instant;
use uuid::Uuid;

use crate::conversation_protocol::{
    ack_envelope, command_rejected_error, dispatch_envelope, ConversationGatewayDurability,
    ConversationGatewayEnvelope, ConversationGatewayOp, ConversationGatewayScope,
    CONVERSATION_GATEWAY_SCHEMA_VERSION,
};
use crate::conversations::{
    create_conversation_handoff, transition_conversation_handoff, upsert_conversation_mode,
    ConversationHandoffCreateRequest, ConversationMessageCreateRequest, ConversationMode,
    ConversationMutationActor, ConversationPresenceUpdateRequest, ConversationService,
    HandoffStatus, ReactionAction,
};
use crate::install::list_provider_configs_connection_with_env;
use crate::llm_gateway::{
    AnthropicMessagesConfig, AnthropicMessagesProvider, DeterministicLlmProvider, LlmGateway,
    LlmGatewayRequest, LlmProviderAdapter, LlmToolRequestReceipt, OllamaChatConfig,
    OllamaChatProvider, OpenAiCompatibleConfig, OpenAiCompatibleProvider,
    OpenAiCompatibleTransport, PromptSlot, ReqwestOpenAiTransport,
};
use crate::policy::{ActorContext, ActorKind};
use crate::secrets::{normalize_secret, OrdoSecretString};
use crate::vault::decrypt_secret;

const DEFAULT_REPLAY_LIMIT: usize = 100;
const MAX_REPLAY_LIMIT: usize = 500;
const COMMAND_RATE_LIMIT: usize = 30;
const COMMAND_RATE_WINDOW_SECONDS: i64 = 60;

use super::types::*;

pub(crate) fn identify(
    session: &mut ConversationGatewaySession,
    envelope: ConversationGatewayEnvelope,
) -> Result<ConversationGatewayOutput> {
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct IdentifyPayload {
        actor_id: Option<String>,
        participant_id: Option<String>,
    }
    let payload: IdentifyPayload = serde_json::from_value(envelope.payload.clone())?;
    session.actor_id = payload.actor_id;
    session.participant_id = payload.participant_id;
    Ok(single_frame(ack_envelope(
        required_client_id(&envelope)?,
        envelope.conversation_id.as_deref(),
        "identify.ack",
        json!({
            "actorId": session.actor_id,
            "participantId": session.participant_id,
        }),
        &Utc::now().to_rfc3339(),
    )))
}

pub(crate) fn subscribe(
    db_path: &Path,
    session: &mut ConversationGatewaySession,
    envelope: ConversationGatewayEnvelope,
) -> Result<ConversationGatewayOutput> {
    let conversation_id = required_conversation_id(&envelope)?.to_string();
    session.subscriptions.insert(conversation_id.clone());
    let replay = replay_conversation_events(
        db_path,
        &conversation_id,
        after_sequence(&envelope),
        limit(&envelope),
    )?;
    let mut frames = vec![ack_envelope(
        required_client_id(&envelope)?,
        Some(&conversation_id),
        "conversation.subscribe.ack",
        json!({ "conversationId": conversation_id }),
        &Utc::now().to_rfc3339(),
    )];
    frames.extend(replay);
    Ok(ConversationGatewayOutput {
        frames,
        broadcast: vec![],
    })
}

pub(crate) fn unsubscribe(
    session: &mut ConversationGatewaySession,
    envelope: ConversationGatewayEnvelope,
) -> Result<ConversationGatewayOutput> {
    let conversation_id = required_conversation_id(&envelope)?.to_string();
    session.subscriptions.remove(&conversation_id);
    Ok(single_frame(ack_envelope(
        required_client_id(&envelope)?,
        Some(&conversation_id),
        "conversation.unsubscribe.ack",
        json!({ "conversationId": conversation_id }),
        &Utc::now().to_rfc3339(),
    )))
}

pub(crate) fn replay(
    db_path: &Path,
    envelope: ConversationGatewayEnvelope,
) -> Result<ConversationGatewayOutput> {
    let conversation_id = required_conversation_id(&envelope)?;
    let replay = replay_conversation_events(
        db_path,
        conversation_id,
        after_sequence(&envelope),
        limit(&envelope),
    )?;
    Ok(ConversationGatewayOutput {
        frames: replay,
        broadcast: vec![],
    })
}

pub(crate) fn heartbeat(
    envelope: ConversationGatewayEnvelope,
) -> Result<ConversationGatewayOutput> {
    Ok(single_frame(ack_envelope(
        envelope.client_id.as_deref().unwrap_or("heartbeat"),
        envelope.conversation_id.as_deref(),
        "heartbeat.ack",
        json!({ "receivedAt": Utc::now().to_rfc3339() }),
        &Utc::now().to_rfc3339(),
    )))
}

pub(crate) fn command(
    db_path: &Path,
    session: &mut ConversationGatewaySession,
    envelope: ConversationGatewayEnvelope,
) -> Result<ConversationGatewayOutput> {
    let command_type = envelope.frame_type.as_str();
    match command_type {
        "conversation.subscribe" => subscribe(db_path, session, envelope),
        "conversation.replay_after_cursor" => replay(db_path, envelope),
        "message.submit" => {
            if let Some(output) = enforce_message_command_rate_limit(session, &envelope)? {
                return Ok(output);
            }
            message_submit(db_path, session, envelope)
        }
        "llm.run.request" => llm_run_request(db_path, session, envelope),
        "llm.run.cancel" => llm_run_cancel(db_path, session, envelope),
        "tool.approve" => llm_tool_approve(db_path, session, envelope),
        "tool.reject" => llm_tool_reject(db_path, session, envelope),
        "tool.execute" => llm_tool_execute(db_path, session, envelope),
        "message.edit" => {
            if let Some(output) = enforce_message_command_rate_limit(session, &envelope)? {
                return Ok(output);
            }
            message_edit(db_path, session, envelope)
        }
        "message.delete" => {
            if let Some(output) = enforce_message_command_rate_limit(session, &envelope)? {
                return Ok(output);
            }
            message_delete(db_path, session, envelope)
        }
        "message.undo" => {
            if let Some(output) = enforce_message_command_rate_limit(session, &envelope)? {
                return Ok(output);
            }
            message_undo(db_path, session, envelope)
        }
        "message.mark_read" => message_mark_read(db_path, session, envelope),
        "message.mark_unread" => message_mark_unread(db_path, session, envelope),
        "message.react" => message_react(db_path, session, envelope),
        "presence.update" => presence_update(db_path, session, envelope),
        "typing.start" | "typing.stop" => typing(session, envelope),
        "conversation.handoff.create" => handoff_create(db_path, session, envelope),
        "conversation.handoff.accept" | "handoff.accept" => handoff_transition(
            db_path,
            session,
            envelope,
            HandoffStatus::Accepted,
            "conversation.handoff.accept.ack",
        ),
        "conversation.handoff.decline" | "handoff.decline" => handoff_transition(
            db_path,
            session,
            envelope,
            HandoffStatus::Declined,
            "conversation.handoff.decline.ack",
        ),
        "conversation.handoff.assign" | "handoff.assign" => handoff_transition(
            db_path,
            session,
            envelope,
            HandoffStatus::Assigned,
            "conversation.handoff.assign.ack",
        ),
        "conversation.handoff.return_to_agent" | "handoff.return_to_agent" => handoff_transition(
            db_path,
            session,
            envelope,
            HandoffStatus::ReturnedToAgent,
            "conversation.handoff.return_to_agent.ack",
        ),
        "conversation.handoff.close" => handoff_transition(
            db_path,
            session,
            envelope,
            HandoffStatus::Closed,
            "conversation.handoff.close.ack",
        ),
        "conversation.mode.set" => conversation_mode_set(db_path, session, envelope),
        "conversation.mode.human_led_active" | "agent.takeover" => conversation_mode_fixed(
            db_path,
            session,
            envelope,
            ConversationMode::HumanLedActive,
            false,
            "conversation.mode.human_led_active.ack",
        ),
        "conversation.mode.return_to_agent" => conversation_mode_fixed(
            db_path,
            session,
            envelope,
            ConversationMode::ReturnedToAgent,
            false,
            "conversation.mode.return_to_agent.ack",
        ),
        "conversation.agent.delegate" | "agent.delegate" => {
            agent_delegation(db_path, session, envelope, true)
        }
        "conversation.agent.delegation_revoke" => {
            agent_delegation(db_path, session, envelope, false)
        }
        _ => Ok(error_output(command_rejected_error(
            envelope.client_id.as_deref(),
            envelope.conversation_id.as_deref(),
            "unsupported_command",
            "Command is not implemented by this gateway slice.",
            false,
            &Utc::now().to_rfc3339(),
        ))),
    }
}

pub(crate) fn message_submit(
    db_path: &Path,
    session: &ConversationGatewaySession,
    envelope: ConversationGatewayEnvelope,
) -> Result<ConversationGatewayOutput> {
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct SubmitPayload {
        participant_id: Option<String>,
        body_markdown: String,
        client_message_id: String,
        message_kind: Option<String>,
        visibility: Option<String>,
        undo_expires_at: Option<String>,
    }
    let conversation_id = required_conversation_id(&envelope)?.to_string();
    let payload: SubmitPayload = serde_json::from_value(envelope.payload.clone())?;
    let participant_id = payload
        .participant_id
        .or_else(|| session.participant_id.clone())
        .ok_or_else(|| anyhow::anyhow!("participantId is required"))?;
    let actor = mutation_actor(session, envelope.client_id.clone());
    let connection = Connection::open(db_path)?;
    let receipt = ConversationService::submit_message(
        &connection,
        &actor,
        &ConversationMessageCreateRequest {
            conversation_id: conversation_id.clone(),
            segment_id: None,
            participant_id,
            message_kind: payload.message_kind.unwrap_or_else(|| "human".to_string()),
            body_markdown: payload.body_markdown,
            visibility: payload
                .visibility
                .unwrap_or_else(|| "participants".to_string()),
            client_message_id: payload.client_message_id,
            reply_to_message_id: None,
            undo_expires_at: payload.undo_expires_at,
        },
    )?;
    command_ack_and_dispatch(
        &envelope,
        "message.submit.ack",
        &receipt.value.id,
        "message.created",
        db_path,
    )
}

pub(crate) fn llm_run_request(
    db_path: &Path,
    session: &ConversationGatewaySession,
    envelope: ConversationGatewayEnvelope,
) -> Result<ConversationGatewayOutput> {
    let env = std::env::vars().collect();
    llm_run_request_with_openai_transport(db_path, session, envelope, &env, ReqwestOpenAiTransport)
}

fn llm_run_request_with_openai_transport<T: OpenAiCompatibleTransport>(
    db_path: &Path,
    session: &ConversationGatewaySession,
    envelope: ConversationGatewayEnvelope,
    env: &std::collections::HashMap<String, String>,
    openai_transport: T,
) -> Result<ConversationGatewayOutput> {
    let request_received_instant = Instant::now();
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct LlmRunPayload {
        run_id: Option<String>,
        assistant_participant_id: String,
        provider_id: Option<String>,
        model_id: Option<String>,
        user_message: String,
        prompt_slots: Option<Vec<PromptSlot>>,
    }

    let conversation_id = required_conversation_id(&envelope)?.to_string();
    let client_id = required_client_id(&envelope)?.to_string();
    let payload: LlmRunPayload = serde_json::from_value(envelope.payload.clone())?;
    let provider_id = payload
        .provider_id
        .unwrap_or_else(|| "local_fake".to_string());
    let mut model_id = payload.model_id.unwrap_or_else(|| "fake-chat".to_string());
    if provider_id == "local" && model_id == "fake-chat" {
        model_id = default_ollama_model(env);
    }
    let app_live_enabled = env_flag_enabled(env, "ORDO_APP_LIVE_LLM");
    if !app_live_enabled
        && provider_id != "local"
        && (provider_id != "local_fake" || model_id != "fake-chat")
    {
        return Ok(error_output(command_rejected_error(
            Some(&client_id),
            Some(&conversation_id),
            "live_provider_disabled",
            "Live provider mode is disabled for member chat. Set explicit owner/developer app live guards before using a live provider.",
            false,
            &Utc::now().to_rfc3339(),
        )));
    }
    let run_id = payload
        .run_id
        .unwrap_or_else(|| format!("llm_run_{}", Uuid::new_v4()));
    let prompt_slots = match payload.prompt_slots {
        Some(slots) if !slots.is_empty() => slots,
        _ => vec![PromptSlot::new(
            "local_member_chat",
            "Local Member Chat",
            "Answer as Ordo using only the current local conversation context.",
            vec![format!("conversation:{conversation_id}")],
            "Deterministic local chat request from the member Ordo room.",
            "participants",
        )?],
    };

    let connection = Connection::open(db_path)?;
    let actor = ActorContext::new(
        ActorKind::BrowserOperator,
        "conversation_gateway",
        session.actor_id.clone(),
    );
    let request = LlmGatewayRequest {
        run_id: run_id.clone(),
        conversation_id: conversation_id.clone(),
        segment_id: envelope.segment_id.clone(),
        assistant_participant_id: payload.assistant_participant_id,
        client_id: Some(client_id.clone()),
        provider_id: provider_id.clone(),
        model_id: model_id.clone(),
        user_message: payload.user_message,
        prompt_slots,
    };
    let result = if provider_id == "local" {
        run_local_ollama_provider(db_path, &connection, &actor, request, env)?
    } else if app_live_enabled {
        run_live_provider_or_reject(
            db_path,
            &connection,
            &actor,
            request,
            env,
            openai_transport,
            &client_id,
            &conversation_id,
        )?
    } else {
        run_llm_with_provider(
            db_path,
            &connection,
            &actor,
            LlmGateway::new(DeterministicLlmProvider::new(&provider_id, &model_id)),
            request,
        )?
    };
    if result
        .run
        .frames
        .iter()
        .any(|frame| frame.op == ConversationGatewayOp::Error)
    {
        return Ok(ConversationGatewayOutput {
            frames: result.run.frames,
            broadcast: vec![],
        });
    }
    let ack_provider_id = result.provider_id.clone();
    let ack_model_id = result.model_id.clone();
    let final_message_id = result
        .run
        .final_message
        .as_ref()
        .map(|message| message.id.clone());
    let mut broadcast = result.run.frames;
    if let Some(message_id) = final_message_id.as_deref() {
        broadcast.push(latest_message_event(
            db_path,
            &conversation_id,
            message_id,
            "message.created",
        )?);
    }

    Ok(ConversationGatewayOutput {
        frames: vec![ack_envelope(
            &client_id,
            Some(&conversation_id),
            "llm.run.request.ack",
            json!({
                "runId": run_id,
                "finalMessageId": final_message_id,
                "providerId": ack_provider_id,
                "modelId": ack_model_id,
                "timeToAckMs": request_received_instant.elapsed().as_millis() as u64,
            }),
            &Utc::now().to_rfc3339(),
        )],
        broadcast,
    })
}

fn run_local_ollama_provider(
    db_path: &Path,
    connection: &Connection,
    actor: &ActorContext,
    request: LlmGatewayRequest,
    env: &std::collections::HashMap<String, String>,
) -> Result<ConversationLlmRunResult> {
    let base_url = first_normalized_env(env, &["ORDO_OLLAMA_BASE_URL", "OLLAMA_BASE_URL"])
        .unwrap_or_else(|| "http://127.0.0.1:11434/api".to_string());
    let timeout_ms = first_normalized_env(env, &["ORDO_OLLAMA_TIMEOUT_MS"])
        .and_then(|value| value.parse::<u64>().ok())
        .filter(|value| *value > 0)
        .unwrap_or_else(|| optional_live_timeout_ms(env));
    let config = OllamaChatConfig::new(
        request.provider_id.clone(),
        request.model_id.clone(),
        base_url,
    )?
    .with_timeout_ms(timeout_ms)?;
    run_llm_with_provider(
        db_path,
        connection,
        actor,
        LlmGateway::new(OllamaChatProvider::new(config)),
        request,
    )
}

fn default_ollama_model(env: &std::collections::HashMap<String, String>) -> String {
    first_normalized_env(env, &["ORDO_OLLAMA_MODEL", "OLLAMA_MODEL"])
        .unwrap_or_else(|| "qwen2.5-coder:7b".to_string())
}

struct ConversationLlmRunResult {
    provider_id: String,
    model_id: String,
    run: crate::llm_gateway::LlmGatewayRunResult,
}

fn run_llm_with_provider<P: LlmProviderAdapter>(
    db_path: &Path,
    connection: &Connection,
    actor: &ActorContext,
    gateway: LlmGateway<P>,
    request: LlmGatewayRequest,
) -> Result<ConversationLlmRunResult> {
    let provider_id = request.provider_id.clone();
    let model_id = request.model_id.clone();
    let run = gateway.run_completion(db_path, connection, actor, request)?;
    Ok(ConversationLlmRunResult {
        provider_id,
        model_id,
        run,
    })
}

fn run_live_provider_or_reject<T: OpenAiCompatibleTransport>(
    db_path: &Path,
    connection: &Connection,
    actor: &ActorContext,
    mut request: LlmGatewayRequest,
    env: &std::collections::HashMap<String, String>,
    openai_transport: T,
    client_id: &str,
    conversation_id: &str,
) -> Result<ConversationLlmRunResult> {
    if request.provider_id == "local_fake" {
        if let Some(provider_id) = first_normalized_env(env, &["ORDO_LIVE_LLM_PROVIDER"]) {
            request.provider_id = provider_id;
            if let Some(model_id) = first_normalized_env(env, &["ORDO_LIVE_LLM_MODEL"]) {
                request.model_id = model_id;
            }
        } else {
            return run_llm_with_provider(
                db_path,
                connection,
                actor,
                LlmGateway::new(DeterministicLlmProvider::new("local_fake", "fake-chat")),
                request,
            );
        }
    }
    if !env_flag_enabled(env, "ORDO_LIVE_LLM_ALLOW_NETWORK") {
        return Ok(rejected_llm_run(
            client_id,
            conversation_id,
            "live_provider_disabled",
            "Live provider network access is disabled for app chat.",
        ));
    }
    if !env.contains_key("ORDO_LIVE_LLM_BUDGET_USD") {
        return Ok(rejected_llm_run(
            client_id,
            conversation_id,
            "live_provider_not_ready",
            "Live provider budget guard is missing for app chat.",
        ));
    }

    let provider_response = list_provider_configs_connection_with_env(connection, env)?;
    let Some(provider) = provider_response
        .providers
        .iter()
        .find(|provider| provider.provider_id == request.provider_id)
    else {
        return Ok(rejected_llm_run(
            client_id,
            conversation_id,
            "live_provider_not_ready",
            "Requested live provider is not in the daemon catalog.",
        ));
    };
    let env_targets_provider = first_normalized_env(env, &["ORDO_LIVE_LLM_PROVIDER"]).as_deref()
        == Some(request.provider_id.as_str());
    let provider_is_configured_for_chat =
        provider.enabled || env_targets_provider || provider.api_key.configured;
    if !provider_is_configured_for_chat || !provider.api_key.configured {
        return Ok(rejected_llm_run(
            client_id,
            conversation_id,
            "live_provider_not_ready",
            "Requested live provider is not configured with a usable key.",
        ));
    }
    if !provider
        .available_models
        .iter()
        .any(|model| model.id == request.model_id)
    {
        return Ok(rejected_llm_run(
            client_id,
            conversation_id,
            "live_provider_not_ready",
            "Requested live provider model is not in the daemon catalog.",
        ));
    }

    let Some(api_key) = resolve_provider_secret(
        db_path,
        connection,
        env,
        &request.provider_id,
        &provider.api_key.source,
    )?
    else {
        return Ok(rejected_llm_run(
            client_id,
            conversation_id,
            "live_provider_not_ready",
            "Requested live provider is missing a usable secret source.",
        ));
    };
    let base_url = provider
        .base_url
        .clone()
        .unwrap_or_else(|| default_provider_base_url(&request.provider_id));
    let timeout_ms = optional_live_timeout_ms(env);

    match request.provider_id.as_str() {
        "openai" | "deepseek" => {
            let config = OpenAiCompatibleConfig::new(
                request.provider_id.clone(),
                request.model_id.clone(),
                base_url,
                api_key,
            )?
            .with_timeout_ms(timeout_ms)?;
            run_llm_with_provider(
                db_path,
                connection,
                actor,
                LlmGateway::new(OpenAiCompatibleProvider::with_transport(
                    config,
                    openai_transport,
                )),
                request,
            )
        }
        "anthropic" => {
            let config = AnthropicMessagesConfig::new(
                request.provider_id.clone(),
                request.model_id.clone(),
                base_url,
                api_key,
            )?
            .with_timeout_ms(timeout_ms)?;
            run_llm_with_provider(
                db_path,
                connection,
                actor,
                LlmGateway::new(AnthropicMessagesProvider::new(config)),
                request,
            )
        }
        _ => Ok(rejected_llm_run(
            client_id,
            conversation_id,
            "live_provider_not_ready",
            "Requested live provider is not supported for app chat.",
        )),
    }
}

fn rejected_llm_run(
    client_id: &str,
    conversation_id: &str,
    code: &str,
    message: &str,
) -> ConversationLlmRunResult {
    ConversationLlmRunResult {
        provider_id: "local_fake".to_string(),
        model_id: "fake-chat".to_string(),
        run: crate::llm_gateway::LlmGatewayRunResult {
            run_id: "llm_run_rejected".to_string(),
            policy_decision_id: "policy_decision_rejected".to_string(),
            prompt: None,
            final_message: None,
            frames: vec![command_rejected_error(
                Some(client_id),
                Some(conversation_id),
                code,
                message,
                false,
                &Utc::now().to_rfc3339(),
            )],
        },
    }
}

fn resolve_provider_secret(
    db_path: &Path,
    connection: &Connection,
    env: &std::collections::HashMap<String, String>,
    provider_id: &str,
    source: &str,
) -> Result<Option<OrdoSecretString>> {
    let keys = provider_secret_env_keys(provider_id);
    match source {
        "env" => Ok(first_normalized_env(env, keys).and_then(normalize_secret)),
        "file" => Ok(first_normalized_secret_file(env, keys).and_then(normalize_secret)),
        "vault" => {
            let secret_ref: Option<String> = connection
                .query_row(
                    "SELECT secret_ref FROM provider_configs WHERE provider_id = ?1",
                    [provider_id],
                    |row| row.get(0),
                )
                .optional()?;
            secret_ref
                .map(|secret_ref| {
                    decrypt_secret(db_path, connection, &secret_ref).map(normalize_secret)
                })
                .transpose()
                .map(Option::flatten)
        }
        _ => Ok(None),
    }
}

fn provider_secret_env_keys(provider_id: &str) -> &'static [&'static str] {
    match provider_id {
        "anthropic" => &["ANTHROPIC_API_KEY", "API__ANTHROPIC_API_KEY"],
        "deepseek" => &["DEEPSEEK_API_KEY", "API__DEEPSEEK_API_KEY", "deepseek"],
        "openai" => &["OPENAI_API_KEY", "API__OPENAI_API_KEY"],
        _ => &[],
    }
}

fn default_provider_base_url(provider_id: &str) -> String {
    match provider_id {
        "anthropic" => "https://api.anthropic.com/v1".to_string(),
        "deepseek" => "https://api.deepseek.com/v1".to_string(),
        _ => "https://api.openai.com/v1".to_string(),
    }
}

fn optional_live_timeout_ms(env: &std::collections::HashMap<String, String>) -> u64 {
    env.get("ORDO_LIVE_LLM_TIMEOUT_MS")
        .and_then(|value| value.trim().parse::<u64>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(30_000)
}

fn first_normalized_env(
    env: &std::collections::HashMap<String, String>,
    keys: &[&str],
) -> Option<String> {
    keys.iter()
        .find_map(|key| normalize_env_value(env.get(*key)))
}

fn first_normalized_secret_file(
    env: &std::collections::HashMap<String, String>,
    keys: &[&str],
) -> Option<String> {
    keys.iter().find_map(|key| {
        let file_key = format!("{key}_FILE");
        normalize_env_value(env.get(&file_key)).and_then(|path| {
            fs::read_to_string(path)
                .ok()
                .and_then(|value| normalize_string_value(&value))
        })
    })
}

fn normalize_env_value(value: Option<&String>) -> Option<String> {
    value.and_then(|value| normalize_string_value(value))
}

fn normalize_string_value(value: &str) -> Option<String> {
    let normalized = value.split_whitespace().collect::<Vec<_>>().join(" ");
    (!normalized.is_empty()).then_some(normalized)
}

fn env_flag_enabled(env: &std::collections::HashMap<String, String>, key: &str) -> bool {
    normalize_env_value(env.get(key)).as_deref() == Some("1")
}

pub(crate) fn llm_run_cancel(
    db_path: &Path,
    session: &ConversationGatewaySession,
    envelope: ConversationGatewayEnvelope,
) -> Result<ConversationGatewayOutput> {
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct CancelPayload {
        run_id: String,
    }

    let conversation_id = required_conversation_id(&envelope)?.to_string();
    let client_id = required_client_id(&envelope)?.to_string();
    let payload: CancelPayload = serde_json::from_value(envelope.payload.clone())?;
    let connection = Connection::open(db_path)?;
    let gateway = local_llm_gateway();
    let actor = llm_gateway_actor(session);
    let result = gateway.cancel_run(
        &connection,
        &actor,
        &conversation_id,
        &payload.run_id,
        Some(&client_id),
    )?;
    if result
        .frames
        .iter()
        .any(|frame| frame.op == ConversationGatewayOp::Error)
    {
        return Ok(ConversationGatewayOutput {
            frames: result.frames,
            broadcast: vec![],
        });
    }

    Ok(ConversationGatewayOutput {
        frames: vec![ack_envelope(
            &client_id,
            Some(&conversation_id),
            "llm.run.cancel.ack",
            json!({
                "runId": result.run_id,
                "policyDecisionId": result.policy_decision_id,
            }),
            &Utc::now().to_rfc3339(),
        )],
        broadcast: result.frames,
    })
}

pub(crate) fn llm_tool_approve(
    db_path: &Path,
    session: &ConversationGatewaySession,
    envelope: ConversationGatewayEnvelope,
) -> Result<ConversationGatewayOutput> {
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct ToolDecisionPayload {
        tool_request_id: String,
        reason: Option<String>,
    }

    let conversation_id = required_conversation_id(&envelope)?.to_string();
    let payload: ToolDecisionPayload = serde_json::from_value(envelope.payload.clone())?;
    let connection = Connection::open(db_path)?;
    let gateway = local_llm_gateway();
    let receipt = gateway.approve_tool_request(
        &connection,
        &llm_gateway_actor(session),
        &conversation_id,
        &payload.tool_request_id,
        payload
            .reason
            .as_deref()
            .unwrap_or("Approved from the conversation gateway."),
    )?;
    llm_tool_ack_and_broadcast(&envelope, "tool.approve.ack", receipt)
}

pub(crate) fn llm_tool_reject(
    db_path: &Path,
    session: &ConversationGatewaySession,
    envelope: ConversationGatewayEnvelope,
) -> Result<ConversationGatewayOutput> {
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct ToolDecisionPayload {
        tool_request_id: String,
        reason: String,
    }

    let conversation_id = required_conversation_id(&envelope)?.to_string();
    let payload: ToolDecisionPayload = serde_json::from_value(envelope.payload.clone())?;
    let connection = Connection::open(db_path)?;
    let gateway = local_llm_gateway();
    let receipt = gateway.reject_tool_request(
        &connection,
        &llm_gateway_actor(session),
        &conversation_id,
        &payload.tool_request_id,
        &payload.reason,
    )?;
    llm_tool_ack_and_broadcast(&envelope, "tool.reject.ack", receipt)
}

pub(crate) fn llm_tool_execute(
    db_path: &Path,
    session: &ConversationGatewaySession,
    envelope: ConversationGatewayEnvelope,
) -> Result<ConversationGatewayOutput> {
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct ToolExecutePayload {
        tool_request_id: String,
        output_summary: String,
    }

    let conversation_id = required_conversation_id(&envelope)?.to_string();
    let payload: ToolExecutePayload = serde_json::from_value(envelope.payload.clone())?;
    let connection = Connection::open(db_path)?;
    let gateway = local_llm_gateway();
    let receipt = gateway.execute_approved_tool_request(
        &connection,
        &llm_gateway_actor(session),
        &conversation_id,
        &payload.tool_request_id,
        &payload.output_summary,
    )?;
    llm_tool_ack_and_broadcast(&envelope, "tool.execute.ack", receipt)
}

fn llm_tool_ack_and_broadcast(
    envelope: &ConversationGatewayEnvelope,
    ack_type: &str,
    receipt: LlmToolRequestReceipt,
) -> Result<ConversationGatewayOutput> {
    if receipt
        .frames
        .iter()
        .any(|frame| frame.op == ConversationGatewayOp::Error)
    {
        return Ok(ConversationGatewayOutput {
            frames: receipt.frames,
            broadcast: vec![],
        });
    }
    let conversation_id = required_conversation_id(envelope)?.to_string();
    let tool_request = receipt
        .tool_request
        .ok_or_else(|| anyhow::anyhow!("LLM tool receipt did not include request state"))?;
    Ok(ConversationGatewayOutput {
        frames: vec![ack_envelope(
            required_client_id(envelope)?,
            Some(&conversation_id),
            ack_type,
            json!({
                "toolRequestId": tool_request.tool_request_id,
                "runId": tool_request.run_id,
                "status": tool_request.status.as_str(),
                "policyDecisionId": receipt.policy_decision_id,
            }),
            &Utc::now().to_rfc3339(),
        )],
        broadcast: receipt.frames,
    })
}

fn local_llm_gateway() -> LlmGateway<DeterministicLlmProvider> {
    LlmGateway::new(DeterministicLlmProvider::new("local_fake", "fake-chat"))
}

fn llm_gateway_actor(session: &ConversationGatewaySession) -> ActorContext {
    ActorContext::new(
        ActorKind::BrowserOperator,
        "conversation_gateway",
        session.actor_id.clone(),
    )
}

pub(crate) fn message_edit(
    db_path: &Path,
    session: &ConversationGatewaySession,
    envelope: ConversationGatewayEnvelope,
) -> Result<ConversationGatewayOutput> {
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct EditPayload {
        participant_id: Option<String>,
        message_id: String,
        body_markdown: String,
        reason: Option<String>,
    }
    let payload: EditPayload = serde_json::from_value(envelope.payload.clone())?;
    let participant_id = payload
        .participant_id
        .or_else(|| session.participant_id.clone())
        .ok_or_else(|| anyhow::anyhow!("participantId is required"))?;
    let actor = mutation_actor(session, envelope.client_id.clone());
    let connection = Connection::open(db_path)?;
    let receipt = ConversationService::edit_message(
        &connection,
        &actor,
        &payload.message_id,
        &participant_id,
        &payload.body_markdown,
        payload.reason.as_deref(),
    )?;
    command_ack_and_dispatch(
        &envelope,
        "message.edit.ack",
        &receipt.value.id,
        "message.edited",
        db_path,
    )
}

pub(crate) fn message_delete(
    db_path: &Path,
    session: &ConversationGatewaySession,
    envelope: ConversationGatewayEnvelope,
) -> Result<ConversationGatewayOutput> {
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct DeletePayload {
        participant_id: Option<String>,
        message_id: String,
        reason: String,
    }
    let payload: DeletePayload = serde_json::from_value(envelope.payload.clone())?;
    let participant_id = payload
        .participant_id
        .or_else(|| session.participant_id.clone())
        .ok_or_else(|| anyhow::anyhow!("participantId is required"))?;
    let actor = mutation_actor(session, envelope.client_id.clone());
    let connection = Connection::open(db_path)?;
    let receipt = ConversationService::delete_message(
        &connection,
        &actor,
        &payload.message_id,
        &participant_id,
        &payload.reason,
    )?;
    command_ack_and_dispatch(
        &envelope,
        "message.delete.ack",
        &receipt.value.id,
        "message.tombstoned",
        db_path,
    )
}

pub(crate) fn message_undo(
    db_path: &Path,
    session: &ConversationGatewaySession,
    envelope: ConversationGatewayEnvelope,
) -> Result<ConversationGatewayOutput> {
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct UndoPayload {
        participant_id: Option<String>,
        message_id: String,
    }
    let payload: UndoPayload = serde_json::from_value(envelope.payload.clone())?;
    let participant_id = payload
        .participant_id
        .or_else(|| session.participant_id.clone())
        .ok_or_else(|| anyhow::anyhow!("participantId is required"))?;
    let actor = mutation_actor(session, envelope.client_id.clone());
    let connection = Connection::open(db_path)?;
    let receipt = ConversationService::undo_message(
        &connection,
        &actor,
        &payload.message_id,
        &participant_id,
    )?;
    command_ack_and_dispatch(
        &envelope,
        "message.undo.ack",
        &receipt.value.id,
        "message.undo.cancelled",
        db_path,
    )
}

pub(crate) fn message_mark_read(
    db_path: &Path,
    session: &ConversationGatewaySession,
    envelope: ConversationGatewayEnvelope,
) -> Result<ConversationGatewayOutput> {
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct MarkReadPayload {
        participant_id: Option<String>,
        message_id: String,
    }
    let conversation_id = required_conversation_id(&envelope)?.to_string();
    let payload: MarkReadPayload = serde_json::from_value(envelope.payload.clone())?;
    let participant_id = payload
        .participant_id
        .or_else(|| session.participant_id.clone())
        .ok_or_else(|| anyhow::anyhow!("participantId is required"))?;
    let actor = mutation_actor(session, envelope.client_id.clone());
    let connection = Connection::open(db_path)?;
    let receipt = ConversationService::mark_read(
        &connection,
        &actor,
        &conversation_id,
        &participant_id,
        &payload.message_id,
    )?;
    ack_with_optional_message_dispatch(
        &envelope,
        "message.mark_read.ack",
        json!({
            "messageId": payload.message_id,
            "readState": receipt.value.value,
            "changed": receipt.value.changed,
        }),
        receipt.value.event_type.as_deref(),
        receipt.value.changed,
        db_path,
        &payload.message_id,
    )
}

pub(crate) fn message_mark_unread(
    db_path: &Path,
    session: &ConversationGatewaySession,
    envelope: ConversationGatewayEnvelope,
) -> Result<ConversationGatewayOutput> {
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct MarkUnreadPayload {
        participant_id: Option<String>,
        message_id: String,
    }
    let conversation_id = required_conversation_id(&envelope)?.to_string();
    let payload: MarkUnreadPayload = serde_json::from_value(envelope.payload.clone())?;
    let participant_id = payload
        .participant_id
        .or_else(|| session.participant_id.clone())
        .ok_or_else(|| anyhow::anyhow!("participantId is required"))?;
    let actor = mutation_actor(session, envelope.client_id.clone());
    let connection = Connection::open(db_path)?;
    let receipt = ConversationService::mark_unread(
        &connection,
        &actor,
        &conversation_id,
        &participant_id,
        &payload.message_id,
    )?;
    ack_with_optional_message_dispatch(
        &envelope,
        "message.mark_unread.ack",
        json!({
            "messageId": payload.message_id,
            "readState": receipt.value.value,
            "changed": receipt.value.changed,
        }),
        receipt.value.event_type.as_deref(),
        receipt.value.changed,
        db_path,
        &payload.message_id,
    )
}

pub(crate) fn message_react(
    db_path: &Path,
    session: &ConversationGatewaySession,
    envelope: ConversationGatewayEnvelope,
) -> Result<ConversationGatewayOutput> {
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct ReactPayload {
        participant_id: Option<String>,
        message_id: String,
        reaction_key: String,
        reaction_kind: Option<String>,
        action: Option<String>,
    }
    let payload: ReactPayload = serde_json::from_value(envelope.payload.clone())?;
    let participant_id = payload
        .participant_id
        .or_else(|| session.participant_id.clone())
        .ok_or_else(|| anyhow::anyhow!("participantId is required"))?;
    let action = ReactionAction::try_from(payload.action.as_deref().unwrap_or("toggle"))?;
    let actor = mutation_actor(session, envelope.client_id.clone());
    let connection = Connection::open(db_path)?;
    let receipt = ConversationService::react_to_message(
        &connection,
        &actor,
        &payload.message_id,
        &participant_id,
        &payload.reaction_key,
        payload.reaction_kind.as_deref().unwrap_or("emoji"),
        action,
    )?;
    ack_with_optional_message_dispatch(
        &envelope,
        "message.react.ack",
        json!({
            "messageId": payload.message_id,
            "reaction": receipt.value.value,
            "changed": receipt.value.changed,
        }),
        receipt.value.event_type.as_deref(),
        receipt.value.changed,
        db_path,
        &payload.message_id,
    )
}

pub(crate) fn presence_update(
    db_path: &Path,
    session: &ConversationGatewaySession,
    envelope: ConversationGatewayEnvelope,
) -> Result<ConversationGatewayOutput> {
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct PresencePayload {
        participant_id: Option<String>,
        status: String,
        visibility: Option<String>,
        status_message: Option<String>,
        device_class: Option<String>,
        expires_at: Option<String>,
    }
    let conversation_id = required_conversation_id(&envelope)?.to_string();
    let payload: PresencePayload = serde_json::from_value(envelope.payload.clone())?;
    let participant_id = payload
        .participant_id
        .or_else(|| session.participant_id.clone())
        .ok_or_else(|| anyhow::anyhow!("participantId is required"))?;
    let actor = mutation_actor(session, envelope.client_id.clone());
    let connection = Connection::open(db_path)?;
    let receipt = ConversationService::update_presence(
        &connection,
        &actor,
        &ConversationPresenceUpdateRequest {
            conversation_id: conversation_id.clone(),
            participant_id: participant_id.clone(),
            status: payload.status,
            visibility: payload
                .visibility
                .unwrap_or_else(|| "participants".to_string()),
            status_message: payload.status_message,
            device_class: payload.device_class,
            expires_at: payload.expires_at,
        },
    )?;
    let dispatch = ConversationGatewayEnvelope {
        schema_version: CONVERSATION_GATEWAY_SCHEMA_VERSION.to_string(),
        op: ConversationGatewayOp::Dispatch,
        frame_type: "presence.changed".to_string(),
        client_id: envelope.client_id.clone(),
        server_id: None,
        conversation_id: Some(conversation_id.clone()),
        segment_id: None,
        sequence: None,
        cursor: None,
        durability: ConversationGatewayDurability::Ephemeral,
        scope: ConversationGatewayScope::Conversation,
        payload: json!({ "presence": receipt.value }),
        occurred_at: Utc::now().to_rfc3339(),
    };
    Ok(ConversationGatewayOutput {
        frames: vec![ack_envelope(
            required_client_id(&envelope)?,
            Some(&conversation_id),
            "presence.update.ack",
            json!({ "participantId": participant_id }),
            &Utc::now().to_rfc3339(),
        )],
        broadcast: vec![dispatch],
    })
}

pub(crate) fn typing(
    session: &mut ConversationGatewaySession,
    envelope: ConversationGatewayEnvelope,
) -> Result<ConversationGatewayOutput> {
    let conversation_id = required_conversation_id(&envelope)?.to_string();
    let participant_id = session
        .participant_id
        .clone()
        .ok_or_else(|| anyhow::anyhow!("participantId is required before typing"))?;
    let participants = session
        .typing_by_conversation
        .entry(conversation_id.clone())
        .or_default();
    if envelope.frame_type == "typing.start" {
        participants.insert(participant_id.clone());
    } else {
        participants.remove(&participant_id);
    }
    let event_type = if envelope.frame_type == "typing.start" {
        "typing.started"
    } else {
        "typing.stopped"
    };
    let dispatch = ConversationGatewayEnvelope {
        schema_version: CONVERSATION_GATEWAY_SCHEMA_VERSION.to_string(),
        op: ConversationGatewayOp::Dispatch,
        frame_type: event_type.to_string(),
        client_id: envelope.client_id.clone(),
        server_id: None,
        conversation_id: Some(conversation_id.clone()),
        segment_id: None,
        sequence: None,
        cursor: None,
        durability: ConversationGatewayDurability::Ephemeral,
        scope: ConversationGatewayScope::Conversation,
        payload: json!({
            "conversationId": conversation_id,
            "participantId": participant_id,
            "expiresAt": if event_type == "typing.started" {
                Some((Utc::now() + chrono::Duration::seconds(5)).to_rfc3339())
            } else {
                None
            },
        }),
        occurred_at: Utc::now().to_rfc3339(),
    };
    Ok(ConversationGatewayOutput {
        frames: vec![ack_envelope(
            required_client_id(&envelope)?,
            dispatch.conversation_id.as_deref(),
            "typing.ack",
            json!({ "eventType": event_type }),
            &Utc::now().to_rfc3339(),
        )],
        broadcast: vec![dispatch],
    })
}

pub(crate) fn handoff_create(
    db_path: &Path,
    session: &ConversationGatewaySession,
    envelope: ConversationGatewayEnvelope,
) -> Result<ConversationGatewayOutput> {
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct HandoffCreatePayload {
        segment_id: Option<String>,
        connection_id: Option<String>,
        requested_by_actor_id: Option<String>,
        assigned_to_actor_id: Option<String>,
        reason: String,
        urgency: Option<String>,
        required_capability_id: Option<String>,
        evidence_summary: String,
        allowed_context: Option<Vec<String>>,
        policy_decision_id: Option<String>,
    }

    let conversation_id = required_conversation_id(&envelope)?.to_string();
    let payload: HandoffCreatePayload = serde_json::from_value(envelope.payload.clone())?;
    let connection = Connection::open(db_path)?;
    let handoff = create_conversation_handoff(
        &connection,
        &ConversationHandoffCreateRequest {
            conversation_id: conversation_id.clone(),
            segment_id: payload.segment_id,
            connection_id: payload.connection_id,
            requested_by_actor_id: payload
                .requested_by_actor_id
                .or_else(|| session.actor_id.clone()),
            assigned_to_actor_id: payload.assigned_to_actor_id,
            reason: payload.reason,
            urgency: payload.urgency.unwrap_or_else(|| "normal".to_string()),
            required_capability_id: payload
                .required_capability_id
                .unwrap_or_else(|| "conversation.handoff.manage".to_string()),
            evidence_summary: payload.evidence_summary,
            allowed_context: payload.allowed_context.unwrap_or_default(),
            policy_decision_id: payload.policy_decision_id,
        },
    )?;
    ack_and_latest_conversation_event(
        &envelope,
        "conversation.handoff.create.ack",
        json!({ "handoffId": handoff.id, "status": "requested" }),
        "conversation.handoff.requested",
        Some(&handoff.id),
        db_path,
    )
}

pub(crate) fn handoff_transition(
    db_path: &Path,
    session: &ConversationGatewaySession,
    envelope: ConversationGatewayEnvelope,
    next_status: HandoffStatus,
    ack_type: &str,
) -> Result<ConversationGatewayOutput> {
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct HandoffTransitionPayload {
        handoff_id: String,
        actor_id: Option<String>,
        assigned_to_actor_id: Option<String>,
        reason: Option<String>,
    }

    let payload: HandoffTransitionPayload = serde_json::from_value(envelope.payload.clone())?;
    let actor_id = payload
        .actor_id
        .or(payload.assigned_to_actor_id)
        .or_else(|| session.actor_id.clone());
    let reason = payload.reason.unwrap_or_else(|| {
        format!(
            "gateway command {}",
            envelope.frame_type.trim_start_matches("conversation.")
        )
    });
    let connection = Connection::open(db_path)?;
    let handoff = transition_conversation_handoff(
        &connection,
        &payload.handoff_id,
        next_status,
        actor_id.as_deref(),
        &reason,
    )?;
    let event_type = handoff_status_event_type(next_status);
    ack_and_latest_conversation_event(
        &envelope,
        ack_type,
        json!({ "handoffId": handoff.id, "status": next_status }),
        event_type,
        Some(&handoff.id),
        db_path,
    )
}

pub(crate) fn conversation_mode_set(
    db_path: &Path,
    session: &ConversationGatewaySession,
    envelope: ConversationGatewayEnvelope,
) -> Result<ConversationGatewayOutput> {
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct ModePayload {
        mode: String,
        led_by_actor_id: Option<String>,
        delegated_to_agent: Option<bool>,
        delegation_scope: Option<Vec<String>>,
        idle_after: Option<String>,
    }

    let conversation_id = required_conversation_id(&envelope)?.to_string();
    let payload: ModePayload = serde_json::from_value(envelope.payload.clone())?;
    let mode = ConversationMode::try_from(payload.mode.as_str())?;
    let led_by_actor_id = payload.led_by_actor_id.or_else(|| session.actor_id.clone());
    let connection = Connection::open(db_path)?;
    let mode_view = upsert_conversation_mode(
        &connection,
        &conversation_id,
        mode,
        led_by_actor_id.as_deref(),
        payload.delegated_to_agent.unwrap_or(false),
        payload.delegation_scope.unwrap_or_default(),
        payload.idle_after.as_deref(),
    )?;
    ack_and_latest_conversation_event(
        &envelope,
        "conversation.mode.set.ack",
        json!({ "mode": mode_view }),
        "conversation.mode.changed",
        None,
        db_path,
    )
}

pub(crate) fn conversation_mode_fixed(
    db_path: &Path,
    session: &ConversationGatewaySession,
    envelope: ConversationGatewayEnvelope,
    mode: ConversationMode,
    delegated_to_agent: bool,
    ack_type: &str,
) -> Result<ConversationGatewayOutput> {
    let conversation_id = required_conversation_id(&envelope)?.to_string();
    let connection = Connection::open(db_path)?;
    let mode_view = upsert_conversation_mode(
        &connection,
        &conversation_id,
        mode,
        session.actor_id.as_deref(),
        delegated_to_agent,
        vec![],
        None,
    )?;
    ack_and_latest_conversation_event(
        &envelope,
        ack_type,
        json!({ "mode": mode_view }),
        "conversation.mode.changed",
        None,
        db_path,
    )
}

pub(crate) fn agent_delegation(
    db_path: &Path,
    session: &ConversationGatewaySession,
    envelope: ConversationGatewayEnvelope,
    delegated_to_agent: bool,
) -> Result<ConversationGatewayOutput> {
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct DelegationPayload {
        delegation_scope: Option<Vec<String>>,
        reason: Option<String>,
    }

    let conversation_id = required_conversation_id(&envelope)?.to_string();
    let payload: DelegationPayload = serde_json::from_value(envelope.payload.clone())?;
    if delegated_to_agent {
        ensure!(
            payload
                .delegation_scope
                .as_ref()
                .is_some_and(|scope| !scope.is_empty()),
            "delegationScope is required for agent delegation"
        );
    }
    let _reason = payload.reason;
    let scope = if delegated_to_agent {
        payload.delegation_scope.unwrap_or_default()
    } else {
        vec![]
    };
    let connection = Connection::open(db_path)?;
    let mode_view = upsert_conversation_mode(
        &connection,
        &conversation_id,
        ConversationMode::HumanLedActive,
        session.actor_id.as_deref(),
        delegated_to_agent,
        scope,
        None,
    )?;
    let ack_type = if delegated_to_agent {
        "conversation.agent.delegate.ack"
    } else {
        "conversation.agent.delegation_revoke.ack"
    };
    ack_and_latest_conversation_event(
        &envelope,
        ack_type,
        json!({ "mode": mode_view }),
        "conversation.mode.changed",
        None,
        db_path,
    )
}

pub(crate) fn command_ack_and_dispatch(
    envelope: &ConversationGatewayEnvelope,
    ack_type: &str,
    message_id: &str,
    event_type: &str,
    db_path: &Path,
) -> Result<ConversationGatewayOutput> {
    let conversation_id = required_conversation_id(envelope)?.to_string();
    let dispatch = latest_message_event(db_path, &conversation_id, message_id, event_type)?;
    Ok(ConversationGatewayOutput {
        frames: vec![ack_envelope(
            required_client_id(envelope)?,
            Some(&conversation_id),
            ack_type,
            json!({ "messageId": message_id }),
            &Utc::now().to_rfc3339(),
        )],
        broadcast: vec![dispatch],
    })
}

pub(crate) fn ack_with_optional_message_dispatch(
    envelope: &ConversationGatewayEnvelope,
    ack_type: &str,
    payload: Value,
    event_type: Option<&str>,
    changed: bool,
    db_path: &Path,
    message_id: &str,
) -> Result<ConversationGatewayOutput> {
    let conversation_id = required_conversation_id(envelope)?.to_string();
    let broadcast = if changed {
        if let Some(event_type) = event_type {
            vec![latest_message_event(
                db_path,
                &conversation_id,
                message_id,
                event_type,
            )?]
        } else {
            vec![]
        }
    } else {
        vec![]
    };
    Ok(ConversationGatewayOutput {
        frames: vec![ack_envelope(
            required_client_id(envelope)?,
            Some(&conversation_id),
            ack_type,
            payload,
            &Utc::now().to_rfc3339(),
        )],
        broadcast,
    })
}

pub(crate) fn ack_and_latest_conversation_event(
    envelope: &ConversationGatewayEnvelope,
    ack_type: &str,
    payload: Value,
    event_type: &str,
    payload_marker: Option<&str>,
    db_path: &Path,
) -> Result<ConversationGatewayOutput> {
    let conversation_id = required_conversation_id(envelope)?.to_string();
    let dispatch =
        latest_conversation_event(db_path, &conversation_id, event_type, payload_marker)?;
    Ok(ConversationGatewayOutput {
        frames: vec![ack_envelope(
            required_client_id(envelope)?,
            Some(&conversation_id),
            ack_type,
            payload,
            &Utc::now().to_rfc3339(),
        )],
        broadcast: vec![dispatch],
    })
}

pub(crate) fn latest_message_event(
    db_path: &Path,
    conversation_id: &str,
    message_id: &str,
    event_type: &str,
) -> Result<ConversationGatewayEnvelope> {
    latest_conversation_event(db_path, conversation_id, event_type, Some(message_id))
}

pub(crate) fn latest_conversation_event(
    db_path: &Path,
    conversation_id: &str,
    event_type: &str,
    payload_marker: Option<&str>,
) -> Result<ConversationGatewayEnvelope> {
    let connection = Connection::open(db_path)?;
    let mut sql = String::from(
        "SELECT sequence, event_type, payload_json, realtime_cursor, occurred_at
         FROM conversation_events
         WHERE conversation_id = ?1
           AND event_type = ?2",
    );
    let marker = payload_marker.map(|marker| format!("%{marker}%"));
    if marker.is_some() {
        sql.push_str(" AND payload_json LIKE ?3");
    }
    sql.push_str(" ORDER BY sequence DESC LIMIT 1");
    let mut statement = connection.prepare(&sql)?;
    let map_row = |row: &rusqlite::Row<'_>| {
        let sequence: i64 = row.get(0)?;
        let event_type: String = row.get(1)?;
        let payload_json: String = row.get(2)?;
        let cursor: Option<i64> = row.get(3)?;
        let occurred_at: String = row.get(4)?;
        let payload = serde_json::from_str(&payload_json).unwrap_or_else(|_| json!({}));
        Ok(dispatch_envelope(
            &event_type,
            conversation_id,
            sequence,
            cursor,
            payload,
            &occurred_at,
        ))
    };
    let event = if let Some(marker) = marker {
        statement
            .query_row(params![conversation_id, event_type, marker], map_row)
            .optional()?
    } else {
        statement
            .query_row(params![conversation_id, event_type], map_row)
            .optional()?
    };
    event.ok_or_else(|| {
        anyhow::anyhow!("durable command completed without replayable {event_type} event")
    })
}

pub(crate) fn handoff_status_event_type(status: HandoffStatus) -> &'static str {
    match status {
        HandoffStatus::Suggested => "conversation.handoff.suggested",
        HandoffStatus::Requested => "conversation.handoff.requested",
        HandoffStatus::Accepted => "conversation.handoff.accepted",
        HandoffStatus::Declined => "conversation.handoff.declined",
        HandoffStatus::Assigned => "conversation.handoff.assigned",
        HandoffStatus::InProgress => "conversation.handoff.in_progress",
        HandoffStatus::ReturnedToAgent => "conversation.handoff.returned_to_agent",
        HandoffStatus::Closed => "conversation.handoff.closed",
    }
}

pub(crate) fn replay_conversation_events(
    db_path: &Path,
    conversation_id: &str,
    after_sequence: i64,
    limit: usize,
) -> Result<Vec<ConversationGatewayEnvelope>> {
    ensure!(
        (1..=MAX_REPLAY_LIMIT).contains(&limit),
        "invalid replay limit"
    );
    let connection = Connection::open(db_path)?;
    let mut statement = connection.prepare(
        "SELECT sequence, event_type, payload_json, realtime_cursor, occurred_at
         FROM conversation_events
         WHERE conversation_id = ?1 AND sequence > ?2
         ORDER BY sequence ASC
         LIMIT ?3",
    )?;
    let rows = statement.query_map(
        params![conversation_id, after_sequence, limit as i64],
        |row| {
            let sequence: i64 = row.get(0)?;
            let event_type: String = row.get(1)?;
            let payload_json: String = row.get(2)?;
            let cursor: Option<i64> = row.get(3)?;
            let occurred_at: String = row.get(4)?;
            let payload = serde_json::from_str(&payload_json).unwrap_or_else(|_| json!({}));
            Ok(dispatch_envelope(
                &event_type,
                conversation_id,
                sequence,
                cursor,
                payload,
                &occurred_at,
            ))
        },
    )?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(Into::into)
}

pub(crate) fn enforce_message_command_rate_limit(
    session: &mut ConversationGatewaySession,
    envelope: &ConversationGatewayEnvelope,
) -> Result<Option<ConversationGatewayOutput>> {
    let now = Utc::now();
    let floor = now - ChronoDuration::seconds(COMMAND_RATE_WINDOW_SECONDS);
    while session
        .recent_message_commands
        .front()
        .is_some_and(|timestamp| *timestamp < floor)
    {
        session.recent_message_commands.pop_front();
    }
    if session.recent_message_commands.len() >= COMMAND_RATE_LIMIT {
        return Ok(Some(error_output(command_rejected_error(
            envelope.client_id.as_deref(),
            envelope.conversation_id.as_deref(),
            "rate_limited",
            "Message command rate limit exceeded.",
            true,
            &now.to_rfc3339(),
        ))));
    }
    session.recent_message_commands.push_back(now);
    Ok(None)
}

pub(crate) fn mutation_actor(
    session: &ConversationGatewaySession,
    request_id: Option<String>,
) -> ConversationMutationActor {
    ConversationMutationActor {
        actor: ActorContext::new(
            ActorKind::BrowserOperator,
            "conversation_gateway",
            session.actor_id.clone(),
        ),
        request_id,
    }
}

pub(crate) fn required_client_id(envelope: &ConversationGatewayEnvelope) -> Result<&str> {
    envelope
        .client_id
        .as_deref()
        .filter(|client_id| !client_id.trim().is_empty())
        .ok_or_else(|| anyhow::anyhow!("clientId is required"))
}

pub(crate) fn required_conversation_id(envelope: &ConversationGatewayEnvelope) -> Result<&str> {
    envelope
        .conversation_id
        .as_deref()
        .filter(|conversation_id| !conversation_id.trim().is_empty())
        .ok_or_else(|| anyhow::anyhow!("conversationId is required"))
}

pub(crate) fn after_sequence(envelope: &ConversationGatewayEnvelope) -> i64 {
    envelope
        .payload
        .get("afterSequence")
        .and_then(Value::as_i64)
        .unwrap_or(0)
        .max(0)
}

pub(crate) fn limit(envelope: &ConversationGatewayEnvelope) -> usize {
    envelope
        .payload
        .get("limit")
        .and_then(Value::as_u64)
        .map(|limit| limit as usize)
        .unwrap_or(DEFAULT_REPLAY_LIMIT)
        .clamp(1, MAX_REPLAY_LIMIT)
}

pub(crate) fn single_frame(frame: ConversationGatewayEnvelope) -> ConversationGatewayOutput {
    ConversationGatewayOutput {
        frames: vec![frame],
        broadcast: vec![],
    }
}

pub(crate) fn error_output(frame: ConversationGatewayEnvelope) -> ConversationGatewayOutput {
    single_frame(frame)
}

pub(crate) fn lagged_client_frame(skipped: u64, occurred_at: &str) -> ConversationGatewayEnvelope {
    command_rejected_error(
        None,
        None,
        "client_lagged",
        &format!(
            "Conversation gateway client skipped {skipped} frame(s); replay from the latest durable cursor."
        ),
        true,
        occurred_at,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::conversation_gateway::{
        handle_gateway_envelope, handle_gateway_text_frame, hello_frame, MAX_TEXT_FRAME_BYTES,
    };
    use crate::conversations::{
        create_conversation_participant, find_or_create_canonical_conversation,
        CanonicalConversationRequest, ConversationParticipantCreateRequest,
    };
    use crate::llm_gateway::{LlmToolRequestCreateRequest, OpenAiTransportResponse};
    use crate::schema::init_database;
    use std::collections::HashMap;

    #[derive(Clone)]
    struct MockOpenAiTransport {
        text: String,
    }

    impl MockOpenAiTransport {
        fn success(text: &str) -> Self {
            Self {
                text: text.to_string(),
            }
        }
    }

    impl OpenAiCompatibleTransport for MockOpenAiTransport {
        fn post_chat_completions(
            &self,
            _endpoint: &str,
            api_key: &str,
            _timeout_ms: u64,
            _body: &Value,
        ) -> Result<OpenAiTransportResponse> {
            assert!(!api_key.is_empty());
            Ok(OpenAiTransportResponse {
                status: 200,
                body: json!({
                    "choices": [{ "message": { "content": self.text } }],
                    "usage": { "prompt_tokens": 8, "completion_tokens": 3 }
                }),
            })
        }
    }

    fn seeded_db() -> (tempfile::TempDir, std::path::PathBuf, String, String) {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let db_path = temp_dir.path().join("local.db");
        init_database(&db_path).unwrap();
        let connection = Connection::open(&db_path).unwrap();
        connection
            .execute(
                "INSERT INTO actors (id, actor_kind, display_name, status, metadata_json, created_at, updated_at)
                 VALUES ('actor_staff', 'staff', 'Staff', 'active', '{}', 'now', 'now')",
                [],
            )
            .unwrap();
        let conversation = find_or_create_canonical_conversation(
            &connection,
            &CanonicalConversationRequest {
                surface: "client_portal".to_string(),
                subject_kind: "connection".to_string(),
                subject_id: "connection_1".to_string(),
                connection_id: None,
                visitor_session_id: None,
                created_by_actor_id: None,
            },
        )
        .unwrap();
        let participant = create_conversation_participant(
            &connection,
            &ConversationParticipantCreateRequest {
                conversation_id: conversation.id.clone(),
                participant_kind: "staff".to_string(),
                actor_id: Some("actor_staff".to_string()),
                connection_id: None,
                visitor_session_id: None,
                display_name: "Staff".to_string(),
                role: "staff".to_string(),
            },
        )
        .unwrap();
        let assistant = create_conversation_participant(
            &connection,
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
        drop(assistant);
        (temp_dir, db_path, conversation.id, participant.id)
    }

    fn assistant_participant_id(db_path: &Path, conversation_id: &str) -> String {
        Connection::open(db_path)
            .unwrap()
            .query_row(
                "SELECT id FROM conversation_participants
                 WHERE conversation_id = ?1 AND role = 'assistant'
                 LIMIT 1",
                [conversation_id],
                |row| row.get(0),
            )
            .unwrap()
    }

    fn session(participant_id: String) -> ConversationGatewaySession {
        let mut session = ConversationGatewaySession::new("session_1");
        session.actor_id = Some("actor_staff".to_string());
        session.participant_id = Some(participant_id);
        session
    }

    fn command(
        frame_type: &str,
        client_id: &str,
        conversation_id: &str,
        payload: Value,
    ) -> ConversationGatewayEnvelope {
        ConversationGatewayEnvelope {
            schema_version: CONVERSATION_GATEWAY_SCHEMA_VERSION.to_string(),
            op: ConversationGatewayOp::Command,
            frame_type: frame_type.to_string(),
            client_id: Some(client_id.to_string()),
            server_id: None,
            conversation_id: Some(conversation_id.to_string()),
            segment_id: None,
            sequence: None,
            cursor: None,
            durability: ConversationGatewayDurability::Durable,
            scope: ConversationGatewayScope::Conversation,
            payload,
            occurred_at: "2026-05-09T00:00:00Z".to_string(),
        }
    }

    #[test]
    fn hello_frame_shape_is_stable() {
        let hello = hello_frame("session_1");

        assert_eq!(hello.schema_version, CONVERSATION_GATEWAY_SCHEMA_VERSION);
        assert_eq!(hello.op, ConversationGatewayOp::Hello);
        assert_eq!(hello.frame_type, "gateway.hello");
        assert_eq!(hello.payload["resumeSupported"], true);
    }

    #[test]
    fn malformed_and_unsupported_frames_return_structured_errors() {
        let (_temp_dir, db_path, _conversation_id, participant_id) = seeded_db();
        let mut session = session(participant_id);

        let malformed = handle_gateway_text_frame(&db_path, &mut session, "{");
        assert_eq!(malformed.frames[0].op, ConversationGatewayOp::Error);
        assert_eq!(malformed.frames[0].payload["code"], "invalid_envelope");

        let unsupported = handle_gateway_text_frame(
            &db_path,
            &mut session,
            r#"{"schemaVersion":"bad","op":"command","type":"message.submit","durability":"durable","scope":"conversation","payload":{},"occurredAt":"now"}"#,
        );
        assert_eq!(
            unsupported.frames[0].payload["code"],
            "unsupported_protocol_version"
        );

        let oversized = handle_gateway_text_frame(
            &db_path,
            &mut session,
            &"x".repeat(MAX_TEXT_FRAME_BYTES + 1),
        );
        assert_eq!(oversized.frames[0].op, ConversationGatewayOp::Error);
        assert_eq!(oversized.frames[0].payload["code"], "frame_too_large");
        assert_eq!(oversized.frames[0].payload["retryable"], false);
    }

    #[test]
    fn lagged_client_error_is_retryable_and_points_to_replay() {
        let frame = lagged_client_frame(7, "2026-05-09T00:00:00Z");

        assert_eq!(frame.op, ConversationGatewayOp::Error);
        assert_eq!(frame.frame_type, "command.rejected");
        assert_eq!(frame.payload["code"], "client_lagged");
        assert_eq!(frame.payload["retryable"], true);
        assert!(frame.payload["message"]
            .as_str()
            .unwrap()
            .contains("replay"));
    }

    #[test]
    fn submit_edit_delete_and_undo_return_ack_and_canonical_dispatch() {
        let (_temp_dir, db_path, conversation_id, participant_id) = seeded_db();
        let mut session = session(participant_id.clone());

        let submit = handle_gateway_envelope(
            &db_path,
            &mut session,
            command(
                "message.submit",
                "client_submit_1",
                &conversation_id,
                json!({
                    "bodyMarkdown": "hello",
                    "clientMessageId": "client_msg_1",
                    "undoExpiresAt": "2099-05-09T00:00:30Z"
                }),
            ),
        )
        .unwrap();
        assert_eq!(submit.frames[0].op, ConversationGatewayOp::Ack);
        assert_eq!(
            submit.frames[0].client_id.as_deref(),
            Some("client_submit_1")
        );
        assert_eq!(submit.broadcast[0].frame_type, "message.created");
        assert!(submit.broadcast[0].sequence.is_some());
        assert!(submit.broadcast[0].cursor.is_some());
        let message_id = submit.frames[0].payload["messageId"].as_str().unwrap();

        let edit = handle_gateway_envelope(
            &db_path,
            &mut session,
            command(
                "message.edit",
                "client_edit_1",
                &conversation_id,
                json!({
                    "messageId": message_id,
                    "bodyMarkdown": "edited",
                    "reason": "clarity"
                }),
            ),
        )
        .unwrap();
        assert_eq!(edit.broadcast[0].frame_type, "message.edited");

        let undo = handle_gateway_envelope(
            &db_path,
            &mut session,
            command(
                "message.undo",
                "client_undo_1",
                &conversation_id,
                json!({ "messageId": message_id }),
            ),
        )
        .unwrap();
        assert_eq!(undo.broadcast[0].frame_type, "message.undo.cancelled");

        let submit_delete = handle_gateway_envelope(
            &db_path,
            &mut session,
            command(
                "message.submit",
                "client_submit_2",
                &conversation_id,
                json!({
                    "bodyMarkdown": "delete me",
                    "clientMessageId": "client_msg_2"
                }),
            ),
        )
        .unwrap();
        let delete_message_id = submit_delete.frames[0].payload["messageId"]
            .as_str()
            .unwrap();
        let delete = handle_gateway_envelope(
            &db_path,
            &mut session,
            command(
                "message.delete",
                "client_delete_1",
                &conversation_id,
                json!({ "messageId": delete_message_id, "reason": "test" }),
            ),
        )
        .unwrap();
        assert_eq!(delete.broadcast[0].frame_type, "message.tombstoned");
    }

    #[test]
    fn subscribe_and_replay_return_ordered_events() {
        let (_temp_dir, db_path, conversation_id, participant_id) = seeded_db();
        let mut session = session(participant_id);
        let _submit = handle_gateway_envelope(
            &db_path,
            &mut session,
            command(
                "message.submit",
                "client_submit_1",
                &conversation_id,
                json!({ "bodyMarkdown": "hello", "clientMessageId": "client_msg_1" }),
            ),
        )
        .unwrap();

        let replay = handle_gateway_envelope(
            &db_path,
            &mut session,
            command(
                "conversation.replay_after_cursor",
                "client_replay_1",
                &conversation_id,
                json!({ "afterSequence": 0, "limit": 20 }),
            ),
        )
        .unwrap();

        assert!(!replay.frames.is_empty());
        assert!(replay
            .frames
            .windows(2)
            .all(|window| window[0].sequence <= window[1].sequence));

        let latest_sequence = replay
            .frames
            .last()
            .and_then(|frame| frame.sequence)
            .unwrap();
        let stale_replay = handle_gateway_envelope(
            &db_path,
            &mut session,
            command(
                "conversation.replay_after_cursor",
                "client_replay_stale",
                &conversation_id,
                json!({ "afterSequence": latest_sequence + 100, "limit": 20 }),
            ),
        )
        .unwrap();
        assert!(stale_replay.frames.is_empty());
    }

    #[test]
    fn duplicate_submit_retry_uses_canonical_message_without_duplicate_events() {
        let (_temp_dir, db_path, conversation_id, participant_id) = seeded_db();
        let mut session = session(participant_id);

        let first = handle_gateway_envelope(
            &db_path,
            &mut session,
            command(
                "message.submit",
                "client_submit_first",
                &conversation_id,
                json!({
                    "bodyMarkdown": "hello",
                    "clientMessageId": "client_msg_retry"
                }),
            ),
        )
        .unwrap();
        let retry = handle_gateway_envelope(
            &db_path,
            &mut session,
            command(
                "message.submit",
                "client_submit_retry",
                &conversation_id,
                json!({
                    "bodyMarkdown": "hello",
                    "clientMessageId": "client_msg_retry"
                }),
            ),
        )
        .unwrap();

        assert_eq!(
            first.frames[0].payload["messageId"],
            retry.frames[0].payload["messageId"]
        );
        assert_eq!(
            retry.frames[0].client_id.as_deref(),
            Some("client_submit_retry")
        );

        let connection = Connection::open(&db_path).unwrap();
        let message_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM conversation_messages WHERE client_message_id = 'client_msg_retry'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let created_event_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM conversation_events
                 WHERE conversation_id = ?1
                   AND event_type = 'message.created'
                   AND payload_json LIKE '%client_msg_retry%'",
                params![conversation_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(message_count, 1);
        assert_eq!(created_event_count, 1);
    }

    #[test]
    fn message_command_flood_returns_structured_rejection() {
        let (_temp_dir, db_path, conversation_id, participant_id) = seeded_db();
        let mut session = session(participant_id);

        for index in 0..COMMAND_RATE_LIMIT {
            let accepted = handle_gateway_envelope(
                &db_path,
                &mut session,
                command(
                    "message.submit",
                    &format!("client_submit_{index}"),
                    &conversation_id,
                    json!({
                        "bodyMarkdown": format!("message {index}"),
                        "clientMessageId": format!("client_msg_{index}")
                    }),
                ),
            )
            .unwrap();
            assert_eq!(accepted.frames[0].op, ConversationGatewayOp::Ack);
        }

        let rejected = handle_gateway_text_frame(
            &db_path,
            &mut session,
            &serde_json::to_string(&command(
                "message.submit",
                "client_submit_over_limit",
                &conversation_id,
                json!({
                    "bodyMarkdown": "too many",
                    "clientMessageId": "client_msg_over_limit"
                }),
            ))
            .unwrap(),
        );
        assert_eq!(rejected.frames[0].op, ConversationGatewayOp::Error);
        assert_eq!(rejected.frames[0].payload["code"], "rate_limited");
        assert_eq!(rejected.frames[0].payload["retryable"], true);
        assert_eq!(
            rejected.frames[0].client_id.as_deref(),
            Some("client_submit_over_limit")
        );
        assert!(rejected.frames[0].payload["message"]
            .as_str()
            .unwrap()
            .contains("rate limit"));
    }

    #[test]
    fn deterministic_llm_run_request_acknowledges_and_broadcasts_safe_evidence() {
        let (_temp_dir, db_path, conversation_id, participant_id) = seeded_db();
        let mut session = session(participant_id);
        let assistant_id = assistant_participant_id(&db_path, &conversation_id);

        let output = handle_gateway_envelope(
            &db_path,
            &mut session,
            command(
                "llm.run.request",
                "client_llm_1",
                &conversation_id,
                json!({
                    "runId": "llm_run_gateway_1",
                    "assistantParticipantId": assistant_id,
                    "providerId": "local_fake",
                    "modelId": "fake-chat",
                    "userMessage": "Please answer without exposing ada@example.com or sk-test-secret.",
                    "promptSlots": [{
                        "id": "conversation_brief",
                        "label": "Conversation Brief",
                        "content": "Client asked for a local deterministic reply.",
                        "sourceRefs": ["conversation_event_1"],
                        "inclusionReason": "Current conversation evidence.",
                        "visibilityCeiling": "participants",
                        "contentHash": "sha256:test"
                    }]
                }),
            ),
        )
        .unwrap();

        assert_eq!(output.frames[0].op, ConversationGatewayOp::Ack);
        assert_eq!(output.frames[0].frame_type, "llm.run.request.ack");
        assert_eq!(output.frames[0].payload["runId"], "llm_run_gateway_1");
        assert_eq!(output.frames[0].payload["providerId"], "local_fake");
        assert_eq!(output.frames[0].payload["modelId"], "fake-chat");
        assert!(output.frames[0].payload["finalMessageId"]
            .as_str()
            .is_some());

        let broadcast_types = output
            .broadcast
            .iter()
            .map(|frame| frame.frame_type.as_str())
            .collect::<Vec<_>>();
        assert!(broadcast_types.contains(&"llm.run.requested"));
        assert!(broadcast_types.contains(&"llm.prompt.compiled"));
        assert!(broadcast_types.contains(&"llm.prompt.slot.included"));
        assert!(broadcast_types.contains(&"llm.provider.started"));
        assert!(broadcast_types.contains(&"llm.text.delta"));
        assert!(broadcast_types.contains(&"llm.text.completed"));
        assert!(broadcast_types.contains(&"llm.usage.recorded"));
        assert!(broadcast_types.contains(&"llm.performance.measured"));
        assert!(broadcast_types.contains(&"llm.run.completed"));
        assert!(broadcast_types.contains(&"message.created"));
        let performance = output
            .broadcast
            .iter()
            .find(|frame| frame.frame_type == "llm.performance.measured")
            .unwrap();
        assert_eq!(performance.payload["providerId"], "local_fake");
        assert_eq!(performance.payload["modelId"], "fake-chat");
        assert!(performance.payload["deltaCount"].as_u64().unwrap() > 0);
        assert!(performance.payload["totalLatencyMs"].as_u64().is_some());

        let serialized = output
            .frames
            .iter()
            .chain(output.broadcast.iter())
            .map(|frame| serde_json::to_string(&frame.payload).unwrap())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(!serialized.contains("ada@example.com"));
        assert!(!serialized.contains("sk-test-secret"));
        assert!(!serialized.contains("Client asked for a local deterministic reply."));

        let connection = Connection::open(&db_path).unwrap();
        let assistant_messages: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM conversation_messages
                 WHERE conversation_id = ?1 AND message_kind = 'assistant' AND body_markdown = 'Drafting answer'",
                [&conversation_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(assistant_messages, 1);
    }

    #[test]
    fn llm_run_request_keeps_deterministic_default_when_live_env_is_present_without_app_guard() {
        let (_temp_dir, db_path, conversation_id, participant_id) = seeded_db();
        let session = session(participant_id);
        let assistant_id = assistant_participant_id(&db_path, &conversation_id);
        let env = HashMap::from([
            ("ORDO_LIVE_LLM_PROVIDER".to_string(), "openai".to_string()),
            ("ORDO_LIVE_LLM_MODEL".to_string(), "gpt-5".to_string()),
            ("ORDO_LIVE_LLM_EVALS".to_string(), "1".to_string()),
            ("ORDO_LIVE_LLM_ALLOW_NETWORK".to_string(), "1".to_string()),
            ("ORDO_LIVE_LLM_BUDGET_USD".to_string(), "0.01".to_string()),
            ("OPENAI_API_KEY".to_string(), "sk-openai-secret".to_string()),
        ]);

        let output = llm_run_request_with_openai_transport(
            &db_path,
            &session,
            command(
                "llm.run.request",
                "client_llm_default_guarded",
                &conversation_id,
                json!({
                    "runId": "llm_run_default_guarded",
                    "assistantParticipantId": assistant_id,
                    "providerId": "local_fake",
                    "modelId": "fake-chat",
                    "userMessage": "Please answer locally without leaking sk-openai-secret."
                }),
            ),
            &env,
            MockOpenAiTransport::success("This mocked live answer must not be used"),
        )
        .unwrap();

        assert_eq!(output.frames[0].op, ConversationGatewayOp::Ack);
        assert_eq!(output.frames[0].payload["providerId"], "local_fake");
        assert_eq!(output.frames[0].payload["modelId"], "fake-chat");
        let serialized = output
            .frames
            .iter()
            .chain(output.broadcast.iter())
            .map(|frame| serde_json::to_string(&frame.payload).unwrap())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(serialized.contains("local_fake"));
        assert!(!serialized.contains("sk-openai-secret"));
        assert!(!serialized.contains("This mocked live answer must not be used"));
    }

    #[test]
    fn llm_run_request_rejects_live_provider_mode_in_gateway_slice() {
        let (_temp_dir, db_path, conversation_id, participant_id) = seeded_db();
        let session = session(participant_id);
        let assistant_id = assistant_participant_id(&db_path, &conversation_id);
        let env = HashMap::from([
            ("ORDO_LIVE_LLM_PROVIDER".to_string(), "openai".to_string()),
            ("ORDO_LIVE_LLM_MODEL".to_string(), "gpt-5".to_string()),
            ("ORDO_LIVE_LLM_EVALS".to_string(), "1".to_string()),
            ("ORDO_LIVE_LLM_ALLOW_NETWORK".to_string(), "1".to_string()),
            ("ORDO_LIVE_LLM_BUDGET_USD".to_string(), "0.01".to_string()),
            ("OPENAI_API_KEY".to_string(), "sk-openai-secret".to_string()),
        ]);

        let rejected = llm_run_request_with_openai_transport(
            &db_path,
            &session,
            command(
                "llm.run.request",
                "client_llm_live",
                &conversation_id,
                json!({
                    "assistantParticipantId": assistant_id,
                    "providerId": "openai",
                    "modelId": "gpt-5",
                    "userMessage": "Please call a live provider using ava@example.com and sk-test-secret.",
                    "promptSlots": [{
                        "id": "live_prompt",
                        "label": "Live Prompt",
                        "content": "Raw prompt content must not leave this blocked request.",
                        "sourceRefs": ["conversation_event_1"],
                        "inclusionReason": "Attempted live provider readiness proof.",
                        "visibilityCeiling": "participants",
                        "contentHash": "sha256:test"
                    }]
                }),
            ),
            &env,
            MockOpenAiTransport::success("Mocked live answer"),
        )
        .unwrap();
        assert_eq!(rejected.frames[0].op, ConversationGatewayOp::Error);
        assert_eq!(rejected.frames[0].frame_type, "command.rejected");
        assert_eq!(
            rejected.frames[0].client_id.as_deref(),
            Some("client_llm_live")
        );
        assert_eq!(
            rejected.frames[0].conversation_id.as_deref(),
            Some(conversation_id.as_str())
        );
        assert_eq!(rejected.frames[0].payload["code"], "live_provider_disabled");
        assert_eq!(rejected.frames[0].payload["retryable"], false);
        assert!(rejected.frames[0].payload["message"]
            .as_str()
            .unwrap()
            .contains("Live provider mode is disabled"));
        assert!(rejected.broadcast.is_empty());

        let serialized = serde_json::to_string(&rejected.frames[0]).unwrap();
        assert!(!serialized.contains("ava@example.com"));
        assert!(!serialized.contains("sk-test-secret"));
        assert!(!serialized.contains("sk-openai-secret"));
        assert!(!serialized.contains("Raw prompt content"));
        assert!(!serialized.contains("OPENAI_API_KEY"));
    }

    #[test]
    fn llm_run_request_uses_mocked_openai_only_when_all_app_live_guards_are_satisfied() {
        let (_temp_dir, db_path, conversation_id, participant_id) = seeded_db();
        let session = session(participant_id);
        let assistant_id = assistant_participant_id(&db_path, &conversation_id);
        let env = HashMap::from([
            ("ORDO_APP_LIVE_LLM".to_string(), "1".to_string()),
            ("ORDO_LIVE_LLM_PROVIDER".to_string(), "openai".to_string()),
            ("ORDO_LIVE_LLM_MODEL".to_string(), "gpt-5".to_string()),
            ("ORDO_LIVE_LLM_EVALS".to_string(), "1".to_string()),
            ("ORDO_LIVE_LLM_ALLOW_NETWORK".to_string(), "1".to_string()),
            ("ORDO_LIVE_LLM_BUDGET_USD".to_string(), "0.01".to_string()),
            ("ORDO_LIVE_LLM_TIMEOUT_MS".to_string(), "30000".to_string()),
            ("ORDO_LIVE_LLM_MAX_CASES".to_string(), "1".to_string()),
            ("OPENAI_API_KEY".to_string(), "sk-openai-secret".to_string()),
        ]);

        let output = llm_run_request_with_openai_transport(
            &db_path,
            &session,
            command(
                "llm.run.request",
                "client_llm_openai_guarded",
                &conversation_id,
                json!({
                    "runId": "llm_run_openai_guarded",
                    "assistantParticipantId": assistant_id,
                    "providerId": "local_fake",
                    "modelId": "fake-chat",
                    "userMessage": "Please answer without exposing ava@example.com or sk-openai-secret.",
                    "promptSlots": [{
                        "id": "live_prompt",
                        "label": "Live Prompt",
                        "content": "Raw prompt content must not leak through frames.",
                        "sourceRefs": ["conversation_event_1"],
                        "inclusionReason": "Guarded app live provider proof.",
                        "visibilityCeiling": "participants",
                        "contentHash": "sha256:test"
                    }]
                }),
            ),
            &env,
            MockOpenAiTransport::success("Mocked live OpenAI answer"),
        )
        .unwrap();

        assert_eq!(output.frames[0].op, ConversationGatewayOp::Ack);
        assert_eq!(output.frames[0].payload["providerId"], "openai");
        assert_eq!(output.frames[0].payload["modelId"], "gpt-5");
        let broadcast_types = output
            .broadcast
            .iter()
            .map(|frame| frame.frame_type.as_str())
            .collect::<Vec<_>>();
        assert!(broadcast_types.contains(&"llm.provider.started"));
        assert!(broadcast_types.contains(&"llm.run.completed"));

        let connection = Connection::open(&db_path).unwrap();
        let assistant_messages: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM conversation_messages
                 WHERE conversation_id = ?1 AND message_kind = 'assistant' AND body_markdown = 'Mocked live OpenAI answer'",
                [&conversation_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(assistant_messages, 1);

        let serialized = output
            .frames
            .iter()
            .chain(output.broadcast.iter())
            .map(|frame| serde_json::to_string(&frame.payload).unwrap())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(!serialized.contains("sk-openai-secret"));
        assert!(!serialized.contains("ava@example.com"));
        assert!(!serialized.contains("Raw prompt content"));
        assert!(!serialized.contains("OPENAI_API_KEY"));
    }

    #[test]
    fn llm_run_request_allows_env_configured_provider_without_db_enablement() {
        let (_temp_dir, db_path, conversation_id, participant_id) = seeded_db();
        let session = session(participant_id);
        let assistant_id = assistant_participant_id(&db_path, &conversation_id);
        let env = HashMap::from([
            ("ORDO_APP_LIVE_LLM".to_string(), "1".to_string()),
            ("ORDO_LIVE_LLM_ALLOW_NETWORK".to_string(), "1".to_string()),
            ("ORDO_LIVE_LLM_BUDGET_USD".to_string(), "0.01".to_string()),
            ("OPENAI_API_KEY".to_string(), "sk-openai-secret".to_string()),
        ]);

        let output = llm_run_request_with_openai_transport(
            &db_path,
            &session,
            command(
                "llm.run.request",
                "client_llm_openai_env_configured",
                &conversation_id,
                json!({
                    "runId": "llm_run_openai_env_configured",
                    "assistantParticipantId": assistant_id,
                    "providerId": "openai",
                    "modelId": "gpt-5",
                    "userMessage": "Use the configured provider without leaking sk-openai-secret."
                }),
            ),
            &env,
            MockOpenAiTransport::success("Configured OpenAI answer"),
        )
        .unwrap();

        assert_eq!(output.frames[0].op, ConversationGatewayOp::Ack);
        assert_eq!(output.frames[0].payload["providerId"], "openai");
        assert_eq!(output.frames[0].payload["modelId"], "gpt-5");
        assert!(output
            .broadcast
            .iter()
            .any(|frame| frame.frame_type == "llm.run.completed"));
        assert!(!serde_json::to_string(&output.frames)
            .unwrap()
            .contains("sk-openai-secret"));
    }

    #[test]
    fn llm_run_cancel_acknowledges_and_broadcasts_canonical_cancel_event() {
        let (_temp_dir, db_path, conversation_id, participant_id) = seeded_db();
        let mut session = session(participant_id);

        let output = handle_gateway_envelope(
            &db_path,
            &mut session,
            command(
                "llm.run.cancel",
                "client_llm_cancel_1",
                &conversation_id,
                json!({ "runId": "llm_run_cancel_gateway_1" }),
            ),
        )
        .unwrap();

        assert_eq!(output.frames[0].op, ConversationGatewayOp::Ack);
        assert_eq!(output.frames[0].frame_type, "llm.run.cancel.ack");
        assert_eq!(
            output.frames[0].payload["runId"],
            "llm_run_cancel_gateway_1"
        );
        assert_eq!(output.broadcast[0].frame_type, "llm.run.cancelled");
    }

    #[test]
    fn llm_tool_commands_acknowledge_and_broadcast_governed_tool_events() {
        let (_temp_dir, db_path, conversation_id, participant_id) = seeded_db();
        let mut session = session(participant_id);
        let connection = Connection::open(&db_path).unwrap();
        let gateway = LlmGateway::new(DeterministicLlmProvider::new("local_fake", "fake-chat"));
        let requested = gateway
            .request_tool(
                &connection,
                &ActorContext::local_owner("test"),
                LlmToolRequestCreateRequest {
                    run_id: "llm_run_tool_gateway_1".to_string(),
                    conversation_id: conversation_id.clone(),
                    requested_capability_id: "system.status.read".to_string(),
                    requested_by: "assistant".to_string(),
                    reason: "Need current daemon status.".to_string(),
                    evidence_refs: vec!["conversation_event_1".to_string()],
                    input_summary: "Read system status.".to_string(),
                    visibility_ceiling: "participants".to_string(),
                    client_id: Some("client_tool_request_seed".to_string()),
                },
            )
            .unwrap()
            .tool_request
            .unwrap();

        let approved = handle_gateway_envelope(
            &db_path,
            &mut session,
            command(
                "tool.approve",
                "client_tool_approve_1",
                &conversation_id,
                json!({
                    "toolRequestId": requested.tool_request_id,
                    "reason": "Owner approved read-only status."
                }),
            ),
        )
        .unwrap();
        assert_eq!(approved.frames[0].frame_type, "tool.approve.ack");
        assert_eq!(approved.frames[0].payload["status"], "approved");
        assert_eq!(approved.broadcast[0].frame_type, "llm.tool.approved");

        let executed = handle_gateway_envelope(
            &db_path,
            &mut session,
            command(
                "tool.execute",
                "client_tool_execute_1",
                &conversation_id,
                json!({
                    "toolRequestId": requested.tool_request_id,
                    "outputSummary": "Daemon is ready."
                }),
            ),
        )
        .unwrap();
        assert_eq!(executed.frames[0].frame_type, "tool.execute.ack");
        assert_eq!(executed.frames[0].payload["status"], "completed");
        assert_eq!(executed.broadcast[0].frame_type, "llm.tool.executing");
        assert_eq!(executed.broadcast[1].frame_type, "llm.tool.completed");

        let second_requested = gateway
            .request_tool(
                &connection,
                &ActorContext::local_owner("test"),
                LlmToolRequestCreateRequest {
                    run_id: "llm_run_tool_gateway_2".to_string(),
                    conversation_id: conversation_id.clone(),
                    requested_capability_id: "system.status.read".to_string(),
                    requested_by: "assistant".to_string(),
                    reason: "Need current daemon status.".to_string(),
                    evidence_refs: vec!["conversation_event_2".to_string()],
                    input_summary: "Read system status.".to_string(),
                    visibility_ceiling: "participants".to_string(),
                    client_id: Some("client_tool_request_seed_2".to_string()),
                },
            )
            .unwrap()
            .tool_request
            .unwrap();
        let rejected = handle_gateway_envelope(
            &db_path,
            &mut session,
            command(
                "tool.reject",
                "client_tool_reject_1",
                &conversation_id,
                json!({
                    "toolRequestId": second_requested.tool_request_id,
                    "reason": "Not needed for this reply."
                }),
            ),
        )
        .unwrap();
        assert_eq!(rejected.frames[0].frame_type, "tool.reject.ack");
        assert_eq!(rejected.frames[0].payload["status"], "rejected");
        assert_eq!(rejected.broadcast[0].frame_type, "llm.tool.rejected");
    }

    #[test]
    fn unsupported_command_returns_structured_rejection_and_typing_is_ephemeral() {
        let (_temp_dir, db_path, conversation_id, participant_id) = seeded_db();
        let mut session = session(participant_id);

        let unsupported = handle_gateway_envelope(
            &db_path,
            &mut session,
            command(
                "unknown.command",
                "client_unknown_1",
                &conversation_id,
                json!({}),
            ),
        )
        .unwrap();
        assert_eq!(unsupported.frames[0].payload["code"], "unsupported_command");

        let typing = handle_gateway_envelope(
            &db_path,
            &mut session,
            command(
                "typing.start",
                "client_typing_1",
                &conversation_id,
                json!({}),
            ),
        )
        .unwrap();
        assert_eq!(typing.frames[0].op, ConversationGatewayOp::Ack);
        assert_eq!(typing.broadcast[0].frame_type, "typing.started");
        assert_eq!(
            typing.broadcast[0].durability,
            ConversationGatewayDurability::Ephemeral
        );
    }

    #[test]
    fn handoff_lifecycle_commands_ack_persist_and_replay_in_order() {
        let (_temp_dir, db_path, conversation_id, participant_id) = seeded_db();
        let mut session = session(participant_id);

        let create = handle_gateway_envelope(
            &db_path,
            &mut session,
            command(
                "conversation.handoff.create",
                "client_handoff_create",
                &conversation_id,
                json!({
                    "reason": "client needs a human decision",
                    "urgency": "high",
                    "evidenceSummary": "message msg_1 asked for staff follow-up",
                    "allowedContext": ["conversation", "latest_message"]
                }),
            ),
        )
        .unwrap();
        assert_eq!(create.frames[0].op, ConversationGatewayOp::Ack);
        assert_eq!(
            create.frames[0].client_id.as_deref(),
            Some("client_handoff_create")
        );
        assert_eq!(
            create.broadcast[0].frame_type,
            "conversation.handoff.requested"
        );
        let handoff_id = create.frames[0].payload["handoffId"].as_str().unwrap();

        let accept = handle_gateway_envelope(
            &db_path,
            &mut session,
            command(
                "conversation.handoff.accept",
                "client_handoff_accept",
                &conversation_id,
                json!({ "handoffId": handoff_id, "reason": "staff accepted" }),
            ),
        )
        .unwrap();
        assert_eq!(
            accept.frames[0].client_id.as_deref(),
            Some("client_handoff_accept")
        );
        assert_eq!(
            accept.broadcast[0].frame_type,
            "conversation.handoff.accepted"
        );

        let assign = handle_gateway_envelope(
            &db_path,
            &mut session,
            command(
                "handoff.assign",
                "client_handoff_assign",
                &conversation_id,
                json!({
                    "handoffId": handoff_id,
                    "assignedToActorId": "actor_staff",
                    "reason": "assign to current staff"
                }),
            ),
        )
        .unwrap();
        assert_eq!(
            assign.broadcast[0].frame_type,
            "conversation.handoff.assigned"
        );

        let returned = handle_gateway_envelope(
            &db_path,
            &mut session,
            command(
                "conversation.handoff.return_to_agent",
                "client_handoff_return",
                &conversation_id,
                json!({ "handoffId": handoff_id, "reason": "agent may resume" }),
            ),
        )
        .unwrap();
        assert_eq!(
            returned.broadcast[0].frame_type,
            "conversation.handoff.returned_to_agent"
        );

        let close = handle_gateway_envelope(
            &db_path,
            &mut session,
            command(
                "conversation.handoff.close",
                "client_handoff_close",
                &conversation_id,
                json!({ "handoffId": handoff_id, "reason": "complete" }),
            ),
        )
        .unwrap();
        assert_eq!(close.broadcast[0].frame_type, "conversation.handoff.closed");

        let replay = handle_gateway_envelope(
            &db_path,
            &mut session,
            command(
                "conversation.replay_after_cursor",
                "client_replay_handoff",
                &conversation_id,
                json!({ "afterSequence": 0, "limit": 50 }),
            ),
        )
        .unwrap();
        let durable_types = replay
            .frames
            .iter()
            .map(|frame| frame.frame_type.as_str())
            .collect::<Vec<_>>();
        let requested_index = durable_types
            .iter()
            .position(|event_type| *event_type == "conversation.handoff.requested")
            .unwrap();
        let closed_index = durable_types
            .iter()
            .position(|event_type| *event_type == "conversation.handoff.closed")
            .unwrap();
        assert!(requested_index < closed_index);
        assert!(durable_types.contains(&"conversation.handoff.accepted"));
        assert!(durable_types.contains(&"conversation.handoff.assigned"));
        assert!(durable_types.contains(&"conversation.handoff.returned_to_agent"));
    }

    #[test]
    fn mode_and_delegation_commands_are_durable_scoped_and_non_leaking() {
        let (_temp_dir, db_path, conversation_id, participant_id) = seeded_db();
        let mut session = session(participant_id);

        let human_led = handle_gateway_envelope(
            &db_path,
            &mut session,
            command(
                "conversation.mode.human_led_active",
                "client_mode_human",
                &conversation_id,
                json!({}),
            ),
        )
        .unwrap();
        assert_eq!(
            human_led.frames[0].client_id.as_deref(),
            Some("client_mode_human")
        );
        assert_eq!(
            human_led.broadcast[0].frame_type,
            "conversation.mode.changed"
        );
        assert_eq!(human_led.broadcast[0].payload["delegatedToAgent"], false);

        let delegate = handle_gateway_envelope(
            &db_path,
            &mut session,
            command(
                "conversation.agent.delegate",
                "client_agent_delegate",
                &conversation_id,
                json!({ "delegationScope": ["draft_reply"], "reason": "staff tagged Ordo" }),
            ),
        )
        .unwrap();
        assert_eq!(
            delegate.frames[0].client_id.as_deref(),
            Some("client_agent_delegate")
        );
        assert_eq!(delegate.broadcast[0].payload["delegatedToAgent"], true);
        assert_eq!(
            delegate.broadcast[0].payload["delegationScope"][0],
            "draft_reply"
        );
        let dispatch_json = serde_json::to_string(&delegate.broadcast[0].payload).unwrap();
        assert!(!dispatch_json.contains("provider"));
        assert!(!dispatch_json.contains("privacyTransform"));
        assert!(!dispatch_json.contains("policyDecision"));

        let revoke = handle_gateway_envelope(
            &db_path,
            &mut session,
            command(
                "conversation.agent.delegation_revoke",
                "client_agent_revoke",
                &conversation_id,
                json!({ "reason": "staff resumed manually" }),
            ),
        )
        .unwrap();
        assert_eq!(revoke.broadcast[0].payload["delegatedToAgent"], false);

        let returned = handle_gateway_envelope(
            &db_path,
            &mut session,
            command(
                "conversation.mode.return_to_agent",
                "client_mode_return",
                &conversation_id,
                json!({}),
            ),
        )
        .unwrap();
        assert_eq!(returned.broadcast[0].payload["mode"], "returned_to_agent");

        let rejected = handle_gateway_text_frame(
            &db_path,
            &mut session,
            &serde_json::to_string(&command(
                "conversation.agent.delegate",
                "client_agent_delegate_bad",
                &conversation_id,
                json!({}),
            ))
            .unwrap(),
        );
        assert_eq!(rejected.frames[0].op, ConversationGatewayOp::Error);
        assert_eq!(rejected.frames[0].payload["code"], "command_failed");
        assert!(rejected.frames[0].payload["message"]
            .as_str()
            .unwrap()
            .contains("delegationScope"));

        let replay = handle_gateway_envelope(
            &db_path,
            &mut session,
            command(
                "conversation.replay_after_cursor",
                "client_replay_mode",
                &conversation_id,
                json!({ "afterSequence": 0, "limit": 50 }),
            ),
        )
        .unwrap();
        assert!(
            replay
                .frames
                .iter()
                .filter(|frame| frame.frame_type == "conversation.mode.changed")
                .count()
                >= 4
        );
    }

    #[test]
    fn receipt_reaction_and_presence_commands_return_ack_and_expected_dispatch() {
        let (_temp_dir, db_path, conversation_id, participant_id) = seeded_db();
        let mut session = session(participant_id);

        let submit = handle_gateway_envelope(
            &db_path,
            &mut session,
            command(
                "message.submit",
                "client_submit_1",
                &conversation_id,
                json!({ "bodyMarkdown": "hello", "clientMessageId": "client_msg_1" }),
            ),
        )
        .unwrap();
        let message_id = submit.frames[0].payload["messageId"].as_str().unwrap();

        let read = handle_gateway_envelope(
            &db_path,
            &mut session,
            command(
                "message.mark_read",
                "client_read_1",
                &conversation_id,
                json!({ "messageId": message_id }),
            ),
        )
        .unwrap();
        assert_eq!(read.frames[0].client_id.as_deref(), Some("client_read_1"));
        assert_eq!(read.broadcast[0].frame_type, "message.read");

        let reaction = handle_gateway_envelope(
            &db_path,
            &mut session,
            command(
                "message.react",
                "client_react_1",
                &conversation_id,
                json!({
                    "messageId": message_id,
                    "reactionKey": "heart",
                    "reactionKind": "emoji",
                    "action": "add"
                }),
            ),
        )
        .unwrap();
        assert_eq!(
            reaction.frames[0].client_id.as_deref(),
            Some("client_react_1")
        );
        assert_eq!(reaction.broadcast[0].frame_type, "message.reaction.added");

        let before_messages: i64 = Connection::open(&db_path)
            .unwrap()
            .query_row("SELECT COUNT(*) FROM conversation_messages", [], |row| {
                row.get(0)
            })
            .unwrap();
        let presence = handle_gateway_envelope(
            &db_path,
            &mut session,
            command(
                "presence.update",
                "client_presence_1",
                &conversation_id,
                json!({ "status": "online", "visibility": "participants" }),
            ),
        )
        .unwrap();
        assert_eq!(
            presence.frames[0].client_id.as_deref(),
            Some("client_presence_1")
        );
        assert_eq!(presence.broadcast[0].frame_type, "presence.changed");
        assert_eq!(
            presence.broadcast[0].durability,
            ConversationGatewayDurability::Ephemeral
        );
        let after_messages: i64 = Connection::open(&db_path)
            .unwrap()
            .query_row("SELECT COUNT(*) FROM conversation_messages", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(before_messages, after_messages);

        let replay = handle_gateway_envelope(
            &db_path,
            &mut session,
            command(
                "conversation.replay_after_cursor",
                "client_replay_2",
                &conversation_id,
                json!({ "afterSequence": 0, "limit": 20 }),
            ),
        )
        .unwrap();
        let durable_types = replay
            .frames
            .iter()
            .map(|frame| frame.frame_type.as_str())
            .collect::<Vec<_>>();
        assert!(durable_types.contains(&"message.read"));
        assert!(durable_types.contains(&"message.reaction.added"));
        assert!(!durable_types.contains(&"presence.changed"));
    }
}
