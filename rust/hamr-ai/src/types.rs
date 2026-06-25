//! Core types for the hamr AI layer.
//!
//! This module defines every message shape, model descriptor, tool contract, and
//! streaming event used across the entire agent stack. It mirrors `packages/ai/src/types.ts`
//! exactly, translated from TypeScript discriminated unions to Rust enums.

use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Content blocks
// ---------------------------------------------------------------------------

/// A text content block within a message.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TextContent {
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text_signature: Option<String>,
}

/// A thinking/reasoning content block (redacted on safety filter).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
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
pub struct ImageContent {
    /// Base64-encoded image data.
    pub data: String,
    /// MIME type, e.g. `"image/png"`.
    pub mime_type: String,
}

/// A tool call requested by the assistant.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
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
pub struct Usage {
    pub input: u64,
    pub output: u64,
    pub cache_read: u64,
    pub cache_write: u64,
    pub total_tokens: u64,
    pub cost: UsageCost,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
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
pub struct UserMessage {
    pub role: UserRole,
    #[serde(default)]
    pub content: Vec<MessageContent>,
    pub timestamp: DateTime<Utc>,
}

/// A message from the assistant (LLM).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
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
    pub timestamp: DateTime<Utc>,
}

/// A tool result message.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ToolResultMessage {
    pub role: ToolResultRole,
    pub tool_call_id: String,
    pub tool_name: String,
    #[serde(default)]
    pub content: Vec<MessageContent>,
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

pub type UserRole = MessageRole;       // always MessageRole::User
pub type AssistantRole = MessageRole;  // always MessageRole::Assistant
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

// ---------------------------------------------------------------------------
// Tool definition (TypeBox equivalent → JSON Schema via Schemars)
// ---------------------------------------------------------------------------

/// A tool definition exposed to the LLM.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
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

/// A registered model with metadata.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Model {
    pub id: String,
    pub name: String,
    pub api: Api,
    pub provider: String,
    pub base_url: String,
    /// Whether the model supports extended thinking/reasoning.
    pub reasoning: bool,
    /// Supported input modalities.
    #[serde(default)]
    pub input: Vec<InputModality>,
    pub cost: ModelCost,
    /// Context window size in tokens.
    pub context_window: u64,
    /// Maximum output tokens.
    pub max_tokens: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum InputModality {
    Text,
    Image,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
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

/// Options passed to `streamSimple()` and every provider backend.
///
/// This is a runtime configuration struct — never serialized/deserialized.
#[derive(Debug, Clone)]
pub struct StreamOptions {
    pub temperature: Option<f64>,
    pub max_tokens: Option<u64>,
    pub reasoning: Option<ThinkingLevel>,
    pub transport: Option<Transport>,
    pub cache_retention: Option<CacheRetention>,
    pub session_id: Option<String>,
    pub api_key: Option<String>,
    /// Abort signal — the stream is cancelled when this receiver sees `true`.
    pub signal: Option<tokio::sync::watch::Receiver<bool>>,
    pub timeout_ms: Option<u64>,
    pub max_retries: Option<u32>,
}

// ---------------------------------------------------------------------------
// Assistant message events (streaming protocol)
// ---------------------------------------------------------------------------

/// Streaming event emitted during an LLM response.
///
/// Mirror of the TypeScript `AssistantMessageEvent` discriminated union.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
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
pub struct ProviderResponse {
    pub status: u16,
    pub headers: std::collections::HashMap<String, String>,
}

// ---------------------------------------------------------------------------
// Provider-scoped environment
// ---------------------------------------------------------------------------

/// Provider-scoped environment overrides (precedence over `std::env`).
pub type ProviderEnv = std::collections::HashMap<String, String>;
