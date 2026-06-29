//! Port of `packages/ai/src/providers/azure-openai-responses.ts`.
//!
//! Azure OpenAI Responses provider backend — closely related to the OpenAI
//! Responses provider, but with Azure base-URL / deployment-name resolution.
//!
//! Unlike the TS source — which uses the `openai` SDK's `AzureOpenAI` client —
//! this port issues raw HTTP requests with `reqwest` and parses the OpenAI
//! Responses `?stream=true` Server-Sent Events stream manually. The shared
//! message/tool conversion and SSE event handling live in
//! [`crate::providers::openai_responses_shared`] and are reused here exactly as
//! the TS imports them from `openai-responses-shared.ts`.
//!
//! Entry points:
//! - [`stream`] (a.k.a. `stream_azure_openai_responses`) — the full provider stream.
//! - [`stream_simple`] (a.k.a. `stream_simple_azure_openai_responses`) — the
//!   `SimpleStreamOptions` wrapper that maps a reasoning level to a reasoning
//!   effort.

use std::collections::HashSet;

use serde_json::Value;
use url::Url;

use crate::models::clamp_thinking_level;
use crate::providers::openai_prompt_cache::clamp_openai_prompt_cache_key;
use crate::providers::openai_responses_shared::{
    ConvertResponsesToolsOptions, ResponseStreamEvent, ResponsesStreamError,
    convert_responses_messages, convert_responses_tools, process_responses_stream,
};
use crate::providers::simple_options::build_base_options;
use crate::types::{
    AssistantMessage, AssistantMessageEvent, Context, DoneReason, ErrorReason, MessageRole, Model,
    ModelThinkingLevel, ProviderResponse, SimpleStreamOptions, StopReason, StreamOptions, Usage,
    UsageCost,
};
use crate::utils::event_stream::{
    AssistantMessageEventStream, AssistantMessageEventStreamSender,
    create_assistant_message_event_stream,
};
use crate::utils::provider_env::get_provider_env_value;

// ---------------------------------------------------------------------------
// Constants (mirror TS module-level constants)
// ---------------------------------------------------------------------------

const DEFAULT_AZURE_API_VERSION: &str = "v1";

/// Providers that participate in Azure tool-call id normalization. Mirrors the TS
/// `AZURE_TOOL_CALL_PROVIDERS`.
fn azure_tool_call_providers() -> HashSet<String> {
    [
        "openai",
        "openai-codex",
        "opencode",
        "azure-openai-responses",
    ]
    .into_iter()
    .map(|s| s.to_string())
    .collect()
}

// ---------------------------------------------------------------------------
// Reasoning effort / summary enums (mirror the TS option string unions)
// ---------------------------------------------------------------------------

/// `reasoningEffort?: "minimal" | "low" | "medium" | "high" | "xhigh"`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AzureReasoningEffort {
    Minimal,
    Low,
    Medium,
    High,
    XHigh,
}

impl AzureReasoningEffort {
    fn as_str(self) -> &'static str {
        match self {
            AzureReasoningEffort::Minimal => "minimal",
            AzureReasoningEffort::Low => "low",
            AzureReasoningEffort::Medium => "medium",
            AzureReasoningEffort::High => "high",
            AzureReasoningEffort::XHigh => "xhigh",
        }
    }

    /// Key used to look up the model's `thinkingLevelMap`.
    fn as_model_thinking_level(self) -> ModelThinkingLevel {
        match self {
            AzureReasoningEffort::Minimal => ModelThinkingLevel::Minimal,
            AzureReasoningEffort::Low => ModelThinkingLevel::Low,
            AzureReasoningEffort::Medium => ModelThinkingLevel::Medium,
            AzureReasoningEffort::High => ModelThinkingLevel::High,
            AzureReasoningEffort::XHigh => ModelThinkingLevel::XHigh,
        }
    }
}

