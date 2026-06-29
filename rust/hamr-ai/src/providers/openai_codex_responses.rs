//! Port of `packages/ai/src/providers/openai-codex-responses.ts`.
//!
//! OpenAI Codex Responses provider. Uses SSE as the primary transport with
//! WebSocket fallback support. OAuth token management is handled by
//! [`crate::utils::oauth::openai_codex`].
//!
//! Entry points:
//! - [`stream`] / [`stream_openai_codex_responses`]
//! - [`stream_simple`] / [`stream_simple_openai_codex_responses`]

use base64::Engine as _;
use chrono::Utc;
use futures::StreamExt;
use regex::Regex;
use serde::Serialize;
use serde_json::{Map, Value};
use std::collections::{HashMap, HashSet};
use std::sync::{LazyLock, Mutex};
use tokio::sync::watch;

use crate::models::clamp_thinking_level;
use crate::providers::openai_prompt_cache::clamp_openai_prompt_cache_key;
use crate::providers::openai_responses_shared::{
    ConvertResponsesMessagesOptions, OpenAIResponsesStreamOptions, ResponseStreamEvent,
    ResponsesStreamError, convert_responses_messages, convert_responses_tools,
    process_responses_stream,
};
use crate::providers::simple_options::build_base_options;
use crate::types::{
    AssistantMessage, AssistantMessageEvent, Context, DoneReason, ErrorReason, MessageRole, Model,
    ModelThinkingLevel, SimpleStreamOptions, StopReason, StreamOptions, Transport, Usage,
    UsageCost,
};
use crate::utils::diagnostics::{
    append_assistant_message_diagnostic, create_assistant_message_diagnostic,
};
use crate::utils::event_stream::{
    AssistantMessageEventStream, AssistantMessageEventStreamSender,
    create_assistant_message_event_stream,
};
use crate::utils::headers::headers_to_record;

// ============================================================================
// Constants
// ============================================================================

const DEFAULT_CODEX_BASE_URL: &str = "https://chatgpt.com/backend-api";
const JWT_CLAIM_PATH: &str = "https://api.openai.com/auth";
const DEFAULT_MAX_RETRIES: u32 = 0;
const BASE_DELAY_MS: u64 = 1000;
const DEFAULT_MAX_RETRY_DELAY_MS: u64 = 60_000;
const DEFAULT_SSE_HEADER_TIMEOUT_MS: u64 = 20_000;
#[allow(dead_code)]
const DEFAULT_WEBSOCKET_CONNECT_TIMEOUT_MS: u64 = 15_000;
#[allow(dead_code)]
const WEBSOCKET_MESSAGE_TOO_BIG_CLOSE_CODE: u16 = 1009;

static CODEX_TOOL_CALL_PROVIDERS: LazyLock<HashSet<String>> = LazyLock::new(|| {
    let mut s = HashSet::new();
    s.insert("openai".to_string());
    s.insert("openai-codex".to_string());
    s.insert("opencode".to_string());
    s
});

const CODEX_RESPONSE_STATUSES: &[&str] = &[
    "completed",
    "incomplete",
    "failed",
    "cancelled",
    "queued",
    "in_progress",
];

// ============================================================================
// Options
// ============================================================================

/// Options specific to the OpenAI Codex Responses provider.
#[derive(Clone, Debug)]
pub struct CodexResponsesOptions {
    pub base: StreamOptions,
    pub reasoning_effort: Option<String>,
    pub reasoning_summary: Option<String>,
    pub service_tier: Option<String>,
    pub text_verbosity: Option<String>,
}

impl Default for CodexResponsesOptions {
    fn default() -> Self {
        Self {
            base: StreamOptions::default(),
            reasoning_effort: None,
            reasoning_summary: None,
            service_tier: None,
            text_verbosity: None,
        }
    }
}

/// Request body for the Codex API.
#[derive(Debug, Clone, Serialize)]
#[allow(dead_code)]
struct RequestBody {
    model: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    store: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stream: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    instructions: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    previous_response_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    input: Option<Vec<Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_choice: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    parallel_tool_calls: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    reasoning: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    service_tier: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    text: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    include: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    prompt_cache_key: Option<String>,
    /// Any extra fields.
    #[serde(flatten)]
    extra: HashMap<String, Value>,
}

// ============================================================================
// Retry helpers
// ============================================================================

fn is_terminal_rate_limit_error(error_text: &str) -> bool {
    Regex::new(
        r"GoUsageLimitError|FreeUsageLimitError|Monthly usage limit reached|available balance|insufficient_quota|out of budget|quota exceeded|billing",
    )
    .map(|re| re.is_match(error_text))
    .unwrap_or(false)
}

fn is_retryable_error(status: u16, error_text: &str) -> bool {
    if status == 429 && is_terminal_rate_limit_error(error_text) {
        return false;
    }
    if matches!(status, 429 | 500 | 502 | 503 | 504) {
        return true;
    }
    Regex::new(r"rate.?limit|overloaded|service.?unavailable|upstream.?connect|connection.?refused")
        .map(|re| re.is_match(error_text))
        .unwrap_or(false)
}

