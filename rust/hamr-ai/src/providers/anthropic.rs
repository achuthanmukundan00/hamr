//! Port of `../../packages/ai/src/providers/anthropic.ts`.
//!
//! Anthropic Messages API provider backend.
//! Uses `reqwest` for HTTP + SSE streaming (no Anthropic Rust SDK).
//!
//! # Entry points
//!
//! - [`stream_anthropic`] — full provider stream with [`AnthropicOptions`].
//! - [`stream_simple_anthropic`] — unified [`SimpleStreamOptions`] wrapper.
//!
//! # Key subsystems
//!
//! 1. **SSE streaming** — byte-level SSE parsing from `POST /v1/messages` with
//!    `stream: true`. Parses `message_start`, `content_block_start/delta/stop`,
//!    `message_delta`, `message_stop`.
//! 2. **Extended thinking** — adaptive (`{type:"adaptive", display}`) for modern
//!    models, budget-based (`{type:"enabled", budget_tokens:N}`) for older ones.
//! 3. **Prompt caching** — `cache_control: { type:"ephemeral" }` on system prompt,
//!    last tool, last user message content block.
//! 4. **Claude Code stealth** — OAuth tokens trigger CC identity headers, tool
//!    name canonicalization.
//! 5. **Special providers** — Cloudflare AI Gateway (`cf-aig-authorization`),
//!    GitHub Copilot (Bearer + dynamic headers), Fireworks (no cache control).

use std::collections::HashMap;

use serde::Deserialize;
use serde_json::Value;

use crate::models::calculate_cost;
use crate::providers::cloudflare::resolve_cloudflare_base_url;
use crate::providers::github_copilot_headers::{
    build_copilot_dynamic_headers, has_copilot_vision_input,
};
use crate::providers::simple_options::{adjust_max_tokens_for_thinking, build_base_options};
use crate::providers::transform_messages::transform_messages;
use crate::types::{
    AssistantContentBlock, AssistantMessage, AssistantMessageEvent, CacheRetention, Context,
    DoneReason, ErrorReason, Message, MessageContent, MessageRole, Model, ProviderEnv,
    ProviderResponse, SimpleStreamOptions, StopReason, StreamOptions, TextContent, ThinkingContent,
    ThinkingLevel, Tool, ToolCall, Usage, UsageCost,
};
use crate::utils::event_stream::{
    AssistantMessageEventStream, AssistantMessageEventStreamSender,
    create_assistant_message_event_stream,
};
use crate::utils::headers::headers_to_record;
use crate::utils::json_parse::parse_streaming_json;
use crate::utils::provider_env::get_provider_env_value;
use crate::utils::sanitize_unicode::sanitize_surrogates;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const CLAUDE_CODE_VERSION: &str = "2.1.75";

/// Claude Code 2.x canonical tool names.
const CLAUDE_CODE_TOOLS: &[&str] = &[
    "Read",
    "Write",
    "Edit",
    "Bash",
    "Grep",
    "Glob",
    "AskUserQuestion",
    "EnterPlanMode",
    "ExitPlanMode",
    "KillShell",
    "NotebookEdit",
    "Skill",
    "Task",
    "TaskOutput",
    "TodoWrite",
    "WebFetch",
    "WebSearch",
];

const FINE_GRAINED_TOOL_STREAMING_BETA: &str = "fine-grained-tool-streaming-2025-05-14";
const INTERLEAVED_THINKING_BETA: &str = "interleaved-thinking-2025-05-14";

// ---------------------------------------------------------------------------
// Option types
// ---------------------------------------------------------------------------

/// Effort level for adaptive thinking models.
pub type AnthropicEffort = String;

/// How thinking content is displayed.
pub type AnthropicThinkingDisplay = String;

/// Tool choice for Anthropic API.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolChoiceTool {
    #[serde(rename = "type")]
    pub choice_type: String,
    pub name: String,
}

/// Cache retention preferences.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CacheControl {
    #[serde(rename = "type")]
    pub cache_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ttl: Option<String>,
}

/// Anthropic-specific stream options.
///
/// Mirrors the TS `interface AnthropicOptions extends StreamOptions`.
#[derive(Clone, Debug)]
pub struct AnthropicOptions {
    pub base: StreamOptions,
    /// Enable extended thinking.
    pub thinking_enabled: Option<bool>,
    /// Token budget for extended thinking (older models only).
    pub thinking_budget_tokens: Option<u32>,
    /// Effort level for adaptive thinking models.
    pub effort: Option<String>,
    /// How thinking content is returned.
    pub thinking_display: Option<String>,
    /// Whether to request interleaved thinking beta (default: true).
    pub interleaved_thinking: Option<bool>,
    /// Tool choice behavior.
    pub tool_choice: Option<serde_json::Value>,
    /// Cache retention preference.
    pub cache_retention: Option<CacheRetention>,
}

impl Default for AnthropicOptions {
    fn default() -> Self {
        Self {
            base: StreamOptions::default(),
            thinking_enabled: None,
            thinking_budget_tokens: None,
            effort: None,
            thinking_display: None,
            interleaved_thinking: None,
            tool_choice: None,
            cache_retention: None,
        }
    }
}

