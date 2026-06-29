//! Core types for the hamr AI layer.
//!
//! This module defines every message shape, model descriptor, tool contract, and
//! streaming event used across the entire agent stack. It mirrors `packages/ai/src/types.ts`
//! exactly, translated from TypeScript discriminated unions to Rust enums.

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Content blocks
// ---------------------------------------------------------------------------

/// A text content block within a message.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct TextContent {
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text_signature: Option<String>,
}

/// A thinking/reasoning content block (redacted on safety filter).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ThinkingContent {
    pub thinking: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking_signature: Option<String>,
    /// True when the thinking content was redacted by safety filters.
    #[serde(default)]
    pub redacted: bool,
}

/// An inline image content block (base64-encoded).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ImageContent {
    /// Base64-encoded image data.
    pub data: String,
    /// MIME type, e.g. `"image/png"`.
    pub mime_type: String,
}

/// A tool call requested by the assistant.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub arguments: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thought_signature: Option<String>,
}

// ---------------------------------------------------------------------------
// Usage
// ---------------------------------------------------------------------------

/// Token usage and cost for a single LLM response.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Usage {
    pub input: u64,
    pub output: u64,
    pub cache_read: u64,
    pub cache_write: u64,
    /// Subset of `cache_write` written with 1h retention. Only Anthropic reports this split.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cache_write_1h: Option<u64>,
    pub total_tokens: u64,
    pub cost: UsageCost,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct UsageCost {
    pub input: f64,
    pub output: f64,
    pub cache_read: f64,
    pub cache_write: f64,
    pub total: f64,
}

// ---------------------------------------------------------------------------
// Stop reasons
// ---------------------------------------------------------------------------

/// Why the model stopped generating.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum StopReason {
    Stop,
    Length,
    ToolUse,
    Error,
    Aborted,
}

// ---------------------------------------------------------------------------
// Messages
// ---------------------------------------------------------------------------

/// A message from the user.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct UserMessage {
    pub role: UserRole,
    #[serde(default)]
    pub content: Vec<MessageContent>,
    pub timestamp: DateTime<Utc>,
}

/// A message from the assistant (LLM).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct AssistantMessage {
    pub role: AssistantRole,
    pub content: Vec<AssistantContentBlock>,
    pub api: String,
    pub provider: String,
    pub model: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_id: Option<String>,
    pub usage: Usage,
    pub stop_reason: StopReason,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
    /// Redacted provider/runtime diagnostics for failures and recoveries.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diagnostics: Option<Vec<crate::utils::diagnostics::AssistantMessageDiagnostic>>,
    pub timestamp: DateTime<Utc>,
}

impl crate::utils::diagnostics::WithDiagnostics for AssistantMessage {
    fn diagnostics_mut(
        &mut self,
    ) -> &mut Option<Vec<crate::utils::diagnostics::AssistantMessageDiagnostic>> {
        &mut self.diagnostics
    }
}

/// A tool result message.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ToolResultMessage {
    pub role: ToolResultRole,
    pub tool_call_id: String,
    pub tool_name: String,
    #[serde(default)]
    pub content: Vec<MessageContent>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
    #[serde(default)]
    pub is_error: bool,
    pub timestamp: DateTime<Utc>,
}

// ---------------------------------------------------------------------------
// Role constants (discriminators)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum MessageRole {
    User,
    Assistant,
    ToolResult,
}

pub type UserRole = MessageRole; // always MessageRole::User
pub type AssistantRole = MessageRole; // always MessageRole::Assistant
pub type ToolResultRole = MessageRole; // always MessageRole::ToolResult

// ---------------------------------------------------------------------------
// Content block unions
// ---------------------------------------------------------------------------

/// Content block within a user or tool-result message (text + images, no tool calls).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type")]
pub enum MessageContent {
    #[serde(rename = "text")]
    Text(TextContent),
    #[serde(rename = "image")]
    Image(ImageContent),
}

