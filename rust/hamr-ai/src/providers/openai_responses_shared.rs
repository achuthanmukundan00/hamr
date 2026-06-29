//! Port of `packages/ai/src/providers/openai-responses-shared.ts`.
//!
//! Shared helpers used by the `openai-responses`, `azure-openai-responses`, and
//! `openai-codex-responses` providers: request payload (message + tool)
//! construction, response-item conversion / reasoning replay, and the streaming
//! event handler (`process_responses_stream`).
//!
//! Because the Rust workspace has no OpenAI SDK dependency, the OpenAI Responses
//! API wire types (`ResponseInput`, reasoning/message/function-call items, and
//! the SSE `ResponseStreamEvent` union) are modelled here directly as serde
//! types. They serialize/deserialize to exactly the JSON shapes the OpenAI
//! Responses API expects.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashSet;

use crate::models::calculate_cost;
use crate::providers::transform_messages::transform_messages;
use crate::types::AssistantMessageEvent;
use crate::types::{
    AssistantContentBlock, AssistantMessage, Context, InputModality, Model, StopReason,
    TextContent, ThinkingContent, Tool, ToolCall, Usage,
};
use crate::utils::event_stream::AssistantMessageEventStreamSender;
use crate::utils::hash::short_hash;
use crate::utils::json_parse::parse_streaming_json;
use crate::utils::sanitize_unicode::sanitize_surrogates;

// =============================================================================
// OpenAI Responses API wire types (no SDK — modelled directly)
// =============================================================================

/// `TextSignatureV1` phase. Mirrors the TS union `"commentary" | "final_answer"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TextSignaturePhase {
    Commentary,
    FinalAnswer,
}

/// Mirrors the TS `interface TextSignatureV1 { v: 1; id: string; phase?: ... }`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextSignatureV1 {
    pub v: u8,
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phase: Option<TextSignaturePhase>,
}

/// A single input-content part for a user message or function-call output:
/// `input_text` or `input_image`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ResponseInputContent {
    #[serde(rename = "input_text")]
    InputText { text: String },
    #[serde(rename = "input_image")]
    InputImage { detail: String, image_url: String },
}

/// A reasoning summary/content text part within a reasoning item.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseReasoningTextPart {
    #[serde(rename = "type")]
    pub kind: String,
    pub text: String,
}

/// An output-content part within an assistant message item: `output_text` or `refusal`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ResponseOutputContent {
    #[serde(rename = "output_text")]
    OutputText {
        text: String,
        #[serde(default)]
        annotations: Vec<Value>,
    },
    #[serde(rename = "refusal")]
    Refusal { refusal: String },
}

/// One item in a [`ResponseInput`] list — also the shape of items echoed back in
/// `output_item.added`/`output_item.done` SSE events.
///
/// The `type` discriminator selects the variant. Reasoning items are replayed
/// verbatim from their JSON-encoded `thinkingSignature`, so they carry an
/// `#[serde(flatten)] extra` bag to preserve any fields the SDK would include.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ResponseItem {
    /// A reasoning item (encrypted/summary reasoning replay).
    #[serde(rename = "reasoning")]
    Reasoning {
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        summary: Option<Vec<ResponseReasoningTextPart>>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        content: Option<Vec<ResponseReasoningTextPart>>,
        /// Preserve any additional reasoning fields (e.g. `encrypted_content`).
        #[serde(flatten)]
        extra: serde_json::Map<String, Value>,
    },
    /// An input user/developer/system message (string-or-parts content).
    ///
    /// Used when *building* the request payload. `content` is an arbitrary JSON
    /// value because input messages carry either a plain string or a list of
    /// input-content parts.
    #[serde(rename = "message")]
    Message {
        role: String,
        content: Value,
        #[serde(skip_serializing_if = "Option::is_none")]
        status: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        phase: Option<TextSignaturePhase>,
    },
    /// A function (tool) call item.
    #[serde(rename = "function_call")]
    FunctionCall {
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        call_id: String,
        name: String,
        arguments: String,
    },
    /// A function (tool) call output (result) item.
    #[serde(rename = "function_call_output")]
    FunctionCallOutput {
        call_id: String,
        /// Either a plain string or a list of input-content parts.
        output: Value,
    },
}

/// The request `input` list sent to the Responses API.
pub type ResponseInput = Vec<ResponseItem>;

/// `function_call_output.output` part list. Mirrors the SDK
/// `ResponseFunctionCallOutputItemList` (input_text / input_image parts).
pub type ResponseFunctionCallOutputItemList = Vec<ResponseInputContent>;

/// A tool definition in the Responses API shape (`type: "function"`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAITool {
    #[serde(rename = "type")]
    pub kind: String,
    pub name: String,
    pub description: String,
    pub parameters: Value,
    pub strict: bool,
}

// =============================================================================
// Options structs
// =============================================================================

