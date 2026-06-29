//! Shared utilities for Google Generative AI and Google Vertex providers.
//!
//! Port of `packages/ai/src/providers/google-shared.ts`.
//!
//! Provides:
//! - Message → Google GenAI Content[] conversion
//! - Tool → Google functionDeclarations conversion
//! - Google GenAI response → hamr `AssistantMessage` parsing
//! - Stop reason mapping (Google `FinishReason` → hamr `StopReason`)
//! - Thinking/reasoning content block handling
//! - Image tool-result routing (inline vs. separate turn)
//!
//! Uses `serde_json::Value` for Google-specific JSON payload construction since we
//! do not depend on the `@google/genai` SDK.

use regex::Regex;
use serde_json::Value;

use crate::types::{
    AssistantContentBlock, Context, InputModality, Message,
    MessageContent, Model, StopReason, TextContent, Tool, ToolCall,
};

// ---------------------------------------------------------------------------
// Thinking helpers
// ---------------------------------------------------------------------------

/// Thinking level for Gemini 3+ models.
/// Mirrors Google's `ThinkingLevel` enum values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GoogleThinkingLevel {
    Unspecified,
    Minimal,
    Low,
    Medium,
    High,
}

impl GoogleThinkingLevel {
    /// Return the Google API string for this thinking level.
    pub fn as_str(&self) -> &'static str {
        match self {
            GoogleThinkingLevel::Unspecified => "THINKING_LEVEL_UNSPECIFIED",
            GoogleThinkingLevel::Minimal => "MINIMAL",
            GoogleThinkingLevel::Low => "LOW",
            GoogleThinkingLevel::Medium => "MEDIUM",
            GoogleThinkingLevel::High => "HIGH",
        }
    }
}

/// Determines whether a streamed Gemini `Part` should be treated as "thinking".
///
/// Protocol note (Gemini / Vertex AI thought signatures):
/// - `thought: true` is the definitive marker for thinking content (thought summaries).
/// - `thoughtSignature` is an encrypted representation of the model's internal thought
///   process used to preserve reasoning context across multi-turn interactions.
/// - `thoughtSignature` can appear on ANY part type (text, functionCall, etc.) — it does NOT
///   indicate the part itself is thinking content.
/// - For non-functionCall responses, the signature appears on the last part for context replay.
/// - When persisting/replaying model outputs, signature-bearing parts must be preserved as-is;
///   do not merge/move signatures across parts.
///
/// See: <https://ai.google.dev/gemini-api/docs/thought-signatures>
pub fn is_thinking_part(part: &Value) -> bool {
    part.get("thought")
        .and_then(|t| t.as_bool())
        .unwrap_or(false)
}

/// Retain thought signatures during streaming.
///
/// Some backends only send `thoughtSignature` on the first delta for a given block;
/// later deltas may omit it. This helper preserves the last non-empty signature for the
/// current block.
///
/// Note: this does NOT merge or move signatures across distinct response parts. It only
/// prevents a signature from being overwritten with `None` within the same streamed block.
pub fn retain_thought_signature(existing: Option<&str>, incoming: Option<&str>) -> Option<String> {
    match incoming {
        Some(s) if !s.is_empty() => Some(s.to_owned()),
        _ => existing.map(|s| s.to_owned()),
    }
}

/// Thought signatures must be base64 for Google APIs (TYPE_BYTES).
fn is_valid_thought_signature(signature: Option<&str>) -> bool {
    let s = match signature {
        Some(s) if !s.is_empty() => s,
        _ => return false,
    };
    // Length must be a multiple of 4
    if s.len() % 4 != 0 {
        return false;
    }
    // Must match base64 alphabet with optional padding
    s.chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '+' || c == '/' || c == '=')
}

/// Only keep signatures from the same provider/model and with valid base64.
fn resolve_thought_signature(
    is_same_provider_and_model: bool,
    signature: Option<&str>,
) -> Option<String> {
    if is_same_provider_and_model && is_valid_thought_signature(signature) {
        signature.map(|s| s.to_owned())
    } else {
        None
    }
}

// ---------------------------------------------------------------------------
// Model identification helpers
// ---------------------------------------------------------------------------

/// Models via Google APIs that require explicit tool call IDs in function calls/responses.
pub fn requires_tool_call_id(model_id: &str) -> bool {
    let lower = model_id.to_lowercase();
    lower.starts_with("claude-") || lower.starts_with("gpt-oss-")
}

fn get_gemini_major_version(model_id: &str) -> Option<u32> {
    let re =
        Regex::new(r"^gemini(?:-live)?-(\d+)").expect("gemini major version regex should be valid");
    re.captures(&model_id.to_lowercase())
        .and_then(|caps| caps.get(1))
        .and_then(|m| m.as_str().parse().ok())
}

fn supports_multimodal_function_response(model_id: &str) -> bool {
    match get_gemini_major_version(model_id) {
        Some(ver) => ver >= 3,
        None => true,
    }
}

/// Normalize a tool call ID for cross-provider compatibility.
pub fn normalize_tool_call_id(model_id: &str, id: &str) -> String {
    if !requires_tool_call_id(model_id) {
        return id.to_owned();
    }
    // Replace non-alphanumeric (except `_`, `-`) and truncate to 64 chars
    let cleaned: String = id
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '-')
        .take(64)
        .collect();
    cleaned
}

// `sanitizeSurrogates` lives in `utils/sanitize-unicode.ts`; re-export the
// canonical port so the conversion code below mirrors the TS import.
pub use crate::utils::sanitize_unicode::sanitize_surrogates;

// ---------------------------------------------------------------------------
// Message conversion: hamr Messages → Google GenAI Content[]
// ---------------------------------------------------------------------------

