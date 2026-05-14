use super::*;
use crate::capabilities::*;
use crate::conversation_protocol::*;
use crate::conversations::*;
use crate::events::*;
use crate::llm_accounting::*;
use crate::policy::*;
use crate::privacy_egress::*;
use anyhow::{ensure, Result};
use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::path::Path;
use std::time::Instant;
use uuid::Uuid;

pub struct LlmGateway<P> {
    pub(crate) provider: P,
    invoke_policy: LlmPolicy,
    cancel_policy: LlmPolicy,
    privacy_firewall: PrivacyEgressFirewall,
}
pub(crate) struct LlmToolTransition<'a> {
    next_status: LlmToolRequestStatus,
    capability_id: &'a str,
    action: PolicyAction,
    reason: &'a str,
}

#[derive(Debug, Clone)]
pub(crate) struct ProviderPrivacyPayload {
    prompt: CompiledPrompt,
    user_message: String,
    transforms: Vec<PrivacyEgressTransform>,
}

impl<P: LlmProviderAdapter> LlmGateway<P> {
    pub fn new(provider: P) -> Self {
        Self {
            provider,
            invoke_policy: LlmPolicy::allow("LLM invocation allowed by daemon gateway policy."),
            cancel_policy: LlmPolicy::allow("LLM cancellation allowed by daemon gateway policy."),
            privacy_firewall: PrivacyEgressFirewall::default(),
        }
    }

    pub fn with_policies(provider: P, invoke_policy: LlmPolicy, cancel_policy: LlmPolicy) -> Self {
        Self {
            provider,
            invoke_policy,
            cancel_policy,
            privacy_firewall: PrivacyEgressFirewall::default(),
        }
    }

    pub fn with_private_terms(mut self, private_terms: Vec<String>) -> Self {
        self.privacy_firewall = PrivacyEgressFirewall::new(private_terms);
        self
    }

