use anyhow::{ensure, Result};
use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::capabilities::{
    load_capability, MCP_EXPORT_POLICY_DANGEROUS_NONE, MCP_EXPORT_POLICY_LOCAL_MUTATION,
    MCP_EXPORT_POLICY_OPERATOR_CONFIRMED, MCP_EXPORT_POLICY_READ_ONLY,
};
use crate::conversation_protocol::{
    command_rejected_error, dispatch_envelope, policy_denied_error, ConversationGatewayDurability,
    ConversationGatewayEnvelope, ConversationGatewayOp, ConversationGatewayScope,
    CONVERSATION_GATEWAY_SCHEMA_VERSION,
};
use crate::conversations::{
    append_conversation_event, create_conversation_message, ConversationMessageCreateRequest,
    ConversationMessageView,
};
use crate::events::RealtimeEvent;
use crate::policy::{
    record_policy_decision, ActorContext, PolicyAction, PolicyDecision, PolicyDecisionCorrelation,
    PolicyOutcome, ResourceKind, ResourceRef,
};

pub const LLM_INVOKE_CAPABILITY_ID: &str = "llm.invoke";
pub const LLM_CANCEL_CAPABILITY_ID: &str = "llm.cancel";
pub const LLM_TOOL_REQUEST_CAPABILITY_ID: &str = "llm.tool.request";
pub const LLM_TOOL_APPROVE_CAPABILITY_ID: &str = "llm.tool.approve";
pub const LLM_TOOL_REJECT_CAPABILITY_ID: &str = "llm.tool.reject";
pub const LLM_TOOL_EXECUTE_CAPABILITY_ID: &str = "llm.tool.execute";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LlmToolRequestStatus {
    Requested,
    Approved,
    Rejected,
    Executing,
    Completed,
    Failed,
    Cancelled,
}

