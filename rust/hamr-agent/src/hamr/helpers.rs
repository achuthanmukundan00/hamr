//! Hamr helper utilities.
//!
//! Mirrors `packages/coding-agent/src/hamr/helpers.ts`.

use hamr_ai::types::{AssistantContentBlock, AssistantMessage, MessageContent};

/// Build a composite model key from provider and model names.
pub fn model_key(provider: &str, model: &str) -> String {
    format!("{}/{}", provider, model)
}

/// Extract concatenated text content from user/tool message content blocks.
///
/// Mirror of `contentText(content)` — handles arrays of `MessageContent`,
/// filtering for text blocks and concatenating without separators.
pub fn content_text(content: &[MessageContent]) -> String {
    let mut out = String::new();
    for block in content {
        if let MessageContent::Text(t) = block {
            out.push_str(&t.text);
        }
    }
    out
}

/// Extract all text content from an assistant message.
///
/// Mirror of `getAssistantText(message)` — joins text blocks with empty
/// string (blocks already include necessary whitespace from the model output).
pub fn get_assistant_text(message: &AssistantMessage) -> String {
    let mut out = String::new();
    for block in &message.content {
        if let AssistantContentBlock::Text(t) = block {
            out.push_str(&t.text);
        }
    }
    out
}

/// Extract thinking/reasoning content from an assistant message.
///
/// Returns `None` when there is no thinking content or it's all empty.
pub fn get_thinking_text(message: &AssistantMessage) -> Option<String> {
    let mut out: Vec<String> = Vec::new();
    for block in &message.content {
        if let AssistantContentBlock::Thinking(t) = block {
            let trimmed = t.thinking.trim();
            if !trimmed.is_empty() {
                out.push(trimmed.to_string());
            }
        }
    }
    if out.is_empty() {
        None
    } else {
        Some(out.join("\n"))
    }
}

/// Check whether an assistant message contains at least one tool call.
pub fn has_tool_calls(message: &AssistantMessage) -> bool {
    message
        .content
        .iter()
        .any(|block| matches!(block, AssistantContentBlock::ToolCall(_)))
}

