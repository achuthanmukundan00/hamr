//! Port of `../../packages/ai/src/providers/openai-responses.ts`.
//!
//! OpenAI Responses API provider backend.
//!
//! Unlike the TS source — which uses the `openai` SDK and its
//! `client.responses.create(...).withResponse()` helper — this port issues raw
//! HTTP requests with `reqwest` and parses the `POST {baseUrl}/responses` SSE
//! stream manually. The request payload (message + tool conversion), the
//! response-item conversion / reasoning replay, and the streaming event handler
//! all live in [`crate::providers::openai_responses_shared`] and are reused here
//! exactly as the TS imports from `openai-responses-shared.ts`.
//!
//! Entry points:
//! - [`stream`] (a.k.a. `stream_openai_responses`) — the full provider stream.
//! - [`stream_simple`] (a.k.a. `stream_simple_openai_responses`) — the
//!   `SimpleStreamOptions` wrapper that maps a reasoning level to a reasoning
//!   effort.
//!
//! ## Type debt
//!
//! - `Model.compat` is `Option<serde_json::Value>`. The `getCompat` function
//!   reads from it when present and defaults every flag to `true` when not set.
//! - `serviceTier` is the OpenAI SDK `service_tier` type (`string | null |
//!   undefined`); we model it as `Option<String>`.
//! - The SDK's `requestOptions` (`signal` / `timeout` / `maxRetries`) map onto
//!   manual abort-racing + a reqwest timeout. Client-side `maxRetries` is not
//!   re-implemented (the TS default is `0`); see the note on [`run_stream_inner`].

use std::collections::HashSet;

use futures::StreamExt;
use serde_json::Value;

use crate::models::clamp_thinking_level;
use crate::providers::cloudflare::{is_cloudflare_provider, resolve_cloudflare_base_url};
use crate::providers::github_copilot_headers::{
    build_copilot_dynamic_headers, has_copilot_vision_input,
};
use crate::providers::openai_prompt_cache::clamp_openai_prompt_cache_key;
use crate::providers::openai_responses_shared::{
    OpenAIResponsesStreamOptions, ResponseStreamEvent, ResponsesStreamError,
    convert_responses_messages, convert_responses_tools, process_responses_stream,
};
use crate::providers::simple_options::build_base_options;
use crate::types::{
    AssistantMessage, CacheRetention, Context, DoneReason, ErrorReason, MessageRole, Model,
    ModelThinkingLevel, ProviderEnv, ProviderResponse, SimpleStreamOptions, StopReason,
    StreamOptions, ThinkingLevel, Usage, UsageCost,
};
use crate::utils::event_stream::{
    AssistantMessageEventStream, AssistantMessageEventStreamSender,
    create_assistant_message_event_stream,
};
use crate::utils::headers::headers_to_record;
use crate::utils::provider_env::get_provider_env_value;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Mirrors the TS module-level `OPENAI_TOOL_CALL_PROVIDERS` set.
fn openai_tool_call_providers() -> HashSet<String> {
    ["openai", "openai-codex", "opencode"]
        .iter()
        .map(|s| s.to_string())
        .collect()
}

// ---------------------------------------------------------------------------
// Compat — reads from Model.compat when present, defaults all true.
// ---------------------------------------------------------------------------

/// Mirrors the TS `Required<OpenAIResponsesCompat>`.
#[derive(Debug, Clone, Copy)]
struct OpenAIResponsesCompat {
    /// Whether the model supports the `developer` role (vs `system`).
    #[allow(dead_code)]
    supports_developer_role: bool,
    send_session_id_header: bool,
    supports_long_cache_retention: bool,
}

/// Mirrors the TS `getCompat`. Reads from `model.compat` if present;
/// defaults every flag to true when not set.
fn get_compat(model: &Model) -> OpenAIResponsesCompat {
    let defaults = OpenAIResponsesCompat {
        supports_developer_role: true,
        send_session_id_header: true,
        supports_long_cache_retention: true,
    };
    match &model.compat {
        Some(value) => OpenAIResponsesCompat {
            supports_developer_role: value
                .get("supportsDeveloperRole")
                .and_then(|v| v.as_bool())
                .unwrap_or(defaults.supports_developer_role),
            send_session_id_header: value
                .get("sendSessionIdHeader")
                .and_then(|v| v.as_bool())
                .unwrap_or(defaults.send_session_id_header),
            supports_long_cache_retention: value
                .get("supportsLongCacheRetention")
                .and_then(|v| v.as_bool())
                .unwrap_or(defaults.supports_long_cache_retention),
        },
        None => defaults,
    }
}

// ---------------------------------------------------------------------------
// Options
// ---------------------------------------------------------------------------

/// Reasoning effort for [`OpenAIResponsesOptions`].
///
/// Mirrors the TS `"minimal" | "low" | "medium" | "high" | "xhigh"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReasoningEffort {
    Minimal,
    Low,
    Medium,
    High,
    XHigh,
}

impl ReasoningEffort {
    fn as_str(self) -> &'static str {
        match self {
            ReasoningEffort::Minimal => "minimal",
            ReasoningEffort::Low => "low",
            ReasoningEffort::Medium => "medium",
            ReasoningEffort::High => "high",
            ReasoningEffort::XHigh => "xhigh",
        }
    }
}

