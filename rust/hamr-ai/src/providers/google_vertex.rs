//! Port of `../../packages/ai/src/providers/google-vertex.ts`.
//!
//! Google **Vertex AI** provider backend. This is near-identical to the sibling
//! Google Generative AI (Gemini) provider in [`crate::providers::google`] — the
//! request/response/SSE handling, thinking config, and tool-call bookkeeping are
//! the same. It differs only in **auth and endpoint resolution**:
//!
//! - The `api` tag is `"google-vertex"`.
//! - A Vertex **API key** path (`?key=` / `x-goog-api-key`) when a real key is
//!   supplied. Placeholder markers (`<authenticated>`-style, `gcp-vertex-credentials`)
//!   and missing keys fall back to ADC (Application Default Credentials).
//! - Otherwise, **ADC** with a resolved GCP `project` + `location`, hitting the
//!   regional `*-aiplatform.googleapis.com` endpoint.
//!
//! Like [`crate::providers::google`], this port issues raw `reqwest` HTTP and
//! parses Google's `streamGenerateContent?alt=sse` SSE manually instead of using
//! the `@google/genai` SDK. The shared message/tool/stop-reason/thinking helpers
//! live in [`crate::providers::google_shared`] and are reused here.
//!
//! ## ADC (type-debt)
//!
//! The TS source uses `google-auth-library` to mint an OAuth access token from
//! Application Default Credentials (a service-account key file or the gcloud
//! metadata server). There is **no equivalent Rust dependency available here and
//! we may not add one**. We therefore resolve a pre-acquired access token from the
//! environment (`GOOGLE_VERTEX_ACCESS_TOKEN` / `GOOGLE_OAUTH_ACCESS_TOKEN`, or a
//! `Bearer` header in `headers`). Full service-account JWT minting from
//! `GOOGLE_APPLICATION_CREDENTIALS` is left as `// TODO(adc)`.
//!
//! Entry points:
//! - [`stream`] (a.k.a. `stream_google_vertex`) — the full provider stream.
//! - [`stream_simple`] (a.k.a. `stream_simple_google_vertex`) — the
//!   `SimpleStreamOptions` wrapper that maps a reasoning level to thinking config.

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
use crate::utils::provider_env::get_provider_env_value;

// ---------------------------------------------------------------------------
// Constants (mirror the TS module-level constants)
// ---------------------------------------------------------------------------

const API_VERSION: &str = "v1";
const GCP_VERTEX_CREDENTIALS_MARKER: &str = "gcp-vertex-credentials";

// ---------------------------------------------------------------------------
// Options
// ---------------------------------------------------------------------------

/// Thinking sub-configuration for [`GoogleVertexOptions`].
///
/// Mirrors the TS `thinking?: { enabled; budgetTokens?; level? }`.
#[derive(Clone, Debug, Default)]
pub struct GoogleVertexThinking {
    pub enabled: bool,
    /// `-1` for dynamic, `0` to disable.
    pub budget_tokens: Option<i64>,
    pub level: Option<GoogleThinkingLevel>,
}