/// Extract file-path-like hints from a text string.
///
/// Returns up to 12 unique file hints. Uses the same regex as the TS
/// implementation, adapted for Rust's regex crate (no look-ahead support).
pub fn file_hints(text: &str) -> Vec<String> {
    let re =
        regex::Regex::new(r"(?:^|\s)([./~]?[A-Za-z0-9._@/-]+\.[A-Za-z0-9]{1,8})(?:\s|$|[:),])")
            .expect("file_hints regex is valid");
    let mut seen = std::collections::HashSet::new();
    let mut results: Vec<String> = Vec::new();

    for cap in re.captures_iter(text) {
        let m = cap.get(1).unwrap().as_str().trim().to_string();
        if m.len() < 240 && seen.insert(m.clone()) {
            results.push(m);
            if results.len() >= 12 {
                break;
            }
        }
    }

    results
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use hamr_ai::types::{TextContent, ThinkingContent, ToolCall};

    #[test]
    fn test_model_key() {
        assert_eq!(model_key("anthropic", "claude-3"), "anthropic/claude-3");
    }

    #[test]
    fn test_content_text() {
        let content = vec![
            MessageContent::Text(TextContent {
                text: "hello".into(),
                text_signature: None,
            }),
            MessageContent::Text(TextContent {
                text: "world".into(),
                text_signature: None,
            }),
        ];
        // TS joins with "" — no space between blocks
        assert_eq!(content_text(&content), "helloworld");
    }

    #[test]
    fn test_content_text_empty() {
        assert_eq!(content_text(&[]), "");
    }

    #[test]
    fn test_get_assistant_text() {
        let msg = AssistantMessage {
            role: hamr_ai::types::MessageRole::Assistant,
            content: vec![AssistantContentBlock::Text(TextContent {
                text: "Hello".into(),
                text_signature: None,
            })],
            api: "anthropic".into(),
            provider: "anthropic".into(),
            model: "claude-3".into(),
            response_model: None,
            response_id: None,
            usage: hamr_ai::types::Usage {
                input: 10,
                output: 20,
                cache_read: 0,
                cache_write: 0,
                cache_write_1h: None,
                total_tokens: 30,
                cost: hamr_ai::types::UsageCost {
                    input: 0.0,
                    output: 0.0,
                    cache_read: 0.0,
                    cache_write: 0.0,
                    total: 0.0,
                },
            },
            stop_reason: hamr_ai::types::StopReason::Stop,
            error_message: None,
            diagnostics: None,
            timestamp: Utc::now(),
        };
        assert_eq!(get_assistant_text(&msg), "Hello");
    }

    #[test]
    fn test_get_assistant_text_multiple_blocks() {
        let msg = AssistantMessage {
            content: vec![
                AssistantContentBlock::Text(TextContent {
                    text: "first".into(),
                    text_signature: None,
                }),
                AssistantContentBlock::Text(TextContent {
                    text: "second".into(),
                    text_signature: None,
                }),
            ],
            ..make_assistant_base()
        };
        // TS joins with "" — no spaces
        assert_eq!(get_assistant_text(&msg), "firstsecond");
    }

    #[test]
    fn test_get_thinking_text() {
        let msg = AssistantMessage {
            content: vec![
                AssistantContentBlock::Thinking(ThinkingContent {
                    thinking: "Let me think...".into(),
                    thinking_signature: None,
                    redacted: false,
                }),
                AssistantContentBlock::Text(TextContent {
                    text: "Here's my answer".into(),
                    text_signature: None,
                }),
            ],
            ..make_assistant_base()
        };
        let thinking = get_thinking_text(&msg);
        assert!(thinking.is_some());
        assert_eq!(thinking.unwrap(), "Let me think...");
    }

    #[test]
    fn test_get_thinking_text_none() {
        let msg = AssistantMessage {
            content: vec![AssistantContentBlock::Text(TextContent {
                text: "No thinking here".into(),
                text_signature: None,
            })],
            ..make_assistant_base()
        };
        assert!(get_thinking_text(&msg).is_none());
    }

    #[test]
    fn test_has_tool_calls() {
        let msg_no_tools = AssistantMessage {
            content: vec![AssistantContentBlock::Text(TextContent {
                text: "no tools".into(),
                text_signature: None,
            })],
            ..make_assistant_base()
        };
        assert!(!has_tool_calls(&msg_no_tools));

        let msg_with_tools = AssistantMessage {
            content: vec![AssistantContentBlock::ToolCall(ToolCall {
                id: "call_1".into(),
                name: "bash".into(),
                arguments: serde_json::json!({"command": "ls"}),
                thought_signature: None,
            })],
            ..make_assistant_base()
        };
        assert!(has_tool_calls(&msg_with_tools));
    }

    #[test]
    fn test_file_hints() {
        let text = "Check out src/main.rs and /etc/hosts";
        let hints = file_hints(text);
        assert!(hints.contains(&"src/main.rs".to_string()));
        // /etc/hosts has no extension, so it won't match the file regex
        assert_eq!(hints.len(), 1);
    }

    #[test]
    fn test_file_hints_limit() {
        let many = (0..20)
            .map(|i| format!("/path/file{}.rs", i))
            .collect::<Vec<_>>()
            .join(" ");
        let hints = file_hints(&many);
        assert!(hints.len() <= 12);
    }

    #[test]
    fn test_file_hints_dedupe() {
        let text = "src/main.rs src/main.rs src/main.rs";
        let hints = file_hints(text);
        assert_eq!(hints.len(), 1);
    }

    fn make_assistant_base() -> AssistantMessage {
        AssistantMessage {
            role: hamr_ai::types::MessageRole::Assistant,
            content: vec![],
            api: "anthropic".into(),
            provider: "anthropic".into(),
            model: "claude-3".into(),
            response_model: None,
            response_id: None,
            usage: hamr_ai::types::Usage {
                input: 0,
                output: 0,
                cache_read: 0,
                cache_write: 0,
                cache_write_1h: None,
                total_tokens: 0,
                cost: hamr_ai::types::UsageCost {
                    input: 0.0,
                    output: 0.0,
                    cache_read: 0.0,
                    cache_write: 0.0,
                    total: 0.0,
                },
            },
            stop_reason: hamr_ai::types::StopReason::Stop,
            error_message: None,
            diagnostics: None,
            timestamp: Utc::now(),
        }
    }
}
