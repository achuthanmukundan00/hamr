//! Port of `../../packages/ai/src/providers/google.ts`.
//!
//! Google Generative AI (Gemini) provider backend.
//!
//! Unlike the TS source — which uses the `@google/genai` SDK — this port issues
//! raw HTTP requests with `reqwest` and parses Google's `streamGenerateContent?alt=sse`
//! Server-Sent Events stream manually. The shared message/tool/stop-reason/thinking
//! helpers already live in [`crate::providers::google_shared`] and are reused here.
//!
//! Entry points:
//! - [`stream`] (a.k.a. `stream_google`) — the full provider stream.
//! - [`stream_simple`] (a.k.a. `stream_simple_google`) — the `SimpleStreamOptions`
//!   wrapper that maps a reasoning level to Google thinking config.

use futures::StreamExt;
use regex::Regex;
use serde_json::Value;

use crate::models::{calculate_cost, clamp_thinking_level};
use crate::providers::google_shared::{
    GoogleThinkingLevel, convert_messages, convert_tools, is_thinking_part, map_stop_reason,
    map_tool_choice, retain_thought_signature, sanitize_surrogates,
};
use crate::providers::simple_options::build_base_options;
use crate::types::{
    AssistantContentBlock, AssistantMessage, AssistantMessageEvent, Context, DoneReason,
    ErrorReason, MessageRole, Model, ModelThinkingLevel, SimpleStreamOptions, StopReason,
    StreamOptions, TextContent, ThinkingBudgets, ThinkingContent, ToolCall, Usage, UsageCost,
};
use crate::utils::event_stream::{
    AssistantMessageEventStream, AssistantMessageEventStreamSender,
    create_assistant_message_event_stream,
};

// ---------------------------------------------------------------------------
// Options
// ---------------------------------------------------------------------------

/// Thinking sub-configuration for [`GoogleOptions`].
///
/// Mirrors the TS `thinking?: { enabled; budgetTokens?; level? }`.
#[derive(Clone, Debug, Default)]
pub struct GoogleThinking {
    pub enabled: bool,
    /// `-1` for dynamic, `0` to disable.
    pub budget_tokens: Option<i64>,
    pub level: Option<GoogleThinkingLevel>,
}

/// Google-specific stream options.
///
/// Mirrors the TS `GoogleOptions extends StreamOptions`.
#[derive(Clone, Debug, Default)]
pub struct GoogleOptions {
    pub base: StreamOptions,
    /// `"auto" | "none" | "any"`.
    pub tool_choice: Option<String>,
    pub thinking: Option<GoogleThinking>,
}

// ---------------------------------------------------------------------------
// Tool call ID counter (mirrors TS module-level `toolCallCounter`)
// ---------------------------------------------------------------------------

use std::sync::atomic::{AtomicU64, Ordering};
static TOOL_CALL_COUNTER: AtomicU64 = AtomicU64::new(0);

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