impl From<ModelThinkingLevel> for AzureReasoningEffort {
    fn from(level: ModelThinkingLevel) -> Self {
        match level {
            // `off` never reaches here (filtered upstream); fold to medium-ish default.
            ModelThinkingLevel::Off => AzureReasoningEffort::Medium,
            ModelThinkingLevel::Minimal => AzureReasoningEffort::Minimal,
            ModelThinkingLevel::Low => AzureReasoningEffort::Low,
            ModelThinkingLevel::Medium => AzureReasoningEffort::Medium,
            ModelThinkingLevel::High => AzureReasoningEffort::High,
            ModelThinkingLevel::XHigh => AzureReasoningEffort::XHigh,
        }
    }
}

/// `reasoningSummary?: "auto" | "detailed" | "concise" | null`.
///
/// The TS `null` (explicitly disable summary) is represented by `Some(None)`
/// at the option level; here the enum just covers the three string values.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AzureReasoningSummary {
    Auto,
    Detailed,
    Concise,
}

impl AzureReasoningSummary {
    fn as_str(self) -> &'static str {
        match self {
            AzureReasoningSummary::Auto => "auto",
            AzureReasoningSummary::Detailed => "detailed",
            AzureReasoningSummary::Concise => "concise",
        }
    }
}

// ---------------------------------------------------------------------------
// Options
// ---------------------------------------------------------------------------