    pub fn run_completion(
        &self,
        db_path: &Path,
        connection: &Connection,
        actor: &ActorContext,
        request: LlmGatewayRequest,
    ) -> Result<LlmGatewayRunResult> {
        let request_received_instant = Instant::now();
        let request_received_at = Utc::now().to_rfc3339();
        validate_request(&request)?;
        ensure!(
            request.provider_id == self.provider.provider_id(),
            "provider id does not match adapter"
        );
        ensure!(
            request.model_id == self.provider.model_id(),
            "model id does not match adapter"
        );
        ensure_capability_registered(connection, LLM_INVOKE_CAPABILITY_ID)?;
        let policy_decision_id = record_llm_policy_decision(
            connection,
            actor,
            PolicyAction::Generate,
            ResourceKind::LlmRun,
            &request.run_id,
            LLM_INVOKE_CAPABILITY_ID,
            &self.invoke_policy,
        )?;
        let now = Utc::now().to_rfc3339();
        if !matches!(self.invoke_policy.outcome, PolicyOutcome::Allowed) {
            return Ok(LlmGatewayRunResult {
                run_id: request.run_id,
                policy_decision_id: policy_decision_id.clone(),
                prompt: None,
                final_message: None,
                frames: vec![policy_denied_error(
                    request.client_id.as_deref().unwrap_or("llm_run"),
                    Some(&policy_decision_id),
                    "LLM invocation denied by daemon policy.",
                    &now,
                )],
            });
        }

        let prompt = compile_prompt(&request.prompt_slots)?;
        record_invocation_started(connection, &request, &prompt, &policy_decision_id)?;
        let mut frames = Vec::new();
        let provider_request_started_at = Utc::now().to_rfc3339();
        let provider_request_started_instant = Instant::now();
        frames.push(persist_dispatch(
            connection,
            &request.conversation_id,
            request.segment_id.as_deref(),
            "llm.run.requested",
            json!({
                "runId": request.run_id,
                "providerId": request.provider_id,
                "modelId": request.model_id,
            }),
            Some(&policy_decision_id),
        )?);
        frames.push(persist_dispatch(
            connection,
            &request.conversation_id,
            request.segment_id.as_deref(),
            "llm.prompt.compiled",
            json!({
                "runId": request.run_id,
                "promptId": prompt.prompt_id,
                "promptHash": prompt.prompt_hash,
                "slotCount": prompt.slots.len(),
            }),
            Some(&policy_decision_id),
        )?);
        for slot in &prompt.slots {
            frames.push(persist_dispatch(
                connection,
                &request.conversation_id,
                request.segment_id.as_deref(),
                "llm.prompt.slot.included",
                json!({
                    "runId": request.run_id,
                    "slotId": slot.id,
                    "label": slot.label,
                    "sourceRefs": slot.source_refs,
                    "inclusionReason": slot.inclusion_reason,
                    "visibilityCeiling": slot.visibility_ceiling,
                    "contentHash": slot.content_hash,
                }),
                Some(&policy_decision_id),
            )?);
        }
        frames.extend(
            record_prompt_slot_usage(connection, &request, &prompt, &policy_decision_id)?
                .into_iter()
                .map(|event| dispatch_from_event(&request.conversation_id, &event)),
        );
        let privacy = match transform_provider_payload(
            &self.privacy_firewall,
            db_path,
            connection,
            &request,
            &prompt,
        ) {
            Ok(privacy) => privacy,
            Err(error) => {
                record_invocation_failed(
                    connection,
                    &request.run_id,
                    "privacy_transform_failed",
                    &error.to_string(),
                )?;
                frames.push(persist_dispatch(
                    connection,
                    &request.conversation_id,
                    request.segment_id.as_deref(),
                    "privacy.egress.blocked",
                    json!({
                        "runId": request.run_id,
                        "reason": "Provider-bound payload failed privacy egress transform.",
                        "errorHash": stable_content_hash(&error.to_string()),
                    }),
                    Some(&policy_decision_id),
                )?);
                frames.push(persist_dispatch(
                    connection,
                    &request.conversation_id,
                    request.segment_id.as_deref(),
                    "llm.run.failed",
                    json!({
                        "runId": request.run_id,
                        "code": "privacy_transform_failed",
                        "message": "Provider-bound payload failed privacy egress transform.",
                    }),
                    Some(&policy_decision_id),
                )?);
                return Ok(LlmGatewayRunResult {
                    run_id: request.run_id,
                    policy_decision_id,
                    prompt: Some(prompt),
                    final_message: None,
                    frames,
                });
            }
        };
        record_privacy_transform_runs(
            connection,
            &request.run_id,
            &privacy
                .transforms
                .iter()
                .map(|transform| transform.transform_run_id.clone())
                .collect::<Vec<_>>(),
        )?;
        for transform in &privacy.transforms {
            frames.push(persist_dispatch(
                connection,
                &request.conversation_id,
                request.segment_id.as_deref(),
                "privacy.egress.transformed",
                privacy_transform_event_payload(&request.run_id, transform),
                Some(&policy_decision_id),
            )?);
        }
        frames.push(persist_dispatch(
            connection,
            &request.conversation_id,
            request.segment_id.as_deref(),
            "llm.provider.started",
            json!({
                "runId": request.run_id,
                "providerId": request.provider_id,
                "modelId": request.model_id,
            }),
            Some(&policy_decision_id),
        )?);

        let provider_request = LlmProviderRequest {
            run_id: request.run_id.clone(),
            provider_id: request.provider_id.clone(),
            model_id: request.model_id.clone(),
            prompt: privacy.prompt.clone(),
            user_message: privacy.user_message.clone(),
        };
        let stream = self.provider.stream(&provider_request)?;
        let mut completed_text = None;
        let mut usage = None;
        let mut first_delta_at = None;
        let mut first_delta_ms = None;
        let mut final_delta_at = None;
        let mut delta_count: u64 = 0;
        let mut approximate_output_chars: u64 = 0;
        for event in stream {
            match event {
                LlmProviderStreamEvent::TextDelta(delta) => {
                    let delta_at = Utc::now().to_rfc3339();
                    if first_delta_at.is_none() {
                        first_delta_at = Some(delta_at.clone());
                        first_delta_ms =
                            Some(provider_request_started_instant.elapsed().as_millis() as u64);
                    }
                    final_delta_at = Some(delta_at);
                    delta_count += 1;
                    approximate_output_chars += delta.chars().count() as u64;
                    frames.push(ephemeral_run_dispatch(
                        "llm.text.delta",
                        &request,
                        json!({
                            "runId": request.run_id,
                            "delta": delta,
                        }),
                    ));
                }
                LlmProviderStreamEvent::Completed {
                    text,
                    usage: event_usage,
                } => {
                    completed_text = Some(text);
                    usage = Some(event_usage);
                }
                LlmProviderStreamEvent::Failed { code, message } => {
                    record_invocation_failed(connection, &request.run_id, &code, &message)?;
                    let completed_at = Utc::now().to_rfc3339();
                    let total_latency_ms = request_received_instant.elapsed().as_millis() as u64;
                    let provider_latency_ms =
                        provider_request_started_instant.elapsed().as_millis() as u64;
                    frames.push(persist_dispatch(
                        connection,
                        &request.conversation_id,
                        request.segment_id.as_deref(),
                        "llm.run.failed",
                        json!({
                            "runId": request.run_id,
                            "code": code,
                            "message": message,
                        }),
                        Some(&policy_decision_id),
                    )?);
                    frames.push(persist_dispatch(
                        connection,
                        &request.conversation_id,
                        request.segment_id.as_deref(),
                        "llm.performance.measured",
                        json!({
                            "runId": request.run_id,
                            "providerId": request.provider_id,
                            "modelId": request.model_id,
                            "requestReceivedAt": request_received_at,
                            "providerRequestStartedAt": provider_request_started_at,
                            "firstDeltaAt": first_delta_at,
                            "finalDeltaAt": final_delta_at,
                            "completedAt": completed_at,
                            "timeToFirstTokenMs": first_delta_ms,
                            "providerLatencyMs": provider_latency_ms,
                            "totalLatencyMs": total_latency_ms,
                            "deltaCount": delta_count,
                            "approximateOutputChars": approximate_output_chars,
                            "charsPerSecond": null,
                            "status": "failed",
                            "failureCode": code,
                            "failureMessage": message,
                        }),
                        Some(&policy_decision_id),
                    )?);
                    return Ok(LlmGatewayRunResult {
                        run_id: request.run_id,
                        policy_decision_id,
                        prompt: Some(prompt),
                        final_message: None,
                        frames,
                    });
                }
            }
        }

        let completed_text = reconstruct_provider_text(
            db_path,
            connection,
            &privacy.transforms,
            completed_text.unwrap_or_default(),
        )?;
        require_text("completed assistant text", &completed_text)?;
        let final_message = create_conversation_message(
            connection,
            &ConversationMessageCreateRequest {
                conversation_id: request.conversation_id.clone(),
                segment_id: request.segment_id.clone(),
                participant_id: request.assistant_participant_id.clone(),
                message_kind: "assistant".to_string(),
                body_markdown: completed_text.clone(),
                visibility: "participants".to_string(),
                client_message_id: format!("llm:{}:assistant", request.run_id),
                reply_to_message_id: None,
                undo_expires_at: None,
            },
        )?;
        frames.push(persist_dispatch(
            connection,
            &request.conversation_id,
            request.segment_id.as_deref(),
            "llm.text.completed",
            json!({
                "runId": request.run_id,
                "messageId": final_message.id,
                "contentHash": stable_content_hash(&completed_text),
            }),
            Some(&policy_decision_id),
        )?);
        if let Some(usage) = usage {
            frames.extend(
                record_invocation_completed(
                    connection,
                    &request,
                    Some(&usage),
                    &policy_decision_id,
                )?
                .into_iter()
                .map(|event| dispatch_from_event(&request.conversation_id, &event)),
            );
            frames.push(persist_dispatch(
                connection,
                &request.conversation_id,
                request.segment_id.as_deref(),
                "llm.usage.recorded",
                json!({
                    "runId": request.run_id,
                    "usage": usage,
                }),
                Some(&policy_decision_id),
            )?);
        } else {
            frames.extend(
                record_invocation_completed(connection, &request, None, &policy_decision_id)?
                    .into_iter()
                    .map(|event| dispatch_from_event(&request.conversation_id, &event)),
            );
        }
        let completed_at = Utc::now().to_rfc3339();
        let total_latency_ms = request_received_instant.elapsed().as_millis() as u64;
        let provider_latency_ms = provider_request_started_instant.elapsed().as_millis() as u64;
        let chars_per_second = if total_latency_ms > 0 {
            Some((approximate_output_chars as f64 / total_latency_ms as f64) * 1000.0)
        } else {
            None
        };
        frames.push(persist_dispatch(
            connection,
            &request.conversation_id,
            request.segment_id.as_deref(),
            "llm.performance.measured",
            json!({
                "runId": request.run_id,
                "providerId": request.provider_id,
                "modelId": request.model_id,
                "requestReceivedAt": request_received_at,
                "providerRequestStartedAt": provider_request_started_at,
                "firstDeltaAt": first_delta_at,
                "finalDeltaAt": final_delta_at,
                "completedAt": completed_at,
                "timeToFirstTokenMs": first_delta_ms,
                "providerLatencyMs": provider_latency_ms,
                "totalLatencyMs": total_latency_ms,
                "deltaCount": delta_count,
                "approximateOutputChars": approximate_output_chars,
                "charsPerSecond": chars_per_second,
                "status": "succeeded",
            }),
            Some(&policy_decision_id),
        )?);
        frames.push(persist_dispatch(
            connection,
            &request.conversation_id,
            request.segment_id.as_deref(),
            "llm.run.completed",
            json!({
                "runId": request.run_id,
                "messageId": final_message.id,
                "providerId": request.provider_id,
                "modelId": request.model_id,
                "totalLatencyMs": total_latency_ms,
                "timeToFirstTokenMs": first_delta_ms,
            }),
            Some(&policy_decision_id),
        )?);

        Ok(LlmGatewayRunResult {
            run_id: request.run_id,
            policy_decision_id,
            prompt: Some(prompt),
            final_message: Some(final_message),
            frames,
        })
    }

