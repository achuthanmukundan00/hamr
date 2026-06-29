//! Built-in tools — bash, read, edit, write, grep, find, ls.
//!
//! Mirror of `packages/coding-agent/src/core/tools/`.

pub mod bash;
pub mod edit;
pub mod edit_diff;
pub mod file_mutation_queue;
pub mod find;
pub mod grep;
pub mod ls;
pub mod output_accumulator;
pub mod path_guard;
pub mod path_utils;
pub mod read;
pub mod render_utils;
pub mod tool_definition_wrapper;
pub mod truncate;
pub mod write;

use std::path::Path;
use std::sync::Arc;

use hamr_ai::types::{MessageContent, TextContent};
use hamr_harness::types::{AgentToolResult, ToolExecutionMode};

use crate::core::extensions::types::ToolDefinition;

/// Create a ToolDefinition for the read tool.
/// Mirrors TS `createReadToolDefinition(cwd, options)`.
pub fn create_read_tool_definition(cwd: &Path) -> ToolDefinition {
    let cwd = cwd.to_path_buf();
    ToolDefinition {
        name: "read".to_string(),
        label: "read".to_string(),
        description: "Read the contents of a file. Supports text files and images (jpg, png, gif, webp). Images are sent as attachments. For text files, output is truncated to 2000 lines or 50KB (whichever is hit first). Use offset/limit for large files. When you need the full file, continue with offset until complete.".to_string(),
        prompt_snippet: Some("Read file contents".to_string()),
        prompt_guidelines: Some(vec!["Use read to examine files instead of cat or sed.".to_string()]),
        parameters: serde_json::json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "Path to the file to read (relative or absolute)" },
                "offset": { "type": "number", "description": "Line number to start reading from (1-indexed)" },
                "limit": { "type": "number", "description": "Maximum number of lines to read" }
            },
            "required": ["path"]
        }),
        render_shell: None,
        prepare_arguments: None,
        execution_mode: None,
        execute: Arc::new(
            move |tool_call_id: String,
                  params: serde_json::Value,
                  signal: Option<tokio::sync::watch::Receiver<bool>>,
                  _on_update: Option<hamr_harness::types::AgentToolUpdateCallback>,
                  _ctx: Arc<dyn crate::core::extensions::types::ExtensionContext>| {
                let tool = read::create_read_tool(&cwd);
                Box::pin(async move {
                    let input: read::ReadToolInput = match serde_json::from_value(params) {
                        Ok(v) => v,
                        Err(e) => return AgentToolResult {
                            content: vec![MessageContent::Text(TextContent {
                                text: format!("Invalid arguments: {e}"),
                                text_signature: None,
                            })],
                            details: None,
                            is_error: true,
                            terminate: false,
                        },
                    };
                    let _ = (tool_call_id, signal);
                    match tool.execute(&input).await {
                        Ok(result) => AgentToolResult {
                            content: result.content,
                            details: None,
                            is_error: false,
                            terminate: false,
                        },
                        Err(e) => AgentToolResult {
                            content: vec![MessageContent::Text(TextContent {
                                text: e.to_string(),
                                text_signature: None,
                            })],
                            details: None,
                            is_error: true,
                            terminate: false,
                        },
                    }
                })
            },
        ),
    }
}

