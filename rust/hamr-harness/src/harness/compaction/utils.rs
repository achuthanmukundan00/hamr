//! Port of `packages/agent/src/harness/compaction/utils.ts`.

use crate::types::AgentMessage;
use hamr_ai::types::{AssistantContentBlock, Message, MessageContent};
use std::collections::HashSet;

const TOOL_RESULT_MAX_CHARS: usize = 2000;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FileOperations {
    pub read: HashSet<String>,
    pub written: HashSet<String>,
    pub edited: HashSet<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileLists {
    pub read_files: Vec<String>,
    pub modified_files: Vec<String>,
}

pub fn create_file_ops() -> FileOperations {
    FileOperations::default()
}

pub fn extract_file_ops_from_message(message: &AgentMessage, file_ops: &mut FileOperations) {
    let AgentMessage::Assistant(message) = message else {
        return;
    };

    for block in &message.content {
        let AssistantContentBlock::ToolCall(tool_call) = block else {
            continue;
        };
        let Some(path) = tool_call
            .arguments
            .get("path")
            .and_then(serde_json::Value::as_str)
        else {
            continue;
        };

        match tool_call.name.as_str() {
            "read" => {
                file_ops.read.insert(path.to_string());
            }
            "write" => {
                file_ops.written.insert(path.to_string());
            }
            "edit" => {
                file_ops.edited.insert(path.to_string());
            }
            _ => {}
        }
    }
}

pub fn compute_file_lists(file_ops: &FileOperations) -> FileLists {
    let modified: HashSet<String> = file_ops
        .edited
        .iter()
        .chain(file_ops.written.iter())
        .cloned()
        .collect();

    let mut read_files = file_ops
        .read
        .iter()
        .filter(|file| !modified.contains(*file))
        .cloned()
        .collect::<Vec<_>>();
    read_files.sort();

    let mut modified_files = modified.into_iter().collect::<Vec<_>>();
    modified_files.sort();

    FileLists {
        read_files,
        modified_files,
    }
}

pub fn format_file_operations(read_files: &[String], modified_files: &[String]) -> String {
    let mut sections = Vec::new();
    if !read_files.is_empty() {
        sections.push(format!(
            "<read-files>\n{}\n</read-files>",
            read_files.join("\n")
        ));
    }
    if !modified_files.is_empty() {
        sections.push(format!(
            "<modified-files>\n{}\n</modified-files>",
            modified_files.join("\n")
        ));
    }
    if sections.is_empty() {
        String::new()
    } else {
        format!("\n\n{}", sections.join("\n\n"))
    }
}

fn safe_json_stringify(value: &serde_json::Value) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| "[unserializable]".to_string())
}

fn truncate_for_summary(text: &str, max_chars: usize) -> String {
    let char_count = text.chars().count();
    if char_count <= max_chars {
        return text.to_string();
    }
    let truncated_chars = char_count - max_chars;
    format!(
        "{}\n\n[... {truncated_chars} more characters truncated]",
        text.chars().take(max_chars).collect::<String>()
    )
}

pub fn serialize_conversation(messages: &[Message]) -> String {
    let mut parts = Vec::new();

    for message in messages {
        match message {
            Message::User(message) => {
                let content = message
                    .content
                    .iter()
                    .filter_map(|content| match content {
                        MessageContent::Text(text) => Some(text.text.as_str()),
                        MessageContent::Image(_) => None,
                    })
                    .collect::<String>();
                if !content.is_empty() {
                    parts.push(format!("[User]: {content}"));
                }
            }
            Message::Assistant(message) => {
                let mut text_parts = Vec::new();
                let mut thinking_parts = Vec::new();
                let mut tool_calls = Vec::new();

                for block in &message.content {
                    match block {
                        AssistantContentBlock::Text(text) => text_parts.push(text.text.clone()),
                        AssistantContentBlock::Thinking(thinking) => {
                            thinking_parts.push(thinking.thinking.clone())
                        }
                        AssistantContentBlock::ToolCall(tool_call) => {
                            let args = tool_call
                                .arguments
                                .as_object()
                                .map(|arguments| {
                                    arguments
                                        .iter()
                                        .map(|(key, value)| {
                                            format!("{key}={}", safe_json_stringify(value))
                                        })
                                        .collect::<Vec<_>>()
                                        .join(", ")
                                })
                                .unwrap_or_default();
                            tool_calls.push(format!("{}({args})", tool_call.name));
                        }
                    }
                }

                if !thinking_parts.is_empty() {
                    parts.push(format!(
                        "[Assistant thinking]: {}",
                        thinking_parts.join("\n")
                    ));
                }
                if !text_parts.is_empty() {
                    parts.push(format!("[Assistant]: {}", text_parts.join("\n")));
                }
                if !tool_calls.is_empty() {
                    parts.push(format!("[Assistant tool calls]: {}", tool_calls.join("; ")));
                }
            }
            Message::ToolResult(message) => {
                let content = message
                    .content
                    .iter()
                    .filter_map(|content| match content {
                        MessageContent::Text(text) => Some(text.text.as_str()),
                        MessageContent::Image(_) => None,
                    })
                    .collect::<String>();
                if !content.is_empty() {
                    parts.push(format!(
                        "[Tool result]: {}",
                        truncate_for_summary(&content, TOOL_RESULT_MAX_CHARS)
                    ));
                }
            }
        }
    }

    parts.join("\n\n")
}