/// Azure OpenAI Responses-specific stream options.
///
/// Mirrors the TS `interface AzureOpenAIResponsesOptions extends StreamOptions`.
/// The base [`StreamOptions`] is embedded (matching the `google.rs` shape) so
/// [`build_base_options`] can copy it directly.
#[derive(Clone, Debug, Default)]
pub struct AzureOpenAIResponsesOptions {
    pub base: StreamOptions,
    pub reasoning_effort: Option<AzureReasoningEffort>,
    /// `reasoningSummary`: `Some(Some(_))` is an explicit summary, `Some(None)`
    /// is the TS `null` (disable), `None` is absent.
    pub reasoning_summary: Option<Option<AzureReasoningSummary>>,
    pub azure_api_version: Option<String>,
    pub azure_resource_name: Option<String>,
    pub azure_base_url: Option<String>,
    pub azure_deployment_name: Option<String>,
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
        api: "azure-openai-responses".to_string(),
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
// Deployment-name resolution
// ---------------------------------------------------------------------------

/// Parse the `AZURE_OPENAI_DEPLOYMENT_NAME_MAP` value (`modelId=deployment,...`).
///
/// Mirrors the TS `parseDeploymentNameMap`.
fn parse_deployment_name_map(value: Option<&str>) -> std::collections::HashMap<String, String> {
    let mut map = std::collections::HashMap::new();
    let value = match value {
        Some(v) => v,
        None => return map,
    };
    for entry in value.split(',') {
        let trimmed = entry.trim();
        if trimmed.is_empty() {
            continue;
        }
        let mut parts = trimmed.splitn(2, '=');
        let model_id = parts.next().unwrap_or("").trim();
        let deployment_name = parts.next().unwrap_or("").trim();
        if model_id.is_empty() || deployment_name.is_empty() {
            continue;
        }
        map.insert(model_id.to_string(), deployment_name.to_string());
    }
    map
}

/// Resolve the Azure deployment name for a model.
///
/// Mirrors the TS `resolveDeploymentName`.
fn resolve_deployment_name(model: &Model, options: &AzureOpenAIResponsesOptions) -> String {
    if let Some(name) = &options.azure_deployment_name {
        return name.clone();
    }
    let env = options.base.env.as_ref();
    let mapped = parse_deployment_name_map(
        get_provider_env_value("AZURE_OPENAI_DEPLOYMENT_NAME_MAP", env).as_deref(),
    )
    .get(&model.id)
    .cloned();
    mapped.unwrap_or_else(|| model.id.clone())
}

// ---------------------------------------------------------------------------
// Azure base-URL resolution
// ---------------------------------------------------------------------------

/// Normalize an Azure (or proxy) base URL.
///
/// Mirrors the TS `normalizeAzureBaseUrl`. Returns `Err(message)` on an invalid
/// URL (mirroring the TS `throw`).
fn normalize_azure_base_url(base_url: &str) -> Result<String, String> {
    // `baseUrl.trim().replace(/\/+$/, "")`
    let trimmed = base_url.trim().trim_end_matches('/');

    let mut url =
        Url::parse(trimmed).map_err(|_| format!("Invalid Azure OpenAI base URL: {base_url}"))?;

    let host = url.host_str().unwrap_or("").to_string();
    let is_azure_host =
        host.ends_with(".openai.azure.com") || host.ends_with(".cognitiveservices.azure.com");

    // `url.pathname.replace(/\/+$/, "")`
    let normalized_path = url.path().trim_end_matches('/').to_string();

    // Ensure Azure hosts have /openai/v1 as base path so the deployment + api
    // version can be appended correctly.
    if is_azure_host
        && (normalized_path.is_empty() || normalized_path == "/" || normalized_path == "/openai")
    {
        url.set_path("/openai/v1");
        url.set_query(None);
    }

    // `url.toString().replace(/\/+$/, "")`
    Ok(url.to_string().trim_end_matches('/').to_string())
}

/// Mirrors the TS `buildDefaultBaseUrl`.
fn build_default_base_url(resource_name: &str) -> String {
    format!("https://{resource_name}.openai.azure.com/openai/v1")
}

/// Resolved Azure configuration.
#[derive(Debug)]
struct AzureConfig {
    base_url: String,
    api_version: String,
}

/// Resolve the Azure base URL and API version.
///
/// Mirrors the TS `resolveAzureConfig`. Returns `Err(message)` when no base URL
/// can be resolved (mirroring the TS `throw`).
fn resolve_azure_config(
    model: &Model,
    options: &AzureOpenAIResponsesOptions,
) -> Result<AzureConfig, String> {
    let env = options.base.env.as_ref();

    let api_version = options
        .azure_api_version
        .clone()
        .filter(|s| !s.is_empty())
        .or_else(|| {
            get_provider_env_value("AZURE_OPENAI_API_VERSION", env).filter(|s| !s.is_empty())
        })
        .unwrap_or_else(|| DEFAULT_AZURE_API_VERSION.to_string());

    // `options?.azureBaseUrl?.trim() || env("AZURE_OPENAI_BASE_URL")?.trim() || undefined`
    let base_url = options
        .azure_base_url
        .as_ref()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .or_else(|| {
            get_provider_env_value("AZURE_OPENAI_BASE_URL", env)
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
        });

    let resource_name = options
        .azure_resource_name
        .clone()
        .filter(|s| !s.is_empty())
        .or_else(|| {
            get_provider_env_value("AZURE_OPENAI_RESOURCE_NAME", env).filter(|s| !s.is_empty())
        });

    let mut resolved_base_url = base_url;

    if resolved_base_url.is_none() {
        if let Some(rn) = &resource_name {
            resolved_base_url = Some(build_default_base_url(rn));
        }
    }

    if resolved_base_url.is_none() && !model.base_url.is_empty() {
        resolved_base_url = Some(model.base_url.clone());
    }

    let resolved_base_url = match resolved_base_url {
        Some(u) => u,
        None => {
            return Err("Azure OpenAI base URL is required. Set AZURE_OPENAI_BASE_URL or AZURE_OPENAI_RESOURCE_NAME, or pass azureBaseUrl, azureResourceName, or model.baseUrl.".to_string());
        }
    };

    Ok(AzureConfig {
        base_url: normalize_azure_base_url(&resolved_base_url)?,
        api_version,
    })
}

// ---------------------------------------------------------------------------
// Public entry points
// ---------------------------------------------------------------------------

/// Stream a completion from an Azure OpenAI Responses model.
///
/// Mirrors the TS `streamAzureOpenAIResponses`.
pub fn stream(
    model: Model,
    context: Context,
    options: Option<AzureOpenAIResponsesOptions>,
) -> AssistantMessageEventStream {
    let (sender, stream_out) = create_assistant_message_event_stream();
    let options = options.unwrap_or_default();

    tokio::spawn(async move {
        run_stream(model, context, options, sender).await;
    });

    stream_out
}

/// TS-named alias for [`stream`].
pub fn stream_azure_openai_responses(
    model: Model,
    context: Context,
    options: Option<AzureOpenAIResponsesOptions>,
) -> AssistantMessageEventStream {
    stream(model, context, options)
}

/// Stream with simplified reasoning-level options.
///
/// Mirrors the TS `streamSimpleAzureOpenAIResponses`.
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
    let clamped_reasoning = options
        .as_ref()
        .and_then(|o| o.reasoning)
        .map(|r| clamp_thinking_level(&model, ModelThinkingLevel::from(r)));