/// Create a ToolDefinition for the bash tool.
/// Mirrors TS `createBashToolDefinition(cwd, options)`.
pub fn create_bash_tool_definition(cwd: &Path) -> ToolDefinition {
    let cwd = cwd.to_path_buf();
    ToolDefinition {
        name: "bash".to_string(),
        label: "bash".to_string(),
        description: "Execute a bash command in the current working directory. Returns stdout and stderr. Output is truncated to last 2000 lines or 50KB (whichever is hit first). If truncated, full output is saved to a temp file. Optionally provide a timeout in seconds.".to_string(),
        prompt_snippet: Some("Execute bash commands (ls, grep, find, etc.)".to_string()),
        prompt_guidelines: None,
        parameters: serde_json::json!({
            "type": "object",
            "properties": {
                "command": { "type": "string", "description": "Bash command to execute" },
                "timeout": { "type": "number", "description": "Optional timeout in seconds" }
            },
            "required": ["command"]
        }),
        render_shell: None,
        prepare_arguments: None,
        execution_mode: Some(ToolExecutionMode::Sequential),
        execute: Arc::new(
            move |_tool_call_id: String,
                  params: serde_json::Value,
                  signal: Option<tokio::sync::watch::Receiver<bool>>,
                  _on_update: Option<hamr_harness::types::AgentToolUpdateCallback>,
                  _ctx: Arc<dyn crate::core::extensions::types::ExtensionContext>| {
                let tool = bash::create_bash_tool(&cwd, None);
                Box::pin(async move {
                    let input: bash::BashToolInput = match serde_json::from_value(params) {
                        Ok(v) => v,
                        Err(e) => return AgentToolResult {
                            content: vec![MessageContent::Text(TextContent {
                                text: format!("Invalid arguments: {e}"),
                                text_signature: None,
                            })],
                            details: None,
                            is_error: true,
                            terminate: false,
                        },
                    };
                    match tool.execute(input, signal, None).await {
                        Ok(result) => AgentToolResult {
                            content: vec![MessageContent::Text(TextContent {
                                text: result.content,
                                text_signature: None,
                            })],
                            details: None,
                            is_error: false,
                            terminate: false,
                        },
                        Err(e) => AgentToolResult {
                            content: vec![MessageContent::Text(TextContent {
                                text: e.to_string(),
                                text_signature: None,
                            })],
                            details: None,
                            is_error: true,
                            terminate: false,
                        },
                    }
                })
            },
        ),
    }
}

/// Create a ToolDefinition for the edit tool.
/// Mirrors TS `createEditToolDefinition(cwd, options)`.
pub fn create_edit_tool_definition(cwd: &Path) -> ToolDefinition {
    let cwd = cwd.to_path_buf();
    ToolDefinition {
        name: "edit".to_string(),
        label: "edit".to_string(),
        description: "Edit a single file using exact text replacement. Every edits[].oldText must match a unique, non-overlapping region of the original file. If two changes affect the same block or nearby lines, merge them into one edit instead of emitting overlapping edits. Do not include large unchanged regions just to connect distant changes.".to_string(),
        prompt_snippet: Some("Make precise file edits with exact text replacement, including multiple disjoint edits in one call".to_string()),
        prompt_guidelines: Some(vec![
            "Use edit for precise changes (edits[].oldText must match exactly)".to_string(),
            "When changing multiple separate locations in one file, use one edit call with multiple entries in edits[] instead of multiple edit calls".to_string(),
            "Each edits[].oldText is matched against the original file, not after earlier edits are applied. Do not emit overlapping or nested edits. Merge nearby changes into one edit.".to_string(),
            "Keep edits[].oldText as small as possible while still being unique in the file. Do not pad with large unchanged regions.".to_string(),
        ]),
        parameters: serde_json::json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "Path to the file to edit (relative or absolute)" },
                "edits": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "properties": {
                            "oldText": { "type": "string", "description": "Exact text to find in the file" },
                            "newText": { "type": "string", "description": "Replacement text" }
                        },
                        "required": ["oldText", "newText"]
                    }
                },
                "dry_run": { "type": "boolean", "description": "If true, perform all validation but do not write to disk" }
            },
            "required": ["path", "edits"]
        }),
        render_shell: Some("self".to_string()),
        prepare_arguments: None,
        execution_mode: None,
        execute: Arc::new(
            move |_tool_call_id: String,
                  params: serde_json::Value,
                  _signal: Option<tokio::sync::watch::Receiver<bool>>,
                  _on_update: Option<hamr_harness::types::AgentToolUpdateCallback>,
                  _ctx: Arc<dyn crate::core::extensions::types::ExtensionContext>| {
                let tool = edit::EditTool::new(&cwd);
                Box::pin(async move {
                    let input: edit::EditToolInput = match serde_json::from_value(params) {
                        Ok(v) => v,
                        Err(e) => return AgentToolResult {
                            content: vec![MessageContent::Text(TextContent {
                                text: format!("Invalid arguments: {e}"),
                                text_signature: None,
                            })],
                            details: None,
                            is_error: true,
                            terminate: false,
                        },
                    };
                    match tool.execute(input, None) {
                        Ok(details) => AgentToolResult {
                            content: vec![MessageContent::Text(TextContent {
                                text: format!("Successfully edited {}", details.diff),
                                text_signature: None,
                            })],
                            details: Some(serde_json::to_value(&details).unwrap_or_default()),
                            is_error: false,
                            terminate: false,
                        },
                        Err(e) => AgentToolResult {
                            content: vec![MessageContent::Text(TextContent {
                                text: e.to_string(),
                                text_signature: None,
                            })],
                            details: None,
                            is_error: true,
                            terminate: false,
                        },
                    }
                })
            },
        ),
    }
}