/// Convert hamr `Context` messages into Google GenAI `Content` format (`Vec<Value>`).
///
/// The Google API expects:
/// - `user` role for user/tool-result messages
/// - `model` role for assistant messages
/// - Each content has a `parts` array of `Part` objects
///
/// Note: `transform_messages` is called internally (via the existing TS pattern).
/// Convert context messages to Google API format.
///
/// Applies `transform_messages` for tool-call ID normalization and image
/// downgrade, then converts each message to Google's `contents` format.
pub fn convert_messages(model: &Model, context: &Context) -> Vec<Value> {
    use crate::providers::transform_messages::transform_messages;

    let mut contents: Vec<Value> = Vec::new();

    // Apply transform_messages for normalization (tool call IDs, image downgrade)
    let transformed = transform_messages(context.messages.clone(), model, None);

    for msg in &transformed {
        match msg {
            Message::User(user_msg) => {
                // User messages: content is Vec<MessageContent> (text or image)
                let parts: Vec<Value> = user_msg
                    .content
                    .iter()
                    .filter_map(|mc| match mc {
                        MessageContent::Text(tc) => Some(serde_json::json!({
                            "text": sanitize_surrogates(&tc.text)
                        })),
                        MessageContent::Image(ic) => Some(serde_json::json!({
                            "inlineData": {
                                "mimeType": ic.mime_type,
                                "data": ic.data
                            }
                        })),
                    })
                    .collect();
                if parts.is_empty() {
                    continue;
                }
                contents.push(serde_json::json!({
                    "role": "user",
                    "parts": parts
                }));
            }

            Message::Assistant(assistant_msg) => {
                let mut parts: Vec<Value> = Vec::new();
                let is_same_provider_and_model =
                    assistant_msg.provider == model.provider && assistant_msg.model == model.id;

                for block in &assistant_msg.content {
                    match block {
                        AssistantContentBlock::Text(tc) => {
                            // Skip empty text blocks
                            if tc.text.trim().is_empty() {
                                continue;
                            }
                            let thought_sig = resolve_thought_signature(
                                is_same_provider_and_model,
                                tc.text_signature.as_deref(),
                            );
                            let mut part = serde_json::json!({
                                "text": sanitize_surrogates(&tc.text)
                            });
                            if let Some(sig) = thought_sig {
                                part["thoughtSignature"] = Value::String(sig);
                            }
                            parts.push(part);
                        }

                        AssistantContentBlock::Thinking(th) => {
                            // Skip empty thinking blocks
                            if th.thinking.trim().is_empty() {
                                continue;
                            }
                            if is_same_provider_and_model {
                                let thought_sig = resolve_thought_signature(
                                    is_same_provider_and_model,
                                    th.thinking_signature.as_deref(),
                                );
                                let mut part = serde_json::json!({
                                    "thought": true,
                                    "text": sanitize_surrogates(&th.thinking)
                                });
                                if let Some(sig) = thought_sig {
                                    part["thoughtSignature"] = Value::String(sig);
                                }
                                parts.push(part);
                            } else {
                                // Convert thinking block to plain text (no tags to avoid
                                // model mimicking them)
                                parts.push(serde_json::json!({
                                    "text": sanitize_surrogates(&th.thinking)
                                }));
                            }
                        }

                        AssistantContentBlock::ToolCall(tc) => {
                            let thought_sig = resolve_thought_signature(
                                is_same_provider_and_model,
                                tc.thought_signature.as_deref(),
                            );
                            let mut fc = serde_json::json!({
                                "name": tc.name,
                                "args": tc.arguments
                            });
                            if requires_tool_call_id(&model.id) {
                                fc["id"] = Value::String(tc.id.clone());
                            }
                            let mut part = serde_json::json!({
                                "functionCall": fc
                            });
                            if let Some(sig) = thought_sig {
                                part["thoughtSignature"] = Value::String(sig);
                            }
                            parts.push(part);
                        }
                    }
                }

                if parts.is_empty() {
                    continue;
                }
                contents.push(serde_json::json!({
                    "role": "model",
                    "parts": parts
                }));
            }

            Message::ToolResult(tr_msg) => {
                // Extract text content
                let text_parts: Vec<&str> = tr_msg
                    .content
                    .iter()
                    .filter_map(|mc| match mc {
                        MessageContent::Text(tc) => Some(tc.text.as_str()),
                        _ => None,
                    })
                    .collect();
                let text_result = text_parts.join("\n");

                // Extract image content (only if model supports images)
                let image_parts: Vec<Value> = if model.input.contains(&InputModality::Image) {
                    tr_msg
                        .content
                        .iter()
                        .filter_map(|mc| match mc {
                            MessageContent::Image(ic) => Some(serde_json::json!({
                                "inlineData": {
                                    "mimeType": ic.mime_type,
                                    "data": ic.data
                                }
                            })),
                            _ => None,
                        })
                        .collect()
                } else {
                    Vec::new()
                };

                let has_text = !text_result.is_empty();
                let has_images = !image_parts.is_empty();

                let model_supports_multi_fn_response =
                    supports_multimodal_function_response(&model.id);

                // Use "output" key for success, "error" key for errors
                let response_value = if has_text {
                    sanitize_surrogates(&text_result)
                } else if has_images {
                    "(see attached image)".to_owned()
                } else {
                    String::new()
                };

                let include_id = requires_tool_call_id(&model.id);

                let mut fr = serde_json::json!({
                    "name": tr_msg.tool_name,
                    "response": if tr_msg.is_error {
                        serde_json::json!({ "error": response_value })
                    } else {
                        serde_json::json!({ "output": response_value })
                    }
                });

                if has_images && model_supports_multi_fn_response {
                    fr["parts"] = Value::Array(image_parts.clone());
                }
                if include_id {
                    fr["id"] = Value::String(tr_msg.tool_call_id.clone());
                }

                let function_response_part = serde_json::json!({
                    "functionResponse": fr
                });

                // Cloud Code Assist API requires all function responses to be in a single user turn.
                // Check if the last content is already a user turn with function responses and merge.
                let can_merge = contents
                    .last()
                    .map(|last| {
                        last.get("role").and_then(|r| r.as_str()) == Some("user")
                            && last
                                .get("parts")
                                .and_then(|p| p.as_array())
                                .map(|arr| arr.iter().any(|p| p.get("functionResponse").is_some()))
                                .unwrap_or(false)
                    })
                    .unwrap_or(false);

                if can_merge {
                    if let Some(last) = contents.last_mut() {
                        if let Some(parts) = last.get_mut("parts").and_then(|p| p.as_array_mut()) {
                            parts.push(function_response_part);
                        }
                    }
                } else {
                    contents.push(serde_json::json!({
                        "role": "user",
                        "parts": [function_response_part]
                    }));
                }

                // For Gemini < 3, add images in a separate user message
                if has_images && !model_supports_multi_fn_response {
                    let mut extra_parts: Vec<Value> = vec![serde_json::json!({
                        "text": "Tool result image:"
                    })];
                    extra_parts.extend(image_parts);
                    contents.push(serde_json::json!({
                        "role": "user",
                        "parts": extra_parts
                    }));
                }
            }
        }
    }

    contents
}