    // `reasoningEffort = clampedReasoning === "off" ? undefined : clampedReasoning`
    let reasoning_effort = match clamped_reasoning {
        Some(ModelThinkingLevel::Off) | None => None,
        Some(level) => Some(AzureReasoningEffort::from(level)),
    };

    let opts = AzureOpenAIResponsesOptions {
        base,
        reasoning_effort,
        ..Default::default()
    };
    stream(model, context, Some(opts))
}

/// TS-named alias for [`stream_simple`].
pub fn stream_simple_azure_openai_responses(
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
// Error formatting
// ---------------------------------------------------------------------------

/// Mirrors the TS `formatAzureOpenAIError`. Here the input is already a string
/// message (Rust errors are surfaced as `String`); we keep an optional status
/// prefix for HTTP errors.
fn format_azure_openai_error(message: &str, status_code: Option<u16>) -> String {
    match status_code {
        Some(code) => format!("Azure OpenAI API error ({code}): {message}"),
        None => message.to_string(),
    }
}

// ---------------------------------------------------------------------------
// Request building
// ---------------------------------------------------------------------------

/// Build the Responses API request body.
///
/// Mirrors the TS `buildParams`.
fn build_params(
    model: &Model,
    context: &Context,
    options: &AzureOpenAIResponsesOptions,
    deployment_name: &str,
) -> Value {
    let messages = convert_responses_messages(model, context, &azure_tool_call_providers(), None);

    let mut params = serde_json::Map::new();
    params.insert("model".into(), Value::String(deployment_name.to_string()));
    params.insert(
        "input".into(),
        serde_json::to_value(&messages).unwrap_or(Value::Null),
    );
    params.insert("stream".into(), Value::Bool(true));
    if let Some(key) = clamp_openai_prompt_cache_key(options.base.session_id.as_deref()) {
        params.insert("prompt_cache_key".into(), Value::String(key));
    } else {
        // `clampOpenAIPromptCacheKey(undefined)` is `undefined`; the SDK omits the
        // key, but TS sets `prompt_cache_key: <value-or-undefined>`. Setting JSON
        // `null` would change the wire shape, so omit it entirely when absent.
    }
    params.insert("store".into(), Value::Bool(false));

    if let Some(max_tokens) = options.base.max_tokens {
        if max_tokens != 0 {
            params.insert("max_output_tokens".into(), serde_json::json!(max_tokens));
        }
    }

    if let Some(temperature) = options.base.temperature {
        params.insert("temperature".into(), serde_json::json!(temperature));
    }

    if !context.tools.is_empty() {
        let tools = convert_responses_tools(
            &context.tools,
            Some(&ConvertResponsesToolsOptions::default()),
        );
        params.insert(
            "tools".into(),
            serde_json::to_value(&tools).unwrap_or(Value::Null),
        );
    }

    if model.reasoning {
        let has_effort = options.reasoning_effort.is_some();
        // `options?.reasoningSummary` is truthy for an explicit summary; `Some(None)`
        // (TS `null`) and `None` are both falsy.
        let summary_is_truthy = matches!(options.reasoning_summary, Some(Some(_)));

        if has_effort || summary_is_truthy {
            // effort = options.reasoningEffort
            //   ? (model.thinkingLevelMap?.[options.reasoningEffort] ?? options.reasoningEffort)
            //   : "medium"
            let effort: String = match options.reasoning_effort {
                Some(effort) => {
                    let mapped = model
                        .thinking_level_map
                        .as_ref()
                        .and_then(|m| m.get(&effort.as_model_thinking_level()))
                        .and_then(|v| v.clone());
                    mapped.unwrap_or_else(|| effort.as_str().to_string())
                }
                None => "medium".to_string(),
            };

            // summary = options?.reasoningSummary || "auto"
            let summary = match options.reasoning_summary {
                Some(Some(s)) => s.as_str().to_string(),
                _ => "auto".to_string(),
            };

            params.insert(
                "reasoning".into(),
                serde_json::json!({ "effort": effort, "summary": summary }),
            );
            params.insert(
                "include".into(),
                serde_json::json!(["reasoning.encrypted_content"]),
            );
        } else {
            // `else if (model.thinkingLevelMap?.off !== null)` — i.e. unless `off` is
            // explicitly mapped to JSON `null`.
            let off_is_null = matches!(
                model
                    .thinking_level_map
                    .as_ref()
                    .map(|m| m.get(&ModelThinkingLevel::Off)),
                Some(Some(None))
            );
            if !off_is_null {
                // effort = model.thinkingLevelMap?.off ?? "none"
                let effort = model
                    .thinking_level_map
                    .as_ref()
                    .and_then(|m| m.get(&ModelThinkingLevel::Off))
                    .and_then(|v| v.clone())
                    .unwrap_or_else(|| "none".to_string());
                params.insert("reasoning".into(), serde_json::json!({ "effort": effort }));
            }
        }
    }

    Value::Object(params)
}

/// Build the streaming request URL: `<baseUrl>/responses?api-version=<v>`.
///
/// The TS `AzureOpenAI` SDK appends `/responses` to the configured `baseURL` and
/// attaches `?api-version=<apiVersion>`.
fn build_url(config: &AzureConfig) -> String {
    let base = config.base_url.trim_end_matches('/');
    format!("{}/responses?api-version={}", base, config.api_version)
}

// ---------------------------------------------------------------------------
// Stream driver
// ---------------------------------------------------------------------------

async fn run_stream(
    model: Model,
    context: Context,
    options: AzureOpenAIResponsesOptions,
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
        Err((message, status_code)) => {
            // Mirror the TS catch: strip streaming-only scratch fields from blocks.
            // Our `AssistantContentBlock` variants carry no `index`/`partialJson`
            // fields (they are streaming-local in this port), so nothing to delete.
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
            output.error_message = Some(format_azure_openai_error(&message, status_code));
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

/// The core streaming logic. Returns `Err((message, status_code))` on failure,
/// which the caller converts into a terminal error event.
async fn run_stream_inner(
    model: &Model,
    context: &Context,
    options: &AzureOpenAIResponsesOptions,
    sender: &mut AssistantMessageEventStreamSender,
    output: &mut AssistantMessage,
) -> Result<(), (String, Option<u16>)> {
    // const apiKey = options?.apiKey; if (!apiKey) throw ...
    let api_key = match &options.base.api_key {
        Some(k) if !k.is_empty() => k.clone(),
        _ => return Err((format!("No API key for provider: {}", model.provider), None)),
    };

    let deployment_name = resolve_deployment_name(model, options);
    let config = resolve_azure_config(model, options).map_err(|e| (e, None))?;

    // params = buildParams(...)
    let mut params = build_params(model, context, options, &deployment_name);

    // onPayload hook.
    if let Some(on_payload) = &options.base.on_payload {
        if let Some(next) = on_payload(params.clone(), model.clone()).await {
            params = next;
        }
    }

    let url = build_url(&config);

    let client = reqwest::Client::new();
    let mut request = client
        .post(&url)
        .header("api-key", &api_key)
        .header("content-type", "application/json")
        .header("accept", "text/event-stream");

    // createClient merges `model.headers` then `options.headers` (options win).
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

    let body_bytes = serde_json::to_vec(&params).map_err(|e| (e.to_string(), None))?;
    request = request.body(body_bytes);

    // Send (raced against the abort signal).
    let response = send_with_abort(request, options.base.signal.clone())
        .await
        .map_err(|e| (e, None))?;

    let status = response.status();

    // onResponse hook (status + headers), before consuming the body.
    if let Some(on_response) = &options.base.on_response {
        let headers = crate::utils::headers::headers_to_record(response.headers());
        on_response(
            ProviderResponse {
                status: status.as_u16(),
                headers,
            },
            model.clone(),
        )
        .await;
    }

    if !status.is_success() {
        let code = status.as_u16();
        let text = response.text().await.unwrap_or_default();
        return Err((text, Some(code)));
    }

    // stream.push({ type: "start", partial: output })
    sender.push(AssistantMessageEvent::Start {
        partial: output.clone(),
    });

    // Build an SSE event stream feeding `process_responses_stream`.
    // `SseResponseStream` is `Unpin` (all fields are `Unpin`), satisfying the
    // `S: Stream + Unpin` bound, so it can be passed by value.
    let byte_stream = response.bytes_stream();
    let event_stream = SseResponseStream::new(byte_stream, options.base.signal.clone());

    process_responses_stream(event_stream, output, sender, model, None)
        .await
        .map_err(|e| match e {
            ResponsesStreamError::Provider(m) => (m, None),
            ResponsesStreamError::Transport(m) => (m, None),
        })?;

    // Post-loop abort/error finalization (mirrors the TS throws).
    if let Some(signal) = &options.base.signal {
        if *signal.borrow() {
            return Err(("Request was aborted".to_string(), None));
        }
    }
    if output.stop_reason == StopReason::Aborted || output.stop_reason == StopReason::Error {
        return Err(("An unknown error occurred".to_string(), None));
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Abort helpers (copied from the google.rs pattern)
// ---------------------------------------------------------------------------

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
// SSE → ResponseStreamEvent adapter
// ---------------------------------------------------------------------------

/// Adapts a raw byte stream (the Responses `?stream=true` SSE body) into a
/// [`futures::Stream`] of decoded [`ResponseStreamEvent`]s, as expected by
/// [`process_responses_stream`].
///
/// SSE framing: lines are buffered; a `data:` line carries a JSON payload; a
/// blank line terminates an event. `data: [DONE]` is ignored. The abort signal
/// is raced between chunks.
struct SseResponseStream<B> {
    byte_stream: B,
    buffer: Vec<u8>,
    /// Decoded events ready to yield (a single chunk may complete several).
    pending: std::collections::VecDeque<Result<ResponseStreamEvent, ResponsesStreamError>>,
    signal: Option<tokio::sync::watch::Receiver<bool>>,
    done: bool,
}

impl<B> SseResponseStream<B> {
    fn new(byte_stream: B, signal: Option<tokio::sync::watch::Receiver<bool>>) -> Self {
        Self {
            byte_stream,
            buffer: Vec::new(),
            pending: std::collections::VecDeque::new(),
            signal,
            done: false,
        }
    }
}

impl<B> SseResponseStream<B> {
    /// Drain complete `\n`-terminated SSE lines from the buffer into `pending`.
    fn drain_lines(&mut self) {
        while let Some(pos) = self.buffer.iter().position(|&b| b == b'\n') {
            let line_bytes: Vec<u8> = self.buffer.drain(..=pos).collect();
            let line = String::from_utf8_lossy(&line_bytes);
            let line = line.trim_end_matches(['\r', '\n']);
            self.process_line(line);
        }
    }

    fn process_line(&mut self, line: &str) {
        if line.is_empty() {
            return;
        }
        let data = match line.strip_prefix("data:") {
            Some(d) => d.trim_start(),
            None => return,
        };
        if data == "[DONE]" {
            return;
        }
        match serde_json::from_str::<ResponseStreamEvent>(data) {
            Ok(event) => self.pending.push_back(Ok(event)),
            // Unknown / unparseable events decode to `Other` via `#[serde(other)]`
            // when they have a `type`; a hard parse failure (malformed JSON) is
            // skipped rather than aborting the stream.
            Err(_) => {}
        }
    }

    /// Flush any trailing buffered data without a final newline.
    fn flush_tail(&mut self) {
        if self.buffer.is_empty() {
            return;
        }
        let line = String::from_utf8_lossy(&self.buffer).to_string();
        self.buffer.clear();
        let line = line.trim();
        self.process_line(line);
    }
}

impl<B> futures::Stream for SseResponseStream<B>
where
    B: futures::Stream<Item = reqwest::Result<bytes::Bytes>> + Unpin,
{
    type Item = Result<ResponseStreamEvent, ResponsesStreamError>;

    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        use std::task::Poll;

        let this = self.get_mut();

        loop {
            // Yield any already-decoded event.
            if let Some(item) = this.pending.pop_front() {
                return Poll::Ready(Some(item));
            }
            if this.done {
                return Poll::Ready(None);
            }

            // Abort check between chunks.
            if let Some(signal) = &this.signal {
                if *signal.borrow() {
                    this.done = true;
                    return Poll::Ready(Some(Err(ResponsesStreamError::Transport(
                        "Request was aborted".to_string(),
                    ))));
                }
            }

            match futures::ready!(std::pin::Pin::new(&mut this.byte_stream).poll_next(cx)) {
                Some(Ok(chunk)) => {
                    this.buffer.extend_from_slice(&chunk);
                    this.drain_lines();
                    // Loop back to yield any newly-decoded events.
                }
                Some(Err(e)) => {
                    this.done = true;
                    return Poll::Ready(Some(Err(ResponsesStreamError::Transport(format!(
                        "Stream error: {e}"
                    )))));
                }
                None => {
                    this.flush_tail();
                    this.done = true;
                    // Loop back to drain any final pending events, then end.
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn opts_with_base_url(base_url: &str) -> AzureOpenAIResponsesOptions {
        AzureOpenAIResponsesOptions {
            azure_base_url: Some(base_url.to_string()),
            ..Default::default()
        }
    }

    fn test_model() -> Model {
        Model {
            id: "gpt-4o-mini".to_string(),
            name: "gpt-4o-mini".to_string(),
            api: crate::types::Api::AzureOpenAiResponses,
            provider: "azure-openai-responses".to_string(),
            base_url: String::new(),
            reasoning: false,
            thinking_level_map: None,
            input: Vec::new(),
            cost: crate::types::ModelCost {
                input: 0.0,
                output: 0.0,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 128_000,
            max_tokens: 16_384,
            headers: None,
            compat: None,
        }
    }

    fn resolve(base_url: &str) -> String {
        let model = test_model();
        let opts = opts_with_base_url(base_url);
        resolve_azure_config(&model, &opts).unwrap().base_url
    }

    #[test]
    fn normalizes_cognitive_services_root_to_openai_v1() {
        assert_eq!(
            resolve("https://marc-quicktests-resource.cognitiveservices.azure.com"),
            "https://marc-quicktests-resource.cognitiveservices.azure.com/openai/v1"
        );
    }

    #[test]
    fn normalizes_azure_openai_root_to_openai_v1() {
        assert_eq!(
            resolve("https://my-resource.openai.azure.com"),
            "https://my-resource.openai.azure.com/openai/v1"
        );
    }

    #[test]
    fn normalizes_openai_to_openai_v1() {
        assert_eq!(
            resolve("https://my-resource.cognitiveservices.azure.com/openai"),
            "https://my-resource.cognitiveservices.azure.com/openai/v1"
        );
    }

    #[test]
    fn preserves_openai_v1_endpoints() {
        assert_eq!(
            resolve("https://my-resource.cognitiveservices.azure.com/openai/v1"),
            "https://my-resource.cognitiveservices.azure.com/openai/v1"
        );
    }

    #[test]
    fn preserves_non_azure_proxy_paths() {
        assert_eq!(
            resolve("https://my-proxy.example.com/v1"),
            "https://my-proxy.example.com/v1"
        );
    }

    #[test]
    fn strips_query_params_when_normalizing_azure_host_urls() {
        assert_eq!(
            resolve("https://my-resource.openai.azure.com/openai?api-version=2024-12-01"),
            "https://my-resource.openai.azure.com/openai/v1"
        );
    }

    #[test]
    fn preserves_query_params_on_non_azure_proxy_urls() {
        assert_eq!(
            resolve("https://my-proxy.example.com/v1?custom=true"),
            "https://my-proxy.example.com/v1?custom=true"
        );
    }

    #[test]
    fn errors_on_invalid_urls() {
        let model = test_model();
        let opts = opts_with_base_url("not-a-url");
        let result = resolve_azure_config(&model, &opts);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .contains("Invalid Azure OpenAI base URL")
        );
    }

    #[test]
    fn builds_default_url_from_resource_name() {
        let model = test_model();
        let opts = AzureOpenAIResponsesOptions {
            azure_resource_name: Some("my-resource".to_string()),
            ..Default::default()
        };
        assert_eq!(
            resolve_azure_config(&model, &opts).unwrap().base_url,
            "https://my-resource.openai.azure.com/openai/v1"
        );
    }

    #[test]
    fn parse_deployment_name_map_parses_entries() {
        let map = parse_deployment_name_map(Some("a=dep-a, b=dep-b ,,c="));
        assert_eq!(map.get("a").map(String::as_str), Some("dep-a"));
        assert_eq!(map.get("b").map(String::as_str), Some("dep-b"));
        assert_eq!(map.get("c"), None);
    }

    #[test]
    fn resolve_deployment_name_prefers_explicit() {
        let model = test_model();
        let opts = AzureOpenAIResponsesOptions {
            azure_deployment_name: Some("explicit-dep".to_string()),
            ..Default::default()
        };
        assert_eq!(resolve_deployment_name(&model, &opts), "explicit-dep");
    }

    #[test]
    fn resolve_deployment_name_falls_back_to_model_id() {
        let model = test_model();
        let opts = AzureOpenAIResponsesOptions::default();
        assert_eq!(resolve_deployment_name(&model, &opts), "gpt-4o-mini");
    }

    #[test]
    fn build_params_sets_store_false_and_stream_true() {
        let model = test_model();
        let context = Context {
            messages: Vec::new(),
            system_prompt: None,
            tools: Vec::new(),
        };
        let opts = AzureOpenAIResponsesOptions {
            base: StreamOptions {
                session_id: Some("x".repeat(67)),
                ..Default::default()
            },
            ..Default::default()
        };
        let params = build_params(&model, &context, &opts, "dep");
        assert_eq!(params.get("store"), Some(&Value::Bool(false)));
        assert_eq!(params.get("stream"), Some(&Value::Bool(true)));
        assert_eq!(params.get("model"), Some(&Value::String("dep".to_string())));
        // prompt_cache_key clamped to 64 chars.
        assert_eq!(
            params.get("prompt_cache_key").and_then(|v| v.as_str()),
            Some("x".repeat(64).as_str())
        );
    }
}
