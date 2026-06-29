//! Core agent types.
//!
//! Mirror of `packages/agent/src/types.ts`.
//!
//! These types extend the base LLM types from [`hamr_ai`] with agent-specific
//! concepts: tool execution, lifecycle events, steering/follow-up queues,
//! compaction hints, and the extension surface.

use hamr_ai::stream::StreamError;
use hamr_ai::types::*;
use serde::{Deserialize, Serialize};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

// ---------------------------------------------------------------------------
// Agent tool definition
// ---------------------------------------------------------------------------

/// A tool that the agent can execute.
///
/// Extends the base [`Tool`] with an `execute` function and optional hooks
/// for argument preparation, execution mode overrides, and UI labels.
///
/// Contains closures — not serializable.
#[derive(Clone)]
pub struct AgentTool {
    /// Human-readable label for UI display.
    pub label: String,
    /// Stable model-visible tool name.
    pub name: String,
    /// Model-visible tool description.
    pub description: String,
    /// JSON schema parameters definition.
    pub parameters: serde_json::Value,
    /// Optional compat shim for raw tool-call args before schema validation.
    pub prepare_arguments:
        Option<Arc<dyn Fn(serde_json::Value) -> serde_json::Value + Send + Sync>>,
    /// Per-tool execution mode override.
    pub execution_mode: Option<ToolExecutionMode>,
    /// Execute the tool. Returns a result or an error.
    pub execute: Arc<
        dyn Fn(
                String,                                     // tool_call_id
                serde_json::Value,                          // params
                Option<tokio::sync::watch::Receiver<bool>>, // signal
                Option<AgentToolUpdateCallback>,            // on_update
            )
                -> std::pin::Pin<Box<dyn std::future::Future<Output = AgentToolResult> + Send>>
            + Send
            + Sync,
    >,
}

pub type AgentToolUpdateCallback = Arc<dyn Fn(AgentToolResult) + Send + Sync>;

impl AgentTool {
    pub fn to_llm_tool(&self) -> Tool {
        Tool {
            name: self.name.clone(),
            description: self.description.clone(),
            parameters: self.parameters.clone(),
        }
    }
}

/// Result produced by a tool execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentToolResult {
    /// Text or image content returned to the model.
    pub content: Vec<MessageContent>,
    /// Arbitrary structured details for logs or UI rendering.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
    /// Whether the tool failed.
    #[serde(default)]
    pub is_error: bool,
    /// Hint that the agent should stop after the current tool batch.
    #[serde(default)]
    pub terminate: bool,
}

// ---------------------------------------------------------------------------
// Tool execution mode
// ---------------------------------------------------------------------------

/// Controls how tool calls within a single assistant message are executed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ToolExecutionMode {
    /// Each tool call runs to completion before the next one starts.
    Sequential,
    /// Multiple tool calls run concurrently.
    Parallel,
}

// ---------------------------------------------------------------------------
// Queue mode
// ---------------------------------------------------------------------------

/// Controls how queued messages are drained.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum QueueMode {
    /// Drain and inject all queued messages at once.
    All,
    /// Drain and inject one message at a time.
    OneAtATime,
}

// ---------------------------------------------------------------------------
// Agent message (extends LLM Message with custom types)
// ---------------------------------------------------------------------------

/// Content payload for a custom user-visible message.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum CustomMessageContent {
    Text(String),
    Blocks(Vec<MessageContent>),
}

/// Message emitted when the harness runs a bash command.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BashExecutionMessage {
    pub command: String,
    pub output: String,
    pub exit_code: Option<i32>,
    pub cancelled: bool,
    pub truncated: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub full_output_path: Option<String>,
    pub timestamp: i64,
    #[serde(default)]
    pub exclude_from_context: bool,
}

/// Generic custom message persisted by the harness.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CustomMessage {
    pub custom_type: String,
    pub content: CustomMessageContent,
    pub display: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
    pub timestamp: i64,
}

/// Branch-summary message shown to the model as a synthetic user message.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BranchSummaryMessage {
    pub summary: String,
    pub from_id: String,
    pub timestamp: i64,
}

/// Compaction-summary message shown to the model as a synthetic user message.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CompactionSummaryMessage {
    pub summary: String,
    pub tokens_before: u64,
    pub timestamp: i64,
}

/// Any message in the agent transcript — base LLM messages + harness custom messages.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "role")]
pub enum AgentMessage {
    #[serde(rename = "user")]
    User(UserMessage),
    #[serde(rename = "assistant")]
    Assistant(AssistantMessage),
    #[serde(rename = "toolResult")]
    ToolResult(ToolResultMessage),
    #[serde(rename = "bashExecution")]
    BashExecution(BashExecutionMessage),
    #[serde(rename = "custom")]
    Custom(CustomMessage),
    #[serde(rename = "branchSummary")]
    BranchSummary(BranchSummaryMessage),
    #[serde(rename = "compactionSummary")]
    CompactionSummary(CompactionSummaryMessage),
}