    pub fn cancel_run(
        &self,
        connection: &Connection,
        actor: &ActorContext,
        conversation_id: &str,
        run_id: &str,
        client_id: Option<&str>,
    ) -> Result<LlmGatewayRunResult> {
        require_text("conversation_id", conversation_id)?;
        require_text("run_id", run_id)?;
        ensure_capability_registered(connection, LLM_CANCEL_CAPABILITY_ID)?;
        let policy_decision_id = record_llm_policy_decision(
            connection,
            actor,
            PolicyAction::Update,
            ResourceKind::LlmRun,
            run_id,
            LLM_CANCEL_CAPABILITY_ID,
            &self.cancel_policy,
        )?;
        let now = Utc::now().to_rfc3339();
        if !matches!(self.cancel_policy.outcome, PolicyOutcome::Allowed) {
            return Ok(LlmGatewayRunResult {
                run_id: run_id.to_string(),
                policy_decision_id: policy_decision_id.clone(),
                prompt: None,
                final_message: None,
                frames: vec![policy_denied_error(
                    client_id.unwrap_or("llm_cancel"),
                    Some(&policy_decision_id),
                    "LLM cancellation denied by daemon policy.",
                    &now,
                )],
            });
        }

        self.provider.cancel(run_id)?;
        let frame = persist_dispatch(
            connection,
            conversation_id,
            None,
            "llm.run.cancelled",
            json!({ "runId": run_id }),
            Some(&policy_decision_id),
        )?;
        Ok(LlmGatewayRunResult {
            run_id: run_id.to_string(),
            policy_decision_id,
            prompt: None,
            final_message: None,
            frames: vec![frame],
        })
    }