/// Create a ToolDefinition for the write tool.
/// Mirrors TS `createWriteToolDefinition(cwd, options)`.
pub fn create_write_tool_definition(cwd: &Path) -> ToolDefinition {
    let cwd = cwd.to_path_buf();
    ToolDefinition {
        name: "write".to_string(),
        label: "write".to_string(),
        description: "Write content to a file. Creates the file if it doesn't exist, overwrites if it does. Automatically creates parent directories.".to_string(),
        prompt_snippet: Some("Create or overwrite files".to_string()),
        prompt_guidelines: Some(vec!["Use write only for new files or complete rewrites.".to_string()]),
        parameters: serde_json::json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "Path to the file to write (relative or absolute)" },
                "content": { "type": "string", "description": "Content to write to the file" }
            },
            "required": ["path", "content"]
        }),
        render_shell: None,
        prepare_arguments: None,
        execution_mode: None,
        execute: Arc::new(
            move |_tool_call_id: String,
                  params: serde_json::Value,
                  _signal: Option<tokio::sync::watch::Receiver<bool>>,
                  _on_update: Option<hamr_harness::types::AgentToolUpdateCallback>,
                  _ctx: Arc<dyn crate::core::extensions::types::ExtensionContext>| {
                let tool = write::create_write_tool(cwd.clone());
                Box::pin(async move {
                    let input: write::WriteToolInput = match serde_json::from_value(params) {
                        Ok(v) => v,
                        Err(e) => return AgentToolResult {
                            content: vec![MessageContent::Text(TextContent {
                                text: format!("Invalid arguments: {e}"),
                                text_signature: None,
                            })],
                            details: None,
                            is_error: true,
                            terminate: false,
                        },
                    };
                    match tool.execute(&input) {
                        Ok(result) => {
                            let msg = result.message.clone();
                            AgentToolResult {
                            content: vec![MessageContent::Text(TextContent {
                                text: msg,
                                text_signature: None,
                            })],
                            details: Some(serde_json::to_value(&result).unwrap_or_default()),
                            is_error: false,
                            terminate: false,
                        }
                        },
                        Err(e) => AgentToolResult {
                            content: vec![MessageContent::Text(TextContent {
                                text: e.to_string(),
                                text_signature: None,
                            })],
                            details: None,
                            is_error: true,
                            terminate: false,
                        },
                    }
                })
            },
        ),
    }
}