// ---------------------------------------------------------------------------
// Tool conversion: hamr Tool[] → Google functionDeclarations[]
// ---------------------------------------------------------------------------

const JSON_SCHEMA_META_DECLARATIONS: &[&str] = &[
    "$schema",
    "$id",
    "$anchor",
    "$dynamicAnchor",
    "$vocabulary",
    "$comment",
    "$defs",
    "definitions", // pre-draft-2019-09 equivalent of $defs
];

/// Strip meta-declarations from a JSON Schema object.
fn sanitize_for_open_api(schema: &Value) -> Value {
    match schema {
        Value::Object(map) => {
            let mut result = serde_json::Map::new();
            for (key, value) in map {
                if JSON_SCHEMA_META_DECLARATIONS.contains(&key.as_str()) {
                    continue;
                }
                result.insert(key.clone(), sanitize_for_open_api(value));
            }
            Value::Object(result)
        }
        Value::Array(arr) => Value::Array(arr.iter().map(sanitize_for_open_api).collect()),
        other => other.clone(),
    }
}

/// Convert hamr `Tool` definitions to Google function declarations format.
///
/// By default uses `parametersJsonSchema` which supports full JSON Schema (including
/// `anyOf`, `oneOf`, `const`, etc.). Set `use_parameters` to true to use the legacy
/// `parameters` field instead (OpenAPI 3.03 Schema). This is needed for Cloud Code
/// Assist with Claude models, where the API translates `parameters` into Anthropic's
/// `input_schema`.
pub fn convert_tools(tools: &[Tool], use_parameters: bool) -> Option<Vec<Value>> {
    if tools.is_empty() {
        return None;
    }

    let function_declarations: Vec<Value> = tools
        .iter()
        .map(|tool| {
            let mut decl = serde_json::json!({
                "name": tool.name,
                "description": tool.description,
            });
            if use_parameters {
                decl["parameters"] = sanitize_for_open_api(&tool.parameters);
            } else {
                decl["parametersJsonSchema"] = tool.parameters.clone();
            }
            decl
        })
        .collect();

    Some(vec![serde_json::json!({
        "functionDeclarations": function_declarations
    })])
}

// ---------------------------------------------------------------------------
// Tool choice mapping
// ---------------------------------------------------------------------------

/// Map a tool choice string to a Google `FunctionCallingConfigMode`.
///
/// Returns the mode as a JSON string value suitable for the `mode` field in
/// `toolConfig.functionCallingConfig`.
pub fn map_tool_choice(choice: &str) -> &'static str {
    match choice {
        "auto" => "AUTO",
        "none" => "NONE",
        "any" => "ANY",
        _ => "AUTO",
    }
}

/// Build a `toolConfig` JSON value from a tool choice string.
pub fn build_tool_config(tool_choice: Option<&str>) -> Option<Value> {
    tool_choice.map(|choice| {
        serde_json::json!({
            "functionCallingConfig": {
                "mode": map_tool_choice(choice)
            }
        })
    })
}

// ---------------------------------------------------------------------------
// Stop reason mapping: Google FinishReason → hamr StopReason
// ---------------------------------------------------------------------------

/// Map a Google GenAI `FinishReason` string to a hamr `StopReason`.
///
/// Covers all `FinishReason` enum values from the Google GenAI SDK.
pub fn map_stop_reason(reason: &str) -> StopReason {
    match reason {
        "STOP" => StopReason::Stop,
        "MAX_TOKENS" => StopReason::Length,
        "BLOCKLIST"
        | "PROHIBITED_CONTENT"
        | "SPII"
        | "SAFETY"
        | "IMAGE_SAFETY"
        | "IMAGE_PROHIBITED_CONTENT"
        | "IMAGE_RECITATION"
        | "IMAGE_OTHER"
        | "RECITATION"
        | "FINISH_REASON_UNSPECIFIED"
        | "OTHER"
        | "LANGUAGE"
        | "MALFORMED_FUNCTION_CALL"
        | "UNEXPECTED_TOOL_CALL"
        | "NO_IMAGE" => StopReason::Error,
        _ => StopReason::Error,
    }
}

/// Map a raw string finish reason to a hamr `StopReason` (for raw API responses).
pub fn map_stop_reason_string(reason: &str) -> StopReason {
    match reason {
        "STOP" => StopReason::Stop,
        "MAX_TOKENS" => StopReason::Length,
        _ => StopReason::Error,
    }
}

// ---------------------------------------------------------------------------
// Response parsing: Google GenAI SSE response → hamr AssistantMessage fields
// ---------------------------------------------------------------------------

/// Parse a Google GenAI streaming chunk (SSE `data:` payload) and return the parts
/// relevant for building an `AssistantMessage`.
///
/// Returns `None` if the chunk cannot be parsed.
pub fn parse_stream_chunk(chunk_json: &str) -> Option<Value> {
    serde_json::from_str(chunk_json).ok()
}

/// Extract the response ID from a chunk, if present.
pub fn extract_response_id(chunk: &Value) -> Option<&str> {
    chunk.get("responseId").and_then(|v| v.as_str())
}

/// Extract the first candidate from a chunk, if present.
pub fn extract_candidate(chunk: &Value) -> Option<&Value> {
    chunk
        .get("candidates")
        .and_then(|c| c.as_array())
        .and_then(|arr| arr.first())
}

/// Extract parts from a candidate's content, if present.
pub fn extract_parts(candidate: &Value) -> Option<&Vec<Value>> {
    candidate
        .get("content")
        .and_then(|c| c.get("parts"))
        .and_then(|p| p.as_array())
}

/// Extract the finish reason from a candidate, if present.
pub fn extract_finish_reason(candidate: &Value) -> Option<&str> {
    candidate.get("finishReason").and_then(|v| v.as_str())
}