    pub fn request_tool(
        &self,
        connection: &Connection,
        actor: &ActorContext,
        request: LlmToolRequestCreateRequest,
    ) -> Result<LlmToolRequestReceipt> {
        validate_tool_request(&request)?;
        ensure_capability_registered(connection, LLM_TOOL_REQUEST_CAPABILITY_ID)?;
        let now = Utc::now().to_rfc3339();
        let policy_decision_id = record_llm_policy_decision(
            connection,
            actor,
            PolicyAction::Create,
            ResourceKind::LlmRun,
            &request.run_id,
            LLM_TOOL_REQUEST_CAPABILITY_ID,
            &LlmPolicy::allow("LLM tool request recorded for governed review."),
        )?;
        let requested_capability =
            match load_supported_tool_capability(connection, &request.requested_capability_id) {
                Ok(capability) => capability,
                Err(error) => {
                    return Ok(LlmToolRequestReceipt {
                        tool_request: None,
                        policy_decision_id: Some(policy_decision_id),
                        frames: vec![command_rejected_error(
                            request.client_id.as_deref(),
                            Some(&request.conversation_id),
                            "unsupported_command",
                            &format!("LLM tool request rejected: {error}"),
                            false,
                            &now,
                        )],
                    });
                }
            };

        let tool_request = LlmToolRequestView {
            tool_request_id: format!("llm_tool_request_{}", Uuid::new_v4()),
            run_id: request.run_id,
            conversation_id: request.conversation_id,
            requested_capability_id: requested_capability.id,
            requested_by: request.requested_by,
            approval_actor_id: None,
            reason: request.reason,
            evidence_refs: request.evidence_refs,
            input_summary: request.input_summary,
            visibility_ceiling: request.visibility_ceiling,
            status: LlmToolRequestStatus::Requested,
            policy_decision_id: Some(policy_decision_id.clone()),
            created_at: now.clone(),
            updated_at: now,
        };
        let frame = persist_tool_request_event(connection, &tool_request, None)?;
        Ok(LlmToolRequestReceipt {
            tool_request: Some(tool_request),
            policy_decision_id: Some(policy_decision_id),
            frames: vec![frame],
        })
    }

    pub fn approve_tool_request(
        &self,
        connection: &Connection,
        actor: &ActorContext,
        conversation_id: &str,
        tool_request_id: &str,
        reason: &str,
    ) -> Result<LlmToolRequestReceipt> {
        self.transition_tool_request(
            connection,
            actor,
            conversation_id,
            tool_request_id,
            LlmToolTransition {
                next_status: LlmToolRequestStatus::Approved,
                capability_id: LLM_TOOL_APPROVE_CAPABILITY_ID,
                action: PolicyAction::Approve,
                reason,
            },
        )
    }

    pub fn reject_tool_request(
        &self,
        connection: &Connection,
        actor: &ActorContext,
        conversation_id: &str,
        tool_request_id: &str,
        reason: &str,
    ) -> Result<LlmToolRequestReceipt> {
        self.transition_tool_request(
            connection,
            actor,
            conversation_id,
            tool_request_id,
            LlmToolTransition {
                next_status: LlmToolRequestStatus::Rejected,
                capability_id: LLM_TOOL_REJECT_CAPABILITY_ID,
                action: PolicyAction::Approve,
                reason,
            },
        )
    }

    pub fn execute_approved_tool_request(
        &self,
        connection: &Connection,
        actor: &ActorContext,
        conversation_id: &str,
        tool_request_id: &str,
        output_summary: &str,
    ) -> Result<LlmToolRequestReceipt> {
        require_text("output_summary", output_summary)?;
        ensure_capability_registered(connection, LLM_TOOL_EXECUTE_CAPABILITY_ID)?;
        let mut current = load_tool_request_state(connection, conversation_id, tool_request_id)?;
        let now = Utc::now().to_rfc3339();
        if current.status != LlmToolRequestStatus::Approved {
            return Ok(LlmToolRequestReceipt {
                tool_request: Some(current),
                policy_decision_id: None,
                frames: vec![command_rejected_error(
                    Some(tool_request_id),
                    Some(conversation_id),
                    "review_required",
                    "LLM tool execution requires an approved tool request.",
                    false,
                    &now,
                )],
            });
        }
        load_supported_tool_capability(connection, &current.requested_capability_id)?;
        let policy_decision_id = record_llm_policy_decision(
            connection,
            actor,
            PolicyAction::Execute,
            ResourceKind::Capability,
            &current.requested_capability_id,
            LLM_TOOL_EXECUTE_CAPABILITY_ID,
            &LlmPolicy::allow("Approved LLM tool request executed through daemon policy."),
        )?;

        current.status = LlmToolRequestStatus::Executing;
        current.updated_at = now.clone();
        current.policy_decision_id = Some(policy_decision_id.clone());
        let executing = persist_tool_request_event(connection, &current, None)?;

        current.status = LlmToolRequestStatus::Completed;
        current.updated_at = Utc::now().to_rfc3339();
        let completed = persist_tool_request_event(
            connection,
            &current,
            Some(json!({ "outputSummary": output_summary })),
        )?;

        Ok(LlmToolRequestReceipt {
            tool_request: Some(current),
            policy_decision_id: Some(policy_decision_id),
            frames: vec![executing, completed],
        })
    }

