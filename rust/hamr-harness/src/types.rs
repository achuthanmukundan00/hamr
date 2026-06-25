//! Core agent types.
//!
//! Mirror of `packages/agent/src/types.ts`.
//!
//! These types extend the base LLM types from [`hamr_ai`] with agent-specific
//! concepts: tool execution, lifecycle events, steering/follow-up queues,
//! compaction hints, and the extension surface.

use hamr_ai::types::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
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
    /// The base tool definition (name, description, JSON Schema parameters).
    pub tool: Tool,
    /// Optional compat shim for raw tool-call args before schema validation.
    pub prepare_arguments: Option<Arc<dyn Fn(serde_json::Value) -> serde_json::Value + Send + Sync>>,
    /// Per-tool execution mode override.
    pub execution_mode: Option<ToolExecutionMode>,
    /// Execute the tool. Returns a result or an error.
    pub execute: Arc<
        dyn Fn(
                String,                   // tool_call_id
                serde_json::Value,         // params
                Option<tokio::sync::watch::Receiver<bool>>, // signal
            ) -> std::pin::Pin<
                Box<dyn std::future::Future<Output = AgentToolResult> + Send>,
            >
            + Send
            + Sync,
    >,
}

/// Result produced by a tool execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentToolResult {
    /// Text or image content returned to the model.
    pub content: Vec<MessageContent>,
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

/// Custom agent message types can be added via an extensible enum.
/// This mirrors TypeScript's `CustomAgentMessages` interface + declaration merging.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "customType")]
pub enum CustomAgentMessage {
    /// Placeholder — extended via declaration merging in TypeScript,
    /// and by matching on unknown tags at runtime in Rust.
    #[serde(untagged)]
    Unknown(serde_json::Value),
}

/// Any message in the agent transcript — LLM messages + custom messages.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AgentMessage {
    Llm(Message),
    Custom(CustomAgentMessage),
}

// ---------------------------------------------------------------------------
// Agent context (snapshot passed into the loop)
// ---------------------------------------------------------------------------

/// A snapshot of the agent's state passed into each loop invocation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentContext {
    pub system_prompt: String,
    pub messages: Vec<AgentMessage>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tools: Vec<String>, // tool names — actual tool defs are looked up
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
    ModelLoading {
        model: String,
        elapsed_ms: u64,
    },
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