/// Parse usage metadata from a chunk into `(input, output, cache_read, cache_write, total)`.
pub fn parse_usage_metadata(chunk: &Value) -> Option<(u64, u64, u64, u64, u64)> {
    let meta = chunk.get("usageMetadata")?;
    let prompt = meta
        .get("promptTokenCount")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    let cached = meta
        .get("cachedContentTokenCount")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    let candidates = meta
        .get("candidatesTokenCount")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    let thoughts = meta
        .get("thoughtsTokenCount")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    let total = meta
        .get("totalTokenCount")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);

    let input = prompt.saturating_sub(cached);
    let output = candidates.saturating_add(thoughts);

    Some((input, output, cached, 0, total))
}

/// Build a `TextContent` from a Google part's text and optional thought signature.
pub fn part_to_text_content(part: &Value) -> Option<(TextContent, bool)> {
    let text = part.get("text").and_then(|t| t.as_str())?;
    let is_thinking = is_thinking_part(part);
    let sig = part
        .get("thoughtSignature")
        .and_then(|s| s.as_str())
        .map(|s| s.to_owned());

    let content = if is_thinking {
        // Will be converted to ThinkingContent by the caller
        TextContent {
            text: text.to_owned(),
            text_signature: sig,
        }
    } else {
        TextContent {
            text: text.to_owned(),
            text_signature: sig,
        }
    };

    Some((content, is_thinking))
}

/// Build a `ToolCall` from a Google functionCall part.
pub fn part_to_tool_call(part: &Value, tool_call_counter: &mut u64) -> Option<ToolCall> {
    let fc = part.get("functionCall")?;
    let name = fc
        .get("name")
        .and_then(|n| n.as_str())
        .unwrap_or("")
        .to_owned();
    let args = fc
        .get("args")
        .cloned()
        .unwrap_or(Value::Object(Default::default()));

    // Generate unique ID if not provided or if it's a duplicate
    let provided_id = fc.get("id").and_then(|id| id.as_str());
    let id = match provided_id {
        Some(pid) if !pid.is_empty() => pid.to_owned(),
        _ => {
            *tool_call_counter += 1;
            let ts = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis();
            format!("{}_{}_{}", name, ts, tool_call_counter)
        }
    };

    let thought_signature = part
        .get("thoughtSignature")
        .and_then(|s| s.as_str())
        .map(|s| s.to_owned());

    Some(ToolCall {
        id,
        name,
        arguments: args,
        thought_signature,
    })
}

// ---------------------------------------------------------------------------
// Thinking config builder
// ---------------------------------------------------------------------------