fn get_retry_after_delay_ms(headers: &reqwest::header::HeaderMap) -> Option<u64> {
    if let Some(val) = headers.get("retry-after-ms") {
        if let Ok(s) = val.to_str() {
            if let Ok(millis) = s.parse::<f64>() {
                if millis.is_finite() {
                    return Some((millis * 1000.0).max(0.0) as u64);
                }
            }
        }
    }
    let retry_after = headers.get("retry-after")?.to_str().ok()?;
    if let Ok(seconds) = retry_after.parse::<f64>() {
        if seconds.is_finite() {
            return Some((seconds * 1000.0).max(0.0) as u64);
        }
    }
    None
}

fn cap_retry_delay_ms(delay_ms: u64, options: &CodexResponsesOptions) -> u64 {
    let max_retry_delay_ms = options
        .base
        .max_retry_delay_ms
        .unwrap_or(DEFAULT_MAX_RETRY_DELAY_MS);
    if max_retry_delay_ms > 0 {
        delay_ms.min(max_retry_delay_ms)
    } else {
        delay_ms
    }
}

// ============================================================================
// Usage / initial output helpers
// ============================================================================

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
        api: "openai-codex-responses".to_string(),
        provider: model.provider.clone(),
        model: model.id.clone(),
        response_model: None,
        response_id: None,
        usage: empty_usage(),
        stop_reason: StopReason::Stop,
        error_message: None,
        diagnostics: None,
        timestamp: Utc::now(),
    }
}

// ============================================================================
// Error types
// ============================================================================

#[derive(Debug, thiserror::Error)]
#[allow(dead_code)]
enum CodexStreamError {
    #[error("Request was aborted")]
    Aborted,
    #[error("{0}")]
    Transport(String),
    #[error("Codex API error: {message} (code={code:?})")]
    Api {
        message: String,
        code: Option<String>,
    },
    #[error("Codex protocol error: {message}")]
    Protocol { message: String },
    #[error("{0}")]
    Other(String),
}

fn is_api_or_protocol_error(e: &CodexStreamError) -> bool {
    matches!(
        e,
        CodexStreamError::Api { .. } | CodexStreamError::Protocol { .. }
    )
}

// ============================================================================
// Public entry points
// ============================================================================

pub fn stream(
    model: Model,
    context: Context,
    options: Option<CodexResponsesOptions>,
) -> AssistantMessageEventStream {
    let (sender, stream_out) = create_assistant_message_event_stream();
    let options = options.unwrap_or_default();

    tokio::spawn(async move {
        run_stream(model, context, options, sender).await;
    });

    stream_out
}

pub fn stream_openai_codex_responses(
    model: Model,
    context: Context,
    options: Option<CodexResponsesOptions>,
) -> AssistantMessageEventStream {
    stream(model, context, options)
}

pub fn stream_simple(
    model: Model,
    context: Context,
    options: Option<SimpleStreamOptions>,
) -> AssistantMessageEventStream {
    // Using the base StreamOptions approach: extract api_key directly
    let api_key = options.as_ref().and_then(|o| o.base.api_key.clone());
    let has_key = api_key.as_ref().map_or(false, |k| !k.is_empty());
    if !has_key {
        return error_stream(
            &model,
            format!("No API key for provider: {}", model.provider),
        );
    }

    let base = build_base_options(&model, options.as_ref(), api_key.as_deref());
    let reasoning = options.as_ref().and_then(|o| o.reasoning);
    let clamped_reasoning =
        reasoning.map(|r| clamp_thinking_level(&model, ModelThinkingLevel::from(r)));
    let reasoning_effort = match clamped_reasoning {
        Some(ModelThinkingLevel::Off) => None,
        Some(level) => Some(level_to_effort_string(level)),
        None => None,
    };

    stream(
        model,
        context,
        Some(CodexResponsesOptions {
            base,
            reasoning_effort,
            reasoning_summary: None,
            service_tier: None,
            text_verbosity: None,
        }),
    )
}

pub fn stream_simple_openai_codex_responses(
    model: Model,
    context: Context,
    options: Option<SimpleStreamOptions>,
) -> AssistantMessageEventStream {
    stream_simple(model, context, options)
}

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

fn level_to_effort_string(level: ModelThinkingLevel) -> String {
    match level {
        ModelThinkingLevel::Off => "none",
        ModelThinkingLevel::Minimal => "minimal",
        ModelThinkingLevel::Low => "low",
        ModelThinkingLevel::Medium => "medium",
        ModelThinkingLevel::High => "high",
        ModelThinkingLevel::XHigh => "xhigh",
    }
    .to_string()
}

// ============================================================================
// Run stream (async core)
// ============================================================================