/// Content block within an assistant message (text, thinking, tool calls).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type")]
pub enum AssistantContentBlock {
    #[serde(rename = "text")]
    Text(TextContent),
    #[serde(rename = "thinking")]
    Thinking(ThinkingContent),
    #[serde(rename = "toolCall")]
    ToolCall(ToolCall),
}

// ---------------------------------------------------------------------------
// Top-level Message enum
// ---------------------------------------------------------------------------

/// Any message in a conversation transcript.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "role")]
pub enum Message {
    #[serde(rename = "user")]
    User(UserMessage),
    #[serde(rename = "assistant")]
    Assistant(AssistantMessage),
    #[serde(rename = "toolResult")]
    ToolResult(ToolResultMessage),
}

impl Message {
    pub fn role(&self) -> MessageRole {
        match self {
            Message::User(_) => MessageRole::User,
            Message::Assistant(_) => MessageRole::Assistant,
            Message::ToolResult(_) => MessageRole::ToolResult,
        }
    }
}

// ---------------------------------------------------------------------------
// Tool definition (TypeBox equivalent → JSON Schema via Schemars)
// ---------------------------------------------------------------------------

/// A tool definition exposed to the LLM.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Tool {
    pub name: String,
    pub description: String,
    /// JSON Schema for the tool's parameters.
    pub parameters: serde_json::Value,
}

// ---------------------------------------------------------------------------
// Context (what gets sent to the LLM)
// ---------------------------------------------------------------------------

/// The full context payload sent to a provider for one LLM call.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Context {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_prompt: Option<String>,
    #[serde(default)]
    pub messages: Vec<Message>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tools: Vec<Tool>,
}

// ---------------------------------------------------------------------------
// Model descriptor
// ---------------------------------------------------------------------------

/// Known API protocols.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum Api {
    OpenAiCompletions,
    MistralConversations,
    OpenAiResponses,
    AzureOpenAiResponses,
    OpenAiCodexResponses,
    AnthropicMessages,
    BedrockConverseStream,
    GoogleGenerativeAi,
    GoogleVertex,
}

impl std::fmt::Display for Api {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Api::OpenAiCompletions => write!(f, "openai-completions"),
            Api::MistralConversations => write!(f, "mistral-conversations"),
            Api::OpenAiResponses => write!(f, "openai-responses"),
            Api::AzureOpenAiResponses => write!(f, "azure-openai-responses"),
            Api::OpenAiCodexResponses => write!(f, "openai-codex-responses"),
            Api::AnthropicMessages => write!(f, "anthropic-messages"),
            Api::BedrockConverseStream => write!(f, "bedrock-converse-stream"),
            Api::GoogleGenerativeAi => write!(f, "google-generative-ai"),
            Api::GoogleVertex => write!(f, "google-vertex"),
        }
    }
}

/// A registered model with metadata.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Model {
    pub id: String,
    pub name: String,
    pub api: Api,
    pub provider: String,
    pub base_url: String,
    /// Whether the model supports extended thinking/reasoning.
    pub reasoning: bool,
    /// Maps hamr thinking levels to provider/model-specific values. Missing keys
    /// use provider defaults; `None` (JSON `null`) marks a level as unsupported.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thinking_level_map: Option<ThinkingLevelMap>,
    /// Supported input modalities.
    #[serde(default)]
    pub input: Vec<InputModality>,
    pub cost: ModelCost,
    /// Context window size in tokens.
    pub context_window: u64,
    /// Maximum output tokens.
    pub max_tokens: u64,
    /// Extra HTTP headers to attach to requests for this model.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub headers: Option<std::collections::HashMap<String, String>>,
    /// Compatibility overrides for provider-specific behavior.
    /// If not set, auto-detected from baseUrl/provider.
    /// Provider modules deserialize this into their specific compat type.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub compat: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum InputModality {
    Text,
    Image,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ModelCost {
    /// USD per million input tokens.
    pub input: f64,
    /// USD per million output tokens.
    pub output: f64,
    /// USD per million cache-read tokens.
    pub cache_read: f64,
    /// USD per million cache-write tokens.
    pub cache_write: f64,
}