/// Build a Google `thinkingConfig` JSON value for a request.
pub fn build_thinking_config(
    enabled: bool,
    model_reasoning: bool,
    thinking_level: Option<GoogleThinkingLevel>,
    budget_tokens: Option<i32>,
) -> Option<Value> {
    if !model_reasoning {
        return None;
    }

    if enabled {
        let mut config = serde_json::json!({
            "includeThoughts": true
        });
        if let Some(level) = thinking_level {
            config["thinkingLevel"] = Value::String(level.as_str().to_owned());
        } else if let Some(budget) = budget_tokens {
            config["thinkingBudget"] = Value::Number(budget.into());
        }
        Some(config)
    } else {
        None
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{
        AssistantMessage, ImageContent, MessageRole, ToolResultMessage, Usage, UsageCost,
        UserMessage,
    };
    use chrono;

    #[test]
    fn test_is_valid_thought_signature_valid() {
        // Valid base64 for "hello" = "aGVsbG8="
        assert!(is_valid_thought_signature(Some("aGVsbG8=")));
    }

    #[test]
    fn test_is_valid_thought_signature_invalid_length() {
        // Length not multiple of 4
        assert!(!is_valid_thought_signature(Some("abc")));
    }

    #[test]
    fn test_is_valid_thought_signature_empty() {
        assert!(!is_valid_thought_signature(Some("")));
        assert!(!is_valid_thought_signature(None));
    }

    #[test]
    fn test_is_thinking_part_true() {
        let part = serde_json::json!({ "thought": true, "text": "reasoning" });
        assert!(is_thinking_part(&part));
    }

    #[test]
    fn test_is_thinking_part_false() {
        let part = serde_json::json!({ "text": "response" });
        assert!(!is_thinking_part(&part));
    }

    #[test]
    fn test_is_thinking_part_missing() {
        let part = serde_json::json!({ "thought": false, "text": "response" });
        assert!(!is_thinking_part(&part));
    }

    #[test]
    fn test_retain_thought_signature_keeps_existing() {
        let existing = Some("aGVsbG8=");
        let result = retain_thought_signature(existing, None);
        assert_eq!(result.as_deref(), Some("aGVsbG8="));
    }

    #[test]
    fn test_retain_thought_signature_replaces_with_new() {
        let result = retain_thought_signature(Some("old"), Some("new"));
        assert_eq!(result.as_deref(), Some("new"));
    }

    #[test]
    fn test_retain_thought_signature_rejects_empty_incoming() {
        let result = retain_thought_signature(Some("aGVsbG8="), Some(""));
        assert_eq!(result.as_deref(), Some("aGVsbG8="));
    }

    #[test]
    fn test_requires_tool_call_id() {
        assert!(requires_tool_call_id("claude-3-sonnet"));
        assert!(requires_tool_call_id("gpt-oss-4"));
        assert!(!requires_tool_call_id("gemini-2.0-flash"));
        assert!(!requires_tool_call_id("gemini-pro"));
    }

    #[test]
    fn test_map_stop_reason_stop() {
        assert_eq!(map_stop_reason("STOP"), StopReason::Stop);
    }

    #[test]
    fn test_map_stop_reason_max_tokens() {
        assert_eq!(map_stop_reason("MAX_TOKENS"), StopReason::Length);
    }

    #[test]
    fn test_map_stop_reason_safety() {
        assert_eq!(map_stop_reason("SAFETY"), StopReason::Error);
        assert_eq!(map_stop_reason("BLOCKLIST"), StopReason::Error);
        assert_eq!(map_stop_reason("PROHIBITED_CONTENT"), StopReason::Error);
    }

    #[test]
    fn test_map_stop_reason_unknown() {
        assert_eq!(map_stop_reason("UNKNOWN_REASON"), StopReason::Error);
    }

    #[test]
    fn test_map_tool_choice() {
        assert_eq!(map_tool_choice("auto"), "AUTO");
        assert_eq!(map_tool_choice("none"), "NONE");
        assert_eq!(map_tool_choice("any"), "ANY");
        assert_eq!(map_tool_choice("unknown"), "AUTO");
    }

    #[test]
    fn test_convert_tools_empty() {
        assert!(convert_tools(&[], false).is_none());
    }

    #[test]
    fn test_convert_tools_basic() {
        let tools = vec![Tool {
            name: "get_weather".into(),
            description: "Get the weather".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "city": { "type": "string" }
                }
            }),
        }];
        let result = convert_tools(&tools, false);
        assert!(result.is_some());
        let arr = result.unwrap();
        assert_eq!(arr.len(), 1);
        let tool_arr = arr[0].as_object().unwrap();
        let fds = tool_arr["functionDeclarations"].as_array().unwrap();
        assert_eq!(fds.len(), 1);
        assert_eq!(fds[0]["name"], "get_weather");
        assert!(fds[0].get("parametersJsonSchema").is_some());
        assert!(fds[0].get("parameters").is_none());
    }

    #[test]
    fn test_convert_tools_use_parameters() {
        let tools = vec![Tool {
            name: "get_weather".into(),
            description: "Get the weather".into(),
            parameters: serde_json::json!({
                "type": "object",
                "$schema": "http://json-schema.org/draft-07/schema#",
                "$defs": {},
                "properties": {
                    "city": { "type": "string" }
                }
            }),
        }];
        let result = convert_tools(&tools, true);
        let arr = result.unwrap();
        let fds = arr[0]["functionDeclarations"].as_array().unwrap();
        // Should use `parameters` field (not `parametersJsonSchema`)
        assert!(fds[0].get("parameters").is_some());
        assert!(fds[0].get("parametersJsonSchema").is_none());
        // Meta declarations should be stripped
        let params = &fds[0]["parameters"];
        assert!(params.get("$schema").is_none());
        assert!(params.get("$defs").is_none());
        assert!(params.get("properties").is_some());
    }

    #[test]
    fn test_convert_tools_recursively_strips_meta_keys() {
        let tools = vec![Tool {
            name: "deep_tool".into(),
            description: "Tool with nested meta keys".into(),
            parameters: serde_json::json!({
                "$schema": "http://json-schema.org/draft-07/schema#",
                "type": "object",
                "properties": {
                    "deep": {
                        "$schema": "http://json-schema.org/draft-07/schema#",
                        "$id": "urn:nested",
                        "type": "string",
                    }
                }
            }),
        }];
        let result = convert_tools(&tools, true);
        let unwrapped = result.unwrap();
        let fds = unwrapped[0]["functionDeclarations"].as_array().unwrap();
        let params = &fds[0]["parameters"];
        assert!(params.get("$schema").is_none());
        let deep_prop = &params["properties"]["deep"];
        assert!(deep_prop.get("$schema").is_none());
        assert!(deep_prop.get("$id").is_none());
        assert_eq!(deep_prop["type"], "string");
    }

    #[test]
    fn test_convert_tools_preserves_ref_while_stripping_meta() {
        let tools = vec![Tool {
            name: "ref_tool".into(),
            description: "Tool with $ref".into(),
            parameters: serde_json::json!({
                "$schema": "http://json-schema.org/draft-07/schema#",
                "type": "object",
                "properties": {
                    "refProp": {
                        "$ref": "#/$defs/someDef",
                        "type": "string",
                    }
                }
            }),
        }];
        let result = convert_tools(&tools, true);
        let unwrapped = result.unwrap();
        let fds = unwrapped[0]["functionDeclarations"].as_array().unwrap();
        let ref_prop = &fds[0]["parameters"]["properties"]["refProp"];
        assert_eq!(ref_prop["$ref"], "#/$defs/someDef");
        assert_eq!(ref_prop["type"], "string");
    }

    #[test]
    fn test_convert_tools_does_not_mutate_original() {
        let original = serde_json::json!({
            "$schema": "http://json-schema.org/draft-07/schema#",
            "type": "object",
            "properties": {
                "command": { "type": "string" }
            }
        });
        let tools = vec![Tool {
            name: "test_tool".into(),
            description: "Test".into(),
            parameters: original.clone(),
        }];
        let _ = convert_tools(&tools, true);
        // Original must be unchanged
        assert!(original.get("$schema").is_some());
        assert!(original.get("type").is_some());
    }

    #[test]
    fn test_convert_tools_preserves_schema_in_parameters_json_schema() {
        let tools = vec![Tool {
            name: "test_tool".into(),
            description: "Test".into(),
            parameters: serde_json::json!({
                "$schema": "http://json-schema.org/draft-07/schema#",
                "type": "object",
                "properties": {
                    "command": { "type": "string" }
                }
            }),
        }];
        let result = convert_tools(&tools, false);
        let unwrapped = result.unwrap();
        let fds = unwrapped[0]["functionDeclarations"].as_array().unwrap();
        let pjs = &fds[0]["parametersJsonSchema"];
        assert!(pjs.get("$schema").is_some());
        assert_eq!(pjs["$schema"], "http://json-schema.org/draft-07/schema#");
        assert_eq!(pjs["properties"]["command"]["type"], "string");
    }

    #[test]
    fn test_convert_tools_handles_no_schema_gracefully() {
        let tools = vec![Tool {
            name: "test_tool".into(),
            description: "Test".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string" }
                }
            }),
        }];
        let result = convert_tools(&tools, true);
        let unwrapped = result.unwrap();
        let fds = unwrapped[0]["functionDeclarations"].as_array().unwrap();
        let params = &fds[0]["parameters"];
        assert_eq!(params["properties"]["path"]["type"], "string");
    }

    #[test]
    fn test_supports_multimodal_function_response() {
        assert!(supports_multimodal_function_response("gemini-3-pro"));
        assert!(supports_multimodal_function_response("gemini-3-flash"));
        assert!(!supports_multimodal_function_response("gemini-2.0-flash"));
        assert!(!supports_multimodal_function_response("gemini-2.5-pro"));
    }

    #[test]
    fn test_google_thinking_level_as_str() {
        assert_eq!(GoogleThinkingLevel::Minimal.as_str(), "MINIMAL");
        assert_eq!(GoogleThinkingLevel::Low.as_str(), "LOW");
        assert_eq!(GoogleThinkingLevel::Medium.as_str(), "MEDIUM");
        assert_eq!(GoogleThinkingLevel::High.as_str(), "HIGH");
        assert_eq!(
            GoogleThinkingLevel::Unspecified.as_str(),
            "THINKING_LEVEL_UNSPECIFIED"
        );
    }

    #[test]
    fn test_build_thinking_config() {
        let config = build_thinking_config(true, true, Some(GoogleThinkingLevel::High), None);
        let config = config.unwrap();
        assert_eq!(config["includeThoughts"], true);
        assert_eq!(config["thinkingLevel"], "HIGH");

        let config = build_thinking_config(true, true, None, Some(8192));
        let config = config.unwrap();
        assert_eq!(config["thinkingBudget"], 8192);

        let config = build_thinking_config(false, true, None, None);
        assert!(config.is_none());
    }

    // ------------------------------------------------------------------
    // convert_messages: Gemini 3 unsigned tool calls (no thoughtSignature)
    // ------------------------------------------------------------------

    fn gemini_model(id: &str) -> Model {
        Model {
            id: id.into(),
            name: id.into(),
            api: crate::types::Api::GoogleGenerativeAi,
            provider: "google".into(),
            base_url: "https://example.com".into(),
            reasoning: true,
            thinking_level_map: None,
            input: vec![InputModality::Text],
            cost: crate::types::ModelCost {
                input: 0.0,
                output: 0.0,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 128000,
            max_tokens: 8192,
            headers: None,
            compat: None,
        }
    }

    fn simple_context(model_id: &str, provider: &str, thought_sig: Option<&str>) -> Context {
        let now = chrono::Utc::now();
        let mut tool_calls = vec![
            AssistantContentBlock::ToolCall(ToolCall {
                id: "call_1".into(),
                name: "bash".into(),
                arguments: serde_json::json!({"command": "echo hi"}),
                thought_signature: None,
            }),
            AssistantContentBlock::ToolCall(ToolCall {
                id: "call_2".into(),
                name: "bash".into(),
                arguments: serde_json::json!({"command": "ls -la"}),
                thought_signature: None,
            }),
        ];
        if let Some(sig) = thought_sig {
            if let AssistantContentBlock::ToolCall(tc) = &mut tool_calls[0] {
                tc.thought_signature = Some(sig.to_owned());
            }
        }
        Context {
            system_prompt: None,
            messages: vec![
                Message::User(UserMessage {
                    role: MessageRole::User,
                    content: vec![MessageContent::Text(TextContent {
                        text: "Hi".into(),
                        text_signature: None,
                    })],
                    timestamp: now,
                }),
                Message::Assistant(AssistantMessage {
                    role: MessageRole::Assistant,
                    content: tool_calls,
                    api: "google-generative-ai".into(),
                    provider: provider.into(),
                    model: model_id.into(),
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
                    stop_reason: StopReason::ToolUse,
                    error_message: None,
                    diagnostics: None,
                    timestamp: now,
                }),
            ],
            tools: vec![],
        }
    }

    #[test]
    fn gemini3_unsigned_tool_call_no_thought_signature_google_genai() {
        let model = gemini_model("gemini-3-pro-preview");
        // Different provider/model on assistant message → signature should be stripped
        let ctx = simple_context("other-model", "google", None);
        let contents = convert_messages(&model, &ctx);
        let model_turn = contents.iter().find(|c| c["role"] == "model").unwrap();
        let fc_parts: Vec<&Value> = model_turn["parts"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|p| p.get("functionCall").is_some())
            .collect();
        assert_eq!(fc_parts.len(), 2);
        assert!(fc_parts[0].get("thoughtSignature").is_none());
        assert!(fc_parts[1].get("thoughtSignature").is_none());
        // No "skip_thought_signature_validator" text
        let json_str = serde_json::to_string(model_turn).unwrap();
        assert!(!json_str.contains("skip_thought_signature_validator"));
    }

    #[test]
    fn gemini3_unsigned_tool_call_preserves_valid_same_provider() {
        let model = gemini_model("gemini-3-pro-preview");
        let ctx = simple_context(
            "gemini-3-pro-preview",
            "google",
            Some("AAAAAAAAAAAAAAAAAAAAAA=="),
        );
        let contents = convert_messages(&model, &ctx);
        let model_turn = contents.iter().find(|c| c["role"] == "model").unwrap();
        let fc_parts: Vec<&Value> = model_turn["parts"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|p| p.get("functionCall").is_some())
            .collect();
        assert_eq!(fc_parts.len(), 2);
        assert_eq!(fc_parts[0]["thoughtSignature"], "AAAAAAAAAAAAAAAAAAAAAA==");
        assert!(fc_parts[1].get("thoughtSignature").is_none());
    }

    #[test]
    fn gemini3_unsigned_tool_call_no_signature_for_older_models() {
        let model = gemini_model("gemini-2.5-flash");
        let ctx = simple_context("other-model", "google", Some("AAAAAAAAAAAAAAAAAAAAAA=="));
        let contents = convert_messages(&model, &ctx);
        let model_turn = contents.iter().find(|c| c["role"] == "model").unwrap();
        let fc_parts: Vec<&Value> = model_turn["parts"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|p| p.get("functionCall").is_some())
            .collect();
        // Different provider/model and not gemini-3 → no signature
        assert_eq!(fc_parts.len(), 2);
        assert!(fc_parts[0].get("thoughtSignature").is_none());
    }

    // ------------------------------------------------------------------
    // convert_messages: image tool result routing
    // ------------------------------------------------------------------

    fn image_routing_model(id: &str) -> Model {
        Model {
            id: id.into(),
            name: id.into(),
            api: crate::types::Api::GoogleGenerativeAi,
            provider: "google".into(),
            base_url: "https://example.com".into(),
            reasoning: true,
            thinking_level_map: None,
            input: vec![InputModality::Text, InputModality::Image],
            cost: crate::types::ModelCost {
                input: 0.0,
                output: 0.0,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 128000,
            max_tokens: 8192,
            headers: None,
            compat: None,
        }
    }

    fn image_routing_context(model_id: &str) -> Context {
        let now = chrono::Utc::now();
        Context {
            system_prompt: None,
            messages: vec![
                Message::User(UserMessage {
                    role: MessageRole::User,
                    content: vec![MessageContent::Text(TextContent {
                        text: "read the files".into(),
                        text_signature: None,
                    })],
                    timestamp: now,
                }),
                Message::Assistant(AssistantMessage {
                    role: MessageRole::Assistant,
                    content: vec![
                        AssistantContentBlock::ToolCall(ToolCall {
                            id: "call_a".into(),
                            name: "read".into(),
                            arguments: serde_json::json!({"path": "a.txt"}),
                            thought_signature: None,
                        }),
                        AssistantContentBlock::ToolCall(ToolCall {
                            id: "call_img".into(),
                            name: "read".into(),
                            arguments: serde_json::json!({"path": "image.png"}),
                            thought_signature: None,
                        }),
                        AssistantContentBlock::ToolCall(ToolCall {
                            id: "call_b".into(),
                            name: "read".into(),
                            arguments: serde_json::json!({"path": "b.txt"}),
                            thought_signature: None,
                        }),
                    ],
                    api: "google-generative-ai".into(),
                    provider: "google".into(),
                    model: model_id.into(),
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
                    stop_reason: StopReason::ToolUse,
                    error_message: None,
                    diagnostics: None,
                    timestamp: now,
                }),
                Message::ToolResult(ToolResultMessage {
                    role: MessageRole::ToolResult,
                    tool_call_id: "call_a".into(),
                    tool_name: "read".into(),
                    content: vec![MessageContent::Text(TextContent {
                        text: "alpha text".into(),
                        text_signature: None,
                    })],
                    details: None,
                    is_error: false,
                    timestamp: now,
                }),
                Message::ToolResult(ToolResultMessage {
                    role: MessageRole::ToolResult,
                    tool_call_id: "call_img".into(),
                    tool_name: "read".into(),
                    content: vec![MessageContent::Image(ImageContent {
                        mime_type: "image/png".into(),
                        data: "abc".into(),
                    })],
                    details: None,
                    is_error: false,
                    timestamp: now,
                }),
                Message::ToolResult(ToolResultMessage {
                    role: MessageRole::ToolResult,
                    tool_call_id: "call_b".into(),
                    tool_name: "read".into(),
                    content: vec![MessageContent::Text(TextContent {
                        text: "beta text".into(),
                        text_signature: None,
                    })],
                    details: None,
                    is_error: false,
                    timestamp: now,
                }),
            ],
            tools: vec![],
        }
    }

    #[test]
    fn image_tool_result_gemini2_separate_synthetic_image_turn() {
        let model = image_routing_model("gemini-2.5-flash");
        let ctx = image_routing_context("gemini-2.5-flash");
        let contents = convert_messages(&model, &ctx);
        // Expected turn order:
        // 0: user "read the files"
        // 1: model (tool calls)
        // 2: user (function response call_a)
        // 3: user (separate text+image turn for image)
        // 4: user (function response call_b)
        assert_eq!(contents.len(), 5);
        // Turn 2: all parts are functionResponse
        let turn2_parts = contents[2]["parts"].as_array().unwrap();
        assert!(
            turn2_parts
                .iter()
                .all(|p| p.get("functionResponse").is_some())
        );
        // Turn 3: text "Tool result image:" followed by inlineData
        assert_eq!(contents[3]["parts"][0]["text"], "Tool result image:");
        assert!(contents[3]["parts"][1].get("inlineData").is_some());
        // Turn 4: functionResponse
        assert!(contents[4]["parts"][0].get("functionResponse").is_some());
    }

    #[test]
    fn image_tool_result_gemini3_nests_image_inline() {
        let model = image_routing_model("gemini-3-pro-preview");
        let ctx = image_routing_context("gemini-3-pro-preview");
        let contents = convert_messages(&model, &ctx);
        // Expected: 3 turns (user, model, user with 3 function responses) — gemini 3 nests images inline
        assert_eq!(contents.len(), 3);
        let turn2_parts = contents[2]["parts"].as_array().unwrap();
        assert_eq!(turn2_parts.len(), 3);
        // The middle part (index 1 = call_img) should have functionResponse with inline image parts
        let fr = &turn2_parts[1]["functionResponse"];
        assert!(fr.is_object());
        let fr_parts = fr.get("parts").and_then(|p| p.as_array());
        assert!(fr_parts.is_some());
        assert_eq!(fr_parts.unwrap().len(), 1);
        assert!(fr_parts.unwrap()[0].get("inlineData").is_some());
    }

    // ------------------------------------------------------------------
    // Normalize tool call ID
    // ------------------------------------------------------------------

    #[test]
    fn test_normalize_tool_call_id_claude_cleans_non_alphanumeric() {
        // claude models require tool call id normalization
        let cleaned = normalize_tool_call_id("claude-3-sonnet", "abc-def_ghi!jkl@mno");
        assert_eq!(cleaned, "abc-def_ghijklmno");
    }

    #[test]
    fn test_normalize_tool_call_id_claude_truncates_to_64() {
        let long = "a".repeat(100);
        let cleaned = normalize_tool_call_id("claude-3-sonnet", &long);
        assert_eq!(cleaned.len(), 64);
    }

    #[test]
    fn test_normalize_tool_call_id_gemini_passthrough() {
        let original = "abc-def_ghi!jkl@mno";
        let cleaned = normalize_tool_call_id("gemini-2.0-flash", original);
        // Gemini doesn't require normalization → passthrough
        assert_eq!(cleaned, original);
    }

    // ------------------------------------------------------------------
    // parse_usage_metadata
    // ------------------------------------------------------------------

    #[test]
    fn test_parse_usage_metadata_basic() {
        let chunk = serde_json::json!({
            "usageMetadata": {
                "promptTokenCount": 10,
                "candidatesTokenCount": 20,
                "totalTokenCount": 30
            }
        });
        let result = parse_usage_metadata(&chunk);
        assert!(result.is_some());
        let (input, output, cache_read, cache_write, total) = result.unwrap();
        assert_eq!(input, 10);
        assert_eq!(output, 20);
        assert_eq!(cache_read, 0);
        assert_eq!(cache_write, 0);
        assert_eq!(total, 30);
    }

    #[test]
    fn test_parse_usage_metadata_with_cached() {
        let chunk = serde_json::json!({
            "usageMetadata": {
                "promptTokenCount": 50,
                "cachedContentTokenCount": 10,
                "candidatesTokenCount": 20,
                "thoughtsTokenCount": 5,
                "totalTokenCount": 65
            }
        });
        let result = parse_usage_metadata(&chunk);
        let (input, output, _, _, total) = result.unwrap();
        // input = prompt - cached = 40
        assert_eq!(input, 40);
        // output = candidates + thoughts = 25
        assert_eq!(output, 25);
        assert_eq!(total, 65);
    }

    #[test]
    fn test_parse_usage_metadata_missing() {
        assert!(parse_usage_metadata(&serde_json::json!({})).is_none());
    }

    // ------------------------------------------------------------------
    // extract_response_id, extract_candidate, etc.
    // ------------------------------------------------------------------

    #[test]
    fn test_extract_response_id_present() {
        let chunk = serde_json::json!({ "responseId": "abc-123" });
        assert_eq!(extract_response_id(&chunk), Some("abc-123"));
    }

    #[test]
    fn test_extract_response_id_missing() {
        let chunk = serde_json::json!({ "foo": "bar" });
        assert_eq!(extract_response_id(&chunk), None);
    }

    #[test]
    fn test_extract_candidate_present() {
        let chunk = serde_json::json!({
            "candidates": [{
                "content": { "parts": [{ "text": "hi" }] },
                "finishReason": "STOP"
            }]
        });
        let candidate = extract_candidate(&chunk);
        assert!(candidate.is_some());
        assert_eq!(candidate.unwrap()["finishReason"], "STOP");
    }

    #[test]
    fn test_extract_candidate_missing() {
        assert!(extract_candidate(&serde_json::json!({})).is_none());
    }

    #[test]
    fn test_extract_parts_present() {
        let candidate = serde_json::json!({
            "content": { "parts": [{ "text": "hi" }] }
        });
        let parts = extract_parts(&candidate);
        assert!(parts.is_some());
        assert_eq!(parts.unwrap().len(), 1);
    }

    #[test]
    fn test_extract_parts_missing() {
        let candidate = serde_json::json!({ "content": {} });
        assert!(extract_parts(&candidate).is_none());
    }

    #[test]
    fn test_extract_finish_reason() {
        let candidate = serde_json::json!({ "finishReason": "MAX_TOKENS" });
        assert_eq!(extract_finish_reason(&candidate), Some("MAX_TOKENS"));
        assert!(extract_finish_reason(&serde_json::json!({})).is_none());
    }

    // ------------------------------------------------------------------
    // build_tool_config
    // ------------------------------------------------------------------

    #[test]
    fn test_build_tool_config_auto() {
        let config = build_tool_config(Some("auto"));
        let config = config.unwrap();
        assert_eq!(config["functionCallingConfig"]["mode"], "AUTO");
    }

    #[test]
    fn test_build_tool_config_none() {
        let config = build_tool_config(Some("none"));
        assert_eq!(config.unwrap()["functionCallingConfig"]["mode"], "NONE");
    }

    #[test]
    fn test_build_tool_config_none_input() {
        assert!(build_tool_config(None).is_none());
    }

    // ------------------------------------------------------------------
    // resolve_thought_signature
    // ------------------------------------------------------------------

    #[test]
    fn test_resolve_thought_signature_same_provider_valid() {
        assert_eq!(
            resolve_thought_signature(true, Some("aGVsbG8=")),
            Some("aGVsbG8=".to_string())
        );
    }

    #[test]
    fn test_resolve_thought_signature_different_provider() {
        assert!(resolve_thought_signature(false, Some("aGVsbG8=")).is_none());
    }

    #[test]
    fn test_resolve_thought_signature_invalid_base64() {
        assert!(resolve_thought_signature(true, Some("!!!")).is_none());
    }

    #[test]
    fn test_resolve_thought_signature_none_input() {
        assert!(resolve_thought_signature(true, None).is_none());
    }

    // ------------------------------------------------------------------
    // part_to_text_content
    // ------------------------------------------------------------------

    #[test]
    fn test_part_to_text_content_plain() {
        let part = serde_json::json!({ "text": "hello" });
        let result = part_to_text_content(&part);
        assert!(result.is_some());
        let (tc, is_thinking) = result.unwrap();
        assert!(!is_thinking);
        assert_eq!(tc.text, "hello");
        assert!(tc.text_signature.is_none());
    }

    #[test]
    fn test_part_to_text_content_thinking() {
        let part = serde_json::json!({ "thought": true, "text": "reasoning" });
        let result = part_to_text_content(&part);
        assert!(result.is_some());
        let (_, is_thinking) = result.unwrap();
        assert!(is_thinking);
    }

    #[test]
    fn test_part_to_text_content_with_signature() {
        let part = serde_json::json!({
            "text": "hello",
            "thoughtSignature": "YWJj"
        });
        let (tc, _) = part_to_text_content(&part).unwrap();
        assert_eq!(tc.text_signature.as_deref(), Some("YWJj"));
    }

    #[test]
    fn test_part_to_text_content_missing_text() {
        assert!(part_to_text_content(&serde_json::json!({ "foo": "bar" })).is_none());
    }

    // ------------------------------------------------------------------
    // map_stop_reason_string
    // ------------------------------------------------------------------

    #[test]
    fn test_map_stop_reason_string_stop() {
        assert_eq!(map_stop_reason_string("STOP"), StopReason::Stop);
    }

    #[test]
    fn test_map_stop_reason_string_length() {
        assert_eq!(map_stop_reason_string("MAX_TOKENS"), StopReason::Length);
    }

    #[test]
    fn test_map_stop_reason_string_error() {
        assert_eq!(map_stop_reason_string("SAFETY"), StopReason::Error);
    }
}
