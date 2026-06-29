//! Port of `../../packages/ai/src/providers/mistral.ts`.
//!
//! Mistral (`mistral-conversations` API) provider backend.
//!
//! Unlike the TS source — which uses the `@mistralai/mistralai` SDK and its
//! `chat.stream` helper — this port issues raw HTTP requests with `reqwest` and
//! parses Mistral's `/v1/chat/completions` Server-Sent Events stream manually.
//! Shared cross-provider message normalization is reused from
//! [`crate::providers::transform_messages`].
//!
//! Entry points:
//! - [`stream`] (a.k.a. `stream_mistral`) — the full provider stream.
//! - [`stream_simple`] (a.k.a. `stream_simple_mistral`) — the `SimpleStreamOptions`
//!   wrapper mapping a reasoning level to Mistral reasoning controls.

use std::cell::RefCell;
use std::collections::HashMap;

use futures::StreamExt;
use serde_json::Value;

use crate::models::{calculate_cost, clamp_thinking_level};
use crate::providers::simple_options::build_base_options;
use crate::providers::transform_messages::transform_messages;
use crate::types::{
    AssistantContentBlock, AssistantMessage, AssistantMessageEvent, Context, DoneReason,
    ErrorReason, InputModality, Message, MessageContent, MessageRole, Model, ModelThinkingLevel,
    SimpleStreamOptions, StopReason, StreamOptions, TextContent, ThinkingContent, Tool, ToolCall,
    Usage, UsageCost,
};
use crate::utils::event_stream::{
    AssistantMessageEventStream, AssistantMessageEventStreamSender,
    create_assistant_message_event_stream,
};
use crate::utils::hash::short_hash;
use crate::utils::json_parse::parse_streaming_json;
use crate::utils::sanitize_unicode::sanitize_surrogates;

const MISTRAL_TOOL_CALL_ID_LENGTH: usize = 9;
const MAX_MISTRAL_ERROR_BODY_CHARS: usize = 4000;

// ---------------------------------------------------------------------------
// Options
// ---------------------------------------------------------------------------

/// Mirrors the TS `MistralReasoningEffort = "none" | "high"`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MistralReasoningEffort {
    None,
    High,
}

impl MistralReasoningEffort {
    fn as_str(self) -> &'static str {
        match self {
            MistralReasoningEffort::None => "none",
            MistralReasoningEffort::High => "high",
        }
    }
}

/// A Mistral `toolChoice` selection.
///
/// Mirrors the TS `"auto" | "none" | "any" | "required" | { type: "function"; function: { name } }`.
#[derive(Clone, Debug)]
pub enum MistralToolChoice {
    Auto,
    None,
    Any,
    Required,
    Function { name: String },
}

/// Mistral-specific stream options.
///
/// Mirrors the TS `MistralOptions extends StreamOptions`.
#[derive(Clone, Debug, Default)]
pub struct MistralOptions {
    pub base: StreamOptions,
    pub tool_choice: Option<MistralToolChoice>,
    /// Only `"reasoning"` is valid; modeled as a bool.
    pub prompt_mode_reasoning: bool,
    pub reasoning_effort: Option<MistralReasoningEffort>,
}

// ---------------------------------------------------------------------------
// Default partial AssistantMessage
// ---------------------------------------------------------------------------

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

/// Mirrors the TS `createOutput`.
fn create_output(model: &Model) -> AssistantMessage {
    AssistantMessage {
        role: MessageRole::Assistant,
        content: Vec::new(),
        api: model.api.to_string(),
        provider: model.provider.clone(),
        model: model.id.clone(),
        response_model: None,
        response_id: None,
        usage: empty_usage(),
        stop_reason: StopReason::Stop,
        error_message: None,
        diagnostics: None,
        timestamp: chrono::Utc::now(),
    }
}

// ---------------------------------------------------------------------------
// Public entry points
// ---------------------------------------------------------------------------

/// Stream a completion from a Mistral model.
///
/// Mirrors the TS `streamMistral`.
pub fn stream(
    model: Model,
    context: Context,
    options: Option<MistralOptions>,
) -> AssistantMessageEventStream {
    let (sender, stream_out) = create_assistant_message_event_stream();
    let options = options.unwrap_or_default();

    tokio::spawn(async move {
        run_stream(model, context, options, sender).await;
    });

    stream_out
}

/// TS-named alias for [`stream`].
pub fn stream_mistral(
    model: Model,
    context: Context,
    options: Option<MistralOptions>,
) -> AssistantMessageEventStream {
    stream(model, context, options)
}

/// Maps provider-agnostic `SimpleStreamOptions` to Mistral options.
///
/// Mirrors the TS `streamSimpleMistral`.
pub fn stream_simple(
    model: Model,
    context: Context,
    options: Option<SimpleStreamOptions>,
) -> AssistantMessageEventStream {
    let api_key = options.as_ref().and_then(|o| o.base.api_key.clone());
    if api_key.as_deref().map(str::is_empty).unwrap_or(true) {
        return error_stream(
            &model,
            format!("No API key for provider: {}", model.provider),
        );
    }

    let base = build_base_options(&model, options.as_ref(), api_key.as_deref());

    // const clampedReasoning = options?.reasoning ? clampThinkingLevel(model, options.reasoning) : undefined;
    // const reasoning = clampedReasoning === "off" ? undefined : clampedReasoning;
    let clamped_reasoning = options
        .as_ref()
        .and_then(|o| o.reasoning)
        .map(|r| clamp_thinking_level(&model, ModelThinkingLevel::from(r)));
    let reasoning = match clamped_reasoning {
        Some(ModelThinkingLevel::Off) | None => None,
        Some(level) => Some(level),
    };

    // const shouldUseReasoning = model.reasoning && reasoning !== undefined;
    let should_use_reasoning = model.reasoning && reasoning.is_some();

    let prompt_mode_reasoning = should_use_reasoning && uses_prompt_mode_reasoning(&model);
    let reasoning_effort = if should_use_reasoning && uses_reasoning_effort(&model) {
        // `reasoning` is Some here because should_use_reasoning implies it.
        reasoning.map(|level| map_reasoning_effort(&model, level))
    } else {
        None
    };

    let opts = MistralOptions {
        base,
        tool_choice: None,
        prompt_mode_reasoning,
        reasoning_effort,
    };

    stream(model, context, Some(opts))
}

/// TS-named alias for [`stream_simple`].
pub fn stream_simple_mistral(
    model: Model,
    context: Context,
    options: Option<SimpleStreamOptions>,
) -> AssistantMessageEventStream {
    stream_simple(model, context, options)
}

/// Produce a stream that immediately emits a terminal error event.
fn error_stream(model: &Model, message: String) -> AssistantMessageEventStream {
    let (mut sender, stream_out) = create_assistant_message_event_stream();
    let mut output = create_output(model);
    output.stop_reason = StopReason::Error;
    output.error_message = Some(message);
    sender.push(AssistantMessageEvent::Error {
        reason: ErrorReason::Error,
        error: output,
    });
    sender.end(None);
    stream_out
}

// ---------------------------------------------------------------------------
// Tool call ID normalization
// ---------------------------------------------------------------------------

/// Mirrors the TS `deriveMistralToolCallId`.
fn derive_mistral_tool_call_id(id: &str, attempt: u32) -> String {
    let normalized: String = id.chars().filter(|c| c.is_ascii_alphanumeric()).collect();
    if attempt == 0 && normalized.len() == MISTRAL_TOOL_CALL_ID_LENGTH {
        return normalized;
    }
    let seed_base = if normalized.is_empty() {
        id.to_string()
    } else {
        normalized
    };
    let seed = if attempt == 0 {
        seed_base
    } else {
        format!("{seed_base}:{attempt}")
    };
    short_hash(&seed)
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .take(MISTRAL_TOOL_CALL_ID_LENGTH)
        .collect()
}