// ---------------------------------------------------------------------------
// Thinking level
// ---------------------------------------------------------------------------

/// Reasoning/thinking effort level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum ThinkingLevel {
    Minimal,
    Low,
    Medium,
    High,
    XHigh,
}

/// A model thinking level including the `off` sentinel.
///
/// Mirrors the TS `ModelThinkingLevel = "off" | ThinkingLevel`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum ModelThinkingLevel {
    Off,
    Minimal,
    Low,
    Medium,
    High,
    XHigh,
}

impl ModelThinkingLevel {
    /// Ordered list mirroring the TS `EXTENDED_THINKING_LEVELS`.
    pub const EXTENDED: [ModelThinkingLevel; 6] = [
        ModelThinkingLevel::Off,
        ModelThinkingLevel::Minimal,
        ModelThinkingLevel::Low,
        ModelThinkingLevel::Medium,
        ModelThinkingLevel::High,
        ModelThinkingLevel::XHigh,
    ];
}

impl From<ThinkingLevel> for ModelThinkingLevel {
    fn from(level: ThinkingLevel) -> Self {
        match level {
            ThinkingLevel::Minimal => ModelThinkingLevel::Minimal,
            ThinkingLevel::Low => ModelThinkingLevel::Low,
            ThinkingLevel::Medium => ModelThinkingLevel::Medium,
            ThinkingLevel::High => ModelThinkingLevel::High,
            ThinkingLevel::XHigh => ModelThinkingLevel::XHigh,
        }
    }
}

/// Maps thinking levels to provider/model-specific values.
///
/// Mirrors the TS `ThinkingLevelMap = Partial<Record<ModelThinkingLevel, string | null>>`:
/// an absent key uses the provider default, a present `None` marks the level
/// unsupported, and a present `Some(value)` maps to a provider-specific string.
pub type ThinkingLevelMap = std::collections::HashMap<ModelThinkingLevel, Option<String>>;

/// Per-level thinking token budgets (mirrors the TS
/// `Partial<Record<"minimal" | "low" | "medium" | "high", number>>`).
#[derive(Debug, Clone, Copy, Default)]
pub struct ThinkingBudgets {
    pub minimal: Option<u64>,
    pub low: Option<u64>,
    pub medium: Option<u64>,
    pub high: Option<u64>,
}

impl ThinkingBudgets {
    /// Validate that any present budget values are non-zero.
    /// Returns `None` values as-is (they mean "not set").
    pub fn validate(&self) -> Result<(), String> {
        for (name, val) in [
            ("minimal", self.minimal),
            ("low", self.low),
            ("medium", self.medium),
            ("high", self.high),
        ] {
            if let Some(v) = val {
                if v == 0 {
                    return Err(format!(
                        "Thinking budget for level '{}' must be > 0, got 0",
                        name
                    ));
                }
            }
        }
        Ok(())
    }

    /// Return the budget for a given thinking level, or `None` if not configured.
    pub fn get(&self, level: ThinkingLevel) -> Option<u64> {
        match level {
            ThinkingLevel::Minimal => self.minimal,
            ThinkingLevel::Low => self.low,
            ThinkingLevel::Medium => self.medium,
            ThinkingLevel::High => self.high,
            ThinkingLevel::XHigh => self.high,
        }
    }
}

// ---------------------------------------------------------------------------
// Transport
// ---------------------------------------------------------------------------

/// Preferred transport protocol for provider streaming.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Transport {
    Sse,
    WebSocket,
    WebSocketCached,
    Auto,
}

// ---------------------------------------------------------------------------
// Cache retention
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum CacheRetention {
    None,
    Short,
    Long,
}

// ---------------------------------------------------------------------------
// Stream options
// ---------------------------------------------------------------------------

/// Callback for inspecting or replacing provider payloads before sending.
///
/// Mirrors the TS `onPayload?: (payload, model) => unknown | undefined | Promise<...>`.
/// Returning `None` keeps the payload unchanged.
pub type OnPayloadCallback = Arc<
    dyn Fn(
            serde_json::Value,
            Model,
        ) -> Pin<Box<dyn Future<Output = Option<serde_json::Value>> + Send>>
        + Send
        + Sync,
