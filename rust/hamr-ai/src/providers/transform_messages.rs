//! Message transformation pipeline for cross-provider compatibility.
//!
//! Port of `packages/ai/src/providers/transform-messages.ts`.
//!
//! Responsibilities:
//! * Downgrade/remove unsupported image content for non-vision models
//! * Normalize tool call IDs for providers with incompatible ID formats
//! * Handle thinking/reasoning blocks when replaying across models
//! * Strip error/aborted assistant messages
//! * Insert synthetic empty tool results for orphaned tool calls

use crate::types::{
    Api, AssistantContentBlock, AssistantMessage, InputModality, Message, MessageContent, Model,
    TextContent, ToolCall, ToolResultMessage,
};
use chrono::Utc;
use std::collections::{HashMap, HashSet};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn api_to_string(api: Api) -> String {
    match api {
        Api::OpenAiCompletions => "openai-completions".into(),
        Api::MistralConversations => "mistral-conversations".into(),
        Api::OpenAiResponses => "openai-responses".into(),
        Api::AzureOpenAiResponses => "azure-openai-responses".into(),
        Api::OpenAiCodexResponses => "openai-codex-responses".into(),
        Api::AnthropicMessages => "anthropic-messages".into(),
        Api::BedrockConverseStream => "bedrock-converse-stream".into(),
        Api::GoogleGenerativeAi => "google-generative-ai".into(),
        Api::GoogleVertex => "google-vertex".into(),
    }
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const NON_VISION_USER_IMAGE_PLACEHOLDER: &str = "(image omitted: model does not support images)";
const NON_VISION_TOOL_IMAGE_PLACEHOLDER: &str =
    "(tool image omitted: model does not support images)";

// ---------------------------------------------------------------------------
// Private helpers
// ---------------------------------------------------------------------------

/// Replace all image content blocks with a single placeholder text block.
/// Consecutive images are collapsed into one placeholder.
fn replace_images_with_placeholder(
    content: &[MessageContent],
    placeholder: &str,
) -> Vec<MessageContent> {
    let mut result: Vec<MessageContent> = Vec::new();
    let mut previous_was_placeholder = false;

    for block in content {
        if matches!(block, MessageContent::Image(_)) {
            if !previous_was_placeholder {
                result.push(MessageContent::Text(TextContent {
                    text: placeholder.to_string(),
                    text_signature: None,
                }));
            }
            previous_was_placeholder = true;
            continue;
        }

        // Check if this text block is already our placeholder
        if let MessageContent::Text(tc) = block {
            if tc.text == placeholder {
                previous_was_placeholder = true;
                // Avoid duplicating: only push if we didn't just insert one
                if !matches!(result.last(), Some(MessageContent::Text(t)) if t.text == placeholder)
                {
                    result.push(block.clone());
                }
            } else {
                previous_was_placeholder = false;
                result.push(block.clone());
            }
        } else {
            result.push(block.clone());
            previous_was_placeholder = false;
        }
    }

    result
}

/// If the model does not support images, replace image blocks in user and
/// tool-result messages with a placeholder.
fn downgrade_unsupported_images(messages: Vec<Message>, model: &Model) -> Vec<Message> {
    if model.input.contains(&InputModality::Image) {
        return messages;
    }

    messages
        .into_iter()
        .map(|msg| match msg {
            Message::User(mut user_msg) => {
                user_msg.content = replace_images_with_placeholder(
                    &user_msg.content,
                    NON_VISION_USER_IMAGE_PLACEHOLDER,
                );
                Message::User(user_msg)
            }
            Message::ToolResult(mut tool_msg) => {
                tool_msg.content = replace_images_with_placeholder(
                    &tool_msg.content,
                    NON_VISION_TOOL_IMAGE_PLACEHOLDER,
                );
                Message::ToolResult(tool_msg)
            }
            other => other,
        })
        .collect()
}

/// Default tool call ID normalizer — a no-op passthrough.
fn default_normalize_tool_call_id(id: &str, _model: &Model, _source: &AssistantMessage) -> String {
    id.to_string()
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Transform a sequence of messages for compatibility with the target model.
///
/// The transformation pipeline:
/// 1. Downgrade/remove images for non-vision models
/// 2. Handle thinking blocks (keep same-model, convert to text cross-model)
/// 3. Normalize tool call IDs (for cross-provider compatibility)
/// 4. Strip error/aborted assistant messages
/// 5. Insert synthetic empty tool results for orphaned tool calls
///
/// `normalize_tool_call_id` is an optional callback that receives the original
/// tool call ID, the target model, and the source assistant message, and returns
/// a normalized ID. This is needed when IDs from one provider (e.g. OpenAI
/// Responses with 450+ char IDs containing `|`) must be shortened to match
/// another provider's constraints (e.g. Anthropic `^[a-zA-Z0-9_-]+$`, max 64).
pub fn transform_messages(
    messages: Vec<Message>,
    model: &Model,
    normalize_tool_call_id: Option<&dyn Fn(&str, &Model, &AssistantMessage) -> String>,
) -> Vec<Message> {
    // Build a map of original tool call IDs to normalized IDs
    let mut tool_call_id_map: HashMap<String, String> = HashMap::new();
    let image_aware_messages = downgrade_unsupported_images(messages, model);

    // Type alias for the normalizer function reference
    let normalize: &dyn Fn(&str, &Model, &AssistantMessage) -> String =
        normalize_tool_call_id.unwrap_or(&default_normalize_tool_call_id);

    // -----------------------------------------------------------------------
    // First pass: transform messages (image downgrade, thinking blocks,
    // tool call ID normalization)
    // -----------------------------------------------------------------------
    let transformed: Vec<Message> = image_aware_messages
        .into_iter()
        .map(|msg| {
            // User messages pass through unchanged
            match msg {
                Message::User(_) => return msg,
                _ => {}
            }

            // Handle toolResult messages — normalize toolCallId if we have a mapping
            if let Message::ToolResult(ref tool_msg) = msg {
                if let Some(normalized_id) = tool_call_id_map.get(&tool_msg.tool_call_id) {
                    if *normalized_id != tool_msg.tool_call_id {
                        let mut updated = tool_msg.clone();
                        updated.tool_call_id = normalized_id.clone();
                        return Message::ToolResult(updated);
                    }
                }
                return msg;
            }

            // Assistant messages need transformation check
            if let Message::Assistant(ref assistant_msg) = msg {
                let is_same_model = assistant_msg.provider == model.provider
                    && assistant_msg.api == api_to_string(model.api)
                    && assistant_msg.model == model.id;

                let transformed_content: Vec<AssistantContentBlock> = assistant_msg
                    .content
                    .iter()
                    .flat_map(|block| -> Option<AssistantContentBlock> {
                        match block {
                            AssistantContentBlock::Thinking(tc) => {
                                // Redacted thinking is opaque encrypted content, only valid
                                // for the same model. Drop it for cross-model to avoid API errors.
                                if tc.redacted {
                                    return if is_same_model {
                                        Some(block.clone())
                                    } else {
                                        None
                                    };
                                }

                                // For same model: keep thinking blocks with signatures
                                // (needed for replay) even if the thinking text is empty
                                // (OpenAI encrypted reasoning).
                                if is_same_model && tc.thinking_signature.is_some() {
                                    return Some(block.clone());
                                }

                                // Skip empty thinking blocks, convert others to plain text
                                if tc.thinking.trim().is_empty() {
                                    return None;
                                }
                                if is_same_model {
                                    return Some(block.clone());
                                }

                                // Cross-model: convert thinking to text
                                Some(AssistantContentBlock::Text(TextContent {
                                    text: tc.thinking.clone(),
                                    text_signature: None,
                                }))
                            }

                            AssistantContentBlock::Text(_txt) => Some(block.clone()),

                            AssistantContentBlock::ToolCall(tool_call) => {
                                let mut normalized_tool_call: ToolCall = tool_call.clone();

                                // Strip thought signatures when crossing models
                                if !is_same_model && tool_call.thought_signature.is_some() {
                                    normalized_tool_call.thought_signature = None;
                                }

                                // Normalize tool call ID when crossing models
                                if !is_same_model {
                                    let normalized_id =
                                        normalize(&tool_call.id, model, assistant_msg);
                                    if normalized_id != tool_call.id {
                                        tool_call_id_map
                                            .insert(tool_call.id.clone(), normalized_id.clone());
                                        normalized_tool_call.id = normalized_id;
                                    }
                                }

                                Some(AssistantContentBlock::ToolCall(normalized_tool_call))
                            }
                        }
                    })
                    .collect();

                let mut updated = assistant_msg.clone();
                updated.content = transformed_content;
                return Message::Assistant(updated);
            }

            msg
        })
        .collect();

    // -----------------------------------------------------------------------
    // Second pass: insert synthetic empty tool results for orphaned tool calls
    // This preserves thinking signatures and satisfies API requirements.
    // -----------------------------------------------------------------------
    let mut result: Vec<Message> = Vec::new();
    let mut pending_tool_calls: Vec<ToolCall> = Vec::new();
    let mut existing_tool_result_ids: HashSet<String> = HashSet::new();

    let insert_synthetic_tool_results =
        |r: &mut Vec<Message>, pending: &mut Vec<ToolCall>, existing_ids: &mut HashSet<String>| {
            for tc in pending.drain(..) {
                if !existing_ids.contains(&tc.id) {
                    r.push(Message::ToolResult(ToolResultMessage {
                        role: crate::types::MessageRole::ToolResult,
                        tool_call_id: tc.id,
                        tool_name: tc.name,
                        content: vec![MessageContent::Text(TextContent {
                            text: "No result provided".to_string(),
                            text_signature: None,
                        })],
                        details: None,
                        is_error: true,
                        timestamp: Utc::now(),
                    }));
                }
            }
            existing_ids.clear();
        };

    for msg in transformed {
        match &msg {
            Message::Assistant(assistant_msg) => {
                // If we have pending orphaned tool calls from a previous assistant,
                // insert synthetic results now.
                insert_synthetic_tool_results(
                    &mut result,
                    &mut pending_tool_calls,
                    &mut existing_tool_result_ids,
                );

                // Skip errored/aborted assistant messages entirely.
                // These are incomplete turns that shouldn't be replayed:
                // - May have partial content (reasoning without message, incomplete tool calls)
                // - Replaying them can cause API errors (e.g., OpenAI "reasoning without following item")
                // - The model should retry from the last valid state
                use crate::types::StopReason;
                if assistant_msg.stop_reason == StopReason::Error
                    || assistant_msg.stop_reason == StopReason::Aborted
                {
                    continue;
                }

                // Track tool calls from this assistant message
                let tool_calls: Vec<ToolCall> = assistant_msg
                    .content
                    .iter()
                    .filter_map(|b| {
                        if let AssistantContentBlock::ToolCall(tc) = b {
                            Some(tc.clone())
                        } else {
                            None
                        }
                    })
                    .collect();

                if !tool_calls.is_empty() {
                    pending_tool_calls = tool_calls;
                    existing_tool_result_ids.clear();
                }

                result.push(msg);
            }

            Message::ToolResult(tool_msg) => {
                existing_tool_result_ids.insert(tool_msg.tool_call_id.clone());
                result.push(msg);
            }

            Message::User(_) => {
                // User message interrupts tool flow - insert synthetic results
                // for orphaned calls
                insert_synthetic_tool_results(
                    &mut result,
                    &mut pending_tool_calls,
                    &mut existing_tool_result_ids,
                );
                result.push(msg);
            }
        }
    }

    // If the conversation ends with unresolved tool calls, synthesize results now.
    insert_synthetic_tool_results(
        &mut result,
        &mut pending_tool_calls,
        &mut existing_tool_result_ids,
    );

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::*;
    use chrono::Utc;

    fn claude_model() -> Model {
        Model {
            id: "claude-sonnet-4-6".into(),
            name: "Claude Sonnet 4.6".into(),
            api: Api::AnthropicMessages,
            provider: "github-copilot".into(),
            base_url: "https://api.individual.githubcopilot.com".into(),
            reasoning: true,
            thinking_level_map: None,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.0,
                output: 0.0,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 128_000,
            max_tokens: 16_000,
            headers: None,
            compat: None,
        }
    }

    fn assistant_base(content: Vec<AssistantContentBlock>, stop_reason: StopReason) -> Message {
        Message::Assistant(AssistantMessage {
            role: MessageRole::Assistant,
            content,
            api: "openai-completions".into(),
            provider: "openai".into(),
            model: "gpt-4o".into(),
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
            stop_reason,
            response_id: None,
            error_message: None,
            diagnostics: None,
            response_model: None,
            timestamp: Utc::now(),
        })
    }

    fn anthropic_normalize(id: &str, _model: &Model, _source: &AssistantMessage) -> String {
        let sanitized: String = id
            .chars()
            .map(|c| {
                if c.is_alphanumeric() || c == '_' || c == '-' {
                    c
                } else {
                    '_'
                }
            })
            .collect();
        sanitized.chars().take(64).collect()
    }

    #[test]
    fn thinking_blocks_converted_to_text_when_source_model_differs() {
        let model = claude_model();
        let messages = vec![
            Message::User(UserMessage {
                role: MessageRole::User,
                content: vec![MessageContent::Text(TextContent {
                    text: "hello".into(),
                    text_signature: None,
                })],
                timestamp: Utc::now(),
            }),
            assistant_base(
                vec![
                    AssistantContentBlock::Thinking(ThinkingContent {
                        thinking: "Let me think about this...".into(),
                        thinking_signature: Some("reasoning_content".into()),
                        redacted: false,
                    }),
                    AssistantContentBlock::Text(TextContent {
                        text: "Hi there!".into(),
                        text_signature: None,
                    }),
                ],
                StopReason::Stop,
            ),
        ];

        let result = transform_messages(messages, &model, Some(&anthropic_normalize));
        let assistant = result
            .iter()
            .find_map(|m| {
                if let Message::Assistant(a) = m {
                    Some(a)
                } else {
                    None
                }
            })
            .expect("expected assistant message");

        let thinking_blocks: Vec<_> = assistant
            .content
            .iter()
            .filter(|b| matches!(b, AssistantContentBlock::Thinking(_)))
            .collect();
        let text_blocks: Vec<_> = assistant
            .content
            .iter()
            .filter(|b| matches!(b, AssistantContentBlock::Text(_)))
            .collect();

        assert_eq!(
            thinking_blocks.len(),
            0,
            "thinking blocks should be converted to text"
        );
        assert!(
            text_blocks.len() >= 2,
            "should have at least 2 text blocks (converted thinking + original)"
        );
    }

    #[test]
    fn strips_thought_signature_from_tool_calls_when_migrating_models() {
        let model = claude_model();
        let messages = vec![
            Message::User(UserMessage {
                role: MessageRole::User,
                content: vec![MessageContent::Text(TextContent {
                    text: "run a command".into(),
                    text_signature: None,
                })],
                timestamp: Utc::now(),
            }),
            assistant_base(
                vec![AssistantContentBlock::ToolCall(ToolCall {
                    id: "call_123".into(),
                    name: "bash".into(),
                    arguments: serde_json::json!({ "command": "ls" }),
                    thought_signature: Some(
                        r#"{"type":"reasoning.encrypted","id":"call_123","data":"encrypted"}"#
                            .into(),
                    ),
                })],
                StopReason::ToolUse,
            ),
            Message::ToolResult(ToolResultMessage {
                role: MessageRole::ToolResult,
                tool_call_id: "call_123".into(),
                tool_name: "bash".into(),
                content: vec![MessageContent::Text(TextContent {
                    text: "output".into(),
                    text_signature: None,
                })],
                details: None,
                is_error: false,
                timestamp: Utc::now(),
            }),
        ];

        let result = transform_messages(messages, &model, Some(&anthropic_normalize));
        let assistant = result
            .iter()
            .find_map(|m| {
                if let Message::Assistant(a) = m {
                    Some(a)
                } else {
                    None
                }
            })
            .expect("expected assistant message");
        let tool_call = assistant
            .content
            .iter()
            .find_map(|b| {
                if let AssistantContentBlock::ToolCall(tc) = b {
                    Some(tc)
                } else {
                    None
                }
            })
            .expect("expected tool call");

        assert!(
            tool_call.thought_signature.is_none(),
            "thought_signature should be stripped across models"
        );
    }

    #[test]
    fn adds_synthetic_tool_result_for_trailing_orphaned_tool_calls() {
        let model = claude_model();
        let messages = vec![
            Message::User(UserMessage {
                role: MessageRole::User,
                content: vec![MessageContent::Text(TextContent {
                    text: "read the file".into(),
                    text_signature: None,
                })],
                timestamp: Utc::now(),
            }),
            assistant_base(
                vec![AssistantContentBlock::ToolCall(ToolCall {
                    id: "call_123|fc_123".into(),
                    name: "read".into(),
                    arguments: serde_json::json!({ "path": "README.md" }),
                    thought_signature: None,
                })],
                StopReason::ToolUse,
            ),
        ];

        let result = transform_messages(messages, &model, Some(&anthropic_normalize));
        let last = result.last().expect("expected at least one message");

        assert!(matches!(last, Message::ToolResult(_)));
        if let Message::ToolResult(t) = last {
            assert_eq!(t.tool_call_id, "call_123_fc_123");
            assert_eq!(t.tool_name, "read");
            assert!(t.is_error);
        }
    }

    #[test]
    fn adds_synthetic_result_only_for_still_orphaned_tool_calls() {
        let model = claude_model();
        let messages = vec![
            Message::User(UserMessage {
                role: MessageRole::User,
                content: vec![MessageContent::Text(TextContent {
                    text: "run commands".into(),
                    text_signature: None,
                })],
                timestamp: Utc::now(),
            }),
            assistant_base(
                vec![
                    AssistantContentBlock::ToolCall(ToolCall {
                        id: "call_1|fc_1".into(),
                        name: "read".into(),
                        arguments: serde_json::json!({ "path": "README.md" }),
                        thought_signature: None,
                    }),
                    AssistantContentBlock::ToolCall(ToolCall {
                        id: "call_2|fc_2".into(),
                        name: "bash".into(),
                        arguments: serde_json::json!({ "command": "pwd" }),
                        thought_signature: None,
                    }),
                ],
                StopReason::ToolUse,
            ),
            Message::ToolResult(ToolResultMessage {
                role: MessageRole::ToolResult,
                tool_call_id: "call_1|fc_1".into(),
                tool_name: "read".into(),
                content: vec![MessageContent::Text(TextContent {
                    text: "done".into(),
                    text_signature: None,
                })],
                details: None,
                is_error: false,
                timestamp: Utc::now(),
            }),
        ];

        let result = transform_messages(messages, &model, Some(&anthropic_normalize));
        let synthetic_results: Vec<_> = result
            .iter()
            .filter(|m| {
                if let Message::ToolResult(t) = m {
                    t.is_error
                } else {
                    false
                }
            })
            .collect();

        assert_eq!(synthetic_results.len(), 1);
        if let Message::ToolResult(t) = &synthetic_results[0] {
            assert_eq!(t.tool_call_id, "call_2_fc_2");
            assert_eq!(t.tool_name, "bash");
        }
    }

    #[test]
    fn strips_errored_assistant_messages() {
        let model = claude_model();
        let messages = vec![
            Message::User(UserMessage {
                role: MessageRole::User,
                content: vec![MessageContent::Text(TextContent {
                    text: "hello".into(),
                    text_signature: None,
                })],
                timestamp: Utc::now(),
            }),
            Message::Assistant(AssistantMessage {
                role: MessageRole::Assistant,
                content: vec![],
                api: "openai-completions".into(),
                provider: "openai".into(),
                model: "gpt-4o".into(),
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
                stop_reason: StopReason::Error,
                response_id: None,
                error_message: Some("oops".into()),
                diagnostics: None,
                response_model: None,
                timestamp: Utc::now(),
            }),
            Message::User(UserMessage {
                role: MessageRole::User,
                content: vec![MessageContent::Text(TextContent {
                    text: "try again".into(),
                    text_signature: None,
                })],
                timestamp: Utc::now(),
            }),
        ];

        let result = transform_messages(messages, &model, Some(&anthropic_normalize));
        let assistant_msgs: Vec<_> = result
            .iter()
            .filter(|m| matches!(m, Message::Assistant(_)))
            .collect();
        // The errored assistant message should be stripped, leaving just user messages
        assert_eq!(assistant_msgs.len(), 0);
        assert_eq!(result.len(), 2); // both user messages survive
    }
}