// ---------------------------------------------------------------------------
// SSE event types (Anthropic wire format)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
struct AnthropicMessageStart {
    #[allow(dead_code)]
    #[serde(rename = "type")]
    event_type: String,
    message: AnthropicMessage,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AnthropicMessage {
    id: String,
    #[serde(default)]
    #[serde(rename = "type")]
    #[allow(dead_code)]
    msg_type: String,
    usage: AnthropicUsage,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AnthropicUsage {
    #[serde(default)]
    input_tokens: u64,
    #[serde(default)]
    output_tokens: u64,
    #[serde(default)]
    cache_read_input_tokens: u64,
    #[serde(default)]
    cache_creation_input_tokens: u64,
    #[serde(default)]
    cache_creation: Option<AnthropicCacheCreation>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AnthropicCacheCreation {
    #[serde(default)]
    ephemeral_1h_input_tokens: u64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AnthropicContentBlockStart {
    #[allow(dead_code)]
    #[serde(rename = "type")]
    event_type: String,
    index: u64,
    content_block: AnthropicContentBlock,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
enum AnthropicContentBlock {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "thinking")]
    Thinking {
        thinking: String,
        #[serde(default)]
        signature: String,
    },
    #[serde(rename = "redacted_thinking")]
    RedactedThinking { data: String },
    #[serde(rename = "tool_use")]
    ToolUse {
        id: String,
        name: String,
        input: Value,
    },
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AnthropicContentBlockDelta {
    #[allow(dead_code)]
    #[serde(rename = "type")]
    event_type: String,
    index: u64,
    delta: AnthropicDelta,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
enum AnthropicDelta {
    #[serde(rename = "text_delta")]
    TextDelta { text: String },
    #[serde(rename = "thinking_delta")]
    ThinkingDelta { thinking: String },
    #[serde(rename = "input_json_delta")]
    InputJsonDelta {
        #[serde(rename = "partial_json")]
        partial_json: String,
    },
    #[serde(rename = "signature_delta")]
    SignatureDelta { signature: String },
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AnthropicContentBlockStop {
    #[allow(dead_code)]
    #[serde(rename = "type")]
    event_type: String,
    index: u64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AnthropicMessageDelta {
    #[allow(dead_code)]
    #[serde(rename = "type")]
    event_type: String,
    delta: AnthropicMessageDeltaInner,
    usage: AnthropicUsage,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AnthropicMessageDeltaInner {
    stop_reason: Option<String>,
    stop_details: Option<Value>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
struct AnthropicMessageStop {
    #[allow(dead_code)]
    #[serde(rename = "type")]
    event_type: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AnthropicStreamError {
    #[allow(dead_code)]
    #[serde(rename = "type")]
    event_type: String,
    error: AnthropicErrorBody,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AnthropicErrorBody {
    #[serde(rename = "type")]
    #[allow(dead_code)]
    error_type: String,
    message: String,
}

#[derive(Debug, Clone, Deserialize)]
struct AnthropicApiError {
    error: Option<AnthropicApiErrorBody>,
}

#[derive(Debug, Clone, Deserialize)]
struct AnthropicApiErrorBody {
    message: Option<String>,
    #[serde(rename = "type")]
    #[allow(dead_code)]
    error_type: Option<String>,
}

// ---------------------------------------------------------------------------
// Helper: Claude Code tool name mapping
// ---------------------------------------------------------------------------

fn build_cc_tool_lookup() -> HashMap<String, &'static str> {
    let mut m = HashMap::new();
    for &t in CLAUDE_CODE_TOOLS {
        m.insert(t.to_lowercase(), t);
    }
    m
}

fn to_claude_code_name(name: &str) -> String {
    let lookup = build_cc_tool_lookup();
    lookup
        .get(&name.to_lowercase())
        .copied()
        .unwrap_or(name)
        .to_string()
}

fn from_claude_code_name(name: &str, tools: &[Tool]) -> String {
    let lower = name.to_lowercase();
    for tool in tools {
        if tool.name.to_lowercase() == lower {
            return tool.name.clone();
        }
    }
    name.to_string()
}

// ---------------------------------------------------------------------------
// Helper: resolve cache retention
// ---------------------------------------------------------------------------

fn resolve_cache_retention(
    cache_retention: Option<CacheRetention>,
    env: Option<&ProviderEnv>,
) -> CacheRetention {
    if let Some(cr) = cache_retention {
        return cr;
    }
    if get_provider_env_value("PI_CACHE_RETENTION", env).as_deref() == Some("long") {
        return CacheRetention::Long;
    }
    CacheRetention::Short
}

// ---------------------------------------------------------------------------
// Helper: get cache control
// ---------------------------------------------------------------------------

fn get_cache_control(
    model: &Model,
    cache_retention: Option<CacheRetention>,
    env: Option<&ProviderEnv>,
) -> (CacheRetention, Option<CacheControl>) {
    let retention = resolve_cache_retention(cache_retention, env);
    if retention == CacheRetention::None {
        return (retention, None);
    }
    let supports_long = get_anthropic_compat(model).supports_long_cache_retention;
    let ttl = if retention == CacheRetention::Long && supports_long {
        Some("1h".to_string())
    } else {
        None
    };
    (
        retention,
        Some(CacheControl {
            cache_type: "ephemeral".to_string(),
            ttl,
        }),
    )
}

// ---------------------------------------------------------------------------
// Helper: get Anthropic compat (auto-detect)
// ---------------------------------------------------------------------------

/// Resolved compatibility settings for an Anthropic-compatible model.
///
/// Mirrors the TS `getAnthropicCompat()` + per-model `compat` override
/// auto-detection from `model.id` and `model.provider`.
#[derive(Clone, Debug)]
struct AnthropicCompat {
    /// Whether adaptive thinking is used (model decides when/how much to think).
    force_adaptive_thinking: bool,
    supports_eager_tool_input_streaming: bool,
    supports_long_cache_retention: bool,
    send_session_affinity_headers: bool,
    supports_cache_control_on_tools: bool,
    supports_temperature: bool,
    allow_empty_signature: bool,
}

/// Detect whether a model requires adaptive thinking based on its ID.
///
/// Mirrors the TS `model.compat?.forceAdaptiveThinking`.
/// Models that have native adaptive thinking built in:
/// - claude-fable-5
/// - claude-opus-4-6 / claude-opus-4.6
/// - claude-opus-4-7 / claude-opus-4.7
/// - claude-opus-4-8 / claude-opus-4.8
/// - claude-sonnet-4-6
fn is_adaptive_thinking_model(model_id: &str) -> bool {
    // Normalise dots to dashes for matching
    let normalised = model_id.replace('.', "-");
    normalised.contains("claude-fable-5")
        || normalised.contains("claude-opus-4-6")
        || normalised.contains("claude-opus-4-7")
        || normalised.contains("claude-opus-4-8")
        || normalised.contains("claude-sonnet-4-6")
}

/// Detect whether a model supports the Anthropic `temperature` field.
///
/// Mirrors the TS `model.compat?.supportsTemperature`.
/// Claude Opus 4.7+ rejects non-default temperature values.
fn model_supports_temperature(model_id: &str) -> bool {
    let normalised = model_id.replace('.', "-");
    !(normalised.contains("claude-opus-4-7") || normalised.contains("claude-opus-4-8"))
}

fn get_anthropic_compat(model: &Model) -> AnthropicCompat {
    let is_fireworks = model.provider == "fireworks";
    let is_cloudflare_ai_gateway_anthropic =
        model.provider == "cloudflare-ai-gateway" && model.base_url.contains("anthropic");
    AnthropicCompat {
        force_adaptive_thinking: is_adaptive_thinking_model(&model.id),
        supports_eager_tool_input_streaming: !is_fireworks,
        supports_long_cache_retention: !is_fireworks,
        send_session_affinity_headers: is_fireworks || is_cloudflare_ai_gateway_anthropic,
        supports_cache_control_on_tools: !is_fireworks,
        supports_temperature: model_supports_temperature(&model.id),
        allow_empty_signature: false,
    }
}

// ---------------------------------------------------------------------------
// Helper: OAuth token detection
// ---------------------------------------------------------------------------

fn is_oauth_token(api_key: &str) -> bool {
    api_key.contains("sk-ant-oat")
}

// ---------------------------------------------------------------------------
// Helper: should use fine-grained tool streaming beta
// ---------------------------------------------------------------------------

fn should_use_fine_grained_tool_streaming_beta(model: &Model, context: &Context) -> bool {
    !context.tools.is_empty() && !get_anthropic_compat(model).supports_eager_tool_input_streaming
}

// ---------------------------------------------------------------------------
// Initial output message
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

fn initial_output(model: &Model) -> AssistantMessage {
    AssistantMessage {
        role: MessageRole::Assistant,
        content: Vec::new(),
        api: "anthropic-messages".to_string(),
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
// Error parsing
// ---------------------------------------------------------------------------

fn parse_anthropic_error(status: u16, body_text: &str) -> String {
    let detail = body_text.trim();
    if let Ok(parsed) = serde_json::from_str::<AnthropicApiError>(body_text) {
        if let Some(err) = parsed.error {
            if let Some(msg) = err.message {
                if !msg.trim().is_empty() {
                    return format!("{} {}", status, msg);
                }
            }
        }
    }
    if detail.is_empty() {
        format!("Provider error ({})", status)
    } else {
        format!("{} {}", status, detail)
    }
}

// ---------------------------------------------------------------------------
// Stop reason mapping
// ---------------------------------------------------------------------------

fn map_stop_reason(reason: &str, _stop_details: Option<&Value>) -> (StopReason, Option<String>) {
    match reason {
        "end_turn" | "pause_turn" | "stop_sequence" => (StopReason::Stop, None),
        "max_tokens" => (StopReason::Length, None),
        "tool_use" => (StopReason::ToolUse, None),
        "refusal" => {
            let msg = "The model refused to complete the request".to_string();
            (StopReason::Error, Some(msg))
        }
        "sensitive" => (StopReason::Error, None),
        other => (
            StopReason::Error,
            Some(format!("Unhandled stop reason: {}", other)),
        ),
    }
}

// ---------------------------------------------------------------------------
// Wait for abort signal
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// SSE streaming (Anthropic wire format)
// ---------------------------------------------------------------------------

/// Process the SSE body from an Anthropic streaming response.
async fn process_anthropic_sse_stream(
    response: reqwest::Response,
    context_tools: &[Tool],
    model: &Model,
    mut output: AssistantMessage,
    options: &AnthropicOptions,
    sender: &mut AssistantMessageEventStreamSender,
) -> Result<AssistantMessage, String> {
    sender.push(AssistantMessageEvent::Start {
        partial: output.clone(),
    });

    // Block tracking: maps Anthropic content block index → output.content index
    let mut block_index_map: HashMap<u64, usize> = HashMap::new();

    // Store partial_json for tool calls that need accumulation
    // maps output.content index → accumulated partial JSON string
    let mut partial_json_scratch: HashMap<usize, String> = HashMap::new();

    // Signature accumulator for thinking blocks
    // maps output.content index → accumulated signature
    let mut thinking_signature_scratch: HashMap<usize, String> = HashMap::new();

    // Track whether saw message_stop
    let mut saw_message_start = false;
    let mut saw_message_stop = false;

    // SSE parsing state
    let mut sse_event: Option<String> = None;
    let mut sse_data: Vec<String> = Vec::new();

    let mut byte_stream = response.bytes_stream();
    let mut buffer: Vec<u8> = Vec::new();
    use futures::StreamExt;

    loop {
        // Abort check between chunks
        if let Some(signal) = &options.base.signal {
            if *signal.borrow() {
                return Err("Request was aborted".to_string());
            }
        }

        let next = if let Some(signal) = options.base.signal.clone() {
            let mut sig = signal;
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
            Some(Err(e)) => return Err(format!("Stream error: {}", e)),
            None => break,
        };

        buffer.extend_from_slice(&chunk);

        // Process complete lines
        while let Some(pos) = buffer.iter().position(|&b| b == b'\n') {
            let line_bytes: Vec<u8> = buffer.drain(..=pos).collect();
            let line = String::from_utf8_lossy(&line_bytes);
            let line = line.trim_end_matches(['\r', '\n']);

            if line.is_empty() {
                // Empty line = flush SSE event
                let event_type = sse_event.take();
                let data = sse_data.join("\n");
                sse_data.clear();

                if data.is_empty() && event_type.is_none() {
                    continue;
                }

                // Process the SSE event
                if let Some(etype) = event_type {
                    match etype.as_str() {
                        "error" => {
                            if let Ok(err_event) =
                                serde_json::from_str::<AnthropicStreamError>(&data)
                            {
                                return Err(format!(
                                    "Anthropic error: {}",
                                    err_event.error.message
                                ));
                            }
                            return Err(format!("Anthropic error: {}", data));
                        }
                        "message_start" => {
                            saw_message_start = true;
                            if let Ok(msg_start) =
                                serde_json::from_str::<AnthropicMessageStart>(&data)
                            {
                                output.response_id = Some(msg_start.message.id);
                                let usage = &msg_start.message.usage;
                                output.usage.input = usage.input_tokens;
                                output.usage.output = usage.output_tokens;
                                output.usage.cache_read = usage.cache_read_input_tokens;
                                output.usage.cache_write = usage.cache_creation_input_tokens;
                                if let Some(ref cc) = usage.cache_creation {
                                    output.usage.cache_write_1h =
                                        Some(cc.ephemeral_1h_input_tokens);
                                }
                                output.usage.total_tokens = output.usage.input
                                    + output.usage.output
                                    + output.usage.cache_read
                                    + output.usage.cache_write;
                                calculate_cost(model, &mut output.usage);
                            }
                        }
                        "message_delta" => {
                            if let Ok(msg_delta) =
                                serde_json::from_str::<AnthropicMessageDelta>(&data)
                            {
                                if let Some(ref stop_reason) = msg_delta.delta.stop_reason {
                                    let (sr, err_msg) = map_stop_reason(
                                        stop_reason,
                                        msg_delta.delta.stop_details.as_ref(),
                                    );
                                    output.stop_reason = sr;
                                    output.error_message = err_msg;
                                }
                                // Update usage fields (preserving input from message_start if null)
                                let usage = &msg_delta.usage;
                                if usage.input_tokens != 0 {
                                    output.usage.input = usage.input_tokens;
                                }
                                if usage.output_tokens != 0 {
                                    output.usage.output = usage.output_tokens;
                                }
                                if usage.cache_read_input_tokens != 0 {
                                    output.usage.cache_read = usage.cache_read_input_tokens;
                                }
                                if usage.cache_creation_input_tokens != 0 {
                                    output.usage.cache_write = usage.cache_creation_input_tokens;
                                }
                                output.usage.total_tokens = output.usage.input
                                    + output.usage.output
                                    + output.usage.cache_read
                                    + output.usage.cache_write;
                                calculate_cost(model, &mut output.usage);
                            }
                        }
                        "message_stop" => {
                            saw_message_stop = true;
                        }
                        "content_block_start" => {
                            if let Ok(block_start) =
                                serde_json::from_str::<AnthropicContentBlockStart>(&data)
                            {
                                let idx = block_start.index;
                                match block_start.content_block {
                                    AnthropicContentBlock::Text { text } => {
                                        let content_idx = output.content.len();
                                        output.content.push(AssistantContentBlock::Text(
                                            TextContent {
                                                text,
                                                text_signature: None,
                                            },
                                        ));
                                        block_index_map.insert(idx, content_idx);
                                        sender.push(AssistantMessageEvent::TextStart {
                                            content_index: content_idx,
                                            partial: output.clone(),
                                        });
                                    }
                                    AnthropicContentBlock::Thinking {
                                        thinking,
                                        signature,
                                    } => {
                                        let content_idx = output.content.len();
                                        output.content.push(AssistantContentBlock::Thinking(
                                            ThinkingContent {
                                                thinking,
                                                thinking_signature: Some(signature.clone()),
                                                redacted: false,
                                            },
                                        ));
                                        block_index_map.insert(idx, content_idx);
                                        if !signature.is_empty() {
                                            thinking_signature_scratch
                                                .insert(content_idx, signature);
                                        }
                                        sender.push(AssistantMessageEvent::ThinkingStart {
                                            content_index: content_idx,
                                            partial: output.clone(),
                                        });
                                    }
                                    AnthropicContentBlock::RedactedThinking { data } => {
                                        let content_idx = output.content.len();
                                        output.content.push(AssistantContentBlock::Thinking(
                                            ThinkingContent {
                                                thinking: "[Reasoning redacted]".to_string(),
                                                thinking_signature: Some(data.clone()),
                                                redacted: true,
                                            },
                                        ));
                                        block_index_map.insert(idx, content_idx);
                                        if !data.is_empty() {
                                            thinking_signature_scratch.insert(content_idx, data);
                                        }
                                        sender.push(AssistantMessageEvent::ThinkingStart {
                                            content_index: content_idx,
                                            partial: output.clone(),
                                        });
                                    }
                                    AnthropicContentBlock::ToolUse { id, name, input } => {
                                        let is_oauth = is_oauth_token(
                                            options.base.api_key.as_deref().unwrap_or(""),
                                        );
                                        let tool_name = if is_oauth {
                                            from_claude_code_name(&name, context_tools)
                                        } else {
                                            name
                                        };
                                        let content_idx = output.content.len();
                                        output.content.push(AssistantContentBlock::ToolCall(
                                            ToolCall {
                                                id,
                                                name: tool_name,
                                                arguments: input,
                                                thought_signature: None,
                                            },
                                        ));
                                        // Initialize partial JSON scratch
                                        partial_json_scratch.insert(content_idx, String::new());
                                        block_index_map.insert(idx, content_idx);
                                        sender.push(AssistantMessageEvent::ToolCallStart {
                                            content_index: content_idx,
                                            partial: output.clone(),
                                        });
                                    }
                                }
                            }
                        }
                        "content_block_delta" => {
                            if let Ok(block_delta) =
                                serde_json::from_str::<AnthropicContentBlockDelta>(&data)
                            {
                                let content_idx = match block_index_map.get(&block_delta.index) {
                                    Some(&idx) => idx,
                                    None => continue,
                                };

                                match block_delta.delta {
                                    AnthropicDelta::TextDelta { text } => {
                                        if let Some(AssistantContentBlock::Text(tc)) =
                                            output.content.get_mut(content_idx)
                                        {
                                            tc.text.push_str(&text);
                                        }
                                        sender.push(AssistantMessageEvent::TextDelta {
                                            content_index: content_idx,
                                            delta: text,
                                            partial: output.clone(),
                                        });
                                    }
                                    AnthropicDelta::ThinkingDelta { thinking } => {
                                        if let Some(AssistantContentBlock::Thinking(tc)) =
                                            output.content.get_mut(content_idx)
                                        {
                                            tc.thinking.push_str(&thinking);
                                        }
                                        sender.push(AssistantMessageEvent::ThinkingDelta {
                                            content_index: content_idx,
                                            delta: thinking,
                                            partial: output.clone(),
                                        });
                                    }
                                    AnthropicDelta::InputJsonDelta { partial_json } => {
                                        if let Some(AssistantContentBlock::ToolCall(tc)) =
                                            output.content.get_mut(content_idx)
                                        {
                                            let scratch = partial_json_scratch
                                                .entry(content_idx)
                                                .or_default();
                                            scratch.push_str(&partial_json);
                                            tc.arguments = parse_streaming_json(Some(scratch));
                                        }
                                        sender.push(AssistantMessageEvent::ToolCallDelta {
                                            content_index: content_idx,
                                            delta: partial_json,
                                            partial: output.clone(),
                                        });
                                    }
                                    AnthropicDelta::SignatureDelta { signature } => {
                                        if let Some(AssistantContentBlock::Thinking(tc)) =
                                            output.content.get_mut(content_idx)
                                        {
                                            let sig = thinking_signature_scratch
                                                .entry(content_idx)
                                                .or_default();
                                            sig.push_str(&signature);
                                            tc.thinking_signature = Some(sig.clone());
                                        }
                                    }
                                }
                            }
                        }
                        "content_block_stop" => {
                            if let Ok(block_stop) =
                                serde_json::from_str::<AnthropicContentBlockStop>(&data)
                            {
                                let content_idx = match block_index_map.get(&block_stop.index) {
                                    Some(&idx) => idx,
                                    None => continue,
                                };

                                match output.content.get(content_idx) {
                                    Some(AssistantContentBlock::Text(tc)) => {
                                        sender.push(AssistantMessageEvent::TextEnd {
                                            content_index: content_idx,
                                            content: tc.text.clone(),
                                            partial: output.clone(),
                                        });
                                    }
                                    Some(AssistantContentBlock::Thinking(tc)) => {
                                        sender.push(AssistantMessageEvent::ThinkingEnd {
                                            content_index: content_idx,
                                            content: tc.thinking.clone(),
                                            partial: output.clone(),
                                        });
                                    }
                                    Some(AssistantContentBlock::ToolCall(tc)) => {
                                        // Finalize: partial_json becomes the actual arguments
                                        // Strip the scratch buffer
                                        sender.push(AssistantMessageEvent::ToolCallEnd {
                                            content_index: content_idx,
                                            tool_call: ToolCall {
                                                id: tc.id.clone(),
                                                name: tc.name.clone(),
                                                arguments: tc.arguments.clone(),
                                                thought_signature: None,
                                            },
                                            partial: output.clone(),
                                        });
                                    }
                                    None => {}
                                }
                                // Clean up tracking
                                partial_json_scratch.remove(&content_idx);
                                thinking_signature_scratch.remove(&content_idx);
                            }
                        }
                        _ => {
                            // Unknown event, skip (but do log it for debugging)
                            // match the TS: only ANTHROPIC_MESSAGE_EVENTS are processed
                        }
                    }
                }
                continue;
            }

            // SSE comment line
            if line.starts_with(':') {
                let comment = line[1..].trim();
                if let Some(rest) = comment.strip_prefix("relay loading model=") {
                    let loading_model = rest.trim().to_string();
                    sender.push(AssistantMessageEvent::Loading {
                        model: loading_model,
                        elapsed_ms: 0,
                    });
                }
                continue;
            }

            // Parse SSE field
            let colon_idx = line.find(':');
            let field_name = match colon_idx {
                Some(i) => &line[..i],
                None => line,
            };
            let value = match colon_idx {
                Some(i) => {
                    let mut v = line[i + 1..].to_string();
                    if v.starts_with(' ') {
                        v = v[1..].to_string();
                    }
                    v
                }
                None => String::new(),
            };

            match field_name {
                "event" => {
                    sse_event = Some(value);
                }
                "data" => {
                    // Special handling for event: data that isn't prefixed by event line
                    // Some Anthropic SSE implementations send data directly
                    sse_data.push(value);
                }
                _ => {}
            }
        }
    }

    // Check for trailing data in buffer
    if !buffer.is_empty() {
        let trailing = String::from_utf8_lossy(&buffer);
        let line = trailing.trim_end_matches(['\r', '\n']);
        if !line.is_empty() {
            // Try to flush if we have a pending event
            if let Some(etype) = sse_event.take() {
                let data = sse_data.join("\n");
                sse_data.clear();
                if etype == "message_stop" && !data.is_empty() {
                    saw_message_stop = true;
                }
            }
        }
    }

    // Post-stream validation
    if let Some(signal) = &options.base.signal {
        if *signal.borrow() {
            return Err("Request was aborted".to_string());
        }
    }

    if saw_message_start && !saw_message_stop {
        return Err("Anthropic stream ended before message_stop".to_string());
    }

    if output.stop_reason == StopReason::Aborted {
        return Err("Request was aborted".to_string());
    }
    if output.stop_reason == StopReason::Error {
        let err_msg = output
            .error_message
            .clone()
            .unwrap_or_else(|| "Provider returned an error stop reason".to_string());
        return Err(err_msg);
    }

    Ok(output)
}

// ---------------------------------------------------------------------------
// HTTP request helper
// ---------------------------------------------------------------------------

async fn send_with_timeout_and_abort(
    request: reqwest::RequestBuilder,
    timeout_ms: u64,
    signal: Option<tokio::sync::watch::Receiver<bool>>,
) -> Result<reqwest::Response, String> {
    use tokio::time::{Duration, sleep};

    let send_fut = request.send();

    match signal {
        Some(mut sig) => {
            tokio::select! {
                resp = send_fut => resp.map_err(|e| e.to_string()),
                _ = wait_for_abort(&mut sig) => Err("Request was aborted".to_string()),
                _ = sleep(Duration::from_millis(timeout_ms)) => {
                    Err(format!("Request timed out after {}ms", timeout_ms))
                }
            }
        }
        None => {
            tokio::select! {
                resp = send_fut => resp.map_err(|e| e.to_string()),
                _ = sleep(Duration::from_millis(timeout_ms)) => {
                    Err(format!("Request timed out after {}ms", timeout_ms))
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Merge headers helper
// ---------------------------------------------------------------------------

#[allow(dead_code)]
fn merge_headers(sources: &[Option<&HashMap<String, String>>]) -> HashMap<String, String> {
    let mut merged = HashMap::new();
    for source in sources {
        if let Some(headers) = source {
            for (k, v) in *headers {
                merged.insert(k.clone(), v.clone());
            }
        }
    }
    merged
}

// ---------------------------------------------------------------------------
// Build HTTP headers
// ---------------------------------------------------------------------------

fn build_anthropic_headers(
    model: &Model,
    api_key: &str,
    interleaved_thinking: bool,
    use_fine_grained_tool_streaming_beta: bool,
    options_headers: Option<&HashMap<String, String>>,
    dynamic_headers: Option<&HashMap<String, String>>,
    session_id: Option<&str>,
    _env: Option<&ProviderEnv>,
) -> (HashMap<String, String>, bool) {
    let is_oauth = is_oauth_token(api_key);
    let compat = get_anthropic_compat(model);
    // Adaptive thinking models have interleaved thinking built in, so skip the beta header.
    let needs_interleaved_beta = interleaved_thinking && !compat.force_adaptive_thinking;
    let mut beta_features: Vec<String> = Vec::new();
    if use_fine_grained_tool_streaming_beta {
        beta_features.push(FINE_GRAINED_TOOL_STREAMING_BETA.to_string());
    }
    if needs_interleaved_beta {
        beta_features.push(INTERLEAVED_THINKING_BETA.to_string());
    }

    let mut headers = HashMap::new();

    if model.provider == "cloudflare-ai-gateway" {
        headers.insert("accept".to_string(), "application/json".to_string());
        headers.insert(
            "anthropic-dangerous-direct-browser-access".to_string(),
            "true".to_string(),
        );
        headers.insert(
            "cf-aig-authorization".to_string(),
            format!("Bearer {}", api_key),
        );
        if !beta_features.is_empty() {
            headers.insert("anthropic-beta".to_string(), beta_features.join(","));
        }
        // Apply model.headers, options.headers, dynamic headers
        if let Some(ref mh) = model.headers {
            for (k, v) in mh {
                headers.insert(k.clone(), v.clone());
            }
        }
        if let Some(oh) = options_headers {
            for (k, v) in oh {
                headers.insert(k.clone(), v.clone());
            }
        }
        return (headers, false);
    }

    if model.provider == "github-copilot" {
        headers.insert("accept".to_string(), "application/json".to_string());
        headers.insert(
            "anthropic-dangerous-direct-browser-access".to_string(),
            "true".to_string(),
        );
        headers.insert("authorization".to_string(), format!("Bearer {}", api_key));
        if !beta_features.is_empty() {
            headers.insert("anthropic-beta".to_string(), beta_features.join(","));
        }
        if let Some(ref mh) = model.headers {
            for (k, v) in mh {
                headers.insert(k.clone(), v.clone());
            }
        }
        if let Some(dh) = dynamic_headers {
            for (k, v) in dh {
                headers.insert(k.clone(), v.clone());
            }
        }
        if let Some(oh) = options_headers {
            for (k, v) in oh {
                headers.insert(k.clone(), v.clone());
            }
        }
        return (headers, false);
    }

    if is_oauth {
        let mut betas = vec![
            "claude-code-20250219".to_string(),
            "oauth-2025-04-20".to_string(),
        ];
        betas.extend(beta_features);
        headers.insert("accept".to_string(), "application/json".to_string());
        headers.insert(
            "anthropic-dangerous-direct-browser-access".to_string(),
            "true".to_string(),
        );
        headers.insert("anthropic-beta".to_string(), betas.join(","));
        headers.insert("authorization".to_string(), format!("Bearer {}", api_key));
        headers.insert(
            "user-agent".to_string(),
            format!("claude-cli/{}", CLAUDE_CODE_VERSION),
        );
        headers.insert("x-app".to_string(), "cli".to_string());
        if let Some(ref mh) = model.headers {
            for (k, v) in mh {
                headers.insert(k.clone(), v.clone());
            }
        }
        if let Some(oh) = options_headers {
            for (k, v) in oh {
                headers.insert(k.clone(), v.clone());
            }
        }
        return (headers, true);
    }

    // Standard API key auth
    let compat = get_anthropic_compat(model);
    headers.insert("accept".to_string(), "application/json".to_string());
    headers.insert(
        "anthropic-dangerous-direct-browser-access".to_string(),
        "true".to_string(),
    );
    headers.insert("x-api-key".to_string(), api_key.to_string());
    headers.insert("anthropic-version".to_string(), "2023-06-01".to_string());
    if !beta_features.is_empty() {
        headers.insert("anthropic-beta".to_string(), beta_features.join(","));
    }

    // Session affinity headers
    if let Some(sid) = session_id {
        if compat.send_session_affinity_headers {
            headers.insert("x-session-affinity".to_string(), sid.to_string());
        }
    }

    if let Some(ref mh) = model.headers {
        for (k, v) in mh {
            headers.insert(k.clone(), v.clone());
        }
    }
    if let Some(oh) = options_headers {
        for (k, v) in oh {
            headers.insert(k.clone(), v.clone());
        }
    }

    (headers, false)
}

// ---------------------------------------------------------------------------
// Convert messages to Anthropic format
// ---------------------------------------------------------------------------

/// Convert hamr messages to Anthropic Messages API format.
fn convert_messages(
    messages: &[Message],
    model: &Model,
    is_oauth: bool,
    cache_control: Option<&CacheControl>,
    compat: &AnthropicCompat,
) -> Vec<Value> {
    let normalize_tool_call_id = |id: &str, _model: &Model, _source: &AssistantMessage| -> String {
        id.chars()
            .map(|c| {
                if c.is_alphanumeric() || c == '_' || c == '-' {
                    c
                } else {
                    '_'
                }
            })
            .take(64)
            .collect()
    };
    let normalize_fn: Option<&dyn Fn(&str, &Model, &AssistantMessage) -> String> =
        Some(&normalize_tool_call_id);

    let transformed = transform_messages(messages.to_vec(), model, normalize_fn);

    let mut params: Vec<Value> = Vec::new();
    let mut i = 0;

    while i < transformed.len() {
        match &transformed[i] {
            Message::User(user_msg) => {
                if user_msg.content.is_empty() {
                    i += 1;
                    continue;
                }
                let blocks: Vec<Value> = user_msg
                    .content
                    .iter()
                    .map(|item| match item {
                        MessageContent::Text(tc) => {
                            serde_json::json!({
                                "type": "text",
                                "text": sanitize_surrogates(&tc.text),
                            })
                        }
                        MessageContent::Image(ic) => {
                            serde_json::json!({
                                "type": "image",
                                "source": {
                                    "type": "base64",
                                    "media_type": ic.mime_type,
                                    "data": ic.data,
                                }
                            })
                        }
                    })
                    .collect();

                if blocks.is_empty() {
                    i += 1;
                    continue;
                }

                let user_val = serde_json::json!({
                    "role": "user",
                    "content": blocks,
                });
                params.push(user_val);
            }

            Message::Assistant(assistant_msg) => {
                let mut blocks: Vec<Value> = Vec::new();

                for block in &assistant_msg.content {
                    match block {
                        AssistantContentBlock::Text(tc) => {
                            if tc.text.trim().is_empty() {
                                continue;
                            }
                            blocks.push(serde_json::json!({
                                "type": "text",
                                "text": sanitize_surrogates(&tc.text),
                            }));
                        }
                        AssistantContentBlock::Thinking(tc) => {
                            if tc.redacted {
                                blocks.push(serde_json::json!({
                                    "type": "redacted_thinking",
                                    "data": tc.thinking_signature.clone().unwrap_or_default(),
                                }));
                                continue;
                            }
                            if tc.thinking.trim().is_empty() {
                                continue;
                            }
                            match &tc.thinking_signature {
                                Some(sig) if !sig.trim().is_empty() => {
                                    blocks.push(serde_json::json!({
                                        "type": "thinking",
                                        "thinking": sanitize_surrogates(&tc.thinking),
                                        "signature": sig,
                                    }));
                                }
                                _ => {
                                    if compat.allow_empty_signature {
                                        blocks.push(serde_json::json!({
                                            "type": "thinking",
                                            "thinking": sanitize_surrogates(&tc.thinking),
                                            "signature": "",
                                        }));
                                    } else {
                                        blocks.push(serde_json::json!({
                                            "type": "text",
                                            "text": sanitize_surrogates(&tc.thinking),
                                        }));
                                    }
                                }
                            }
                        }
                        AssistantContentBlock::ToolCall(tool_call) => {
                            let tool_name = if is_oauth {
                                to_claude_code_name(&tool_call.name)
                            } else {
                                tool_call.name.clone()
                            };
                            blocks.push(serde_json::json!({
                                "type": "tool_use",
                                "id": tool_call.id,
                                "name": tool_name,
                                "input": tool_call.arguments,
                            }));
                        }
                    }
                }

                if blocks.is_empty() {
                    i += 1;
                    continue;
                }
                params.push(serde_json::json!({
                    "role": "assistant",
                    "content": blocks,
                }));
            }

            Message::ToolResult(_tool_result_msg) => {
                // Collect consecutive tool results
                let mut tool_results: Vec<Value> = Vec::new();
                let mut j = i;
                while j < transformed.len() {
                    match &transformed[j] {
                        Message::ToolResult(tr) => {
                            let content = convert_tool_result_content(&tr.content);
                            let tr_val = serde_json::json!({
                                "type": "tool_result",
                                "tool_use_id": tr.tool_call_id,
                                "content": content,
                                "is_error": tr.is_error,
                            });
                            tool_results.push(tr_val);
                            j += 1;
                        }
                        _ => break,
                    }
                }
                i = j - 1;
                params.push(serde_json::json!({
                    "role": "user",
                    "content": tool_results,
                }));
            }
        }
        i += 1;
    }

    // Add cache_control to the last user message
    if let Some(ref cc) = cache_control {
        if let Some(last_msg) = params.last_mut() {
            if last_msg.get("role").and_then(|r| r.as_str()) == Some("user") {
                let cc_val = serde_json::json!({
                    "type": "ephemeral",
                    "ttl": cc.ttl,
                });
                if let Some(content) = last_msg.get_mut("content") {
                    if let Some(arr) = content.as_array_mut() {
                        if let Some(last_block) = arr.last_mut() {
                            if let Some(obj) = last_block.as_object_mut() {
                                obj.insert("cache_control".to_string(), cc_val);
                            }
                        }
                    }
                }
            }
        }
    }

    params
}

fn convert_tool_result_content(content: &[MessageContent]) -> Value {
    let has_images = content
        .iter()
        .any(|c| matches!(c, MessageContent::Image(_)));
    if !has_images {
        let text: String = content
            .iter()
            .filter_map(|c| {
                if let MessageContent::Text(tc) = c {
                    Some(tc.text.as_str())
                } else {
                    None
                }
            })
            .collect::<Vec<&str>>()
            .join("\n");
        return Value::String(sanitize_surrogates(&text));
    }

    let blocks: Vec<Value> = content
        .iter()
        .map(|item| match item {
            MessageContent::Text(tc) => {
                serde_json::json!({
                    "type": "text",
                    "text": sanitize_surrogates(&tc.text),
                })
            }
            MessageContent::Image(ic) => {
                serde_json::json!({
                    "type": "image",
                    "source": {
                        "type": "base64",
                        "media_type": ic.mime_type,
                        "data": ic.data,
                    }
                })
            }
        })
        .collect();

    let has_text = blocks
        .iter()
        .any(|b| b.get("type").and_then(|t| t.as_str()) == Some("text"));
    let mut result = blocks;
    if !has_text {
        result.insert(
            0,
            serde_json::json!({
                "type": "text",
                "text": "(see attached image)",
            }),
        );
    }
    Value::Array(result)
}

// ---------------------------------------------------------------------------
// Convert tools to Anthropic format
// ---------------------------------------------------------------------------

fn convert_tools(
    tools: &[Tool],
    is_oauth: bool,
    supports_eager_tool_input_streaming: bool,
    cache_control: Option<&CacheControl>,
    skip_cache_control_on_tools: bool,
) -> Value {
    if tools.is_empty() {
        return Value::Array(Vec::new());
    }

    let result: Vec<Value> = tools
        .iter()
        .enumerate()
        .map(|(index, tool)| {
            let params = tool.parameters.as_object().cloned().unwrap_or_default();
            let properties = params
                .get("properties")
                .cloned()
                .unwrap_or(Value::Object(Default::default()));
            let required = params
                .get("required")
                .and_then(|r| r.as_array())
                .cloned()
                .unwrap_or_default();

            let mut tool_obj = serde_json::json!({
                "name": if is_oauth { to_claude_code_name(&tool.name) } else { tool.name.clone() },
                "description": tool.description,
                "input_schema": {
                    "type": "object",
                    "properties": properties,
                    "required": required,
                }
            });

            if supports_eager_tool_input_streaming {
                if let Some(obj) = tool_obj.as_object_mut() {
                    obj.insert("eager_input_streaming".to_string(), Value::Bool(true));
                }
            }

            // Last tool gets cache_control
            if let Some(cc) = cache_control {
                if !skip_cache_control_on_tools && index == tools.len() - 1 {
                    if let Some(obj) = tool_obj.as_object_mut() {
                        obj.insert(
                            "cache_control".to_string(),
                            serde_json::json!({
                                "type": "ephemeral",
                                "ttl": cc.ttl,
                            }),
                        );
                    }
                }
            }

            tool_obj
        })
        .collect();

    Value::Array(result)
}

// ---------------------------------------------------------------------------
// Build request params
// ---------------------------------------------------------------------------

fn build_params(
    model: &Model,
    context: &Context,
    is_oauth: bool,
    options: &AnthropicOptions,
    _cache_retention: CacheRetention,
    compat: &AnthropicCompat,
    cache_control: Option<&CacheControl>,
) -> Value {
    let mut params = serde_json::Map::new();

    // Model
    params.insert("model".to_string(), Value::String(model.id.clone()));

    // Messages
    let messages = convert_messages(&context.messages, model, is_oauth, cache_control, compat);
    params.insert("messages".to_string(), Value::Array(messages));

    // Max tokens
    let max_tokens = options.base.max_tokens.unwrap_or(model.max_tokens);
    params.insert("max_tokens".to_string(), serde_json::json!(max_tokens));

    // Stream
    params.insert("stream".to_string(), Value::Bool(true));

    // System prompt
    if is_oauth {
        let mut system: Vec<Value> = Vec::new();
        system.push(serde_json::json!({
            "type": "text",
            "text": "You are Claude Code, Anthropic's official CLI for Claude.",
            "cache_control": cache_control.map(|cc| serde_json::json!({
                "type": "ephemeral",
                "ttl": cc.ttl,
            })),
        }));
        if let Some(ref sp) = context.system_prompt {
            system.push(serde_json::json!({
                "type": "text",
                "text": sanitize_surrogates(sp),
                "cache_control": cache_control.map(|cc| serde_json::json!({
                    "type": "ephemeral",
                    "ttl": cc.ttl,
                })),
            }));
        }
        params.insert("system".to_string(), Value::Array(system));
    } else if let Some(ref sp) = context.system_prompt {
        let mut system_val = serde_json::json!({
            "type": "text",
            "text": sanitize_surrogates(sp),
        });
        if let Some(cc) = cache_control {
            if let Some(obj) = system_val.as_object_mut() {
                obj.insert(
                    "cache_control".to_string(),
                    serde_json::json!({
                        "type": "ephemeral",
                        "ttl": cc.ttl,
                    }),
                );
            }
        }
        params.insert("system".to_string(), Value::Array(vec![system_val]));
    }

    // Temperature (incompatible with extended thinking and unsupported on some models)
    if let Some(temp) = options.base.temperature {
        let thinking_enabled = options.thinking_enabled.unwrap_or(false);
        if !thinking_enabled && compat.supports_temperature {
            params.insert("temperature".to_string(), serde_json::json!(temp));
        }
    }

    // Tools
    if !context.tools.is_empty() {
        let tools = convert_tools(
            &context.tools,
            is_oauth,
            compat.supports_eager_tool_input_streaming,
            cache_control,
            !compat.supports_cache_control_on_tools,
        );
        params.insert("tools".to_string(), tools);
    }

    // Thinking configuration
    if model.reasoning {
        if let Some(true) = options.thinking_enabled {
            // Default to "summarized" so Opus 4.7 and Mythos Preview behave like
            // older Claude 4 models (whose API default is also "summarized").
            let display = options.thinking_display.as_deref().unwrap_or("summarized");
            if compat.force_adaptive_thinking {
                // Adaptive thinking: Claude decides when and how much to think.
                let mut thinking = serde_json::Map::new();
                thinking.insert("type".to_string(), Value::String("adaptive".to_string()));
                thinking.insert("display".to_string(), Value::String(display.to_string()));
                params.insert("thinking".to_string(), Value::Object(thinking));
                if let Some(ref effort) = options.effort {
                    params.insert(
                        "output_config".to_string(),
                        serde_json::json!({ "effort": effort }),
                    );
                }
            } else {
                // Budget-based thinking for older models
                let budget = options.thinking_budget_tokens.unwrap_or(1024);
                params.insert(
                    "thinking".to_string(),
                    serde_json::json!({
                        "type": "enabled",
                        "budget_tokens": budget,
                        "display": display,
                    }),
                );
            }
        } else if options.thinking_enabled == Some(false)
            && !matches!(
                model
                    .thinking_level_map
                    .as_ref()
                    .and_then(|m| m.get(&crate::types::ModelThinkingLevel::Off)),
                Some(None)
            )
        {
            // Emit explicit disable unless thinkingLevelMap marks "off" as explicitly
            // null (i.e. `Some(None)` in Rust). The TS checks:
            //   model.thinkingLevelMap?.off !== null
            // Only when "off" is explicitly `null` do we skip emitting the disable.
            params.insert(
                "thinking".to_string(),
                serde_json::json!({ "type": "disabled" }),
            );
        }
    }

    // Metadata
    if let Some(ref metadata) = options.base.metadata {
        if let Some(user_id) = metadata.get("user_id").and_then(|v| v.as_str()) {
            params.insert(
                "metadata".to_string(),
                serde_json::json!({ "user_id": user_id }),
            );
        }
    }

    // Tool choice
    if let Some(ref tool_choice) = options.tool_choice {
        params.insert("tool_choice".to_string(), tool_choice.clone());
    }

    Value::Object(params)
}

// ---------------------------------------------------------------------------
// Stream inner driver
// ---------------------------------------------------------------------------

async fn run_stream_inner(
    model: &Model,
    context: &Context,
    options: &AnthropicOptions,
    sender: &mut AssistantMessageEventStreamSender,
    output: &mut AssistantMessage,
) -> Result<(), String> {
    let api_key = match &options.base.api_key {
        Some(k) if !k.is_empty() => k.clone(),
        _ => return Err(format!("No API key for provider: {}", model.provider)),
    };

    let compat = get_anthropic_compat(model);
    let (cache_retention, cache_control) =
        get_cache_control(model, options.cache_retention, options.base.env.as_ref());

    // Build params
    let is_oauth_token = is_oauth_token(&api_key);
    let mut params = build_params(
        model,
        context,
        is_oauth_token,
        options,
        cache_retention,
        &compat,
        cache_control.as_ref(),
    );

    // onPayload hook
    if let Some(on_payload) = &options.base.on_payload {
        if let Some(next) = on_payload(params.clone(), model.clone()).await {
            params = next;
        }
    }

    // Build headers
    let interleaved = options.interleaved_thinking.unwrap_or(true);
    let use_fine_grained_beta = should_use_fine_grained_tool_streaming_beta(model, context);

    // Copilot dynamic headers
    let mut copilot_dynamic: Option<HashMap<String, String>> = None;
    if model.provider == "github-copilot" {
        let has_images = has_copilot_vision_input(&context.messages);
        let dh = build_copilot_dynamic_headers(&context.messages, has_images);
        copilot_dynamic = Some(dh);
    }

    let cache_session_id = if cache_retention == CacheRetention::None {
        None
    } else {
        options.base.session_id.as_deref()
    };

    let (headers, _is_oauth) = build_anthropic_headers(
        model,
        &api_key,
        interleaved,
        use_fine_grained_beta,
        options.base.headers.as_ref(),
        copilot_dynamic.as_ref(),
        cache_session_id,
        options.base.env.as_ref(),
    );

    // Resolve base URL
    let base_url = if model.provider == "cloudflare-ai-gateway" {
        resolve_cloudflare_base_url(model, options.base.env.as_ref()).map_err(|e| e)?
    } else {
        model.base_url.clone()
    };

    let url = format!("{}/v1/messages", base_url.trim_end_matches('/'));

    let client = reqwest::Client::new();
    let mut request = client.post(&url).header("content-type", "application/json");

    for (k, v) in &headers {
        request = request.header(k.as_str(), v.as_str());
    }

    let body_bytes = serde_json::to_vec(&params).map_err(|e| e.to_string())?;
    request = request.body(body_bytes);

    // Send
    let timeout_ms = options.base.timeout_ms.unwrap_or(600_000);
    let response =
        match send_with_timeout_and_abort(request, timeout_ms, options.base.signal.clone()).await {
            Ok(resp) => resp,
            Err(e) => return Err(e),
        };

    // onResponse hook
    if let Some(on_response) = &options.base.on_response {
        let provider_response = ProviderResponse {
            status: response.status().as_u16(),
            headers: headers_to_record(response.headers()),
        };
        on_response(provider_response, model.clone()).await;
    }

    if !response.status().is_success() {
        let status = response.status().as_u16();
        let body_text = response.text().await.unwrap_or_default();
        return Err(parse_anthropic_error(status, &body_text));
    }

    *output = process_anthropic_sse_stream(
        response,
        &context.tools,
        model,
        output.clone(),
        options,
        sender,
    )
    .await?;

    Ok(())
}

// ---------------------------------------------------------------------------
// Run stream (top-level driver)
// ---------------------------------------------------------------------------

async fn run_stream(
    model: Model,
    context: Context,
    options: AnthropicOptions,
    mut sender: AssistantMessageEventStreamSender,
) {
    let mut output = initial_output(&model);

    match run_stream_inner(&model, &context, &options, &mut sender, &mut output).await {
        Ok(()) => {
            let reason = match output.stop_reason {
                StopReason::Length => DoneReason::Length,
                StopReason::ToolUse => DoneReason::ToolUse,
                _ => DoneReason::Stop,
            };
            sender.push(AssistantMessageEvent::Done {
                reason,
                message: output,
            });
            sender.end(None);
        }
        Err(err) => {
            // Strip scratch buffers from content blocks
            for block in &mut output.content {
                if let AssistantContentBlock::ToolCall(tc) = block {
                    tc.thought_signature = None;
                }
            }

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

/// Produce a stream that immediately emits a terminal error event.
fn error_stream(model: &Model, message: String) -> AssistantMessageEventStream {
    let (mut sender, stream_out) = create_assistant_message_event_stream();
    let mut output = initial_output(model);
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
// Public entry points
// ---------------------------------------------------------------------------

/// Stream a completion from the Anthropic Messages API.
///
/// Mirrors the TS `streamAnthropic`.
pub fn stream_anthropic(
    model: Model,
    context: Context,
    options: Option<AnthropicOptions>,
) -> AssistantMessageEventStream {
    let (sender, stream_out) = create_assistant_message_event_stream();
    let options = options.unwrap_or_default();

    tokio::spawn(async move {
        run_stream(model, context, options, sender).await;
    });

    stream_out
}

/// Stream with simplified reasoning-level options.
///
/// Mirrors the TS `streamSimpleAnthropic`: maps the unified `reasoning` level into
/// Anthropic effort or budget-based thinking.
pub fn stream_simple_anthropic(
    model: Model,
    context: Context,
    options: Option<SimpleStreamOptions>,
) -> AssistantMessageEventStream {
    let api_key = options.as_ref().and_then(|o| o.base.api_key.clone());
    if api_key.is_none() {
        return error_stream(
            &model,
            format!("No API key for provider: {}", model.provider),
        );
    }

    let base = build_base_options(&model, options.as_ref(), api_key.as_deref());

    let reasoning = options.as_ref().and_then(|o| o.reasoning);

    if reasoning.is_none() {
        return stream_anthropic(
            model,
            context,
            Some(AnthropicOptions {
                base,
                thinking_enabled: Some(false),
                ..Default::default()
            }),
        );
    }

    let reasoning = reasoning.unwrap();

    // For models with adaptive thinking: use an effort level.
    // For older models: use budget-based thinking.
    let compat = get_anthropic_compat(&model);

    if compat.force_adaptive_thinking {
        let effort = map_thinking_level_to_effort(&model, reasoning);
        return stream_anthropic(
            model,
            context,
            Some(AnthropicOptions {
                base,
                thinking_enabled: Some(true),
                effort: Some(effort),
                ..Default::default()
            }),
        );
    }

    let adjusted = adjust_max_tokens_for_thinking(
        base.max_tokens,
        model.max_tokens,
        reasoning,
        options.as_ref().and_then(|o| o.thinking_budgets.as_ref()),
    );

    let mut base = base;
    base.max_tokens = Some(adjusted.max_tokens);

    stream_anthropic(
        model,
        context,
        Some(AnthropicOptions {
            base,
            thinking_enabled: Some(true),
            thinking_budget_tokens: Some(adjusted.thinking_budget as u32),
            ..Default::default()
        }),
    )
}

/// Map a thinking level to an Anthropic effort string for adaptive thinking.
fn map_thinking_level_to_effort(model: &Model, level: ThinkingLevel) -> String {
    // Try model's thinking level map first
    let level_key = match level {
        ThinkingLevel::Minimal => crate::types::ModelThinkingLevel::Minimal,
        ThinkingLevel::Low => crate::types::ModelThinkingLevel::Low,
        ThinkingLevel::Medium => crate::types::ModelThinkingLevel::Medium,
        ThinkingLevel::High => crate::types::ModelThinkingLevel::High,
        ThinkingLevel::XHigh => crate::types::ModelThinkingLevel::XHigh,
    };
    if let Some(ref tlm) = model.thinking_level_map {
        if let Some(Some(mapped)) = tlm.get(&level_key) {
            return mapped.clone();
        }
    }

    match level {
        ThinkingLevel::Minimal | ThinkingLevel::Low => "low".to_string(),
        ThinkingLevel::Medium => "medium".to_string(),
        ThinkingLevel::High | ThinkingLevel::XHigh => "high".to_string(),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::*;
    use chrono::Utc;

    fn test_model() -> Model {
        Model {
            id: "claude-sonnet-4-20250514".to_string(),
            name: "Claude Sonnet 4".to_string(),
            api: Api::AnthropicMessages,
            provider: "anthropic".to_string(),
            base_url: "https://api.anthropic.com".to_string(),
            reasoning: false,
            thinking_level_map: None,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 3.0,
                output: 15.0,
                cache_read: 0.3,
                cache_write: 3.0,
            },
            context_window: 200000,
            max_tokens: 8192,
            headers: None,
            compat: None,
        }
    }

    fn empty_context() -> Context {
        Context {
            system_prompt: None,
            messages: Vec::new(),
            tools: Vec::new(),
        }
    }

    #[test]
    fn detect_oauth_token() {
        assert!(is_oauth_token("sk-ant-oat-abc123"));
        assert!(!is_oauth_token("sk-ant-api03-abc123"));
        assert!(!is_oauth_token(""));
    }

    #[test]
    fn claude_code_name_roundtrip() {
        assert_eq!(to_claude_code_name("read"), "Read");
        assert_eq!(to_claude_code_name("Read"), "Read");
        assert_eq!(to_claude_code_name("unknown_tool"), "unknown_tool");

        let tools = vec![Tool {
            name: "custom_tool".to_string(),
            description: "".to_string(),
            parameters: serde_json::json!({}),
        }];
        assert_eq!(from_claude_code_name("Custom_Tool", &tools), "custom_tool");
        assert_eq!(from_claude_code_name("Read", &tools), "Read");
    }

    #[test]
    fn cache_retention_defaults_short() {
        assert_eq!(resolve_cache_retention(None, None), CacheRetention::Short);
    }

    #[test]
    fn cache_retention_explicit() {
        assert_eq!(
            resolve_cache_retention(Some(CacheRetention::Long), None),
            CacheRetention::Long
        );
    }

    #[test]
    fn map_stop_reason_values() {
        let (reason, _) = map_stop_reason("end_turn", None);
        assert_eq!(reason, StopReason::Stop);

        let (reason, _) = map_stop_reason("max_tokens", None);
        assert_eq!(reason, StopReason::Length);

        let (reason, _) = map_stop_reason("tool_use", None);
        assert_eq!(reason, StopReason::ToolUse);

        let (reason, msg) = map_stop_reason("refusal", None);
        assert_eq!(reason, StopReason::Error);
        assert!(msg.is_some());

        let (reason, _) = map_stop_reason("pause_turn", None);
        assert_eq!(reason, StopReason::Stop);

        let (reason, _) = map_stop_reason("stop_sequence", None);
        assert_eq!(reason, StopReason::Stop);

        let (reason, _) = map_stop_reason("sensitive", None);
        assert_eq!(reason, StopReason::Error);
    }

    #[test]
    fn get_anthropic_compat_fireworks() {
        let model = Model {
            provider: "fireworks".to_string(),
            ..test_model()
        };
        let compat = get_anthropic_compat(&model);
        assert!(!compat.supports_eager_tool_input_streaming);
        assert!(!compat.supports_long_cache_retention);
    }

    #[test]
    fn get_anthropic_compat_standard() {
        let compat = get_anthropic_compat(&test_model());
        assert!(compat.supports_eager_tool_input_streaming);
        assert!(compat.supports_long_cache_retention);
    }

    #[test]
    fn is_adaptive_thinking_model_detects_all_variants() {
        // Dash-separated IDs
        assert!(is_adaptive_thinking_model("claude-fable-5"));
        assert!(is_adaptive_thinking_model("claude-opus-4-6"));
        assert!(is_adaptive_thinking_model("claude-opus-4-7"));
        assert!(is_adaptive_thinking_model("claude-opus-4-8"));
        assert!(is_adaptive_thinking_model("claude-sonnet-4-6"));
        // Dot-separated IDs (e.g. openrouter prefixes)
        assert!(is_adaptive_thinking_model("anthropic/claude-opus-4.6"));
        assert!(is_adaptive_thinking_model("anthropic/claude-opus-4.7"));
        assert!(is_adaptive_thinking_model("anthropic/claude-opus-4.8"));
        assert!(is_adaptive_thinking_model("anthropic/claude-sonnet-4-6"));
        // Bedrock-style prefixes
        assert!(is_adaptive_thinking_model("us.anthropic.claude-sonnet-4-6"));
        // Non-adaptive models
        assert!(!is_adaptive_thinking_model("claude-3-5-sonnet-20241022"));
        assert!(!is_adaptive_thinking_model("claude-3-7-sonnet-20250219"));
    }

    #[test]
    fn model_supports_temperature_detection() {
        assert!(!model_supports_temperature("claude-opus-4-7"));
        assert!(!model_supports_temperature("claude-opus-4-8"));
        assert!(!model_supports_temperature("anthropic/claude-opus-4.7"));
        assert!(model_supports_temperature("claude-opus-4-6"));
        assert!(model_supports_temperature("claude-sonnet-4-6"));
        assert!(model_supports_temperature("claude-3-5-sonnet-20241022"));
    }

    #[test]
    fn should_use_fine_grained_beta_for_fireworks_with_tools() {
        let fireworks_model = Model {
            provider: "fireworks".to_string(),
            ..test_model()
        };
        let context = Context {
            tools: vec![Tool {
                name: "test".to_string(),
                description: "Test".to_string(),
                parameters: serde_json::json!({}),
            }],
            ..empty_context()
        };
        // Fireworks does NOT support eager tool input streaming, so the beta IS needed.
        assert!(should_use_fine_grained_tool_streaming_beta(
            &fireworks_model,
            &context
        ));
    }

    #[test]
    fn should_not_use_fine_grained_beta_for_standard_anthropic_even_with_tools() {
        let context = Context {
            tools: vec![Tool {
                name: "test".to_string(),
                description: "Test".to_string(),
                parameters: serde_json::json!({}),
            }],
            ..empty_context()
        };
        // Standard Anthropic supports eager tool input streaming, so the beta is NOT needed.
        assert!(!should_use_fine_grained_tool_streaming_beta(
            &test_model(),
            &context
        ));
    }

    #[test]
    fn should_not_use_fine_grained_beta_without_tools() {
        let fireworks_model = Model {
            provider: "fireworks".to_string(),
            ..test_model()
        };
        assert!(!should_use_fine_grained_tool_streaming_beta(
            &fireworks_model,
            &empty_context()
        ));
    }

    #[test]
    fn convert_messages_user_text() {
        let compat = get_anthropic_compat(&test_model());
        let context = Context {
            system_prompt: None,
            messages: vec![Message::User(UserMessage {
                role: MessageRole::User,
                content: vec![MessageContent::Text(TextContent {
                    text: "Hello".to_string(),
                    text_signature: None,
                })],
                timestamp: Utc::now(),
            })],
            tools: vec![],
        };
        let result = convert_messages(&context.messages, &test_model(), false, None, &compat);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].get("role").and_then(|r| r.as_str()), Some("user"));
        let content = result[0].get("content").and_then(|c| c.as_array()).unwrap();
        assert_eq!(content.len(), 1);
        assert_eq!(
            content[0].get("text").and_then(|t| t.as_str()),
            Some("Hello")
        );
    }

    #[test]
    fn build_params_sets_basics() {
        let model = test_model();
        let compat = get_anthropic_compat(&model);
        let options = AnthropicOptions::default();
        let params = build_params(
            &model,
            &empty_context(),
            false,
            &options,
            CacheRetention::Short,
            &compat,
            None,
        );
        assert_eq!(
            params.get("model").and_then(|v| v.as_str()),
            Some("claude-sonnet-4-20250514")
        );
        assert_eq!(params.get("stream").and_then(|v| v.as_bool()), Some(true));
        assert!(params.get("messages").is_some());
    }

    #[test]
    fn build_params_with_tools() {
        let model = test_model();
        let compat = get_anthropic_compat(&model);
        let options = AnthropicOptions::default();
        let context = Context {
            system_prompt: None,
            messages: vec![],
            tools: vec![Tool {
                name: "get_weather".to_string(),
                description: "Get weather".to_string(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "location": { "type": "string" }
                    }
                }),
            }],
        };
        let params = build_params(
            &model,
            &context,
            false,
            &options,
            CacheRetention::Short,
            &compat,
            None,
        );
        assert!(params.get("tools").is_some());
        let tools = params.get("tools").and_then(|t| t.as_array()).unwrap();
        assert_eq!(tools.len(), 1);
        assert_eq!(
            tools[0].get("name").and_then(|n| n.as_str()),
            Some("get_weather")
        );
    }

    #[test]
    fn build_params_with_thinking_enabled() {
        let model = Model {
            reasoning: true,
            ..test_model()
        };
        let compat = get_anthropic_compat(&model);
        let options = AnthropicOptions {
            thinking_enabled: Some(true),
            thinking_budget_tokens: Some(2048),
            ..Default::default()
        };
        let params = build_params(
            &model,
            &empty_context(),
            false,
            &options,
            CacheRetention::Short,
            &compat,
            None,
        );
        let thinking = params.get("thinking").unwrap();
        assert_eq!(
            thinking.get("type").and_then(|t| t.as_str()),
            Some("enabled")
        );
        assert_eq!(
            thinking.get("budget_tokens").and_then(|b| b.as_u64()),
            Some(2048)
        );
    }

    #[test]
    fn build_params_with_temperature() {
        let model = test_model();
        let compat = get_anthropic_compat(&model);
        let options = AnthropicOptions {
            base: StreamOptions {
                temperature: Some(0.7),
                ..Default::default()
            },
            thinking_enabled: Some(false),
            ..Default::default()
        };
        let params = build_params(
            &model,
            &empty_context(),
            false,
            &options,
            CacheRetention::Short,
            &compat,
            None,
        );
        assert_eq!(
            params.get("temperature").and_then(|t| t.as_f64()),
            Some(0.7)
        );
    }

    #[test]
    fn temperature_omitted_with_thinking() {
        let model = Model {
            reasoning: true,
            ..test_model()
        };
        let compat = get_anthropic_compat(&model);
        let options = AnthropicOptions {
            base: StreamOptions {
                temperature: Some(0.7),
                ..Default::default()
            },
            thinking_enabled: Some(true),
            ..Default::default()
        };
        let params = build_params(
            &model,
            &empty_context(),
            false,
            &options,
            CacheRetention::Short,
            &compat,
            None,
        );
        assert!(params.get("temperature").is_none());
    }

    #[test]
    fn parse_anthropic_error_from_json() {
        let body =
            r#"{"error": {"message": "Insufficient credits", "type": "insufficient_credits"}}"#;
        let msg = parse_anthropic_error(429, body);
        assert!(msg.contains("429"));
        assert!(msg.contains("Insufficient credits"));
    }

    #[test]
    fn parse_anthropic_error_from_plain_text() {
        let body = "Internal Server Error";
        let msg = parse_anthropic_error(500, body);
        assert!(msg.contains("500"));
        assert!(msg.contains("Internal Server Error"));
    }

    #[test]
    fn build_anthropic_headers_standard_auth() {
        let model = test_model();
        let (headers, is_oauth) = build_anthropic_headers(
            &model,
            "sk-ant-api03-abc123",
            true,
            true,
            None,
            None,
            None,
            None,
        );
        assert!(!is_oauth);
        assert_eq!(
            headers.get("x-api-key").map(|s| s.as_str()),
            Some("sk-ant-api03-abc123")
        );
        assert!(headers.contains_key("anthropic-version"));
        assert!(headers.contains_key("anthropic-beta"));
    }

    #[test]
    fn build_anthropic_headers_oauth() {
        let model = test_model();
        let (headers, is_oauth) = build_anthropic_headers(
            &model,
            "sk-ant-oat-abc123",
            true,
            false,
            None,
            None,
            None,
            None,
        );
        assert!(is_oauth);
        assert_eq!(
            headers.get("user-agent").map(|s| s.as_str()),
            Some("claude-cli/2.1.75")
        );
        assert!(
            headers
                .get("anthropic-beta")
                .map(|s| s.as_str())
                .unwrap_or("")
                .contains("oauth")
        );
        assert!(headers.contains_key("x-app"));
    }

    #[test]
    fn map_thinking_level_to_effort_defaults() {
        let model = test_model();
        assert_eq!(
            map_thinking_level_to_effort(&model, ThinkingLevel::Low),
            "low"
        );
        assert_eq!(
            map_thinking_level_to_effort(&model, ThinkingLevel::Medium),
            "medium"
        );
        assert_eq!(
            map_thinking_level_to_effort(&model, ThinkingLevel::High),
            "high"
        );
    }

    #[test]
    fn map_thinking_level_through_model_map() {
        let model = Model {
            reasoning: true,
            thinking_level_map: Some({
                let mut m = std::collections::HashMap::new();
                m.insert(
                    crate::types::ModelThinkingLevel::High,
                    Some("xhigh".to_string()),
                );
                m
            }),
            ..test_model()
        };
        assert_eq!(
            map_thinking_level_to_effort(&model, ThinkingLevel::High),
            "xhigh"
        );
    }

    #[test]
    fn empty_usage_basics() {
        let usage = empty_usage();
        assert_eq!(usage.input, 0);
        assert_eq!(usage.output, 0);
        assert_eq!(usage.cache_read, 0);
        assert_eq!(usage.cache_write, 0);
    }

    #[test]
    fn convert_tool_result_content_text_only() {
        let content = vec![MessageContent::Text(TextContent {
            text: "Result: 42".to_string(),
            text_signature: None,
        })];
        let result = convert_tool_result_content(&content);
        assert_eq!(result, Value::String("Result: 42".to_string()));
    }

    #[test]
    fn convert_tool_result_content_with_images() {
        let content = vec![
            MessageContent::Text(TextContent {
                text: "Here's the image:".to_string(),
                text_signature: None,
            }),
            MessageContent::Image(ImageContent {
                data: "iVBOR".to_string(),
                mime_type: "image/png".to_string(),
            }),
        ];
        let result = convert_tool_result_content(&content);
        assert!(result.is_array());
        let arr = result.as_array().unwrap();
        assert_eq!(arr.len(), 2);
        assert_eq!(
            arr[1]
                .get("source")
                .and_then(|s| s.get("type"))
                .and_then(|t| t.as_str()),
            Some("base64")
        );
    }

    #[test]
    fn convert_messages_preserves_thinking_block_with_allow_empty_signature() {
        let model = test_model();
        // When allow_empty_signature is true, a thinking block with an empty
        // signature is preserved as a thinking block (with `signature: ""`)
        // instead of being converted to plain text.
        let mut compat = get_anthropic_compat(&model);
        compat.allow_empty_signature = true;
        let context = Context {
            system_prompt: None,
            messages: vec![Message::Assistant(AssistantMessage {
                role: MessageRole::Assistant,
                content: vec![AssistantContentBlock::Thinking(ThinkingContent {
                    thinking: "I am thinking deeply...".to_string(),
                    thinking_signature: Some(String::new()),
                    redacted: false,
                })],
                api: "anthropic-messages".to_string(),
                provider: "anthropic".to_string(),
                model: model.id.clone(),
                response_model: None,
                response_id: None,
                usage: empty_usage(),
                stop_reason: StopReason::Stop,
                error_message: None,
                diagnostics: None,
                timestamp: Utc::now(),
            })],
            tools: vec![],
        };
        let result = convert_messages(&context.messages, &model, false, None, &compat);
        assert_eq!(result.len(), 1);
        let content = result[0].get("content").and_then(|c| c.as_array()).unwrap();
        assert_eq!(content.len(), 1);
        assert_eq!(
            content[0].get("type").and_then(|t| t.as_str()),
            Some("thinking")
        );
        assert_eq!(
            content[0].get("signature").and_then(|s| s.as_str()),
            Some("")
        );
    }

    #[test]
    fn convert_messages_converts_empty_signature_to_text_when_not_allowed() {
        let model = test_model();
        // Default compat has allow_empty_signature = false.
        // A thinking block with an empty signature should be converted to text.
        let compat = get_anthropic_compat(&model);
        assert!(!compat.allow_empty_signature);
        let context = Context {
            system_prompt: None,
            messages: vec![Message::Assistant(AssistantMessage {
                role: MessageRole::Assistant,
                content: vec![AssistantContentBlock::Thinking(ThinkingContent {
                    thinking: "Fallback text".to_string(),
                    thinking_signature: Some(String::new()),
                    redacted: false,
                })],
                api: "anthropic-messages".to_string(),
                provider: "anthropic".to_string(),
                model: model.id.clone(),
                response_model: None,
                response_id: None,
                usage: empty_usage(),
                stop_reason: StopReason::Stop,
                error_message: None,
                diagnostics: None,
                timestamp: Utc::now(),
            })],
            tools: vec![],
        };
        let result = convert_messages(&context.messages, &model, false, None, &compat);
        assert_eq!(result.len(), 1);
        let content = result[0].get("content").and_then(|c| c.as_array()).unwrap();
        assert_eq!(content.len(), 1);
        assert_eq!(
            content[0].get("type").and_then(|t| t.as_str()),
            Some("text")
        );
        assert_eq!(
            content[0].get("text").and_then(|t| t.as_str()),
            Some("Fallback text")
        );
    }

    #[test]
    fn temperature_suppressed_when_supports_temperature_is_false() {
        let model = test_model();
        let mut compat = get_anthropic_compat(&model);
        compat.supports_temperature = false;
        let options = AnthropicOptions {
            base: StreamOptions {
                temperature: Some(0.7),
                ..Default::default()
            },
            thinking_enabled: Some(false),
            ..Default::default()
        };
        let params = build_params(
            &model,
            &empty_context(),
            false,
            &options,
            CacheRetention::Short,
            &compat,
            None,
        );
        // Temperature should be absent even though thinking is disabled,
        // because the compat says temperature is not supported.
        assert!(params.get("temperature").is_none());
    }

    // -----------------------------------------------------------------------
    // Anthropic thinking disable tests
    // (corresponds to anthropic-thinking-disable.test.ts)
    // -----------------------------------------------------------------------

    #[test]
    fn build_params_with_thinking_disabled_on_reasoning_model() {
        // For a reasoning model, thinking_enabled=Some(false) should produce
        // thinking: { type: "disabled" }
        let model = Model {
            reasoning: true,
            ..test_model()
        };
        let compat = get_anthropic_compat(&model);
        let options = AnthropicOptions {
            thinking_enabled: Some(false),
            ..Default::default()
        };
        let params = build_params(
            &model,
            &empty_context(),
            false,
            &options,
            CacheRetention::Short,
            &compat,
            None,
        );
        let thinking = params.get("thinking").unwrap();
        assert_eq!(
            thinking.get("type").and_then(|t| t.as_str()),
            Some("disabled")
        );
    }

    #[test]
    fn build_params_thinking_omitted_when_not_reasoning_and_not_explicitly_disabled() {
        // If model.reasoning is false and thinking_enabled is not set, no
        // thinking field should appear.
        let model = test_model(); // reasoning: false
        let compat = get_anthropic_compat(&model);
        let options = AnthropicOptions::default();
        let params = build_params(
            &model,
            &empty_context(),
            false,
            &options,
            CacheRetention::Short,
            &compat,
            None,
        );
        assert!(params.get("thinking").is_none());
    }

    #[test]
    fn build_params_thinking_omitted_when_reasoning_false_and_explicitly_off() {
        // Non-reasoning model with thinking_enabled=Some(false) — no thinking field.
        let model = test_model(); // reasoning: false
        let compat = get_anthropic_compat(&model);
        let options = AnthropicOptions {
            thinking_enabled: Some(false),
            ..Default::default()
        };
        let params = build_params(
            &model,
            &empty_context(),
            false,
            &options,
            CacheRetention::Short,
            &compat,
            None,
        );
        assert!(params.get("thinking").is_none());
    }

    // -----------------------------------------------------------------------
    // Eager tool input streaming compatibility tests
    // (corresponds to anthropic-eager-tool-input-compat.test.ts)
    // -----------------------------------------------------------------------

    #[test]
    fn convert_tools_sets_eager_input_streaming_by_default() {
        // Standard Anthropic compat: eager_input_streaming should be set on each tool.
        let compat = get_anthropic_compat(&test_model());
        assert!(compat.supports_eager_tool_input_streaming);
        let tools = vec![Tool {
            name: "lookup".to_string(),
            description: "Look up a value".to_string(),
            parameters: serde_json::json!({ "type": "object", "properties": { "value": { "type": "string" } } }),
        }];
        let result = convert_tools(&tools, false, true, None, false);
        let arr = result.as_array().unwrap();
        assert_eq!(
            arr[0]
                .get("eager_input_streaming")
                .and_then(|v| v.as_bool()),
            Some(true)
        );
    }

    #[test]
    fn convert_tools_omits_eager_input_streaming_when_not_supported() {
        let tools = vec![Tool {
            name: "lookup".to_string(),
            description: "Look up a value".to_string(),
            parameters: serde_json::json!({ "type": "object", "properties": { "value": { "type": "string" } } }),
        }];
        // Pass supports_eager_tool_input_streaming=false
        let result = convert_tools(&tools, false, false, None, false);
        let arr = result.as_array().unwrap();
        assert!(arr[0].get("eager_input_streaming").is_none());
    }

    #[test]
    fn convert_tools_sets_cache_control_on_last_tool() {
        let tools: Vec<Tool> = (0..2)
            .map(|i| Tool {
                name: format!("tool_{}", i),
                description: format!("Tool {}", i),
                parameters: serde_json::json!({ "type": "object", "properties": {} }),
            })
            .collect();
        let cc = CacheControl {
            cache_type: "ephemeral".to_string(),
            ttl: None,
        };
        let result = convert_tools(&tools, false, false, Some(&cc), false);
        let arr = result.as_array().unwrap();
        // Last tool should have cache_control; first should not (since it's not last)
        assert!(arr[0].get("cache_control").is_none());
        assert!(arr[1].get("cache_control").is_some());
        assert_eq!(
            arr[1]
                .get("cache_control")
                .and_then(|c| c.get("type"))
                .and_then(|t| t.as_str()),
            Some("ephemeral")
        );
    }

    #[test]
    fn convert_tools_omits_cache_control_on_fireworks() {
        let tools = vec![Tool {
            name: "tool".to_string(),
            description: "Tool".to_string(),
            parameters: serde_json::json!({}),
        }];
        let cc = CacheControl {
            cache_type: "ephemeral".to_string(),
            ttl: None,
        };
        // is_fireworks = true => no cache_control on tools
        let result = convert_tools(&tools, false, false, Some(&cc), true);
        let arr = result.as_array().unwrap();
        assert!(arr[0].get("cache_control").is_none());
    }

    #[test]
    fn convert_tools_empty_returns_empty_array() {
        let result = convert_tools(&[], false, false, None, false);
        assert!(result.as_array().unwrap().is_empty());
    }

    #[test]
    fn build_anthropic_headers_cloudflare_ai_gateway() {
        // Cloudflare AI gateway uses cf-aig-authorization, not x-api-key
        let model = Model {
            provider: "cloudflare-ai-gateway".to_string(),
            ..test_model()
        };
        let (headers, is_oauth) =
            build_anthropic_headers(&model, "some-key", false, true, None, None, None, None);
        assert!(!is_oauth);
        assert_eq!(
            headers.get("cf-aig-authorization").map(|s| s.as_str()),
            Some("Bearer some-key")
        );
        assert!(headers.get("x-api-key").is_none());
        assert!(headers.contains_key("anthropic-dangerous-direct-browser-access"));
    }

    #[test]
    fn build_anthropic_headers_github_copilot() {
        let model = Model {
            provider: "github-copilot".to_string(),
            ..test_model()
        };
        let (headers, is_oauth) =
            build_anthropic_headers(&model, "gh-token", false, false, None, None, None, None);
        assert!(!is_oauth);
        assert_eq!(
            headers.get("authorization").map(|s| s.as_str()),
            Some("Bearer gh-token")
        );
        assert!(headers.get("x-api-key").is_none());
    }

    #[test]
    fn build_anthropic_headers_with_session_affinity_fireworks() {
        let model = Model {
            provider: "fireworks".to_string(),
            ..test_model()
        };
        let (headers, _) = build_anthropic_headers(
            &model,
            "key",
            false,
            false,
            None,
            None,
            Some("session-123"),
            None,
        );
        // Fireworks gets session affinity headers
        assert_eq!(
            headers.get("x-session-affinity").map(|s| s.as_str()),
            Some("session-123")
        );
    }

    #[test]
    fn build_anthropic_headers_without_session_affinity_for_standard() {
        // Standard Anthropic does NOT send session affinity headers
        let (headers, _) = build_anthropic_headers(
            &test_model(),
            "key",
            false,
            false,
            None,
            None,
            Some("session-123"),
            None,
        );
        assert!(headers.get("x-session-affinity").is_none());
    }

    #[test]
    fn build_anthropic_headers_interleaved_beta() {
        let model = test_model();
        let (headers, _) = build_anthropic_headers(
            &model, "key", true,  // interleaved_thinking = true
            false, // fine_grained = false
            None, None, None, None,
        );
        let beta = headers
            .get("anthropic-beta")
            .map(|s| s.as_str())
            .unwrap_or("");
        assert!(beta.contains("interleaved-thinking"));
    }

    #[test]
    fn build_anthropic_headers_fine_grained_beta() {
        let model = test_model();
        let (headers, _) = build_anthropic_headers(
            &model, "key", false, true, // fine_grained = true
            None, None, None, None,
        );
        let beta = headers
            .get("anthropic-beta")
            .map(|s| s.as_str())
            .unwrap_or("");
        assert!(beta.contains("fine-grained-tool-streaming"));
    }

    // -----------------------------------------------------------------------
    // Cache control / convert_tool_result_content edge cases
    // -----------------------------------------------------------------------

    #[test]
    fn convert_tool_result_content_images_but_no_text_inserts_placeholder() {
        // When there are only images, a "(see attached image)" text should be inserted.
        let content = vec![MessageContent::Image(ImageContent {
            data: "iVBOR".to_string(),
            mime_type: "image/png".to_string(),
        })];
        let result = convert_tool_result_content(&content);
        let arr = result.as_array().unwrap();
        assert_eq!(arr.len(), 2);
        assert_eq!(
            arr[0].get("text").and_then(|t| t.as_str()),
            Some("(see attached image)")
        );
    }

    #[test]
    fn convert_tool_result_content_empty_content() {
        let content: Vec<MessageContent> = Vec::new();
        let result = convert_tool_result_content(&content);
        assert_eq!(result, Value::String("".to_string()));
    }

    // -----------------------------------------------------------------------
    // convert_messages: OAuth outbound tool name normalization
    // (corresponds to anthropic-tool-name-normalization.test.ts payload logic)
    // -----------------------------------------------------------------------

    #[test]
    fn convert_messages_oauth_tool_name_outbound_maps_to_cc_case() {
        // When is_oauth = true, tool name "read" should become "Read" on outbound.
        let model = test_model();
        let compat = get_anthropic_compat(&model);
        let context = Context {
            system_prompt: None,
            messages: vec![Message::Assistant(AssistantMessage {
                role: MessageRole::Assistant,
                content: vec![AssistantContentBlock::ToolCall(ToolCall {
                    id: "toolu_abc".to_string(),
                    name: "read".to_string(),
                    arguments: serde_json::json!({"path": "/tmp/test.txt"}),
                    thought_signature: None,
                })],
                api: "anthropic-messages".to_string(),
                provider: "anthropic".to_string(),
                model: model.id.clone(),
                response_model: None,
                response_id: None,
                usage: empty_usage(),
                stop_reason: StopReason::Stop,
                error_message: None,
                diagnostics: None,
                timestamp: Utc::now(),
            })],
            tools: vec![],
        };
        let result = convert_messages(&context.messages, &model, true, None, &compat);
        let content = result[0].get("content").and_then(|c| c.as_array()).unwrap();
        assert_eq!(
            content[0].get("name").and_then(|n| n.as_str()),
            Some("Read")
        );
    }

    #[test]
    fn convert_messages_non_oauth_tool_name_passthrough() {
        // Without OAuth, tool name should pass through unchanged.
        let model = test_model();
        let compat = get_anthropic_compat(&model);
        let context = Context {
            system_prompt: None,
            messages: vec![Message::Assistant(AssistantMessage {
                role: MessageRole::Assistant,
                content: vec![AssistantContentBlock::ToolCall(ToolCall {
                    id: "toolu_abc".to_string(),
                    name: "read".to_string(),
                    arguments: serde_json::json!({"path": "/tmp/test.txt"}),
                    thought_signature: None,
                })],
                api: "anthropic-messages".to_string(),
                provider: "anthropic".to_string(),
                model: model.id.clone(),
                response_model: None,
                response_id: None,
                usage: empty_usage(),
                stop_reason: StopReason::Stop,
                error_message: None,
                diagnostics: None,
                timestamp: Utc::now(),
            })],
            tools: vec![],
        };
        let result = convert_messages(&context.messages, &model, false, None, &compat);
        let content = result[0].get("content").and_then(|c| c.as_array()).unwrap();
        assert_eq!(
            content[0].get("name").and_then(|n| n.as_str()),
            Some("read")
        );
    }

    #[test]
    fn convert_messages_skips_empty_user_messages() {
        let model = test_model();
        let compat = get_anthropic_compat(&model);
        let context = Context {
            system_prompt: None,
            messages: vec![
                Message::User(UserMessage {
                    role: MessageRole::User,
                    content: vec![],
                    timestamp: Utc::now(),
                }),
                Message::User(UserMessage {
                    role: MessageRole::User,
                    content: vec![MessageContent::Text(TextContent {
                        text: "Hello".to_string(),
                        text_signature: None,
                    })],
                    timestamp: Utc::now(),
                }),
            ],
            tools: vec![],
        };
        let result = convert_messages(&context.messages, &model, false, None, &compat);
        assert_eq!(result.len(), 1);
        let content = result[0].get("content").and_then(|c| c.as_array()).unwrap();
        assert_eq!(content.len(), 1);
        assert_eq!(
            content[0].get("text").and_then(|t| t.as_str()),
            Some("Hello")
        );
    }

    #[test]
    fn convert_messages_skips_empty_assistant_text_blocks() {
        let model = test_model();
        let compat = get_anthropic_compat(&model);
        let context = Context {
            system_prompt: None,
            messages: vec![Message::Assistant(AssistantMessage {
                role: MessageRole::Assistant,
                content: vec![
                    AssistantContentBlock::Text(TextContent {
                        text: "   ".to_string(),
                        text_signature: None,
                    }),
                    AssistantContentBlock::Text(TextContent {
                        text: "Hello".to_string(),
                        text_signature: None,
                    }),
                ],
                api: "anthropic-messages".to_string(),
                provider: "anthropic".to_string(),
                model: model.id.clone(),
                response_model: None,
                response_id: None,
                usage: empty_usage(),
                stop_reason: StopReason::Stop,
                error_message: None,
                diagnostics: None,
                timestamp: Utc::now(),
            })],
            tools: vec![],
        };
        let result = convert_messages(&context.messages, &model, false, None, &compat);
        assert_eq!(result.len(), 1);
        let content = result[0].get("content").and_then(|c| c.as_array()).unwrap();
        assert_eq!(content.len(), 1);
        assert_eq!(
            content[0].get("text").and_then(|t| t.as_str()),
            Some("Hello")
        );
    }

    // -----------------------------------------------------------------------
    // Cache control tests
    // (corresponds to anthropic-cache-write-1h-cost.test.ts SSE shape tests)
    // -----------------------------------------------------------------------

    #[test]
    fn get_anthropic_compat_fireworks_sends_session_affinity() {
        let model = Model {
            provider: "fireworks".to_string(),
            ..test_model()
        };
        let compat = get_anthropic_compat(&model);
        assert!(compat.send_session_affinity_headers);
    }

    #[test]
    fn get_anthropic_compat_cloudflare_ai_gateway_with_anthropic_sends_session_affinity() {
        let model = Model {
            provider: "cloudflare-ai-gateway".to_string(),
            base_url: "https://gateway.example.com/anthropic".to_string(),
            ..test_model()
        };
        let compat = get_anthropic_compat(&model);
        assert!(compat.send_session_affinity_headers);
    }

    // -----------------------------------------------------------------------
    // get_cache_control tests
    // -----------------------------------------------------------------------

    #[test]
    fn get_cache_control_short_returns_ephemeral_no_ttl() {
        let model = test_model();
        let (retention, cc) = get_cache_control(&model, Some(CacheRetention::Short), None);
        assert_eq!(retention, CacheRetention::Short);
        assert!(cc.is_some());
        assert_eq!(cc.as_ref().unwrap().cache_type, "ephemeral");
        assert!(cc.as_ref().unwrap().ttl.is_none());
    }

    #[test]
    fn get_cache_control_long_returns_ephemeral_with_1h_ttl() {
        let model = test_model();
        let (retention, cc) = get_cache_control(&model, Some(CacheRetention::Long), None);
        assert_eq!(retention, CacheRetention::Long);
        assert!(cc.is_some());
        assert_eq!(cc.as_ref().unwrap().ttl.as_deref(), Some("1h"));
    }

    #[test]
    fn get_cache_control_long_on_fireworks_no_ttl() {
        let model = Model {
            provider: "fireworks".to_string(),
            ..test_model()
        };
        let (retention, cc) = get_cache_control(&model, Some(CacheRetention::Long), None);
        assert_eq!(retention, CacheRetention::Long);
        assert!(cc.is_some());
        assert!(cc.as_ref().unwrap().ttl.is_none());
    }

    #[test]
    fn get_cache_control_none_returns_none() {
        let model = test_model();
        let (retention, cc) = get_cache_control(&model, Some(CacheRetention::None), None);
        assert_eq!(retention, CacheRetention::None);
        assert!(cc.is_none());
    }

    // -----------------------------------------------------------------------
    // Cache write cost / SSE event parsing (process_anthropic_sse_stream
    // shape assertion via parse_anthropic_error for fallback)
    // -----------------------------------------------------------------------

    #[test]
    fn parse_anthropic_error_empty_body() {
        let msg = parse_anthropic_error(500, "");
        assert_eq!(msg, "Provider error (500)");
    }

    #[test]
    fn parse_anthropic_error_body_no_error_field() {
        let body = r#"{"type":"error","message":"oops"}"#;
        let msg = parse_anthropic_error(400, body);
        assert_eq!(msg, "400 {\"type\":\"error\",\"message\":\"oops\"}");
    }

    #[test]
    fn parse_anthropic_error_body_with_only_status() {
        // If there's a body but it's not JSON, use the raw text
        let msg = parse_anthropic_error(503, "rate limited");
        assert!(msg.contains("503"));
        assert!(msg.contains("rate limited"));
    }

    // -----------------------------------------------------------------------
    // build_params with system prompt
    // -----------------------------------------------------------------------

    #[test]
    fn build_params_with_system_prompt() {
        let model = test_model();
        let compat = get_anthropic_compat(&model);
        let context = Context {
            system_prompt: Some("You are a helpful assistant.".to_string()),
            messages: vec![],
            tools: vec![],
        };
        let params = build_params(
            &model,
            &context,
            false,
            &AnthropicOptions::default(),
            CacheRetention::Short,
            &compat,
            None,
        );
        let system = params.get("system").and_then(|s| s.as_array()).unwrap();
        assert_eq!(system.len(), 1);
        assert_eq!(
            system[0].get("text").and_then(|t| t.as_str()),
            Some("You are a helpful assistant.")
        );
    }

    #[test]
    fn build_params_oauth_system_prompt_includes_claude_code_intro() {
        let model = test_model();
        let compat = get_anthropic_compat(&model);
        let context = Context {
            system_prompt: Some("Do stuff.".to_string()),
            messages: vec![],
            tools: vec![],
        };
        let params = build_params(
            &model,
            &context,
            true,
            &AnthropicOptions::default(),
            CacheRetention::Short,
            &compat,
            None,
        );
        let system = params.get("system").and_then(|s| s.as_array()).unwrap();
        // OAuth adds a "You are Claude Code" intro block before the user's system prompt
        assert_eq!(system.len(), 2);
        assert_eq!(
            system[0].get("text").and_then(|t| t.as_str()),
            Some("You are Claude Code, Anthropic's official CLI for Claude.")
        );
        assert_eq!(
            system[1].get("text").and_then(|t| t.as_str()),
            Some("Do stuff.")
        );
    }

    // -----------------------------------------------------------------------
    // build_params: temperature with default (1.0) should be included
    // (corresponds to anthropic-temperature-compat.test.ts)
    // -----------------------------------------------------------------------

    #[test]
    fn temperature_with_default_value_is_included() {
        let model = test_model();
        let compat = get_anthropic_compat(&model);
        let options = AnthropicOptions {
            base: StreamOptions {
                temperature: Some(1.0),
                ..Default::default()
            },
            thinking_enabled: Some(false),
            ..Default::default()
        };
        let params = build_params(
            &model,
            &empty_context(),
            false,
            &options,
            CacheRetention::Short,
            &compat,
            None,
        );
        assert_eq!(
            params.get("temperature").and_then(|t| t.as_f64()),
            Some(1.0)
        );
    }

    // -----------------------------------------------------------------------
    // Tool choice passthrough
    // -----------------------------------------------------------------------

    #[test]
    fn build_params_with_tool_choice() {
        let model = test_model();
        let compat = get_anthropic_compat(&model);
        let options = AnthropicOptions {
            tool_choice: Some(serde_json::json!({"type": "any"})),
            ..Default::default()
        };
        let params = build_params(
            &model,
            &empty_context(),
            false,
            &options,
            CacheRetention::Short,
            &compat,
            None,
        );
        assert_eq!(
            params
                .get("tool_choice")
                .and_then(|t| t.get("type"))
                .and_then(|t| t.as_str()),
            Some("any")
        );
    }

    // -----------------------------------------------------------------------
    // build_params with metadata
    // -----------------------------------------------------------------------

    #[test]
    fn build_params_with_user_id_metadata() {
        let model = test_model();
        let compat = get_anthropic_compat(&model);
        let options = AnthropicOptions {
            base: StreamOptions {
                metadata: Some({
                    let mut m = std::collections::HashMap::new();
                    m.insert(
                        "user_id".to_string(),
                        serde_json::Value::String("user-abc".to_string()),
                    );
                    m
                }),
                ..Default::default()
            },
            ..Default::default()
        };
        let params = build_params(
            &model,
            &empty_context(),
            false,
            &options,
            CacheRetention::Short,
            &compat,
            None,
        );
        assert_eq!(
            params
                .get("metadata")
                .and_then(|m| m.get("user_id"))
                .and_then(|u| u.as_str()),
            Some("user-abc")
        );
    }

    // -----------------------------------------------------------------------
    // from_claude_code_name: "Find" should NOT match "Glob"
    // (corresponds to anthropic-tool-name-normalization.test.ts:
    //  "should NOT map find to Glob")
    // -----------------------------------------------------------------------

    #[test]
    fn from_claude_code_name_does_not_map_find_to_glob() {
        // The old behavior incorrectly mapped "find" <-> "Glob".
        // In the correct implementation, "Glob" is a CC tool name but "Find"
        // is NOT. A tool called "find" should remain "find".
        let tools = vec![Tool {
            name: "find".to_string(),
            description: "Find files".to_string(),
            parameters: serde_json::json!({}),
        }];
        // "Glob" is a CC tool name, but there's no tool named "Glob" in context.tools
        // So from_claude_code_name("Glob", ...) should return "Glob"
        let result = from_claude_code_name("Glob", &tools);
        assert_eq!(result, "Glob");
        // to_claude_code_name("find") should return "find" since "Find" is NOT a CC tool
        assert_eq!(to_claude_code_name("find"), "find");
    }

    // -----------------------------------------------------------------------
    // resolve_cache_retention with PI_CACHE_RETENTION env var
    // -----------------------------------------------------------------------

    #[test]
    fn resolve_cache_retention_env_var_long() {
        let env: ProviderEnv = {
            let mut m = std::collections::HashMap::new();
            m.insert("PI_CACHE_RETENTION".to_string(), "long".to_string());
            m
        };
        assert_eq!(
            resolve_cache_retention(None, Some(&env)),
            CacheRetention::Long
        );
    }

    // -----------------------------------------------------------------------
    // build_params with cache_control on system prompt and last message
    // -----------------------------------------------------------------------

    #[test]
    fn build_params_with_cache_control_adds_to_last_user_message_block() {
        let model = test_model();
        let compat = get_anthropic_compat(&model);
        let cc = CacheControl {
            cache_type: "ephemeral".to_string(),
            ttl: Some("1h".to_string()),
        };
        let context = Context {
            system_prompt: Some("System prompt.".to_string()),
            messages: vec![Message::User(UserMessage {
                role: MessageRole::User,
                content: vec![MessageContent::Text(TextContent {
                    text: "Hello".to_string(),
                    text_signature: None,
                })],
                timestamp: Utc::now(),
            })],
            tools: vec![],
        };
        let params = build_params(
            &model,
            &context,
            false,
            &AnthropicOptions::default(),
            CacheRetention::Long,
            &compat,
            Some(&cc),
        );
        let system = params.get("system").and_then(|s| s.as_array()).unwrap();
        assert!(system[0].get("cache_control").is_some());
        let messages = params.get("messages").and_then(|m| m.as_array()).unwrap();
        let last_msg = messages.last().unwrap();
        let content = last_msg.get("content").and_then(|c| c.as_array()).unwrap();
        assert!(
            content
                .last()
                .and_then(|b| b.get("cache_control"))
                .is_some()
        );
    }
}