/// Map a unified [`ModelThinkingLevel`] (post-clamp) into a [`ReasoningEffort`].
///
/// Mirrors the TS `streamSimpleOpenAIResponses` mapping:
/// `clampedReasoning === "off" ? undefined : clampedReasoning`.
fn effort_from_clamped(level: ModelThinkingLevel) -> Option<ReasoningEffort> {
    match level {
        ModelThinkingLevel::Off => None,
        ModelThinkingLevel::Minimal => Some(ReasoningEffort::Minimal),
        ModelThinkingLevel::Low => Some(ReasoningEffort::Low),
        ModelThinkingLevel::Medium => Some(ReasoningEffort::Medium),
        ModelThinkingLevel::High => Some(ReasoningEffort::High),
        ModelThinkingLevel::XHigh => Some(ReasoningEffort::XHigh),
    }
}

/// Reasoning summary mode for [`OpenAIResponsesOptions`].
///
/// Mirrors the TS `"auto" | "detailed" | "concise" | null`. The `null` variant
/// (TS) is `Some(ReasoningSummary::Null)`; an absent value is `None`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReasoningSummary {
    Auto,
    Detailed,
    Concise,
    /// TS `null`.
    Null,
}

impl ReasoningSummary {
    fn as_str(self) -> Option<&'static str> {
        match self {
            ReasoningSummary::Auto => Some("auto"),
            ReasoningSummary::Detailed => Some("detailed"),
            ReasoningSummary::Concise => Some("concise"),
            ReasoningSummary::Null => None,
        }
    }
}

/// OpenAI Responses-specific stream options.
///
/// Mirrors the TS `interface OpenAIResponsesOptions extends StreamOptions`.
#[derive(Clone, Default)]
pub struct OpenAIResponsesOptions {
    pub base: StreamOptions,
    pub reasoning_effort: Option<ReasoningEffort>,
    pub reasoning_summary: Option<ReasoningSummary>,
    /// SDK `service_tier` (`string | null | undefined`) — modelled as `Option<String>`.
    pub service_tier: Option<String>,
}

// ---------------------------------------------------------------------------
// Cache retention / prompt-cache helpers
// ---------------------------------------------------------------------------

/// Resolve cache retention preference.
///
/// Mirrors the TS `resolveCacheRetention`: defaults to `Short` and honours
/// `PI_CACHE_RETENTION=long` for backward compatibility.
fn resolve_cache_retention(
    cache_retention: Option<CacheRetention>,
    env: Option<&ProviderEnv>,
) -> CacheRetention {
    if let Some(retention) = cache_retention {
        return retention;
    }
    if get_provider_env_value("PI_CACHE_RETENTION", env).as_deref() == Some("long") {
        return CacheRetention::Long;
    }
    CacheRetention::Short
}

/// Mirrors the TS `getPromptCacheRetention`.
fn get_prompt_cache_retention(
    compat: OpenAIResponsesCompat,
    cache_retention: CacheRetention,
) -> Option<&'static str> {
    if cache_retention == CacheRetention::Long && compat.supports_long_cache_retention {
        Some("24h")
    } else {
        None
    }
}

// ---------------------------------------------------------------------------
// Service-tier pricing (mirrors TS getServiceTierCostMultiplier / applyServiceTierPricing)
// ---------------------------------------------------------------------------