/// Mirrors the TS `createMistralToolCallIdNormalizer` — a stateful closure that
/// maps original tool-call IDs onto short, unique, alphanumeric IDs.
///
/// The TS variant returns a `(id) => string` closure with private `Map`s. We back
/// the same state with `RefCell` so the `&dyn Fn` passed to `transform_messages`
/// can mutate it. The full pipeline runs synchronously on one task, so the single
/// `RefCell` borrow per call never overlaps.
struct MistralToolCallIdNormalizer {
    id_map: RefCell<HashMap<String, String>>,
    reverse_map: RefCell<HashMap<String, String>>,
}

impl MistralToolCallIdNormalizer {
    fn new() -> Self {
        Self {
            id_map: RefCell::new(HashMap::new()),
            reverse_map: RefCell::new(HashMap::new()),
        }
    }

    fn normalize(&self, id: &str) -> String {
        if let Some(existing) = self.id_map.borrow().get(id) {
            return existing.clone();
        }

        let mut attempt = 0u32;
        loop {
            let candidate = derive_mistral_tool_call_id(id, attempt);
            let owner = self.reverse_map.borrow().get(&candidate).cloned();
            match owner {
                Some(ref o) if o != id => {
                    attempt += 1;
                    continue;
                }
                _ => {
                    self.id_map
                        .borrow_mut()
                        .insert(id.to_string(), candidate.clone());
                    self.reverse_map
                        .borrow_mut()
                        .insert(candidate.clone(), id.to_string());
                    return candidate;
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Error formatting
// ---------------------------------------------------------------------------

/// Mirrors the TS `truncateErrorText`.
fn truncate_error_text(text: &str, max_chars: usize) -> String {
    let count = text.chars().count();
    if count <= max_chars {
        return text.to_string();
    }
    let head: String = text.chars().take(max_chars).collect();
    format!("{head}... [truncated {} chars]", count - max_chars)
}

/// Mirrors the TS `formatMistralError` for the HTTP-error case (status + body).
fn format_mistral_http_error(status: u16, body: &str) -> String {
    let trimmed = body.trim();
    if !trimmed.is_empty() {
        format!(
            "Mistral API error ({}): {}",
            status,
            truncate_error_text(trimmed, MAX_MISTRAL_ERROR_BODY_CHARS)
        )
    } else {
        format!("Mistral API error ({status})")
    }
}

// ---------------------------------------------------------------------------
// Reasoning-mode selection
// ---------------------------------------------------------------------------

/// Mirrors the TS `usesReasoningEffort`.
fn uses_reasoning_effort(model: &Model) -> bool {
    model.id == "mistral-small-2603"
        || model.id == "mistral-small-latest"
        || model.id == "mistral-medium-3.5"
}

/// Mirrors the TS `usesPromptModeReasoning`.
fn uses_prompt_mode_reasoning(model: &Model) -> bool {
    model.reasoning && !uses_reasoning_effort(model)
}

/// Mirrors the TS `mapReasoningEffort`.
///
/// `model.thinkingLevelMap?.[level] ?? "high"`. A mapped value is parsed back into
/// a [`MistralReasoningEffort`]; anything not `"none"`/`"high"` defaults to `High`.
fn map_reasoning_effort(model: &Model, level: ModelThinkingLevel) -> MistralReasoningEffort {
    let mapped = model
        .thinking_level_map
        .as_ref()
        .and_then(|m| m.get(&level))
        .and_then(|v| v.as_deref());
    match mapped {
        Some("none") => MistralReasoningEffort::None,
        Some("high") => MistralReasoningEffort::High,
        // Default (absent key, JSON null, or unrecognized value) → "high".
        _ => MistralReasoningEffort::High,
    }
}

// ---------------------------------------------------------------------------
// Stop reason
// ---------------------------------------------------------------------------

/// Mirrors the TS `mapChatStopReason`.
fn map_chat_stop_reason(reason: Option<&str>) -> StopReason {
    match reason {
        None => StopReason::Stop,
        Some("stop") => StopReason::Stop,
        Some("length") | Some("model_length") => StopReason::Length,
        Some("tool_calls") => StopReason::ToolUse,
        Some("error") => StopReason::Error,
        _ => StopReason::Stop,
    }
}

// ---------------------------------------------------------------------------
// Payload building
// ---------------------------------------------------------------------------

/// Mirrors the TS `stripSymbolKeys`. JSON `Value`s carry no symbol keys, so this is
/// an identity pass — kept for parity with the TS pipeline (and the tool-schema test).
fn strip_symbol_keys(value: &Value) -> Value {
    value.clone()
}

/// Mirrors the TS `toFunctionTools`.
fn to_function_tools(tools: &[Tool]) -> Vec<Value> {
    tools
        .iter()
        .map(|tool| {
            serde_json::json!({
                "type": "function",
                "function": {
                    "name": tool.name,
                    "description": tool.description,
                    "parameters": strip_symbol_keys(&tool.parameters),
                    "strict": false,
                }
            })
        })
        .collect()
}

/// Mirrors the TS `mapToolChoice`.
fn map_tool_choice(choice: &MistralToolChoice) -> Value {
    match choice {
        MistralToolChoice::Auto => Value::String("auto".into()),
        MistralToolChoice::None => Value::String("none".into()),
        MistralToolChoice::Any => Value::String("any".into()),
        MistralToolChoice::Required => Value::String("required".into()),
        MistralToolChoice::Function { name } => serde_json::json!({
            "type": "function",
            "function": { "name": name }
        }),
    }
}

/// Mirrors the TS `buildToolResultText`.
fn build_tool_result_text(
    text: &str,
    has_images: bool,
    supports_images: bool,
    is_error: bool,
) -> String {
    let trimmed = text.trim();
    let error_prefix = if is_error { "[tool error] " } else { "" };

    if !trimmed.is_empty() {
        let image_suffix = if has_images && !supports_images {
            "\n[tool image omitted: model does not support images]"
        } else {
            ""
        };
        return format!("{error_prefix}{trimmed}{image_suffix}");
    }

    if has_images {
        if supports_images {
            return if is_error {
                "[tool error] (see attached image)".to_string()
            } else {
                "(see attached image)".to_string()
            };
        }
        return if is_error {
            "[tool error] (image omitted: model does not support images)".to_string()
        } else {
            "(image omitted: model does not support images)".to_string()
        };
    }

    if is_error {
        "[tool error] (no tool output)".to_string()
    } else {
        "(no tool output)".to_string()
    }
}

/// Mirrors the TS `toChatMessages`. Produces the Mistral chat `messages` array.
fn to_chat_messages(messages: &[Message], supports_images: bool) -> Vec<Value> {
    let mut result: Vec<Value> = Vec::new();

    for msg in messages {
        match msg {
            Message::User(user_msg) => {
                // The Rust `UserMessage.content` is always `Vec<MessageContent>`
                // (never a bare string), unlike the TS union. A single text block
                // serializes to a plain string to match the TS string branch.
                if user_msg.content.len() == 1 {
                    if let MessageContent::Text(tc) = &user_msg.content[0] {
                        result.push(serde_json::json!({
                            "role": "user",
                            "content": sanitize_surrogates(&tc.text),
                        }));
                        continue;
                    }
                }

                let had_images = user_msg
                    .content
                    .iter()
                    .any(|item| matches!(item, MessageContent::Image(_)));

                let content: Vec<Value> = user_msg
                    .content
                    .iter()
                    .filter(|item| matches!(item, MessageContent::Text(_)) || supports_images)
                    .map(|item| match item {
                        MessageContent::Text(tc) => serde_json::json!({
                            "type": "text",
                            "text": sanitize_surrogates(&tc.text),
                        }),
                        MessageContent::Image(img) => serde_json::json!({
                            "type": "image_url",
                            "imageUrl": format!("data:{};base64,{}", img.mime_type, img.data),
                        }),
                    })
                    .collect();

                if !content.is_empty() {
                    result.push(serde_json::json!({
                        "role": "user",
                        "content": content,
                    }));
                    continue;
                }
                if had_images && !supports_images {
                    result.push(serde_json::json!({
                        "role": "user",
                        "content": "(image omitted: model does not support images)",
                    }));
                }
            }

            Message::Assistant(assistant_msg) => {
                let mut content_parts: Vec<Value> = Vec::new();
                let mut tool_calls: Vec<Value> = Vec::new();

                for block in &assistant_msg.content {
                    match block {
                        AssistantContentBlock::Text(tc) => {
                            if !tc.text.trim().is_empty() {
                                content_parts.push(serde_json::json!({
                                    "type": "text",
                                    "text": sanitize_surrogates(&tc.text),
                                }));
                            }
                        }
                        AssistantContentBlock::Thinking(tk) => {
                            if !tk.thinking.trim().is_empty() {
                                content_parts.push(serde_json::json!({
                                    "type": "thinking",
                                    "thinking": [
                                        { "type": "text", "text": sanitize_surrogates(&tk.thinking) }
                                    ],
                                }));
                            }
                        }
                        AssistantContentBlock::ToolCall(call) => {
                            let args = serde_json::to_string(&call.arguments)
                                .unwrap_or_else(|_| "{}".to_string());
                            tool_calls.push(serde_json::json!({
                                "id": call.id,
                                "type": "function",
                                "function": { "name": call.name, "arguments": args },
                            }));
                        }
                    }
                }

                let mut assistant_message = serde_json::Map::new();
                assistant_message.insert("role".into(), Value::String("assistant".into()));
                let has_content = !content_parts.is_empty();
                let has_tool_calls = !tool_calls.is_empty();
                if has_content {
                    assistant_message.insert("content".into(), Value::Array(content_parts));
                }
                if has_tool_calls {
                    assistant_message.insert("toolCalls".into(), Value::Array(tool_calls));
                }
                if has_content || has_tool_calls {
                    result.push(Value::Object(assistant_message));
                }
            }

            Message::ToolResult(tool_msg) => {
                let text_result = tool_msg
                    .content
                    .iter()
                    .filter_map(|part| match part {
                        MessageContent::Text(tc) => Some(sanitize_surrogates(&tc.text)),
                        _ => None,
                    })
                    .collect::<Vec<_>>()
                    .join("\n");
                let has_images = tool_msg
                    .content
                    .iter()
                    .any(|part| matches!(part, MessageContent::Image(_)));
                let tool_text = build_tool_result_text(
                    &text_result,
                    has_images,
                    supports_images,
                    tool_msg.is_error,
                );

                let mut tool_content: Vec<Value> =
                    vec![serde_json::json!({ "type": "text", "text": tool_text })];

                for part in &tool_msg.content {
                    if !supports_images {
                        continue;
                    }
                    if let MessageContent::Image(img) = part {
                        tool_content.push(serde_json::json!({
                            "type": "image_url",
                            "imageUrl": format!("data:{};base64,{}", img.mime_type, img.data),
                        }));
                    }
                }

                result.push(serde_json::json!({
                    "role": "tool",
                    "toolCallId": tool_msg.tool_call_id,
                    "name": tool_msg.tool_name,
                    "content": tool_content,
                }));
            }
        }
    }

    result
}

/// Mirrors the TS `buildChatPayload`.
fn build_chat_payload(
    model: &Model,
    context: &Context,
    messages: &[Message],
    options: &MistralOptions,
) -> Value {
    let supports_images = model.input.contains(&InputModality::Image);
    let mut chat_messages = to_chat_messages(messages, supports_images);

    let mut payload = serde_json::Map::new();
    payload.insert("model".into(), Value::String(model.id.clone()));
    payload.insert("stream".into(), Value::Bool(true));

    if !context.tools.is_empty() {
        payload.insert(
            "tools".into(),
            Value::Array(to_function_tools(&context.tools)),
        );
    }
    if let Some(temp) = options.base.temperature {
        payload.insert("temperature".into(), serde_json::json!(temp));
    }
    if let Some(max_tokens) = options.base.max_tokens {
        payload.insert("maxTokens".into(), serde_json::json!(max_tokens));
    }
    if let Some(tool_choice) = &options.tool_choice {
        payload.insert("toolChoice".into(), map_tool_choice(tool_choice));
    }
    if options.prompt_mode_reasoning {
        payload.insert("promptMode".into(), Value::String("reasoning".into()));
    }
    if let Some(effort) = options.reasoning_effort {
        payload.insert(
            "reasoningEffort".into(),
            Value::String(effort.as_str().into()),
        );
    }

    if let Some(system_prompt) = &context.system_prompt {
        if !system_prompt.is_empty() {
            chat_messages.insert(
                0,
                serde_json::json!({
                    "role": "system",
                    "content": sanitize_surrogates(system_prompt),
                }),
            );
        }
    }

    payload.insert("messages".into(), Value::Array(chat_messages));

    Value::Object(payload)
}

// ---------------------------------------------------------------------------
// Request options (headers / URL)
// ---------------------------------------------------------------------------

/// Mirrors the TS `buildRequestOptions` header assembly.
fn build_request_headers(model: &Model, options: &MistralOptions) -> HashMap<String, String> {
    let mut headers: HashMap<String, String> = HashMap::new();
    if let Some(model_headers) = &model.headers {
        for (k, v) in model_headers {
            headers.insert(k.clone(), v.clone());
        }
    }
    if let Some(opt_headers) = &options.base.headers {
        for (k, v) in opt_headers {
            headers.insert(k.clone(), v.clone());
        }
    }

    // Mistral infrastructure uses `x-affinity` for KV-cache reuse (prefix caching).
    // Respect explicit caller-provided header values.
    if let Some(session_id) = &options.base.session_id {
        if !headers.contains_key("x-affinity") {
            headers.insert("x-affinity".into(), session_id.clone());
        }
    }

    headers
}

/// Compute the chat-completions endpoint URL.
///
/// The TS SDK uses `serverURL: model.baseUrl` and posts to `/v1/chat/completions`.
fn build_url(model: &Model) -> String {
    let trimmed = model.base_url.trim_end_matches('/');
    let base = if trimmed.is_empty() {
        "https://api.mistral.ai"
    } else {
        trimmed
    };
    if base.ends_with("/v1") {
        format!("{base}/chat/completions")
    } else {
        format!("{base}/v1/chat/completions")
    }
}

// ---------------------------------------------------------------------------
// Stream driver
// ---------------------------------------------------------------------------

async fn run_stream(
    model: Model,
    context: Context,
    options: MistralOptions,
    mut sender: AssistantMessageEventStreamSender,
) {
    let mut output = create_output(&model);

    match run_stream_inner(&model, &context, &options, &mut sender, &mut output).await {
        Ok(()) => {
            let reason = done_reason_from_stop(output.stop_reason);
            sender.push(AssistantMessageEvent::Done {
                reason,
                message: output,
            });
            sender.end(None);
        }
        Err(err) => {
            // partialArgs is only a streaming scratch buffer and is never persisted
            // in this port (tool-call args live as parsed JSON), so nothing to strip.
            let aborted = options
                .base
                .signal
                .as_ref()
                .map(|s| *s.borrow())
                .unwrap_or(false);
            output.stop_reason = if aborted {
                StopReason::Aborted
            } else {
                StopReason::Error
            };
            output.error_message = Some(err);
            let reason = if aborted {
                ErrorReason::Aborted
            } else {
                ErrorReason::Error
            };
            sender.push(AssistantMessageEvent::Error {
                reason,
                error: output,
            });
            sender.end(None);
        }
    }
}

/// Map a final `StopReason` to a `DoneReason` for the terminal `done` event.
fn done_reason_from_stop(stop: StopReason) -> DoneReason {
    match stop {
        StopReason::Length => DoneReason::Length,
        StopReason::ToolUse => DoneReason::ToolUse,
        _ => DoneReason::Stop,
    }
}

/// The core streaming logic. Returns `Err(message)` for any failure, which the
/// caller converts into a terminal error event.
async fn run_stream_inner(
    model: &Model,
    context: &Context,
    options: &MistralOptions,
    sender: &mut AssistantMessageEventStreamSender,
    output: &mut AssistantMessage,
) -> Result<(), String> {
    let api_key = match &options.base.api_key {
        Some(k) if !k.is_empty() => k.clone(),
        _ => return Err(format!("No API key for provider: {}", model.provider)),
    };

    // Normalize tool-call IDs across the transcript (mirrors the TS normalizer).
    let normalizer = MistralToolCallIdNormalizer::new();
    let normalize_cb =
        move |id: &str, _model: &Model, _src: &AssistantMessage| normalizer.normalize(id);
    let transformed_messages =
        transform_messages(context.messages.clone(), model, Some(&normalize_cb));

    let mut body = build_chat_payload(model, context, &transformed_messages, options);

    // onPayload hook: allow inspection/replacement before sending.
    if let Some(on_payload) = &options.base.on_payload {
        if let Some(next) = on_payload(body.clone(), model.clone()).await {
            body = next;
        }
    }

    let url = build_url(model);
    let headers = build_request_headers(model, options);

    let client = reqwest::Client::new();
    let mut request = client
        .post(&url)
        .header("authorization", format!("Bearer {api_key}"))
        .header("content-type", "application/json")
        .header("accept", "text/event-stream");

    for (k, v) in &headers {
        request = request.header(k, v);
    }

    let body_bytes = serde_json::to_vec(&body).map_err(|e| e.to_string())?;
    request = request.body(body_bytes);

    let response = send_with_abort(request, options.base.signal.clone()).await?;

    if !response.status().is_success() {
        let status = response.status().as_u16();
        let text = response.text().await.unwrap_or_default();
        return Err(format_mistral_http_error(status, &text));
    }

    // Emit `start`.
    sender.push(AssistantMessageEvent::Start {
        partial: output.clone(),
    });

    consume_chat_stream(model, output, sender, response, options.base.signal.clone()).await?;

    // Post-loop finalization (mirrors the TS post-stream throws).
    if let Some(signal) = &options.base.signal {
        if *signal.borrow() {
            return Err("Request was aborted".to_string());
        }
    }
    if output.stop_reason == StopReason::Aborted || output.stop_reason == StopReason::Error {
        return Err("An unknown error occurred".to_string());
    }

    Ok(())
}

/// Wait until the abort signal becomes `true`.
async fn wait_for_abort(signal: &mut tokio::sync::watch::Receiver<bool>) {
    loop {
        if *signal.borrow() {
            return;
        }
        if signal.changed().await.is_err() {
            std::future::pending::<()>().await;
        }
    }
}

/// Send a request, racing it against the abort signal.
async fn send_with_abort(
    request: reqwest::RequestBuilder,
    signal: Option<tokio::sync::watch::Receiver<bool>>,
) -> Result<reqwest::Response, String> {
    match signal {
        Some(mut sig) => {
            tokio::select! {
                resp = request.send() => resp.map_err(|e| e.to_string()),
                _ = wait_for_abort(&mut sig) => Err("Request was aborted".to_string()),
            }
        }
        None => request.send().await.map_err(|e| e.to_string()),
    }
}

// ---------------------------------------------------------------------------
// Streaming-block bookkeeping
// ---------------------------------------------------------------------------

/// The currently-open text or thinking block during streaming.
#[derive(Clone, Copy, PartialEq, Eq)]
enum CurrentBlockKind {
    Text,
    Thinking,
}

// ---------------------------------------------------------------------------
// Stream consumption
// ---------------------------------------------------------------------------

/// Mirrors the TS `consumeChatStream`, driving SSE parsing + event emission.
async fn consume_chat_stream(
    model: &Model,
    output: &mut AssistantMessage,
    sender: &mut AssistantMessageEventStreamSender,
    response: reqwest::Response,
    signal: Option<tokio::sync::watch::Receiver<bool>>,
) -> Result<(), String> {
    let mut current_block: Option<CurrentBlockKind> = None;
    // Scratch buffer of accumulated tool-call argument text, keyed by tool-block
    // content index (mirrors the TS per-block `partialArgs`).
    let mut tool_partial_args: HashMap<usize, String> = HashMap::new();
    // `${callId}:${index}` → content index in `output.content`.
    let mut tool_blocks_by_key: HashMap<String, usize> = HashMap::new();

    let mut byte_stream = response.bytes_stream();
    let mut buffer: Vec<u8> = Vec::new();

    loop {
        if let Some(sig) = &signal {
            if *sig.borrow() {
                return Err("Request was aborted".to_string());
            }
        }

        let next = if let Some(sig) = signal.clone() {
            let mut sig = sig;
            tokio::select! {
                chunk = byte_stream.next() => chunk,
                _ = wait_for_abort(&mut sig) => {
                    return Err("Request was aborted".to_string());
                }
            }
        } else {
            byte_stream.next().await
        };

        let chunk = match next {
            Some(Ok(bytes)) => bytes,
            Some(Err(e)) => return Err(format!("Stream error: {e}")),
            None => break,
        };

        buffer.extend_from_slice(&chunk);

        while let Some(pos) = buffer.iter().position(|&b| b == b'\n') {
            let line_bytes: Vec<u8> = buffer.drain(..=pos).collect();
            let line = String::from_utf8_lossy(&line_bytes);
            let line = line.trim_end_matches(['\r', '\n']);

            if line.is_empty() {
                continue;
            }
            let data = match line.strip_prefix("data:") {
                Some(d) => d.trim_start(),
                None => continue,
            };
            if data == "[DONE]" {
                continue;
            }
            let chunk_json: Value = match serde_json::from_str(data) {
                Ok(v) => v,
                Err(_) => continue,
            };
            process_chunk(
                model,
                &chunk_json,
                output,
                &mut current_block,
                &mut tool_partial_args,
                &mut tool_blocks_by_key,
                sender,
            );
        }
    }

    // Flush any trailing buffered data (no trailing newline).
    if !buffer.is_empty() {
        let line = String::from_utf8_lossy(&buffer);
        let line = line.trim();
        if let Some(data) = line.strip_prefix("data:") {
            let data = data.trim();
            if data != "[DONE]" {
                if let Ok(chunk_json) = serde_json::from_str::<Value>(data) {
                    process_chunk(
                        model,
                        &chunk_json,
                        output,
                        &mut current_block,
                        &mut tool_partial_args,
                        &mut tool_blocks_by_key,
                        sender,
                    );
                }
            }
        }
    }

    // finishCurrentBlock(currentBlock)
    if let Some(kind) = current_block.take() {
        finish_current_block(kind, output, sender);
    }

    // Finalize tool blocks: re-parse args and emit toolcall_end.
    let mut tool_indices: Vec<usize> = tool_blocks_by_key.values().copied().collect();
    tool_indices.sort_unstable();
    tool_indices.dedup();
    for index in tool_indices {
        let partial = tool_partial_args.get(&index).cloned();
        let parsed = parse_streaming_json(partial.as_deref());
        if let Some(AssistantContentBlock::ToolCall(tc)) = output.content.get_mut(index) {
            tc.arguments = parsed;
            let tool_call = tc.clone();
            sender.push(AssistantMessageEvent::ToolCallEnd {
                content_index: index,
                tool_call,
                partial: output.clone(),
            });
        }
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn process_chunk(
    model: &Model,
    event: &Value,
    output: &mut AssistantMessage,
    current_block: &mut Option<CurrentBlockKind>,
    tool_partial_args: &mut HashMap<usize, String>,
    tool_blocks_by_key: &mut HashMap<String, usize>,
    sender: &mut AssistantMessageEventStreamSender,
) {
    // The SDK wraps each SSE payload as `{ data: CompletionChunk }`; the raw wire
    // form is the chunk itself. Accept either shape.
    let chunk = event.get("data").filter(|d| d.is_object()).unwrap_or(event);

    // output.responseId ||= chunk.id
    if output.response_id.is_none() {
        if let Some(id) = chunk.get("id").and_then(|v| v.as_str()) {
            if !id.is_empty() {
                output.response_id = Some(id.to_owned());
            }
        }
    }

    // usage
    if let Some(usage) = chunk.get("usage").filter(|u| !u.is_null()) {
        let input = usage_field(usage, &["promptTokens", "prompt_tokens"]);
        let out = usage_field(usage, &["completionTokens", "completion_tokens"]);
        let total_field = usage_field(usage, &["totalTokens", "total_tokens"]);
        output.usage.input = input;
        output.usage.output = out;
        output.usage.cache_read = 0;
        output.usage.cache_write = 0;
        output.usage.total_tokens = if total_field != 0 {
            total_field
        } else {
            input + out
        };
        calculate_cost(model, &mut output.usage);
    }

    let choice = match chunk
        .get("choices")
        .and_then(|c| c.as_array())
        .and_then(|arr| arr.first())
    {
        Some(c) => c,
        None => return,
    };

    if let Some(finish) = choice
        .get("finishReason")
        .or_else(|| choice.get("finish_reason"))
    {
        if let Some(reason) = finish.as_str() {
            output.stop_reason = map_chat_stop_reason(Some(reason));
        }
        // null finish reason → no change (TS only assigns when truthy).
    }

    let delta = match choice.get("delta") {
        Some(d) => d,
        None => return,
    };

    // ----- content -----
    if let Some(content) = delta.get("content") {
        if !content.is_null() {
            process_content(content, output, current_block, sender);
        }
    }

    // ----- tool calls -----
    let tool_calls = delta
        .get("toolCalls")
        .or_else(|| delta.get("tool_calls"))
        .and_then(|t| t.as_array());

    if let Some(tool_calls) = tool_calls {
        for tool_call in tool_calls {
            // Close any open text/thinking block.
            if let Some(kind) = current_block.take() {
                finish_current_block(kind, output, sender);
            }
            process_tool_call_delta(
                tool_call,
                output,
                tool_partial_args,
                tool_blocks_by_key,
                sender,
            );
        }
    }
}

/// Look up a usage field under any of the candidate (camelCase / snake_case) keys.
fn usage_field(usage: &Value, keys: &[&str]) -> u64 {
    for key in keys {
        if let Some(v) = usage.get(key).and_then(|v| v.as_u64()) {
            return v;
        }
    }
    0
}

/// Handle the `delta.content` field, which may be a string or an array of items.
fn process_content(
    content: &Value,
    output: &mut AssistantMessage,
    current_block: &mut Option<CurrentBlockKind>,
    sender: &mut AssistantMessageEventStreamSender,
) {
    // `typeof delta.content === "string" ? [delta.content] : delta.content`
    if let Some(s) = content.as_str() {
        push_text_delta(&sanitize_surrogates(s), output, current_block, sender);
        return;
    }

    let items = match content.as_array() {
        Some(arr) => arr,
        None => return,
    };

    for item in items {
        // String item.
        if let Some(s) = item.as_str() {
            push_text_delta(&sanitize_surrogates(s), output, current_block, sender);
            continue;
        }

        let item_type = item.get("type").and_then(|t| t.as_str());
        match item_type {
            Some("thinking") => {
                // item.thinking: Array<{ text?: string }>
                let delta_text: String = item
                    .get("thinking")
                    .and_then(|t| t.as_array())
                    .map(|parts| {
                        parts
                            .iter()
                            .filter_map(|p| p.get("text").and_then(|t| t.as_str()))
                            .filter(|t| !t.is_empty())
                            .collect::<Vec<_>>()
                            .join("")
                    })
                    .unwrap_or_default();
                let thinking_delta = sanitize_surrogates(&delta_text);
                if thinking_delta.is_empty() {
                    continue;
                }
                push_thinking_delta(&thinking_delta, output, current_block, sender);
            }
            Some("text") => {
                let text = item.get("text").and_then(|t| t.as_str()).unwrap_or("");
                push_text_delta(&sanitize_surrogates(text), output, current_block, sender);
            }
            _ => {}
        }
    }
}

/// Append a text delta, opening a new text block if necessary.
fn push_text_delta(
    text_delta: &str,
    output: &mut AssistantMessage,
    current_block: &mut Option<CurrentBlockKind>,
    sender: &mut AssistantMessageEventStreamSender,
) {
    if *current_block != Some(CurrentBlockKind::Text) {
        if let Some(kind) = current_block.take() {
            finish_current_block(kind, output, sender);
        }
        output
            .content
            .push(AssistantContentBlock::Text(TextContent {
                text: String::new(),
                text_signature: None,
            }));
        *current_block = Some(CurrentBlockKind::Text);
        let idx = output.content.len() - 1;
        sender.push(AssistantMessageEvent::TextStart {
            content_index: idx,
            partial: output.clone(),
        });
    }

    let idx = output.content.len() - 1;
    if let Some(AssistantContentBlock::Text(tc)) = output.content.get_mut(idx) {
        tc.text.push_str(text_delta);
    }
    sender.push(AssistantMessageEvent::TextDelta {
        content_index: idx,
        delta: text_delta.to_owned(),
        partial: output.clone(),
    });
}

/// Append a thinking delta, opening a new thinking block if necessary.
fn push_thinking_delta(
    thinking_delta: &str,
    output: &mut AssistantMessage,
    current_block: &mut Option<CurrentBlockKind>,
    sender: &mut AssistantMessageEventStreamSender,
) {
    if *current_block != Some(CurrentBlockKind::Thinking) {
        if let Some(kind) = current_block.take() {
            finish_current_block(kind, output, sender);
        }
        output
            .content
            .push(AssistantContentBlock::Thinking(ThinkingContent {
                thinking: String::new(),
                thinking_signature: None,
                redacted: false,
            }));
        *current_block = Some(CurrentBlockKind::Thinking);
        let idx = output.content.len() - 1;
        sender.push(AssistantMessageEvent::ThinkingStart {
            content_index: idx,
            partial: output.clone(),
        });
    }

    let idx = output.content.len() - 1;
    if let Some(AssistantContentBlock::Thinking(tc)) = output.content.get_mut(idx) {
        tc.thinking.push_str(thinking_delta);
    }
    sender.push(AssistantMessageEvent::ThinkingDelta {
        content_index: idx,
        delta: thinking_delta.to_owned(),
        partial: output.clone(),
    });
}

/// Handle a single tool-call delta, creating or extending the matching tool block.
fn process_tool_call_delta(
    tool_call: &Value,
    output: &mut AssistantMessage,
    tool_partial_args: &mut HashMap<usize, String>,
    tool_blocks_by_key: &mut HashMap<String, usize>,
    sender: &mut AssistantMessageEventStreamSender,
) {
    let index = tool_call.get("index").and_then(|v| v.as_u64()).unwrap_or(0);

    let raw_id = tool_call.get("id").and_then(|v| v.as_str());
    let call_id = match raw_id {
        Some(id) if !id.is_empty() && id != "null" => id.to_string(),
        _ => derive_mistral_tool_call_id(&format!("toolcall:{index}"), 0),
    };
    let key = format!("{call_id}:{index}");

    let function = tool_call.get("function");
    let name = function
        .and_then(|f| f.get("name"))
        .and_then(|n| n.as_str())
        .unwrap_or("")
        .to_string();

    let content_index = match tool_blocks_by_key.get(&key) {
        Some(&idx)
            if matches!(
                output.content.get(idx),
                Some(AssistantContentBlock::ToolCall(_))
            ) =>
        {
            idx
        }
        _ => {
            output
                .content
                .push(AssistantContentBlock::ToolCall(ToolCall {
                    id: call_id.clone(),
                    name,
                    arguments: Value::Object(Default::default()),
                    thought_signature: None,
                }));
            let idx = output.content.len() - 1;
            tool_blocks_by_key.insert(key.clone(), idx);
            tool_partial_args.insert(idx, String::new());
            sender.push(AssistantMessageEvent::ToolCallStart {
                content_index: idx,
                partial: output.clone(),
            });
            idx
        }
    };

    // argsDelta: string args verbatim, else JSON.stringify(args || {}).
    let args_delta = match function.and_then(|f| f.get("arguments")) {
        Some(Value::String(s)) => s.clone(),
        Some(other) if !other.is_null() => {
            serde_json::to_string(other).unwrap_or_else(|_| "{}".to_string())
        }
        _ => "{}".to_string(),
    };

    let buf = tool_partial_args.entry(content_index).or_default();
    buf.push_str(&args_delta);
    let parsed = parse_streaming_json(Some(buf.as_str()));
    if let Some(AssistantContentBlock::ToolCall(tc)) = output.content.get_mut(content_index) {
        tc.arguments = parsed;
    }
    sender.push(AssistantMessageEvent::ToolCallDelta {
        content_index,
        delta: args_delta,
        partial: output.clone(),
    });
}

/// Emit the appropriate `*_end` event for a closing text/thinking block.
fn finish_current_block(
    kind: CurrentBlockKind,
    output: &AssistantMessage,
    sender: &mut AssistantMessageEventStreamSender,
) {
    let idx = output.content.len().saturating_sub(1);
    match kind {
        CurrentBlockKind::Text => {
            let content = match output.content.get(idx) {
                Some(AssistantContentBlock::Text(tc)) => tc.text.clone(),
                _ => String::new(),
            };
            sender.push(AssistantMessageEvent::TextEnd {
                content_index: idx,
                content,
                partial: output.clone(),
            });
        }
        CurrentBlockKind::Thinking => {
            let content = match output.content.get(idx) {
                Some(AssistantContentBlock::Thinking(tc)) => tc.thinking.clone(),
                _ => String::new(),
            };
            sender.push(AssistantMessageEvent::ThinkingEnd {
                content_index: idx,
                content,
                partial: output.clone(),
            });
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Api, ModelCost, ThinkingLevel};
    use std::collections::HashMap as Map;

    fn base_model() -> Model {
        Model {
            id: "mistral-small-2603".into(),
            name: "Mistral Small".into(),
            api: Api::MistralConversations,
            provider: "mistral".into(),
            base_url: "https://api.mistral.ai".into(),
            reasoning: true,
            thinking_level_map: None,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.0,
                output: 0.0,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 128_000,
            max_tokens: 8192,
            headers: None,
            compat: None,
        }
    }

    fn user_context() -> Context {
        Context {
            system_prompt: None,
            messages: vec![Message::User(crate::types::UserMessage {
                role: MessageRole::User,
                content: vec![MessageContent::Text(TextContent {
                    text: "Hello".into(),
                    text_signature: None,
                })],
                timestamp: chrono::Utc::now(),
            })],
            tools: Vec::new(),
        }
    }

    fn simple_opts(reasoning: Option<ThinkingLevel>) -> SimpleStreamOptions {
        let mut base = StreamOptions::default();
        base.api_key = Some("fake-key".into());
        SimpleStreamOptions {
            base,
            reasoning,
            thinking_budgets: None,
        }
    }

    /// Drive `stream_simple`, capturing the payload via the `on_payload` hook
    /// without performing a real network request.
    async fn capture_payload(model: Model, opts: SimpleStreamOptions) -> Value {
        use std::sync::{Arc, Mutex};
        let captured: Arc<Mutex<Option<Value>>> = Arc::new(Mutex::new(None));
        let captured_cb = Arc::clone(&captured);

        let mut opts = opts;
        opts.base.api_key = Some("fake-key".into());
        opts.base.on_payload = Some(Arc::new(move |payload: Value, _model: Model| {
            let slot = Arc::clone(&captured_cb);
            Box::pin(async move {
                *slot.lock().unwrap_or_else(|e| e.into_inner()) = Some(payload.clone());
                Some(payload)
            })
        }));

        let mut model = model;
        // Point at an unroutable host so the request fails fast after capture.
        model.base_url = "http://127.0.0.1:9".into();

        let stream = stream_simple(model, user_context(), Some(opts));
        let _ = stream.result().await;

        let value = captured.lock().unwrap_or_else(|e| e.into_inner()).clone();
        value.expect("payload captured before request failure")
    }

    // --- reasoning-mode tests (port of mistral-reasoning-mode.test.ts) ---

    #[tokio::test]
    async fn reasoning_effort_for_small_4() {
        let model = base_model(); // mistral-small-2603
        let payload = capture_payload(model, simple_opts(Some(ThinkingLevel::Medium))).await;
        assert_eq!(
            payload.get("reasoningEffort").and_then(|v| v.as_str()),
            Some("high")
        );
        assert!(payload.get("promptMode").is_none());
    }

    #[tokio::test]
    async fn no_reasoning_controls_when_off_small_4() {
        let model = base_model();
        let payload = capture_payload(model, simple_opts(None)).await;
        assert!(payload.get("reasoningEffort").is_none());
        assert!(payload.get("promptMode").is_none());
    }

    #[tokio::test]
    async fn prompt_mode_for_magistral() {
        let mut model = base_model();
        model.id = "magistral-medium-latest".into(); // not in usesReasoningEffort set
        let payload = capture_payload(model, simple_opts(Some(ThinkingLevel::Medium))).await;
        assert_eq!(
            payload.get("promptMode").and_then(|v| v.as_str()),
            Some("reasoning")
        );
        assert!(payload.get("reasoningEffort").is_none());
    }

    #[tokio::test]
    async fn reasoning_effort_for_medium_3_5() {
        let mut model = base_model();
        model.id = "mistral-medium-3.5".into();
        let payload = capture_payload(model, simple_opts(Some(ThinkingLevel::Medium))).await;
        assert_eq!(
            payload.get("reasoningEffort").and_then(|v| v.as_str()),
            Some("high")
        );
        assert!(payload.get("promptMode").is_none());
    }

    // --- tool-schema test (port of mistral-tool-schema.test.ts) ---

    #[tokio::test]
    async fn tool_schema_serialized_as_function_tools() {
        use std::sync::{Arc, Mutex};

        let mut model = base_model();
        model.id = "devstral-medium-latest".into();
        model.reasoning = false;

        let parameters = serde_json::json!({
            "type": "object",
            "properties": {
                "nested": {
                    "type": "object",
                    "properties": { "value": { "type": "string" } }
                }
            }
        });
        let mut ctx = user_context();
        ctx.tools = vec![Tool {
            name: "inspect_schema".into(),
            description: "Inspect the schema".into(),
            parameters: parameters.clone(),
        }];

        let captured: Arc<Mutex<Option<Value>>> = Arc::new(Mutex::new(None));
        let captured_cb = Arc::clone(&captured);

        let mut base = StreamOptions::default();
        base.api_key = Some("fake-key".into());
        base.on_payload = Some(Arc::new(move |payload: Value, _m: Model| {
            let slot = Arc::clone(&captured_cb);
            Box::pin(async move {
                *slot.lock().unwrap_or_else(|e| e.into_inner()) = Some(payload.clone());
                Some(payload)
            })
        }));
        model.base_url = "http://127.0.0.1:9".into();

        let opts = MistralOptions {
            base,
            tool_choice: None,
            prompt_mode_reasoning: false,
            reasoning_effort: None,
        };

        let stream = stream(model, ctx, Some(opts));
        let result = stream.result().await;

        let payload = captured
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
            .expect("payload captured");
        let tools = payload
            .get("tools")
            .and_then(|t| t.as_array())
            .expect("tools array");
        assert_eq!(tools.len(), 1);
        let func = &tools[0]["function"];
        assert_eq!(func["name"], "inspect_schema");
        assert_eq!(func["parameters"], parameters);
        assert_eq!(func["strict"], serde_json::json!(false));
        assert_eq!(tools[0]["type"], "function");

        // The request to an unroutable host fails → error stop reason, but the
        // failure is a connection error (never an input-validation error).
        assert_eq!(result.stop_reason, StopReason::Error);
        let err = result.error_message.unwrap_or_default();
        assert!(!err.contains("Input validation failed"), "got {err}");
    }

    // --- pure-helper tests ---

    #[test]
    fn derive_id_keeps_exact_length_alnum() {
        // 9-char alnum input is returned unchanged at attempt 0.
        assert_eq!(derive_mistral_tool_call_id("abc123xyz", 0), "abc123xyz");
    }

    #[test]
    fn derive_id_hashes_non_conforming() {
        let id = derive_mistral_tool_call_id("call_with_underscores!", 0);
        assert_eq!(id.len(), MISTRAL_TOOL_CALL_ID_LENGTH);
        assert!(id.chars().all(|c| c.is_ascii_alphanumeric()));
    }

    #[test]
    fn normalizer_is_stable_and_unique() {
        let n = MistralToolCallIdNormalizer::new();
        let a = n.normalize("toolcall-A");
        let b = n.normalize("toolcall-A");
        assert_eq!(a, b);
        let c = n.normalize("toolcall-B");
        assert_ne!(a, c);
    }

    #[test]
    fn map_reasoning_effort_defaults_high() {
        let model = base_model();
        assert_eq!(
            map_reasoning_effort(&model, ModelThinkingLevel::Medium),
            MistralReasoningEffort::High
        );
    }

    #[test]
    fn map_reasoning_effort_respects_none_mapping() {
        let mut model = base_model();
        let mut map: Map<ModelThinkingLevel, Option<String>> = Map::new();
        map.insert(ModelThinkingLevel::Low, Some("none".into()));
        model.thinking_level_map = Some(map);
        assert_eq!(
            map_reasoning_effort(&model, ModelThinkingLevel::Low),
            MistralReasoningEffort::None
        );
    }

    #[test]
    fn stop_reason_mapping() {
        assert_eq!(map_chat_stop_reason(None), StopReason::Stop);
        assert_eq!(map_chat_stop_reason(Some("stop")), StopReason::Stop);
        assert_eq!(map_chat_stop_reason(Some("length")), StopReason::Length);
        assert_eq!(
            map_chat_stop_reason(Some("model_length")),
            StopReason::Length
        );
        assert_eq!(
            map_chat_stop_reason(Some("tool_calls")),
            StopReason::ToolUse
        );
        assert_eq!(map_chat_stop_reason(Some("error")), StopReason::Error);
        assert_eq!(map_chat_stop_reason(Some("weird")), StopReason::Stop);
    }

    #[test]
    fn url_appends_v1_chat_completions() {
        let mut m = base_model();
        m.base_url = "https://api.mistral.ai".into();
        assert_eq!(build_url(&m), "https://api.mistral.ai/v1/chat/completions");
        m.base_url = "https://api.mistral.ai/v1".into();
        assert_eq!(build_url(&m), "https://api.mistral.ai/v1/chat/completions");
    }

    #[test]
    fn tool_result_text_variants() {
        assert_eq!(build_tool_result_text("ok", false, false, false), "ok");
        assert_eq!(
            build_tool_result_text("ok", false, false, true),
            "[tool error] ok"
        );
        assert_eq!(
            build_tool_result_text("", false, false, false),
            "(no tool output)"
        );
        assert_eq!(
            build_tool_result_text("", true, false, false),
            "(image omitted: model does not support images)"
        );
        assert_eq!(
            build_tool_result_text("", true, true, false),
            "(see attached image)"
        );
        assert_eq!(
            build_tool_result_text("", true, true, true),
            "[tool error] (see attached image)"
        );
        assert_eq!(
            build_tool_result_text("", true, false, true),
            "[tool error] (image omitted: model does not support images)"
        );
        assert_eq!(
            build_tool_result_text("", false, false, true),
            "[tool error] (no tool output)"
        );
        assert_eq!(build_tool_result_text("text", true, true, false), "text");
    }

    // -----------------------------------------------------------------------
    // Reasoning payload tests (port of mistral-reasoning-mode.test.ts)
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn no_reasoning_controls_when_off_medium_3_5() {
        let mut model = base_model();
        model.id = "mistral-medium-3.5".into();
        let payload = capture_payload(model, simple_opts(None)).await;
        assert!(payload.get("reasoningEffort").is_none());
        assert!(payload.get("promptMode").is_none());
    }

    // -----------------------------------------------------------------------
    // Error formatting tests
    // -----------------------------------------------------------------------

    #[test]
    fn format_mistral_http_error_with_body() {
        let err = format_mistral_http_error(400, "Bad request");
        assert_eq!(err, "Mistral API error (400): Bad request");
    }

    #[test]
    fn format_mistral_http_error_without_body() {
        let err = format_mistral_http_error(500, "");
        assert_eq!(err, "Mistral API error (500)");
    }

    #[test]
    fn format_mistral_http_error_truncates_long_body() {
        let long = "x".repeat(5000);
        let err = format_mistral_http_error(400, &long);
        assert!(err.len() < long.len() + 100);
        assert!(err.contains("[truncated"));
    }

    // -----------------------------------------------------------------------
    // Header tests
    // -----------------------------------------------------------------------

    #[test]
    fn build_request_headers_includes_x_affinity_from_session_id() {
        let model = base_model();
        let opts = MistralOptions {
            base: StreamOptions {
                session_id: Some("sess-123".into()),
                ..Default::default()
            },
            ..Default::default()
        };
        let headers = build_request_headers(&model, &opts);
        assert_eq!(headers.get("x-affinity").unwrap(), "sess-123");
    }

    #[test]
    fn build_request_headers_model_and_option_headers_merged() {
        let model = Model {
            headers: Some(std::collections::HashMap::from([(
                "x-model".into(),
                "model-val".into(),
            )])),
            ..base_model()
        };
        let opts = MistralOptions {
            base: StreamOptions {
                headers: Some(std::collections::HashMap::from([(
                    "x-option".into(),
                    "opt-val".into(),
                )])),
                ..Default::default()
            },
            ..Default::default()
        };
        let headers = build_request_headers(&model, &opts);
        assert_eq!(headers.get("x-model").unwrap(), "model-val");
        assert_eq!(headers.get("x-option").unwrap(), "opt-val");
    }

    #[test]
    fn build_request_headers_option_wins_over_model() {
        let model = Model {
            headers: Some(std::collections::HashMap::from([(
                "x-override".into(),
                "model-val".into(),
            )])),
            ..base_model()
        };
        let opts = MistralOptions {
            base: StreamOptions {
                headers: Some(std::collections::HashMap::from([(
                    "x-override".into(),
                    "option-val".into(),
                )])),
                ..Default::default()
            },
            ..Default::default()
        };
        let headers = build_request_headers(&model, &opts);
        assert_eq!(headers.get("x-override").unwrap(), "option-val");
    }

    // -----------------------------------------------------------------------
    // URL building tests
    // -----------------------------------------------------------------------

    #[test]
    fn url_builds_default_when_empty_base() {
        let mut m = base_model();
        m.base_url = "".into();
        assert_eq!(build_url(&m), "https://api.mistral.ai/v1/chat/completions");
    }

    #[test]
    fn url_builds_from_custom_base() {
        let mut m = base_model();
        m.base_url = "https://custom.example.com".into();
        assert_eq!(
            build_url(&m),
            "https://custom.example.com/v1/chat/completions"
        );
    }

    #[test]
    fn url_builds_strips_trailing_slash() {
        let mut m = base_model();
        m.base_url = "https://api.mistral.ai/".into();
        assert_eq!(build_url(&m), "https://api.mistral.ai/v1/chat/completions");
    }

    // -----------------------------------------------------------------------
    // Usage field tests
    // -----------------------------------------------------------------------

    #[test]
    fn usage_field_finds_camel_case() {
        let usage = serde_json::json!({"promptTokens": 10, "completionTokens": 20});
        assert_eq!(usage_field(&usage, &["promptTokens", "prompt_tokens"]), 10);
        assert_eq!(
            usage_field(&usage, &["completionTokens", "completion_tokens"]),
            20
        );
    }

    #[test]
    fn usage_field_finds_snake_case() {
        let usage = serde_json::json!({"prompt_tokens": 15, "completion_tokens": 25});
        assert_eq!(usage_field(&usage, &["promptTokens", "prompt_tokens"]), 15);
        assert_eq!(
            usage_field(&usage, &["completionTokens", "completion_tokens"]),
            25
        );
    }

    #[test]
    fn usage_field_returns_zero_for_missing() {
        let usage = serde_json::json!({});
        assert_eq!(usage_field(&usage, &["promptTokens", "prompt_tokens"]), 0);
    }

    // -----------------------------------------------------------------------
    // Strip symbol keys test (identity function port)
    // -----------------------------------------------------------------------

    #[test]
    fn strip_symbol_keys_is_identity() {
        let value = serde_json::json!({"a": 1, "b": [2, 3]});
        assert_eq!(strip_symbol_keys(&value), value);
    }

    // -----------------------------------------------------------------------
    // To function tools test
    // -----------------------------------------------------------------------

    #[test]
    fn to_function_tools_produces_correct_schema() {
        let tools = vec![Tool {
            name: "get_weather".into(),
            description: "Get weather".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": { "city": { "type": "string" } }
            }),
        }];
        let result = to_function_tools(&tools);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0]["type"], "function");
        assert_eq!(result[0]["function"]["name"], "get_weather");
        assert_eq!(result[0]["function"]["strict"], false);
    }

    // -----------------------------------------------------------------------
    // Tool choice mapping tests
    // -----------------------------------------------------------------------

    #[test]
    fn map_tool_choice_variants() {
        assert_eq!(map_tool_choice(&MistralToolChoice::Auto), "auto");
        assert_eq!(map_tool_choice(&MistralToolChoice::None), "none");
        assert_eq!(map_tool_choice(&MistralToolChoice::Any), "any");
        assert_eq!(map_tool_choice(&MistralToolChoice::Required), "required");
        let func_choice = map_tool_choice(&MistralToolChoice::Function {
            name: "my_func".into(),
        });
        assert_eq!(func_choice["type"], "function");
        assert_eq!(func_choice["function"]["name"], "my_func");
    }

    // -----------------------------------------------------------------------
    // Truncate error text tests
    // -----------------------------------------------------------------------

    #[test]
    fn truncate_error_text_short() {
        assert_eq!(truncate_error_text("short", 100), "short");
    }

    #[test]
    fn truncate_error_text_long() {
        let long = "a".repeat(5000);
        let result = truncate_error_text(&long, 100);
        assert!(result.len() > 100); // head + "... [truncated X chars]"
        assert!(result.starts_with(&"a".repeat(100)));
        assert!(result.contains("[truncated"));
    }

    // -----------------------------------------------------------------------
    // Uses reasoning effort / uses prompt mode reasoning tests
    // -----------------------------------------------------------------------

    #[test]
    fn uses_reasoning_effort_for_small_4() {
        let mut m = base_model();
        m.id = "mistral-small-2603".into();
        assert!(uses_reasoning_effort(&m));
    }

    #[test]
    fn uses_reasoning_effort_for_medium_3_5() {
        let mut m = base_model();
        m.id = "mistral-medium-3.5".into();
        assert!(uses_reasoning_effort(&m));
    }

    #[test]
    fn uses_prompt_mode_reasoning_for_magistral() {
        let mut m = base_model();
        m.id = "magistral-medium-latest".into();
        m.reasoning = true;
        assert!(uses_prompt_mode_reasoning(&m));
    }

    #[test]
    fn does_not_use_prompt_mode_when_model_has_no_reasoning() {
        let mut m = base_model();
        m.reasoning = false;
        assert!(!uses_prompt_mode_reasoning(&m));
    }

    // -----------------------------------------------------------------------
    // Chat messages conversion test
    // -----------------------------------------------------------------------

    #[test]
    fn to_chat_messages_skips_empty_assistant_content() {
        let messages = vec![Message::Assistant(AssistantMessage {
            role: MessageRole::Assistant,
            content: vec![],
            api: "mistral-conversations".into(),
            provider: "mistral".into(),
            model: "test".into(),
            response_model: None,
            response_id: None,
            usage: crate::types::Usage {
                input: 0,
                output: 0,
                cache_read: 0,
                cache_write: 0,
                cache_write_1h: None,
                total_tokens: 0,
                cost: crate::types::UsageCost {
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
            timestamp: chrono::Utc::now(),
        })];
        let result = to_chat_messages(&messages, false);
        assert!(result.is_empty());
    }
}