/// Deserialize a persisted message by its `role` discriminator.
///
/// `AgentMessage` wraps base message structs that also carry a role field, so
/// routing to the concrete struct avoids serde's duplicate-tag ambiguity.
pub fn agent_message_from_value(value: serde_json::Value) -> Result<AgentMessage, String> {
    let role = value
        .get("role")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| "missing role".to_string())?;
    match role {
        "user" => serde_json::from_value(value)
            .map(AgentMessage::User)
            .map_err(|error| error.to_string()),
        "assistant" => serde_json::from_value(value)
            .map(AgentMessage::Assistant)
            .map_err(|error| error.to_string()),
        "toolResult" => serde_json::from_value(value)
            .map(AgentMessage::ToolResult)
            .map_err(|error| error.to_string()),
        "bashExecution" => serde_json::from_value(value)
            .map(AgentMessage::BashExecution)
            .map_err(|error| error.to_string()),
        "custom" => serde_json::from_value(value)
            .map(AgentMessage::Custom)
            .map_err(|error| error.to_string()),
        "branchSummary" => serde_json::from_value(value)
            .map(AgentMessage::BranchSummary)
            .map_err(|error| error.to_string()),
        "compactionSummary" => serde_json::from_value(value)
            .map(AgentMessage::CompactionSummary)
            .map_err(|error| error.to_string()),
        _ => Err(format!("unknown role: {role}")),
    }
}

// ---------------------------------------------------------------------------
// Agent context (snapshot passed into the loop)
// ---------------------------------------------------------------------------

/// A snapshot of the agent's state passed into each loop invocation.
#[derive(Clone)]
pub struct AgentContext {
    pub system_prompt: String,
    pub messages: Vec<AgentMessage>,
    pub tools: Vec<AgentTool>,
}

// ---------------------------------------------------------------------------
// Compaction result (for events)
// ---------------------------------------------------------------------------

/// Result of a successful compaction, included in CompactionEnd events.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompactionResult {
    pub summary: String,
    pub tokens_before: u64,
    pub tokens_after: u64,
}

// ---------------------------------------------------------------------------
// Agent events
// ---------------------------------------------------------------------------

/// Every event emitted by the agent loop.
///
/// Mirror of the TypeScript `AgentEvent` discriminated union.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum AgentEvent {
    // Agent lifecycle
    #[serde(rename = "agent_start")]
    AgentStart,

    #[serde(rename = "agent_end")]
    AgentEnd { messages: Vec<AgentMessage> },

    // Turn lifecycle
    #[serde(rename = "turn_start")]
    TurnStart,

    #[serde(rename = "turn_end")]
    TurnEnd {
        message: AgentMessage,
        tool_results: Vec<ToolResultMessage>,
    },

    // Message lifecycle
    #[serde(rename = "message_start")]
    MessageStart { message: AgentMessage },

    #[serde(rename = "message_update")]
    MessageUpdate {
        message: AgentMessage,
        assistant_message_event: AssistantMessageEvent,
    },

    #[serde(rename = "message_end")]
    MessageEnd { message: AgentMessage },

    // Tool execution lifecycle
    #[serde(rename = "tool_execution_start")]
    ToolExecutionStart {
        tool_call_id: String,
        tool_name: String,
        args: serde_json::Value,
    },

    #[serde(rename = "tool_execution_update")]
    ToolExecutionUpdate {
        tool_call_id: String,
        tool_name: String,
        args: serde_json::Value,
        partial_result: serde_json::Value,
    },

    #[serde(rename = "tool_execution_end")]
    ToolExecutionEnd {
        tool_call_id: String,
        tool_name: String,
        result: serde_json::Value,
        is_error: bool,
    },

    // Model cold-start progress
    #[serde(rename = "model_loading")]
    ModelLoading { model: String, elapsed_ms: u64 },

    // Compaction lifecycle
    #[serde(rename = "compaction_start")]
    CompactionStart { reason: String },

    #[serde(rename = "compaction_end")]
    CompactionEnd {
        aborted: bool,
        reason: String,
        result: Option<CompactionResult>,
    },

    #[serde(rename = "compaction_summary")]
    CompactionSummary { summary: String, tokens_before: u64 },
}

// ---------------------------------------------------------------------------
// Agent loop config
// ---------------------------------------------------------------------------