impl LlmToolRequestStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Requested => "requested",
            Self::Approved => "approved",
            Self::Rejected => "rejected",
            Self::Executing => "executing",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }

    fn event_type(self) -> &'static str {
        match self {
            Self::Requested => "llm.tool.requested",
            Self::Approved => "llm.tool.approved",
            Self::Rejected => "llm.tool.rejected",
            Self::Executing => "llm.tool.executing",
            Self::Completed => "llm.tool.completed",
            Self::Failed => "llm.tool.failed",
            Self::Cancelled => "llm.tool.cancelled",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LlmToolRequestCreateRequest {
    pub run_id: String,
    pub conversation_id: String,
    pub requested_capability_id: String,
    pub requested_by: String,
    pub reason: String,
    pub evidence_refs: Vec<String>,
    pub input_summary: String,
    pub visibility_ceiling: String,
    pub client_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LlmToolRequestView {
    pub tool_request_id: String,
    pub run_id: String,
    pub conversation_id: String,
    pub requested_capability_id: String,
    pub requested_by: String,
    pub approval_actor_id: Option<String>,
    pub reason: String,
    pub evidence_refs: Vec<String>,
    pub input_summary: String,
    pub visibility_ceiling: String,
    pub status: LlmToolRequestStatus,
    pub policy_decision_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LlmToolRequestReceipt {
    pub tool_request: Option<LlmToolRequestView>,
    pub policy_decision_id: Option<String>,
    pub frames: Vec<ConversationGatewayEnvelope>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PromptSlot {
    pub id: String,
    pub label: String,
    pub content: String,
    pub source_refs: Vec<String>,
    pub inclusion_reason: String,
    pub visibility_ceiling: String,
    pub content_hash: String,
}

impl PromptSlot {
    pub fn new(
        id: impl Into<String>,
        label: impl Into<String>,
        content: impl Into<String>,
        source_refs: Vec<String>,
        inclusion_reason: impl Into<String>,
        visibility_ceiling: impl Into<String>,
    ) -> Result<Self> {
        let id = id.into();
        let label = label.into();
        let content = content.into();
        let inclusion_reason = inclusion_reason.into();
        let visibility_ceiling = visibility_ceiling.into();
        require_text("prompt slot id", &id)?;
        require_text("prompt slot label", &label)?;
        require_text("prompt slot content", &content)?;
        require_text("prompt slot inclusion reason", &inclusion_reason)?;
        require_text("prompt slot visibility ceiling", &visibility_ceiling)?;
        ensure!(
            !source_refs.is_empty(),
            "prompt slot source refs are required"
        );
        let content_hash = stable_content_hash(&content);
        Ok(Self {
            id,
            label,
            content,
            source_refs,
            inclusion_reason,
            visibility_ceiling,
            content_hash,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompiledPrompt {
    pub prompt_id: String,
    pub prompt_hash: String,
    pub slots: Vec<PromptSlot>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LlmProviderRequest {
    pub run_id: String,
    pub provider_id: String,
    pub model_id: String,
    pub prompt: CompiledPrompt,
    pub user_message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LlmUsageMetadata {
    pub input_tokens: i64,
    pub output_tokens: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LlmProviderStreamEvent {
    TextDelta(String),
    Completed {
        text: String,
        usage: LlmUsageMetadata,
    },
    Failed {
        code: String,
        message: String,
    },
}

pub trait LlmProviderAdapter {
    fn provider_id(&self) -> &str;
    fn model_id(&self) -> &str;
    fn stream(&self, request: &LlmProviderRequest) -> Result<Vec<LlmProviderStreamEvent>>;
    fn cancel(&self, run_id: &str) -> Result<()>;
}

#[derive(Debug, Clone)]
pub struct DeterministicLlmProvider {
    provider_id: String,
    model_id: String,
    deltas: Vec<String>,
    final_text: String,
    fail_with: Option<(String, String)>,
}

impl DeterministicLlmProvider {
    pub fn new(provider_id: impl Into<String>, model_id: impl Into<String>) -> Self {
        Self {
            provider_id: provider_id.into(),
            model_id: model_id.into(),
            deltas: vec!["Drafting ".to_string(), "answer".to_string()],
            final_text: "Drafting answer".to_string(),
            fail_with: None,
        }
    }

    pub fn failing(
        provider_id: impl Into<String>,
        model_id: impl Into<String>,
        code: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            provider_id: provider_id.into(),
            model_id: model_id.into(),
            deltas: Vec::new(),
            final_text: String::new(),
            fail_with: Some((code.into(), message.into())),
        }
    }
}

impl LlmProviderAdapter for DeterministicLlmProvider {
    fn provider_id(&self) -> &str {
        &self.provider_id
    }

    fn model_id(&self) -> &str {
        &self.model_id
    }

    fn stream(&self, _request: &LlmProviderRequest) -> Result<Vec<LlmProviderStreamEvent>> {
        if let Some((code, message)) = &self.fail_with {
            return Ok(vec![LlmProviderStreamEvent::Failed {
                code: code.clone(),
                message: message.clone(),
            }]);
        }
        let mut events = self
            .deltas
            .iter()
            .cloned()
            .map(LlmProviderStreamEvent::TextDelta)
            .collect::<Vec<_>>();
        events.push(LlmProviderStreamEvent::Completed {
            text: self.final_text.clone(),
            usage: LlmUsageMetadata {
                input_tokens: 12,
                output_tokens: self.final_text.split_whitespace().count() as i64,
            },
        });
        Ok(events)
    }

    fn cancel(&self, _run_id: &str) -> Result<()> {
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LlmGatewayRequest {
    pub run_id: String,
    pub conversation_id: String,
    pub segment_id: Option<String>,
    pub assistant_participant_id: String,
    pub client_id: Option<String>,
    pub provider_id: String,
    pub model_id: String,
    pub user_message: String,
    pub prompt_slots: Vec<PromptSlot>,
}

#[derive(Debug, Clone)]
pub struct LlmPolicy {
    outcome: PolicyOutcome,
    reason: String,
}

impl LlmPolicy {
    pub fn allow(reason: impl Into<String>) -> Self {
        Self {
            outcome: PolicyOutcome::Allowed,
            reason: reason.into(),
        }
    }

    pub fn deny(reason: impl Into<String>) -> Self {
        Self {
            outcome: PolicyOutcome::Denied,
            reason: reason.into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LlmGatewayRunResult {
    pub run_id: String,
    pub policy_decision_id: String,
    pub prompt: Option<CompiledPrompt>,
    pub final_message: Option<ConversationMessageView>,
    pub frames: Vec<ConversationGatewayEnvelope>,
}

pub struct LlmGateway<P> {
    provider: P,
    invoke_policy: LlmPolicy,
    cancel_policy: LlmPolicy,
}

struct LlmToolTransition<'a> {
    next_status: LlmToolRequestStatus,
    capability_id: &'a str,
    action: PolicyAction,
    reason: &'a str,
}

impl<P: LlmProviderAdapter> LlmGateway<P> {
    pub fn new(provider: P) -> Self {
        Self {
            provider,
            invoke_policy: LlmPolicy::allow("LLM invocation allowed by daemon gateway policy."),
            cancel_policy: LlmPolicy::allow("LLM cancellation allowed by daemon gateway policy."),
        }
    }

    pub fn with_policies(provider: P, invoke_policy: LlmPolicy, cancel_policy: LlmPolicy) -> Self {
        Self {
            provider,
            invoke_policy,
            cancel_policy,
        }
    }

    pub fn run_completion(
        &self,
        connection: &Connection,
        actor: &ActorContext,
        request: LlmGatewayRequest,
    ) -> Result<LlmGatewayRunResult> {
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
        let mut frames = Vec::new();
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
            prompt: prompt.clone(),
            user_message: request.user_message.clone(),
        };
        let stream = self.provider.stream(&provider_request)?;
        let mut completed_text = None;
        let mut usage = None;
        for event in stream {
            match event {
                LlmProviderStreamEvent::TextDelta(delta) => {
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

        let completed_text = completed_text.unwrap_or_default();
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
        }
        frames.push(persist_dispatch(
            connection,
            &request.conversation_id,
            request.segment_id.as_deref(),
            "llm.run.completed",
            json!({
                "runId": request.run_id,
                "messageId": final_message.id,
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

    fn transition_tool_request(
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

fn persist_dispatch(
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

fn persist_tool_request_event(
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

fn load_tool_request_state(
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

fn tool_request_from_payload(event_type: &str, payload: &Value) -> Result<LlmToolRequestView> {
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

fn dispatch_from_event(
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

fn ephemeral_run_dispatch(
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

fn record_llm_policy_decision(
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

fn ensure_capability_registered(connection: &Connection, capability_id: &str) -> Result<()> {
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

fn load_supported_tool_capability(
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
            MCP_EXPORT_POLICY_READ_ONLY
                | MCP_EXPORT_POLICY_LOCAL_MUTATION
                | MCP_EXPORT_POLICY_OPERATOR_CONFIRMED
        ),
        "capability {capability_id} has unsupported export policy"
    );
    Ok(capability)
}

fn validate_request(request: &LlmGatewayRequest) -> Result<()> {
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

fn validate_tool_request(request: &LlmToolRequestCreateRequest) -> Result<()> {
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

fn merge_payload(payload: &mut Value, extra_payload: Value) {
    if let (Some(target), Some(extra)) = (payload.as_object_mut(), extra_payload.as_object()) {
        for (key, value) in extra {
            target.insert(key.clone(), value.clone());
        }
    }
}

fn required_json_string(payload: &Value, field: &str) -> Result<String> {
    payload[field]
        .as_str()
        .map(ToString::to_string)
        .ok_or_else(|| anyhow::anyhow!("LLM tool request payload missing {field}"))
}

fn stable_content_hash(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    format!("sha256:{:x}", hasher.finalize())
}

fn require_text(field: &str, value: &str) -> Result<()> {
    ensure!(!value.trim().is_empty(), "{field} is required");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capabilities::seed_builtin_capabilities;
    use crate::conversations::{
        create_conversation_participant, find_or_create_canonical_conversation,
        CanonicalConversationRequest, ConversationParticipantCreateRequest,
    };
    use crate::policy::ActorContext;
    use crate::schema::init_schema;
    use rusqlite::Connection;
    use std::cell::Cell;

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
    fn provider_stream_normalizes_ephemeral_deltas_and_durable_completion() {
        let connection = test_connection();
        let (conversation_id, assistant_id) = conversation_and_assistant(&connection);
        let gateway = LlmGateway::new(DeterministicLlmProvider::new("local_fake", "fake-chat"));

        let result = gateway
            .run_completion(
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
        let (conversation_id, assistant_id) = conversation_and_assistant(&connection);
        let gateway = LlmGateway::new(DeterministicLlmProvider::new("local_fake", "fake-chat"));

        let result = gateway
            .run_completion(
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
        let (conversation_id, assistant_id) = conversation_and_assistant(&connection);
        let gateway = LlmGateway::new(DeterministicLlmProvider::failing(
            "local_fake",
            "fake-chat",
            "provider_unavailable",
            "provider offline",
        ));

        let result = gateway
            .run_completion(
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

    #[test]
    fn policy_denial_records_evidence_and_does_not_invoke_provider() {
        let connection = test_connection();
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
        let (conversation_id, assistant_id) = conversation_and_assistant(&connection);
        let gateway = LlmGateway::new(DeterministicLlmProvider::new("local_fake", "fake-chat"));

        gateway
            .run_completion(
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
}
