//! Port of `packages/agent/src/harness/types.ts`.
//!
//! Shared harness types used by the environment, prompt/skill loaders, and
//! higher-level session/compaction infrastructure.

use crate::types::{AgentMessage, CustomMessageContent};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::HashMap;
use std::sync::Arc;

/// Create a successful `Result`.
pub fn ok<T, E>(value: T) -> std::result::Result<T, E> {
    Ok(value)
}

/// Create a failed `Result`.
pub fn err<T, E>(error: E) -> std::result::Result<T, E> {
    Err(error)
}

/// Return the success value or panic with the error display.
pub fn get_or_throw<T, E: std::fmt::Display>(result: std::result::Result<T, E>) -> T {
    match result {
        Ok(value) => value,
        Err(error) => panic!("{error}"),
    }
}

/// Normalize arbitrary displayable values into a concrete I/O error.
pub fn to_error(error: impl std::fmt::Display) -> std::io::Error {
    std::io::Error::other(error.to_string())
}

/// Skill loaded from a `SKILL.md` file or provided by an application.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Skill {
    pub name: String,
    pub description: String,
    pub content: String,
    pub file_path: String,
    #[serde(default)]
    pub disable_model_invocation: bool,
}

/// Prompt template that can be explicitly invoked.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PromptTemplate {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub content: String,
}

/// Concrete resources available to prompt/system-prompt helpers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AgentHarnessResources<TSkill = Skill, TPromptTemplate = PromptTemplate> {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_templates: Option<Vec<TPromptTemplate>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skills: Option<Vec<TSkill>>,
}

/// Stable, backend-independent file error codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FileErrorCode {
    Aborted,
    NotFound,
    PermissionDenied,
    NotDirectory,
    IsDirectory,
    Invalid,
    NotSupported,
    Unknown,
}

/// Error returned by file operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileError {
    pub code: FileErrorCode,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
}

impl FileError {
    pub fn new(code: FileErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            path: None,
        }
    }

    pub fn with_path(mut self, path: impl Into<String>) -> Self {
        self.path = Some(path.into());
        self
    }
}

impl std::fmt::Display for FileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for FileError {}

/// Stable, backend-independent execution error codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionErrorCode {
    Aborted,
    Timeout,
    ShellUnavailable,
    SpawnError,
    CallbackError,
    Unknown,
}

/// Error returned by shell execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionError {
    pub code: ExecutionErrorCode,
    pub message: String,
}

impl ExecutionError {
    pub fn new(code: ExecutionErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

impl std::fmt::Display for ExecutionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for ExecutionError {}

/// Stable session subsystem error codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionErrorCode {
    NotFound,
    InvalidSession,
    InvalidEntry,
    InvalidForkTarget,
    Storage,
    Unknown,
}

/// Error thrown by session storage, repositories, and tree operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionError {
    pub code: SessionErrorCode,
    pub message: String,
}

impl SessionError {
    pub fn new(code: SessionErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

impl std::fmt::Display for SessionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for SessionError {}

/// Kind of filesystem object. Symlinks are not followed automatically.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FileKind {
    File,
    Directory,
    Symlink,
}

/// Metadata for one filesystem object.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileInfo {
    pub name: String,
    pub path: String,
    pub kind: FileKind,
    pub size: u64,
    pub mtime_ms: f64,
}

/// Options for `Shell::exec`.
pub struct ExecutionEnvExecOptions {
    pub cwd: Option<String>,
    pub env: Option<HashMap<String, String>>,
    pub timeout: Option<f64>,
    pub abort_signal: Option<tokio::sync::watch::Receiver<bool>>,
    pub on_stdout: Option<Arc<dyn Fn(String) + Send + Sync>>,
    pub on_stderr: Option<Arc<dyn Fn(String) + Send + Sync>>,
}

impl Default for ExecutionEnvExecOptions {
    fn default() -> Self {
        Self {
            cwd: None,
            env: None,
            timeout: None,
            abort_signal: None,
            on_stdout: None,
            on_stderr: None,
        }
    }
}