/// Mirrors the TS `interface OpenAIResponsesStreamOptions`.
///
/// **Type debt:** the TS `serviceTier` / `resolveServiceTier` /
/// `applyServiceTierPricing` fields use the SDK `service_tier` type
/// (`string | null | undefined`) and synchronous callbacks. We model the tier as
/// `Option<String>` and the two callbacks as boxed closures. `resolve_service_tier`
/// takes `(response_tier, request_tier)` and returns the resolved tier;
/// `apply_service_tier_pricing` mutates `Usage` in place.
#[derive(Default)]
pub struct OpenAIResponsesStreamOptions {
    pub service_tier: Option<String>,
    #[allow(clippy::type_complexity)]
    pub resolve_service_tier:
        Option<Box<dyn Fn(Option<String>, Option<String>) -> Option<String> + Send + Sync>>,
    #[allow(clippy::type_complexity)]
    pub apply_service_tier_pricing: Option<Box<dyn Fn(&mut Usage, Option<String>) + Send + Sync>>,
}

/// Mirrors the TS `interface ConvertResponsesMessagesOptions`.
#[derive(Debug, Clone, Default)]
pub struct ConvertResponsesMessagesOptions {
    pub include_system_prompt: Option<bool>,
}

/// Mirrors the TS `interface ConvertResponsesToolsOptions`.
#[derive(Debug, Clone, Default)]
pub struct ConvertResponsesToolsOptions {
    /// `Option<Option<bool>>` mirrors the TS `boolean | null` (default `false`).
    pub strict: Option<Option<bool>>,
}

// =============================================================================
// Utilities
// =============================================================================

fn encode_text_signature_v1(id: &str, phase: Option<TextSignaturePhase>) -> String {
    let payload = TextSignatureV1 {
        v: 1,
        id: id.to_string(),
        phase,
    };
    // Serializing a fixed struct of strings cannot fail; fall back to the legacy
    // plain id if it ever did, rather than panicking.
    serde_json::to_string(&payload).unwrap_or_else(|_| id.to_string())
}

struct ParsedTextSignature {
    id: String,
    phase: Option<TextSignaturePhase>,
}

fn parse_text_signature(signature: Option<&str>) -> Option<ParsedTextSignature> {
    let signature = signature?;
    if signature.is_empty() {
        return None;
    }
    if signature.starts_with('{') {
        if let Ok(parsed) = serde_json::from_str::<Value>(signature) {
            let v_is_1 = parsed.get("v").and_then(Value::as_u64) == Some(1);
            let id = parsed.get("id").and_then(Value::as_str);
            if v_is_1 {
                if let Some(id) = id {
                    let phase = match parsed.get("phase").and_then(Value::as_str) {
                        Some("commentary") => Some(TextSignaturePhase::Commentary),
                        Some("final_answer") => Some(TextSignaturePhase::FinalAnswer),
                        _ => None,
                    };
                    return Some(ParsedTextSignature {
                        id: id.to_string(),
                        phase,
                    });
                }
            }
        }
        // Fall through to legacy plain-string handling.
    }
    Some(ParsedTextSignature {
        id: signature.to_string(),
        phase: None,
    })
}

// =============================================================================
// Message conversion
// =============================================================================