/// Create a ToolDefinition for the grep tool.
/// Mirrors TS `createGrepToolDefinition(cwd, options)`.
pub fn create_grep_tool_definition(cwd: &Path) -> ToolDefinition {
    let cwd = cwd.to_path_buf();
    ToolDefinition {
        name: "grep".to_string(),
        label: "grep".to_string(),
        description: "Search file contents using ripgrep. Returns matching lines with file paths and line numbers. Respects .gitignore. Pattern can be regex or literal string. Output is capped at a match limit (default 100) or a byte limit, whichever hits first.".to_string(),
        prompt_snippet: Some("Search file contents with rg".to_string()),
        prompt_guidelines: None,
        parameters: serde_json::json!({
            "type": "object",
            "properties": {
                "pattern": { "type": "string", "description": "Search pattern (regex or literal string)" },
                "path": { "type": "string", "description": "Directory or file to search (default: current directory)" },
                "glob": { "type": "string", "description": "Filter files by glob pattern, e.g. *.ts" },
                "ignore_case": { "type": "boolean", "description": "Case-insensitive search (default: false)" },
                "literal": { "type": "boolean", "description": "Treat pattern as a literal string instead of regex (default: false)" },
                "context": { "type": "number", "description": "Lines of context to show before and after each match (default: 0)" },
                "limit": { "type": "number", "description": "Maximum number of matches to return (default: 100)" }
            },
            "required": ["pattern"]
        }),
        render_shell: None,
        prepare_arguments: None,
        execution_mode: None,
        execute: Arc::new(
            move |_tool_call_id: String,
                  params: serde_json::Value,
                  signal: Option<tokio::sync::watch::Receiver<bool>>,
                  _on_update: Option<hamr_harness::types::AgentToolUpdateCallback>,
                  _ctx: Arc<dyn crate::core::extensions::types::ExtensionContext>| {
                let tool = grep::GrepTool::new(&cwd, grep::GrepToolOptions::default());
                Box::pin(async move {
                    let input: grep::GrepToolInput = match serde_json::from_value(params) {
                        Ok(v) => v,
                        Err(e) => return AgentToolResult {
                            content: vec![MessageContent::Text(TextContent {
                                text: format!("Invalid arguments: {e}"),
                                text_signature: None,
                            })],
                            details: None,
                            is_error: true,
                            terminate: false,
                        },
                    };
                    match tool.execute(&input, signal).await {
                        Ok(result) => AgentToolResult {
                            content: result.content,
                            details: None,
                            is_error: false,
                            terminate: false,
                        },
                        Err(e) => AgentToolResult {
                            content: vec![MessageContent::Text(TextContent {
                                text: e.to_string(),
                                text_signature: None,
                            })],
                            details: None,
                            is_error: true,
                            terminate: false,
                        },
                    }
                })
            },
        ),
    }
}

/// Create a ToolDefinition for the find tool.
/// Mirrors TS `createFindToolDefinition(cwd, options)`.
pub fn create_find_tool_definition(cwd: &Path) -> ToolDefinition {
    let cwd = cwd.to_path_buf();
    ToolDefinition {
        name: "find".to_string(),
        label: "find".to_string(),
        description: "Find files by glob pattern using fd. Returns relative file paths. Respects .gitignore. Use for locating files by name or extension. Output is capped at a result limit (default 1000) or a byte limit, whichever hits first.".to_string(),
        prompt_snippet: Some("Find files by glob pattern with fd".to_string()),
        prompt_guidelines: None,
        parameters: serde_json::json!({
            "type": "object",
            "properties": {
                "pattern": { "type": "string", "description": "Glob pattern to match files, e.g. *.ts, **/*.json" },
                "path": { "type": "string", "description": "Directory to search in (default: current directory)" },
                "limit": { "type": "number", "description": "Maximum number of results (default: 1000)" }
            },
            "required": ["pattern"]
        }),
        render_shell: None,
        prepare_arguments: None,
        execution_mode: None,
        execute: Arc::new(
            move |_tool_call_id: String,
                  params: serde_json::Value,
                  signal: Option<tokio::sync::watch::Receiver<bool>>,
                  _on_update: Option<hamr_harness::types::AgentToolUpdateCallback>,
                  _ctx: Arc<dyn crate::core::extensions::types::ExtensionContext>| {
                let tool = find::FindTool::new(&cwd, find::FindToolOptions::default());
                Box::pin(async move {
                    let input: find::FindToolInput = match serde_json::from_value(params) {
                        Ok(v) => v,
                        Err(e) => return AgentToolResult {
                            content: vec![MessageContent::Text(TextContent {
                                text: format!("Invalid arguments: {e}"),
                                text_signature: None,
                            })],
                            details: None,
                            is_error: true,
                            terminate: false,
                        },
                    };
                    match tool.execute(&input, signal).await {
                        Ok(result) => AgentToolResult {
                            content: result.content,
                            details: None,
                            is_error: false,
                            terminate: false,
                        },
                        Err(e) => AgentToolResult {
                            content: vec![MessageContent::Text(TextContent {
                                text: e.to_string(),
                                text_signature: None,
                            })],
                            details: None,
                            is_error: true,
                            terminate: false,
                        },
                    }
                })
            },
        ),
    }
}

