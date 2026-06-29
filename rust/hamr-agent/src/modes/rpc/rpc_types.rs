//! Port of `packages/coding-agent/src/modes/rpc/rpc_types.ts`
//!
//! RPC protocol types for headless operation.
//! Commands are sent as JSON lines on stdin.
//! Responses and events are emitted as JSON lines on stdout.

use serde::{Deserialize, Serialize};

// ============================================================================
// RPC Commands (stdin)
// ============================================================================

/// All RPC commands that can be sent to the agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RpcCommand {
    // Prompting
    Prompt {
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        message: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        images: Option<Vec<hamr_ai::types::ImageContent>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        streaming_behavior: Option<String>,
    },
    #[serde(rename = "steer")]
    Steer {
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        message: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        images: Option<Vec<hamr_ai::types::ImageContent>>,
    },
    #[serde(rename = "follow_up")]
    FollowUp {
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        message: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        images: Option<Vec<hamr_ai::types::ImageContent>>,
    },
    #[serde(rename = "abort")]
    Abort {
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<String>,
    },
    #[serde(rename = "new_session")]
    NewSession {
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        parent_session: Option<String>,
    },

    // State
    #[serde(rename = "get_state")]
    GetState {
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<String>,
    },

    // Model
    #[serde(rename = "set_model")]
    SetModel {
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        provider: String,
        #[serde(rename = "modelId")]
        model_id: String,
    },
    #[serde(rename = "cycle_model")]
    CycleModel {
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<String>,
    },
    #[serde(rename = "get_available_models")]
    GetAvailableModels {
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<String>,
    },

    // Thinking
    #[serde(rename = "set_thinking_level")]
    SetThinkingLevel {
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        level: hamr_ai::types::ThinkingLevel,
    },
    #[serde(rename = "cycle_thinking_level")]
    CycleThinkingLevel {
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<String>,
    },

    // Queue modes
    #[serde(rename = "set_steering_mode")]
    SetSteeringMode {
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        mode: String,
    },
    #[serde(rename = "set_follow_up_mode")]
    SetFollowUpMode {
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        mode: String,
    },

    // Compaction
    #[serde(rename = "compact")]
    Compact {
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        custom_instructions: Option<String>,
    },
    #[serde(rename = "set_auto_compaction")]
    SetAutoCompaction {
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        enabled: bool,
    },

    // Retry
    #[serde(rename = "set_auto_retry")]
    SetAutoRetry {
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        enabled: bool,
    },
    #[serde(rename = "abort_retry")]
    AbortRetry {
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<String>,
    },

    // Bash
    #[serde(rename = "bash")]
    Bash {
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        command: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        exclude_from_context: Option<bool>,
    },
    #[serde(rename = "abort_bash")]
    AbortBash {
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<String>,
    },

    // Session
    #[serde(rename = "get_session_stats")]
    GetSessionStats {
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<String>,
    },
    #[serde(rename = "export_html")]
    ExportHtml {
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        output_path: Option<String>,
    },
    #[serde(rename = "switch_session")]
    SwitchSession {
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        session_path: String,
    },
    #[serde(rename = "fork")]
    Fork {
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        entry_id: String,
    },
    #[serde(rename = "clone")]
    Clone {
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<String>,
    },
    #[serde(rename = "get_fork_messages")]
    GetForkMessages {
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<String>,
    },
    #[serde(rename = "get_last_assistant_text")]
    GetLastAssistantText {
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<String>,
    },
    #[serde(rename = "set_session_name")]
    SetSessionName {
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        name: String,
    },

    // Messages
    #[serde(rename = "get_messages")]
    GetMessages {
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<String>,
    },

    // Commands (available for invocation via prompt)
    #[serde(rename = "get_commands")]
    GetCommands {
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<String>,
    },
}

// ============================================================================
// RPC Slash Command (for get_commands response)
// ============================================================================

/// A command available for invocation via prompt.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RpcSlashCommand {
    /// Command name (without leading slash).
    pub name: String,
    /// Human-readable description.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// What kind of command this is.
    pub source: String,
    /// Source metadata for the owning resource (opaque JSON value).
    pub source_info: serde_json::Value,
}

// ============================================================================
// RPC Session State
// ============================================================================

/// State snapshot returned by get_state.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RpcSessionState {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<serde_json::Value>,
    pub thinking_level: hamr_ai::types::ThinkingLevel,
    pub is_streaming: bool,
    pub is_compacting: bool,
    pub steering_mode: String,
    pub follow_up_mode: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_file: Option<String>,
    pub session_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_name: Option<String>,
    pub auto_compaction_enabled: bool,
    pub message_count: usize,
    pub pending_message_count: usize,
}

// ============================================================================
// RPC Response (stdout)
// ============================================================================

/// A response to an RPC command.
/// Serializes as `{ type: "response", id, command, success, data?, error? }`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcResponse {
    pub r#type: String, // always "response"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub command: String,
    pub success: bool,
    /// Optional data payload (only present on success).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
    /// Error message (only present on failure).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl RpcResponse {
    pub fn success(id: Option<String>, command: &str, data: Option<serde_json::Value>) -> Self {
        Self {
            r#type: "response".to_string(),
            id,
            command: command.to_string(),
            success: true,
            data,
            error: None,
        }
    }

    pub fn failure(id: Option<String>, command: &str, error: String) -> Self {
        Self {
            r#type: "response".to_string(),
            id,
            command: command.to_string(),
            success: false,
            data: None,
            error: Some(error),
        }
    }
}

// ============================================================================
// Extension UI Types
// ============================================================================

/// Emitted when an extension needs user input.
/// Serializes as `{ type: "extension_ui_request", id, method, ... }`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcExtensionUIRequest {
    pub r#type: String, // always "extension_ui_request"
    pub id: String,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub placeholder: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prefill: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notify_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status_text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub widget_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub widget_lines: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub widget_placement: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
}

/// Response to an extension UI request.
/// Serializes as `{ type: "extension_ui_response", id, value?, confirmed?, cancelled? }`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcExtensionUIResponse {
    pub r#type: String, // always "extension_ui_response"
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confirmed: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cancelled: Option<bool>,
}

// ============================================================================
// Command type extractor helper
// ============================================================================

pub type RpcCommandType = String;
