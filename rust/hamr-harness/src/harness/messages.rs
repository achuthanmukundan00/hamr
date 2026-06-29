//! Port of `packages/agent/src/harness/messages.ts`.

use crate::types::{
    AgentMessage, BashExecutionMessage, BranchSummaryMessage, CompactionSummaryMessage,
    CustomMessage, CustomMessageContent,
};
use chrono::{DateTime, TimeZone, Utc};
use hamr_ai::types::{Message, MessageContent, MessageRole, TextContent, UserMessage};

pub const COMPACTION_SUMMARY_PREFIX: &str = "The conversation history before this point was compacted into the following summary:\n\n<summary>\n";
pub const COMPACTION_SUMMARY_SUFFIX: &str = "\n</summary>";

pub const BRANCH_SUMMARY_PREFIX: &str =
    "The following is a summary of a branch that this conversation came back from:\n\n<summary>\n";
pub const BRANCH_SUMMARY_SUFFIX: &str = "</summary>";

fn timestamp_from_rfc3339(timestamp: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(timestamp)
        .map(|timestamp| timestamp.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now())
}

fn timestamp_from_millis(timestamp_ms: i64) -> DateTime<Utc> {
    Utc.timestamp_millis_opt(timestamp_ms)
        .single()
        .unwrap_or_else(Utc::now)
}

pub fn bash_execution_to_text(message: &BashExecutionMessage) -> String {
    let mut text = format!("Ran `{}`\n", message.command);
    if message.output.is_empty() {
        text.push_str("(no output)");
    } else {
        text.push_str("```\n");
        text.push_str(&message.output);
        text.push_str("\n```");
    }

    if message.cancelled {
        text.push_str("\n\n(command cancelled)");
    } else if message.exit_code.is_some() && message.exit_code != Some(0) {
        text.push_str(&format!(
            "\n\nCommand exited with code {}",
            message.exit_code.unwrap_or_default()
        ));
    }

    if message.truncated {
        if let Some(full_output_path) = &message.full_output_path {
            text.push_str(&format!(
                "\n\n[Output truncated. Full output: {full_output_path}]"
            ));
        }
    }

    text
}

pub fn create_branch_summary_message(
    summary: impl Into<String>,
    from_id: impl Into<String>,
    timestamp: &str,
) -> BranchSummaryMessage {
    BranchSummaryMessage {
        summary: summary.into(),
        from_id: from_id.into(),
        timestamp: timestamp_from_rfc3339(timestamp).timestamp_millis(),
    }
}

pub fn create_compaction_summary_message(
    summary: impl Into<String>,
    tokens_before: u64,
    timestamp: &str,
) -> CompactionSummaryMessage {
    CompactionSummaryMessage {
        summary: summary.into(),
        tokens_before,
        timestamp: timestamp_from_rfc3339(timestamp).timestamp_millis(),
    }
}

pub fn create_custom_message(
    custom_type: impl Into<String>,
    content: CustomMessageContent,
    display: bool,
    details: Option<serde_json::Value>,
    timestamp: &str,
) -> CustomMessage {
    CustomMessage {
        custom_type: custom_type.into(),
        content,
        display,
        details,
        timestamp: timestamp_from_rfc3339(timestamp).timestamp_millis(),
    }
}

fn custom_content_to_blocks(content: &CustomMessageContent) -> Vec<MessageContent> {
    match content {
        CustomMessageContent::Text(text) => vec![MessageContent::Text(TextContent {
            text: text.clone(),
            text_signature: None,
        })],
        CustomMessageContent::Blocks(blocks) => blocks.clone(),
    }
}