    pub fn fail_approved_tool_request(
        &self,
        connection: &Connection,
        actor: &ActorContext,
        conversation_id: &str,
        tool_request_id: &str,
        failure_code: &str,
        failure_message: &str,
    ) -> Result<LlmToolRequestReceipt> {
        require_text("failure_code", failure_code)?;
        require_text("failure_message", failure_message)?;
        ensure_capability_registered(connection, LLM_TOOL_EXECUTE_CAPABILITY_ID)?;
        let mut current = load_tool_request_state(connection, conversation_id, tool_request_id)?;
        let now = Utc::now().to_rfc3339();
        if current.status != LlmToolRequestStatus::Approved {
            return Ok(LlmToolRequestReceipt {
                tool_request: Some(current),
                policy_decision_id: None,
                frames: vec![command_rejected_error(
                    Some(tool_request_id),
                    Some(conversation_id),
                    "review_required",
                    "LLM tool failure recording requires an approved tool request.",
                    false,
                    &now,
                )],
            });
        }
        let policy_decision_id = record_llm_policy_decision(
            connection,
            actor,
            PolicyAction::Execute,
            ResourceKind::Capability,
            &current.requested_capability_id,
            LLM_TOOL_EXECUTE_CAPABILITY_ID,
            &LlmPolicy::allow("Approved LLM tool request failed through daemon policy."),
        )?;
        current.status = LlmToolRequestStatus::Failed;
        current.updated_at = now;
        current.policy_decision_id = Some(policy_decision_id.clone());
        let frame = persist_tool_request_event(
            connection,
            &current,
            Some(json!({
                "failureCode": failure_code,
                "failureMessage": failure_message,
            })),
        )?;
        Ok(LlmToolRequestReceipt {
            tool_request: Some(current),
            policy_decision_id: Some(policy_decision_id),
            frames: vec![frame],
        })
    }
    pub(crate) fn transition_tool_request(
        &self,
        connection: &Connection,
        actor: &ActorContext,
        conversation_id: &str,
        tool_request_id: &str,
        transition: LlmToolTransition<'_>,
    ) -> Result<LlmToolRequestReceipt> {
        require_text("reason", transition.reason)?;
        ensure_capability_registered(connection, transition.capability_id)?;
        let mut current = load_tool_request_state(connection, conversation_id, tool_request_id)?;
        ensure!(
            current.status == LlmToolRequestStatus::Requested,
            "LLM tool request must be requested before approval or rejection"
        );
        let policy_decision_id = record_llm_policy_decision(
            connection,
            actor,
            transition.action,
            ResourceKind::LlmRun,
            &current.run_id,
            transition.capability_id,
            &LlmPolicy::allow(transition.reason),
        )?;
        current.status = transition.next_status;
        current.updated_at = Utc::now().to_rfc3339();
        current.approval_actor_id = actor.id.clone();
        current.reason = transition.reason.to_string();
        current.policy_decision_id = Some(policy_decision_id.clone());
        let frame = persist_tool_request_event(connection, &current, None)?;
        Ok(LlmToolRequestReceipt {
            tool_request: Some(current),
            policy_decision_id: Some(policy_decision_id),
            frames: vec![frame],
        })
    }
}