#[cfg(test)]
mod tests {
    use super::{
        compute_file_lists, create_file_ops, extract_file_ops_from_message, format_file_operations,
        serialize_conversation,
    };
    use crate::types::AgentMessage;
    use chrono::Utc;
    use hamr_ai::types::{
        AssistantContentBlock, AssistantMessage, Message, MessageContent, MessageRole, StopReason,
        TextContent, ToolCall, ToolResultMessage, Usage, UsageCost,
    };

    fn empty_usage() -> Usage {
        Usage {
            input: 0,
            output: 0,
            cache_read: 0,
            cache_write: 0,
            cache_write_1h: None,
            total_tokens: 0,
            cost: UsageCost {
                input: 0.0,
                output: 0.0,
                cache_read: 0.0,
                cache_write: 0.0,
                total: 0.0,
            },
        }
    }

    #[test]
    fn extracts_and_formats_file_ops() {
        let message = AgentMessage::Assistant(AssistantMessage {
            role: MessageRole::Assistant,
            content: vec![
                AssistantContentBlock::ToolCall(ToolCall {
                    id: "1".to_string(),
                    name: "read".to_string(),
                    arguments: serde_json::json!({ "path": "/tmp/a.txt" }),
                    thought_signature: None,
                }),
                AssistantContentBlock::ToolCall(ToolCall {
                    id: "2".to_string(),
                    name: "edit".to_string(),
                    arguments: serde_json::json!({ "path": "/tmp/b.txt" }),
                    thought_signature: None,
                }),
            ],
            api: "api".to_string(),
            provider: "provider".to_string(),
            model: "model".to_string(),
            response_model: None,
            response_id: None,
            usage: empty_usage(),
            stop_reason: StopReason::Stop,
            error_message: None,
            diagnostics: None,
            timestamp: Utc::now(),
        });

        let mut file_ops = create_file_ops();
        extract_file_ops_from_message(&message, &mut file_ops);
        let lists = compute_file_lists(&file_ops);
        assert_eq!(lists.read_files, vec!["/tmp/a.txt".to_string()]);
        assert_eq!(lists.modified_files, vec!["/tmp/b.txt".to_string()]);

        let formatted = format_file_operations(&lists.read_files, &lists.modified_files);
        assert!(formatted.contains("<read-files>"));
        assert!(formatted.contains("<modified-files>"));
    }

    #[test]
    fn serializes_conversation() {
        let output = serialize_conversation(&[
            Message::User(hamr_ai::types::UserMessage {
                role: MessageRole::User,
                content: vec![MessageContent::Text(TextContent {
                    text: "hi".to_string(),
                    text_signature: None,
                })],
                timestamp: Utc::now(),
            }),
            Message::Assistant(AssistantMessage {
                role: MessageRole::Assistant,
                content: vec![
                    AssistantContentBlock::Text(TextContent {
                        text: "hello".to_string(),
                        text_signature: None,
                    }),
                    AssistantContentBlock::ToolCall(ToolCall {
                        id: "1".to_string(),
                        name: "read".to_string(),
                        arguments: serde_json::json!({ "path": "a.txt" }),
                        thought_signature: None,
                    }),
                ],
                api: "api".to_string(),
                provider: "provider".to_string(),
                model: "model".to_string(),
                response_model: None,
                response_id: None,
                usage: empty_usage(),
                stop_reason: StopReason::Stop,
                error_message: None,
                diagnostics: None,
                timestamp: Utc::now(),
            }),
            Message::ToolResult(ToolResultMessage {
                role: MessageRole::ToolResult,
                tool_call_id: "1".to_string(),
                tool_name: "read".to_string(),
                content: vec![MessageContent::Text(TextContent {
                    text: "result".to_string(),
                    text_signature: None,
                })],
                details: None,
                is_error: false,
                timestamp: Utc::now(),
            }),
        ]);

        assert!(output.contains("[User]: hi"));
        assert!(output.contains("[Assistant]: hello"));
        assert!(output.contains("[Assistant tool calls]: read(path=\"a.txt\")"));
        assert!(output.contains("[Tool result]: result"));
    }
}