/// Google Vertex-specific stream options.
///
/// Mirrors the TS `GoogleVertexOptions extends StreamOptions`.
#[derive(Clone, Debug, Default)]
pub struct GoogleVertexOptions {
    pub base: StreamOptions,
    /// `"auto" | "none" | "any"`.
    pub tool_choice: Option<String>,
    pub thinking: Option<GoogleVertexThinking>,
    /// GCP project ID (ADC path).
    pub project: Option<String>,
    /// GCP location / region (ADC path).
    pub location: Option<String>,
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
        api: "google-vertex".to_string(),
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
/// Mirrors the TS `currentBlock: TextContent | ThinkingContent | null` that is
/// also pushed into `output.content`.
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
// Auth resolution: how the request should be authenticated.
// ---------------------------------------------------------------------------

/// How a Vertex request is authenticated. Mirrors the TS branch between
/// `createClientWithApiKey` and `createClient` (ADC).
enum VertexAuth {
    /// A real Vertex API key (`apiKey` query / header path).
    ApiKey(String),
    /// ADC path with a resolved project + location and an OAuth access token.
    Adc {
        project: String,
        location: String,
        access_token: String,
    },
}

// ---------------------------------------------------------------------------
// Public entry points
// ---------------------------------------------------------------------------

/// Stream a completion from a Google Vertex AI model.
///
/// Mirrors the TS `streamGoogleVertex`.
pub fn stream(
    model: Model,
    context: Context,
    options: Option<GoogleVertexOptions>,
) -> AssistantMessageEventStream {
    let (sender, stream_out) = create_assistant_message_event_stream();
    let options = options.unwrap_or_default();

    tokio::spawn(async move {
        run_stream(model, context, options, sender).await;
    });

    stream_out
}

/// TS-named alias for [`stream`].
pub fn stream_google_vertex(
    model: Model,
    context: Context,
    options: Option<GoogleVertexOptions>,
) -> AssistantMessageEventStream {
    stream(model, context, options)
}

/// Stream with simplified reasoning-level options.
///
/// Mirrors the TS `streamSimpleGoogleVertex`: maps the unified `reasoning` level
/// into Vertex thinking config (level for Gemini 3, budget tokens otherwise).
pub fn stream_simple(
    model: Model,
    context: Context,
    options: Option<SimpleStreamOptions>,
) -> AssistantMessageEventStream {
    // `buildBaseOptions(model, options, undefined)` — no apiKey override.
    let base = build_base_options(&model, options.as_ref(), None);

    let reasoning = options.as_ref().and_then(|o| o.reasoning);
    let thinking_budgets = options.as_ref().and_then(|o| o.thinking_budgets);

    // `if (!options?.reasoning)` → thinking disabled.
    let reasoning = match reasoning {
        Some(r) => r,
        None => {
            let opts = GoogleVertexOptions {
                base,
                tool_choice: None,
                thinking: Some(GoogleVertexThinking {
                    enabled: false,
                    budget_tokens: None,
                    level: None,
                }),
                project: None,
                location: None,
            };
            return stream(model, context, Some(opts));
        }
    };

    // clampThinkingLevel(model, options.reasoning)
    let clamped = clamp_thinking_level(&model, ModelThinkingLevel::from(reasoning));
    // effort = clamped === "off" ? "high" : clamped
    let effort = clamped_to_effort(clamped);

    if is_gemini3_pro_model(&model) || is_gemini3_flash_model(&model) {
        let opts = GoogleVertexOptions {
            base,
            tool_choice: None,
            thinking: Some(GoogleVertexThinking {
                enabled: true,
                budget_tokens: None,
                level: Some(get_gemini3_thinking_level(effort, &model)),
            }),
            project: None,
            location: None,
        };
        return stream(model, context, Some(opts));
    }

    let budget = get_google_budget(&model, effort, thinking_budgets.as_ref());
    let opts = GoogleVertexOptions {
        base,
        tool_choice: None,
        thinking: Some(GoogleVertexThinking {
            enabled: true,
            budget_tokens: Some(budget),
            level: None,
        }),
        project: None,
        location: None,
    };
    stream(model, context, Some(opts))
}

/// TS-named alias for [`stream_simple`].
pub fn stream_simple_google_vertex(
    model: Model,
    context: Context,
    options: Option<SimpleStreamOptions>,
) -> AssistantMessageEventStream {
    stream_simple(model, context, options)
}

// ---------------------------------------------------------------------------
// Effort level (ClampedThinkingLevel = Exclude<ThinkingLevel, "xhigh">)
// ---------------------------------------------------------------------------

/// The non-`xhigh` thinking levels used for Vertex budget/level resolution.
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
// Model identification helpers (mirror the TS regex helpers)
// ---------------------------------------------------------------------------

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
    // Gemini 2.x supports disabling via thinkingBudget = 0.
    serde_json::json!({ "thinkingBudget": 0 })
}