>;

/// Callback invoked after an HTTP response is received and before its body is consumed.
///
/// Mirrors the TS `onResponse?: (response, model) => void | Promise<void>`.
pub type OnResponseCallback =
    Arc<dyn Fn(ProviderResponse, Model) -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + Sync>;

/// Options passed to `stream()` and every provider backend.
///
/// This is a runtime configuration struct — never serialized/deserialized.
/// Mirrors the TS `interface StreamOptions`.
///
/// `Debug` is implemented manually because the callback fields are not `Debug`.
#[derive(Clone)]
pub struct StreamOptions {
    pub temperature: Option<f64>,
    pub max_tokens: Option<u64>,
    /// Abort signal — the stream is cancelled when this receiver sees `true`.
    pub signal: Option<tokio::sync::watch::Receiver<bool>>,
    pub api_key: Option<String>,
    /// Preferred transport for providers that support multiple transports.
    pub transport: Option<Transport>,
    /// Prompt cache retention preference. Default: `Short`.
    pub cache_retention: Option<CacheRetention>,
    /// Optional session identifier for providers that support session-based caching.
    pub session_id: Option<String>,
    /// Inspect/replace provider payloads before sending.
    pub on_payload: Option<OnPayloadCallback>,
    /// Invoked after an HTTP response is received, before the body is consumed.
    pub on_response: Option<OnResponseCallback>,
    /// Optional custom HTTP headers to include in API requests.
    pub headers: Option<std::collections::HashMap<String, String>>,
    /// HTTP request timeout in milliseconds.
    pub timeout_ms: Option<u64>,
    /// WebSocket connect timeout in milliseconds (connection/open handshake only).
    pub websocket_connect_timeout_ms: Option<u64>,
    /// Maximum retry attempts for providers/SDKs that support client-side retries.
    pub max_retries: Option<u32>,
    /// Maximum delay in milliseconds to wait for a server-requested retry.
    /// Default: 60000. Set to 0 to disable the cap.
    pub max_retry_delay_ms: Option<u64>,
    /// Optional metadata to include in API requests.
    pub metadata: Option<std::collections::HashMap<String, serde_json::Value>>,
    /// Provider-scoped environment values (precedence over `std::env`).
    pub env: Option<ProviderEnv>,
}

impl std::fmt::Debug for StreamOptions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StreamOptions")
            .field("temperature", &self.temperature)
            .field("max_tokens", &self.max_tokens)
            .field("signal", &self.signal.as_ref().map(|_| "<signal>"))
            .field("api_key", &self.api_key.as_ref().map(|_| "<redacted>"))
            .field("transport", &self.transport)
            .field("cache_retention", &self.cache_retention)
            .field("session_id", &self.session_id)
            .field("on_payload", &self.on_payload.as_ref().map(|_| "<fn>"))
            .field("on_response", &self.on_response.as_ref().map(|_| "<fn>"))
            .field("headers", &self.headers)
            .field("timeout_ms", &self.timeout_ms)
            .field(
                "websocket_connect_timeout_ms",
                &self.websocket_connect_timeout_ms,
            )
            .field("max_retries", &self.max_retries)
            .field("max_retry_delay_ms", &self.max_retry_delay_ms)
            .field("metadata", &self.metadata)
            .field("env", &self.env)
            .finish()
    }
}

impl Default for StreamOptions {
    fn default() -> Self {
        Self {
            temperature: None,
            max_tokens: None,
            signal: None,
            api_key: None,
            transport: None,
            cache_retention: None,
            session_id: None,
            on_payload: None,
            on_response: None,
            headers: None,
            timeout_ms: None,
            websocket_connect_timeout_ms: None,
            max_retries: None,
            max_retry_delay_ms: None,
            metadata: None,
            env: None,
        }
    }
}

