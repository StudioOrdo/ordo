use super::*;
use crate::conversation_protocol::*;
use crate::conversations::*;
use crate::policy::*;
use crate::secrets::{expose_secret, OrdoSecretString};
use anyhow::{ensure, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::fmt;
use std::fs;
use std::path::Path;
use std::time::Duration;

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

    pub(crate) fn event_type(self) -> &'static str {
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
        crate::conversations::require_text("prompt slot id", &id)?;
        crate::conversations::require_text("prompt slot label", &label)?;
        crate::conversations::require_text("prompt slot content", &content)?;
        crate::conversations::require_text("prompt slot inclusion reason", &inclusion_reason)?;
        crate::conversations::require_text("prompt slot visibility ceiling", &visibility_ceiling)?;
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
pub struct ReplayFixtureRedactionSummary {
    pub redacted_value_count: usize,
    pub detectors: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum ReplayLlmFixtureEvent {
    TextDelta {
        delta: String,
    },
    Completed {
        text: String,
        usage: LlmUsageMetadata,
    },
    Failed {
        code: String,
        message: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReplayLlmFixture {
    pub schema_version: String,
    pub fixture_id: String,
    pub provider_id: String,
    pub model_id: String,
    pub request_fingerprint: String,
    pub prompt_hash: String,
    pub expected_prompt_slot_ids: Vec<String>,
    pub events: Vec<ReplayLlmFixtureEvent>,
    pub redaction_summary: ReplayFixtureRedactionSummary,
    pub provenance_refs: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone)]
pub struct ReplayLlmProvider {
    provider_id: String,
    model_id: String,
    fixtures: Vec<ReplayLlmFixture>,
}

impl ReplayLlmProvider {
    pub fn new(fixtures: Vec<ReplayLlmFixture>) -> Result<Self> {
        ensure!(!fixtures.is_empty(), "replay provider requires fixtures");
        for fixture in &fixtures {
            validate_replay_fixture(fixture)?;
        }
        let provider_id = fixtures[0].provider_id.clone();
        let model_id = fixtures[0].model_id.clone();
        ensure!(
            fixtures
                .iter()
                .all(|fixture| fixture.provider_id == provider_id && fixture.model_id == model_id),
            "replay provider fixtures must share one provider/model"
        );
        Ok(Self {
            provider_id,
            model_id,
            fixtures,
        })
    }

    pub fn from_fixture_file(path: &Path) -> Result<Self> {
        let raw = fs::read_to_string(path)?;
        let fixture: ReplayLlmFixture = serde_json::from_str(&raw)?;
        Self::new(vec![fixture])
    }
}

impl LlmProviderAdapter for ReplayLlmProvider {
    fn provider_id(&self) -> &str {
        &self.provider_id
    }

    fn model_id(&self) -> &str {
        &self.model_id
    }

    fn stream(&self, request: &LlmProviderRequest) -> Result<Vec<LlmProviderStreamEvent>> {
        let fingerprint = replay_request_fingerprint(request);
        let Some(fixture) = self
            .fixtures
            .iter()
            .find(|fixture| fixture.request_fingerprint == fingerprint)
        else {
            return Ok(vec![LlmProviderStreamEvent::Failed {
                code: "replay_fixture_not_found".to_string(),
                message: "No approved replay fixture matched the provider request.".to_string(),
            }]);
        };
        ensure!(
            fixture.prompt_hash == request.prompt.prompt_hash,
            "replay fixture prompt hash does not match request"
        );
        ensure!(
            fixture
                .expected_prompt_slot_ids
                .iter()
                .all(|slot_id| request.prompt.slots.iter().any(|slot| &slot.id == slot_id)),
            "replay fixture expected prompt slot ids are missing from request"
        );
        Ok(fixture
            .events
            .iter()
            .map(|event| match event {
                ReplayLlmFixtureEvent::TextDelta { delta } => {
                    LlmProviderStreamEvent::TextDelta(delta.clone())
                }
                ReplayLlmFixtureEvent::Completed { text, usage } => {
                    LlmProviderStreamEvent::Completed {
                        text: text.clone(),
                        usage: usage.clone(),
                    }
                }
                ReplayLlmFixtureEvent::Failed { code, message } => LlmProviderStreamEvent::Failed {
                    code: code.clone(),
                    message: message.clone(),
                },
            })
            .collect())
    }

    fn cancel(&self, _run_id: &str) -> Result<()> {
        Ok(())
    }
}

pub fn replay_request_fingerprint(request: &LlmProviderRequest) -> String {
    stable_content_hash(&format!(
        "{}\0{}\0{}\0{}",
        request.provider_id,
        request.model_id,
        request.prompt.prompt_hash,
        stable_content_hash(&request.user_message)
    ))
}

#[derive(Clone)]
pub struct OpenAiCompatibleConfig {
    pub provider_id: String,
    pub model_id: String,
    pub base_url: String,
    pub api_key: OrdoSecretString,
    pub timeout_ms: u64,
}

impl OpenAiCompatibleConfig {
    pub fn new(
        provider_id: impl Into<String>,
        model_id: impl Into<String>,
        base_url: impl Into<String>,
        api_key: impl Into<OrdoSecretString>,
    ) -> Result<Self> {
        let config = Self {
            provider_id: provider_id.into(),
            model_id: model_id.into(),
            base_url: base_url.into(),
            api_key: api_key.into(),
            timeout_ms: 30_000,
        };
        config.validate()?;
        Ok(config)
    }

    pub fn openai(
        model_id: impl Into<String>,
        api_key: impl Into<OrdoSecretString>,
    ) -> Result<Self> {
        Self::new("openai", model_id, "https://api.openai.com/v1", api_key)
    }

    pub fn with_timeout_ms(mut self, timeout_ms: u64) -> Result<Self> {
        self.timeout_ms = timeout_ms;
        self.validate()?;
        Ok(self)
    }

    fn chat_completions_url(&self) -> String {
        format!("{}/chat/completions", self.base_url.trim_end_matches('/'))
    }

    fn validate(&self) -> Result<()> {
        crate::conversations::require_text("provider_id", &self.provider_id)?;
        crate::conversations::require_text("model_id", &self.model_id)?;
        crate::conversations::require_text("base_url", &self.base_url)?;
        crate::conversations::require_text("api_key", expose_secret(&self.api_key))?;
        ensure!(self.timeout_ms > 0, "timeout_ms must be positive");
        Ok(())
    }
}

impl fmt::Debug for OpenAiCompatibleConfig {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("OpenAiCompatibleConfig")
            .field("provider_id", &self.provider_id)
            .field("model_id", &self.model_id)
            .field("base_url", &self.base_url)
            .field("api_key", &"[redacted]")
            .field("timeout_ms", &self.timeout_ms)
            .finish()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenAiTransportResponse {
    pub status: u16,
    pub body: Value,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OllamaTransportResponse {
    pub status: u16,
    pub body: Value,
}

pub trait OpenAiCompatibleTransport: Clone {
    fn post_chat_completions(
        &self,
        endpoint: &str,
        api_key: &str,
        timeout_ms: u64,
        body: &Value,
    ) -> Result<OpenAiTransportResponse>;
}

pub trait OllamaChatTransport: Clone {
    fn post_chat(
        &self,
        endpoint: &str,
        timeout_ms: u64,
        body: &Value,
    ) -> Result<OllamaTransportResponse>;
}

#[derive(Debug, Clone, Default)]
pub struct ReqwestOpenAiTransport;

#[derive(Debug, Clone, Default)]
pub struct ReqwestOllamaTransport;

impl OpenAiCompatibleTransport for ReqwestOpenAiTransport {
    fn post_chat_completions(
        &self,
        endpoint: &str,
        api_key: &str,
        timeout_ms: u64,
        body: &Value,
    ) -> Result<OpenAiTransportResponse> {
        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_millis(timeout_ms))
            .build()?;
        let response = client
            .post(endpoint)
            .bearer_auth(api_key)
            .json(body)
            .send()?;
        let status = response.status().as_u16();
        let body = response.json::<Value>().unwrap_or_else(
            |_| json!({ "error": { "message": "Provider returned non-JSON response." } }),
        );
        Ok(OpenAiTransportResponse { status, body })
    }
}

impl OllamaChatTransport for ReqwestOllamaTransport {
    fn post_chat(
        &self,
        endpoint: &str,
        timeout_ms: u64,
        body: &Value,
    ) -> Result<OllamaTransportResponse> {
        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_millis(timeout_ms))
            .build()?;
        let response = client.post(endpoint).json(body).send()?;
        let status = response.status().as_u16();
        let body = response
            .json::<Value>()
            .unwrap_or_else(|_| json!({ "error": "Ollama returned non-JSON response." }));
        Ok(OllamaTransportResponse { status, body })
    }
}

#[derive(Clone)]
pub struct OllamaChatConfig {
    pub provider_id: String,
    pub model_id: String,
    pub base_url: String,
    pub timeout_ms: u64,
}

impl OllamaChatConfig {
    pub fn new(
        provider_id: impl Into<String>,
        model_id: impl Into<String>,
        base_url: impl Into<String>,
    ) -> Result<Self> {
        let config = Self {
            provider_id: provider_id.into(),
            model_id: model_id.into(),
            base_url: base_url.into(),
            timeout_ms: 120_000,
        };
        config.validate()?;
        Ok(config)
    }

    pub fn with_timeout_ms(mut self, timeout_ms: u64) -> Result<Self> {
        self.timeout_ms = timeout_ms;
        self.validate()?;
        Ok(self)
    }

    fn chat_url(&self) -> String {
        format!("{}/chat", self.base_url.trim_end_matches('/'))
    }

    fn validate(&self) -> Result<()> {
        crate::conversations::require_text("provider_id", &self.provider_id)?;
        crate::conversations::require_text("model_id", &self.model_id)?;
        crate::conversations::require_text("base_url", &self.base_url)?;
        ensure!(self.timeout_ms > 0, "timeout_ms must be positive");
        Ok(())
    }
}

impl fmt::Debug for OllamaChatConfig {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("OllamaChatConfig")
            .field("provider_id", &self.provider_id)
            .field("model_id", &self.model_id)
            .field("base_url", &self.base_url)
            .field("timeout_ms", &self.timeout_ms)
            .finish()
    }
}

#[derive(Debug, Clone)]
pub struct OllamaChatProvider<T = ReqwestOllamaTransport> {
    config: OllamaChatConfig,
    transport: T,
}

#[derive(Debug, Clone)]
pub struct OpenAiCompatibleProvider<T = ReqwestOpenAiTransport> {
    config: OpenAiCompatibleConfig,
    transport: T,
}

#[derive(Clone)]
pub struct AnthropicMessagesConfig {
    pub provider_id: String,
    pub model_id: String,
    pub base_url: String,
    pub api_key: OrdoSecretString,
    pub timeout_ms: u64,
}

impl AnthropicMessagesConfig {
    pub fn new(
        provider_id: impl Into<String>,
        model_id: impl Into<String>,
        base_url: impl Into<String>,
        api_key: impl Into<OrdoSecretString>,
    ) -> Result<Self> {
        let config = Self {
            provider_id: provider_id.into(),
            model_id: model_id.into(),
            base_url: base_url.into(),
            api_key: api_key.into(),
            timeout_ms: 30_000,
        };
        config.validate()?;
        Ok(config)
    }

    pub fn with_timeout_ms(mut self, timeout_ms: u64) -> Result<Self> {
        self.timeout_ms = timeout_ms;
        self.validate()?;
        Ok(self)
    }

    fn messages_url(&self) -> String {
        format!("{}/messages", self.base_url.trim_end_matches('/'))
    }

    fn validate(&self) -> Result<()> {
        crate::conversations::require_text("provider_id", &self.provider_id)?;
        crate::conversations::require_text("model_id", &self.model_id)?;
        crate::conversations::require_text("base_url", &self.base_url)?;
        crate::conversations::require_text("api_key", expose_secret(&self.api_key))?;
        ensure!(self.timeout_ms > 0, "timeout_ms must be positive");
        Ok(())
    }
}

impl fmt::Debug for AnthropicMessagesConfig {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("AnthropicMessagesConfig")
            .field("provider_id", &self.provider_id)
            .field("model_id", &self.model_id)
            .field("base_url", &self.base_url)
            .field("api_key", &"[redacted]")
            .field("timeout_ms", &self.timeout_ms)
            .finish()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnthropicTransportResponse {
    pub status: u16,
    pub body: Value,
}

pub trait AnthropicMessagesTransport: Clone {
    fn post_messages(
        &self,
        endpoint: &str,
        api_key: &str,
        timeout_ms: u64,
        body: &Value,
    ) -> Result<AnthropicTransportResponse>;
}

#[derive(Debug, Clone, Default)]
pub struct ReqwestAnthropicMessagesTransport;

impl AnthropicMessagesTransport for ReqwestAnthropicMessagesTransport {
    fn post_messages(
        &self,
        endpoint: &str,
        api_key: &str,
        timeout_ms: u64,
        body: &Value,
    ) -> Result<AnthropicTransportResponse> {
        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_millis(timeout_ms))
            .build()?;
        let response = client
            .post(endpoint)
            .header("x-api-key", api_key)
            .header("anthropic-version", "2023-06-01")
            .json(body)
            .send()?;
        let status = response.status().as_u16();
        let body = response.json::<Value>().unwrap_or_else(
            |_| json!({ "error": { "message": "Provider returned non-JSON response." } }),
        );
        Ok(AnthropicTransportResponse { status, body })
    }
}

#[derive(Debug, Clone)]
pub struct AnthropicMessagesProvider<T = ReqwestAnthropicMessagesTransport> {
    config: AnthropicMessagesConfig,
    transport: T,
}

impl AnthropicMessagesProvider<ReqwestAnthropicMessagesTransport> {
    pub fn new(config: AnthropicMessagesConfig) -> Self {
        Self {
            config,
            transport: ReqwestAnthropicMessagesTransport,
        }
    }
}

impl<T: AnthropicMessagesTransport> AnthropicMessagesProvider<T> {
    pub fn with_transport(config: AnthropicMessagesConfig, transport: T) -> Self {
        Self { config, transport }
    }
}

impl<T: AnthropicMessagesTransport> LlmProviderAdapter for AnthropicMessagesProvider<T> {
    fn provider_id(&self) -> &str {
        &self.config.provider_id
    }

    fn model_id(&self) -> &str {
        &self.config.model_id
    }

    fn stream(&self, request: &LlmProviderRequest) -> Result<Vec<LlmProviderStreamEvent>> {
        let body = anthropic_messages_body(&self.config.model_id, request);
        let response = self.transport.post_messages(
            &self.config.messages_url(),
            expose_secret(&self.config.api_key),
            self.config.timeout_ms,
            &body,
        );
        let response = match response {
            Ok(response) => response,
            Err(error) => {
                return Ok(vec![LlmProviderStreamEvent::Failed {
                    code: "provider_transport_failed".to_string(),
                    message: safe_provider_error_message(&error.to_string()),
                }]);
            }
        };
        Ok(normalize_anthropic_response(response))
    }

    fn cancel(&self, _run_id: &str) -> Result<()> {
        Ok(())
    }
}

impl OpenAiCompatibleProvider<ReqwestOpenAiTransport> {
    pub fn new(config: OpenAiCompatibleConfig) -> Self {
        Self {
            config,
            transport: ReqwestOpenAiTransport,
        }
    }
}

impl<T: OpenAiCompatibleTransport> OpenAiCompatibleProvider<T> {
    pub fn with_transport(config: OpenAiCompatibleConfig, transport: T) -> Self {
        Self { config, transport }
    }

    pub fn request_body(&self, request: &LlmProviderRequest) -> Value {
        openai_chat_completion_body(&self.config.model_id, request)
    }
}

impl<T: OpenAiCompatibleTransport> LlmProviderAdapter for OpenAiCompatibleProvider<T> {
    fn provider_id(&self) -> &str {
        &self.config.provider_id
    }

    fn model_id(&self) -> &str {
        &self.config.model_id
    }

    fn stream(&self, request: &LlmProviderRequest) -> Result<Vec<LlmProviderStreamEvent>> {
        let body = self.request_body(request);
        let response = self.transport.post_chat_completions(
            &self.config.chat_completions_url(),
            expose_secret(&self.config.api_key),
            self.config.timeout_ms,
            &body,
        );
        let response = match response {
            Ok(response) => response,
            Err(error) => {
                return Ok(vec![LlmProviderStreamEvent::Failed {
                    code: "provider_transport_failed".to_string(),
                    message: safe_provider_error_message(&error.to_string()),
                }]);
            }
        };
        Ok(normalize_openai_response(response))
    }

    fn cancel(&self, _run_id: &str) -> Result<()> {
        Ok(())
    }
}

impl OllamaChatProvider<ReqwestOllamaTransport> {
    pub fn new(config: OllamaChatConfig) -> Self {
        Self {
            config,
            transport: ReqwestOllamaTransport,
        }
    }
}

impl<T: OllamaChatTransport> OllamaChatProvider<T> {
    pub fn with_transport(config: OllamaChatConfig, transport: T) -> Self {
        Self { config, transport }
    }
}

impl<T: OllamaChatTransport> LlmProviderAdapter for OllamaChatProvider<T> {
    fn provider_id(&self) -> &str {
        &self.config.provider_id
    }

    fn model_id(&self) -> &str {
        &self.config.model_id
    }

    fn stream(&self, request: &LlmProviderRequest) -> Result<Vec<LlmProviderStreamEvent>> {
        let body = ollama_chat_body(&self.config.model_id, request);
        let response =
            self.transport
                .post_chat(&self.config.chat_url(), self.config.timeout_ms, &body);
        let response = match response {
            Ok(response) => response,
            Err(error) => {
                return Ok(vec![LlmProviderStreamEvent::Failed {
                    code: "provider_transport_failed".to_string(),
                    message: safe_provider_error_message(&error.to_string()),
                }]);
            }
        };
        Ok(normalize_ollama_response(response))
    }

    fn cancel(&self, _run_id: &str) -> Result<()> {
        Ok(())
    }
}

fn openai_chat_completion_body(model_id: &str, request: &LlmProviderRequest) -> Value {
    let system_content = request
        .prompt
        .slots
        .iter()
        .map(|slot| format!("{}:\n{}", slot.label, slot.content))
        .collect::<Vec<_>>()
        .join("\n\n");
    json!({
        "model": model_id,
        "stream": false,
        "messages": [
            {
                "role": "system",
                "content": system_content,
            },
            {
                "role": "user",
                "content": request.user_message,
            }
        ]
    })
}

fn anthropic_messages_body(model_id: &str, request: &LlmProviderRequest) -> Value {
    let system_content = request
        .prompt
        .slots
        .iter()
        .map(|slot| format!("{}:\n{}", slot.label, slot.content))
        .collect::<Vec<_>>()
        .join("\n\n");
    json!({
        "model": model_id,
        "max_tokens": 512,
        "system": system_content,
        "messages": [
            {
                "role": "user",
                "content": request.user_message,
            }
        ],
        "metadata": {
            "user_id": stable_content_hash(&request.run_id),
        }
    })
}

fn ollama_chat_body(model_id: &str, request: &LlmProviderRequest) -> Value {
    let system_content = request
        .prompt
        .slots
        .iter()
        .map(|slot| format!("{}:\n{}", slot.label, slot.content))
        .collect::<Vec<_>>()
        .join("\n\n");
    json!({
        "model": model_id,
        "stream": false,
        "messages": [
            {
                "role": "system",
                "content": system_content,
            },
            {
                "role": "user",
                "content": request.user_message,
            }
        ]
    })
}

pub(crate) fn normalize_openai_response(
    response: OpenAiTransportResponse,
) -> Vec<LlmProviderStreamEvent> {
    if !(200..300).contains(&response.status) {
        let (code, message) = openai_error_code_message(&response.body);
        return vec![LlmProviderStreamEvent::Failed {
            code,
            message: safe_provider_error_message(&message),
        }];
    }

    let Some(text) = response
        .body
        .get("choices")
        .and_then(Value::as_array)
        .and_then(|choices| choices.first())
        .and_then(|choice| choice.get("message"))
        .and_then(|message| message.get("content"))
        .and_then(Value::as_str)
        .filter(|text| !text.trim().is_empty())
    else {
        return vec![LlmProviderStreamEvent::Failed {
            code: "unsupported_provider_response".to_string(),
            message: "OpenAI-compatible response did not include assistant text.".to_string(),
        }];
    };

    let input_tokens = response
        .body
        .get("usage")
        .and_then(|usage| usage.get("prompt_tokens"))
        .and_then(Value::as_i64)
        .unwrap_or(0);
    let output_tokens = response
        .body
        .get("usage")
        .and_then(|usage| usage.get("completion_tokens"))
        .and_then(Value::as_i64)
        .unwrap_or_else(|| text.split_whitespace().count() as i64);

    let mut events = text_delta_chunks(text)
        .into_iter()
        .map(LlmProviderStreamEvent::TextDelta)
        .collect::<Vec<_>>();
    events.push(LlmProviderStreamEvent::Completed {
        text: text.to_string(),
        usage: LlmUsageMetadata {
            input_tokens,
            output_tokens,
        },
    });
    events
}

pub(crate) fn normalize_ollama_response(
    response: OllamaTransportResponse,
) -> Vec<LlmProviderStreamEvent> {
    if !(200..300).contains(&response.status) {
        let message = response
            .body
            .get("error")
            .and_then(Value::as_str)
            .filter(|message| !message.trim().is_empty())
            .unwrap_or("Ollama provider returned an error.")
            .to_string();
        return vec![LlmProviderStreamEvent::Failed {
            code: "provider_error".to_string(),
            message: safe_provider_error_message(&message),
        }];
    }

    let Some(text) = response
        .body
        .get("message")
        .and_then(|message| message.get("content"))
        .and_then(Value::as_str)
        .filter(|text| !text.trim().is_empty())
    else {
        return vec![LlmProviderStreamEvent::Failed {
            code: "unsupported_provider_response".to_string(),
            message: "Ollama response did not include assistant text.".to_string(),
        }];
    };

    let input_tokens = response
        .body
        .get("prompt_eval_count")
        .and_then(Value::as_i64)
        .unwrap_or(0);
    let output_tokens = response
        .body
        .get("eval_count")
        .and_then(Value::as_i64)
        .unwrap_or_else(|| text.split_whitespace().count() as i64);

    let mut events = text_delta_chunks(text)
        .into_iter()
        .map(LlmProviderStreamEvent::TextDelta)
        .collect::<Vec<_>>();
    events.push(LlmProviderStreamEvent::Completed {
        text: text.to_string(),
        usage: LlmUsageMetadata {
            input_tokens,
            output_tokens,
        },
    });
    events
}

fn text_delta_chunks(text: &str) -> Vec<String> {
    let mut chunks = Vec::new();
    let mut current = String::new();
    for part in text.split_inclusive(' ') {
        current.push_str(part);
        if current.len() >= 24 {
            chunks.push(std::mem::take(&mut current));
        }
    }
    if !current.is_empty() {
        chunks.push(current);
    }
    if chunks.is_empty() && !text.is_empty() {
        chunks.push(text.to_string());
    }
    chunks
}

fn openai_error_code_message(body: &Value) -> (String, String) {
    let code = body
        .get("error")
        .and_then(|error| error.get("code").or_else(|| error.get("type")))
        .and_then(Value::as_str)
        .filter(|code| !code.trim().is_empty())
        .unwrap_or("provider_error")
        .to_string();
    let message = body
        .get("error")
        .and_then(|error| error.get("message"))
        .and_then(Value::as_str)
        .filter(|message| !message.trim().is_empty())
        .unwrap_or("OpenAI-compatible provider returned an error.")
        .to_string();
    (code, message)
}

pub(crate) fn normalize_anthropic_response(
    response: AnthropicTransportResponse,
) -> Vec<LlmProviderStreamEvent> {
    if !(200..300).contains(&response.status) {
        let code = response
            .body
            .get("error")
            .and_then(|error| error.get("type"))
            .and_then(Value::as_str)
            .filter(|code| !code.trim().is_empty())
            .unwrap_or("provider_error")
            .to_string();
        let message = response
            .body
            .get("error")
            .and_then(|error| error.get("message"))
            .and_then(Value::as_str)
            .filter(|message| !message.trim().is_empty())
            .unwrap_or("Anthropic provider returned an error.")
            .to_string();
        return vec![LlmProviderStreamEvent::Failed {
            code,
            message: safe_provider_error_message(&message),
        }];
    }

    let text = response
        .body
        .get("content")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|item| {
            let kind = item.get("type").and_then(Value::as_str);
            let text = item.get("text").and_then(Value::as_str);
            (kind == Some("text")).then_some(text).flatten()
        })
        .collect::<Vec<_>>()
        .join("");
    if text.trim().is_empty() {
        return vec![LlmProviderStreamEvent::Failed {
            code: "unsupported_provider_response".to_string(),
            message: "Anthropic response did not include assistant text.".to_string(),
        }];
    }

    let input_tokens = response
        .body
        .get("usage")
        .and_then(|usage| usage.get("input_tokens"))
        .and_then(Value::as_i64)
        .unwrap_or(0);
    let output_tokens = response
        .body
        .get("usage")
        .and_then(|usage| usage.get("output_tokens"))
        .and_then(Value::as_i64)
        .unwrap_or_else(|| text.split_whitespace().count() as i64);

    let mut events = text_delta_chunks(&text)
        .into_iter()
        .map(LlmProviderStreamEvent::TextDelta)
        .collect::<Vec<_>>();
    events.push(LlmProviderStreamEvent::Completed {
        text,
        usage: LlmUsageMetadata {
            input_tokens,
            output_tokens,
        },
    });
    events
}

fn safe_provider_error_message(message: &str) -> String {
    if text_contains_sensitive_fixture_value(message) {
        format!(
            "Provider error redacted; message hash {}.",
            stable_content_hash(message)
        )
    } else {
        message.to_string()
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
    pub(crate) outcome: PolicyOutcome,
    pub(crate) reason: String,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_ollama_response_returns_text_and_usage() {
        let events = normalize_ollama_response(OllamaTransportResponse {
            status: 200,
            body: json!({
                "message": { "role": "assistant", "content": "OK from local Ollama" },
                "prompt_eval_count": 11,
                "eval_count": 4,
                "done": true,
            }),
        });

        assert_eq!(
            events,
            vec![
                LlmProviderStreamEvent::TextDelta("OK from local Ollama".to_string()),
                LlmProviderStreamEvent::Completed {
                    text: "OK from local Ollama".to_string(),
                    usage: LlmUsageMetadata {
                        input_tokens: 11,
                        output_tokens: 4,
                    },
                },
            ]
        );
    }

    #[test]
    fn normalize_ollama_response_reports_safe_provider_error() {
        let events = normalize_ollama_response(OllamaTransportResponse {
            status: 404,
            body: json!({ "error": "model not found" }),
        });

        assert_eq!(
            events,
            vec![LlmProviderStreamEvent::Failed {
                code: "provider_error".to_string(),
                message: "model not found".to_string(),
            }]
        );
    }
}