/// Convert hamr [`Context`] messages into the Responses API `input` list.
///
/// Mirrors the TS `convertResponsesMessages`.
pub fn convert_responses_messages(
    model: &Model,
    context: &Context,
    allowed_tool_call_providers: &HashSet<String>,
    options: Option<&ConvertResponsesMessagesOptions>,
) -> ResponseInput {
    let mut messages: ResponseInput = Vec::new();

    fn normalize_id_part(part: &str) -> String {
        let sanitized: String = part
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
                    c
                } else {
                    '_'
                }
            })
            .collect();
        let normalized = if sanitized.len() > 64 {
            sanitized[..64].to_string()
        } else {
            sanitized
        };
        normalized.trim_end_matches('_').to_string()
    }

    fn build_foreign_responses_item_id(item_id: &str) -> String {
        let normalized = format!("fc_{}", short_hash(item_id));
        if normalized.len() > 64 {
            normalized[..64].to_string()
        } else {
            normalized
        }
    }

    // Capture for the normalize closure.
    let model_provider = model.provider.clone();
    let model_api = model.api;
    let allowed = allowed_tool_call_providers.clone();

    let normalize_tool_call_id =
        move |id: &str, _target_model: &Model, source: &AssistantMessage| -> String {
            if !allowed.contains(&model_provider) {
                return normalize_id_part(id);
            }
            if !id.contains('|') {
                return normalize_id_part(id);
            }
            let mut parts = id.splitn(2, '|');
            let call_id = parts.next().unwrap_or("");
            let item_id = parts.next().unwrap_or("");
            let normalized_call_id = normalize_id_part(call_id);
            let is_foreign_tool_call =
                source.provider != model_provider || source.api != model_api.to_string();
            let mut normalized_item_id = if is_foreign_tool_call {
                build_foreign_responses_item_id(item_id)
            } else {
                normalize_id_part(item_id)
            };
            // OpenAI Responses API requires item id to start with "fc"
            if !normalized_item_id.starts_with("fc_") {
                normalized_item_id = normalize_id_part(&format!("fc_{normalized_item_id}"));
            }
            format!("{normalized_call_id}|{normalized_item_id}")
        };

    let transformed_messages = transform_messages(
        context.messages.clone(),
        model,
        Some(&normalize_tool_call_id),
    );

    let include_system_prompt = options
        .and_then(|o| o.include_system_prompt)
        .unwrap_or(true);
    if include_system_prompt {
        if let Some(system_prompt) = &context.system_prompt {
            // Read `compat?.supportsDeveloperRole` from model.compat;
            // defaults to `true` when absent, so a reasoning model uses the "developer" role.
            let supports_developer = model
                .compat
                .as_ref()
                .and_then(|c| c.get("supportsDeveloperRole"))
                .and_then(|v| v.as_bool())
                .unwrap_or(true);
            let role = if model.reasoning && supports_developer {
                "developer"
            } else {
                "system"
            };
            messages.push(ResponseItem::Message {
                role: role.to_string(),
                content: Value::String(sanitize_surrogates(system_prompt)),
                status: None,
                id: None,
                phase: None,
            });
        }
    }

    let mut msg_index: usize = 0;
    for msg in &transformed_messages {
        match msg {
            crate::types::Message::User(user_msg) => {
                // User content in Rust is always a Vec<MessageContent>.
                let content: Vec<ResponseInputContent> = user_msg
                    .content
                    .iter()
                    .map(|item| match item {
                        crate::types::MessageContent::Text(t) => ResponseInputContent::InputText {
                            text: sanitize_surrogates(&t.text),
                        },
                        crate::types::MessageContent::Image(img) => {
                            ResponseInputContent::InputImage {
                                detail: "auto".to_string(),
                                image_url: format!("data:{};base64,{}", img.mime_type, img.data),
                            }
                        }
                    })
                    .collect();
                if content.is_empty() {
                    msg_index += 1;
                    continue;
                }
                messages.push(ResponseItem::Message {
                    role: "user".to_string(),
                    content: serde_json::to_value(content).unwrap_or(Value::Null),
                    status: None,
                    id: None,
                    phase: None,
                });
            }
            crate::types::Message::Assistant(assistant_msg) => {
                let mut output: ResponseInput = Vec::new();
                let is_different_model = assistant_msg.model != model.id
                    && assistant_msg.provider == model.provider
                    && assistant_msg.api == model.api.to_string();
                let mut text_block_index: usize = 0;

                for block in &assistant_msg.content {
                    match block {
                        AssistantContentBlock::Thinking(thinking) => {
                            if let Some(sig) = &thinking.thinking_signature {
                                if let Ok(reasoning_item) =
                                    serde_json::from_str::<ResponseItem>(sig)
                                {
                                    output.push(reasoning_item);
                                }
                            }
                        }
                        AssistantContentBlock::Text(text_block) => {
                            let parsed_signature =
                                parse_text_signature(text_block.text_signature.as_deref());
                            let fallback_message_id = if text_block_index == 0 {
                                format!("msg_pi_{msg_index}")
                            } else {
                                format!("msg_pi_{msg_index}_{text_block_index}")
                            };
                            text_block_index += 1;
                            // OpenAI requires id to be max 64 characters.
                            let mut msg_id = parsed_signature.as_ref().map(|p| p.id.clone());
                            match &msg_id {
                                None => msg_id = Some(fallback_message_id),
                                Some(id) if id.len() > 64 => {
                                    msg_id = Some(format!("msg_{}", short_hash(id)));
                                }
                                Some(_) => {}
                            }
                            let phase = parsed_signature.as_ref().and_then(|p| p.phase);
                            let content = vec![ResponseOutputContent::OutputText {
                                text: sanitize_surrogates(&text_block.text),
                                annotations: Vec::new(),
                            }];
                            output.push(ResponseItem::Message {
                                role: "assistant".to_string(),
                                content: serde_json::to_value(content).unwrap_or(Value::Null),
                                status: Some("completed".to_string()),
                                id: msg_id,
                                phase,
                            });
                        }
                        AssistantContentBlock::ToolCall(tool_call) => {
                            let mut parts = tool_call.id.splitn(2, '|');
                            let call_id = parts.next().unwrap_or("").to_string();
                            let item_id_raw = parts.next().map(|s| s.to_string());
                            let mut item_id = item_id_raw;

                            // For different-model messages, set id to undefined to avoid
                            // pairing validation (OpenAI tracks which fc_xxx IDs were paired
                            // with rs_xxx reasoning items).
                            if is_different_model
                                && item_id
                                    .as_ref()
                                    .map(|s| s.starts_with("fc_"))
                                    .unwrap_or(false)
                            {
                                item_id = None;
                            }

                            output.push(ResponseItem::FunctionCall {
                                id: item_id,
                                call_id,
                                name: tool_call.name.clone(),
                                arguments: serde_json::to_string(&tool_call.arguments)
                                    .unwrap_or_else(|_| "{}".to_string()),
                            });
                        }
                    }
                }
                if output.is_empty() {
                    msg_index += 1;
                    continue;
                }
                messages.append(&mut output);
            }
            crate::types::Message::ToolResult(tool_result) => {
                let text_result = tool_result
                    .content
                    .iter()
                    .filter_map(|c| match c {
                        crate::types::MessageContent::Text(t) => Some(t.text.clone()),
                        _ => None,
                    })
                    .collect::<Vec<_>>()
                    .join("\n");
                let has_images = tool_result
                    .content
                    .iter()
                    .any(|c| matches!(c, crate::types::MessageContent::Image(_)));
                let has_text = !text_result.is_empty();
                let call_id = tool_result
                    .tool_call_id
                    .split('|')
                    .next()
                    .unwrap_or("")
                    .to_string();

                let output: Value;
                if has_images && model.input.contains(&InputModality::Image) {
                    let mut content_parts: ResponseFunctionCallOutputItemList = Vec::new();
                    if has_text {
                        content_parts.push(ResponseInputContent::InputText {
                            text: sanitize_surrogates(&text_result),
                        });
                    }
                    for block in &tool_result.content {
                        if let crate::types::MessageContent::Image(img) = block {
                            content_parts.push(ResponseInputContent::InputImage {
                                detail: "auto".to_string(),
                                image_url: format!("data:{};base64,{}", img.mime_type, img.data),
                            });
                        }
                    }
                    output = serde_json::to_value(content_parts).unwrap_or(Value::Null);
                } else {
                    let text = if has_text {
                        text_result
                    } else {
                        "(see attached image)".to_string()
                    };
                    output = Value::String(sanitize_surrogates(&text));
                }

                messages.push(ResponseItem::FunctionCallOutput { call_id, output });
            }
        }
        msg_index += 1;
    }

    messages
}