/// Configuration for one agent loop invocation.
pub struct AgentLoopConfig {
    pub model: Model,
    pub reasoning: Option<ThinkingLevel>,
    pub session_id: Option<String>,
    pub transport: Option<Transport>,
    pub tool_execution: ToolExecutionMode,
    pub max_retry_delay_ms: Option<u64>,
    pub convert_to_llm: Arc<
        dyn Fn(Vec<AgentMessage>) -> Pin<Box<dyn Future<Output = Vec<Message>> + Send>>
            + Send
            + Sync,
    >,
    pub transform_context: Option<
        Arc<
            dyn Fn(
                    Vec<AgentMessage>,
                    Option<tokio::sync::watch::Receiver<bool>>,
                ) -> Pin<Box<dyn Future<Output = Vec<AgentMessage>> + Send>>
                + Send
                + Sync,
        >,
    >,
    pub get_api_key: Option<
        Arc<dyn Fn(String) -> Pin<Box<dyn Future<Output = Option<String>> + Send>> + Send + Sync>,
    >,
    pub should_stop_after_turn: Option<
        Arc<
            dyn Fn(ShouldStopAfterTurnContext) -> Pin<Box<dyn Future<Output = bool> + Send>>
                + Send
                + Sync,
        >,
    >,
    pub prepare_next_turn: Option<
        Arc<
            dyn Fn(
                    PrepareNextTurnContext,
                )
                    -> Pin<Box<dyn Future<Output = Option<AgentLoopTurnUpdate>> + Send>>
                + Send
                + Sync,
        >,
    >,
    pub get_steering_messages: Option<
        Arc<dyn Fn() -> Pin<Box<dyn Future<Output = Vec<AgentMessage>> + Send>> + Send + Sync>,
    >,
    pub get_follow_up_messages: Option<
        Arc<dyn Fn() -> Pin<Box<dyn Future<Output = Vec<AgentMessage>> + Send>> + Send + Sync>,
    >,
    pub before_tool_call: Option<
        Arc<
            dyn Fn(
                    BeforeToolCallContext,
                    Option<tokio::sync::watch::Receiver<bool>>,
                )
                    -> Pin<Box<dyn Future<Output = Option<BeforeToolCallResult>> + Send>>
                + Send
                + Sync,
        >,
    >,
    pub after_tool_call: Option<
        Arc<
            dyn Fn(
                    AfterToolCallContext,
                    Option<tokio::sync::watch::Receiver<bool>>,
                )
                    -> Pin<Box<dyn Future<Output = Option<AfterToolCallResult>> + Send>>
                + Send
                + Sync,
        >,
    >,
}

// ---------------------------------------------------------------------------
// Before/after tool call hooks
// ---------------------------------------------------------------------------

/// Context passed to `before_tool_call` hooks.
pub struct BeforeToolCallContext {
    pub assistant_message: AssistantMessage,
    pub tool_call: ToolCall,
    pub args: serde_json::Value,
    pub context: AgentContext,
}

/// Result from a `before_tool_call` hook.
pub struct BeforeToolCallResult {
    /// Block the tool from executing.
    pub block: bool,
    /// Reason shown in the error tool result when blocked.
    pub reason: Option<String>,
}

/// Context passed to `after_tool_call` hooks.
pub struct AfterToolCallContext {
    pub assistant_message: AssistantMessage,
    pub tool_call: ToolCall,
    pub args: serde_json::Value,
    pub result: AgentToolResult,
    pub is_error: bool,
    pub context: AgentContext,
}

/// Partial override returned from `after_tool_call`.
pub struct AfterToolCallResult {
    pub content: Option<Vec<MessageContent>>,
    pub details: Option<serde_json::Value>,
    pub is_error: Option<bool>,
    pub terminate: Option<bool>,
}

// ---------------------------------------------------------------------------
// Should-stop / prepare-next-turn
// ---------------------------------------------------------------------------

/// Context passed to `should_stop_after_turn`.
pub struct ShouldStopAfterTurnContext {
    pub message: AssistantMessage,
    pub tool_results: Vec<ToolResultMessage>,
    pub context: AgentContext,
    pub new_messages: Vec<AgentMessage>,
}

/// Returned by `prepare_next_turn` to override state.
pub struct AgentLoopTurnUpdate {
    pub context: Option<AgentContext>,
    pub model: Option<Model>,
    pub thinking_level: Option<ThinkingLevel>,
}

pub type PrepareNextTurnContext = ShouldStopAfterTurnContext;

pub type StreamFn = Arc<
    dyn Fn(
            Model,
            Context,
            Option<SimpleStreamOptions>,
        )
            -> Pin<Box<dyn Future<Output = Result<AssistantMessageEventStream, StreamError>> + Send>>
        + Send
        + Sync,
>;

// ---------------------------------------------------------------------------
// Agent state (public read surface)
// ---------------------------------------------------------------------------

/// Public read-only view of agent state.
pub struct AgentState {
    pub system_prompt: String,
    pub model: Model,
    pub thinking_level: ThinkingLevel,
    pub tools: Vec<String>,
    pub messages: Vec<AgentMessage>,
    pub is_streaming: bool,
    pub streaming_message: Option<AgentMessage>,
    pub pending_tool_calls: std::collections::HashSet<String>,
    pub error_message: Option<String>,
}