fn initial_output(model: &Model) -> AssistantMessage {
    AssistantMessage {
        role: MessageRole::Assistant,
        content: Vec::new(),
        api: "google-generative-ai".to_string(),
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
// Streaming-block bookkeeping
// ---------------------------------------------------------------------------

/// The currently-open text or thinking block during streaming.
///
/// The TS code keeps a `currentBlock: TextContent | ThinkingContent | null` that
/// is also pushed into `output.content`. In Rust we keep the live data here and
/// mirror it into `output.content` by index so the `partial` snapshots are accurate.
enum CurrentBlock {
    Text {
        text: String,
        signature: Option<String>,
    },
    Thinking {
        thinking: String,
        signature: Option<String>,
    },
}

// ---------------------------------------------------------------------------
// Public entry points
// ---------------------------------------------------------------------------

/// Stream a completion from a Google Generative AI model.
///
/// Mirrors the TS `streamGoogle`.
pub fn stream(
    model: Model,
    context: Context,
    options: Option<GoogleOptions>,
) -> AssistantMessageEventStream {
    let (sender, stream_out) = create_assistant_message_event_stream();
    let options = options.unwrap_or_default();

    tokio::spawn(async move {
        run_stream(model, context, options, sender).await;
    });

    stream_out
}

/// TS-named alias for [`stream`].
pub fn stream_google(
    model: Model,
    context: Context,
    options: Option<GoogleOptions>,
) -> AssistantMessageEventStream {
    stream(model, context, options)
}

/// Stream with simplified reasoning-level options.
///
/// Mirrors the TS `streamSimpleGoogle`: maps the unified `reasoning` level into
/// Google thinking config (level for Gemini 3 / Gemma 4, budget tokens otherwise).
pub fn stream_simple(
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
    let thinking_budgets = options.as_ref().and_then(|o| o.thinking_budgets);

    // `if (!options?.reasoning)` → thinking disabled.
    let reasoning = match reasoning {
        Some(r) => r,
        None => {
            let opts = GoogleOptions {
                base,
                tool_choice: None,
                thinking: Some(GoogleThinking {
                    enabled: false,
                    budget_tokens: None,
                    level: None,
                }),
            };
            return stream(model, context, Some(opts));
        }
    };

    // clampThinkingLevel(model, options.reasoning)
    let clamped = clamp_thinking_level(&model, ModelThinkingLevel::from(reasoning));
    // effort = clamped === "off" ? "high" : clamped
    let effort = clamped_to_effort(clamped);

    if is_gemini3_pro_model(&model) || is_gemini3_flash_model(&model) || is_gemma4_model(&model) {
        let opts = GoogleOptions {
            base,
            tool_choice: None,
            thinking: Some(GoogleThinking {
                enabled: true,
                budget_tokens: None,
                level: Some(get_thinking_level(effort, &model)),
            }),
        };
        return stream(model, context, Some(opts));
    }

    let budget = get_google_budget(&model, effort, thinking_budgets.as_ref());
    let opts = GoogleOptions {
        base,
        tool_choice: None,
        thinking: Some(GoogleThinking {
            enabled: true,
            budget_tokens: Some(budget),
            level: None,
        }),
    };
    stream(model, context, Some(opts))
}

/// TS-named alias for [`stream_simple`].
pub fn stream_simple_google(
    model: Model,
    context: Context,
    options: Option<SimpleStreamOptions>,
) -> AssistantMessageEventStream {
    stream_simple(model, context, options)
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
// Effort level (ClampedThinkingLevel = Exclude<ThinkingLevel, "xhigh">)
// ---------------------------------------------------------------------------

/// The non-`xhigh` thinking levels used for Google budget/level resolution.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Effort {
    Minimal,
    Low,
    Medium,
    High,
}

/// `effort = (clampedReasoning === "off" ? "high" : clampedReasoning)`, with
/// `xhigh` folded to `high` (Google never sees `xhigh`).
fn clamped_to_effort(level: ModelThinkingLevel) -> Effort {
    match level {
        ModelThinkingLevel::Off => Effort::High,
        ModelThinkingLevel::Minimal => Effort::Minimal,
        ModelThinkingLevel::Low => Effort::Low,
        ModelThinkingLevel::Medium => Effort::Medium,
        ModelThinkingLevel::High => Effort::High,
        ModelThinkingLevel::XHigh => Effort::High,
    }
}

// ---------------------------------------------------------------------------
// Model identification helpers
// ---------------------------------------------------------------------------

fn is_gemma4_model(model: &Model) -> bool {
    Regex::new(r"gemma-?4")
        .map(|re| re.is_match(&model.id.to_lowercase()))
        .unwrap_or(false)
}

fn is_gemini3_pro_model(model: &Model) -> bool {
    Regex::new(r"gemini-3(?:\.\d+)?-pro")
        .map(|re| re.is_match(&model.id.to_lowercase()))
        .unwrap_or(false)
}

fn is_gemini3_flash_model(model: &Model) -> bool {
    let id = model.id.to_lowercase();
    let matches_flash = Regex::new(r"gemini-3(?:\.\d+)?-flash")
        .map(|re| re.is_match(&id))
        .unwrap_or(false);
    matches_flash || id == "gemini-flash-latest" || id == "gemini-flash-lite-latest"
}

// ---------------------------------------------------------------------------
// Thinking config helpers
// ---------------------------------------------------------------------------

/// Mirrors the TS `getDisabledThinkingConfig`. Returns the `thinkingConfig` JSON.
fn get_disabled_thinking_config(model: &Model) -> Value {
    if is_gemini3_pro_model(model) {
        return serde_json::json!({ "thinkingLevel": "LOW" });
    }
    if is_gemini3_flash_model(model) {
        return serde_json::json!({ "thinkingLevel": "MINIMAL" });
    }
    if is_gemma4_model(model) {
        return serde_json::json!({ "thinkingLevel": "MINIMAL" });
    }
    // Gemini 2.x supports disabling via thinkingBudget = 0.
    serde_json::json!({ "thinkingBudget": 0 })
}

/// Mirrors the TS `getThinkingLevel`.
fn get_thinking_level(effort: Effort, model: &Model) -> GoogleThinkingLevel {
    if is_gemini3_pro_model(model) {
        return match effort {
            Effort::Minimal | Effort::Low => GoogleThinkingLevel::Low,
            Effort::Medium | Effort::High => GoogleThinkingLevel::High,
        };
    }
    if is_gemma4_model(model) {
        return match effort {
            Effort::Minimal | Effort::Low => GoogleThinkingLevel::Minimal,
            Effort::Medium | Effort::High => GoogleThinkingLevel::High,
        };
    }
    match effort {
        Effort::Minimal => GoogleThinkingLevel::Minimal,
        Effort::Low => GoogleThinkingLevel::Low,
        Effort::Medium => GoogleThinkingLevel::Medium,
        Effort::High => GoogleThinkingLevel::High,
    }
}

/// Mirrors the TS `getGoogleBudget`.
fn get_google_budget(
    model: &Model,
    effort: Effort,
    custom_budgets: Option<&ThinkingBudgets>,
) -> i64 {
    if let Some(custom) = custom_budgets {
        let value = match effort {
            Effort::Minimal => custom.minimal,
            Effort::Low => custom.low,
            Effort::Medium => custom.medium,
            Effort::High => custom.high,
        };
        if let Some(v) = value {
            return v as i64;
        }
    }

    if model.id.contains("2.5-pro") {
        return match effort {
            Effort::Minimal => 128,
            Effort::Low => 2048,
            Effort::Medium => 8192,
            Effort::High => 32768,
        };
    }
    if model.id.contains("2.5-flash-lite") {
        return match effort {
            Effort::Minimal => 512,
            Effort::Low => 2048,
            Effort::Medium => 8192,
            Effort::High => 24576,
        };
    }
    if model.id.contains("2.5-flash") {
        return match effort {
            Effort::Minimal => 128,
            Effort::Low => 2048,
            Effort::Medium => 8192,
            Effort::High => 24576,
        };
    }

    -1
}

// ---------------------------------------------------------------------------
// Request building
// ---------------------------------------------------------------------------

/// Build the `config` portion of the GenAI request body.
///
/// Mirrors the TS `buildParams` (the GenerateContentConfig assembly). Returns the
/// `config` object as JSON. Errors are propagated as `Err(message)` (the TS throws
/// on `signal.aborted`).
fn build_config(
    model: &Model,
    context: &Context,
    options: &GoogleOptions,
) -> Result<Value, String> {
    let mut config = serde_json::Map::new();

    // generationConfig (temperature / maxOutputTokens) — Google nests these under
    // `generationConfig` for the REST API; the SDK flattens them but the wire form
    // is `generationConfig`.
    let mut generation_config = serde_json::Map::new();
    if let Some(temp) = options.base.temperature {
        generation_config.insert("temperature".into(), serde_json::json!(temp));
    }
    if let Some(max_tokens) = options.base.max_tokens {
        generation_config.insert("maxOutputTokens".into(), serde_json::json!(max_tokens));
    }
    if !generation_config.is_empty() {
        config.insert("generationConfig".into(), Value::Object(generation_config));
    }

    // systemInstruction
    if let Some(system_prompt) = &context.system_prompt {
        if !system_prompt.is_empty() {
            config.insert(
                "systemInstruction".into(),
                serde_json::json!({
                    "parts": [{ "text": sanitize_surrogates(system_prompt) }]
                }),
            );
        }
    }

    // tools
    let has_tools = !context.tools.is_empty();
    if has_tools {
        // Read `compat?.useParameters` from model.compat; defaults to false
        // (parametersJsonSchema), matching the common case.
        let use_parameters = model
            .compat
            .as_ref()
            .and_then(|c| c.get("useParameters"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        if let Some(tools) = convert_tools(&context.tools, use_parameters) {
            config.insert("tools".into(), Value::Array(tools));
        }
    }

    // toolConfig
    if has_tools {
        if let Some(tool_choice) = &options.tool_choice {
            config.insert(
                "toolConfig".into(),
                serde_json::json!({
                    "functionCallingConfig": {
                        "mode": map_tool_choice(tool_choice)
                    }
                }),
            );
        }
    }
    // else: TS sets config.toolConfig = undefined → simply omit it.

    // thinkingConfig
    if let Some(thinking) = &options.thinking {
        if thinking.enabled && model.reasoning {
            let mut thinking_config = serde_json::Map::new();
            thinking_config.insert("includeThoughts".into(), Value::Bool(true));
            if let Some(level) = thinking.level {
                thinking_config.insert(
                    "thinkingLevel".into(),
                    Value::String(level.as_str().to_owned()),
                );
            } else if let Some(budget) = thinking.budget_tokens {
                thinking_config.insert("thinkingBudget".into(), serde_json::json!(budget));
            }
            config.insert("thinkingConfig".into(), Value::Object(thinking_config));
        } else if model.reasoning && !thinking.enabled {
            config.insert("thinkingConfig".into(), get_disabled_thinking_config(model));
        }
    }

    // abortSignal: the TS throws synchronously if already aborted; we mirror that.
    if let Some(signal) = &options.base.signal {
        if *signal.borrow() {
            return Err("Request aborted".to_string());
        }
    }

    Ok(Value::Object(config))
}

/// Build the full request body (`{ contents, ...config }` — the REST shape merges
/// `config` keys at the top level alongside `contents`).
fn build_request_body(
    model: &Model,
    context: &Context,
    options: &GoogleOptions,
) -> Result<Value, String> {
    let contents = convert_messages(model, context);
    let config = build_config(model, context, options)?;

    let mut body = serde_json::Map::new();
    if let Value::Object(config_map) = config {
        for (k, v) in config_map {
            body.insert(k, v);
        }
    }
    body.insert("contents".into(), Value::Array(contents));
    Ok(Value::Object(body))
}

/// Compute the streaming endpoint URL.
///
/// If `model.base_url` is set, it already includes the version path (matching the
/// TS `apiVersion = ""`); otherwise default to the public GenAI endpoint.
fn build_url(model: &Model) -> String {
    let base = if !model.base_url.is_empty() {
        model.base_url.trim_end_matches('/').to_string()
    } else {
        "https://generativelanguage.googleapis.com/v1beta".to_string()
    };
    format!("{}/models/{}:streamGenerateContent?alt=sse", base, model.id)
}

// ---------------------------------------------------------------------------
// Stream driver
// ---------------------------------------------------------------------------

async fn run_stream(
    model: Model,
    context: Context,
    options: GoogleOptions,
    mut sender: AssistantMessageEventStreamSender,
) {
    let mut output = initial_output(&model);

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
        // Stop / Error / Aborted → Stop (Error/Aborted are handled on the error path).
        _ => DoneReason::Stop,
    }
}

/// The core streaming logic. Returns `Err(message)` for any failure, which the
/// caller converts into a terminal error event.
async fn run_stream_inner(
    model: &Model,
    context: &Context,
    options: &GoogleOptions,
    sender: &mut AssistantMessageEventStreamSender,
    output: &mut AssistantMessage,
) -> Result<(), String> {
    let api_key = match &options.base.api_key {
        Some(k) if !k.is_empty() => k.clone(),
        _ => return Err(format!("No API key for provider: {}", model.provider)),
    };

    let mut body = build_request_body(model, context, options)?;

    // onPayload hook: allow inspection/replacement before sending.
    if let Some(on_payload) = &options.base.on_payload {
        if let Some(next) = on_payload(body.clone(), model.clone()).await {
            body = next;
        }
    }

    let url = build_url(model);

    let client = reqwest::Client::new();
    let mut request = client
        .post(&url)
        .header("x-goog-api-key", &api_key)
        .header("content-type", "application/json");

    // Merge model.headers then options.headers (options win), mirroring createClient.
    if let Some(model_headers) = &model.headers {
        for (k, v) in model_headers {
            request = request.header(k, v);
        }
    }
    if let Some(opt_headers) = &options.base.headers {
        for (k, v) in opt_headers {
            request = request.header(k, v);
        }
    }

    // Body is serialized manually since reqwest's `json` feature is not enabled.
    let body_bytes = serde_json::to_vec(&body).map_err(|e| e.to_string())?;
    request = request.body(body_bytes);

    // Send (raced against the abort signal).
    let response = match send_with_abort(request, options.base.signal.clone()).await {
        Ok(resp) => resp,
        Err(e) => return Err(e),
    };

    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        return Err(format!("Google API error {}: {}", status.as_u16(), text));
    }

    // Emit `start`.
    sender.push(AssistantMessageEvent::Start {
        partial: output.clone(),
    });

    let mut current_block: Option<CurrentBlock> = None;

    // SSE parsing: accumulate bytes into a line buffer, split on `\n`, collect
    // `data:` payloads. A blank line terminates an event.
    let mut byte_stream = response.bytes_stream();
    let mut buffer: Vec<u8> = Vec::new();

    loop {
        // Abort check between chunks.
        if let Some(signal) = &options.base.signal {
            if *signal.borrow() {
                return Err("Request was aborted".to_string());
            }
        }

        let next = if let Some(signal) = options.base.signal.clone() {
            let mut signal = signal;
            tokio::select! {
                chunk = byte_stream.next() => chunk,
                _ = wait_for_abort(&mut signal) => {
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

        // Process complete lines.
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
            process_chunk(model, &chunk_json, output, &mut current_block, sender);
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
                    process_chunk(model, &chunk_json, output, &mut current_block, sender);
                }
            }
        }
    }

    // Close any open text/thinking block.
    if let Some(block) = current_block.take() {
        finish_current_block(block, output, sender);
    }

    // Abort/error finalization (mirrors the TS post-loop throws).
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
            // Sender dropped: never aborts.
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
// Chunk processing
// ---------------------------------------------------------------------------

/// Process a single parsed SSE chunk, mutating `output` and emitting events.
///
/// Mirrors the body of the TS `for await (const chunk of googleStream)` loop.
fn process_chunk(
    model: &Model,
    chunk: &Value,
    output: &mut AssistantMessage,
    current_block: &mut Option<CurrentBlock>,
    sender: &mut AssistantMessageEventStreamSender,
) {
    // output.responseId ||= chunk.responseId
    if output.response_id.is_none() {
        if let Some(id) = chunk.get("responseId").and_then(|v| v.as_str()) {
            if !id.is_empty() {
                output.response_id = Some(id.to_owned());
            }
        }
    }

    let candidate = chunk
        .get("candidates")
        .and_then(|c| c.as_array())
        .and_then(|arr| arr.first());

    if let Some(candidate) = candidate {
        if let Some(parts) = candidate
            .get("content")
            .and_then(|c| c.get("parts"))
            .and_then(|p| p.as_array())
        {
            for part in parts {
                process_part(part, output, current_block, sender);
            }
        }

        // finishReason
        if let Some(finish) = candidate.get("finishReason").and_then(|v| v.as_str()) {
            if !finish.is_empty() {
                output.stop_reason = map_stop_reason(finish);
                if output
                    .content
                    .iter()
                    .any(|b| matches!(b, AssistantContentBlock::ToolCall(_)))
                {
                    output.stop_reason = StopReason::ToolUse;
                }
            }
        }
    }

    // usageMetadata
    if let Some(meta) = chunk.get("usageMetadata") {
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

        output.usage = Usage {
            input: prompt.saturating_sub(cached),
            output: candidates.saturating_add(thoughts),
            cache_read: cached,
            cache_write: 0,
            cache_write_1h: None,
            total_tokens: total,
            cost: UsageCost {
                input: 0.0,
                output: 0.0,
                cache_read: 0.0,
                cache_write: 0.0,
                total: 0.0,
            },
        };
        calculate_cost(model, &mut output.usage);
    }
}

/// Process a single `Part`, updating text/thinking/tool-call state.
fn process_part(
    part: &Value,
    output: &mut AssistantMessage,
    current_block: &mut Option<CurrentBlock>,
    sender: &mut AssistantMessageEventStreamSender,
) {
    // ----- text / thinking -----
    if let Some(text) = part.get("text").and_then(|t| t.as_str()) {
        let is_thinking = is_thinking_part(part);
        let part_signature = part.get("thoughtSignature").and_then(|s| s.as_str());

        // Determine whether we need to open a new block.
        let needs_new_block = match current_block {
            None => true,
            Some(CurrentBlock::Thinking { .. }) => !is_thinking,
            Some(CurrentBlock::Text { .. }) => is_thinking,
        };

        if needs_new_block {
            if let Some(block) = current_block.take() {
                finish_current_block(block, output, sender);
            }
            if is_thinking {
                *current_block = Some(CurrentBlock::Thinking {
                    thinking: String::new(),
                    signature: None,
                });
                output
                    .content
                    .push(AssistantContentBlock::Thinking(ThinkingContent {
                        thinking: String::new(),
                        thinking_signature: None,
                        redacted: false,
                    }));
                let idx = output.content.len() - 1;
                sender.push(AssistantMessageEvent::ThinkingStart {
                    content_index: idx,
                    partial: output.clone(),
                });
            } else {
                *current_block = Some(CurrentBlock::Text {
                    text: String::new(),
                    signature: None,
                });
                output
                    .content
                    .push(AssistantContentBlock::Text(TextContent {
                        text: String::new(),
                        text_signature: None,
                    }));
                let idx = output.content.len() - 1;
                sender.push(AssistantMessageEvent::TextStart {
                    content_index: idx,
                    partial: output.clone(),
                });
            }
        }

        let idx = output.content.len() - 1;
        match current_block {
            Some(CurrentBlock::Thinking {
                thinking,
                signature,
            }) => {
                thinking.push_str(text);
                *signature = retain_thought_signature(signature.as_deref(), part_signature);
                // Mirror into output.content.
                if let Some(AssistantContentBlock::Thinking(tc)) = output.content.get_mut(idx) {
                    tc.thinking = thinking.clone();
                    tc.thinking_signature = signature.clone();
                }
                sender.push(AssistantMessageEvent::ThinkingDelta {
                    content_index: idx,
                    delta: text.to_owned(),
                    partial: output.clone(),
                });
            }
            Some(CurrentBlock::Text {
                text: buf,
                signature,
            }) => {
                buf.push_str(text);
                *signature = retain_thought_signature(signature.as_deref(), part_signature);
                if let Some(AssistantContentBlock::Text(tc)) = output.content.get_mut(idx) {
                    tc.text = buf.clone();
                    tc.text_signature = signature.clone();
                }
                sender.push(AssistantMessageEvent::TextDelta {
                    content_index: idx,
                    delta: text.to_owned(),
                    partial: output.clone(),
                });
            }
            None => {}
        }
    }

    // ----- functionCall -----
    if part.get("functionCall").is_some() {
        // Close any open text/thinking block first.
        if let Some(block) = current_block.take() {
            finish_current_block(block, output, sender);
        }

        let tool_call = part_to_tool_call(part, output);
        output
            .content
            .push(AssistantContentBlock::ToolCall(tool_call.clone()));
        let idx = output.content.len() - 1;

        sender.push(AssistantMessageEvent::ToolCallStart {
            content_index: idx,
            partial: output.clone(),
        });
        let args_str = serde_json::to_string(&tool_call.arguments).unwrap_or_else(|_| "{}".into());
        sender.push(AssistantMessageEvent::ToolCallDelta {
            content_index: idx,
            delta: args_str,
            partial: output.clone(),
        });
        sender.push(AssistantMessageEvent::ToolCallEnd {
            content_index: idx,
            tool_call,
            partial: output.clone(),
        });
    }
}

/// Build a `ToolCall` from a functionCall part, generating a unique id when the
/// provided id is missing or a duplicate. Mirrors the TS id logic exactly.
fn part_to_tool_call(part: &Value, output: &AssistantMessage) -> ToolCall {
    let fc = part.get("functionCall");
    let name = fc
        .and_then(|f| f.get("name"))
        .and_then(|n| n.as_str())
        .unwrap_or("")
        .to_owned();
    let args = fc
        .and_then(|f| f.get("args"))
        .cloned()
        .unwrap_or_else(|| Value::Object(Default::default()));

    let provided_id = fc.and_then(|f| f.get("id")).and_then(|id| id.as_str());

    let is_duplicate = |id: &str| {
        output.content.iter().any(|b| match b {
            AssistantContentBlock::ToolCall(tc) => tc.id == id,
            _ => false,
        })
    };

    let needs_new_id = match provided_id {
        None => true,
        Some("") => true,
        Some(id) => is_duplicate(id),
    };

    let id = if needs_new_id {
        let counter = TOOL_CALL_COUNTER.fetch_add(1, Ordering::SeqCst) + 1;
        let ts = chrono::Utc::now().timestamp_millis();
        format!("{}_{}_{}", name, ts, counter)
    } else {
        provided_id.unwrap_or("").to_owned()
    };

    let thought_signature = part
        .get("thoughtSignature")
        .and_then(|s| s.as_str())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_owned());

    ToolCall {
        id,
        name,
        arguments: args,
        thought_signature,
    }
}

/// Emit the appropriate `*_end` event for a closing block.
fn finish_current_block(
    block: CurrentBlock,
    output: &AssistantMessage,
    sender: &mut AssistantMessageEventStreamSender,
) {
    let idx = output.content.len().saturating_sub(1);
    match block {
        CurrentBlock::Text { text, .. } => {
            sender.push(AssistantMessageEvent::TextEnd {
                content_index: idx,
                content: text,
                partial: output.clone(),
            });
        }
        CurrentBlock::Thinking { thinking, .. } => {
            sender.push(AssistantMessageEvent::ThinkingEnd {
                content_index: idx,
                content: thinking,
                partial: output.clone(),
            });
        }
    }
}
