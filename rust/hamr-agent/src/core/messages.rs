//! Custom message types and transformers for the coding agent.
//!
//! Port of `packages/coding-agent/src/core/messages.ts`.
//!
//! Extends the base AgentMessage type with coding-agent specific message types,
//! and provides a transformer to convert them to LLM-compatible messages.

pub use hamr_harness::types::{
    BashExecutionMessage, BranchSummaryMessage, CompactionSummaryMessage, CustomMessage,
    CustomMessageContent,
};

pub const COMPACTION_SUMMARY_PREFIX: &str = "The conversation history before this point was compacted into the following summary:\n\n<summary>\n";
pub const COMPACTION_SUMMARY_SUFFIX: &str = "\n</summary>";
pub const BRANCH_SUMMARY_PREFIX: &str =
    "The following is a summary of a branch that this conversation came back from:\n\n<summary>\n";
pub const BRANCH_SUMMARY_SUFFIX: &str = "</summary>";

/// Convert a BashExecutionMessage to user message text for LLM context.
pub fn bash_execution_to_text(msg: &BashExecutionMessage) -> String {
    let mut text = format!("Ran `{}`\n", msg.command);
    if !msg.output.is_empty() {
        text.push_str(&format!("```\n{}\n```", msg.output));
    } else {
        text.push_str("(no output)");
    }
    if msg.cancelled {
        text.push_str("\n\n(command cancelled)");
    } else if let Some(code) = msg.exit_code {
        if code != 0 {
            text.push_str(&format!("\n\nCommand exited with code {code}"));
        }
    }
    if msg.truncated {
        if let Some(ref path) = msg.full_output_path {
            text.push_str(&format!("\n\n[Output truncated. Full output: {path}]"));
        }
    }
    text
}

pub fn create_branch_summary_message(
    summary: String,
    from_id: String,
    timestamp: i64,
) -> BranchSummaryMessage {
    BranchSummaryMessage {
        summary,
        from_id,
        timestamp,
    }
}

pub fn create_compaction_summary_message(
    summary: String,
    tokens_before: u64,
    timestamp: i64,
) -> CompactionSummaryMessage {
    CompactionSummaryMessage {
        summary,
        tokens_before,
        timestamp,
    }
}

pub fn create_custom_message(
    custom_type: String,
    content: CustomMessageContent,
    display: bool,
    details: Option<serde_json::Value>,
    timestamp: i64,
) -> CustomMessage {
    CustomMessage {
        custom_type,
        content,
        display,
        details,
        timestamp,
    }
}