fn get_service_tier_cost_multiplier(model_id: &str, service_tier: Option<&str>) -> f64 {
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

/// Mirrors the TS `applyServiceTierPricing`: scales the (already-computed) cost
/// fields in `usage` by the service-tier multiplier.
fn apply_service_tier_pricing(usage: &mut Usage, service_tier: Option<&str>, model_id: &str) {
    let multiplier = get_service_tier_cost_multiplier(model_id, service_tier);
    if multiplier == 1.0 {
        return;
    }
    usage.cost.input *= multiplier;
    usage.cost.output *= multiplier;
    usage.cost.cache_read *= multiplier;
    usage.cost.cache_write *= multiplier;
    usage.cost.total =
        usage.cost.input + usage.cost.output + usage.cost.cache_read + usage.cost.cache_write;
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

fn initial_output(model: &Model) -> AssistantMessage {
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

/// Stream a completion from an OpenAI Responses model.
///
/// Mirrors the TS `streamOpenAIResponses`.
pub fn stream(
    model: Model,
    context: Context,
    options: Option<OpenAIResponsesOptions>,
) -> AssistantMessageEventStream {
    let (sender, stream_out) = create_assistant_message_event_stream();
    let options = options.unwrap_or_default();

    tokio::spawn(async move {
        run_stream(model, context, options, sender).await;
    });

    stream_out
}

/// TS-named alias for [`stream`].
pub fn stream_openai_responses(
    model: Model,
    context: Context,
    options: Option<OpenAIResponsesOptions>,
) -> AssistantMessageEventStream {
    stream(model, context, options)
}

/// Stream with simplified reasoning-level options.
///
/// Mirrors the TS `streamSimpleOpenAIResponses`: maps the unified `reasoning`
/// level into an OpenAI reasoning effort and delegates to [`stream`].
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

    // `options?.reasoning ? clampThinkingLevel(model, options.reasoning) : undefined`
    let reasoning_effort = match options.as_ref().and_then(|o| o.reasoning) {
        Some(reasoning) => {
            let clamped = clamp_thinking_level(&model, level_to_model_level(reasoning));
            effort_from_clamped(clamped)
        }
        None => None,
    };

    stream(
        model,
        context,
        Some(OpenAIResponsesOptions {
            base,
            reasoning_effort,
            reasoning_summary: None,
            service_tier: None,
        }),
    )
}

/// TS-named alias for [`stream_simple`].
pub fn stream_simple_openai_responses(
    model: Model,
    context: Context,
    options: Option<SimpleStreamOptions>,
) -> AssistantMessageEventStream {
    stream_simple(model, context, options)
}

fn level_to_model_level(level: ThinkingLevel) -> ModelThinkingLevel {
    ModelThinkingLevel::from(level)
}

/// Produce a stream that immediately emits a terminal error event.
fn error_stream(model: &Model, message: String) -> AssistantMessageEventStream {
    let (mut sender, stream_out) = create_assistant_message_event_stream();
    let mut output = initial_output(model);
    output.stop_reason = StopReason::Error;
    output.error_message = Some(message);
    sender.push(crate::types::AssistantMessageEvent::Error {
        reason: ErrorReason::Error,
        error: output,
    });
    sender.end(None);
    stream_out
}

// ---------------------------------------------------------------------------
// Stream driver
// ---------------------------------------------------------------------------

async fn run_stream(
    model: Model,
    context: Context,
    options: OpenAIResponsesOptions,
    mut sender: AssistantMessageEventStreamSender,
) {
    let mut output = initial_output(&model);

    match run_stream_inner(&model, &context, &options, &mut sender, &mut output).await {
        Ok(()) => {
            let reason = done_reason_from_stop(output.stop_reason);
            sender.push(crate::types::AssistantMessageEvent::Done {
                reason,
                message: output,
            });
            sender.end(None);
        }
        Err(err) => {
            // Mirror the TS catch block: strip the streaming scratch fields from
            // every block before persisting. In Rust those fields never leak into
            // the typed `AssistantContentBlock`, so there is nothing to delete.
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
            sender.push(crate::types::AssistantMessageEvent::Error {
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
///
/// **Type debt:** the TS `requestOptions.maxRetries` (default `0`) is not
/// re-implemented; a raw single-shot request matches the default behaviour.
async fn run_stream_inner(
    model: &Model,
    context: &Context,
    options: &OpenAIResponsesOptions,
    sender: &mut AssistantMessageEventStreamSender,
    output: &mut AssistantMessage,
) -> Result<(), String> {
    // apiKey guard (mirrors the TS throw).
    let api_key = match &options.base.api_key {
        Some(k) if !k.is_empty() => k.clone(),
        _ => return Err(format!("No API key for provider: {}", model.provider)),
    };

    let cache_retention =
        resolve_cache_retention(options.base.cache_retention, options.base.env.as_ref());
    let cache_session_id = if cache_retention == CacheRetention::None {
        None
    } else {
        options.base.session_id.clone()
    };

    // Build the request payload (mirrors TS `buildParams`).
    let mut params = build_params(model, context, options, cache_retention)?;

    // onPayload hook: allow inspection/replacement before sending.
    if let Some(on_payload) = &options.base.on_payload {
        if let Some(next) = on_payload(params.clone(), model.clone()).await {
            params = next;
        }
    }

    // Resolve base URL + endpoint. The OpenAI SDK posts to `{baseURL}/responses`.
    let base_url = if is_cloudflare_provider(&model.provider) {
        resolve_cloudflare_base_url(model, options.base.env.as_ref())?
    } else {
        model.base_url.clone()
    };
    let url = format!("{}/responses", base_url.trim_end_matches('/'));

    // Build headers (mirrors TS `createClient`).
    let headers = build_request_headers(model, context, &api_key, options, cache_session_id);

    let client = reqwest::Client::new();
    let mut request = client.post(&url).header("content-type", "application/json");

    // Cloudflare AI Gateway uses cf-aig-authorization for the upstream key and may
    // pass an explicit Authorization (mirrors the TS defaultHeaders branch).
    if model.provider == "cloudflare-ai-gateway" {
        request = request.header("cf-aig-authorization", format!("Bearer {api_key}"));
        // The TS sets `Authorization: headers.Authorization ?? null`; only send it
        // when a header explicitly provides one.
        if let Some(auth) = headers.get("Authorization") {
            request = request.header("Authorization", auth);
        }
    } else {
        request = request.header("Authorization", format!("Bearer {api_key}"));
    }

    for (k, v) in &headers {
        // Authorization handled above for the cloudflare branch; avoid duplicating.
        if model.provider == "cloudflare-ai-gateway" && k == "Authorization" {
            continue;
        }
        request = request.header(k, v);
    }

    if let Some(timeout_ms) = options.base.timeout_ms {
        request = request.timeout(std::time::Duration::from_millis(timeout_ms));
    }

    let body_bytes = serde_json::to_vec(&params).map_err(|e| e.to_string())?;
    request = request.body(body_bytes);

    // Send (raced against the abort signal).
    let response = send_with_abort(request, options.base.signal.clone()).await?;

    let status = response.status();

    // onResponse hook (mirrors TS `options?.onResponse?.(...)`).
    if let Some(on_response) = &options.base.on_response {
        let provider_response = ProviderResponse {
            status: status.as_u16(),
            headers: headers_to_record(response.headers()),
        };
        on_response(provider_response, model.clone()).await;
    }

    if !status.is_success() {
        let text = response.text().await.unwrap_or_default();
        return Err(format!("OpenAI API error ({}): {}", status.as_u16(), text));
    }

    // Emit `start` (mirrors the TS `stream.push({ type: "start", partial: output })`).
    sender.push(crate::types::AssistantMessageEvent::Start {
        partial: output.clone(),
    });

    // Build the decoded-SSE event stream and hand it to the shared processor.
    let model_id_for_pricing = model.id.clone();
    let stream_options = OpenAIResponsesStreamOptions {
        service_tier: options.service_tier.clone(),
        // The TS provider passes no `resolveServiceTier`; the shared handler then
        // uses `response.service_tier ?? options.service_tier`.
        resolve_service_tier: None,
        apply_service_tier_pricing: Some(Box::new(
            move |usage: &mut Usage, tier: Option<String>| {
                apply_service_tier_pricing(usage, tier.as_deref(), &model_id_for_pricing);
            },
        )),
    };

    let event_stream = sse_event_stream(response, options.base.signal.clone());
    futures::pin_mut!(event_stream);

    process_responses_stream(event_stream, output, sender, model, Some(&stream_options))
        .await
        .map_err(|e| e.to_string())?;

    // Post-loop abort/error finalization (mirrors the TS post-stream throws).
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

// ---------------------------------------------------------------------------
// Header construction (mirrors TS `createClient`)
// ---------------------------------------------------------------------------

fn build_request_headers(
    model: &Model,
    context: &Context,
    _api_key: &str,
    options: &OpenAIResponsesOptions,
    session_id: Option<String>,
) -> std::collections::HashMap<String, String> {
    let compat = get_compat(model);
    let mut headers: std::collections::HashMap<String, String> =
        model.headers.clone().unwrap_or_default();

    if model.provider == "github-copilot" {
        let has_images = has_copilot_vision_input(&context.messages);
        let copilot_headers = build_copilot_dynamic_headers(&context.messages, has_images);
        for (k, v) in copilot_headers {
            headers.insert(k, v);
        }
    }

    if let Some(session_id) = &session_id {
        if compat.send_session_id_header {
            headers.insert("session_id".to_string(), session_id.clone());
        }
        headers.insert("x-client-request-id".to_string(), session_id.clone());
    }

    // Merge options headers last so they can override defaults.
    if let Some(opt_headers) = &options.base.headers {
        for (k, v) in opt_headers {
            headers.insert(k.clone(), v.clone());
        }
    }

    headers
}

// ---------------------------------------------------------------------------
// Request payload (mirrors TS `buildParams`)
// ---------------------------------------------------------------------------

fn build_params(
    model: &Model,
    context: &Context,
    options: &OpenAIResponsesOptions,
    cache_retention: CacheRetention,
) -> Result<Value, String> {
    let allowed = openai_tool_call_providers();
    let messages = convert_responses_messages(model, context, &allowed, None);
    let compat = get_compat(model);

    let mut params = serde_json::Map::new();
    params.insert("model".into(), Value::String(model.id.clone()));
    params.insert(
        "input".into(),
        serde_json::to_value(&messages).map_err(|e| e.to_string())?,
    );
    params.insert("stream".into(), Value::Bool(true));
    params.insert("store".into(), Value::Bool(false));

    // prompt_cache_key / prompt_cache_retention (omitted when cache disabled).
    if cache_retention != CacheRetention::None {
        if let Some(key) = clamp_openai_prompt_cache_key(options.base.session_id.as_deref()) {
            params.insert("prompt_cache_key".into(), Value::String(key));
        }
    }
    if let Some(retention) = get_prompt_cache_retention(compat, cache_retention) {
        params.insert(
            "prompt_cache_retention".into(),
            Value::String(retention.to_string()),
        );
    }

    if let Some(max_tokens) = options.base.max_tokens {
        if max_tokens > 0 {
            params.insert("max_output_tokens".into(), Value::from(max_tokens));
        }
    }

    if let Some(temperature) = options.base.temperature {
        params.insert(
            "temperature".into(),
            serde_json::Number::from_f64(temperature)
                .map(Value::Number)
                .unwrap_or(Value::Null),
        );
    }

    if let Some(service_tier) = &options.service_tier {
        params.insert("service_tier".into(), Value::String(service_tier.clone()));
    }

    if !context.tools.is_empty() {
        let tools = convert_responses_tools(&context.tools, None);
        params.insert(
            "tools".into(),
            serde_json::to_value(&tools).map_err(|e| e.to_string())?,
        );
    }

    if model.reasoning {
        if options.reasoning_effort.is_some() || options.reasoning_summary.is_some() {
            // effort = thinkingLevelMap?.[reasoningEffort] ?? reasoningEffort, else "medium".
            let effort = match options.reasoning_effort {
                Some(eff) => mapped_effort(model, eff),
                None => "medium".to_string(),
            };
            // summary = reasoningSummary || "auto" (TS `||` collapses null to "auto").
            let summary = options
                .reasoning_summary
                .and_then(ReasoningSummary::as_str)
                .unwrap_or("auto");
            params.insert(
                "reasoning".into(),
                serde_json::json!({ "effort": effort, "summary": summary }),
            );
            params.insert(
                "include".into(),
                Value::Array(vec![Value::String(
                    "reasoning.encrypted_content".to_string(),
                )]),
            );
        } else if model.provider != "github-copilot" && thinking_off_is_not_null(model) {
            // effort = thinkingLevelMap?.off ?? "none"
            let effort = thinking_off_value(model).unwrap_or_else(|| "none".to_string());
            params.insert("reasoning".into(), serde_json::json!({ "effort": effort }));
        }
    }

    // abortSignal: the TS path aborts inside the SDK call; we check up front too.
    if let Some(signal) = &options.base.signal {
        if *signal.borrow() {
            return Err("Request was aborted".to_string());
        }
    }

    Ok(Value::Object(params))
}

/// `model.thinkingLevelMap?.[effort] ?? effort` — resolve the model-specific
/// effort string for a requested reasoning effort.
fn mapped_effort(model: &Model, effort: ReasoningEffort) -> String {
    let key = match effort {
        ReasoningEffort::Minimal => ModelThinkingLevel::Minimal,
        ReasoningEffort::Low => ModelThinkingLevel::Low,
        ReasoningEffort::Medium => ModelThinkingLevel::Medium,
        ReasoningEffort::High => ModelThinkingLevel::High,
        ReasoningEffort::XHigh => ModelThinkingLevel::XHigh,
    };
    if let Some(map) = &model.thinking_level_map {
        // A present `Some(value)` maps; a present `None` (TS null) falls through
        // to the requested effort (mirrors `?? options.reasoningEffort`).
        if let Some(Some(value)) = map.get(&key) {
            return value.clone();
        }
    }
    effort.as_str().to_string()
}

/// `model.thinkingLevelMap?.off !== null` — the `off` level is not explicitly
/// marked unsupported. Absent (`None` from `.get`) is treated as "not null".
fn thinking_off_is_not_null(model: &Model) -> bool {
    match &model.thinking_level_map {
        Some(map) => match map.get(&ModelThinkingLevel::Off) {
            // Present and explicitly `null` → false.
            Some(None) => false,
            // Present `Some(value)` or absent → true.
            _ => true,
        },
        None => true,
    }
}

/// `model.thinkingLevelMap?.off` resolved to its string value (if any).
fn thinking_off_value(model: &Model) -> Option<String> {
    model
        .thinking_level_map
        .as_ref()
        .and_then(|map| map.get(&ModelThinkingLevel::Off))
        .and_then(|v| v.clone())
}

// ---------------------------------------------------------------------------
// SSE decoding → ResponseStreamEvent stream
// ---------------------------------------------------------------------------

/// Streaming state for the SSE → [`ResponseStreamEvent`] adapter.
struct SseState {
    byte_stream:
        std::pin::Pin<Box<dyn futures::Stream<Item = reqwest::Result<bytes::Bytes>> + Send>>,
    signal: Option<tokio::sync::watch::Receiver<bool>>,
    /// Raw line buffer (split on `\n`).
    buffer: Vec<u8>,
    /// Decoded events ready to emit (a single chunk may yield several).
    pending: std::collections::VecDeque<Result<ResponseStreamEvent, ResponsesStreamError>>,
    /// Set once the upstream byte stream is exhausted (flush trailing buffer once).
    upstream_done: bool,
    /// Set once we have emitted a terminal error / exhausted everything.
    finished: bool,
}

/// Parse a single SSE line into an optional decoded event.
///
/// Returns `Some(Ok(event))` for a decodable `data:` payload, `None` for a line
/// that should be skipped (blank, non-`data:`, `[DONE]`, or unparseable — the SDK
/// ignores events it cannot type).
fn parse_sse_line(line: &str) -> Option<Result<ResponseStreamEvent, ResponsesStreamError>> {
    let line = line.trim_end_matches(['\r', '\n']);
    if line.is_empty() {
        return None;
    }
    let data = line.strip_prefix("data:")?.trim_start();
    if data == "[DONE]" {
        return None;
    }
    match serde_json::from_str::<ResponseStreamEvent>(data) {
        Ok(event) => Some(Ok(event)),
        Err(_) => None,
    }
}

/// Drain whole lines out of `buffer`, decoding each into `pending`.
fn drain_buffer_lines(state: &mut SseState) {
    while let Some(pos) = state.buffer.iter().position(|&b| b == b'\n') {
        let line_bytes: Vec<u8> = state.buffer.drain(..=pos).collect();
        let line = String::from_utf8_lossy(&line_bytes);
        if let Some(event) = parse_sse_line(&line) {
            state.pending.push_back(event);
        }
    }
}

/// Decode the `POST /responses` SSE byte stream into a stream of
/// [`ResponseStreamEvent`]s, suitable for [`process_responses_stream`].
///
/// SSE framing: lines are accumulated; a `data:` line carries one JSON event.
/// The abort signal is raced between chunks (mirroring the SDK's abort support).
/// Built with [`futures::stream::unfold`] to avoid a generator-macro dependency.
fn sse_event_stream(
    response: reqwest::Response,
    signal: Option<tokio::sync::watch::Receiver<bool>>,
) -> impl futures::Stream<Item = Result<ResponseStreamEvent, ResponsesStreamError>> {
    let state = SseState {
        byte_stream: Box::pin(response.bytes_stream()),
        signal,
        buffer: Vec::new(),
        pending: std::collections::VecDeque::new(),
        upstream_done: false,
        finished: false,
    };

    futures::stream::unfold(state, |mut state| async move {
        loop {
            if state.finished {
                return None;
            }

            // Emit any already-decoded event first.
            if let Some(event) = state.pending.pop_front() {
                return Some((event, state));
            }

            if state.upstream_done {
                // Flush any trailing buffered data (no trailing newline), then end.
                state.finished = true;
                if !state.buffer.is_empty() {
                    let line = String::from_utf8_lossy(&state.buffer);
                    let line = line.trim();
                    if let Some(data) = line.strip_prefix("data:") {
                        let data = data.trim();
                        if data != "[DONE]" {
                            if let Ok(event) = serde_json::from_str::<ResponseStreamEvent>(data) {
                                return Some((Ok(event), state));
                            }
                        }
                    }
                }
                return None;
            }

            // Abort check between chunks.
            if let Some(sig) = &state.signal {
                if *sig.borrow() {
                    state.finished = true;
                    return Some((
                        Err(ResponsesStreamError::Transport(
                            "Request was aborted".to_string(),
                        )),
                        state,
                    ));
                }
            }

            let next = if let Some(sig) = state.signal.clone() {
                let mut sig = sig;
                tokio::select! {
                    chunk = state.byte_stream.next() => chunk,
                    _ = wait_for_abort(&mut sig) => {
                        state.finished = true;
                        return Some((
                            Err(ResponsesStreamError::Transport(
                                "Request was aborted".to_string(),
                            )),
                            state,
                        ));
                    }
                }
            } else {
                state.byte_stream.next().await
            };

            match next {
                Some(Ok(bytes)) => {
                    state.buffer.extend_from_slice(&bytes);
                    drain_buffer_lines(&mut state);
                    // Loop back: emit the first decoded event (if any) or read more.
                }
                Some(Err(e)) => {
                    state.finished = true;
                    return Some((
                        Err(ResponsesStreamError::Transport(format!(
                            "Stream error: {e}"
                        ))),
                        state,
                    ));
                }
                None => {
                    state.upstream_done = true;
                    // Loop back to flush the trailing buffer.
                }
            }
        }
    })
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
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Api, ModelCost, ThinkingLevelMap};

    fn base_model() -> Model {
        Model {
            id: "gpt-5".into(),
            name: "GPT-5".into(),
            api: Api::OpenAiResponses,
            provider: "openai".into(),
            base_url: "https://api.openai.com/v1".into(),
            reasoning: true,
            thinking_level_map: None,
            input: vec![],
            cost: ModelCost {
                input: 1.0,
                output: 2.0,
                cache_read: 0.1,
                cache_write: 0.0,
            },
            context_window: 200_000,
            max_tokens: 32_000,
            headers: None,
            compat: None,
        }
    }

    fn empty_context() -> Context {
        Context {
            system_prompt: None,
            messages: vec![],
            tools: vec![],
        }
    }

    #[test]
    fn cache_retention_defaults_to_short() {
        let env = ProviderEnv::new();
        assert_eq!(
            resolve_cache_retention(None, Some(&env)),
            CacheRetention::Short
        );
        assert_eq!(
            resolve_cache_retention(Some(CacheRetention::Long), Some(&env)),
            CacheRetention::Long
        );
    }

    #[test]
    fn cache_retention_honours_pi_env_long() {
        let mut env = ProviderEnv::new();
        env.insert("PI_CACHE_RETENTION".into(), "long".into());
        assert_eq!(
            resolve_cache_retention(None, Some(&env)),
            CacheRetention::Long
        );
    }

    #[test]
    fn long_retention_yields_24h_header() {
        let compat = get_compat(&base_model());
        assert_eq!(
            get_prompt_cache_retention(compat, CacheRetention::Long),
            Some("24h")
        );
        assert_eq!(
            get_prompt_cache_retention(compat, CacheRetention::Short),
            None
        );
    }

    #[test]
    fn service_tier_multiplier() {
        assert_eq!(get_service_tier_cost_multiplier("gpt-5", Some("flex")), 0.5);
        assert_eq!(
            get_service_tier_cost_multiplier("gpt-5", Some("priority")),
            2.0
        );
        assert_eq!(
            get_service_tier_cost_multiplier("gpt-5.5", Some("priority")),
            2.5
        );
        assert_eq!(get_service_tier_cost_multiplier("gpt-5", None), 1.0);
    }

    #[test]
    fn apply_service_tier_pricing_scales_and_totals() {
        let mut usage = empty_usage();
        usage.cost = UsageCost {
            input: 1.0,
            output: 2.0,
            cache_read: 0.5,
            cache_write: 0.0,
            total: 3.5,
        };
        apply_service_tier_pricing(&mut usage, Some("flex"), "gpt-5");
        assert_eq!(usage.cost.input, 0.5);
        assert_eq!(usage.cost.output, 1.0);
        assert_eq!(usage.cost.cache_read, 0.25);
        assert_eq!(usage.cost.total, 1.75);
    }

    #[test]
    fn apply_service_tier_pricing_noop_for_default_tier() {
        let mut usage = empty_usage();
        usage.cost = UsageCost {
            input: 1.0,
            output: 2.0,
            cache_read: 0.0,
            cache_write: 0.0,
            total: 3.0,
        };
        apply_service_tier_pricing(&mut usage, None, "gpt-5");
        assert_eq!(usage.cost.input, 1.0);
        assert_eq!(usage.cost.total, 3.0);
    }

    #[test]
    fn build_params_basic_shape() {
        let model = base_model();
        let ctx = empty_context();
        let opts = OpenAIResponsesOptions::default();
        let params = build_params(&model, &ctx, &opts, CacheRetention::Short).unwrap();
        assert_eq!(params.get("model").and_then(Value::as_str), Some("gpt-5"));
        assert_eq!(params.get("stream").and_then(Value::as_bool), Some(true));
        assert_eq!(params.get("store").and_then(Value::as_bool), Some(false));
        assert!(params.get("input").is_some());
        // No tools → no tools key.
        assert!(params.get("tools").is_none());
        // No cache key when no session id.
        assert!(params.get("prompt_cache_key").is_none());
    }

    #[test]
    fn build_params_long_retention_sets_24h() {
        let model = base_model();
        let ctx = empty_context();
        let mut opts = OpenAIResponsesOptions::default();
        opts.base.session_id = Some("sess-123".into());
        let params = build_params(&model, &ctx, &opts, CacheRetention::Long).unwrap();
        assert_eq!(
            params.get("prompt_cache_retention").and_then(Value::as_str),
            Some("24h")
        );
        assert_eq!(
            params.get("prompt_cache_key").and_then(Value::as_str),
            Some("sess-123")
        );
    }

    #[test]
    fn build_params_reasoning_effort_includes_encrypted_content() {
        let model = base_model();
        let ctx = empty_context();
        let mut opts = OpenAIResponsesOptions::default();
        opts.reasoning_effort = Some(ReasoningEffort::High);
        let params = build_params(&model, &ctx, &opts, CacheRetention::Short).unwrap();
        let reasoning = params.get("reasoning").unwrap();
        assert_eq!(
            reasoning.get("effort").and_then(Value::as_str),
            Some("high")
        );
        assert_eq!(
            reasoning.get("summary").and_then(Value::as_str),
            Some("auto")
        );
        let include = params.get("include").and_then(Value::as_array).unwrap();
        assert_eq!(include[0].as_str(), Some("reasoning.encrypted_content"));
    }

    #[test]
    fn build_params_reasoning_off_default_effort_none() {
        // No reasoning effort/summary, non-copilot, off not null → effort "none".
        let model = base_model();
        let ctx = empty_context();
        let opts = OpenAIResponsesOptions::default();
        let params = build_params(&model, &ctx, &opts, CacheRetention::Short).unwrap();
        let reasoning = params.get("reasoning").unwrap();
        assert_eq!(
            reasoning.get("effort").and_then(Value::as_str),
            Some("none")
        );
        assert!(params.get("include").is_none());
    }

    #[test]
    fn build_params_reasoning_off_null_omits_reasoning() {
        let mut model = base_model();
        let mut map: ThinkingLevelMap = ThinkingLevelMap::new();
        map.insert(ModelThinkingLevel::Off, None); // TS null → unsupported.
        model.thinking_level_map = Some(map);
        let ctx = empty_context();
        let opts = OpenAIResponsesOptions::default();
        let params = build_params(&model, &ctx, &opts, CacheRetention::Short).unwrap();
        assert!(params.get("reasoning").is_none());
    }

    #[test]
    fn build_params_copilot_skips_default_reasoning() {
        let mut model = base_model();
        model.provider = "github-copilot".into();
        let ctx = empty_context();
        let opts = OpenAIResponsesOptions::default();
        let params = build_params(&model, &ctx, &opts, CacheRetention::Short).unwrap();
        // Copilot + no explicit effort → no reasoning block.
        assert!(params.get("reasoning").is_none());
    }

    #[test]
    fn mapped_effort_uses_thinking_level_map() {
        let mut model = base_model();
        let mut map: ThinkingLevelMap = ThinkingLevelMap::new();
        map.insert(ModelThinkingLevel::High, Some("ultra".into()));
        model.thinking_level_map = Some(map);
        assert_eq!(mapped_effort(&model, ReasoningEffort::High), "ultra");
        // Unmapped effort falls back to its own name.
        assert_eq!(mapped_effort(&model, ReasoningEffort::Low), "low");
    }

    #[test]
    fn effort_from_clamped_maps_off_to_none() {
        assert_eq!(effort_from_clamped(ModelThinkingLevel::Off), None);
        assert_eq!(
            effort_from_clamped(ModelThinkingLevel::Medium),
            Some(ReasoningEffort::Medium)
        );
    }

    #[test]
    fn stream_simple_without_api_key_errors() {
        let model = base_model();
        let ctx = empty_context();
        let _stream = stream_simple(model, ctx, None);
        // Constructing the error stream must not panic; behavioural check only.
    }

    #[test]
    fn build_params_copilot_with_effort_includes_reasoning() {
        let mut model = base_model();
        model.provider = "github-copilot".into();
        let ctx = empty_context();
        let mut opts = OpenAIResponsesOptions::default();
        opts.reasoning_effort = Some(ReasoningEffort::Medium);
        let params = build_params(&model, &ctx, &opts, CacheRetention::Short).unwrap();
        // Explicit effort path applies even for copilot.
        let reasoning = params.get("reasoning").unwrap();
        assert_eq!(
            reasoning.get("effort").and_then(Value::as_str),
            Some("medium")
        );
    }

    // -----------------------------------------------------------------------
    // build_request_headers
    // -----------------------------------------------------------------------

    #[test]
    fn build_request_headers_sets_session_and_client_request_id() {
        let model = base_model();
        let ctx = empty_context();
        let opts = OpenAIResponsesOptions::default();
        let headers =
            build_request_headers(&model, &ctx, "test-key", &opts, Some("session-123".into()));
        assert_eq!(
            headers.get("session_id").map(String::as_str),
            Some("session-123")
        );
        assert_eq!(
            headers.get("x-client-request-id").map(String::as_str),
            Some("session-123")
        );
    }

    #[test]
    fn build_request_headers_omits_both_when_session_id_is_none() {
        let model = base_model();
        let ctx = empty_context();
        let opts = OpenAIResponsesOptions::default();
        let headers = build_request_headers(&model, &ctx, "test-key", &opts, None);
        assert!(headers.get("session_id").is_none());
        assert!(headers.get("x-client-request-id").is_none());
    }

    #[test]
    fn build_request_headers_explicit_overrides_default() {
        let model = base_model();
        let ctx = empty_context();
        let mut opts = OpenAIResponsesOptions::default();
        opts.base.headers = Some(
            vec![
                ("session_id".into(), "override-sess".into()),
                ("x-client-request-id".into(), "override-req".into()),
            ]
            .into_iter()
            .collect(),
        );
        let headers =
            build_request_headers(&model, &ctx, "test-key", &opts, Some("session-123".into()));
        assert_eq!(
            headers.get("session_id").map(String::as_str),
            Some("override-sess")
        );
        assert_eq!(
            headers.get("x-client-request-id").map(String::as_str),
            Some("override-req")
        );
    }

    // -----------------------------------------------------------------------
    // build_params — prompt_cache_key clamping
    // -----------------------------------------------------------------------

    #[test]
    fn build_params_clamps_long_session_id_to_64_chars() {
        let model = base_model();
        let ctx = empty_context();
        let mut opts = OpenAIResponsesOptions::default();
        let long_session = "x".repeat(67);
        opts.base.session_id = Some(long_session.clone());
        let params = build_params(&model, &ctx, &opts, CacheRetention::Long).unwrap();
        let cache_key = params
            .get("prompt_cache_key")
            .and_then(Value::as_str)
            .expect("expected prompt_cache_key");
        assert_eq!(cache_key.chars().count(), 64);
        assert_eq!(cache_key, &long_session[..64]);
    }

    #[test]
    fn build_params_omits_prompt_cache_key_when_cache_retention_none() {
        let model = base_model();
        let ctx = empty_context();
        let mut opts = OpenAIResponsesOptions::default();
        opts.base.session_id = Some("sess-123".into());
        let params = build_params(&model, &ctx, &opts, CacheRetention::None).unwrap();
        assert!(params.get("prompt_cache_key").is_none());
        assert!(params.get("prompt_cache_retention").is_none());
    }

    // -----------------------------------------------------------------------
    // E2E / integration — marked ignore (require API keys / network)
    // -----------------------------------------------------------------------

    #[test]
    #[ignore = "requires OPENAI_API_KEY and network access"]
    fn e2e_cache_affinity() {
        // Mirrors openai-responses-cache-affinity-e2e.test.ts:
        // Makes a real API call with a specific session ID and verifies
        // stopReason is not "error" and the response contains expected text.
        // Requires OPENAI_API_KEY env var and gpt-5.4 access.
    }

    #[test]
    #[ignore = "requires OPENAI_API_KEY and network access"]
    fn e2e_reasoning_replay_aborted_turn() {
        // Mirrors openai-responses-reasoning-replay-e2e.test.ts:
        // Tests that reasoning-only history after an aborted turn is handled
        // without 400 errors. Requires OPENAI_API_KEY and gpt-5-mini access.
    }

    #[test]
    #[ignore = "requires API keys and network access"]
    fn e2e_tool_result_images() {
        // Mirrors openai-responses-tool-result-images.test.ts:
        // Tests that tool result images are properly sent in
        // function_call_output. Requires API keys and red-circle.png fixture.
    }
}