/// Create a ToolDefinition for the ls tool.
/// Mirrors TS `createLsToolDefinition(cwd, options)`.
pub fn create_ls_tool_definition(cwd: &Path) -> ToolDefinition {
    let cwd = cwd.to_path_buf();
    ToolDefinition {
        name: "ls".to_string(),
        label: "ls".to_string(),
        description: "List directory contents sorted alphabetically (case-insensitive), with a `/` suffix for directories, dotfiles included. Output is capped at a configurable entry limit (default 500) or a byte limit, whichever hits first.".to_string(),
        prompt_snippet: Some("List directory contents".to_string()),
        prompt_guidelines: None,
        parameters: serde_json::json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "Directory to list (default: current directory)" },
                "limit": { "type": "number", "description": "Maximum number of entries to return (default: 500)" }
            },
            "required": []
        }),
        render_shell: None,
        prepare_arguments: None,
        execution_mode: None,
        execute: Arc::new(
            move |_tool_call_id: String,
                  params: serde_json::Value,
                  signal: Option<tokio::sync::watch::Receiver<bool>>,
                  _on_update: Option<hamr_harness::types::AgentToolUpdateCallback>,
                  _ctx: Arc<dyn crate::core::extensions::types::ExtensionContext>| {
                let tool = ls::LsTool::new(&cwd, ls::LsToolOptions::default());
                Box::pin(async move {
                    let input: ls::LsToolInput = match serde_json::from_value(params) {
                        Ok(v) => v,
                        Err(e) => return AgentToolResult {
                            content: vec![MessageContent::Text(TextContent {
                                text: format!("Invalid arguments: {e}"),
                                text_signature: None,
                            })],
                            details: None,
                            is_error: true,
                            terminate: false,
                        },
                    };
                    match tool.execute(&input, signal.as_ref()).await {
                        Ok(result) => AgentToolResult {
                            content: result.content,
                            details: None,
                            is_error: false,
                            terminate: false,
                        },
                        Err(e) => AgentToolResult {
                            content: vec![MessageContent::Text(TextContent {
                                text: e.to_string(),
                                text_signature: None,
                            })],
                            details: None,
                            is_error: true,
                            terminate: false,
                        },
                    }
                })
            },
        ),
    }
}

/// All built-in tool definitions keyed by tool name.
/// Mirrors TS `createAllToolDefinitions(cwd, options)`.
pub fn create_all_tool_definitions(cwd: &Path) -> Vec<ToolDefinition> {
    vec![
        create_read_tool_definition(cwd),
        create_bash_tool_definition(cwd),
        create_edit_tool_definition(cwd),
        create_write_tool_definition(cwd),
        create_grep_tool_definition(cwd),
        create_find_tool_definition(cwd),
        create_ls_tool_definition(cwd),
    ]
}

/// Default active tool names.
/// Mirrors TS `defaultActiveToolNames`.
/// grep, find, ls are available but off by default (read-only tools).
pub fn default_active_tool_names() -> Vec<String> {
    vec!["read".into(), "bash".into(), "edit".into(), "write".into()]
}

/// All known tool names (including read-only tools that are off by default).
pub fn all_tool_names() -> Vec<String> {
    vec![
        "read".into(),
        "bash".into(),
        "edit".into(),
        "write".into(),
        "grep".into(),
        "find".into(),
        "ls".into(),
    ]
}