pub fn compile_prompt(slots: &[PromptSlot]) -> Result<CompiledPrompt> {
    ensure!(!slots.is_empty(), "at least one prompt slot is required");
    for slot in slots {
        require_text("prompt slot id", &slot.id)?;
        require_text("prompt slot content", &slot.content)?;
        ensure!(
            !slot.source_refs.is_empty(),
            "prompt slot source refs are required"
        );
        require_text("prompt slot inclusion reason", &slot.inclusion_reason)?;
        require_text("prompt slot visibility ceiling", &slot.visibility_ceiling)?;
    }
    let mut hasher = Sha256::new();
    for slot in slots {
        hasher.update(slot.id.as_bytes());
        hasher.update(b"\0");
        hasher.update(slot.content_hash.as_bytes());
        hasher.update(b"\0");
    }
    let prompt_hash = format!("sha256:{:x}", hasher.finalize());
    Ok(CompiledPrompt {
        prompt_id: format!("prompt_{}", Uuid::new_v4()),
        prompt_hash,
        slots: slots.to_vec(),
    })
}
pub(crate) fn transform_provider_payload(
    firewall: &PrivacyEgressFirewall,
    db_path: &Path,
    connection: &Connection,
    request: &LlmGatewayRequest,
    prompt: &CompiledPrompt,
) -> Result<ProviderPrivacyPayload> {
    let scope = PrivacyEgressScope {
        scope_kind: "llm_run".to_string(),
        scope_id: request.run_id.clone(),
    };
    let user_transform =
        firewall.transform_payload(db_path, connection, scope.clone(), &request.user_message)?;
    let mut transformed_slots = Vec::new();
    let mut transforms = vec![user_transform.clone()];
    for slot in &prompt.slots {
        let transform =
            firewall.transform_payload(db_path, connection, scope.clone(), &slot.content)?;
        let mut transformed_slot = slot.clone();
        transformed_slot.content = transform.transformed_payload.clone();
        transformed_slot.content_hash = stable_content_hash(&transformed_slot.content);
        transformed_slots.push(transformed_slot);
        transforms.push(transform);
    }
    let transformed_prompt = compile_prompt(&transformed_slots)?;
    Ok(ProviderPrivacyPayload {
        prompt: transformed_prompt,
        user_message: user_transform.transformed_payload,
        transforms,
    })
}
pub(crate) fn reconstruct_provider_text(
    db_path: &Path,
    connection: &Connection,
    transforms: &[PrivacyEgressTransform],
    mut payload: String,
) -> Result<String> {
    for transform in transforms {
        if transform
            .findings
            .iter()
            .any(|finding| payload.contains(&finding.placeholder))
        {
            payload = PrivacyEgressFirewall::reconstruct_payload(
                db_path,
                connection,
                &transform.transform_run_id,
                transform.scope.clone(),
                &payload,
            )?
            .reconstructed_payload;
        }
    }
    Ok(payload)
}
pub(crate) fn privacy_transform_event_payload(
    run_id: &str,
    transform: &PrivacyEgressTransform,
) -> Value {
    json!({
        "runId": run_id,
        "transformRunId": transform.transform_run_id,
        "scopeKind": transform.scope.scope_kind,
        "scopeId": transform.scope.scope_id,
        "sourcePayloadHash": transform.source_payload_hash,
        "transformedPayloadHash": crate::privacy_egress::stable_hash(&transform.transformed_payload),
        "detectorVersion": transform.detector_version,
        "transformVersion": transform.transform_version,
        "findingCount": transform.findings.len(),
        "placeholderCount": transform.findings.len(),
        "findings": transform.findings,
    })
}
pub(crate) fn persist_dispatch(
    connection: &Connection,
    conversation_id: &str,
    segment_id: Option<&str>,
    event_type: &str,
    payload: Value,
    policy_decision_id: Option<&str>,
) -> Result<ConversationGatewayEnvelope> {
    let event = append_conversation_event(
        connection,
        conversation_id,
        segment_id,
        None,
        event_type,
        payload,
        policy_decision_id,
    )?;
    Ok(dispatch_from_event(conversation_id, &event))
}
pub(crate) fn persist_tool_request_event(
    connection: &Connection,
    tool_request: &LlmToolRequestView,
    extra_payload: Option<Value>,
) -> Result<ConversationGatewayEnvelope> {
    let mut payload = json!({
        "toolRequestId": tool_request.tool_request_id,
        "runId": tool_request.run_id,
        "conversationId": tool_request.conversation_id,
        "requestedCapabilityId": tool_request.requested_capability_id,
        "requestedBy": tool_request.requested_by,
        "approvalActorId": tool_request.approval_actor_id,
        "reason": tool_request.reason,
        "evidenceRefs": tool_request.evidence_refs,
        "inputSummary": tool_request.input_summary,
        "visibilityCeiling": tool_request.visibility_ceiling,
        "status": tool_request.status.as_str(),
        "policyDecisionId": tool_request.policy_decision_id,
        "createdAt": tool_request.created_at,
        "updatedAt": tool_request.updated_at,
    });
    if let Some(extra_payload) = extra_payload {
        merge_payload(&mut payload, extra_payload);
    }
    persist_dispatch(
        connection,
        &tool_request.conversation_id,
        None,
        tool_request.status.event_type(),
        payload,
        tool_request.policy_decision_id.as_deref(),
    )
}
pub(crate) fn load_tool_request_state(
    connection: &Connection,
    conversation_id: &str,
    tool_request_id: &str,
) -> Result<LlmToolRequestView> {
    require_text("conversation_id", conversation_id)?;
    require_text("tool_request_id", tool_request_id)?;
    let mut statement = connection.prepare(
        "SELECT event_type, payload_json
         FROM conversation_events
         WHERE conversation_id = ?1
           AND event_type IN (
                'llm.tool.requested',
                'llm.tool.approved',
                'llm.tool.rejected',
                'llm.tool.executing',
                'llm.tool.completed',
                'llm.tool.failed',
                'llm.tool.cancelled'
           )
         ORDER BY sequence ASC",
    )?;
    let mut rows = statement.query(params![conversation_id])?;
    let mut latest = None;
    while let Some(row) = rows.next()? {
        let event_type: String = row.get(0)?;
        let payload_json: String = row.get(1)?;
        let payload: Value = serde_json::from_str(&payload_json).unwrap_or_else(|_| json!({}));
        if payload["toolRequestId"].as_str() == Some(tool_request_id) {
            latest = Some(tool_request_from_payload(&event_type, &payload)?);
        }
    }
    latest.ok_or_else(|| anyhow::anyhow!("LLM tool request {tool_request_id} was not found"))
}
pub(crate) fn tool_request_from_payload(
    event_type: &str,
    payload: &Value,
) -> Result<LlmToolRequestView> {
    let status = match event_type {
        "llm.tool.requested" => LlmToolRequestStatus::Requested,
        "llm.tool.approved" => LlmToolRequestStatus::Approved,
        "llm.tool.rejected" => LlmToolRequestStatus::Rejected,
        "llm.tool.executing" => LlmToolRequestStatus::Executing,
        "llm.tool.completed" => LlmToolRequestStatus::Completed,
        "llm.tool.failed" => LlmToolRequestStatus::Failed,
        "llm.tool.cancelled" => LlmToolRequestStatus::Cancelled,
        _ => anyhow::bail!("unsupported LLM tool event type {event_type}"),
    };
    Ok(LlmToolRequestView {
        tool_request_id: required_json_string(payload, "toolRequestId")?,
        run_id: required_json_string(payload, "runId")?,
        conversation_id: required_json_string(payload, "conversationId")?,
        requested_capability_id: required_json_string(payload, "requestedCapabilityId")?,
        requested_by: required_json_string(payload, "requestedBy")?,
        approval_actor_id: payload["approvalActorId"].as_str().map(ToString::to_string),
        reason: required_json_string(payload, "reason")?,
        evidence_refs: payload["evidenceRefs"]
            .as_array()
            .map(|values| {
                values
                    .iter()
                    .filter_map(|value| value.as_str().map(ToString::to_string))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default(),
        input_summary: required_json_string(payload, "inputSummary")?,
        visibility_ceiling: required_json_string(payload, "visibilityCeiling")?,
        status,
        policy_decision_id: payload["policyDecisionId"]
            .as_str()
            .map(ToString::to_string),
        created_at: required_json_string(payload, "createdAt")?,
        updated_at: required_json_string(payload, "updatedAt")?,
    })
}
pub(crate) fn dispatch_from_event(
    conversation_id: &str,
    event: &RealtimeEvent,
) -> ConversationGatewayEnvelope {
    dispatch_envelope(
        &event.event_type,
        conversation_id,
        event.sequence.unwrap_or_default(),
        event.cursor,
        event.payload.clone(),
        &event.occurred_at,
    )
}
pub(crate) fn ephemeral_run_dispatch(
    event_type: &str,
    request: &LlmGatewayRequest,
    payload: Value,
) -> ConversationGatewayEnvelope {
    ConversationGatewayEnvelope {
        schema_version: CONVERSATION_GATEWAY_SCHEMA_VERSION.to_string(),
        op: ConversationGatewayOp::Dispatch,
        frame_type: event_type.to_string(),
        client_id: request.client_id.clone(),
        server_id: Some(format!("{}:{event_type}", request.run_id)),
        conversation_id: Some(request.conversation_id.clone()),
        segment_id: request.segment_id.clone(),
        sequence: None,
        cursor: None,
        durability: ConversationGatewayDurability::Ephemeral,
        scope: ConversationGatewayScope::Run,
        payload,
        occurred_at: Utc::now().to_rfc3339(),
    }
}
pub(crate) fn record_llm_policy_decision(
    connection: &Connection,
    actor: &ActorContext,
    action: PolicyAction,
    resource_kind: ResourceKind,
    resource_id: &str,
    capability_id: &str,
    policy: &LlmPolicy,
) -> Result<String> {
    Ok(record_policy_decision(
        connection,
        &PolicyDecision {
            outcome: policy.outcome,
            actor: actor.clone(),
            action,
            resource: ResourceRef::new(resource_kind, resource_id),
            capability_id: Some(capability_id.to_string()),
            reason: policy.reason.clone(),
        },
        PolicyDecisionCorrelation {
            request_id: Some(resource_id.to_string()),
            ..PolicyDecisionCorrelation::default()
        },
    )?)
}
pub(crate) fn ensure_capability_registered(
    connection: &Connection,
    capability_id: &str,
) -> Result<()> {
    let exists: Option<String> = connection
        .query_row(
            "SELECT id FROM capabilities WHERE id = ?1",
            params![capability_id],
            |row| row.get(0),
        )
        .optional()?;
    ensure!(
        exists.is_some(),
        "required LLM capability {capability_id} is not registered"
    );
    Ok(())
}
pub(crate) fn load_supported_tool_capability(
    connection: &Connection,
    capability_id: &str,
) -> Result<crate::capabilities::CapabilityDefinition> {
    require_text("requested_capability_id", capability_id)?;
    let capability = load_capability(connection, capability_id)?
        .ok_or_else(|| anyhow::anyhow!("unknown capability {capability_id}"))?;
    ensure!(
        capability.mcp_export_policy != MCP_EXPORT_POLICY_DANGEROUS_NONE,
        "capability {capability_id} is not exported for governed tool use"
    );
    ensure!(
        matches!(
            capability.mcp_export_policy.as_str(),
            crate::capabilities::MCP_EXPORT_POLICY_READ_ONLY
                | crate::capabilities::MCP_EXPORT_POLICY_LOCAL_MUTATION
                | crate::capabilities::MCP_EXPORT_POLICY_OPERATOR_CONFIRMED
        ),
        "capability {capability_id} has unsupported export policy"
    );
    Ok(capability)
}
pub(crate) fn validate_request(request: &LlmGatewayRequest) -> Result<()> {
    require_text("run_id", &request.run_id)?;
    require_text("conversation_id", &request.conversation_id)?;
    require_text(
        "assistant_participant_id",
        &request.assistant_participant_id,
    )?;
    require_text("provider_id", &request.provider_id)?;
    require_text("model_id", &request.model_id)?;
    require_text("user_message", &request.user_message)?;
    ensure!(
        !request.prompt_slots.is_empty(),
        "LLM request requires prompt slots"
    );
    Ok(())
}
pub(crate) fn validate_replay_fixture(fixture: &ReplayLlmFixture) -> Result<()> {
    ensure!(
        fixture.schema_version == LLM_REPLAY_FIXTURE_SCHEMA_VERSION,
        "unsupported replay fixture schema version"
    );
    require_text("fixture_id", &fixture.fixture_id)?;
    require_text("provider_id", &fixture.provider_id)?;
    require_text("model_id", &fixture.model_id)?;
    require_text("request_fingerprint", &fixture.request_fingerprint)?;
    require_text("prompt_hash", &fixture.prompt_hash)?;
    ensure!(
        !fixture.expected_prompt_slot_ids.is_empty(),
        "replay fixture expected prompt slot ids are required"
    );
    ensure!(
        !fixture.events.is_empty(),
        "replay fixture events are required"
    );
    ensure!(
        !fixture.provenance_refs.is_empty(),
        "replay fixture provenance refs are required"
    );
    let fixture_value = serde_json::to_value(fixture)?;
    ensure!(
        !json_value_contains_sensitive_fixture_text(&fixture_value),
        "replay fixture contains raw sensitive values"
    );
    Ok(())
}
pub(crate) fn json_value_contains_sensitive_fixture_text(value: &Value) -> bool {
    match value {
        Value::String(text) => text_contains_sensitive_fixture_value(text),
        Value::Array(items) => items.iter().any(json_value_contains_sensitive_fixture_text),
        Value::Object(map) => map.values().any(json_value_contains_sensitive_fixture_text),
        _ => false,
    }
}
pub(crate) fn text_contains_sensitive_fixture_value(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    if lower.contains("project orchid") {
        return true;
    }
    for token in text.split_whitespace() {
        let trimmed = token.trim_matches(|character: char| {
            character == ','
                || character == '.'
                || character == ';'
                || character == ':'
                || character == '"'
                || character == '\''
        });
        let lowered = trimmed.to_ascii_lowercase();
        if looks_like_fixture_email(trimmed)
            || looks_like_fixture_phone(trimmed)
            || lowered.starts_with("sk-")
            || lowered.starts_with("api_")
            || lowered.starts_with("pat_")
            || lowered.starts_with("ghp_")
            || lowered == "bearer"
            || lowered.starts_with("bearer_")
            || lowered.starts_with("bearer-")
        {
            return true;
        }
    }
    false
}
pub(crate) fn looks_like_fixture_email(value: &str) -> bool {
    let Some((local, domain)) = value.split_once('@') else {
        return false;
    };
    !local.is_empty() && domain.contains('.') && !domain.ends_with('.')
}
pub(crate) fn looks_like_fixture_phone(value: &str) -> bool {
    let digit_count = value
        .chars()
        .filter(|character| character.is_ascii_digit())
        .count();
    digit_count >= 10
        && value
            .chars()
            .all(|character| character.is_ascii_digit() || "()+-. ".contains(character))
}
pub(crate) fn validate_tool_request(request: &LlmToolRequestCreateRequest) -> Result<()> {
    require_text("run_id", &request.run_id)?;
    require_text("conversation_id", &request.conversation_id)?;
    require_text("requested_capability_id", &request.requested_capability_id)?;
    require_text("requested_by", &request.requested_by)?;
    require_text("reason", &request.reason)?;
    require_text("input_summary", &request.input_summary)?;
    require_text("visibility_ceiling", &request.visibility_ceiling)?;
    ensure!(
        !request.evidence_refs.is_empty(),
        "LLM tool request evidence refs are required"
    );
    Ok(())
}
pub(crate) fn merge_payload(payload: &mut Value, extra_payload: Value) {
    if let (Some(target), Some(extra)) = (payload.as_object_mut(), extra_payload.as_object()) {
        for (key, value) in extra {
            target.insert(key.clone(), value.clone());
        }
    }
}
pub(crate) fn required_json_string(payload: &Value, field: &str) -> Result<String> {
    payload[field]
        .as_str()
        .map(ToString::to_string)
        .ok_or_else(|| anyhow::anyhow!("LLM tool request payload missing {field}"))
}
pub(crate) fn stable_content_hash(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    format!("sha256:{:x}", hasher.finalize())
}
pub(crate) fn require_text(field: &str, value: &str) -> Result<()> {
    ensure!(!value.trim().is_empty(), "{field} is required");
    Ok(())
}