async fn run_stream(
    model: Model,
    context: Context,
    options: CodexResponsesOptions,
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
            let aborted = matches!(&err, CodexStreamError::Aborted);
            let aborted_signal = options
                .base
                .signal
                .as_ref()
                .map(|s| *s.borrow())
                .unwrap_or(false);
            let is_aborted = aborted || aborted_signal;

            output.stop_reason = if is_aborted {
                StopReason::Aborted
            } else {
                StopReason::Error
            };
            let msg = err.to_string();
            output.error_message = Some(msg);

            let reason = if is_aborted {
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

async fn run_stream_inner(
    model: &Model,
    context: &Context,
    options: &CodexResponsesOptions,
    sender: &mut AssistantMessageEventStreamSender,
    output: &mut AssistantMessage,
) -> Result<(), CodexStreamError> {
    let api_key = options
        .base
        .api_key
        .as_ref()
        .filter(|k| !k.is_empty())
        .ok_or_else(|| {
            CodexStreamError::Other(format!("No API key for provider: {}", model.provider))
        })?;

    let account_id = extract_account_id(api_key)?;

    // Build initial request body as Value
    let body_value = build_request_body(model, context, options)?;

    // onPayload hook
    let body_value = if let Some(on_payload) = &options.base.on_payload {
        on_payload(body_value.clone(), model.clone())
            .await
            .unwrap_or(body_value)
    } else {
        body_value
    };

    let body_json =
        serde_json::to_string(&body_value).map_err(|e| CodexStreamError::Other(e.to_string()))?;
    let ws_request_id = create_codex_request_id();

    // Decide transport
    let transport = options.base.transport.unwrap_or(Transport::Auto);
    let websocket_disabled = transport != Transport::Sse
        && is_websocket_sse_fallback_active(options.base.session_id.as_deref());

    // Try WebSocket first (if allowed)
    if transport != Transport::Sse && !websocket_disabled {
        let mut ws_started = false;
        match try_websocket_impl(
            model,
            &body_value,
            &account_id,
            &api_key,
            &ws_request_id,
            output,
            sender,
            &mut ws_started,
            options,
        )
        .await
        {
            Ok(()) => return Ok(()),
            Err(ws_err) => {
                if matches!(&ws_err, CodexStreamError::Aborted) || is_api_or_protocol_error(&ws_err)
                {
                    return Err(ws_err);
                }

                let io_err = std::io::Error::new(std::io::ErrorKind::Other, ws_err.to_string());
                let diagnostic = create_assistant_message_diagnostic(
                    "provider_transport_failure",
                    &io_err,
                    Some({
                        let mut d = HashMap::new();
                        d.insert(
                            "configuredTransport".into(),
                            Value::String(format!("{transport:?}")),
                        );
                        d.insert(
                            "fallbackTransport".into(),
                            if ws_started {
                                Value::Null
                            } else {
                                Value::String("sse".into())
                            },
                        );
                        d.insert("eventsEmitted".into(), Value::Bool(ws_started));
                        d.insert(
                            "phase".into(),
                            Value::String(
                                if ws_started {
                                    "after_message_stream_start"
                                } else {
                                    "before_message_stream_start"
                                }
                                .to_string(),
                            ),
                        );
                        d.insert("requestBytes".into(), serde_json::json!(body_json.len()));
                        d
                    }),
                );
                append_assistant_message_diagnostic(output, diagnostic);

                record_websocket_failure(options.base.session_id.as_deref());
                if ws_started {
                    return Err(ws_err);
                }
                record_websocket_sse_fallback(options.base.session_id.as_deref());
            }
        }
    }

    // SSE path with retry
    let sse_url = resolve_codex_url(&model.base_url);
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .map_err(|e| CodexStreamError::Other(e.to_string()))?;

    let max_retries = options.base.max_retries.unwrap_or(DEFAULT_MAX_RETRIES);
    let _last_err: Option<String> = None;

    for attempt in 0..=max_retries {
        check_abort(options)?;

        let sse_headers = build_sse_headers(
            model.headers.as_ref(),
            options.base.headers.as_ref(),
            &account_id,
            api_key,
            options.base.session_id.as_deref(),
        );

        // Use reqwest timeout directly; SSE header timeout is handled by the
        // client timeout setting and the abort signal.
        let send_result = {
            let mut req = client.post(&sse_url);
            for (k, v) in &sse_headers {
                req = req.header(k.as_str(), v.as_str());
            }
            req = req.body(body_json.clone());

            if let Some(sig) = &options.base.signal {
                let mut sig_clone = sig.clone();
                tokio::select! {
                    resp = req.send() => Ok(resp.map_err(|e| e.to_string())),
                    _ = wait_for_abort(&mut sig_clone) => Err("Request was aborted".to_string()),
                }
            } else {
                req.send().await.map_err(|e| e.to_string()).map(Ok)
            }
        };

        let resp = match send_result {
            Ok(Ok(resp)) => resp,
            Ok(Err(msg)) => {
                // Network retry
                if attempt < max_retries {
                    let delay = BASE_DELAY_MS * 2u64.pow(attempt);
                    sleep_ms(delay, options.base.signal.clone()).await;
                    continue;
                }
                return Err(CodexStreamError::Transport(msg));
            }
            Err(_msg) => {
                // Timeout or abort
                let timeout_msg = format!(
                    "Codex SSE response headers timed out after {}ms",
                    DEFAULT_SSE_HEADER_TIMEOUT_MS
                );
                if attempt < max_retries {
                    let delay = BASE_DELAY_MS * 2u64.pow(attempt);
                    sleep_ms(delay, options.base.signal.clone()).await;
                    continue;
                }
                return Err(CodexStreamError::Transport(timeout_msg));
            }
        };

        // onResponse callback
        if let Some(on_response) = &options.base.on_response {
            let status = resp.status().as_u16();
            let headers = headers_to_record(resp.headers());
            on_response(
                crate::types::ProviderResponse { status, headers },
                model.clone(),
            )
            .await;
        }

        if resp.status().is_success() {
            // Process SSE stream
            return process_sse_response(resp, output, sender, model, options).await;
        }

        let status = resp.status().as_u16();
        // Extract retry-after headers before consuming the response body
        let retry_header_delay = get_retry_after_delay_ms(resp.headers());
        let error_text = resp.text().await.unwrap_or_default();

        if attempt < max_retries && is_retryable_error(status, &error_text) {
            let delay_ms = retry_header_delay.unwrap_or_else(|| BASE_DELAY_MS * 2u64.pow(attempt));

            let delay_ms = if status == 429 {
                cap_retry_delay_ms(delay_ms, options)
            } else {
                delay_ms
            };

            sleep_ms(delay_ms, options.base.signal.clone()).await;
            continue;
        }

        // Parse error for friendly message
        let err_info = parse_error_response(&error_text, status);
        let msg = err_info
            .friendly_message
            .unwrap_or_else(|| err_info.message.clone());
        return Err(CodexStreamError::Other(msg));
    }

    Err(CodexStreamError::Other(
        _last_err.unwrap_or_else(|| "Failed after retries".to_string()),
    ))
}

fn done_reason_from_stop(stop: StopReason) -> DoneReason {
    match stop {
        StopReason::Length => DoneReason::Length,
        StopReason::ToolUse => DoneReason::ToolUse,
        _ => DoneReason::Stop,
    }
}

fn check_abort(options: &CodexResponsesOptions) -> Result<(), CodexStreamError> {
    if let Some(signal) = &options.base.signal {
        if *signal.borrow() {
            return Err(CodexStreamError::Aborted);
        }
    }
    Ok(())
}

async fn wait_for_abort(signal: &mut watch::Receiver<bool>) {
    loop {
        if *signal.borrow() {
            return;
        }
        if signal.changed().await.is_err() {
            std::future::pending::<()>().await;
        }
    }
}

async fn sleep_ms(ms: u64, signal: Option<watch::Receiver<bool>>) {
    match signal {
        Some(mut sig) => {
            tokio::select! {
                _ = tokio::time::sleep(std::time::Duration::from_millis(ms)) => {}
                _ = wait_for_abort(&mut sig) => {}
            }
        }
        None => {
            tokio::time::sleep(std::time::Duration::from_millis(ms)).await;
        }
    }
}

// ============================================================================
// Request building
// ============================================================================

fn build_request_body(
    model: &Model,
    context: &Context,
    options: &CodexResponsesOptions,
) -> Result<Value, CodexStreamError> {
    let messages = convert_responses_messages(
        model,
        context,
        &CODEX_TOOL_CALL_PROVIDERS,
        Some(&ConvertResponsesMessagesOptions {
            include_system_prompt: Some(false),
        }),
    );

    let input: Vec<Value> = messages
        .iter()
        .filter_map(|item| serde_json::to_value(item).ok())
        .collect();

    let tools = if !context.tools.is_empty() {
        let raw_tools = convert_responses_tools(context.tools.as_slice(), None);
        let tools_json: Vec<Value> = raw_tools
            .iter()
            .filter_map(|t| serde_json::to_value(t).ok())
            .collect();
        Some(tools_json)
    } else {
        None
    };

    let instructions = context
        .system_prompt
        .clone()
        .unwrap_or_else(|| "You are a helpful assistant.".to_string());

    let text = serde_json::json!({
        "verbosity": options.text_verbosity.as_deref().unwrap_or("low")
    });

    let include = vec!["reasoning.encrypted_content".to_string()];
    let prompt_cache_key = clamp_openai_prompt_cache_key(options.base.session_id.as_deref());

    let mut body = serde_json::json!({
        "model": model.id,
        "store": false,
        "stream": true,
        "instructions": instructions,
        "input": input,
        "text": text,
        "include": include,
        "tool_choice": "auto",
        "parallel_tool_calls": true,
    });

    if let Some(tools) = tools {
        body["tools"] = Value::Array(tools);
    }

    if let Some(ref key) = prompt_cache_key {
        body["prompt_cache_key"] = Value::String(key.clone());
    }

    if let Some(temp) = options.base.temperature {
        body["temperature"] = serde_json::json!(temp);
    }

    if let Some(ref service_tier) = options.service_tier {
        body["service_tier"] = Value::String(service_tier.clone());
    }

    if let Some(ref reasoning_effort) = options.reasoning_effort {
        let effort = if reasoning_effort == "none" {
            model
                .thinking_level_map
                .as_ref()
                .and_then(|map| map.get(&ModelThinkingLevel::Off))
                .and_then(|v| v.clone())
                .unwrap_or_else(|| "none".to_string())
        } else if let Some(level_map) = &model.thinking_level_map {
            let level = string_to_model_level(reasoning_effort);
            level
                .and_then(|l| level_map.get(&l))
                .and_then(|v| v.clone())
                .unwrap_or_else(|| reasoning_effort.clone())
        } else {
            reasoning_effort.clone()
        };

        body["reasoning"] = serde_json::json!({
            "effort": effort,
            "summary": options.reasoning_summary.as_deref().unwrap_or("auto"),
        });
    }

    Ok(body)
}

fn string_to_model_level(s: &str) -> Option<ModelThinkingLevel> {
    match s {
        "off" => Some(ModelThinkingLevel::Off),
        "minimal" => Some(ModelThinkingLevel::Minimal),
        "low" => Some(ModelThinkingLevel::Low),
        "medium" => Some(ModelThinkingLevel::Medium),
        "high" => Some(ModelThinkingLevel::High),
        "xhigh" => Some(ModelThinkingLevel::XHigh),
        _ => None,
    }
}

fn get_service_tier_multiplier(model_id: &str, service_tier: Option<&str>) -> f64 {
    match service_tier {
        Some("flex") => 0.5,
        Some("priority") => {
            if model_id == "gpt-5.5" {
                2.5
            } else {
                2.0
            }
        }
        _ => 1.0,
    }
}

fn apply_service_tier_pricing(usage: &mut Usage, service_tier: Option<&str>, model_id: &str) {
    let multiplier = get_service_tier_multiplier(model_id, service_tier);
    if (multiplier - 1.0).abs() < f64::EPSILON {
        return;
    }
    usage.cost.input *= multiplier;
    usage.cost.output *= multiplier;
    usage.cost.cache_read *= multiplier;
    usage.cost.cache_write *= multiplier;
    usage.cost.total =
        usage.cost.input + usage.cost.output + usage.cost.cache_read + usage.cost.cache_write;
}

fn resolve_codex_service_tier(
    response_tier: Option<String>,
    request_tier: Option<String>,
) -> Option<String> {
    match &response_tier {
        Some(t) if t == "default" => match &request_tier {
            Some(rt) if rt == "flex" || rt == "priority" => request_tier,
            _ => response_tier,
        },
        Some(_) => response_tier,
        None => request_tier,
    }
}

fn resolve_codex_url(base_url: &str) -> String {
    let raw = if base_url.trim().is_empty() {
        DEFAULT_CODEX_BASE_URL.to_string()
    } else {
        base_url.trim_end_matches('/').to_string()
    };

    if raw.ends_with("/codex/responses") {
        raw
    } else if raw.ends_with("/codex") {
        format!("{}/responses", raw)
    } else {
        format!("{}/codex/responses", raw)
    }
}

#[allow(dead_code)]
fn resolve_codex_websocket_url(base_url: &str) -> String {
    let url_str = resolve_codex_url(base_url);
    if let Ok(mut url) = url::Url::parse(&url_str) {
        match url.scheme() {
            "https" => {
                let _ = url.set_scheme("wss");
            }
            "http" => {
                let _ = url.set_scheme("ws");
            }
            _ => {}
        }
        url.to_string()
    } else {
        "wss://chatgpt.com/backend-api/codex/responses".to_string()
    }
}

// ============================================================================
// SSE processing
// ============================================================================

async fn process_sse_response(
    response: reqwest::Response,
    output: &mut AssistantMessage,
    sender: &mut AssistantMessageEventStreamSender,
    model: &Model,
    options: &CodexResponsesOptions,
) -> Result<(), CodexStreamError> {
    let service_tier = options.service_tier.clone();
    let model_id = model.id.clone();

    let opts = OpenAIResponsesStreamOptions {
        service_tier: service_tier.clone(),
        resolve_service_tier: Some(Box::new(move |resp_tier, req_tier| {
            resolve_codex_service_tier(resp_tier, req_tier)
        })),
        apply_service_tier_pricing: Some(Box::new(move |usage, tier| {
            apply_service_tier_pricing(usage, tier.as_deref(), &model_id);
        })),
    };

    // Emit start
    sender.push(AssistantMessageEvent::Start {
        partial: output.clone(),
    });

    // Parse SSE and map events, then feed into the shared processor
    let raw_events = parse_sse_and_map(response, options.base.signal.clone())?;
    process_responses_stream(raw_events, output, sender, model, Some(&opts))
        .await
        .map_err(|e| CodexStreamError::Other(e.to_string()))
}

/// Parse SSE from an HTTP response body byte stream, mapping Codex raw events
/// to standard [`ResponseStreamEvent`]s. This function owns the response,
/// reads the body on a channel, and returns a stream.
fn parse_sse_and_map(
    response: reqwest::Response,
    signal: Option<watch::Receiver<bool>>,
) -> Result<
    impl futures::Stream<Item = Result<ResponseStreamEvent, ResponsesStreamError>>,
    CodexStreamError,
> {
    // We use an mpsc channel to bridge the async byte-reading loop with the
    // returned Stream (which the shared processor drives via poll_next).
    let (tx, rx) =
        tokio::sync::mpsc::unbounded_channel::<Result<ResponseStreamEvent, ResponsesStreamError>>();

    tokio::spawn(async move {
        let mut byte_stream = response.bytes_stream();
        let mut buffer: Vec<u8> = Vec::new();
        #[allow(unused_assignments)]
        let mut stream_done = false;
        let signal = signal;

        loop {
            if let Some(ref sig) = signal {
                if *sig.borrow() {
                    let _ = tx.send(Err(ResponsesStreamError::Transport(
                        "Request was aborted".to_string(),
                    )));
                    return;
                }
            }

            let next = if let Some(_sig) = signal.clone() {
                tokio::select! {
                    chunk = byte_stream.next() => chunk,
                    _ = tokio::time::sleep(std::time::Duration::from_secs(60)) => {
                        let _ = tx.send(Err(ResponsesStreamError::Transport("SSE stream idle timeout".to_string())));
                        return;
                    }
                }
            } else {
                byte_stream.next().await
            };

            let chunk = match next {
                Some(Ok(bytes)) => bytes,
                Some(Err(e)) => {
                    let _ = tx.send(Err(ResponsesStreamError::Transport(format!(
                        "SSE error: {}",
                        e
                    ))));
                    return;
                }
                None => {
                    stream_done = true;
                    break;
                }
            };

            buffer.extend_from_slice(&chunk);

            // Process complete events (delimited by \n\n)
            loop {
                let event_start = find_double_newline(&buffer);
                match event_start {
                    Some(end) => {
                        let event_bytes = buffer[..end].to_vec();
                        buffer.drain(..=end + 1); // +1 for the second \n
                        if let Some(codex_event) = parse_codex_raw_event(&event_bytes) {
                            let mapped = map_codex_raw_event(codex_event);
                            match mapped {
                                Ok(event) => {
                                    let is_complete = matches!(
                                        &event,
                                        ResponseStreamEvent::ResponseCompleted { .. }
                                    );
                                    if tx.send(Ok(event)).is_err() {
                                        return;
                                    }
                                    if is_complete {
                                        // After response.completed, no more events expected
                                        return;
                                    }
                                }
                                Err(e) => {
                                    if tx.send(Err(e)).is_err() {
                                        return;
                                    }
                                }
                            }
                        }
                    }
                    None => break,
                }
            }
        }

        // Flush remaining buffer
        if !buffer.is_empty() && !stream_done {
            if let Some(codex_event) = parse_codex_raw_event(&buffer) {
                if let Ok(event) = map_codex_raw_event(codex_event) {
                    let _ = tx.send(Ok(event));
                }
            }
        }
    });

    Ok(tokio_stream::wrappers::UnboundedReceiverStream::new(rx))
}

/// Find the position of the first \n\n boundary in a buffer.
/// Returns the ending index of the first \n (before the double newline).
fn find_double_newline(buf: &[u8]) -> Option<usize> {
    for i in 0..buf.len().saturating_sub(1) {
        if buf[i] == b'\n' && buf[i + 1] == b'\n' {
            return Some(i);
        }
    }
    None
}

/// A raw Codex event from the wire.
#[derive(Debug, Clone, serde::Deserialize)]
struct CodexRawEvent {
    #[serde(rename = "type")]
    event_type: Option<String>,
    #[serde(default)]
    code: Option<String>,
    #[serde(default)]
    message: Option<String>,
    #[serde(flatten)]
    extra: HashMap<String, Value>,
}

/// Parse `data:` lines from a raw SSE chunk into a CodexRawEvent.
fn parse_codex_raw_event(chunk: &[u8]) -> Option<CodexRawEvent> {
    let text = String::from_utf8_lossy(chunk);
    let data_lines: Vec<&str> = text
        .lines()
        .filter(|l| l.starts_with("data:"))
        .map(|l| l[5..].trim())
        .collect();

    if data_lines.is_empty() {
        return None;
    }

    let data = data_lines.join("\n").trim().to_string();
    if data.is_empty() || data == "[DONE]" {
        return None;
    }

    serde_json::from_str::<CodexRawEvent>(&data).ok()
}

/// Map a CodexRawEvent to a standard ResponseStreamEvent.
fn map_codex_raw_event(raw: CodexRawEvent) -> Result<ResponseStreamEvent, ResponsesStreamError> {
    let event_type = raw.event_type.as_deref().unwrap_or("");

    if event_type == "error" {
        let code = raw.code.unwrap_or_default();
        let message = raw.message.unwrap_or_default();
        return Err(ResponsesStreamError::Transport(format!(
            "Codex error: {}",
            if message.is_empty() { code } else { message }
        )));
    }

    if event_type == "response.failed" {
        let code = raw
            .extra
            .get("response")
            .and_then(|r| r.get("error"))
            .and_then(|e| e.get("code"))
            .and_then(|c| c.as_str())
            .unwrap_or("unknown");
        let message = raw
            .extra
            .get("response")
            .and_then(|r| r.get("error"))
            .and_then(|e| e.get("message"))
            .and_then(|m| m.as_str())
            .unwrap_or("Codex response failed");
        return Err(ResponsesStreamError::Transport(format!(
            "{code}: {message}"
        )));
    }

    // Normalize response.done / response.incomplete → response.completed
    let mapped_type = match event_type {
        "response.done" | "response.incomplete" => "response.completed",
        other => other,
    };

    let mut map = raw.extra.clone();
    map.insert("type".to_string(), Value::String(mapped_type.to_string()));
    if let Some(code) = &raw.code {
        map.insert("code".to_string(), Value::String(code.clone()));
    }
    if let Some(message) = &raw.message {
        map.insert("message".to_string(), Value::String(message.clone()));
    }

    // Normalize status for completed events
    if mapped_type == "response.completed" {
        if let Some(response) = map.get_mut("response") {
            if let Some(status) = response.get("status").and_then(|s| s.as_str()) {
                let normalized = if CODEX_RESPONSE_STATUSES.contains(&status) {
                    status.to_string()
                } else {
                    return Err(ResponsesStreamError::Transport(format!(
                        "Unknown status: {status}"
                    )));
                };
                if let Some(obj) = response.as_object_mut() {
                    obj.insert("status".to_string(), Value::String(normalized));
                }
            }
        }
    }

    let json_map: Map<String, Value> = map.into_iter().collect();
    serde_json::from_value::<ResponseStreamEvent>(Value::Object(json_map))
        .map_err(|e| ResponsesStreamError::Transport(format!("Failed to parse event: {e}")))
}

// ============================================================================
// WebSocket
// ============================================================================
// TODO(websocket): Full WebSocket transport requires tokio-tungstenite.
// The SSE-only path is fully implemented. Marking the WebSocket fallback as a TODO.
// When adding WebSocket, implement:
// 1. Connection pool (cached connections per session)
// 2. Cached continuation using previous_response_id
// 3. WS-parsed event mapping into the same map_codex_raw_event logic

async fn try_websocket_impl(
    _model: &Model,
    _body: &Value,
    _account_id: &str,
    _api_key: &str,
    _request_id: &str,
    _output: &mut AssistantMessage,
    _sender: &mut AssistantMessageEventStreamSender,
    _ws_started: &mut bool,
    _options: &CodexResponsesOptions,
) -> Result<(), CodexStreamError> {
    Err(CodexStreamError::Transport(
        "WebSocket transport not yet implemented".to_string(),
    ))
}

// ============================================================================
// WebSocket session fallback state
// ============================================================================

static WEBSOCKET_SSE_FALLBACK_SESSIONS: LazyLock<Mutex<HashSet<String>>> =
    LazyLock::new(|| Mutex::new(HashSet::new()));

fn is_websocket_sse_fallback_active(session_id: Option<&str>) -> bool {
    match (session_id, WEBSOCKET_SSE_FALLBACK_SESSIONS.lock()) {
        (Some(id), Ok(s)) => s.contains(id),
        _ => false,
    }
}

fn record_websocket_sse_fallback(session_id: Option<&str>) {
    if let Some(id) = session_id {
        if let Ok(mut s) = WEBSOCKET_SSE_FALLBACK_SESSIONS.lock() {
            s.insert(id.to_string());
        }
    }
}

fn record_websocket_failure(session_id: Option<&str>) {
    if let Some(id) = session_id {
        if let Ok(mut s) = WEBSOCKET_SSE_FALLBACK_SESSIONS.lock() {
            s.insert(id.to_string());
        }
    }
}

// ============================================================================
// Auth & headers
// ============================================================================

fn extract_account_id(token: &str) -> Result<String, CodexStreamError> {
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 3 {
        return Err(CodexStreamError::Other(
            "Failed to extract accountId from token".to_string(),
        ));
    }
    let payload_bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(parts[1])
        .map_err(|_| {
            CodexStreamError::Other("Failed to extract accountId from token".to_string())
        })?;
    let payload: Value = serde_json::from_slice(&payload_bytes).map_err(|_| {
        CodexStreamError::Other("Failed to extract accountId from token".to_string())
    })?;
    payload
        .get(JWT_CLAIM_PATH)
        .and_then(|v| v.get("chatgpt_account_id"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| {
            CodexStreamError::Other("Failed to extract accountId from token".to_string())
        })
}

fn create_codex_request_id() -> String {
    let mut bytes = [0u8; 4];
    let _ = getrandom::getrandom(&mut bytes);
    format!(
        "codex_{}_{}",
        Utc::now().timestamp_millis(),
        bytes.iter().map(|b| format!("{b:02x}")).collect::<String>()
    )
}

fn build_base_codex_headers(
    init_headers: Option<&HashMap<String, String>>,
    additional_headers: Option<&HashMap<String, String>>,
    account_id: &str,
    token: &str,
) -> HashMap<String, String> {
    let mut headers = HashMap::new();
    if let Some(h) = init_headers {
        headers.extend(h.iter().map(|(k, v)| (k.clone(), v.clone())));
    }
    if let Some(h) = additional_headers {
        headers.extend(h.iter().map(|(k, v)| (k.clone(), v.clone())));
    }
    headers.insert("Authorization".to_string(), format!("Bearer {token}"));
    headers.insert("chatgpt-account-id".to_string(), account_id.to_string());
    headers.insert("originator".to_string(), "hamr".to_string());
    headers.insert(
        "User-Agent".to_string(),
        format!("hamr ({} {})", std::env::consts::OS, std::env::consts::ARCH),
    );
    headers
}

fn build_sse_headers(
    init_headers: Option<&HashMap<String, String>>,
    additional_headers: Option<&HashMap<String, String>>,
    account_id: &str,
    token: &str,
    session_id: Option<&str>,
) -> HashMap<String, String> {
    let mut headers = build_base_codex_headers(init_headers, additional_headers, account_id, token);
    headers.insert(
        "OpenAI-Beta".to_string(),
        "responses=experimental".to_string(),
    );
    headers.insert("accept".to_string(), "text/event-stream".to_string());
    headers.insert("content-type".to_string(), "application/json".to_string());
    if let Some(sid) = session_id {
        headers.insert("session-id".to_string(), sid.to_string());
        headers.insert("x-client-request-id".to_string(), sid.to_string());
    }
    headers
}

fn _build_websocket_headers(
    init_headers: Option<&HashMap<String, String>>,
    additional_headers: Option<&HashMap<String, String>>,
    account_id: &str,
    token: &str,
    request_id: &str,
) -> HashMap<String, String> {
    let mut headers = build_base_codex_headers(init_headers, additional_headers, account_id, token);
    headers.remove("accept");
    headers.remove("content-type");
    headers.remove("OpenAI-Beta");
    headers.remove("openai-beta");
    headers.insert(
        "OpenAI-Beta".to_string(),
        "responses_websockets=2026-02-06".to_string(),
    );
    headers.insert("x-client-request-id".to_string(), request_id.to_string());
    headers.insert("session-id".to_string(), request_id.to_string());
    headers
}

// ============================================================================
// Error parsing
// ============================================================================

struct ParsedError {
    message: String,
    friendly_message: Option<String>,
}

fn parse_error_response(raw: &str, status: u16) -> ParsedError {
    let message = if raw.is_empty() {
        format!("Request failed ({status})")
    } else {
        raw.to_string()
    };
    let mut friendly_message: Option<String> = None;

    if let Ok(parsed) = serde_json::from_str::<Value>(raw) {
        if let Some(err) = parsed.get("error") {
            let code = err
                .get("code")
                .and_then(|c| c.as_str())
                .or_else(|| err.get("type").and_then(|t| t.as_str()))
                .unwrap_or("")
                .to_string();
            let err_message = err
                .get("message")
                .and_then(|m| m.as_str())
                .unwrap_or("")
                .to_string();

            if status == 429
                || Regex::new(r"usage_limit_reached|usage_not_included|rate_limit_exceeded")
                    .map(|re| re.is_match(&code))
                    .unwrap_or(false)
            {
                let plan = err
                    .get("plan_type")
                    .and_then(|p| p.as_str())
                    .map(|p| format!(" ({})", p.to_lowercase()))
                    .unwrap_or_default();
                let mins = err.get("resets_at").and_then(|r| r.as_f64()).map(|ts| {
                    let now = Utc::now().timestamp_millis() as f64;
                    let diff_ms = ts * 1000.0 - now;
                    (diff_ms / 60000.0).max(0.0).round() as u64
                });
                let when = mins
                    .map(|m| format!(" Try again in ~{m} min."))
                    .unwrap_or_default();
                friendly_message = Some(
                    format!("You have hit your ChatGPT usage limit{plan}.{when}")
                        .trim()
                        .to_string(),
                );
            }

            let msg = if err_message.is_empty() {
                friendly_message.clone().unwrap_or_else(|| message.clone())
            } else {
                err_message
            };
            return ParsedError {
                message: msg,
                friendly_message,
            };
        }
    }

    ParsedError {
        message,
        friendly_message,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_account_id_valid() {
        // Create a minimal JWT-like token with the chatgpt_account_id claim
        let header = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(b"{\"alg\":\"none\"}");
        let payload = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .encode(b"{\"https://api.openai.com/auth\": {\"chatgpt_account_id\": \"abc123\"}}");
        let token = format!("{header}.{payload}.sig");
        let result = extract_account_id(&token);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "abc123");
    }

    #[test]
    fn test_extract_account_id_invalid() {
        let result = extract_account_id("invalid");
        assert!(result.is_err());
    }

    #[test]
    fn test_resolve_codex_url_default() {
        let url = resolve_codex_url("");
        assert_eq!(url, "https://chatgpt.com/backend-api/codex/responses");
    }

    #[test]
    fn test_resolve_codex_url_custom_base() {
        let url = resolve_codex_url("https://custom.example.com");
        assert_eq!(url, "https://custom.example.com/codex/responses");
    }

    #[test]
    fn test_resolve_codex_url_already_ends() {
        let url = resolve_codex_url("https://example.com/codex/responses");
        assert_eq!(url, "https://example.com/codex/responses");
    }

    #[test]
    fn test_resolve_codex_websocket_url() {
        let url = resolve_codex_websocket_url("");
        assert!(url.starts_with("wss://"));
    }

    #[test]
    fn test_is_terminal_rate_limit_error() {
        assert!(is_terminal_rate_limit_error("Monthly usage limit reached"));
        assert!(is_terminal_rate_limit_error("insufficient_quota"));
        assert!(!is_terminal_rate_limit_error("rate limit exceeded"));
    }

    #[test]
    fn test_is_retryable_error_429() {
        assert!(is_retryable_error(429, "rate limit exceeded"));
        assert!(!is_retryable_error(429, "Monthly usage limit reached"));
    }

    #[test]
    fn test_is_retryable_error_server() {
        assert!(is_retryable_error(500, ""));
        assert!(is_retryable_error(502, ""));
        assert!(is_retryable_error(503, ""));
        assert!(is_retryable_error(504, ""));
        assert!(!is_retryable_error(400, ""));
    }

    #[test]
    fn test_parse_error_response_rate_limit() {
        let result = parse_error_response(
            r#"{"error":{"code":"rate_limit_exceeded","message":"Too fast","resets_at":9999999999.0}}"#,
            429,
        );
        assert!(result.friendly_message.is_some());
        assert!(!result.message.is_empty());
    }

    #[test]
    fn test_parse_error_response_regular() {
        let result = parse_error_response(r#"{"error":{"message":"Bad request"}}"#, 400);
        assert_eq!(result.message, "Bad request");
        assert!(result.friendly_message.is_none());
    }

    #[test]
    fn test_create_codex_request_id_format() {
        let id = create_codex_request_id();
        assert!(id.starts_with("codex_"));
        assert!(id.len() > 10);
    }

    #[test]
    fn test_get_service_tier_multiplier() {
        assert!((get_service_tier_multiplier("gpt-4o", None) - 1.0).abs() < f64::EPSILON);
        assert!((get_service_tier_multiplier("gpt-4o", Some("flex")) - 0.5).abs() < f64::EPSILON);
        assert!(
            (get_service_tier_multiplier("gpt-4o", Some("priority")) - 2.0).abs() < f64::EPSILON
        );
        assert!(
            (get_service_tier_multiplier("gpt-5.5", Some("priority")) - 2.5).abs() < f64::EPSILON
        );
    }

    #[test]
    fn test_level_to_effort() {
        assert_eq!(level_to_effort_string(ModelThinkingLevel::Off), "none");
        assert_eq!(level_to_effort_string(ModelThinkingLevel::High), "high");
    }
}