// =============================================================================
// Tool conversion
// =============================================================================

/// Convert hamr [`Tool`]s into the Responses API tool shape.
///
/// Mirrors the TS `convertResponsesTools`.
pub fn convert_responses_tools(
    tools: &[Tool],
    options: Option<&ConvertResponsesToolsOptions>,
) -> Vec<OpenAITool> {
    // TS: `options?.strict === undefined ? false : options.strict` — note a
    // present `null` stays `null`. We coerce `None`/absent to `false`; a present
    // `Some(None)` (TS `null`) also collapses to `false` since the wire field is
    // a bool.
    let strict = match options.and_then(|o| o.strict) {
        None => false,
        Some(None) => false,
        Some(Some(v)) => v,
    };
    tools
        .iter()
        .map(|tool| OpenAITool {
            kind: "function".to_string(),
            name: tool.name.clone(),
            description: tool.description.clone(),
            // TypeBox already generates JSON Schema; pass through verbatim.
            parameters: tool.parameters.clone(),
            strict,
        })
        .collect()
}

// =============================================================================
// Stream processing
// =============================================================================

/// In-flight block being assembled during streaming. Mirrors the TS
/// `currentBlock` union; the tool-call variant carries a `partial_json` scratch
/// buffer that is stripped before the finalized [`ToolCall`] is replayed.
enum CurrentBlock {
    Thinking,
    Text,
    ToolCall { partial_json: String },
    None,
}

/// Tracks the kind of the in-flight output item. Mirrors the TS `currentItem`
/// type-narrowing (we only need the discriminant + the live message content).
enum CurrentItemKind {
    Reasoning,
    /// Message item; tracks the live content parts so deltas can target the last part.
    Message {
        content: Vec<ResponseOutputContent>,
    },
    FunctionCall,
    None,
}