/// Mirrors the TS `getGemini3ThinkingLevel`.
fn get_gemini3_thinking_level(effort: Effort, model: &Model) -> GoogleThinkingLevel {
    if is_gemini3_pro_model(model) {
        return match effort {
            Effort::Minimal | Effort::Low => GoogleThinkingLevel::Low,
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
// Auth / endpoint resolution (mirror the TS resolve* helpers)
// ---------------------------------------------------------------------------

/// Mirrors the TS `isPlaceholderApiKey`: `/^<[^>]+>$/`.
fn is_placeholder_api_key(api_key: &str) -> bool {
    Regex::new(r"^<[^>]+>$")
        .map(|re| re.is_match(api_key))
        .unwrap_or(false)
}

/// Mirrors the TS `resolveApiKey`.
///
/// A trimmed `apiKey` that is empty, the `gcp-vertex-credentials` marker, or a
/// `<placeholder>` → resolves to `None` (i.e. fall back to ADC). The TS only reads
/// `options.apiKey`, but the api-key-resolution test also covers
/// `GOOGLE_CLOUD_API_KEY` as a placeholder; that env key is wired in by the model
/// registry into `options.apiKey` upstream, so a placeholder there must also fall
/// back. We mirror that by additionally checking `GOOGLE_CLOUD_API_KEY` when no
/// usable `options.apiKey` is present.
fn resolve_api_key(options: &GoogleVertexOptions) -> Option<String> {
    fn usable(candidate: Option<&str>) -> Option<String> {
        let api_key = candidate?.trim();
        if api_key.is_empty()
            || api_key == GCP_VERTEX_CREDENTIALS_MARKER
            || is_placeholder_api_key(api_key)
        {
            return None;
        }
        Some(api_key.to_string())
    }

    if let Some(key) = usable(options.base.api_key.as_deref()) {
        return Some(key);
    }
    // The model registry may have surfaced GOOGLE_CLOUD_API_KEY as the api key;
    // re-check the env directly so a placeholder there also falls back to ADC and
    // a real value still selects the api-key path.
    usable(get_provider_env_value("GOOGLE_CLOUD_API_KEY", options.base.env.as_ref()).as_deref())
}

/// Mirrors the TS `resolveProject`.
fn resolve_project(options: &GoogleVertexOptions) -> Result<String, String> {
    let project = options
        .project
        .clone()
        .filter(|p| !p.is_empty())
        .or_else(|| get_provider_env_value("GOOGLE_CLOUD_PROJECT", options.base.env.as_ref()))
        .or_else(|| get_provider_env_value("GCLOUD_PROJECT", options.base.env.as_ref()));
    match project {
        Some(p) if !p.is_empty() => Ok(p),
        _ => Err(
            "Vertex AI requires a project ID. Set GOOGLE_CLOUD_PROJECT/GCLOUD_PROJECT or pass project in options."
                .to_string(),
        ),
    }
}

/// Mirrors the TS `resolveLocation`.
fn resolve_location(options: &GoogleVertexOptions) -> Result<String, String> {
    let location = options
        .location
        .clone()
        .filter(|l| !l.is_empty())
        .or_else(|| get_provider_env_value("GOOGLE_CLOUD_LOCATION", options.base.env.as_ref()));
    match location {
        Some(l) if !l.is_empty() => Ok(l),
        _ => Err(
            "Vertex AI requires a location. Set GOOGLE_CLOUD_LOCATION or pass location in options."
                .to_string(),
        ),
    }
}

/// Resolve an ADC OAuth access token.
///
/// TODO(adc): the TS source mints an OAuth token from Application Default
/// Credentials via `google-auth-library` (service-account key file from
/// `GOOGLE_APPLICATION_CREDENTIALS`, or the GCE metadata server). There is no
/// permissible Rust dependency for that here, so for now we accept a
/// pre-acquired access token from the environment / a Bearer header. Full
/// service-account JWT minting from `GOOGLE_APPLICATION_CREDENTIALS` is unimplemented.
fn resolve_adc_access_token(options: &GoogleVertexOptions) -> Option<String> {
    let env = options.base.env.as_ref();
    if let Some(tok) = get_provider_env_value("GOOGLE_VERTEX_ACCESS_TOKEN", env) {
        if !tok.is_empty() {
            return Some(tok);
        }
    }
    if let Some(tok) = get_provider_env_value("GOOGLE_OAUTH_ACCESS_TOKEN", env) {
        if !tok.is_empty() {
            return Some(tok);
        }
    }
    // A caller may pass `Authorization: Bearer <token>` directly via headers.
    if let Some(headers) = &options.base.headers {
        for (k, v) in headers {
            if k.eq_ignore_ascii_case("authorization") {
                if let Some(tok) = v.trim().strip_prefix("Bearer ") {
                    if !tok.is_empty() {
                        return Some(tok.to_string());
                    }
                }
            }
        }
    }
    None
}

/// Resolve how this request authenticates: API key or ADC.
///
/// Mirrors the TS branch in `streamGoogleVertex`:
/// `apiKey ? createClientWithApiKey(...) : createClient(project, location, ...)`.
fn resolve_auth(_model: &Model, options: &GoogleVertexOptions) -> Result<VertexAuth, String> {
    if let Some(api_key) = resolve_api_key(options) {
        return Ok(VertexAuth::ApiKey(api_key));
    }

    // ADC path — resolve project + location first (these throw in the TS), then a token.
    let project = resolve_project(options)?;
    let location = resolve_location(options)?;
    let access_token = resolve_adc_access_token(options).ok_or_else(|| {
        // TODO(adc): mint this from GOOGLE_APPLICATION_CREDENTIALS service-account JWT.
        let _key_file =
            get_provider_env_value("GOOGLE_APPLICATION_CREDENTIALS", options.base.env.as_ref());
        "Vertex AI ADC requires an access token. Provide a real Vertex apiKey, or set \
         GOOGLE_VERTEX_ACCESS_TOKEN / pass an Authorization: Bearer header. \
         Service-account JWT minting from GOOGLE_APPLICATION_CREDENTIALS is not yet implemented."
            .to_string()
    })?;

    Ok(VertexAuth::Adc {
        project,
        location,
        access_token,
    })
}

// ---------------------------------------------------------------------------
// Base URL helpers (mirror the TS resolveCustomBaseUrl / baseUrlIncludesApiVersion)
// ---------------------------------------------------------------------------

/// Mirrors the TS `resolveCustomBaseUrl`: trim, reject empties and unexpanded
/// `{location}` placeholders (the generated Vertex base URL template).
fn resolve_custom_base_url(base_url: &str) -> Option<String> {
    let trimmed = base_url.trim();
    if trimmed.is_empty() || trimmed.contains("{location}") {
        return None;
    }
    Some(trimmed.to_string())
}

/// Mirrors the TS `baseUrlIncludesApiVersion`: does any path segment look like
/// `v1`, `v1beta`, `v2beta3`, ...?
fn base_url_includes_api_version(base_url: &str) -> bool {
    let seg_re = Regex::new(r"^v\d+(?:beta\d*)?$");
    if let Ok(url) = url::Url::parse(base_url) {
        if let Some(re) = seg_re.as_ref().ok() {
            return url.path().split('/').any(|part| re.is_match(part));
        }
    }
    // Fallback regex over the raw string.
    Regex::new(r"(?:^|/)v\d+(?:beta\d*)?(?:/|$)")
        .map(|re| re.is_match(base_url))
        .unwrap_or(false)
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
    options: &GoogleVertexOptions,
) -> Result<Value, String> {
    let mut config = serde_json::Map::new();

    // generationConfig (temperature / maxOutputTokens).
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

    // tools — Vertex `buildParams` calls `convertTools(context.tools)` with the
    // default (`useParameters = false` → `parametersJsonSchema`).
    let has_tools = !context.tools.is_empty();
    if has_tools {
        if let Some(tools) = convert_tools(&context.tools, false) {
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

/// Build the full request body (`{ contents, ...config }`).
fn build_request_body(
    model: &Model,
    context: &Context,
    options: &GoogleVertexOptions,
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

/// Compute the Vertex streaming endpoint URL.
///
/// Mirrors the SDK's Vertex endpoint construction plus the TS custom-base-url
/// handling (`resolveCustomBaseUrl` / `baseUrlIncludesApiVersion`):
/// - A usable custom `model.base_url` is used verbatim as the host/prefix. If it
///   already contains an `vN`/`vNbeta` segment, no `apiVersion` is appended
///   (matching the TS `apiVersion = ""`); otherwise `/v1` is appended.
/// - Otherwise the regional host `https://{location}-aiplatform.googleapis.com/v1`
///   is used (`global` → host without a region prefix).
fn build_url(model: &Model, auth: &VertexAuth) -> String {
    let custom = resolve_custom_base_url(&model.base_url);

    let (prefix, location_for_path, project_for_path) = match auth {
        VertexAuth::Adc {
            project, location, ..
        } => {
            let base = match &custom {
                Some(base) => {
                    let base = base.trim_end_matches('/').to_string();
                    if base_url_includes_api_version(&base) {
                        base
                    } else {
                        format!("{}/{}", base, API_VERSION)
                    }
                }
                None => {
                    let host = if location == "global" {
                        "https://aiplatform.googleapis.com".to_string()
                    } else {
                        format!("https://{}-aiplatform.googleapis.com", location)
                    };
                    format!("{}/{}", host, API_VERSION)
                }
            };
            (base, location.clone(), Some(project.clone()))
        }
        VertexAuth::ApiKey(_) => {
            // API-key path: Vertex express endpoint. Custom base URL still honored.
            let base = match &custom {
                Some(base) => {
                    let base = base.trim_end_matches('/').to_string();
                    if base_url_includes_api_version(&base) {
                        base
                    } else {
                        format!("{}/{}", base, API_VERSION)
                    }
                }
                None => format!("https://aiplatform.googleapis.com/{}", API_VERSION),
            };
            (base, String::new(), None)
        }
    };

    match project_for_path {
        Some(project) => format!(
            "{}/projects/{}/locations/{}/publishers/google/models/{}:streamGenerateContent?alt=sse",
            prefix, project, location_for_path, model.id
        ),
        None => format!(
            "{}/publishers/google/models/{}:streamGenerateContent?alt=sse",
            prefix, model.id
        ),
    }
}

// ---------------------------------------------------------------------------
// Stream driver
// ---------------------------------------------------------------------------

async fn run_stream(
    model: Model,
    context: Context,
    options: GoogleVertexOptions,
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
        _ => DoneReason::Stop,
    }
}

/// The core streaming logic. Returns `Err(message)` for any failure, which the
/// caller converts into a terminal error event.
async fn run_stream_inner(
    model: &Model,
    context: &Context,
    options: &GoogleVertexOptions,
    sender: &mut AssistantMessageEventStreamSender,
    output: &mut AssistantMessage,
) -> Result<(), String> {
    let auth = resolve_auth(model, options)?;

    let mut body = build_request_body(model, context, options)?;

    // onPayload hook: allow inspection/replacement before sending.
    if let Some(on_payload) = &options.base.on_payload {
        if let Some(next) = on_payload(body.clone(), model.clone()).await {
            body = next;
        }
    }

    let url = build_url(model, &auth);

    let client = reqwest::Client::new();
    let mut request = client.post(&url).header("content-type", "application/json");

    // Auth header / query: API key vs. ADC bearer.
    request = match &auth {
        VertexAuth::ApiKey(key) => request.header("x-goog-api-key", key),
        VertexAuth::Adc { access_token, .. } => {
            request.header("authorization", format!("Bearer {}", access_token))
        }
    };

    // Merge model.headers then options.headers (options win), mirroring buildHttpOptions.
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
    let response = send_with_abort(request, options.base.signal.clone()).await?;

    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        return Err(format!("Vertex AI error {}: {}", status.as_u16(), text));
    }

    // Emit `start`.
    sender.push(AssistantMessageEvent::Start {
        partial: output.clone(),
    });

    let mut current_block: Option<CurrentBlock> = None;

    // SSE parsing: accumulate bytes into a line buffer, split on `\n`, collect
    // `data:` payloads.
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

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Api, ModelCost, ProviderEnv};

    fn test_model(id: &str, base_url: &str) -> Model {
        Model {
            id: id.to_string(),
            name: id.to_string(),
            api: Api::GoogleVertex,
            provider: "google-vertex".to_string(),
            base_url: base_url.to_string(),
            reasoning: true,
            thinking_level_map: None,
            input: vec![],
            cost: ModelCost {
                input: 0.0,
                output: 0.0,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 1_000_000,
            max_tokens: 8192,
            headers: None,
            compat: None,
        }
    }

    fn opts_with_env(pairs: &[(&str, &str)]) -> GoogleVertexOptions {
        let mut env: ProviderEnv = ProviderEnv::new();
        for (k, v) in pairs {
            env.insert((*k).to_string(), (*v).to_string());
        }
        let mut base = StreamOptions::default();
        base.env = Some(env);
        GoogleVertexOptions {
            base,
            ..Default::default()
        }
    }

    // ----- API key / placeholder resolution -----

    #[test]
    fn placeholder_marker_falls_back_to_adc() {
        let mut options = GoogleVertexOptions::default();
        options.base.api_key = Some("<authenticated>".to_string());
        assert_eq!(resolve_api_key(&options), None);
    }

    #[test]
    fn gcp_vertex_credentials_marker_falls_back_to_adc() {
        let mut options = GoogleVertexOptions::default();
        options.base.api_key = Some(GCP_VERTEX_CREDENTIALS_MARKER.to_string());
        assert_eq!(resolve_api_key(&options), None);
    }

    #[test]
    fn empty_api_key_falls_back_to_adc() {
        let mut options = GoogleVertexOptions::default();
        options.base.api_key = Some("   ".to_string());
        assert_eq!(resolve_api_key(&options), None);
    }

    #[test]
    fn real_api_key_is_used() {
        let mut options = GoogleVertexOptions::default();
        options.base.api_key = Some("AIzaSyExampleRealisticLookingApiKey123456".to_string());
        assert_eq!(
            resolve_api_key(&options).as_deref(),
            Some("AIzaSyExampleRealisticLookingApiKey123456")
        );
    }

    #[test]
    fn is_placeholder_api_key_matches_angle_brackets() {
        assert!(is_placeholder_api_key("<authenticated>"));
        assert!(is_placeholder_api_key("<your-key-here>"));
        assert!(!is_placeholder_api_key("AIzaSyReal"));
        assert!(!is_placeholder_api_key("<>"));
    }

    // ----- project / location resolution -----

    #[test]
    fn project_from_options_wins() {
        let mut options = opts_with_env(&[("GOOGLE_CLOUD_PROJECT", "env-project")]);
        options.project = Some("opt-project".to_string());
        assert_eq!(resolve_project(&options).as_deref(), Ok("opt-project"));
    }

    #[test]
    fn project_from_google_cloud_project_env() {
        let options = opts_with_env(&[("GOOGLE_CLOUD_PROJECT", "env-project")]);
        assert_eq!(resolve_project(&options).as_deref(), Ok("env-project"));
    }

    #[test]
    fn project_from_gcloud_project_env() {
        let options = opts_with_env(&[("GCLOUD_PROJECT", "gcloud-project")]);
        assert_eq!(resolve_project(&options).as_deref(), Ok("gcloud-project"));
    }

    #[test]
    fn project_missing_errors() {
        let options = opts_with_env(&[]);
        assert!(resolve_project(&options).is_err());
    }

    #[test]
    fn location_from_options_wins() {
        let mut options = opts_with_env(&[("GOOGLE_CLOUD_LOCATION", "us-east1")]);
        options.location = Some("us-central1".to_string());
        assert_eq!(resolve_location(&options).as_deref(), Ok("us-central1"));
    }

    #[test]
    fn location_from_env() {
        let options = opts_with_env(&[("GOOGLE_CLOUD_LOCATION", "us-central1")]);
        assert_eq!(resolve_location(&options).as_deref(), Ok("us-central1"));
    }

    #[test]
    fn location_missing_errors() {
        let options = opts_with_env(&[]);
        assert!(resolve_location(&options).is_err());
    }

    // ----- endpoint resolution -----

    #[test]
    fn adc_url_regional_host() {
        let model = test_model("gemini-2.5-pro", "");
        let auth = VertexAuth::Adc {
            project: "test-project".to_string(),
            location: "us-central1".to_string(),
            access_token: "tok".to_string(),
        };
        let url = build_url(&model, &auth);
        assert_eq!(
            url,
            "https://us-central1-aiplatform.googleapis.com/v1/projects/test-project/locations/us-central1/publishers/google/models/gemini-2.5-pro:streamGenerateContent?alt=sse"
        );
    }

    #[test]
    fn adc_url_global_host() {
        let model = test_model("gemini-2.5-pro", "");
        let auth = VertexAuth::Adc {
            project: "test-project".to_string(),
            location: "global".to_string(),
            access_token: "tok".to_string(),
        };
        let url = build_url(&model, &auth);
        assert_eq!(
            url,
            "https://aiplatform.googleapis.com/v1/projects/test-project/locations/global/publishers/google/models/gemini-2.5-pro:streamGenerateContent?alt=sse"
        );
    }

    #[test]
    fn adc_url_custom_base_without_version_appends_v1() {
        let model = test_model("gemini-2.5-pro", "https://proxy.example.com");
        let auth = VertexAuth::Adc {
            project: "test-project".to_string(),
            location: "us-central1".to_string(),
            access_token: "tok".to_string(),
        };
        let url = build_url(&model, &auth);
        assert_eq!(
            url,
            "https://proxy.example.com/v1/projects/test-project/locations/us-central1/publishers/google/models/gemini-2.5-pro:streamGenerateContent?alt=sse"
        );
    }

    #[test]
    fn adc_url_custom_base_with_version_is_verbatim() {
        let model = test_model(
            "gemini-2.5-pro",
            "https://proxy.example.com/v1/projects/test-project/locations/global",
        );
        let auth = VertexAuth::Adc {
            project: "test-project".to_string(),
            location: "us-central1".to_string(),
            access_token: "tok".to_string(),
        };
        let url = build_url(&model, &auth);
        // base already includes the `/v1` segment → no extra version appended.
        assert!(url.starts_with(
            "https://proxy.example.com/v1/projects/test-project/locations/global/projects/test-project/locations/us-central1/publishers/google/models/gemini-2.5-pro:streamGenerateContent"
        ));
    }

    #[test]
    fn placeholder_location_base_url_is_ignored() {
        // The generated Vertex template contains `{location}` and must be dropped.
        assert_eq!(
            resolve_custom_base_url("https://{location}-aiplatform.googleapis.com"),
            None
        );
        assert_eq!(
            resolve_custom_base_url("  https://proxy.example.com  ").as_deref(),
            Some("https://proxy.example.com")
        );
    }

    #[test]
    fn base_url_includes_api_version_detection() {
        assert!(base_url_includes_api_version(
            "https://proxy.example.com/v1/projects/p/locations/global"
        ));
        assert!(base_url_includes_api_version("https://x.com/v1beta/models"));
        assert!(!base_url_includes_api_version("https://proxy.example.com"));
        assert!(!base_url_includes_api_version(
            "https://proxy.example.com/projects/p"
        ));
    }

    // ----- thinking budget / level -----

    #[test]
    fn gemini3_pro_thinking_level() {
        let model = test_model("gemini-3-pro-preview", "");
        assert!(is_gemini3_pro_model(&model));
        assert_eq!(
            get_gemini3_thinking_level(Effort::Minimal, &model),
            GoogleThinkingLevel::Low
        );
        assert_eq!(
            get_gemini3_thinking_level(Effort::High, &model),
            GoogleThinkingLevel::High
        );
    }

    #[test]
    fn google_budget_2_5_pro() {
        let model = test_model("gemini-2.5-pro", "");
        assert_eq!(get_google_budget(&model, Effort::Minimal, None), 128);
        assert_eq!(get_google_budget(&model, Effort::High, None), 32768);
    }

    #[test]
    fn google_budget_2_5_flash() {
        let model = test_model("gemini-2.5-flash", "");
        assert_eq!(get_google_budget(&model, Effort::High, None), 24576);
    }

    #[test]
    fn google_budget_default_dynamic() {
        let model = test_model("gemini-1.5-pro", "");
        assert_eq!(get_google_budget(&model, Effort::High, None), -1);
    }
}