/// Transform AgentMessages (including custom types) to LLM-compatible Messages.
///
/// This is used by:
/// - Agent's transformToLlm option (for prompt calls and queued messages)
/// - Compaction's generateSummary (for summarization)
/// - Custom extensions and tools
pub fn convert_to_llm(
    messages: &[hamr_harness::types::AgentMessage],
) -> Vec<hamr_ai::types::Message> {
    use hamr_ai::types::{Message, MessageContent, MessageRole, TextContent, UserMessage};
    use hamr_harness::types::AgentMessage;

    messages
        .iter()
        .filter_map(|m| match m {
            AgentMessage::BashExecution(msg) => {
                if msg.exclude_from_context {
                    return None;
                }
                Some(Message::User(UserMessage {
                    role: MessageRole::User,
                    content: vec![MessageContent::Text(TextContent {
                        text: bash_execution_to_text(msg),
                        text_signature: None,
                    })],
                    timestamp: chrono::DateTime::from_timestamp_millis(msg.timestamp)
                        .unwrap_or(chrono::DateTime::<chrono::Utc>::MIN_UTC),
                }))
            }
            AgentMessage::Custom(msg) => {
                let content = match &msg.content {
                    CustomMessageContent::Text(t) => {
                        vec![MessageContent::Text(TextContent {
                            text: t.clone(),
                            text_signature: None,
                        })]
                    }
                    CustomMessageContent::Blocks(blocks) => blocks.clone(),
                };
                Some(Message::User(UserMessage {
                    role: MessageRole::User,
                    content,
                    timestamp: chrono::DateTime::from_timestamp_millis(msg.timestamp)
                        .unwrap_or(chrono::DateTime::<chrono::Utc>::MIN_UTC),
                }))
            }
            AgentMessage::BranchSummary(msg) => {
                let text = format!(
                    "{}{}{}",
                    BRANCH_SUMMARY_PREFIX, msg.summary, BRANCH_SUMMARY_SUFFIX
                );
                Some(Message::User(UserMessage {
                    role: MessageRole::User,
                    content: vec![MessageContent::Text(TextContent {
                        text,
                        text_signature: None,
                    })],
                    timestamp: chrono::DateTime::from_timestamp_millis(msg.timestamp)
                        .unwrap_or(chrono::DateTime::<chrono::Utc>::MIN_UTC),
                }))
            }
            AgentMessage::CompactionSummary(msg) => {
                let text = format!(
                    "{}{}{}",
                    COMPACTION_SUMMARY_PREFIX, msg.summary, COMPACTION_SUMMARY_SUFFIX
                );
                Some(Message::User(UserMessage {
                    role: MessageRole::User,
                    content: vec![MessageContent::Text(TextContent {
                        text,
                        text_signature: None,
                    })],
                    timestamp: chrono::DateTime::from_timestamp_millis(msg.timestamp)
                        .unwrap_or(chrono::DateTime::<chrono::Utc>::MIN_UTC),
                }))
            }
            AgentMessage::User(m) => Some(Message::User(m.clone())),
            AgentMessage::Assistant(m) => Some(Message::Assistant(m.clone())),
            AgentMessage::ToolResult(m) => Some(Message::ToolResult(m.clone())),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use hamr_harness::types::AgentMessage;

    #[test]
    fn test_bash_execution_to_text_with_output() {
        let msg = BashExecutionMessage {
            command: "ls -la".to_string(),
            output: "file1.txt\nfile2.txt".to_string(),
            exit_code: Some(0),
            cancelled: false,
            truncated: false,
            full_output_path: None,
            exclude_from_context: false,
            timestamp: 0,
        };
        let text = bash_execution_to_text(&msg);
        assert!(text.contains("ls -la"));
        assert!(text.contains("file1.txt"));
        assert!(text.contains("file2.txt"));
        assert!(!text.contains("cancelled"));
        assert!(!text.contains("exit code"));
    }

    #[test]
    fn test_bash_execution_to_text_no_output() {
        let msg = BashExecutionMessage {
            command: "echo hello".to_string(),
            output: String::new(),
            exit_code: Some(0),
            cancelled: false,
            truncated: false,
            full_output_path: None,
            exclude_from_context: false,
            timestamp: 0,
        };
        let text = bash_execution_to_text(&msg);
        assert!(!text.contains("```"));
        assert!(text.contains("(no output)"));
    }

    #[test]
    fn test_bash_execution_to_text_nonzero_exit() {
        let msg = BashExecutionMessage {
            command: "bad-command".to_string(),
            output: String::new(),
            exit_code: Some(127),
            cancelled: false,
            truncated: false,
            full_output_path: None,
            exclude_from_context: false,
            timestamp: 0,
        };
        let text = bash_execution_to_text(&msg);
        assert!(text.contains("127"));
    }

    #[test]
    fn test_bash_execution_text_cancelled() {
        let msg = BashExecutionMessage {
            command: "sleep 100".to_string(),
            output: String::new(),
            exit_code: None,
            cancelled: true,
            truncated: false,
            full_output_path: None,
            exclude_from_context: false,
            timestamp: 0,
        };
        let text = bash_execution_to_text(&msg);
        assert!(text.contains("cancelled"));
    }

    #[test]
    fn test_bash_execution_truncated_with_path() {
        let msg = BashExecutionMessage {
            command: "long-output".to_string(),
            output: "lots of data".to_string(),
            exit_code: Some(0),
            cancelled: false,
            truncated: true,
            full_output_path: Some("/tmp/full-output.txt".to_string()),
            exclude_from_context: false,
            timestamp: 0,
        };
        let text = bash_execution_to_text(&msg);
        assert!(text.contains("Output truncated"));
        assert!(text.contains("/tmp/full-output.txt"));
    }

    #[test]
    fn test_create_branch_summary_message() {
        let msg =
            create_branch_summary_message("summary text".to_string(), "branch-1".to_string(), 1000);
        assert_eq!(msg.summary, "summary text");
        assert_eq!(msg.from_id, "branch-1");
        assert_eq!(msg.timestamp, 1000);
    }

    #[test]
    fn test_create_compaction_summary_message() {
        let msg = create_compaction_summary_message("summary".to_string(), 1000, 2000);
        assert_eq!(msg.summary, "summary");
        assert_eq!(msg.tokens_before, 1000);
        assert_eq!(msg.timestamp, 2000);
    }

    #[test]
    fn test_create_custom_message_text() {
        let msg = create_custom_message(
            "note".to_string(),
            CustomMessageContent::Text("hello".to_string()),
            true,
            None,
            3000,
        );
        assert_eq!(msg.custom_type, "note");
        match &msg.content {
            CustomMessageContent::Text(t) => assert_eq!(t, "hello"),
            _ => panic!("expected Text variant"),
        }
        assert!(msg.display);
        assert_eq!(msg.timestamp, 3000);
    }

    fn content_text(content: &[hamr_ai::types::MessageContent]) -> &str {
        match &content[0] {
            hamr_ai::types::MessageContent::Text(t) => &t.text,
            _ => panic!("expected Text content"),
        }
    }

    #[test]
    fn test_convert_to_llm_user_message() {
        use hamr_ai::types::MessageRole;
        let agent_msg = AgentMessage::User(hamr_ai::types::UserMessage {
            role: MessageRole::User,
            content: vec![hamr_ai::types::MessageContent::Text(
                hamr_ai::types::TextContent {
                    text: "hello".to_string(),
                    text_signature: None,
                },
            )],
            timestamp: chrono::DateTime::from_timestamp_millis(0).unwrap(),
        });
        let llm = convert_to_llm(&[agent_msg]);
        assert_eq!(llm.len(), 1);
        match &llm[0] {
            hamr_ai::types::Message::User(m) => {
                assert_eq!(content_text(&m.content), "hello");
            }
            _ => panic!("Expected User message"),
        }
    }

    #[test]
    fn test_convert_to_llm_excludes_bash_when_exclude_from_context() {
        use hamr_ai::types::MessageRole;
        let bash_msg = BashExecutionMessage {
            command: "secret".to_string(),
            output: String::new(),
            exit_code: Some(0),
            cancelled: false,
            truncated: false,
            full_output_path: None,
            exclude_from_context: true,
            timestamp: 0,
        };
        let agent_msg = AgentMessage::BashExecution(bash_msg);
        let llm = convert_to_llm(&[agent_msg]);
        assert_eq!(llm.len(), 0);
    }

    #[test]
    fn test_convert_to_llm_branch_summary_includes_prefix_suffix() {
        let msg = create_branch_summary_message("test summary".to_string(), "b1".to_string(), 0);
        let agent_msg = AgentMessage::BranchSummary(msg);
        let llm = convert_to_llm(&[agent_msg]);
        assert_eq!(llm.len(), 1);
        match &llm[0] {
            hamr_ai::types::Message::User(m) => {
                let text = content_text(&m.content);
                assert!(text.contains("summary of a branch"));
                assert!(text.contains("test summary"));
            }
            _ => panic!("Expected User message"),
        }
    }

    #[test]
    fn test_convert_to_llm_compaction_summary_includes_prefix_suffix() {
        let msg = create_compaction_summary_message("compacted".to_string(), 500, 0);
        let agent_msg = AgentMessage::CompactionSummary(msg);
        let llm = convert_to_llm(&[agent_msg]);
        assert_eq!(llm.len(), 1);
        match &llm[0] {
            hamr_ai::types::Message::User(m) => {
                let text = content_text(&m.content);
                assert!(text.contains("compacted"));
                assert!(text.contains("<summary>"));
            }
            _ => panic!("Expected User message"),
        }
    }

    #[test]
    fn test_convert_to_llm_custom_message() {
        let msg = create_custom_message(
            "tool_result".to_string(),
            CustomMessageContent::Text("tool output".to_string()),
            false,
            None,
            0,
        );
        let agent_msg = AgentMessage::Custom(msg);
        let llm = convert_to_llm(&[agent_msg]);
        assert_eq!(llm.len(), 1);
    }

    #[test]
    fn test_convert_to_llm_assistant_and_tool_result() {
        use hamr_ai::types::{AssistantContentBlock, MessageRole, StopReason, Usage, UsageCost};
        let asst = hamr_ai::types::AssistantMessage {
            role: MessageRole::Assistant,
            content: vec![AssistantContentBlock::Text(hamr_ai::types::TextContent {
                text: "response".to_string(),
                text_signature: None,
            })],
            api: "test".to_string(),
            provider: "test".to_string(),
            model: "test-model".to_string(),
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
            timestamp: chrono::DateTime::from_timestamp_millis(0).unwrap(),
        };
        let tool_res = hamr_ai::types::ToolResultMessage {
            role: MessageRole::ToolResult,
            tool_call_id: "tc-1".to_string(),
            tool_name: "test".to_string(),
            content: vec![hamr_ai::types::MessageContent::Text(
                hamr_ai::types::TextContent {
                    text: "tool result".to_string(),
                    text_signature: None,
                },
            )],
            details: None,
            is_error: false,
            timestamp: chrono::DateTime::from_timestamp_millis(0).unwrap(),
        };
        let msgs = vec![
            AgentMessage::Assistant(asst),
            AgentMessage::ToolResult(tool_res),
        ];
        let llm = convert_to_llm(&msgs);
        assert_eq!(llm.len(), 2);
    }
}