/// Process an OpenAI Responses SSE event stream, populating `output` and pushing
/// streaming events to `stream`.
///
/// Mirrors the TS `processResponsesStream`. The `stream` parameter is the
/// producer half of the assistant-message event stream (the TS object's `push`
/// method maps to the Rust [`AssistantMessageEventStreamSender`]).
///
/// `events` is an async stream of decoded SSE [`ResponseStreamEvent`]s (the
/// caller is responsible for SSE framing / JSON decoding upstream).
pub async fn process_responses_stream<S>(
    mut openai_stream: S,
    output: &mut AssistantMessage,
    stream: &mut AssistantMessageEventStreamSender,
    model: &Model,
    options: Option<&OpenAIResponsesStreamOptions>,
) -> Result<(), ResponsesStreamError>
where
    S: futures::Stream<Item = Result<ResponseStreamEvent, ResponsesStreamError>> + Unpin,
{
    use futures::StreamExt;

    let mut current_item = CurrentItemKind::None;
    let mut current_block = CurrentBlock::None;

    // Index of the last content block we appended.
    let block_index =
        |output: &AssistantMessage| -> usize { output.content.len().saturating_sub(1) };

    while let Some(event) = openai_stream.next().await {
        let event = event?;
        match event {
            ResponseStreamEvent::ResponseCreated { response } => {
                // TS reads `event.response.id` directly (always present on created).
                output.response_id = response.id;
            }
            ResponseStreamEvent::OutputItemAdded { item } => match item {
                ResponseItem::Reasoning { .. } => {
                    current_item = CurrentItemKind::Reasoning;
                    current_block = CurrentBlock::Thinking;
                    output
                        .content
                        .push(AssistantContentBlock::Thinking(ThinkingContent {
                            thinking: String::new(),
                            thinking_signature: None,
                            redacted: false,
                        }));
                    let idx = block_index(output);
                    stream.push(AssistantMessageEvent::ThinkingStart {
                        content_index: idx,
                        partial: output.clone(),
                    });
                }
                ResponseItem::Message { .. } => {
                    current_item = CurrentItemKind::Message {
                        content: Vec::new(),
                    };
                    current_block = CurrentBlock::Text;
                    output
                        .content
                        .push(AssistantContentBlock::Text(TextContent {
                            text: String::new(),
                            text_signature: None,
                        }));
                    let idx = block_index(output);
                    stream.push(AssistantMessageEvent::TextStart {
                        content_index: idx,
                        partial: output.clone(),
                    });
                }
                ResponseItem::FunctionCall {
                    id,
                    call_id,
                    name,
                    arguments,
                } => {
                    current_item = CurrentItemKind::FunctionCall;
                    let partial_json = arguments;
                    current_block = CurrentBlock::ToolCall {
                        partial_json: partial_json.clone(),
                    };
                    output
                        .content
                        .push(AssistantContentBlock::ToolCall(ToolCall {
                            id: format!("{}|{}", call_id, id.unwrap_or_default()),
                            name,
                            arguments: Value::Object(serde_json::Map::new()),
                            thought_signature: None,
                        }));
                    let idx = block_index(output);
                    stream.push(AssistantMessageEvent::ToolCallStart {
                        content_index: idx,
                        partial: output.clone(),
                    });
                }
                ResponseItem::FunctionCallOutput { .. } => {}
            },
            ResponseStreamEvent::ReasoningSummaryPartAdded { .. } => {
                // The summary parts are accumulated in the live block; we track the
                // thinking text on the content block directly (the SDK kept a
                // mirror on `currentItem.summary` for the *.done path, which we
                // reconstruct from the final item there).
            }
            ResponseStreamEvent::ReasoningSummaryTextDelta { delta } => {
                if matches!(current_item, CurrentItemKind::Reasoning)
                    && matches!(current_block, CurrentBlock::Thinking)
                {
                    append_thinking(output, &delta);
                    let idx = block_index(output);
                    stream.push(AssistantMessageEvent::ThinkingDelta {
                        content_index: idx,
                        delta,
                        partial: output.clone(),
                    });
                }
            }
            ResponseStreamEvent::ReasoningSummaryPartDone => {
                if matches!(current_item, CurrentItemKind::Reasoning)
                    && matches!(current_block, CurrentBlock::Thinking)
                {
                    append_thinking(output, "\n\n");
                    let idx = block_index(output);
                    stream.push(AssistantMessageEvent::ThinkingDelta {
                        content_index: idx,
                        delta: "\n\n".to_string(),
                        partial: output.clone(),
                    });
                }
            }
            ResponseStreamEvent::ReasoningTextDelta { delta } => {
                if matches!(current_item, CurrentItemKind::Reasoning)
                    && matches!(current_block, CurrentBlock::Thinking)
                {
                    append_thinking(output, &delta);
                    let idx = block_index(output);
                    stream.push(AssistantMessageEvent::ThinkingDelta {
                        content_index: idx,
                        delta,
                        partial: output.clone(),
                    });
                }
            }
            ResponseStreamEvent::ContentPartAdded { part } => {
                if let CurrentItemKind::Message { content } = &mut current_item {
                    // Filter out ReasoningText, only accept output_text and refusal.
                    if matches!(
                        part,
                        ResponseOutputContent::OutputText { .. }
                            | ResponseOutputContent::Refusal { .. }
                    ) {
                        content.push(part);
                    }
                }
            }
            ResponseStreamEvent::OutputTextDelta { delta } => {
                if let CurrentItemKind::Message { content } = &mut current_item {
                    if matches!(current_block, CurrentBlock::Text) {
                        match content.last_mut() {
                            Some(ResponseOutputContent::OutputText { text, .. }) => {
                                append_text(output, &delta);
                                text.push_str(&delta);
                                let idx = block_index(output);
                                stream.push(AssistantMessageEvent::TextDelta {
                                    content_index: idx,
                                    delta,
                                    partial: output.clone(),
                                });
                            }
                            _ => {
                                // No content yet, or last part is not output_text.
                            }
                        }
                    }
                }
            }
            ResponseStreamEvent::RefusalDelta { delta } => {
                if let CurrentItemKind::Message { content } = &mut current_item {
                    if matches!(current_block, CurrentBlock::Text) {
                        match content.last_mut() {
                            Some(ResponseOutputContent::Refusal { refusal }) => {
                                append_text(output, &delta);
                                refusal.push_str(&delta);
                                let idx = block_index(output);
                                stream.push(AssistantMessageEvent::TextDelta {
                                    content_index: idx,
                                    delta,
                                    partial: output.clone(),
                                });
                            }
                            _ => {}
                        }
                    }
                }
            }
            ResponseStreamEvent::FunctionCallArgumentsDelta { delta } => {
                if matches!(current_item, CurrentItemKind::FunctionCall) {
                    if let CurrentBlock::ToolCall { partial_json } = &mut current_block {
                        partial_json.push_str(&delta);
                        let parsed = parse_streaming_json(Some(partial_json));
                        set_tool_call_arguments(output, parsed);
                        let idx = block_index(output);
                        stream.push(AssistantMessageEvent::ToolCallDelta {
                            content_index: idx,
                            delta,
                            partial: output.clone(),
                        });
                    }
                }
            }
            ResponseStreamEvent::FunctionCallArgumentsDone { arguments } => {
                if matches!(current_item, CurrentItemKind::FunctionCall) {
                    if let CurrentBlock::ToolCall { partial_json } = &mut current_block {
                        let previous_partial_json = partial_json.clone();
                        *partial_json = arguments.clone();
                        let parsed = parse_streaming_json(Some(partial_json));
                        set_tool_call_arguments(output, parsed);

                        if arguments.starts_with(&previous_partial_json) {
                            let delta = arguments[previous_partial_json.len()..].to_string();
                            if !delta.is_empty() {
                                let idx = block_index(output);
                                stream.push(AssistantMessageEvent::ToolCallDelta {
                                    content_index: idx,
                                    delta,
                                    partial: output.clone(),
                                });
                            }
                        }
                    }
                }
            }
            ResponseStreamEvent::OutputItemDone { item } => match item {
                ResponseItem::Reasoning {
                    summary, content, ..
                } if matches!(current_block, CurrentBlock::Thinking) => {
                    let summary_text = summary
                        .as_ref()
                        .map(|s| {
                            s.iter()
                                .map(|p| p.text.clone())
                                .collect::<Vec<_>>()
                                .join("\n\n")
                        })
                        .unwrap_or_default();
                    let content_text = content
                        .as_ref()
                        .map(|c| {
                            c.iter()
                                .map(|p| p.text.clone())
                                .collect::<Vec<_>>()
                                .join("\n\n")
                        })
                        .unwrap_or_default();

                    // Reconstruct the full reasoning item JSON for `thinkingSignature`.
                    let item_value = serde_json::to_string(&ResponseItem::Reasoning {
                        id: None,
                        summary,
                        content,
                        extra: serde_json::Map::new(),
                    })
                    .unwrap_or_else(|_| "{}".to_string());

                    let final_text = if !summary_text.is_empty() {
                        summary_text
                    } else if !content_text.is_empty() {
                        content_text
                    } else {
                        current_thinking_text(output)
                    };
                    set_thinking_final(output, &final_text, &item_value);
                    let idx = block_index(output);
                    stream.push(AssistantMessageEvent::ThinkingEnd {
                        content_index: idx,
                        content: final_text,
                        partial: output.clone(),
                    });
                    current_block = CurrentBlock::None;
                }
                ResponseItem::Message {
                    content: content_value,
                    id,
                    phase,
                    ..
                } if matches!(current_block, CurrentBlock::Text) => {
                    let parts: Vec<ResponseOutputContent> =
                        serde_json::from_value(content_value).unwrap_or_default();
                    let final_text: String = parts
                        .iter()
                        .map(|c| match c {
                            ResponseOutputContent::OutputText { text, .. } => text.clone(),
                            ResponseOutputContent::Refusal { refusal } => refusal.clone(),
                        })
                        .collect::<Vec<_>>()
                        .join("");
                    let signature = encode_text_signature_v1(&id.unwrap_or_default(), phase);
                    set_text_final(output, &final_text, &signature);
                    let idx = block_index(output);
                    stream.push(AssistantMessageEvent::TextEnd {
                        content_index: idx,
                        content: final_text,
                        partial: output.clone(),
                    });
                    current_block = CurrentBlock::None;
                }
                ResponseItem::FunctionCall {
                    id,
                    call_id,
                    name,
                    arguments,
                } => {
                    let args = match &current_block {
                        CurrentBlock::ToolCall { partial_json } if !partial_json.is_empty() => {
                            parse_streaming_json(Some(partial_json))
                        }
                        _ => parse_streaming_json(Some(if arguments.is_empty() {
                            "{}"
                        } else {
                            &arguments
                        })),
                    };

                    let tool_call: ToolCall =
                        if matches!(current_block, CurrentBlock::ToolCall { .. }) {
                            // Finalize the in-place tool call block, stripping the scratch
                            // buffer so replay only carries parsed arguments.
                            set_tool_call_arguments(output, args);
                            current_tool_call(output).unwrap_or_else(|| ToolCall {
                                id: format!("{call_id}|{}", id.clone().unwrap_or_default()),
                                name: name.clone(),
                                arguments: Value::Object(serde_json::Map::new()),
                                thought_signature: None,
                            })
                        } else {
                            ToolCall {
                                id: format!("{call_id}|{}", id.unwrap_or_default()),
                                name,
                                arguments: args,
                                thought_signature: None,
                            }
                        };

                    current_block = CurrentBlock::None;
                    let idx = block_index(output);
                    stream.push(AssistantMessageEvent::ToolCallEnd {
                        content_index: idx,
                        tool_call,
                        partial: output.clone(),
                    });
                }
                _ => {}
            },
            ResponseStreamEvent::ResponseCompleted { response } => {
                if let Some(id) = &response.id {
                    output.response_id = Some(id.clone());
                }
                if let Some(usage) = &response.usage {
                    let cached_tokens = usage
                        .input_tokens_details
                        .as_ref()
                        .and_then(|d| d.cached_tokens)
                        .unwrap_or(0);
                    output.usage = Usage {
                        // OpenAI includes cached tokens in input_tokens, so subtract.
                        input: usage
                            .input_tokens
                            .unwrap_or(0)
                            .saturating_sub(cached_tokens),
                        output: usage.output_tokens.unwrap_or(0),
                        cache_read: cached_tokens,
                        cache_write: 0,
                        cache_write_1h: None,
                        total_tokens: usage.total_tokens.unwrap_or(0),
                        cost: crate::types::UsageCost {
                            input: 0.0,
                            output: 0.0,
                            cache_read: 0.0,
                            cache_write: 0.0,
                            total: 0.0,
                        },
                    };
                }
                calculate_cost(model, &mut output.usage);
                if let Some(opts) = options {
                    if let Some(apply) = &opts.apply_service_tier_pricing {
                        let service_tier = if let Some(resolve) = &opts.resolve_service_tier {
                            resolve(response.service_tier.clone(), opts.service_tier.clone())
                        } else {
                            response.service_tier.clone().or(opts.service_tier.clone())
                        };
                        apply(&mut output.usage, service_tier);
                    }
                }
                // Map status to stop reason.
                output.stop_reason = map_stop_reason(response.status.as_deref());
                if output
                    .content
                    .iter()
                    .any(|b| matches!(b, AssistantContentBlock::ToolCall(_)))
                    && output.stop_reason == StopReason::Stop
                {
                    output.stop_reason = StopReason::ToolUse;
                }
            }
            ResponseStreamEvent::Error { code, message } => {
                let msg = format!(
                    "Error Code {}: {}",
                    code.unwrap_or_default(),
                    message.unwrap_or_default()
                );
                return Err(ResponsesStreamError::Provider(msg));
            }
            ResponseStreamEvent::ResponseFailed { response } => {
                let msg = if let Some(error) = response.error {
                    format!(
                        "{}: {}",
                        error.code.unwrap_or_else(|| "unknown".to_string()),
                        error.message.unwrap_or_else(|| "no message".to_string())
                    )
                } else if let Some(reason) = response.incomplete_details.and_then(|d| d.reason) {
                    format!("incomplete: {reason}")
                } else {
                    "Unknown error (no error details in response)".to_string()
                };
                return Err(ResponsesStreamError::Provider(msg));
            }
            ResponseStreamEvent::Other => {}
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Block-mutation helpers (operate on the last content block of `output`)
// ---------------------------------------------------------------------------

fn append_thinking(output: &mut AssistantMessage, delta: &str) {
    if let Some(AssistantContentBlock::Thinking(t)) = output.content.last_mut() {
        t.thinking.push_str(delta);
    }
}

fn append_text(output: &mut AssistantMessage, delta: &str) {
    if let Some(AssistantContentBlock::Text(t)) = output.content.last_mut() {
        t.text.push_str(delta);
    }
}

fn current_thinking_text(output: &AssistantMessage) -> String {
    match output.content.last() {
        Some(AssistantContentBlock::Thinking(t)) => t.thinking.clone(),
        _ => String::new(),
    }
}

fn set_thinking_final(output: &mut AssistantMessage, text: &str, signature: &str) {
    if let Some(AssistantContentBlock::Thinking(t)) = output.content.last_mut() {
        t.thinking = text.to_string();
        t.thinking_signature = Some(signature.to_string());
    }
}

fn set_text_final(output: &mut AssistantMessage, text: &str, signature: &str) {
    if let Some(AssistantContentBlock::Text(t)) = output.content.last_mut() {
        t.text = text.to_string();
        t.text_signature = Some(signature.to_string());
    }
}

fn set_tool_call_arguments(output: &mut AssistantMessage, args: Value) {
    if let Some(AssistantContentBlock::ToolCall(tc)) = output.content.last_mut() {
        tc.arguments = args;
    }
}

fn current_tool_call(output: &AssistantMessage) -> Option<ToolCall> {
    match output.content.last() {
        Some(AssistantContentBlock::ToolCall(tc)) => Some(tc.clone()),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Stop reason mapping
// ---------------------------------------------------------------------------

fn map_stop_reason(status: Option<&str>) -> StopReason {
    match status {
        None => StopReason::Stop,
        Some("completed") => StopReason::Stop,
        Some("incomplete") => StopReason::Length,
        Some("failed") | Some("cancelled") => StopReason::Error,
        // These two are wonky ...
        Some("in_progress") | Some("queued") => StopReason::Stop,
        // TS throws on unhandled; we degrade to Stop to avoid panicking on a
        // future/unknown status value.
        Some(_) => StopReason::Stop,
    }
}

// =============================================================================
// SSE event + response types
// =============================================================================

/// A decoded OpenAI Responses SSE event. Mirrors the SDK `ResponseStreamEvent`
/// union, limited to the variants `process_responses_stream` handles; everything
/// else decodes to [`ResponseStreamEvent::Other`].
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ResponseStreamEvent {
    #[serde(rename = "response.created")]
    ResponseCreated { response: ResponseObject },
    #[serde(rename = "response.output_item.added")]
    OutputItemAdded { item: ResponseItem },
    #[serde(rename = "response.reasoning_summary_part.added")]
    ReasoningSummaryPartAdded { part: ResponseReasoningTextPart },
    #[serde(rename = "response.reasoning_summary_text.delta")]
    ReasoningSummaryTextDelta { delta: String },
    #[serde(rename = "response.reasoning_summary_part.done")]
    ReasoningSummaryPartDone,
    #[serde(rename = "response.reasoning_text.delta")]
    ReasoningTextDelta { delta: String },
    #[serde(rename = "response.content_part.added")]
    ContentPartAdded { part: ResponseOutputContent },
    #[serde(rename = "response.output_text.delta")]
    OutputTextDelta { delta: String },
    #[serde(rename = "response.refusal.delta")]
    RefusalDelta { delta: String },
    #[serde(rename = "response.function_call_arguments.delta")]
    FunctionCallArgumentsDelta { delta: String },
    #[serde(rename = "response.function_call_arguments.done")]
    FunctionCallArgumentsDone { arguments: String },
    #[serde(rename = "response.output_item.done")]
    OutputItemDone { item: ResponseItem },
    #[serde(rename = "response.completed")]
    ResponseCompleted { response: ResponseObject },
    #[serde(rename = "error")]
    Error {
        #[serde(default)]
        code: Option<String>,
        #[serde(default)]
        message: Option<String>,
    },
    #[serde(rename = "response.failed")]
    ResponseFailed { response: ResponseObject },
    /// Any other event type we don't explicitly handle.
    #[serde(other)]
    Other,
}

/// The `response` payload carried by created/completed/failed events.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ResponseObject {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub usage: Option<ResponseUsage>,
    #[serde(default)]
    pub service_tier: Option<String>,
    #[serde(default)]
    pub error: Option<ResponseError>,
    #[serde(default)]
    pub incomplete_details: Option<ResponseIncompleteDetails>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ResponseUsage {
    #[serde(default)]
    pub input_tokens: Option<u64>,
    #[serde(default)]
    pub output_tokens: Option<u64>,
    #[serde(default)]
    pub total_tokens: Option<u64>,
    #[serde(default)]
    pub input_tokens_details: Option<ResponseInputTokensDetails>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ResponseInputTokensDetails {
    #[serde(default)]
    pub cached_tokens: Option<u64>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ResponseError {
    #[serde(default)]
    pub code: Option<String>,
    #[serde(default)]
    pub message: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ResponseIncompleteDetails {
    #[serde(default)]
    pub reason: Option<String>,
}

// =============================================================================
// Errors
// =============================================================================

/// Errors raised while processing the Responses SSE stream.
#[derive(Debug, thiserror::Error)]
pub enum ResponsesStreamError {
    /// A provider-reported error (`error` / `response.failed` event).
    #[error("{0}")]
    Provider(String),
    /// A transport/decoding error surfaced by the upstream SSE stream.
    #[error("{0}")]
    Transport(String),
}