impl Clone for ExecutionEnvExecOptions {
    fn clone(&self) -> Self {
        Self {
            cwd: self.cwd.clone(),
            env: self.env.clone(),
            timeout: self.timeout,
            abort_signal: self.abort_signal.clone(),
            on_stdout: self.on_stdout.clone(),
            on_stderr: self.on_stderr.clone(),
        }
    }
}

/// Result of a shell execution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecResult {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionTreeEntryBase {
    pub id: String,
    pub parent_id: Option<String>,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageEntry {
    #[serde(flatten)]
    pub base: SessionTreeEntryBase,
    pub message: AgentMessage,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThinkingLevelChangeEntry {
    #[serde(flatten)]
    pub base: SessionTreeEntryBase,
    pub thinking_level: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelChangeEntry {
    #[serde(flatten)]
    pub base: SessionTreeEntryBase,
    pub provider: String,
    pub model_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActiveToolsChangeEntry {
    #[serde(flatten)]
    pub base: SessionTreeEntryBase,
    pub active_tool_names: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompactionEntry {
    #[serde(flatten)]
    pub base: SessionTreeEntryBase,
    pub summary: String,
    pub first_kept_entry_id: String,
    pub tokens_before: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from_hook: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BranchSummaryEntry {
    #[serde(flatten)]
    pub base: SessionTreeEntryBase,
    pub from_id: String,
    pub summary: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from_hook: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CustomEntry {
    #[serde(flatten)]
    pub base: SessionTreeEntryBase,
    pub custom_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomMessageEntry {
    #[serde(flatten)]
    pub base: SessionTreeEntryBase,
    pub custom_type: String,
    pub content: CustomMessageContent,
    pub display: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LabelEntry {
    #[serde(flatten)]
    pub base: SessionTreeEntryBase,
    pub target_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionInfoEntry {
    #[serde(flatten)]
    pub base: SessionTreeEntryBase,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LeafEntry {
    #[serde(flatten)]
    pub base: SessionTreeEntryBase,
    pub target_id: Option<String>,
}

#[derive(Debug, Clone)]
pub enum SessionTreeEntry {
    Message { entry: MessageEntry },
    ThinkingLevelChange { entry: ThinkingLevelChangeEntry },
    ModelChange { entry: ModelChangeEntry },
    ActiveToolsChange { entry: ActiveToolsChangeEntry },
    Compaction { entry: CompactionEntry },
    BranchSummary { entry: BranchSummaryEntry },
    Custom { entry: CustomEntry },
    CustomMessage { entry: CustomMessageEntry },
    Label { entry: LabelEntry },
    SessionInfo { entry: SessionInfoEntry },
    Leaf { entry: LeafEntry },
}

impl Serialize for SessionTreeEntry {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::ser::SerializeMap;

        let mut map = serializer.serialize_map(None)?;
        let (entry_type, base) = match self {
            SessionTreeEntry::Message { entry } => ("message", &entry.base),
            SessionTreeEntry::ThinkingLevelChange { entry } => {
                ("thinking_level_change", &entry.base)
            }
            SessionTreeEntry::ModelChange { entry } => ("model_change", &entry.base),
            SessionTreeEntry::ActiveToolsChange { entry } => ("active_tools_change", &entry.base),
            SessionTreeEntry::Compaction { entry } => ("compaction", &entry.base),
            SessionTreeEntry::BranchSummary { entry } => ("branch_summary", &entry.base),
            SessionTreeEntry::Custom { entry } => ("custom", &entry.base),
            SessionTreeEntry::CustomMessage { entry } => ("custom_message", &entry.base),
            SessionTreeEntry::Label { entry } => ("label", &entry.base),
            SessionTreeEntry::SessionInfo { entry } => ("session_info", &entry.base),
            SessionTreeEntry::Leaf { entry } => ("leaf", &entry.base),
        };

        map.serialize_entry("type", entry_type)?;
        map.serialize_entry("id", &base.id)?;
        map.serialize_entry("parentId", &base.parent_id)?;
        map.serialize_entry("timestamp", &base.timestamp)?;

        match self {
            SessionTreeEntry::Message { entry } => {
                map.serialize_entry("message", &agent_message_to_value(&entry.message))?;
            }
            SessionTreeEntry::ThinkingLevelChange { entry } => {
                map.serialize_entry("thinkingLevel", &entry.thinking_level)?;
            }
            SessionTreeEntry::ModelChange { entry } => {
                map.serialize_entry("provider", &entry.provider)?;
                map.serialize_entry("modelId", &entry.model_id)?;
            }
            SessionTreeEntry::ActiveToolsChange { entry } => {
                map.serialize_entry("activeToolNames", &entry.active_tool_names)?;
            }
            SessionTreeEntry::Compaction { entry } => {
                map.serialize_entry("summary", &entry.summary)?;
                map.serialize_entry("firstKeptEntryId", &entry.first_kept_entry_id)?;
                map.serialize_entry("tokensBefore", &entry.tokens_before)?;
                if let Some(details) = &entry.details {
                    map.serialize_entry("details", details)?;
                }
                if let Some(from_hook) = &entry.from_hook {
                    map.serialize_entry("fromHook", from_hook)?;
                }
            }
            SessionTreeEntry::BranchSummary { entry } => {
                map.serialize_entry("fromId", &entry.from_id)?;
                map.serialize_entry("summary", &entry.summary)?;
                if let Some(details) = &entry.details {
                    map.serialize_entry("details", details)?;
                }
                if let Some(from_hook) = &entry.from_hook {
                    map.serialize_entry("fromHook", from_hook)?;
                }
            }
            SessionTreeEntry::Custom { entry } => {
                map.serialize_entry("customType", &entry.custom_type)?;
                if let Some(data) = &entry.data {
                    map.serialize_entry("data", data)?;
                }
            }
            SessionTreeEntry::CustomMessage { entry } => {
                map.serialize_entry("customType", &entry.custom_type)?;
                map.serialize_entry("content", &entry.content)?;
                map.serialize_entry("display", &entry.display)?;
                if let Some(details) = &entry.details {
                    map.serialize_entry("details", details)?;
                }
            }
            SessionTreeEntry::Label { entry } => {
                map.serialize_entry("targetId", &entry.target_id)?;
                map.serialize_entry("label", &entry.label)?;
            }
            SessionTreeEntry::SessionInfo { entry } => {
                map.serialize_entry("name", &entry.name)?;
            }
            SessionTreeEntry::Leaf { entry } => {
                map.serialize_entry("targetId", &entry.target_id)?;
            }
        }

        map.end()
    }
}

impl<'de> Deserialize<'de> for SessionTreeEntry {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = serde_json::Value::deserialize(deserializer)?;
        let object = value
            .as_object()
            .ok_or_else(|| serde::de::Error::custom("session entry must be an object"))?;

        let entry_type = object
            .get("type")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| serde::de::Error::custom("missing entry type"))?;
        let id = object
            .get("id")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| serde::de::Error::custom("missing entry id"))?
            .to_string();
        let parent_id = match object.get("parentId") {
            Some(serde_json::Value::String(value)) => Some(value.clone()),
            Some(serde_json::Value::Null) | None => None,
            _ => return Err(serde::de::Error::custom("invalid parentId")),
        };
        let timestamp = object
            .get("timestamp")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| serde::de::Error::custom("missing timestamp"))?
            .to_string();
        let base = SessionTreeEntryBase {
            id,
            parent_id,
            timestamp,
        };

        match entry_type {
            "message" => Ok(SessionTreeEntry::Message {
                entry: MessageEntry {
                    base,
                    message: agent_message_from_value(
                        object
                            .get("message")
                            .cloned()
                            .ok_or_else(|| serde::de::Error::custom("missing message"))?,
                    )
                    .map_err(serde::de::Error::custom)?,
                },
            }),
            "thinking_level_change" => Ok(SessionTreeEntry::ThinkingLevelChange {
                entry: ThinkingLevelChangeEntry {
                    base,
                    thinking_level: object
                        .get("thinkingLevel")
                        .and_then(serde_json::Value::as_str)
                        .ok_or_else(|| serde::de::Error::custom("missing thinkingLevel"))?
                        .to_string(),
                },
            }),
            "model_change" => Ok(SessionTreeEntry::ModelChange {
                entry: ModelChangeEntry {
                    base,
                    provider: object
                        .get("provider")
                        .and_then(serde_json::Value::as_str)
                        .ok_or_else(|| serde::de::Error::custom("missing provider"))?
                        .to_string(),
                    model_id: object
                        .get("modelId")
                        .and_then(serde_json::Value::as_str)
                        .ok_or_else(|| serde::de::Error::custom("missing modelId"))?
                        .to_string(),
                },
            }),
            "active_tools_change" => Ok(SessionTreeEntry::ActiveToolsChange {
                entry: ActiveToolsChangeEntry {
                    base,
                    active_tool_names: serde_json::from_value(
                        object
                            .get("activeToolNames")
                            .cloned()
                            .ok_or_else(|| serde::de::Error::custom("missing activeToolNames"))?,
                    )
                    .map_err(serde::de::Error::custom)?,
                },
            }),
            "compaction" => Ok(SessionTreeEntry::Compaction {
                entry: CompactionEntry {
                    base,
                    summary: object
                        .get("summary")
                        .and_then(serde_json::Value::as_str)
                        .ok_or_else(|| serde::de::Error::custom("missing summary"))?
                        .to_string(),
                    first_kept_entry_id: object
                        .get("firstKeptEntryId")
                        .and_then(serde_json::Value::as_str)
                        .ok_or_else(|| serde::de::Error::custom("missing firstKeptEntryId"))?
                        .to_string(),
                    tokens_before: object
                        .get("tokensBefore")
                        .and_then(serde_json::Value::as_u64)
                        .ok_or_else(|| serde::de::Error::custom("missing tokensBefore"))?,
                    details: object.get("details").cloned(),
                    from_hook: object.get("fromHook").and_then(serde_json::Value::as_bool),
                },
            }),
            "branch_summary" => Ok(SessionTreeEntry::BranchSummary {
                entry: BranchSummaryEntry {
                    base,
                    from_id: object
                        .get("fromId")
                        .and_then(serde_json::Value::as_str)
                        .ok_or_else(|| serde::de::Error::custom("missing fromId"))?
                        .to_string(),
                    summary: object
                        .get("summary")
                        .and_then(serde_json::Value::as_str)
                        .ok_or_else(|| serde::de::Error::custom("missing summary"))?
                        .to_string(),
                    details: object.get("details").cloned(),
                    from_hook: object.get("fromHook").and_then(serde_json::Value::as_bool),
                },
            }),
            "custom" => Ok(SessionTreeEntry::Custom {
                entry: CustomEntry {
                    base,
                    custom_type: object
                        .get("customType")
                        .and_then(serde_json::Value::as_str)
                        .ok_or_else(|| serde::de::Error::custom("missing customType"))?
                        .to_string(),
                    data: object.get("data").cloned(),
                },
            }),
            "custom_message" => Ok(SessionTreeEntry::CustomMessage {
                entry: CustomMessageEntry {
                    base,
                    custom_type: object
                        .get("customType")
                        .and_then(serde_json::Value::as_str)
                        .ok_or_else(|| serde::de::Error::custom("missing customType"))?
                        .to_string(),
                    content: serde_json::from_value(
                        object
                            .get("content")
                            .cloned()
                            .ok_or_else(|| serde::de::Error::custom("missing content"))?,
                    )
                    .map_err(serde::de::Error::custom)?,
                    display: object
                        .get("display")
                        .and_then(serde_json::Value::as_bool)
                        .ok_or_else(|| serde::de::Error::custom("missing display"))?,
                    details: object.get("details").cloned(),
                },
            }),
            "label" => Ok(SessionTreeEntry::Label {
                entry: LabelEntry {
                    base,
                    target_id: object
                        .get("targetId")
                        .and_then(serde_json::Value::as_str)
                        .ok_or_else(|| serde::de::Error::custom("missing targetId"))?
                        .to_string(),
                    label: object
                        .get("label")
                        .and_then(|value| value.as_str().map(ToOwned::to_owned)),
                },
            }),
            "session_info" => Ok(SessionTreeEntry::SessionInfo {
                entry: SessionInfoEntry {
                    base,
                    name: object
                        .get("name")
                        .and_then(|value| value.as_str().map(ToOwned::to_owned)),
                },
            }),
            "leaf" => Ok(SessionTreeEntry::Leaf {
                entry: LeafEntry {
                    base,
                    target_id: object
                        .get("targetId")
                        .and_then(|value| value.as_str().map(ToOwned::to_owned)),
                },
            }),
            _ => Err(serde::de::Error::custom("unknown session entry type")),
        }
    }
}

fn agent_message_to_value(message: &AgentMessage) -> serde_json::Value {
    let mut value = match message {
        AgentMessage::User(message) => {
            serde_json::to_value(message).unwrap_or(serde_json::Value::Null)
        }
        AgentMessage::Assistant(message) => {
            serde_json::to_value(message).unwrap_or(serde_json::Value::Null)
        }
        AgentMessage::ToolResult(message) => {
            serde_json::to_value(message).unwrap_or(serde_json::Value::Null)
        }
        AgentMessage::BashExecution(message) => {
            serde_json::to_value(message).unwrap_or(serde_json::Value::Null)
        }
        AgentMessage::Custom(message) => {
            serde_json::to_value(message).unwrap_or(serde_json::Value::Null)
        }
        AgentMessage::BranchSummary(message) => {
            serde_json::to_value(message).unwrap_or(serde_json::Value::Null)
        }
        AgentMessage::CompactionSummary(message) => {
            serde_json::to_value(message).unwrap_or(serde_json::Value::Null)
        }
    };

    if let serde_json::Value::Object(ref mut object) = value {
        object.insert(
            "role".to_string(),
            serde_json::Value::String(
                match message {
                    AgentMessage::User(_) => "user",
                    AgentMessage::Assistant(_) => "assistant",
                    AgentMessage::ToolResult(_) => "toolResult",
                    AgentMessage::BashExecution(_) => "bashExecution",
                    AgentMessage::Custom(_) => "custom",
                    AgentMessage::BranchSummary(_) => "branchSummary",
                    AgentMessage::CompactionSummary(_) => "compactionSummary",
                }
                .to_string(),
            ),
        );
    }

    value
}

fn agent_message_from_value(value: serde_json::Value) -> Result<AgentMessage, String> {
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
        _ => Err("unknown role".to_string()),
    }
}

impl SessionTreeEntry {
    pub fn id(&self) -> &str {
        match self {
            SessionTreeEntry::Message { entry } => &entry.base.id,
            SessionTreeEntry::ThinkingLevelChange { entry } => &entry.base.id,
            SessionTreeEntry::ModelChange { entry } => &entry.base.id,
            SessionTreeEntry::ActiveToolsChange { entry } => &entry.base.id,
            SessionTreeEntry::Compaction { entry } => &entry.base.id,
            SessionTreeEntry::BranchSummary { entry } => &entry.base.id,
            SessionTreeEntry::Custom { entry } => &entry.base.id,
            SessionTreeEntry::CustomMessage { entry } => &entry.base.id,
            SessionTreeEntry::Label { entry } => &entry.base.id,
            SessionTreeEntry::SessionInfo { entry } => &entry.base.id,
            SessionTreeEntry::Leaf { entry } => &entry.base.id,
        }
    }

    pub fn parent_id(&self) -> Option<&str> {
        match self {
            SessionTreeEntry::Message { entry } => entry.base.parent_id.as_deref(),
            SessionTreeEntry::ThinkingLevelChange { entry } => entry.base.parent_id.as_deref(),
            SessionTreeEntry::ModelChange { entry } => entry.base.parent_id.as_deref(),
            SessionTreeEntry::ActiveToolsChange { entry } => entry.base.parent_id.as_deref(),
            SessionTreeEntry::Compaction { entry } => entry.base.parent_id.as_deref(),
            SessionTreeEntry::BranchSummary { entry } => entry.base.parent_id.as_deref(),
            SessionTreeEntry::Custom { entry } => entry.base.parent_id.as_deref(),
            SessionTreeEntry::CustomMessage { entry } => entry.base.parent_id.as_deref(),
            SessionTreeEntry::Label { entry } => entry.base.parent_id.as_deref(),
            SessionTreeEntry::SessionInfo { entry } => entry.base.parent_id.as_deref(),
            SessionTreeEntry::Leaf { entry } => entry.base.parent_id.as_deref(),
        }
    }

    pub fn timestamp(&self) -> &str {
        match self {
            SessionTreeEntry::Message { entry } => &entry.base.timestamp,
            SessionTreeEntry::ThinkingLevelChange { entry } => &entry.base.timestamp,
            SessionTreeEntry::ModelChange { entry } => &entry.base.timestamp,
            SessionTreeEntry::ActiveToolsChange { entry } => &entry.base.timestamp,
            SessionTreeEntry::Compaction { entry } => &entry.base.timestamp,
            SessionTreeEntry::BranchSummary { entry } => &entry.base.timestamp,
            SessionTreeEntry::Custom { entry } => &entry.base.timestamp,
            SessionTreeEntry::CustomMessage { entry } => &entry.base.timestamp,
            SessionTreeEntry::Label { entry } => &entry.base.timestamp,
            SessionTreeEntry::SessionInfo { entry } => &entry.base.timestamp,
            SessionTreeEntry::Leaf { entry } => &entry.base.timestamp,
        }
    }

    pub fn type_name(&self) -> &'static str {
        match self {
            SessionTreeEntry::Message { .. } => "message",
            SessionTreeEntry::ThinkingLevelChange { .. } => "thinking_level_change",
            SessionTreeEntry::ModelChange { .. } => "model_change",
            SessionTreeEntry::ActiveToolsChange { .. } => "active_tools_change",
            SessionTreeEntry::Compaction { .. } => "compaction",
            SessionTreeEntry::BranchSummary { .. } => "branch_summary",
            SessionTreeEntry::Custom { .. } => "custom",
            SessionTreeEntry::CustomMessage { .. } => "custom_message",
            SessionTreeEntry::Label { .. } => "label",
            SessionTreeEntry::SessionInfo { .. } => "session_info",
            SessionTreeEntry::Leaf { .. } => "leaf",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionModelRef {
    pub provider: String,
    pub model_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionContext {
    pub messages: Vec<AgentMessage>,
    pub thinking_level: String,
    pub model: Option<SessionModelRef>,
    pub active_tool_names: Option<Vec<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionMetadata {
    pub id: String,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JsonlSessionMetadata {
    pub id: String,
    pub created_at: String,
    pub cwd: String,
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_session_path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionEntryType {
    Message,
    ThinkingLevelChange,
    ModelChange,
    ActiveToolsChange,
    Compaction,
    BranchSummary,
    Custom,
    CustomMessage,
    Label,
    SessionInfo,
    Leaf,
}

#[async_trait::async_trait]
pub trait SessionStorage<TMetadata>: Send + Sync {
    async fn get_metadata(&self) -> Result<TMetadata, SessionError>;
    async fn get_leaf_id(&self) -> Result<Option<String>, SessionError>;
    async fn set_leaf_id(&self, leaf_id: Option<String>) -> Result<(), SessionError>;
    async fn create_entry_id(&self) -> Result<String, SessionError>;
    async fn append_entry(&self, entry: SessionTreeEntry) -> Result<(), SessionError>;
    async fn get_entry(&self, id: &str) -> Result<Option<SessionTreeEntry>, SessionError>;
    async fn find_entries(
        &self,
        entry_type: SessionEntryType,
    ) -> Result<Vec<SessionTreeEntry>, SessionError>;
    async fn get_label(&self, id: &str) -> Result<Option<String>, SessionError>;
    async fn get_path_to_root(
        &self,
        leaf_id: Option<String>,
    ) -> Result<Vec<SessionTreeEntry>, SessionError>;
    async fn get_entries(&self) -> Result<Vec<SessionTreeEntry>, SessionError>;
}

/// Filesystem capability used by the harness.
#[async_trait::async_trait]
pub trait FileSystem: Send + Sync {
    fn cwd(&self) -> &str;

    async fn absolute_path(
        &self,
        path: &str,
        abort_signal: Option<tokio::sync::watch::Receiver<bool>>,
    ) -> std::result::Result<String, FileError>;

    async fn join_path(
        &self,
        parts: &[String],
        abort_signal: Option<tokio::sync::watch::Receiver<bool>>,
    ) -> std::result::Result<String, FileError>;

    async fn read_text_file(
        &self,
        path: &str,
        abort_signal: Option<tokio::sync::watch::Receiver<bool>>,
    ) -> std::result::Result<String, FileError>;

    async fn read_text_lines(
        &self,
        path: &str,
        max_lines: Option<usize>,
        abort_signal: Option<tokio::sync::watch::Receiver<bool>>,
    ) -> std::result::Result<Vec<String>, FileError>;

    async fn read_binary_file(
        &self,
        path: &str,
        abort_signal: Option<tokio::sync::watch::Receiver<bool>>,
    ) -> std::result::Result<Vec<u8>, FileError>;

    async fn write_file(
        &self,
        path: &str,
        content: &[u8],
        abort_signal: Option<tokio::sync::watch::Receiver<bool>>,
    ) -> std::result::Result<(), FileError>;

    async fn append_file(
        &self,
        path: &str,
        content: &[u8],
        abort_signal: Option<tokio::sync::watch::Receiver<bool>>,
    ) -> std::result::Result<(), FileError>;

    async fn file_info(
        &self,
        path: &str,
        abort_signal: Option<tokio::sync::watch::Receiver<bool>>,
    ) -> std::result::Result<FileInfo, FileError>;

    async fn list_dir(
        &self,
        path: &str,
        abort_signal: Option<tokio::sync::watch::Receiver<bool>>,
    ) -> std::result::Result<Vec<FileInfo>, FileError>;

    async fn canonical_path(
        &self,
        path: &str,
        abort_signal: Option<tokio::sync::watch::Receiver<bool>>,
    ) -> std::result::Result<String, FileError>;

    async fn exists(
        &self,
        path: &str,
        abort_signal: Option<tokio::sync::watch::Receiver<bool>>,
    ) -> std::result::Result<bool, FileError>;

    async fn create_dir(
        &self,
        path: &str,
        recursive: bool,
        abort_signal: Option<tokio::sync::watch::Receiver<bool>>,
    ) -> std::result::Result<(), FileError>;

    async fn remove(
        &self,
        path: &str,
        recursive: bool,
        force: bool,
        abort_signal: Option<tokio::sync::watch::Receiver<bool>>,
    ) -> std::result::Result<(), FileError>;

    async fn create_temp_dir(
        &self,
        prefix: &str,
        abort_signal: Option<tokio::sync::watch::Receiver<bool>>,
    ) -> std::result::Result<String, FileError>;

    async fn create_temp_file(
        &self,
        prefix: &str,
        suffix: &str,
        abort_signal: Option<tokio::sync::watch::Receiver<bool>>,
    ) -> std::result::Result<String, FileError>;

    async fn cleanup(&self);
}

/// Shell execution capability used by the harness.
#[async_trait::async_trait]
pub trait Shell: Send + Sync {
    async fn exec(
        &self,
        command: &str,
        options: Option<ExecutionEnvExecOptions>,
    ) -> std::result::Result<ExecResult, ExecutionError>;

    async fn cleanup(&self);
}

/// Filesystem and shell execution environment used by the harness.
pub trait ExecutionEnv: FileSystem + Shell {}

impl<T: FileSystem + Shell> ExecutionEnv for T {}
