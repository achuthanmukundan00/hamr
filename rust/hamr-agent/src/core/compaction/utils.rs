//! Port of `packages/coding-agent/src/core/compaction/utils.ts`.
//!
//! Shared utilities for compaction and branch summarization.

use hamr_ai::types::*;
use hamr_harness::types::AgentMessage;
use std::collections::HashSet;

// ============================================================================
// File Operation Tracking
// ============================================================================

/// Tracks which files were read, written, or edited across tool calls.
#[derive(Debug, Clone, Default)]
pub struct FileOperations {
    pub read: HashSet<String>,
    pub written: HashSet<String>,
    pub edited: HashSet<String>,
}

pub fn create_file_ops() -> FileOperations {
    FileOperations {
        read: HashSet::new(),
        written: HashSet::new(),
        edited: HashSet::new(),
    }
}

/// Extract file operations from tool calls in an assistant message.
pub fn extract_file_ops_from_message(message: &AgentMessage, file_ops: &mut FileOperations) {
    let assistant = match message {
        AgentMessage::Assistant(msg) => msg,
        _ => return,
    };

    for block in &assistant.content {
        let tool_call = match block {
            AssistantContentBlock::ToolCall(tc) => tc,
            _ => continue,
        };

        let path = tool_call.arguments.get("path").and_then(|v| v.as_str());
        let Some(path) = path else { continue };

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

/// Compute final file lists from file operations.
/// Returns read_files (files only read, not modified) and modified_files.
pub fn compute_file_lists(file_ops: &FileOperations) -> FileLists {
    let mut modified: HashSet<&str> = file_ops.edited.iter().map(|s| s.as_str()).collect();
    for f in &file_ops.written {
        modified.insert(f.as_str());
    }

    let mut read_only: Vec<String> = file_ops
        .read
        .iter()
        .filter(|f| !modified.contains(f.as_str()))
        .cloned()
        .collect();

    let mut modified_files: Vec<String> = modified.iter().map(|s| s.to_string()).collect();

    read_only.sort();
    modified_files.sort();

    FileLists {
        read_files: read_only,
        modified_files,
    }
}

pub struct FileLists {
    pub read_files: Vec<String>,
    pub modified_files: Vec<String>,
}

/// Format file operations as XML tags for summary.
pub fn format_file_operations(read_files: &[String], modified_files: &[String]) -> String {
    let mut sections: Vec<String> = Vec::new();
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
        return String::new();
    }
    format!("\n\n{}", sections.join("\n\n"))
}

// ============================================================================
// Message Serialization
// ============================================================================

/// Maximum characters for a tool result in serialized summaries.
const TOOL_RESULT_MAX_CHARS: usize = 2000;

/// Truncate text to a maximum character length for summarization.
/// Keeps the beginning and appends a truncation marker.
fn truncate_for_summary(text: &str, max_chars: usize) -> String {
    if text.len() <= max_chars {
        return text.to_string();
    }
    let truncated_chars = text.len() - max_chars;
    format!(
        "{}\n\n[... {} more characters truncated]",
        &text[..max_chars],
        truncated_chars
    )
}

/// Serialize LLM messages to text for summarization.
/// This prevents the model from treating it as a conversation to continue.
/// Call convert_to_llm() first to handle custom message types.
///
/// Tool results are truncated to keep the summarization request within reasonable token budgets.
pub fn serialize_conversation(messages: &[Message]) -> String {
    let mut parts: Vec<String> = Vec::new();

    for msg in messages {
        match msg {
            Message::User(user_msg) => {
                let content = collect_text_content(&user_msg.content);
                if !content.is_empty() {
                    parts.push(format!("[User]: {}", content));
                }
            }
            Message::Assistant(assistant_msg) => {
                let mut text_parts: Vec<&str> = Vec::new();
                let mut thinking_parts: Vec<&str> = Vec::new();
                let mut tool_calls: Vec<String> = Vec::new();

                for block in &assistant_msg.content {
                    match block {
                        AssistantContentBlock::Text(tc) => {
                            text_parts.push(&tc.text);
                        }
                        AssistantContentBlock::Thinking(tc) => {
                            thinking_parts.push(&tc.thinking);
                        }
                        AssistantContentBlock::ToolCall(tc) => {
                            let args_str = tc
                                .arguments
                                .as_object()
                                .map(|obj| {
                                    obj.iter()
                                        .map(|(k, v)| {
                                            format!(
                                                "{}={}",
                                                k,
                                                serde_json::to_string(v).unwrap_or_default()
                                            )
                                        })
                                        .collect::<Vec<_>>()
                                        .join(", ")
                                })
                                .unwrap_or_default();
                            tool_calls.push(format!("{}({})", tc.name, args_str));
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
            Message::ToolResult(tool_result_msg) => {
                let content = collect_text_content(&tool_result_msg.content);
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

/// Collect text from MessageContent blocks into a single string.
fn collect_text_content(content: &[MessageContent]) -> String {
    content
        .iter()
        .filter_map(|c| match c {
            MessageContent::Text(tc) => Some(tc.text.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("")
}

// ============================================================================
// Summarization System Prompt
// ============================================================================

pub const SUMMARIZATION_SYSTEM_PROMPT: &str = "You are a context summarization assistant. Your task is to read a conversation between a user and an AI assistant, then produce a structured summary following the exact format specified.\n\nDo NOT continue the conversation. Do NOT respond to any questions in the conversation. ONLY output the structured summary.";

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn make_tool_call(name: &str, path: &str) -> AssistantContentBlock {
        AssistantContentBlock::ToolCall(ToolCall {
            id: "tc1".to_string(),
            name: name.to_string(),
            arguments: serde_json::json!({"path": path}),
            thought_signature: None,
        })
    }

    fn make_assistant(tool_calls: Vec<AssistantContentBlock>) -> AgentMessage {
        AgentMessage::Assistant(AssistantMessage {
            role: MessageRole::Assistant,
            content: tool_calls,
            api: "anthropic-messages".to_string(),
            provider: "anthropic".to_string(),
            model: "claude".to_string(),
            response_model: None,
            response_id: None,
            usage: Usage {
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
            },
            stop_reason: StopReason::Stop,
            error_message: None,
            diagnostics: None,
            timestamp: Utc::now(),
        })
    }

    #[test]
    fn extracts_file_ops_from_assistant() {
        let mut ops = create_file_ops();
        let msg = make_assistant(vec![
            make_tool_call("read", "/foo/bar.txt"),
            make_tool_call("write", "/foo/baz.txt"),
            make_tool_call("edit", "/foo/bar.txt"),
        ]);
        extract_file_ops_from_message(&msg, &mut ops);

        assert!(ops.read.contains("/foo/bar.txt"));
        assert!(ops.written.contains("/foo/baz.txt"));
        assert!(ops.edited.contains("/foo/bar.txt"));
    }

    #[test]
    fn ignores_non_assistant_messages() {
        let mut ops = create_file_ops();
        let msg = AgentMessage::User(UserMessage {
            role: MessageRole::User,
            content: vec![MessageContent::Text(TextContent {
                text: "hello".to_string(),
                text_signature: None,
            })],
            timestamp: Utc::now(),
        });
        extract_file_ops_from_message(&msg, &mut ops);
        assert!(ops.read.is_empty());
        assert!(ops.written.is_empty());
        assert!(ops.edited.is_empty());
    }

    #[test]
    fn compute_file_lists_dedup() {
        let mut ops = create_file_ops();
        ops.read.insert("a.txt".to_string());
        ops.read.insert("b.txt".to_string());
        ops.edited.insert("b.txt".to_string());
        ops.written.insert("c.txt".to_string());

        let result = compute_file_lists(&ops);
        assert_eq!(result.read_files, vec!["a.txt"]);
        assert_eq!(result.modified_files, vec!["b.txt", "c.txt"]);
    }

    #[test]
    fn format_file_operations_xml() {
        let output = format_file_operations(&["a.txt".to_string()], &["b.txt".to_string()]);
        assert!(output.contains("<read-files>"));
        assert!(output.contains("a.txt"));
        assert!(output.contains("<modified-files>"));
        assert!(output.contains("b.txt"));
    }

    #[test]
    fn format_file_operations_empty() {
        let output = format_file_operations(&[], &[]);
        assert!(output.is_empty());
    }

    #[test]
    fn serialize_conversation_produces_text() {
        let messages = vec![Message::User(UserMessage {
            role: MessageRole::User,
            content: vec![MessageContent::Text(TextContent {
                text: "hello".to_string(),
                text_signature: None,
            })],
            timestamp: Utc::now(),
        })];
        let output = serialize_conversation(&messages);
        assert_eq!(output, "[User]: hello");
    }

    #[test]
    fn truncate_for_summary_keeps_short() {
        let text = "short";
        let result = truncate_for_summary(text, 100);
        assert_eq!(result, "short");
    }

    #[test]
    fn truncate_for_summary_truncates_long() {
        let text = "a".repeat(2500);
        let result = truncate_for_summary(&text, 2000);
        assert!(result.starts_with(&"a".repeat(2000)));
        assert!(result.contains("more characters truncated"));
    }

    #[test]
    fn serialize_conversation_handles_assistant_with_thinking_and_tool_calls() {
        use chrono::Utc;
        use hamr_ai::types::*;

        let tool_call_block = AssistantContentBlock::ToolCall(ToolCall {
            id: "tc1".to_string(),
            name: "read".to_string(),
            arguments: serde_json::json!({"path": "/foo/bar.txt"}),
            thought_signature: None,
        });

        let msg = Message::Assistant(AssistantMessage {
            role: AssistantRole::Assistant,
            content: vec![
                AssistantContentBlock::Text(TextContent {
                    text: "Let me read that file.".to_string(),
                    text_signature: None,
                }),
                AssistantContentBlock::Thinking(ThinkingContent {
                    thinking: "I need to check the file content.".to_string(),
                    thinking_signature: None,
                    redacted: false,
                }),
                tool_call_block,
            ],
            api: "anthropic-messages".to_string(),
            provider: "anthropic".to_string(),
            model: "claude".to_string(),
            response_model: None,
            response_id: None,
            usage: Usage {
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
            },
            stop_reason: StopReason::Stop,
            error_message: None,
            diagnostics: None,
            timestamp: Utc::now(),
        });

        let output = serialize_conversation(&[msg]);
        assert!(output.contains("[Assistant thinking]:"));
        assert!(output.contains("I need to check the file content."));
        assert!(output.contains("[Assistant]:"));
        assert!(output.contains("Let me read that file."));
        assert!(output.contains("[Assistant tool calls]:"));
        assert!(output.contains("read("));
    }

    #[test]
    fn serialize_conversation_truncates_tool_results() {
        use chrono::Utc;
        use hamr_ai::types::*;

        let long_text = "x".repeat(3000);
        let msg = Message::ToolResult(ToolResultMessage {
            role: ToolResultRole::ToolResult,
            content: vec![MessageContent::Text(TextContent {
                text: long_text.clone(),
                text_signature: None,
            })],
            tool_name: "read".to_string(),
            tool_call_id: "tc1".to_string(),
            details: None,
            is_error: false,
            timestamp: Utc::now(),
        });

        let output = serialize_conversation(&[msg]);
        assert!(output.contains("[Tool result]:"));
        assert!(output.contains("more characters truncated"));
    }
}