pub fn convert_to_llm(messages: &[AgentMessage]) -> Vec<Message> {
    messages
        .iter()
        .filter_map(|message| match message {
            AgentMessage::User(message) => Some(Message::User(message.clone())),
            AgentMessage::Assistant(message) => Some(Message::Assistant(message.clone())),
            AgentMessage::ToolResult(message) => Some(Message::ToolResult(message.clone())),
            AgentMessage::BashExecution(message) => {
                if message.exclude_from_context {
                    None
                } else {
                    Some(Message::User(UserMessage {
                        role: MessageRole::User,
                        content: vec![MessageContent::Text(TextContent {
                            text: bash_execution_to_text(message),
                            text_signature: None,
                        })],
                        timestamp: timestamp_from_millis(message.timestamp),
                    }))
                }
            }
            AgentMessage::Custom(message) => Some(Message::User(UserMessage {
                role: MessageRole::User,
                content: custom_content_to_blocks(&message.content),
                timestamp: timestamp_from_millis(message.timestamp),
            })),
            AgentMessage::BranchSummary(message) => Some(Message::User(UserMessage {
                role: MessageRole::User,
                content: vec![MessageContent::Text(TextContent {
                    text: format!(
                        "{BRANCH_SUMMARY_PREFIX}{}{BRANCH_SUMMARY_SUFFIX}",
                        message.summary
                    ),
                    text_signature: None,
                })],
                timestamp: timestamp_from_millis(message.timestamp),
            })),
            AgentMessage::CompactionSummary(message) => Some(Message::User(UserMessage {
                role: MessageRole::User,
                content: vec![MessageContent::Text(TextContent {
                    text: format!(
                        "{COMPACTION_SUMMARY_PREFIX}{}{COMPACTION_SUMMARY_SUFFIX}",
                        message.summary
                    ),
                    text_signature: None,
                })],
                timestamp: timestamp_from_millis(message.timestamp),
            })),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{
        COMPACTION_SUMMARY_PREFIX, bash_execution_to_text, convert_to_llm,
        create_branch_summary_message, create_compaction_summary_message,
    };
    use crate::types::{AgentMessage, BashExecutionMessage, CustomMessage, CustomMessageContent};
    use hamr_ai::types::{Message, MessageContent};

    #[test]
    fn formats_bash_execution_text() {
        let text = bash_execution_to_text(&BashExecutionMessage {
            command: "echo hi".to_string(),
            output: "hi".to_string(),
            exit_code: Some(1),
            cancelled: false,
            truncated: true,
            full_output_path: Some("/tmp/out.txt".to_string()),
            timestamp: 0,
            exclude_from_context: false,
        });

        assert!(text.contains("Ran `echo hi`"));
        assert!(text.contains("Command exited with code 1"));
        assert!(text.contains("/tmp/out.txt"));
    }

    #[test]
    fn converts_custom_messages_to_user_messages() {
        let messages = vec![
            AgentMessage::Custom(CustomMessage {
                custom_type: "note".to_string(),
                content: CustomMessageContent::Text("hello".to_string()),
                display: true,
                details: None,
                timestamp: 1_700_000_000_000,
            }),
            AgentMessage::BashExecution(BashExecutionMessage {
                command: "pwd".to_string(),
                output: String::new(),
                exit_code: Some(0),
                cancelled: false,
                truncated: false,
                full_output_path: None,
                timestamp: 1_700_000_000_001,
                exclude_from_context: true,
            }),
        ];

        let converted = convert_to_llm(&messages);
        assert_eq!(converted.len(), 1);

        match &converted[0] {
            Message::User(message) => match &message.content[0] {
                MessageContent::Text(text) => assert_eq!(text.text, "hello"),
                other => panic!("unexpected content: {other:?}"),
            },
            other => panic!("unexpected message: {other:?}"),
        }
    }

    #[test]
    fn builds_summary_messages() {
        let branch = create_branch_summary_message("branch body", "node-1", "2025-01-01T00:00:00Z");
        assert_eq!(branch.from_id, "node-1");

        let compaction = create_compaction_summary_message("summary", 42, "2025-01-01T00:00:00Z");
        assert_eq!(compaction.tokens_before, 42);

        let converted = convert_to_llm(&[AgentMessage::CompactionSummary(compaction)]);
        match &converted[0] {
            Message::User(message) => match &message.content[0] {
                MessageContent::Text(text) => {
                    assert!(text.text.starts_with(COMPACTION_SUMMARY_PREFIX));
                    assert!(text.text.contains("summary"));
                }
                other => panic!("unexpected content: {other:?}"),
            },
            other => panic!("unexpected message: {other:?}"),
        }
    }
}