/// Provider-scoped stream options: TS `StreamOptions & Record<string, unknown>`.
///
/// **Type debt:** the TS intersection allows arbitrary extra keys (`Record<string,
/// unknown>`) for provider-specific options. We model that as the base options plus
/// an `extra` bag of untyped JSON values. Providers that need extra fields read
/// from `extra`.
#[derive(Clone, Debug, Default)]
pub struct ProviderStreamOptions {
    pub base: StreamOptions,
    pub extra: std::collections::HashMap<String, serde_json::Value>,
}

/// Unified options with reasoning, passed to `stream_simple()` and `complete_simple()`.
///
/// Mirrors the TS `interface SimpleStreamOptions extends StreamOptions`. The base
/// `StreamOptions` is embedded so [`crate::providers::simple_options::build_base_options`]
/// can copy it directly.
#[derive(Clone, Debug, Default)]
pub struct SimpleStreamOptions {
    pub base: StreamOptions,
    /// Reasoning/thinking effort level.
    pub reasoning: Option<ThinkingLevel>,
    /// Custom token budgets for thinking levels (token-based providers only).
    pub thinking_budgets: Option<ThinkingBudgets>,
}

// ---------------------------------------------------------------------------
// Assistant message events (streaming protocol)
// ---------------------------------------------------------------------------

/// Streaming event emitted during an LLM response.
///
/// Mirror of the TypeScript `AssistantMessageEvent` discriminated union.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all_fields = "camelCase")]
pub enum AssistantMessageEvent {
    #[serde(rename = "start")]
    Start { partial: AssistantMessage },
    #[serde(rename = "text_start")]
    TextStart {
        content_index: usize,
        partial: AssistantMessage,
    },
    #[serde(rename = "text_delta")]
    TextDelta {
        content_index: usize,
        delta: String,
        partial: AssistantMessage,
    },
    #[serde(rename = "text_end")]
    TextEnd {
        content_index: usize,
        content: String,
        partial: AssistantMessage,
    },
    #[serde(rename = "thinking_start")]
    ThinkingStart {
        content_index: usize,
        partial: AssistantMessage,
    },
    #[serde(rename = "thinking_delta")]
    ThinkingDelta {
        content_index: usize,
        delta: String,
        partial: AssistantMessage,
    },
    #[serde(rename = "thinking_end")]
    ThinkingEnd {
        content_index: usize,
        content: String,
        partial: AssistantMessage,
    },
    #[serde(rename = "toolcall_start")]
    ToolCallStart {
        content_index: usize,
        partial: AssistantMessage,
    },
    #[serde(rename = "toolcall_delta")]
    ToolCallDelta {
        content_index: usize,
        delta: String,
        partial: AssistantMessage,
    },
    #[serde(rename = "toolcall_end")]
    ToolCallEnd {
        content_index: usize,
        tool_call: ToolCall,
        partial: AssistantMessage,
    },
    #[serde(rename = "done")]
    Done {
        reason: DoneReason,
        message: AssistantMessage,
    },
    #[serde(rename = "error")]
    Error {
        reason: ErrorReason,
        error: AssistantMessage,
    },
    #[serde(rename = "loading")]
    Loading { model: String, elapsed_ms: u64 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum DoneReason {
    Stop,
    Length,
    ToolUse,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum ErrorReason {
    Aborted,
    Error,
}

// ---------------------------------------------------------------------------
// Provider response
// ---------------------------------------------------------------------------

/// HTTP response metadata from a provider call.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderResponse {
    pub status: u16,
    pub headers: std::collections::HashMap<String, String>,
}

// ---------------------------------------------------------------------------
// Provider-scoped environment
// ---------------------------------------------------------------------------

/// Provider-scoped environment overrides (precedence over `std::env`).
pub type ProviderEnv = std::collections::HashMap<String, String>;

// ---------------------------------------------------------------------------
// Re-exports (mirror `export type { ... } from "./utils/event-stream.ts"`)
// ---------------------------------------------------------------------------

pub use crate::utils::event_stream::AssistantMessageEventStream;
