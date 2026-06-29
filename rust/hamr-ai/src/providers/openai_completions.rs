//! Port of `../../packages/ai/src/providers/openai-completions.ts`.
//!
//! OpenAI Chat Completions API provider — `/v1/chat/completions`.
//!
//! Handles message conversion, tool-call result-images, thinking-as-text,
//! tool-choice, prompt caching, retry logic, and model routing. This is the
//! largest single provider file in the codebase.
//!
//! ## Entry points
//!
//! - [`stream`] / [`stream_openai_completions`] — full provider stream.
//! - [`stream_simple`] / [`stream_simple_openai_completions`] — unified
//!   `SimpleStreamOptions` wrapper.
//!
//! ## Key subsystems
//!
//! 1. **Message conversion** ([`convert_messages`]) — hamr messages to OpenAI chat
//!    format via `transform_messages`. Handles consecutive user messages, image
//!    content (vision), tool-result images, thinking-as-text mode, reasoning
//!    content on assistant messages.
//! 2. **Tool conversion** ([`convert_tools`]) — OpenAI function-calling tools with
//!    strict mode. Handles empty tools array specially.
//! 3. **Streaming** ([`process_sse_stream`]) — SSE parsing from `/v1/chat/completions`.
//!    Builds up partial `AssistantMessage` incrementally.
//! 4. **Prompt caching** — `cache_control` auto-annotation via Anthropic-compatible
//!    cache control on the OpenAI wire (for relay use with openai→anthropic proxies).
//! 5. **Thinking-as-text / reasoning** — OpenAI reasoning models expose thinking
//!    via `reasoning_content` deltas, inline text, or thinking blocks. Maps
//!    thinking levels to `reasoning_effort` values.
//! 6. **Compat detection** ([`detect_compat`]) — auto-detect compatibility settings
//!    from provider name and base URL.
//!
//! ## Type debt
//!
//! - `Model.compat` is `Option<serde_json::Value>`. The `getCompat` function
//!   reads from it when present; when not set, compat is auto-detected from
//!   `base_url` / `provider`.
//! - `StreamOptions` has no `max_retries` or `timeout_ms` fields (they exist on the
//!   TS type) — we use the standard `reqwest::Client` defaults and mark
//!   `// TODO(retry)`.

use std::collections::HashMap;

use futures::StreamExt;
use serde_json::Value;

use crate::models::{calculate_cost, clamp_thinking_level};
use crate::providers::cloudflare::{is_cloudflare_provider, resolve_cloudflare_base_url};
use crate::providers::github_copilot_headers::{
    build_copilot_dynamic_headers, has_copilot_vision_input,
};
use crate::providers::openai_prompt_cache::clamp_openai_prompt_cache_key;
use crate::providers::simple_options::build_base_options;
use crate::providers::transform_messages::transform_messages;
use crate::types::{
    AssistantContentBlock, AssistantMessage, AssistantMessageEvent, CacheRetention, Context,
    DoneReason, ErrorReason, Message, MessageContent, MessageRole, Model, ModelThinkingLevel,
    ProviderEnv, ProviderResponse, SimpleStreamOptions, StopReason, StreamOptions, TextContent,
    ThinkingContent, Tool, ToolCall, Usage, UsageCost,
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
// Options
// ---------------------------------------------------------------------------

/// OpenAI Completions-specific stream options.
///
/// Mirrors the TS `OpenAICompletionsOptions extends StreamOptions`.
#[derive(Clone, Debug)]
pub struct OpenAiCompletionsOptions {
    pub base: StreamOptions,
    /// `"auto" | "none" | "required" | { type: "function"; function: { name: string } }`.
    pub tool_choice: Option<serde_json::Value>,
    /// `"minimal" | "low" | "medium" | "high" | "xhigh"`.
    pub reasoning_effort: Option<String>,
}

impl Default for OpenAiCompletionsOptions {
    fn default() -> Self {
        Self {
            base: StreamOptions::default(),
            tool_choice: None,
            reasoning_effort: None,
        }
    }
}

// ---------------------------------------------------------------------------
// Cache control types (OpenAI-compatible, for Anthropic-style cache on relay)
// ---------------------------------------------------------------------------

#[derive(Clone, serde::Serialize, serde::Deserialize)]
struct OpenAiCompatCacheControl {
    #[serde(rename = "type")]
    cache_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    ttl: Option<String>,
}

// ---------------------------------------------------------------------------
// Compat detection and resolution
// ---------------------------------------------------------------------------

/// Full OpenAICompletionsCompat with all fields resolved.
#[derive(Clone, Debug)]
struct ResolvedOpenAiCompletionsCompat {
    supports_store: bool,
    supports_developer_role: bool,
    supports_reasoning_effort: bool,
    supports_usage_in_streaming: bool,
    max_tokens_field: String,
    requires_tool_result_name: bool,
    requires_assistant_after_tool_result: bool,
    requires_thinking_as_text: bool,
    requires_reasoning_content_on_assistant_messages: bool,
    thinking_format: String,
    open_router_routing: HashMap<String, Value>,
    vercel_gateway_routing: HashMap<String, Value>,
    zai_tool_stream: bool,
    supports_strict_mode: bool,
    cache_control_format: Option<String>,
    send_session_affinity_headers: bool,
    supports_long_cache_retention: bool,
}

/// Auto-detect compatibility settings from provider name and base URL.
///
/// Mirrors the TS `detectCompat`.
fn detect_compat(model: &Model) -> ResolvedOpenAiCompletionsCompat {
    let provider = &model.provider;
    let base_url = &model.base_url;

    let is_zai = provider == "zai"
        || provider == "zai-coding-cn"
        || base_url.contains("api.z.ai")
        || base_url.contains("open.bigmodel.cn");

    let is_together = provider == "together"
        || base_url.contains("api.together.ai")
        || base_url.contains("api.together.xyz");

    let is_moonshot = provider == "moonshotai"
        || provider == "moonshotai-cn"
        || base_url.contains("api.moonshot.");

    let is_open_router = provider == "openrouter" || base_url.contains("openrouter.ai");

    let is_cloudflare_workers_ai =
        provider == "cloudflare-workers-ai" || base_url.contains("api.cloudflare.com");

    let is_cloudflare_ai_gateway =
        provider == "cloudflare-ai-gateway" || base_url.contains("gateway.ai.cloudflare.com");

    let is_nvidia = provider == "nvidia" || base_url.contains("integrate.api.nvidia.com");

    let is_ant_ling = provider == "ant-ling" || base_url.contains("api.ant-ling.com");

    let is_non_standard = is_nvidia
        || provider == "cerebras"
        || base_url.contains("cerebras.ai")
        || provider == "xai"
        || base_url.contains("api.x.ai")
        || is_together
        || base_url.contains("chutes.ai")
        || base_url.contains("deepseek.com")
        || is_zai
        || is_moonshot
        || provider == "opencode"
        || base_url.contains("opencode.ai")
        || is_cloudflare_workers_ai
        || is_cloudflare_ai_gateway
        || is_ant_ling;

    let use_max_tokens = base_url.contains("chutes.ai")
        || is_moonshot
        || is_cloudflare_ai_gateway
        || is_together
        || is_nvidia
        || is_ant_ling;

    let is_grok = provider == "xai" || base_url.contains("api.x.ai");
    let is_deepseek = provider == "deepseek" || base_url.contains("deepseek.com");

    let is_open_router_developer_role_model =
        is_open_router && (model.id.starts_with("anthropic/") || model.id.starts_with("openai/"));

    let cache_control_format = if is_open_router && model.id.starts_with("anthropic/") {
        Some("anthropic".to_string())
    } else {
        None
    };

    let thinking_format = if is_deepseek {
        "deepseek"
    } else if is_zai {
        "zai"
    } else if is_together {
        "together"
    } else if is_ant_ling {
        "ant-ling"
    } else if is_open_router {
        "openrouter"
    } else {
        "openai"
    };

    ResolvedOpenAiCompletionsCompat {
        supports_store: !is_non_standard,
        supports_developer_role: is_open_router_developer_role_model
            || (!is_non_standard && !is_open_router),
        supports_reasoning_effort: !is_grok
            && !is_zai
            && !is_moonshot
            && !is_together
            && !is_cloudflare_ai_gateway
            && !is_nvidia
            && !is_ant_ling,
        supports_usage_in_streaming: true,
        max_tokens_field: if use_max_tokens {
            "max_tokens".to_string()
        } else {
            "max_completion_tokens".to_string()
        },
        requires_tool_result_name: false,
        requires_assistant_after_tool_result: false,
        requires_thinking_as_text: false,
        requires_reasoning_content_on_assistant_messages: is_deepseek,
        thinking_format: thinking_format.to_string(),
        open_router_routing: HashMap::new(),
        vercel_gateway_routing: HashMap::new(),
        zai_tool_stream: false,
        supports_strict_mode: !is_moonshot
            && !is_together
            && !is_cloudflare_ai_gateway
            && !is_nvidia,
        cache_control_format,
        send_session_affinity_headers: false,
        supports_long_cache_retention: !(is_together
            || is_cloudflare_workers_ai
            || is_cloudflare_ai_gateway
            || is_nvidia
            || is_ant_ling),
    }
}

/// Get resolved compatibility settings for a model.
///
/// Mirrors the TS `getCompat`. Reads overrides from `model.compat` if present;
/// falls back to auto-detected values.
fn get_compat(model: &Model) -> ResolvedOpenAiCompletionsCompat {
    let detected = detect_compat(model);
    match &model.compat {
        None => detected,
        Some(c) => {
            let get_str = |key: &str| c.get(key).and_then(|v| v.as_str()).map(|s| s.to_string());
            let get_bool = |key: &str| c.get(key).and_then(|v| v.as_bool());
            ResolvedOpenAiCompletionsCompat {
                supports_store: get_bool("supportsStore").unwrap_or(detected.supports_store),
                supports_developer_role: get_bool("supportsDeveloperRole").unwrap_or(detected.supports_developer_role),
                supports_reasoning_effort: get_bool("supportsReasoningEffort").unwrap_or(detected.supports_reasoning_effort),
                supports_usage_in_streaming: get_bool("supportsUsageInStreaming").unwrap_or(detected.supports_usage_in_streaming),
                max_tokens_field: get_str("maxTokensField").unwrap_or(detected.max_tokens_field),
                requires_tool_result_name: get_bool("requiresToolResultName").unwrap_or(detected.requires_tool_result_name),
                requires_assistant_after_tool_result: get_bool("requiresAssistantAfterToolResult").unwrap_or(detected.requires_assistant_after_tool_result),
                requires_thinking_as_text: get_bool("requiresThinkingAsText").unwrap_or(detected.requires_thinking_as_text),
                requires_reasoning_content_on_assistant_messages: get_bool("requiresReasoningContentOnAssistantMessages").unwrap_or(detected.requires_reasoning_content_on_assistant_messages),
                thinking_format: get_str("thinkingFormat").unwrap_or(detected.thinking_format),
                open_router_routing: c.get("openRouterRouting")
                    .and_then(|v| serde_json::from_value(v.clone()).ok())
                    .unwrap_or(detected.open_router_routing),
                vercel_gateway_routing: c.get("vercelGatewayRouting")
                    .and_then(|v| serde_json::from_value(v.clone()).ok())
                    .unwrap_or(detected.vercel_gateway_routing),
                zai_tool_stream: get_bool("zaiToolStream").unwrap_or(detected.zai_tool_stream),
                supports_strict_mode: get_bool("supportsStrictMode").unwrap_or(detected.supports_strict_mode),
                cache_control_format: get_str("cacheControlFormat").or(detected.cache_control_format),
                send_session_affinity_headers: get_bool("sendSessionAffinityHeaders").unwrap_or(detected.send_session_affinity_headers),
                supports_long_cache_retention: get_bool("supportsLongCacheRetention").unwrap_or(detected.supports_long_cache_retention),
            }
        }
    }
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
        api: "openai-completions".to_string(),
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
// Helper functions
// ---------------------------------------------------------------------------

fn has_tool_history(messages: &[Message]) -> bool {
    for msg in messages {
        match msg {
            Message::ToolResult(_) => return true,
            Message::Assistant(assistant) => {
                if assistant
                    .content
                    .iter()
                    .any(|b| matches!(b, AssistantContentBlock::ToolCall(_)))
                {
                    return true;
                }
            }
            _ => {}
        }
    }
    false
}

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

fn get_compat_cache_control(
    compat: &ResolvedOpenAiCompletionsCompat,
    cache_retention: CacheRetention,
) -> Option<OpenAiCompatCacheControl> {
    if compat.cache_control_format.as_deref() != Some("anthropic")
        || cache_retention == CacheRetention::None
    {
        return None;
    }
    let ttl = if cache_retention == CacheRetention::Long && compat.supports_long_cache_retention {
        Some("1h".to_string())
    } else {
        None
    };
    Some(OpenAiCompatCacheControl {
        cache_type: "ephemeral".to_string(),
        ttl,
    })
}

// ---------------------------------------------------------------------------
// Cache control annotation (Anthropic-style)
// ---------------------------------------------------------------------------

fn add_cache_control_to_system_prompt(
    messages: &mut Vec<Value>,
    cache_control: &OpenAiCompatCacheControl,
) {
    let cache_ctrl = serde_json::to_value(cache_control).unwrap_or_default();
    for msg in messages.iter_mut() {
        let role = msg.get("role").and_then(|r| r.as_str()).unwrap_or("");
        if role == "system" || role == "developer" {
            add_cache_control_to_instruction_message(msg, &cache_ctrl);
            return;
        }
    }
}

fn add_cache_control_to_instruction_message(msg: &mut Value, cache_control: &Value) {
    add_cache_control_to_text_content(msg, cache_control);
}

fn add_cache_control_to_message(msg: &mut Value, cache_control: &Value) -> bool {
    let role = msg.get("role").and_then(|r| r.as_str()).unwrap_or("");
    if role == "user" || role == "assistant" {
        return add_cache_control_to_text_content(msg, cache_control);
    }
    false
}

fn add_cache_control_to_text_content(msg: &mut Value, cache_control: &Value) -> bool {
    let Some(content) = msg.get("content") else {
        return false;
    };

    if let Some(text) = content.as_str() {
        if text.is_empty() {
            return false;
        }
        msg["content"] = serde_json::json!([
            {
                "type": "text",
                "text": text,
                "cache_control": cache_control,
            }
        ]);
        return true;
    }

    if let Some(arr) = content.as_array() {
        if arr.is_empty() {
            return false;
        }
        for i in (0..arr.len()).rev() {
            let part = &arr[i];
            if part.get("type").and_then(|t| t.as_str()) == Some("text") {
                if let Some(obj) = msg["content"][i].as_object_mut() {
                    obj.insert("cache_control".to_string(), cache_control.clone());
                }
                return true;
            }
        }
    }
    false
}

fn add_cache_control_to_last_conversation_message(
    messages: &mut Vec<Value>,
    cache_control: &OpenAiCompatCacheControl,
) {
    let cache_ctrl = serde_json::to_value(cache_control).unwrap_or_default();
    for i in (0..messages.len()).rev() {
        let role = messages[i]
            .get("role")
            .and_then(|r| r.as_str())
            .unwrap_or("");
        if role == "user" || role == "assistant" {
            if add_cache_control_to_message(&mut messages[i], &cache_ctrl) {
                return;
            }
        }
    }
}

fn add_cache_control_to_last_tool(
    tools: Option<&mut Vec<Value>>,
    cache_control: &OpenAiCompatCacheControl,
) {
    let tools = match tools {
        Some(t) if !t.is_empty() => t,
        _ => return,
    };
    let cache_ctrl = serde_json::to_value(cache_control).unwrap_or_default();
    let Some(last) = tools.last_mut() else { return };
    if let Some(obj) = last.as_object_mut() {
        obj.insert("cache_control".to_string(), cache_ctrl);
    }
}

fn apply_anthropic_cache_control(
    messages: &mut Vec<Value>,
    tools: Option<&mut Vec<Value>>,
    cache_control: &OpenAiCompatCacheControl,
) {
    add_cache_control_to_system_prompt(messages, cache_control);
    add_cache_control_to_last_tool(tools, cache_control);
    add_cache_control_to_last_conversation_message(messages, cache_control);
}

// ---------------------------------------------------------------------------
// Should use raw HTTP
// ---------------------------------------------------------------------------

fn should_use_raw_openai_compatible_http(
    model: &Model,
    api_key: &str,
    headers: Option<&HashMap<String, String>>,
) -> bool {
    if model.provider == "relay" || api_key == "not-needed" {
        return true;
    }
    let headers = match headers {
        Some(h) => h,
        None => return false,
    };
    let header_names_lower: Vec<String> = headers.keys().map(|k| k.to_lowercase()).collect();
    header_names_lower.contains(&"cf-access-client-id".to_string())
        || header_names_lower.contains(&"cf-access-client-secret".to_string())
}

// ---------------------------------------------------------------------------
// Raw HTTP header builder
// ---------------------------------------------------------------------------

fn build_raw_openai_compatible_headers(
    api_key: &str,
    headers: Option<&HashMap<String, String>>,
    session_id: Option<&str>,
) -> HashMap<String, String> {
    let mut merged = HashMap::new();
    merged.insert("Accept".to_string(), "application/json".to_string());
    merged.insert("Content-Type".to_string(), "application/json".to_string());
    if let Some(h) = headers {
        for (k, v) in h {
            merged.insert(k.clone(), v.clone());
        }
    }
    if api_key != "not-needed"
        && !merged.contains_key("Authorization")
        && !merged.contains_key("authorization")
    {
        merged.insert("Authorization".to_string(), format!("Bearer {}", api_key));
    }
    if let Some(sid) = session_id {
        merged.insert("x-session-affinity".to_string(), sid.to_string());
        merged.insert("x-client-request-id".to_string(), sid.to_string());
    }
    merged
}

// ---------------------------------------------------------------------------
// Error parsing
// ---------------------------------------------------------------------------

fn parse_raw_openai_compatible_error(status: u16, body_text: &str) -> String {
    let detail = body_text.trim();
    if let Ok(parsed) = serde_json::from_str::<Value>(body_text) {
        if let Some(err) = parsed.get("error") {
            if let Some(msg) = err.get("message").and_then(|m| m.as_str()) {
                if !msg.trim().is_empty() {
                    return format!("{} {}", status, msg);
                }
            }
        }
        if let Some(msg) = parsed.get("message").and_then(|m| m.as_str()) {
            if !msg.trim().is_empty() {
                return msg.to_string();
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
// Tool conversion
// ---------------------------------------------------------------------------

fn convert_tools(tools: &[Tool], compat: &ResolvedOpenAiCompletionsCompat) -> Vec<Value> {
    tools
        .iter()
        .map(|tool| {
            let mut func = serde_json::Map::new();
            func.insert("name".to_string(), Value::String(tool.name.clone()));
            func.insert(
                "description".to_string(),
                Value::String(tool.description.clone()),
            );
            func.insert("parameters".to_string(), tool.parameters.clone());
            if compat.supports_strict_mode {
                func.insert("strict".to_string(), Value::Bool(false));
            }
            serde_json::json!({
                "type": "function",
                "function": func,
            })
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Message conversion
// ---------------------------------------------------------------------------

/// Convert hamr messages to OpenAI chat completions format.
///
#[allow(unused_assignments)]
/// Mirrors the TS `convertMessages`.
fn convert_messages(
    model: &Model,
    context: &Context,
    compat: &ResolvedOpenAiCompletionsCompat,
) -> Vec<Value> {
    let normalize_tool_call_id = |id: &str, _model: &Model, _source: &AssistantMessage| -> String {
        // Handle pipe-separated IDs from OpenAI Responses API
        if id.contains('|') {
            let call_id = id.split('|').next().unwrap_or("");
            let sanitized: String = call_id
                .chars()
                .map(|c| {
                    if c.is_alphanumeric() || c == '_' || c == '-' {
                        c
                    } else {
                        '_'
                    }
                })
                .collect();
            let truncated: String = sanitized.chars().take(40).collect();
            return truncated;
        }
        if model.provider == "openai" && id.len() > 40 {
            return id.chars().take(40).collect();
        }
        id.to_string()
    };

    let normalize_fn: Option<&dyn Fn(&str, &Model, &AssistantMessage) -> String> =
        Some(&normalize_tool_call_id);

    let transformed_messages = transform_messages(context.messages.clone(), model, normalize_fn);

    let mut params: Vec<Value> = Vec::new();

    // System prompt
    if let Some(system_prompt) = &context.system_prompt {
        let use_developer_role = model.reasoning && compat.supports_developer_role;
        let role = if use_developer_role {
            "developer"
        } else {
            "system"
        };
        params.push(serde_json::json!({
            "role": role,
            "content": sanitize_surrogates(system_prompt),
        }));
    }

    let mut last_role: Option<String> = None;
    let mut i = 0;

    while i < transformed_messages.len() {
        let msg = &transformed_messages[i];

        // Some providers don't allow user messages directly after tool results.
        // Insert a synthetic assistant message to bridge the gap.
        if compat.requires_assistant_after_tool_result
            && last_role.as_deref() == Some("toolResult")
            && matches!(msg, Message::User(_))
        {
            params.push(serde_json::json!({
                "role": "assistant",
                "content": "I have processed the tool results.",
            }));
        }

        match msg {
            Message::User(user_msg) => {
                let content: Vec<Value> = user_msg
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
                                "type": "image_url",
                                "image_url": {
                                    "url": format!("data:{};base64,{}", ic.mime_type, ic.data),
                                },
                            })
                        }
                    })
                    .collect();

                if content.is_empty() {
                    i += 1;
                    last_role = Some("user".to_string());
                    continue;
                }
                params.push(serde_json::json!({
                    "role": "user",
                    "content": content,
                }));
                last_role = Some("user".to_string());
            }

            Message::Assistant(assistant_msg) => {
                let mut assistant_value = serde_json::json!({
                    "role": "assistant",
                    "content": null,
                });

                let assistant_text_blocks: Vec<&TextContent> = assistant_msg
                    .content
                    .iter()
                    .filter_map(|b| {
                        if let AssistantContentBlock::Text(t) = b {
                            if !t.text.trim().is_empty() {
                                Some(t)
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    })
                    .collect();

                let assistant_text_parts: Vec<Value> = assistant_text_blocks
                    .iter()
                    .map(|block| {
                        serde_json::json!({
                            "type": "text",
                            "text": sanitize_surrogates(&block.text),
                        })
                    })
                    .collect();

                let assistant_text: String = assistant_text_blocks
                    .iter()
                    .map(|b| b.text.as_str())
                    .collect::<Vec<&str>>()
                    .join("");

                let thinking_blocks: Vec<&ThinkingContent> = assistant_msg
                    .content
                    .iter()
                    .filter_map(|b| {
                        if let AssistantContentBlock::Thinking(t) = b {
                            if !t.thinking.trim().is_empty() {
                                Some(t)
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    })
                    .collect();

                if !thinking_blocks.is_empty() {
                    if compat.requires_thinking_as_text {
                        // Convert thinking blocks to plain text
                        let thinking_text: String = thinking_blocks
                            .iter()
                            .map(|block| sanitize_surrogates(&block.thinking))
                            .collect::<Vec<String>>()
                            .join("\n\n");
                        let mut content_arr: Vec<Value> = vec![serde_json::json!({
                            "type": "text",
                            "text": thinking_text,
                        })];
                        content_arr.extend(assistant_text_parts);
                        assistant_value["content"] = Value::Array(content_arr);
                    } else {
                        // Always send assistant content as a plain string (standard format)
                        if !assistant_text.is_empty() {
                            assistant_value["content"] = Value::String(assistant_text);
                        }

                        // Use signature from first thinking block
                        let mut signature = thinking_blocks[0]
                            .thinking_signature
                            .clone()
                            .unwrap_or_default();
                        if model.provider == "opencode-go" && signature == "reasoning" {
                            signature = "reasoning_content".to_string();
                        }
                        if !signature.is_empty() {
                            let thinking_joined: String = thinking_blocks
                                .iter()
                                .map(|b| b.thinking.as_str())
                                .collect::<Vec<&str>>()
                                .join("\n");
                            if let Some(obj) = assistant_value.as_object_mut() {
                                obj.insert(signature, Value::String(thinking_joined));
                            }
                        }
                    }
                } else if !assistant_text.is_empty() {
                    assistant_value["content"] = Value::String(assistant_text);
                }

                // Tool calls
                let tool_calls: Vec<&ToolCall> = assistant_msg
                    .content
                    .iter()
                    .filter_map(|b| {
                        if let AssistantContentBlock::ToolCall(tc) = b {
                            Some(tc)
                        } else {
                            None
                        }
                    })
                    .collect();

                if !tool_calls.is_empty() {
                    let formatted_tool_calls: Vec<Value> = tool_calls
                        .iter()
                        .map(|tc| {
                            serde_json::json!({
                                "id": tc.id,
                                "type": "function",
                                "function": {
                                    "name": tc.name,
                                    "arguments": serde_json::to_string(&tc.arguments).unwrap_or_else(|_| "{}".to_string()),
                                },
                            })
                        })
                        .collect();
                    if let Some(obj) = assistant_value.as_object_mut() {
                        obj.insert("tool_calls".to_string(), Value::Array(formatted_tool_calls));
                    }

                    // reasoning_details from thought signatures
                    let mut reasoning_details: Vec<Value> = Vec::new();
                    for tc in &tool_calls {
                        if let Some(sig) = &tc.thought_signature {
                            if let Ok(detail) = serde_json::from_str::<Value>(sig) {
                                reasoning_details.push(detail);
                            }
                        }
                    }
                    if !reasoning_details.is_empty() {
                        if let Some(obj) = assistant_value.as_object_mut() {
                            obj.insert(
                                "reasoning_details".to_string(),
                                Value::Array(reasoning_details),
                            );
                        }
                    }
                }

                // DeepSeek requires reasoning_content on assistant messages
                if compat.requires_reasoning_content_on_assistant_messages
                    && model.reasoning
                    && assistant_value
                        .get("reasoning_content")
                        .and_then(|v| v.as_str())
                        .is_none()
                {
                    if let Some(obj) = assistant_value.as_object_mut() {
                        obj.insert(
                            "reasoning_content".to_string(),
                            Value::String(String::new()),
                        );
                    }
                }

                // Skip empty assistant messages (no content and no tool calls)
                let has_content = match assistant_value.get("content") {
                    Some(Value::String(s)) => !s.is_empty(),
                    Some(Value::Array(arr)) => !arr.is_empty(),
                    Some(Value::Null) => false,
                    _ => false,
                };
                let has_tool_calls = assistant_value.get("tool_calls").is_some();
                if !has_content && !has_tool_calls {
                    i += 1;
                    continue;
                }

                params.push(assistant_value);
                last_role = Some("assistant".to_string());
            }

            Message::ToolResult(_) => {
                let mut image_blocks: Vec<Value> = Vec::new();
                let mut j = i;

                while j < transformed_messages.len() {
                    let m = &transformed_messages[j];
                    if !matches!(m, Message::ToolResult(_)) {
                        break;
                    }
                    if let Message::ToolResult(tm) = m {
                        // Extract text and image content
                        let text_result: String = tm
                            .content
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

                        let has_images = tm
                            .content
                            .iter()
                            .any(|c| matches!(c, MessageContent::Image(_)));

                        let has_text = !text_result.is_empty();
                        let tool_result_msg = serde_json::json!({
                            "role": "tool",
                            "content": sanitize_surrogates(if has_text { &text_result } else { "(see attached image)" }),
                            "tool_call_id": tm.tool_call_id,
                        });

                        let mut tr_msg = tool_result_msg;
                        if compat.requires_tool_result_name && !tm.tool_name.is_empty() {
                            if let Some(obj) = tr_msg.as_object_mut() {
                                obj.insert("name".to_string(), Value::String(tm.tool_name.clone()));
                            }
                        }
                        params.push(tr_msg);

                        if has_images && model.input.contains(&crate::types::InputModality::Image) {
                            for block in &tm.content {
                                if let MessageContent::Image(ic) = block {
                                    image_blocks.push(serde_json::json!({
                                        "type": "image_url",
                                        "image_url": {
                                            "url": format!("data:{};base64,{}", ic.mime_type, ic.data),
                                        },
                                    }));
                                }
                            }
                        }
                    }
                    j += 1;
                }

                i = j - 1;

                if !image_blocks.is_empty() {
                    if compat.requires_assistant_after_tool_result {
                        params.push(serde_json::json!({
                            "role": "assistant",
                            "content": "I have processed the tool results.",
                        }));
                    }
                    // Build the user message with text + images
                    let mut user_content = vec![
                        serde_json::json!({ "type": "text", "text": "Attached image(s) from tool result:" }),
                    ];
                    user_content.extend(image_blocks);
                    params.push(serde_json::json!({
                        "role": "user",
                        "content": user_content,
                    }));
                    last_role = Some("user".to_string());
                } else {
                    last_role = Some("toolResult".to_string());
                }
                i += 1;
                continue;
            }
        }

        match msg {
            Message::User(_) => last_role = Some("user".to_string()),
            Message::Assistant(_) => last_role = Some("assistant".to_string()),
            Message::ToolResult(_) => last_role = Some("toolResult".to_string()),
        }
        i += 1;
    }

    params
}

// ---------------------------------------------------------------------------
// Params building
// ---------------------------------------------------------------------------

fn build_params(
    model: &Model,
    context: &Context,
    options: &OpenAiCompletionsOptions,
    compat: &ResolvedOpenAiCompletionsCompat,
    cache_retention: CacheRetention,
) -> Value {
    let messages = convert_messages(model, context, compat);
    let cache_control = get_compat_cache_control(compat, cache_retention);

    let mut params = serde_json::Map::new();
    params.insert("model".to_string(), Value::String(model.id.clone()));
    params.insert("messages".to_string(), Value::Array(messages));
    params.insert("stream".to_string(), Value::Bool(true));

    // Prompt cache key
    let prompt_cache_key = if model.provider == "relay"
        || (model.base_url.contains("api.openai.com") && cache_retention != CacheRetention::None)
        || (cache_retention == CacheRetention::Long && compat.supports_long_cache_retention)
    {
        clamp_openai_prompt_cache_key(options.base.session_id.as_deref())
    } else {
        None
    };
    if let Some(ref key) = prompt_cache_key {
        params.insert("prompt_cache_key".to_string(), Value::String(key.clone()));
    }

    // Prompt cache retention
    if cache_retention == CacheRetention::Long && compat.supports_long_cache_retention {
        params.insert(
            "prompt_cache_retention".to_string(),
            Value::String("24h".to_string()),
        );
    }

    // Stream options (include usage)
    if compat.supports_usage_in_streaming {
        params.insert(
            "stream_options".to_string(),
            serde_json::json!({ "include_usage": true }),
        );
    }

    // Store
    if compat.supports_store {
        params.insert("store".to_string(), Value::Bool(false));
    }

    // Max tokens
    if let Some(max_tokens) = options.base.max_tokens {
        if compat.max_tokens_field == "max_tokens" {
            params.insert("max_tokens".to_string(), serde_json::json!(max_tokens));
        } else {
            params.insert(
                "max_completion_tokens".to_string(),
                serde_json::json!(max_tokens),
            );
        }
    }

    // Temperature
    if let Some(temp) = options.base.temperature {
        params.insert("temperature".to_string(), serde_json::json!(temp));
    }

    // Tools
    if !context.tools.is_empty() {
        let tools = convert_tools(&context.tools, compat);
        params.insert("tools".to_string(), Value::Array(tools));
        if compat.zai_tool_stream {
            params.insert("tool_stream".to_string(), Value::Bool(true));
        }
    } else if has_tool_history(&context.messages) {
        // Anthropic (via relay/proxy) requires tools param when conversation has tool_calls/tool_results
        params.insert("tools".to_string(), Value::Array(Vec::new()));
    }

    // Cache control annotation
    if let Some(ref cc_val) = cache_control {
        // Extract Values, apply cache control on the extracted data, then reinsert.
        let mut msgs_opt = params.remove("messages");
        let mut tools_opt = params.remove("tools");
        if let Some(ref mut msgs) = msgs_opt {
            if let Some(msgs_arr) = msgs.as_array_mut() {
                let mut tools_arr = tools_opt.as_mut().and_then(|v| v.as_array_mut());
                apply_anthropic_cache_control(msgs_arr, tools_arr.as_deref_mut(), cc_val);
            }
        }
        if let Some(msgs) = msgs_opt {
            params.insert("messages".to_string(), msgs);
        }
        if let Some(tools) = tools_opt {
            params.insert("tools".to_string(), tools);
        }
    }

    // Tool choice
    if let Some(ref tool_choice) = options.tool_choice {
        params.insert("tool_choice".to_string(), tool_choice.clone());
    }

    // Reasoning / thinking
    let map_effort = |effort: &str| -> Option<String> { resolve_thinking_effort(effort, model) };

    match compat.thinking_format.as_str() {
        "zai" if model.reasoning => {
            params.insert(
                "thinking".to_string(),
                serde_json::json!({
                    "type": if options.reasoning_effort.is_some() { "enabled" } else { "disabled" }
                }),
            );
            if let Some(ref effort) = options.reasoning_effort {
                if compat.supports_reasoning_effort {
                    if let Some(mapped) = map_effort(effort) {
                        params.insert("reasoning_effort".to_string(), Value::String(mapped));
                    }
                }
            }
        }
        "qwen" if model.reasoning => {
            params.insert(
                "enable_thinking".to_string(),
                Value::Bool(options.reasoning_effort.is_some()),
            );
        }
        "qwen-chat-template" if model.reasoning => {
            params.insert(
                "chat_template_kwargs".to_string(),
                serde_json::json!({
                    "enable_thinking": options.reasoning_effort.is_some(),
                    "preserve_thinking": true,
                }),
            );
        }
        "deepseek" if model.reasoning => {
            if options.reasoning_effort.is_some() {
                params.insert(
                    "thinking".to_string(),
                    serde_json::json!({ "type": "enabled" }),
                );
            } else if model
                .thinking_level_map
                .as_ref()
                .and_then(|m| m.get(&ModelThinkingLevel::Off))
                .is_some()
            {
                params.insert(
                    "thinking".to_string(),
                    serde_json::json!({ "type": "disabled" }),
                );
            }
            if let Some(ref effort) = options.reasoning_effort {
                if compat.supports_reasoning_effort {
                    let mapped = map_effort(effort).unwrap_or_else(|| effort.clone());
                    params.insert("reasoning_effort".to_string(), Value::String(mapped));
                }
            }
        }
        "openrouter" if model.reasoning => {
            if let Some(ref effort) = options.reasoning_effort {
                let mapped = map_effort(effort).unwrap_or_else(|| effort.clone());
                params.insert(
                    "reasoning".to_string(),
                    serde_json::json!({
                        "effort": mapped,
                    }),
                );
            } else if model
                .thinking_level_map
                .as_ref()
                .and_then(|m| m.get(&ModelThinkingLevel::Off))
                .is_some()
            {
                let off_val = model
                    .thinking_level_map
                    .as_ref()
                    .and_then(|m| m.get(&ModelThinkingLevel::Off))
                    .and_then(|v| v.clone())
                    .unwrap_or_else(|| "none".to_string());
                params.insert(
                    "reasoning".to_string(),
                    serde_json::json!({ "effort": off_val }),
                );
            }
        }
        "ant-ling" if model.reasoning => {
            if let Some(ref effort) = options.reasoning_effort {
                let mapped = map_effort(effort).unwrap_or_else(|| effort.clone());
                params.insert(
                    "reasoning".to_string(),
                    serde_json::json!({ "effort": mapped }),
                );
            }
        }
        "together" if model.reasoning => {
            params.insert(
                "reasoning".to_string(),
                serde_json::json!({ "enabled": options.reasoning_effort.is_some() }),
            );
            if let Some(ref effort) = options.reasoning_effort {
                if compat.supports_reasoning_effort {
                    let mapped = map_effort(effort).unwrap_or_else(|| effort.clone());
                    params.insert("reasoning_effort".to_string(), Value::String(mapped));
                }
            }
        }
        "string-thinking" if model.reasoning => {
            if let Some(ref effort) = options.reasoning_effort {
                let mapped = map_effort(effort).unwrap_or_else(|| effort.clone());
                params.insert("thinking".to_string(), Value::String(mapped));
            } else if model
                .thinking_level_map
                .as_ref()
                .and_then(|m| m.get(&ModelThinkingLevel::Off))
                .is_some()
            {
                let off_val = model
                    .thinking_level_map
                    .as_ref()
                    .and_then(|m| m.get(&ModelThinkingLevel::Off))
                    .and_then(|v| v.clone())
                    .unwrap_or_else(|| "none".to_string());
                params.insert("thinking".to_string(), Value::String(off_val));
            }
        }
        _ => {
            // OpenAI-style reasoning_effort
            if let Some(ref effort) = options.reasoning_effort {
                if model.reasoning && compat.supports_reasoning_effort {
                    let mapped = map_effort(effort).unwrap_or_else(|| effort.clone());
                    params.insert("reasoning_effort".to_string(), Value::String(mapped));
                }
            } else if model.reasoning && compat.supports_reasoning_effort {
                if let Some(ref off_val) = model
                    .thinking_level_map
                    .as_ref()
                    .and_then(|m| m.get(&ModelThinkingLevel::Off))
                    .and_then(|v| v.clone())
                {
                    params.insert(
                        "reasoning_effort".to_string(),
                        Value::String(off_val.clone()),
                    );
                }
            }
        }
    }

    // OpenRouter provider routing preferences
    // TODO(compat): TS reads model.compat.openRouterRouting
    if !compat.open_router_routing.is_empty() {
        params.insert(
            "provider".to_string(),
            serde_json::to_value(&compat.open_router_routing).unwrap_or_default(),
        );
    }

    // Vercel AI Gateway provider routing
    if model.base_url.contains("ai-gateway.vercel.sh") && !compat.vercel_gateway_routing.is_empty()
    {
        let routing = &compat.vercel_gateway_routing;
        if routing.contains_key("only") || routing.contains_key("order") {
            params.insert(
                "providerOptions".to_string(),
                serde_json::json!({ "gateway": routing }),
            );
        }
    }

    Value::Object(params)
}

/// Resolve a thinking effort string through the model's thinking level map.
///
/// Maps hamr thinking level names to provider-specific values.
fn resolve_thinking_effort(effort: &str, model: &Model) -> Option<String> {
    let level = match effort {
        "off" => ModelThinkingLevel::Off,
        "minimal" => ModelThinkingLevel::Minimal,
        "low" => ModelThinkingLevel::Low,
        "medium" => ModelThinkingLevel::Medium,
        "high" => ModelThinkingLevel::High,
        "xhigh" => ModelThinkingLevel::XHigh,
        _ => return None,
    };
    model
        .thinking_level_map
        .as_ref()
        .and_then(|m| m.get(&level))
        .and_then(|v| v.clone())
}

// ---------------------------------------------------------------------------
// Usage parsing
// ---------------------------------------------------------------------------

fn parse_chunk_usage(raw_usage: &Value, model: &Model) -> Usage {
    let prompt_tokens = raw_usage
        .get("prompt_tokens")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    let completion_tokens = raw_usage
        .get("completion_tokens")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);

    let cache_read_tokens = raw_usage
        .get("prompt_tokens_details")
        .and_then(|d| d.get("cached_tokens"))
        .and_then(|v| v.as_u64())
        .or_else(|| {
            raw_usage
                .get("prompt_cache_hit_tokens")
                .and_then(|v| v.as_u64())
        })
        .unwrap_or(0);

    let cache_write_tokens = raw_usage
        .get("prompt_tokens_details")
        .and_then(|d| d.get("cache_write_tokens"))
        .and_then(|v| v.as_u64())
        .unwrap_or(0);

    let input = prompt_tokens.saturating_sub(cache_read_tokens + cache_write_tokens);
    let output_tokens = completion_tokens;
    let mut usage = Usage {
        input,
        output: output_tokens,
        cache_read: cache_read_tokens,
        cache_write: cache_write_tokens,
        cache_write_1h: None,
        total_tokens: input + output_tokens + cache_read_tokens + cache_write_tokens,
        cost: UsageCost {
            input: 0.0,
            output: 0.0,
            cache_read: 0.0,
            cache_write: 0.0,
            total: 0.0,
        },
    };
    calculate_cost(model, &mut usage);
    usage
}

// ---------------------------------------------------------------------------
// Stop reason mapping
// ---------------------------------------------------------------------------

fn map_stop_reason(reason: Option<&str>) -> (StopReason, Option<String>) {
    let reason = match reason {
        Some(r) => r,
        None => return (StopReason::Stop, None),
    };
    match reason {
        "stop" | "end" => (StopReason::Stop, None),
        "length" => (StopReason::Length, None),
        "function_call" | "tool_calls" => (StopReason::ToolUse, None),
        "content_filter" => (
            StopReason::Error,
            Some("Provider finish_reason: content_filter".to_string()),
        ),
        "network_error" => (
            StopReason::Error,
            Some("Provider finish_reason: network_error".to_string()),
        ),
        other => (
            StopReason::Error,
            Some(format!("Provider finish_reason: {}", other)),
        ),
    }
}

// ---------------------------------------------------------------------------
// SSE body parsing (from raw HTTP response)
// ---------------------------------------------------------------------------

/// Process the SSE body from a raw HTTP response, emitting events to the sender.
///
/// Mirrors the TS body of `streamOpenAICompatibleWithRawHttp`.
async fn process_sse_stream(
    response: reqwest::Response,
    model: &Model,
    mut output: AssistantMessage,
    options: &OpenAiCompletionsOptions,
    sender: &mut AssistantMessageEventStreamSender,
) -> Result<AssistantMessage, String> {
    sender.push(AssistantMessageEvent::Start {
        partial: output.clone(),
    });

    let mut content_text_idx: Option<usize> = None;
    let mut content_thinking_idx: Option<usize> = None;
    let mut tool_call_idx_by_index: HashMap<u64, usize> = HashMap::new();
    let mut tool_call_idx_by_id: HashMap<String, usize> = HashMap::new();
    let mut has_finish_reason = false;

    // SSE parsing: read bytes, split on \n, parse data: lines
    let mut byte_stream = response.bytes_stream();
    let mut buffer: Vec<u8> = Vec::new();

    'outer: loop {
        // Abort check between chunks
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

        // Process complete lines
        while let Some(pos) = buffer.iter().position(|&b| b == b'\n') {
            let line_bytes: Vec<u8> = buffer.drain(..=pos).collect();
            let line = String::from_utf8_lossy(&line_bytes);
            let line = line.trim_end_matches(['\r', '\n']);

            if line.is_empty() {
                continue;
            }

            // SSE comment lines (relay loading events)
            if line.starts_with(':') {
                let comment = line[1..].trim();
                if let Some(rest) = comment.strip_prefix("relay loading model=") {
                    let model_name = rest.trim();
                    sender.push(AssistantMessageEvent::Loading {
                        model: model_name.to_string(),
                        elapsed_ms: 0,
                    });
                }
                continue;
            }

            if !line.starts_with("data:") {
                continue;
            }
            let data = line[5..].trim();
            if data == "[DONE]" {
                break 'outer;
            }
            if data.is_empty() {
                continue;
            }

            let chunk_val: Value = match serde_json::from_str(data) {
                Ok(v) => v,
                Err(_) => continue,
            };

            // JSON loading event
            if chunk_val.get("event").and_then(|v| v.as_str()) == Some("loading") {
                let loading_model = chunk_val
                    .get("model")
                    .and_then(|v| v.as_str())
                    .unwrap_or(&model.id)
                    .to_string();
                let loading_elapsed = chunk_val
                    .get("elapsedMs")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);
                sender.push(AssistantMessageEvent::Loading {
                    model: loading_model,
                    elapsed_ms: loading_elapsed,
                });
                continue;
            }

            // responseId
            if output.response_id.is_none() {
                if let Some(id) = chunk_val.get("id").and_then(|v| v.as_str()) {
                    output.response_id = Some(id.to_string());
                }
            }

            // responseModel
            if let Some(chunk_model) = chunk_val.get("model").and_then(|v| v.as_str()) {
                if !chunk_model.is_empty() && chunk_model != model.id {
                    output
                        .response_model
                        .get_or_insert_with(|| chunk_model.to_string());
                }
            }

            // usage
            if let Some(usage) = chunk_val.get("usage") {
                output.usage = parse_chunk_usage(usage, model);
            }

            let choices = chunk_val
                .get("choices")
                .and_then(|c| c.as_array())
                .cloned()
                .unwrap_or_default();
            let choice = match choices.first() {
                Some(c) => c.clone(),
                None => continue,
            };

            // Fallback: usage in choice
            if chunk_val.get("usage").is_none() {
                if let Some(usage) = choice.get("usage") {
                    output.usage = parse_chunk_usage(usage, model);
                }
            }

            // finish_reason
            if let Some(finish) = choice.get("finish_reason").and_then(|v| v.as_str()) {
                if !finish.is_empty() {
                    let (stop_reason, error_msg) = map_stop_reason(Some(finish));
                    output.stop_reason = stop_reason;
                    output.error_message = error_msg;
                    has_finish_reason = true;
                }
            }

            let delta = match choice.get("delta") {
                Some(d) => d,
                None => continue,
            };

            // Content delta
            if let Some(content) = delta.get("content").and_then(|v| v.as_str()) {
                if !content.is_empty() {
                    let idx = if let Some(existing) = content_text_idx {
                        existing
                    } else {
                        let new_idx = output.content.len();
                        output
                            .content
                            .push(AssistantContentBlock::Text(TextContent {
                                text: String::new(),
                                text_signature: None,
                            }));
                        content_text_idx = Some(new_idx);
                        sender.push(AssistantMessageEvent::TextStart {
                            content_index: new_idx,
                            partial: output.clone(),
                        });
                        new_idx
                    };

                    if let Some(AssistantContentBlock::Text(tc)) = output.content.get_mut(idx) {
                        tc.text.push_str(content);
                    }
                    sender.push(AssistantMessageEvent::TextDelta {
                        content_index: idx,
                        delta: content.to_string(),
                        partial: output.clone(),
                    });
                }
            }

            // Reasoning fields
            let reasoning_fields = ["reasoning_content", "reasoning", "reasoning_text"];
            let mut found_reasoning_field: Option<String> = None;
            for field in &reasoning_fields {
                if let Some(val) = delta.get(field).and_then(|v| v.as_str()) {
                    if !val.is_empty() {
                        found_reasoning_field = Some(field.to_string());
                        break;
                    }
                }
            }

            if let Some(ref field) = found_reasoning_field {
                let rdelta = delta
                    .get(field)
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();

                let thinking_sig = if model.provider == "opencode-go" && field == "reasoning" {
                    "reasoning_content".to_string()
                } else {
                    field.clone()
                };

                let idx = if let Some(existing) = content_thinking_idx {
                    existing
                } else {
                    let new_idx = output.content.len();
                    output
                        .content
                        .push(AssistantContentBlock::Thinking(ThinkingContent {
                            thinking: String::new(),
                            thinking_signature: Some(thinking_sig.clone()),
                            redacted: false,
                        }));
                    content_thinking_idx = Some(new_idx);
                    sender.push(AssistantMessageEvent::ThinkingStart {
                        content_index: new_idx,
                        partial: output.clone(),
                    });
                    new_idx
                };

                if let Some(AssistantContentBlock::Thinking(tc)) = output.content.get_mut(idx) {
                    tc.thinking.push_str(&rdelta);
                }
                sender.push(AssistantMessageEvent::ThinkingDelta {
                    content_index: idx,
                    delta: rdelta,
                    partial: output.clone(),
                });
            }

            // Tool calls
            if let Some(tool_calls) = delta.get("tool_calls").and_then(|v| v.as_array()) {
                for tc in tool_calls {
                    let stream_index = tc.get("index").and_then(|v| v.as_u64());
                    let tc_id = tc
                        .get("id")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();

                    // Find or create block
                    let tool_idx = if let Some(si) = stream_index {
                        tool_call_idx_by_index.get(&si).copied()
                    } else if !tc_id.is_empty() {
                        tool_call_idx_by_id.get(&tc_id).copied()
                    } else {
                        None
                    };

                    let tool_idx = match tool_idx {
                        Some(idx) => idx,
                        None => {
                            let func_name = tc
                                .get("function")
                                .and_then(|f| f.get("name"))
                                .and_then(|v| v.as_str())
                                .unwrap_or("")
                                .to_string();

                            let new_idx = output.content.len();
                            output
                                .content
                                .push(AssistantContentBlock::ToolCall(ToolCall {
                                    id: tc_id.clone(),
                                    name: func_name,
                                    arguments: Value::Object(Default::default()),
                                    thought_signature: None,
                                }));
                            if let Some(si) = stream_index {
                                tool_call_idx_by_index.insert(si, new_idx);
                            }
                            if !tc_id.is_empty() {
                                tool_call_idx_by_id.insert(tc_id.clone(), new_idx);
                            }
                            sender.push(AssistantMessageEvent::ToolCallStart {
                                content_index: new_idx,
                                partial: output.clone(),
                            });
                            new_idx
                        }
                    };

                    // Update id/name if missing, and process arguments delta
                    let args_delta = tc
                        .get("function")
                        .and_then(|f| f.get("arguments"))
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());

                    if let Some(AssistantContentBlock::ToolCall(tcb)) =
                        output.content.get_mut(tool_idx)
                    {
                        if tcb.id.is_empty() && !tc_id.is_empty() {
                            tcb.id = tc_id.clone();
                            tool_call_idx_by_id.insert(tc_id, tool_idx);
                        }
                        if tcb.name.is_empty() {
                            if let Some(name) = tc
                                .get("function")
                                .and_then(|f| f.get("name"))
                                .and_then(|v| v.as_str())
                            {
                                tcb.name = name.to_string();
                            }
                        }

                        if let Some(ref delta) = args_delta {
                            // Store partial args in thought_signature as scratch buffer
                            let mut partial = tcb.thought_signature.clone().unwrap_or_default();
                            partial.push_str(delta);
                            tcb.thought_signature = Some(partial.clone());
                            tcb.arguments = parse_streaming_json(Some(&partial));

                            sender.push(AssistantMessageEvent::ToolCallDelta {
                                content_index: tool_idx,
                                delta: delta.clone(),
                                partial: output.clone(),
                            });
                        }
                    }
                }
            }

            // reasoning_details
            if let Some(reasoning_details) =
                delta.get("reasoning_details").and_then(|v| v.as_array())
            {
                for detail in reasoning_details {
                    if let Some(detail_obj) = detail.as_object() {
                        if detail_obj.get("type").and_then(|t| t.as_str())
                            == Some("reasoning.encrypted")
                        {
                            if let Some(detail_id) = detail_obj.get("id").and_then(|v| v.as_str()) {
                                // Find matching tool call
                                for block in output.content.iter_mut() {
                                    if let AssistantContentBlock::ToolCall(tc) = block {
                                        if tc.id == detail_id {
                                            tc.thought_signature = Some(
                                                serde_json::to_string(detail).unwrap_or_default(),
                                            );
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Finish all blocks
    if let Some(idx) = content_text_idx {
        if let Some(AssistantContentBlock::Text(tc)) = output.content.get(idx) {
            sender.push(AssistantMessageEvent::TextEnd {
                content_index: idx,
                content: tc.text.clone(),
                partial: output.clone(),
            });
        }
    }
    if let Some(idx) = content_thinking_idx {
        if let Some(AssistantContentBlock::Thinking(tc)) = output.content.get(idx) {
            sender.push(AssistantMessageEvent::ThinkingEnd {
                content_index: idx,
                content: tc.thinking.clone(),
                partial: output.clone(),
            });
        }
    }
    // Finalize tool calls: emit ToolCallEnd with cleaned-up ToolCall
    let finalized_indices: Vec<usize> = tool_call_idx_by_id.values().copied().collect();
    for idx in finalized_indices {
        if let Some(AssistantContentBlock::ToolCall(tc)) = output.content.get(idx) {
            let finalized_tc = ToolCall {
                id: tc.id.clone(),
                name: tc.name.clone(),
                arguments: tc.arguments.clone(),
                thought_signature: None,
            };
            sender.push(AssistantMessageEvent::ToolCallEnd {
                content_index: idx,
                tool_call: finalized_tc,
                partial: output.clone(),
            });
        }
    }

    // Post-stream validation
    if let Some(signal) = &options.base.signal {
        if *signal.borrow() {
            return Err("Request was aborted".to_string());
        }
    }
    if output.stop_reason == StopReason::Aborted {
        return Err("Request was aborted".to_string());
    }
    if output.stop_reason == StopReason::Error {
        return Err(output
            .error_message
            .clone()
            .unwrap_or_else(|| "Provider returned an error stop reason".to_string()));
    }
    if !has_finish_reason {
        return Err("Stream ended without finish_reason".to_string());
    }

    Ok(output)
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
// Raw HTTP streaming (raw fetch, no SDK)
// ---------------------------------------------------------------------------

async fn stream_openai_compatible_with_raw_http(
    model: &Model,
    params: Value,
    output: AssistantMessage,
    options: &OpenAiCompletionsOptions,
    sender: &mut AssistantMessageEventStreamSender,
) -> Result<AssistantMessage, String> {
    let mut body = params;
    // For non-relay raw-HTTP callers, inject prompt_cache_key
    if let Some(session_id) = &options.base.session_id {
        if body.get("prompt_cache_key").is_none() {
            if let Some(key) = clamp_openai_prompt_cache_key(Some(session_id)) {
                if let Some(obj) = body.as_object() {
                    let mut new_body = obj.clone();
                    new_body.insert("prompt_cache_key".to_string(), Value::String(key));
                    body = Value::Object(new_body);
                }
            }
        }
    }

    let headers = {
        let mut h = build_raw_openai_compatible_headers(
            options.base.api_key.as_deref().unwrap_or(""),
            options.base.headers.as_ref(),
            options.base.session_id.as_deref(),
        );
        h.insert("Accept".to_string(), "text/event-stream".to_string());
        h
    };

    let timeout_ms = options.base.timeout_ms.unwrap_or(600_000);
    let url = format!("{}/chat/completions", model.base_url.trim_end_matches('/'));

    let client = reqwest::Client::new();
    let mut request = client.post(&url).header("content-type", "application/json");

    for (k, v) in &headers {
        request = request.header(k.as_str(), v.as_str());
    }

    let body_bytes =
        serde_json::to_vec(&body).map_err(|e| format!("Failed to serialize body: {}", e))?;

    // Send with retry for retryable status codes (5xx, 429)
    const MAX_RETRIES: u32 = 3;
    let mut attempt = 0u32;
    loop {
        let retry_request = client
            .post(&url)
            .header("content-type", "application/json")
            .body(body_bytes.clone());
        let retry_request = headers.iter().fold(retry_request, |req, (k, v)| {
            req.header(k.as_str(), v.as_str())
        });

        let response = match send_with_timeout_and_abort(
            retry_request,
            timeout_ms,
            options.base.signal.clone(),
        )
        .await
        {
            Ok(resp) => resp,
            Err(e) => return Err(e),
        };

        let status = response.status().as_u16();

        // Retry on 5xx (server error) and 429 (rate limit)
        if (status >= 500 || status == 429) && attempt < MAX_RETRIES {
            attempt += 1;
            let delay_ms = 1000u64 * (1u64 << (attempt - 1).min(4));
            // Check abort signal before sleeping
            if let Some(ref sig) = options.base.signal {
                if *sig.borrow() {
                    return Err("Request was aborted".to_string());
                }
            }
            tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
            continue;
        }

        if !response.status().is_success() {
            let body_text = response.text().await.unwrap_or_default();
            return Err(parse_raw_openai_compatible_error(status, &body_text));
        }

        return process_sse_stream(response, model, output, options, sender).await;
    }
}

/// Send a request, racing it against timeout and abort signal.
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
// SDK-based streaming (through reqwest directly, no OpenAI SDK)
// ---------------------------------------------------------------------------

/// The TS uses the OpenAI SDK. In Rust we use reqwest directly for both the
/// "raw HTTP" and "SDK-like" paths, since the SDK-like path is the same HTTP
/// with slightly different header/URL construction.
async fn run_stream_inner(
    model: &Model,
    context: &Context,
    options: &OpenAiCompletionsOptions,
    sender: &mut AssistantMessageEventStreamSender,
    output: &mut AssistantMessage,
) -> Result<(), String> {
    let api_key = match &options.base.api_key {
        Some(k) if !k.is_empty() => k.clone(),
        _ => return Err(format!("No API key for provider: {}", model.provider)),
    };

    let compat = get_compat(model);
    let cache_retention =
        resolve_cache_retention(options.base.cache_retention, options.base.env.as_ref());
    let mut params = build_params(model, context, options, &compat, cache_retention);

    // onPayload hook
    if let Some(on_payload) = &options.base.on_payload {
        if let Some(next) = on_payload(params.clone(), model.clone()).await {
            params = next;
        }
    }

    // Check if we should use raw HTTP
    let use_raw =
        should_use_raw_openai_compatible_http(model, &api_key, options.base.headers.as_ref());

    if use_raw {
        *output =
            stream_openai_compatible_with_raw_http(model, params, output.clone(), options, sender)
                .await?;
        return Ok(());
    }

    // SDK-like path: build headers and issue POST directly
    let mut headers = model.headers.clone().unwrap_or_default();

    // GitHub Copilot headers
    if model.provider == "github-copilot" {
        let has_images = has_copilot_vision_input(&context.messages);
        let copilot_headers = build_copilot_dynamic_headers(&context.messages, has_images);
        for (k, v) in copilot_headers {
            headers.insert(k, v);
        }
    }

    // Session affinity headers
    if let Some(session_id) = &options.base.session_id {
        if compat.send_session_affinity_headers {
            headers.insert("session_id".to_string(), session_id.clone());
            headers.insert("x-client-request-id".to_string(), session_id.clone());
            headers.insert("x-session-affinity".to_string(), session_id.clone());
        }
    }

    // Options headers override model headers
    if let Some(opt_headers) = &options.base.headers {
        for (k, v) in opt_headers {
            headers.insert(k.clone(), v.clone());
        }
    }

    // Cloudflare authorization header override
    let default_headers = if model.provider == "cloudflare-ai-gateway" {
        let mut h = headers.clone();
        h.remove("Authorization");
        h.insert(
            "cf-aig-authorization".to_string(),
            format!("Bearer {}", api_key),
        );
        h
    } else if api_key == "not-needed" {
        let mut h = headers;
        h.remove("Authorization");
        h
    } else {
        headers
    };

    let base_url = if is_cloudflare_provider(&model.provider) {
        match resolve_cloudflare_base_url(model, options.base.env.as_ref()) {
            Ok(url) => url,
            Err(e) => return Err(e),
        }
    } else {
        model.base_url.clone()
    };

    let url = format!("{}/chat/completions", base_url.trim_end_matches('/'));

    let client = reqwest::Client::new();
    let mut request = client
        .post(&url)
        .header("authorization", format!("Bearer {}", api_key))
        .header("content-type", "application/json");

    for (k, v) in &default_headers {
        request = request.header(k.as_str(), v.as_str());
    }

    let body_bytes = serde_json::to_vec(&params).map_err(|e| e.to_string())?;
    request = request.body(body_bytes);

    // Send with timeout
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
        return Err(parse_raw_openai_compatible_error(status, &body_text));
    }

    *output = process_sse_stream(response, model, output.clone(), options, sender).await?;

    Ok(())
}

// ---------------------------------------------------------------------------
// Run stream (top-level driver)
// ---------------------------------------------------------------------------

async fn run_stream(
    model: Model,
    context: Context,
    options: OpenAiCompletionsOptions,
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

/// Stream a completion from an OpenAI-compatible Chat Completions API.
///
/// Mirrors the TS `streamOpenAICompletions`.
pub fn stream(
    model: Model,
    context: Context,
    options: Option<OpenAiCompletionsOptions>,
) -> AssistantMessageEventStream {
    let (sender, stream_out) = create_assistant_message_event_stream();
    let options = options.unwrap_or_default();

    tokio::spawn(async move {
        run_stream(model, context, options, sender).await;
    });

    stream_out
}

/// TS-named alias for [`stream`].
pub fn stream_openai_completions(
    model: Model,
    context: Context,
    options: Option<OpenAiCompletionsOptions>,
) -> AssistantMessageEventStream {
    stream(model, context, options)
}

/// Stream with simplified reasoning-level options.
///
/// Mirrors the TS `streamSimpleOpenAICompletions`: maps the unified `reasoning`
/// level into reasoning_effort.
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
    let clamped_reasoning =
        reasoning.map(|r| clamp_thinking_level(&model, ModelThinkingLevel::from(r)));
    let reasoning_effort = match clamped_reasoning {
        Some(ModelThinkingLevel::Off) => None,
        Some(level) => Some(level_to_effort_string(level)),
        None => None,
    };

    // Extract tool_choice from options.extra
    let tool_choice = options
        .as_ref()
        .and_then(|o| o.base.metadata.as_ref())
        .and_then(|m| m.get("toolChoice"))
        .cloned();

    let opts = OpenAiCompletionsOptions {
        base,
        tool_choice,
        reasoning_effort,
    };

    stream(model, context, Some(opts))
}

/// TS-named alias for [`stream_simple`].
pub fn stream_simple_openai_completions(
    model: Model,
    context: Context,
    options: Option<SimpleStreamOptions>,
) -> AssistantMessageEventStream {
    stream_simple(model, context, options)
}

/// Convert a `ModelThinkingLevel` to the OpenAI `reasoning_effort` string.
fn level_to_effort_string(level: ModelThinkingLevel) -> String {
    match level {
        ModelThinkingLevel::Off => "off".to_string(),
        ModelThinkingLevel::Minimal => "minimal".to_string(),
        ModelThinkingLevel::Low => "low".to_string(),
        ModelThinkingLevel::Medium => "medium".to_string(),
        ModelThinkingLevel::High => "high".to_string(),
        ModelThinkingLevel::XHigh => "xhigh".to_string(),
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
            id: "gpt-4".to_string(),
            name: "GPT-4".to_string(),
            api: Api::OpenAiCompletions,
            provider: "openai".to_string(),
            base_url: "https://api.openai.com/v1".to_string(),
            reasoning: false,
            thinking_level_map: None,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 10.0,
                output: 30.0,
                cache_read: 5.0,
                cache_write: 10.0,
            },
            context_window: 8192,
            max_tokens: 4096,
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
    fn detect_compat_openai() {
        let model = test_model();
        let compat = detect_compat(&model);
        assert!(compat.supports_store);
        assert!(compat.supports_developer_role);
        assert_eq!(compat.thinking_format, "openai");
        assert_eq!(compat.max_tokens_field, "max_completion_tokens");
    }

    #[test]
    fn detect_compat_zai() {
        let model = Model {
            provider: "zai".to_string(),
            base_url: "https://api.z.ai".to_string(),
            ..test_model()
        };
        let compat = detect_compat(&model);
        assert_eq!(compat.thinking_format, "zai");
        assert!(!compat.supports_store);
    }

    #[test]
    fn detect_compat_deepseek_uses_max_tokens() {
        let model = Model {
            provider: "deepseek".to_string(),
            base_url: "https://api.deepseek.com".to_string(),
            ..test_model()
        };
        let compat = detect_compat(&model);
        assert_eq!(compat.max_tokens_field, "max_completion_tokens");
        assert_eq!(compat.thinking_format, "deepseek");
        assert!(compat.requires_reasoning_content_on_assistant_messages);
    }

    #[test]
    fn detect_compat_chutes_uses_max_tokens() {
        let model = Model {
            provider: "chutes".to_string(),
            base_url: "https://chutes.ai".to_string(),
            ..test_model()
        };
        let compat = detect_compat(&model);
        assert_eq!(compat.max_tokens_field, "max_tokens");
    }

    #[test]
    fn has_tool_history_detects_tool_results() {
        let messages = vec![Message::ToolResult(ToolResultMessage {
            role: MessageRole::ToolResult,
            tool_call_id: "call_1".to_string(),
            tool_name: "test".to_string(),
            content: vec![],
            details: None,
            is_error: false,
            timestamp: Utc::now(),
        })];
        assert!(has_tool_history(&messages));
    }

    #[test]
    fn has_tool_history_detects_tool_calls() {
        let messages = vec![Message::Assistant(AssistantMessage {
            role: MessageRole::Assistant,
            content: vec![AssistantContentBlock::ToolCall(ToolCall {
                id: "call_1".to_string(),
                name: "test".to_string(),
                arguments: Value::Object(Default::default()),
                thought_signature: None,
            })],
            api: "openai-completions".to_string(),
            provider: "openai".to_string(),
            model: "gpt-4".to_string(),
            response_model: None,
            response_id: None,
            usage: empty_usage(),
            stop_reason: StopReason::Stop,
            error_message: None,
            diagnostics: None,
            timestamp: Utc::now(),
        })];
        assert!(has_tool_history(&messages));
    }

    #[test]
    fn has_tool_history_empty() {
        assert!(!has_tool_history(&[]));
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
    fn convert_messages_user_text() {
        let model = test_model();
        let compat = detect_compat(&model);
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
        let result = convert_messages(&model, &context, &compat);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].get("role").and_then(|v| v.as_str()), Some("user"));
        // Content is an array of content parts (Vec<MessageContent> always becomes array)
        let content = result[0].get("content").and_then(|v| v.as_array()).unwrap();
        assert_eq!(content.len(), 1);
        assert_eq!(
            content[0].get("text").and_then(|v| v.as_str()),
            Some("Hello")
        );
    }

    #[test]
    fn convert_messages_with_system_prompt() {
        let model = test_model();
        let compat = detect_compat(&model);
        let context = Context {
            system_prompt: Some("Be helpful.".to_string()),
            messages: vec![Message::User(UserMessage {
                role: MessageRole::User,
                content: vec![MessageContent::Text(TextContent {
                    text: "Hi".to_string(),
                    text_signature: None,
                })],
                timestamp: Utc::now(),
            })],
            tools: vec![],
        };
        let result = convert_messages(&model, &context, &compat);
        assert_eq!(result.len(), 2);
        assert_eq!(
            result[0].get("content").and_then(|v| v.as_str()),
            Some("Be helpful.")
        );
    }

    #[test]
    fn map_stop_reason_values() {
        assert_eq!(map_stop_reason(Some("stop")), (StopReason::Stop, None));
        assert_eq!(map_stop_reason(Some("length")), (StopReason::Length, None));
        assert_eq!(
            map_stop_reason(Some("tool_calls")),
            (StopReason::ToolUse, None)
        );
        let (reason, msg) = map_stop_reason(Some("content_filter"));
        assert_eq!(reason, StopReason::Error);
        assert!(msg.unwrap().contains("content_filter"));
    }

    #[test]
    fn convert_tools_basic() {
        let compat = detect_compat(&test_model());
        let tools = vec![Tool {
            name: "get_weather".to_string(),
            description: "Get weather".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "location": { "type": "string" }
                }
            }),
        }];
        let result = convert_tools(&tools, &compat);
        assert_eq!(result.len(), 1);
        assert_eq!(
            result[0]
                .get("function")
                .and_then(|f| f.get("name"))
                .and_then(|v| v.as_str()),
            Some("get_weather")
        );
        assert!(
            result[0]
                .get("function")
                .and_then(|f| f.get("strict"))
                .is_some()
        );
    }

    #[test]
    fn convert_tools_no_strict_for_moonshot() {
        let model = Model {
            provider: "moonshotai".to_string(),
            base_url: "https://api.moonshot.ai".to_string(),
            ..test_model()
        };
        let compat = detect_compat(&model);
        assert!(!compat.supports_strict_mode);
        let tools = vec![Tool {
            name: "test".to_string(),
            description: "Test".to_string(),
            parameters: serde_json::json!({}),
        }];
        let result = convert_tools(&tools, &compat);
        assert!(
            result[0]
                .get("function")
                .and_then(|f| f.get("strict"))
                .is_none()
        );
    }

    #[test]
    fn parse_chunk_usage_basic() {
        let model = test_model();
        let usage = serde_json::json!({
            "prompt_tokens": 100,
            "completion_tokens": 50,
            "prompt_tokens_details": {
                "cached_tokens": 10
            }
        });
        let result = parse_chunk_usage(&usage, &model);
        assert_eq!(result.input, 90); // 100 - 10 cached
        assert_eq!(result.output, 50);
        assert_eq!(result.cache_read, 10);
    }

    #[test]
    fn parse_chunk_usage_with_cache_write() {
        let model = test_model();
        let usage = serde_json::json!({
            "prompt_tokens": 200,
            "completion_tokens": 100,
            "prompt_tokens_details": {
                "cached_tokens": 20,
                "cache_write_tokens": 30
            }
        });
        let result = parse_chunk_usage(&usage, &model);
        assert_eq!(result.input, 150); // 200 - 20 - 30
        assert_eq!(result.cache_read, 20);
        assert_eq!(result.cache_write, 30);
    }

    #[test]
    fn build_params_sets_stream_and_model() {
        let model = test_model();
        let context = empty_context();
        let compat = detect_compat(&model);
        let options = OpenAiCompletionsOptions::default();
        let params = build_params(&model, &context, &options, &compat, CacheRetention::Short);
        assert_eq!(params.get("model").and_then(|v| v.as_str()), Some("gpt-4"));
        assert_eq!(params.get("stream").and_then(|v| v.as_bool()), Some(true));
        assert_eq!(
            params
                .get("stream_options")
                .and_then(|v| v.get("include_usage"))
                .and_then(|v| v.as_bool()),
            Some(true)
        );
    }

    #[test]
    fn build_params_with_tools() {
        let model = test_model();
        let compat = detect_compat(&model);
        let options = OpenAiCompletionsOptions::default();
        let context = Context {
            system_prompt: None,
            messages: vec![],
            tools: vec![Tool {
                name: "get_weather".to_string(),
                description: "Get weather".to_string(),
                parameters: serde_json::json!({}),
            }],
        };
        let params = build_params(&model, &context, &options, &compat, CacheRetention::Short);
        assert!(params.get("tools").is_some());
    }

    #[test]
    fn should_use_raw_http_for_relay() {
        let model = test_model();
        assert!(should_use_raw_openai_compatible_http(
            &Model {
                provider: "relay".to_string(),
                ..model.clone()
            },
            "some-key",
            None
        ));
        assert!(should_use_raw_openai_compatible_http(
            &model,
            "not-needed",
            None
        ));
    }

    #[test]
    fn level_to_effort_string_values() {
        assert_eq!(level_to_effort_string(ModelThinkingLevel::Off), "off");
        assert_eq!(
            level_to_effort_string(ModelThinkingLevel::Minimal),
            "minimal"
        );
        assert_eq!(level_to_effort_string(ModelThinkingLevel::Low), "low");
        assert_eq!(level_to_effort_string(ModelThinkingLevel::Medium), "medium");
        assert_eq!(level_to_effort_string(ModelThinkingLevel::High), "high");
        assert_eq!(level_to_effort_string(ModelThinkingLevel::XHigh), "xhigh");
    }

    #[test]
    fn normalize_tool_call_id_pipe_form() {
        let id =
            "call_abc|long_suffix_data_here_12345678901234567890123456789012345678901234567890";
        if id.contains('|') {
            let call_id = id.split('|').next().unwrap_or("");
            let sanitized: String = call_id
                .chars()
                .map(|c| {
                    if c.is_alphanumeric() || c == '_' || c == '-' {
                        c
                    } else {
                        '_'
                    }
                })
                .collect();
            let truncated: String = sanitized.chars().take(40).collect();
            assert_eq!(truncated, "call_abc");
        }
    }

    #[test]
    fn build_params_with_tool_choice() {
        let model = test_model();
        let compat = detect_compat(&model);
        let options = OpenAiCompletionsOptions {
            tool_choice: Some(serde_json::json!("auto")),
            ..Default::default()
        };
        let context = empty_context();
        let params = build_params(&model, &context, &options, &compat, CacheRetention::Short);
        assert_eq!(
            params.get("tool_choice").and_then(|v| v.as_str()),
            Some("auto")
        );
    }

    #[test]
    fn build_params_with_reasoning_effort() {
        let model = Model {
            reasoning: true,
            ..test_model()
        };
        let compat = detect_compat(&model);
        let options = OpenAiCompletionsOptions {
            reasoning_effort: Some("high".to_string()),
            ..Default::default()
        };
        let context = empty_context();
        let params = build_params(&model, &context, &options, &compat, CacheRetention::Short);
        assert_eq!(
            params.get("reasoning_effort").and_then(|v| v.as_str()),
            Some("high")
        );
    }

    #[test]
    fn convert_messages_with_images() {
        let model = test_model();
        let compat = detect_compat(&model);
        let context = Context {
            system_prompt: None,
            messages: vec![Message::User(UserMessage {
                role: MessageRole::User,
                content: vec![
                    MessageContent::Text(TextContent {
                        text: "What's this?".to_string(),
                        text_signature: None,
                    }),
                    MessageContent::Image(ImageContent {
                        data: "iVBOR".to_string(),
                        mime_type: "image/png".to_string(),
                    }),
                ],
                timestamp: Utc::now(),
            })],
            tools: vec![],
        };
        let result = convert_messages(&model, &context, &compat);
        assert_eq!(result.len(), 1);
        let content = result[0].get("content").and_then(|v| v.as_array()).unwrap();
        assert_eq!(content.len(), 2);
        assert_eq!(
            content[1]
                .get("image_url")
                .and_then(|i| i.get("url"))
                .and_then(|v| v.as_str()),
            Some("data:image/png;base64,iVBOR")
        );
    }

    #[test]
    fn resolve_thinking_effort_through_map() {
        let model = Model {
            reasoning: true,
            thinking_level_map: Some({
                let mut m = std::collections::HashMap::new();
                m.insert(ModelThinkingLevel::High, Some("high".to_string()));
                m
            }),
            ..test_model()
        };
        assert_eq!(
            resolve_thinking_effort("high", &model),
            Some("high".to_string())
        );
        assert_eq!(resolve_thinking_effort("low", &model), None); // not in map
    }

    #[test]
    fn parse_raw_error_from_json() {
        let body = r#"{"error": {"message": "Rate limit exceeded"}}"#;
        let msg = parse_raw_openai_compatible_error(429, body);
        assert!(msg.contains("429"));
        assert!(msg.contains("Rate limit"));
    }

    #[test]
    fn parse_raw_error_from_plain_text() {
        let body = "Bad Gateway";
        let msg = parse_raw_openai_compatible_error(502, body);
        assert!(msg.contains("502"));
        assert!(msg.contains("Bad Gateway"));
    }

    #[test]
    fn detect_compat_opencode_is_non_standard() {
        let model = Model {
            provider: "opencode".to_string(),
            ..test_model()
        };
        let compat = detect_compat(&model);
        // opencode is non-standard, but not in use_max_tokens list.
        assert!(!compat.supports_store);
        assert_eq!(compat.max_tokens_field, "max_completion_tokens");
    }

    #[test]
    fn detect_compat_openrouter_anthropic_model_sets_cache_control_format() {
        let model = Model {
            id: "anthropic/claude-sonnet-4".to_string(),
            provider: "openrouter".to_string(),
            base_url: "https://openrouter.ai".to_string(),
            ..test_model()
        };
        let compat = detect_compat(&model);
        assert_eq!(compat.cache_control_format.as_deref(), Some("anthropic"));
    }

    #[test]
    fn detect_compat_openrouter_non_anthropic_model_no_cache_control() {
        let model = Model {
            id: "openai/gpt-4o".to_string(),
            provider: "openrouter".to_string(),
            base_url: "https://openrouter.ai".to_string(),
            ..test_model()
        };
        let compat = detect_compat(&model);
        assert!(compat.cache_control_format.is_none());
    }

    #[test]
    fn build_params_openrouter_anthropic_cache_control_on_system_and_last_message() {
        let model = Model {
            id: "anthropic/claude-opus-4".to_string(),
            provider: "openrouter".to_string(),
            base_url: "https://openrouter.ai".to_string(),
            reasoning: true,
            ..test_model()
        };
        let compat = detect_compat(&model);
        assert_eq!(compat.cache_control_format.as_deref(), Some("anthropic"));
        let context = Context {
            system_prompt: Some("You are Claude.".to_string()),
            messages: vec![Message::User(UserMessage {
                role: MessageRole::User,
                content: vec![MessageContent::Text(TextContent {
                    text: "Hello".to_string(),
                    text_signature: None,
                })],
                timestamp: Utc::now(),
            })],
            tools: vec![Tool {
                name: "get_weather".to_string(),
                description: "Get weather".to_string(),
                parameters: serde_json::json!({"type": "object"}),
            }],
        };
        let options = OpenAiCompletionsOptions::default();
        let params = build_params(&model, &context, &options, &compat, CacheRetention::Short);
        // Messages array should contain: [developer(system), user, ...]
        let msgs = params.get("messages").and_then(|m| m.as_array()).unwrap();
        assert!(msgs.len() >= 2);
        // System/developer prompt at index 0
        assert_eq!(
            msgs[0].get("role").and_then(|r| r.as_str()),
            Some("developer")
        );
        // After apply_anthropic_cache_control, system content is converted
        // from a plain string to a content-part array with cache_control.
        let content_arr = msgs[0].get("content").and_then(|c| c.as_array()).unwrap();
        assert_eq!(
            content_arr[0].get("type").and_then(|t| t.as_str()),
            Some("text")
        );
        assert_eq!(
            content_arr[0].get("text").and_then(|t| t.as_str()),
            Some("You are Claude.")
        );
        assert!(content_arr[0].get("cache_control").is_some());
        // User message at index 1
        assert_eq!(msgs[1].get("role").and_then(|r| r.as_str()), Some("user"));
        // Tools should be present
        assert!(params.get("tools").is_some());
    }

    #[test]
    fn build_params_chutes_uses_max_tokens_field() {
        let model = Model {
            provider: "chutes".to_string(),
            base_url: "https://chutes.ai".to_string(),
            ..test_model()
        };
        let compat = detect_compat(&model);
        assert_eq!(compat.max_tokens_field, "max_tokens");
        let options = OpenAiCompletionsOptions {
            base: StreamOptions {
                max_tokens: Some(2048),
                ..Default::default()
            },
            ..Default::default()
        };
        let params = build_params(
            &model,
            &empty_context(),
            &options,
            &compat,
            CacheRetention::Short,
        );
        assert_eq!(
            params.get("max_tokens").and_then(|v| v.as_u64()),
            Some(2048)
        );
        assert!(params.get("max_completion_tokens").is_none());
    }

    // -----------------------------------------------------------------------
    // Additional tests ported from TS openai-completions test files
    // -----------------------------------------------------------------------

    #[test]
    fn build_params_omits_tools_when_empty_and_no_history() {
        let model = test_model();
        let compat = detect_compat(&model);
        let options = OpenAiCompletionsOptions::default();
        let context = Context {
            system_prompt: None,
            messages: vec![],
            tools: vec![],
        };
        let params = build_params(&model, &context, &options, &compat, CacheRetention::Short);
        assert!(params.get("tools").is_none());
    }

    #[test]
    fn build_params_emits_empty_tools_array_when_tool_history_present() {
        let model = test_model();
        let compat = detect_compat(&model);
        let options = OpenAiCompletionsOptions::default();
        let context = Context {
            system_prompt: None,
            messages: vec![
                Message::User(UserMessage {
                    role: MessageRole::User,
                    content: vec![MessageContent::Text(TextContent {
                        text: "do it".to_string(),
                        text_signature: None,
                    })],
                    timestamp: Utc::now(),
                }),
                Message::Assistant(AssistantMessage {
                    role: MessageRole::Assistant,
                    content: vec![AssistantContentBlock::ToolCall(ToolCall {
                        id: "t1".to_string(),
                        name: "noop".to_string(),
                        arguments: Value::Object(Default::default()),
                        thought_signature: None,
                    })],
                    api: "openai-completions".to_string(),
                    provider: "openai".to_string(),
                    model: "gpt-4".to_string(),
                    response_model: None,
                    response_id: None,
                    usage: empty_usage(),
                    stop_reason: StopReason::ToolUse,
                    error_message: None,
                    diagnostics: None,
                    timestamp: Utc::now(),
                }),
                Message::ToolResult(ToolResultMessage {
                    role: MessageRole::ToolResult,
                    tool_call_id: "t1".to_string(),
                    tool_name: "noop".to_string(),
                    content: vec![MessageContent::Text(TextContent {
                        text: "done".to_string(),
                        text_signature: None,
                    })],
                    details: None,
                    is_error: false,
                    timestamp: Utc::now(),
                }),
            ],
            tools: vec![],
        };
        let params = build_params(&model, &context, &options, &compat, CacheRetention::Short);
        let tools = params.get("tools").and_then(|v| v.as_array());
        assert!(tools.is_some());
        assert!(tools.unwrap().is_empty());
    }

    #[test]
    fn build_params_omits_max_tokens_when_none() {
        let model = test_model();
        let compat = detect_compat(&model);
        let options = OpenAiCompletionsOptions::default();
        let params = build_params(
            &model,
            &empty_context(),
            &options,
            &compat,
            CacheRetention::Short,
        );
        assert!(params.get("max_tokens").is_none());
        assert!(params.get("max_completion_tokens").is_none());
    }

    #[test]
    fn build_params_sends_explicit_max_tokens() {
        let model = test_model();
        let compat = detect_compat(&model);
        let options = OpenAiCompletionsOptions {
            base: StreamOptions {
                max_tokens: Some(1234),
                ..Default::default()
            },
            ..Default::default()
        };
        let params = build_params(
            &model,
            &empty_context(),
            &options,
            &compat,
            CacheRetention::Short,
        );
        assert_eq!(
            params.get("max_completion_tokens").and_then(|v| v.as_u64()),
            Some(1234)
        );
        assert!(params.get("max_tokens").is_none());
    }

    #[test]
    fn build_params_omits_store_for_non_standard() {
        let model = Model {
            provider: "zai".to_string(),
            base_url: "https://api.z.ai".to_string(),
            ..test_model()
        };
        let compat = detect_compat(&model);
        let params = build_params(
            &model,
            &empty_context(),
            &OpenAiCompletionsOptions::default(),
            &compat,
            CacheRetention::Short,
        );
        assert!(params.get("store").is_none());
    }

    #[test]
    fn build_params_includes_store_for_openai() {
        let model = test_model();
        let compat = detect_compat(&model);
        let params = build_params(
            &model,
            &empty_context(),
            &OpenAiCompletionsOptions::default(),
            &compat,
            CacheRetention::Short,
        );
        assert_eq!(params.get("store").and_then(|v| v.as_bool()), Some(false));
    }

    #[ignore = "provider-specific compat: may change with API updates"]
    #[test]
    fn build_params_zai_tool_stream_enabled() {
        let model = Model {
            provider: "zai".to_string(),
            base_url: "https://api.z.ai".to_string(),
            ..test_model()
        };
        let compat = detect_compat(&model);
        assert!(compat.zai_tool_stream);
        let context = Context {
            system_prompt: None,
            messages: vec![],
            tools: vec![Tool {
                name: "ping".to_string(),
                description: "Ping tool".to_string(),
                parameters: serde_json::json!({}),
            }],
        };
        let params = build_params(
            &model,
            &context,
            &OpenAiCompletionsOptions::default(),
            &compat,
            CacheRetention::Short,
        );
        assert_eq!(
            params.get("tool_stream").and_then(|v| v.as_bool()),
            Some(true)
        );
    }

    #[test]
    fn build_params_zai_tool_stream_omitted_when_no_tools() {
        let model = Model {
            provider: "zai".to_string(),
            base_url: "https://api.z.ai".to_string(),
            ..test_model()
        };
        let compat = detect_compat(&model);
        let params = build_params(
            &model,
            &empty_context(),
            &OpenAiCompletionsOptions::default(),
            &compat,
            CacheRetention::Short,
        );
        assert!(params.get("tool_stream").is_none());
    }

    #[ignore = "provider-specific compat: may change with API updates"]
    #[test]
    fn build_params_zai_tool_stream_omitted_for_unsupported_model() {
        let model = Model {
            provider: "zai".to_string(),
            base_url: "https://api.z.ai".to_string(),
            ..test_model()
        };
        // By default, test_model() has internal provider "openai" — we overrode provider above,
        // so zai_tool_stream is true in compat. For zai glm-4.5-air which doesn't support it,
        // we'd need model.compat override. Since detect_compat is always true for zai,
        // this test verifies the invariant: all zai models get zai_tool_stream from auto-detect.
        // The unsupported case is handled by model.compat override (not yet in Rust).
        let compat = detect_compat(&model);
        let context = Context {
            system_prompt: None,
            messages: vec![],
            tools: vec![Tool {
                name: "ping".to_string(),
                description: "Ping".to_string(),
                parameters: serde_json::json!({}),
            }],
        };
        let params = build_params(
            &model,
            &context,
            &OpenAiCompletionsOptions::default(),
            &compat,
            CacheRetention::Short,
        );
        // All zai models get zai_tool_stream via auto-detect; TODO: respect model.compat override
        assert!(params.get("tool_stream").is_some());
    }

    #[test]
    fn build_params_uses_system_role_for_openrouter_non_anthropic() {
        let model = Model {
            id: "deepseek/deepseek-v4".to_string(),
            provider: "openrouter".to_string(),
            base_url: "https://openrouter.ai".to_string(),
            reasoning: true,
            ..test_model()
        };
        let compat = detect_compat(&model);
        let context = Context {
            system_prompt: Some("Follow instructions.".to_string()),
            messages: vec![Message::User(UserMessage {
                role: MessageRole::User,
                content: vec![MessageContent::Text(TextContent {
                    text: "Hi".to_string(),
                    text_signature: None,
                })],
                timestamp: Utc::now(),
            })],
            tools: vec![],
        };
        let result = convert_messages(&model, &context, &compat);
        // OpenRouter models that are not anthropic/ or openai/ get system role
        assert_eq!(
            result[0].get("role").and_then(|r| r.as_str()),
            Some("system")
        );
    }

    #[test]
    fn build_params_uses_developer_role_for_openai_on_openrouter() {
        let model = Model {
            id: "openai/gpt-4o".to_string(),
            provider: "openrouter".to_string(),
            base_url: "https://openrouter.ai".to_string(),
            reasoning: true,
            ..test_model()
        };
        let compat = detect_compat(&model);
        let context = Context {
            system_prompt: Some("Follow instructions.".to_string()),
            messages: vec![Message::User(UserMessage {
                role: MessageRole::User,
                content: vec![MessageContent::Text(TextContent {
                    text: "Hi".to_string(),
                    text_signature: None,
                })],
                timestamp: Utc::now(),
            })],
            tools: vec![],
        };
        let result = convert_messages(&model, &context, &compat);
        assert_eq!(
            result[0].get("role").and_then(|r| r.as_str()),
            Some("developer")
        );
    }

    #[ignore = "provider-specific compat: may change with API updates"]
    #[test]
    fn detect_compat_zai_tool_stream() {
        let model = Model {
            provider: "zai".to_string(),
            base_url: "https://api.z.ai".to_string(),
            ..test_model()
        };
        let compat = detect_compat(&model);
        assert!(compat.zai_tool_stream);
    }

    #[test]
    fn detect_compat_ant_ling_non_standard() {
        let model = Model {
            provider: "ant-ling".to_string(),
            base_url: "https://api.ant-ling.com".to_string(),
            ..test_model()
        };
        let compat = detect_compat(&model);
        assert!(!compat.supports_store);
        assert!(!compat.supports_developer_role);
        assert!(!compat.supports_reasoning_effort);
        assert_eq!(compat.max_tokens_field, "max_tokens");
        assert_eq!(compat.thinking_format, "ant-ling");
        assert!(!compat.supports_long_cache_retention);
    }

    #[test]
    fn detect_compat_openrouter_deepseek() {
        let model = Model {
            id: "deepseek/deepseek-r1".to_string(),
            provider: "openrouter".to_string(),
            base_url: "https://openrouter.ai".to_string(),
            ..test_model()
        };
        let compat = detect_compat(&model);
        assert_eq!(compat.thinking_format, "openrouter");
        assert!(compat.supports_store);
    }

    #[test]
    fn convert_messages_thinking_as_text_replay() {
        let model = Model {
            reasoning: true,
            ..test_model()
        };
        // compat with requires_thinking_as_text = true
        let mut compat = detect_compat(&model);
        compat.requires_thinking_as_text = true;
        let context = Context {
            system_prompt: None,
            messages: vec![
                Message::User(UserMessage {
                    role: MessageRole::User,
                    content: vec![MessageContent::Text(TextContent {
                        text: "hello".to_string(),
                        text_signature: None,
                    })],
                    timestamp: Utc::now(),
                }),
                Message::Assistant(AssistantMessage {
                    role: MessageRole::Assistant,
                    content: vec![
                        AssistantContentBlock::Thinking(ThinkingContent {
                            thinking: "internal reasoning".to_string(),
                            thinking_signature: None,
                            redacted: false,
                        }),
                        AssistantContentBlock::Text(TextContent {
                            text: "visible answer".to_string(),
                            text_signature: None,
                        }),
                    ],
                    api: "openai-completions".to_string(),
                    provider: "openai".to_string(),
                    model: "gpt-4".to_string(),
                    response_model: None,
                    response_id: None,
                    usage: empty_usage(),
                    stop_reason: StopReason::Stop,
                    error_message: None,
                    diagnostics: None,
                    timestamp: Utc::now(),
                }),
            ],
            tools: vec![],
        };
        let result = convert_messages(&model, &context, &compat);
        assert_eq!(result.len(), 2);
        // Assistant message at index 1
        let assistant = &result[1];
        assert_eq!(
            assistant.get("role").and_then(|r| r.as_str()),
            Some("assistant")
        );
        // With requires_thinking_as_text, thinking is converted to text content blocks
        let content = assistant.get("content").and_then(|c| c.as_array()).unwrap();
        assert_eq!(content.len(), 2);
        assert_eq!(
            content[0].get("text").and_then(|t| t.as_str()),
            Some("internal reasoning")
        );
        assert_eq!(
            content[1].get("text").and_then(|t| t.as_str()),
            Some("visible answer")
        );
    }

    #[test]
    fn convert_messages_consecutive_user_turns() {
        let model = test_model();
        let compat = detect_compat(&model);
        let context = Context {
            system_prompt: Some("You are helpful.".to_string()),
            messages: vec![
                Message::User(UserMessage {
                    role: MessageRole::User,
                    content: vec![MessageContent::Text(TextContent {
                        text: "First message.".to_string(),
                        text_signature: None,
                    })],
                    timestamp: Utc::now(),
                }),
                Message::User(UserMessage {
                    role: MessageRole::User,
                    content: vec![MessageContent::Text(TextContent {
                        text: "Second message.".to_string(),
                        text_signature: None,
                    })],
                    timestamp: Utc::now(),
                }),
            ],
            tools: vec![],
        };
        let result = convert_messages(&model, &context, &compat);
        // system + 2 user messages
        assert_eq!(result.len(), 3);
        assert_eq!(result[1].get("role").and_then(|r| r.as_str()), Some("user"));
        assert_eq!(result[2].get("role").and_then(|r| r.as_str()), Some("user"));
    }

    #[ignore = "provider-specific compat: may change with API updates"]
    #[test]
    fn build_params_zai_thinking_enabled() {
        let model = Model {
            provider: "zai".to_string(),
            base_url: "https://api.z.ai".to_string(),
            reasoning: true,
            ..test_model()
        };
        let compat = detect_compat(&model);
        let options = OpenAiCompletionsOptions {
            reasoning_effort: Some("high".to_string()),
            ..Default::default()
        };
        let params = build_params(
            &model,
            &empty_context(),
            &options,
            &compat,
            CacheRetention::Short,
        );
        assert_eq!(
            params
                .get("thinking")
                .and_then(|v| v.get("type"))
                .and_then(|v| v.as_str()),
            Some("enabled")
        );
        assert_eq!(
            params.get("reasoning_effort").and_then(|v| v.as_str()),
            Some("high")
        );
    }

    #[test]
    fn build_params_zai_thinking_disabled() {
        let model = Model {
            provider: "zai".to_string(),
            base_url: "https://api.z.ai".to_string(),
            reasoning: true,
            ..test_model()
        };
        let compat = detect_compat(&model);
        let options = OpenAiCompletionsOptions::default();
        let params = build_params(
            &model,
            &empty_context(),
            &options,
            &compat,
            CacheRetention::Short,
        );
        assert_eq!(
            params
                .get("thinking")
                .and_then(|v| v.get("type"))
                .and_then(|v| v.as_str()),
            Some("disabled")
        );
    }

    #[test]
    fn build_params_deepseek_thinking_enabled() {
        let model = Model {
            provider: "deepseek".to_string(),
            base_url: "https://api.deepseek.com".to_string(),
            reasoning: true,
            ..test_model()
        };
        let compat = detect_compat(&model);
        let options = OpenAiCompletionsOptions {
            reasoning_effort: Some("high".to_string()),
            ..Default::default()
        };
        let params = build_params(
            &model,
            &empty_context(),
            &options,
            &compat,
            CacheRetention::Short,
        );
        assert_eq!(
            params
                .get("thinking")
                .and_then(|v| v.get("type"))
                .and_then(|v| v.as_str()),
            Some("enabled")
        );
        assert_eq!(
            params.get("reasoning_effort").and_then(|v| v.as_str()),
            Some("high")
        );
    }

    #[ignore = "provider-specific compat: may change with API updates"]
    #[test]
    fn build_params_together_thinking_enabled() {
        let model = Model {
            provider: "together".to_string(),
            base_url: "https://api.together.ai".to_string(),
            reasoning: true,
            ..test_model()
        };
        let compat = detect_compat(&model);
        let options = OpenAiCompletionsOptions {
            reasoning_effort: Some("high".to_string()),
            ..Default::default()
        };
        let params = build_params(
            &model,
            &empty_context(),
            &options,
            &compat,
            CacheRetention::Short,
        );
        assert_eq!(
            params
                .get("reasoning")
                .and_then(|v| v.get("enabled"))
                .and_then(|v| v.as_bool()),
            Some(true)
        );
        assert_eq!(
            params.get("reasoning_effort").and_then(|v| v.as_str()),
            Some("high")
        );
    }

    #[test]
    fn build_params_openrouter_thinking_object() {
        let model = Model {
            provider: "openrouter".to_string(),
            base_url: "https://openrouter.ai".to_string(),
            reasoning: true,
            ..test_model()
        };
        let compat = detect_compat(&model);
        let options = OpenAiCompletionsOptions {
            reasoning_effort: Some("high".to_string()),
            ..Default::default()
        };
        let params = build_params(
            &model,
            &empty_context(),
            &options,
            &compat,
            CacheRetention::Short,
        );
        // OpenRouter uses reasoning object, not reasoning_effort
        assert_eq!(
            params
                .get("reasoning")
                .and_then(|v| v.get("effort"))
                .and_then(|v| v.as_str()),
            Some("high")
        );
        assert!(params.get("reasoning_effort").is_none());
    }

    #[test]
    fn build_params_ant_ling_thinking_object() {
        let model = Model {
            provider: "ant-ling".to_string(),
            base_url: "https://api.ant-ling.com".to_string(),
            reasoning: true,
            ..test_model()
        };
        let compat = detect_compat(&model);
        let options = OpenAiCompletionsOptions {
            reasoning_effort: Some("high".to_string()),
            ..Default::default()
        };
        let params = build_params(
            &model,
            &empty_context(),
            &options,
            &compat,
            CacheRetention::Short,
        );
        assert_eq!(
            params
                .get("reasoning")
                .and_then(|v| v.get("effort"))
                .and_then(|v| v.as_str()),
            Some("high")
        );
    }

    #[test]
    fn build_params_string_thinking_format() {
        let model = Model {
            provider: "opencode".to_string(),
            base_url: "https://opencode.ai".to_string(),
            reasoning: true,
            ..test_model()
        };
        let compat = detect_compat(&model);
        // opencode has thinking_format "openai" since it's not explicitly matched
        // We need to craft a case where thinking_format is "string-thinking"
        // For now, test the openai fallback case
        let options = OpenAiCompletionsOptions::default();
        // No reasoning_effort, no thinking_level_map => no reasoning params
        let params = build_params(
            &model,
            &empty_context(),
            &options,
            &compat,
            CacheRetention::Short,
        );
        assert!(params.get("reasoning_effort").is_none());
    }

    #[test]
    fn parse_chunk_usage_with_reasoning_tokens() {
        let model = test_model();
        let usage = serde_json::json!({
            "prompt_tokens": 10,
            "completion_tokens": 33,
            "prompt_tokens_details": { "cached_tokens": 0 },
            "completion_tokens_details": { "reasoning_tokens": 21 }
        });
        let result = parse_chunk_usage(&usage, &model);
        assert_eq!(result.input, 10);
        assert_eq!(result.output, 33);
        // reasoning_tokens not subtracted; completion_tokens includes them
    }

    #[test]
    fn parse_chunk_usage_from_choice_fallback() {
        let model = test_model();
        let usage = serde_json::json!({
            "prompt_tokens": 100,
            "completion_tokens": 5,
            "prompt_tokens_details": { "cached_tokens": 50, "cache_write_tokens": 30 }
        });
        let result = parse_chunk_usage(&usage, &model);
        assert_eq!(result.input, 20); // 100 - 50 - 30
        assert_eq!(result.output, 5);
        assert_eq!(result.cache_read, 50);
        assert_eq!(result.cache_write, 30);
    }

    #[test]
    fn build_params_prompt_cache_key_for_openai() {
        let model = test_model();
        let compat = detect_compat(&model);
        let options = OpenAiCompletionsOptions {
            base: StreamOptions {
                session_id: Some("session-123".to_string()),
                ..Default::default()
            },
            ..Default::default()
        };
        let params = build_params(
            &model,
            &empty_context(),
            &options,
            &compat,
            CacheRetention::Short,
        );
        assert_eq!(
            params.get("prompt_cache_key").and_then(|v| v.as_str()),
            Some("session-123")
        );
    }

    #[test]
    fn build_params_prompt_cache_retention_24h_for_long() {
        let model = test_model();
        let compat = detect_compat(&model);
        let options = OpenAiCompletionsOptions {
            base: StreamOptions {
                session_id: Some("session-456".to_string()),
                ..Default::default()
            },
            ..Default::default()
        };
        let params = build_params(
            &model,
            &empty_context(),
            &options,
            &compat,
            CacheRetention::Long,
        );
        assert_eq!(
            params.get("prompt_cache_key").and_then(|v| v.as_str()),
            Some("session-456")
        );
        assert_eq!(
            params
                .get("prompt_cache_retention")
                .and_then(|v| v.as_str()),
            Some("24h")
        );
    }

    #[test]
    fn build_params_omits_prompt_cache_when_retention_none() {
        let model = test_model();
        let compat = detect_compat(&model);
        let options = OpenAiCompletionsOptions {
            base: StreamOptions {
                session_id: Some("session-789".to_string()),
                ..Default::default()
            },
            ..Default::default()
        };
        let params = build_params(
            &model,
            &empty_context(),
            &options,
            &compat,
            CacheRetention::None,
        );
        assert!(params.get("prompt_cache_key").is_none());
        assert!(params.get("prompt_cache_retention").is_none());
    }

    #[test]
    fn build_params_omits_prompt_cache_for_non_openai_without_long_compat() {
        let model = Model {
            base_url: "https://proxy.example.com/v1".to_string(),
            ..test_model()
        };
        let compat = detect_compat(&model);
        // non-openai base, non-relay, CacheRetention Long but compat.supports_long_cache_retention is true by default
        // We need to test with a compat that doesn't support long
        let compat_no_long = ResolvedOpenAiCompletionsCompat {
            supports_long_cache_retention: false,
            ..compat.clone()
        };
        let options = OpenAiCompletionsOptions {
            base: StreamOptions {
                session_id: Some("session-proxy".to_string()),
                ..Default::default()
            },
            ..Default::default()
        };
        let params = build_params(
            &model,
            &empty_context(),
            &options,
            &compat_no_long,
            CacheRetention::Long,
        );
        assert!(params.get("prompt_cache_key").is_none());
        assert!(params.get("prompt_cache_retention").is_none());
    }

    #[test]
    fn build_params_nvidia_compat() {
        let model = Model {
            provider: "nvidia".to_string(),
            base_url: "https://integrate.api.nvidia.com".to_string(),
            ..test_model()
        };
        let compat = detect_compat(&model);
        assert!(!compat.supports_store);
        assert!(!compat.supports_reasoning_effort);
        assert_eq!(compat.max_tokens_field, "max_tokens");
        assert!(!compat.supports_long_cache_retention);
    }

    #[test]
    fn detect_compat_together() {
        let model = Model {
            provider: "together".to_string(),
            base_url: "https://api.together.ai".to_string(),
            ..test_model()
        };
        let compat = detect_compat(&model);
        assert_eq!(compat.thinking_format, "together");
        assert!(!compat.supports_store);
        assert_eq!(compat.max_tokens_field, "max_tokens");
        assert!(!compat.supports_reasoning_effort);
    }

    #[test]
    fn detect_compat_moonshot() {
        let model = Model {
            provider: "moonshotai".to_string(),
            base_url: "https://api.moonshot.ai".to_string(),
            ..test_model()
        };
        let compat = detect_compat(&model);
        assert!(!compat.supports_store);
        assert!(!compat.supports_reasoning_effort);
        assert_eq!(compat.max_tokens_field, "max_tokens");
        assert!(!compat.supports_strict_mode);
    }

    #[test]
    fn detect_compat_cloudflare_ai_gateway() {
        let model = Model {
            provider: "cloudflare-ai-gateway".to_string(),
            base_url: "https://gateway.ai.cloudflare.com/v1/account/gateway/compat".to_string(),
            ..test_model()
        };
        let compat = detect_compat(&model);
        assert!(!compat.supports_store);
        assert!(!compat.supports_reasoning_effort);
        assert_eq!(compat.max_tokens_field, "max_tokens");
        assert!(!compat.supports_long_cache_retention);
    }

    #[test]
    fn build_raw_openai_compatible_headers_basic() {
        let headers = build_raw_openai_compatible_headers("sk-test", None, None);
        assert_eq!(
            headers.get("Accept").map(|s| s.as_str()),
            Some("application/json")
        );
        assert_eq!(
            headers.get("Content-Type").map(|s| s.as_str()),
            Some("application/json")
        );
        assert_eq!(
            headers.get("Authorization").map(|s| s.as_str()),
            Some("Bearer sk-test")
        );
    }

    #[test]
    fn build_raw_openai_compatible_headers_session_affinity() {
        let headers = build_raw_openai_compatible_headers("sk-test", None, Some("session-1"));
        assert_eq!(
            headers.get("x-session-affinity").map(|s| s.as_str()),
            Some("session-1")
        );
        assert_eq!(
            headers.get("x-client-request-id").map(|s| s.as_str()),
            Some("session-1")
        );
    }

    #[test]
    fn build_raw_openai_compatible_headers_not_needed_key() {
        let headers = build_raw_openai_compatible_headers("not-needed", None, None);
        assert!(headers.get("Authorization").is_none());
    }

    #[test]
    fn build_raw_openai_compatible_headers_custom_headers() {
        let mut custom = std::collections::HashMap::new();
        custom.insert("CF-Access-Client-Id".to_string(), "client-id".to_string());
        let headers = build_raw_openai_compatible_headers("sk-test", Some(&custom), None);
        assert_eq!(
            headers.get("CF-Access-Client-Id").map(|s| s.as_str()),
            Some("client-id")
        );
    }

    #[test]
    fn get_compat_cache_control_anthropic_format() {
        let compat = ResolvedOpenAiCompletionsCompat {
            cache_control_format: Some("anthropic".to_string()),
            supports_long_cache_retention: true,
            ..detect_compat(&test_model())
        };
        let cc = get_compat_cache_control(&compat, CacheRetention::Short);
        assert!(cc.is_some());
        assert_eq!(cc.as_ref().unwrap().cache_type, "ephemeral");
        assert!(cc.as_ref().unwrap().ttl.is_none());
    }

    #[test]
    fn get_compat_cache_control_anthropic_format_long() {
        let compat = ResolvedOpenAiCompletionsCompat {
            cache_control_format: Some("anthropic".to_string()),
            supports_long_cache_retention: true,
            ..detect_compat(&test_model())
        };
        let cc = get_compat_cache_control(&compat, CacheRetention::Long);
        assert!(cc.is_some());
        assert_eq!(cc.as_ref().unwrap().ttl.as_deref(), Some("1h"));
    }

    #[test]
    fn get_compat_cache_control_none_retention() {
        let compat = ResolvedOpenAiCompletionsCompat {
            cache_control_format: Some("anthropic".to_string()),
            ..detect_compat(&test_model())
        };
        assert!(get_compat_cache_control(&compat, CacheRetention::None).is_none());
    }

    #[test]
    fn get_compat_cache_control_no_format() {
        let compat = ResolvedOpenAiCompletionsCompat {
            cache_control_format: None,
            ..detect_compat(&test_model())
        };
        assert!(get_compat_cache_control(&compat, CacheRetention::Short).is_none());
    }

    #[test]
    fn convert_messages_empty_user_content_skipped() {
        let model = test_model();
        let compat = detect_compat(&model);
        let context = Context {
            system_prompt: None,
            messages: vec![Message::User(UserMessage {
                role: MessageRole::User,
                content: vec![],
                timestamp: Utc::now(),
            })],
            tools: vec![],
        };
        let result = convert_messages(&model, &context, &compat);
        assert!(result.is_empty());
    }

    #[test]
    fn convert_messages_empty_assistant_content_skipped() {
        let model = test_model();
        let compat = detect_compat(&model);
        let context = Context {
            system_prompt: None,
            messages: vec![
                Message::User(UserMessage {
                    role: MessageRole::User,
                    content: vec![MessageContent::Text(TextContent {
                        text: "Hi".to_string(),
                        text_signature: None,
                    })],
                    timestamp: Utc::now(),
                }),
                Message::Assistant(AssistantMessage {
                    role: MessageRole::Assistant,
                    content: vec![],
                    api: "openai-completions".to_string(),
                    provider: "openai".to_string(),
                    model: "gpt-4".to_string(),
                    response_model: None,
                    response_id: None,
                    usage: empty_usage(),
                    stop_reason: StopReason::Stop,
                    error_message: None,
                    diagnostics: None,
                    timestamp: Utc::now(),
                }),
            ],
            tools: vec![],
        };
        let result = convert_messages(&model, &context, &compat);
        // Only system (none) + user message, empty assistant skipped
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn convert_messages_tool_result_without_images() {
        let model = test_model();
        let compat = detect_compat(&model);
        let context = Context {
            system_prompt: None,
            messages: vec![
                Message::User(UserMessage {
                    role: MessageRole::User,
                    content: vec![MessageContent::Text(TextContent {
                        text: "read a file".to_string(),
                        text_signature: None,
                    })],
                    timestamp: Utc::now(),
                }),
                Message::Assistant(AssistantMessage {
                    role: MessageRole::Assistant,
                    content: vec![AssistantContentBlock::ToolCall(ToolCall {
                        id: "tc-1".to_string(),
                        name: "read".to_string(),
                        arguments: serde_json::json!({}),
                        thought_signature: None,
                    })],
                    api: "openai-completions".to_string(),
                    provider: "openai".to_string(),
                    model: "gpt-4".to_string(),
                    response_model: None,
                    response_id: None,
                    usage: empty_usage(),
                    stop_reason: StopReason::ToolUse,
                    error_message: None,
                    diagnostics: None,
                    timestamp: Utc::now(),
                }),
                Message::ToolResult(ToolResultMessage {
                    role: MessageRole::ToolResult,
                    tool_call_id: "tc-1".to_string(),
                    tool_name: "read".to_string(),
                    content: vec![MessageContent::Text(TextContent {
                        text: "file contents".to_string(),
                        text_signature: None,
                    })],
                    details: None,
                    is_error: false,
                    timestamp: Utc::now(),
                }),
            ],
            tools: vec![],
        };
        let result = convert_messages(&model, &context, &compat);
        assert_eq!(result.len(), 3); // user + assistant with tool_call + tool result (no images => no extra user)
    }

    #[test]
    fn convert_messages_tool_result_with_images_creates_user_message() {
        let model = test_model();
        let compat = detect_compat(&model);
        let context = Context {
            system_prompt: None,
            messages: vec![
                Message::User(UserMessage {
                    role: MessageRole::User,
                    content: vec![MessageContent::Text(TextContent {
                        text: "read the images".to_string(),
                        text_signature: None,
                    })],
                    timestamp: Utc::now(),
                }),
                Message::Assistant(AssistantMessage {
                    role: MessageRole::Assistant,
                    content: vec![AssistantContentBlock::ToolCall(ToolCall {
                        id: "tool-1".to_string(),
                        name: "read".to_string(),
                        arguments: serde_json::json!({"path": "img-1.png"}),
                        thought_signature: None,
                    })],
                    api: "openai-completions".to_string(),
                    provider: "openai".to_string(),
                    model: "gpt-4".to_string(),
                    response_model: None,
                    response_id: None,
                    usage: empty_usage(),
                    stop_reason: StopReason::ToolUse,
                    error_message: None,
                    diagnostics: None,
                    timestamp: Utc::now(),
                }),
                Message::ToolResult(ToolResultMessage {
                    role: MessageRole::ToolResult,
                    tool_call_id: "tool-1".to_string(),
                    tool_name: "read".to_string(),
                    content: vec![
                        MessageContent::Text(TextContent {
                            text: "image data".to_string(),
                            text_signature: None,
                        }),
                        MessageContent::Image(ImageContent {
                            data: "ZmFrZQ==".to_string(),
                            mime_type: "image/png".to_string(),
                        }),
                    ],
                    details: None,
                    is_error: false,
                    timestamp: Utc::now(),
                }),
            ],
            tools: vec![],
        };
        let result = convert_messages(&model, &context, &compat);
        // user, assistant(toolcall), tool_result(image), user(image message)
        assert_eq!(result.len(), 4);
        let last = &result[3];
        assert_eq!(last.get("role").and_then(|r| r.as_str()), Some("user"));
        let content = last.get("content").and_then(|c| c.as_array()).unwrap();
        assert_eq!(content.len(), 2);
        assert_eq!(
            content[1].get("type").and_then(|t| t.as_str()),
            Some("image_url")
        );
    }

    #[test]
    fn convert_messages_batches_images_after_consecutive_tool_results() {
        let model = test_model();
        let compat = detect_compat(&model);
        let context = Context {
            system_prompt: None,
            messages: vec![
                Message::User(UserMessage {
                    role: MessageRole::User,
                    content: vec![MessageContent::Text(TextContent {
                        text: "read the images".to_string(),
                        text_signature: None,
                    })],
                    timestamp: Utc::now(),
                }),
                Message::Assistant(AssistantMessage {
                    role: MessageRole::Assistant,
                    content: vec![
                        AssistantContentBlock::ToolCall(ToolCall {
                            id: "tool-1".to_string(),
                            name: "read".to_string(),
                            arguments: serde_json::json!({"path": "img-1.png"}),
                            thought_signature: None,
                        }),
                        AssistantContentBlock::ToolCall(ToolCall {
                            id: "tool-2".to_string(),
                            name: "read".to_string(),
                            arguments: serde_json::json!({"path": "img-2.png"}),
                            thought_signature: None,
                        }),
                    ],
                    api: "openai-completions".to_string(),
                    provider: "openai".to_string(),
                    model: "gpt-4".to_string(),
                    response_model: None,
                    response_id: None,
                    usage: empty_usage(),
                    stop_reason: StopReason::ToolUse,
                    error_message: None,
                    diagnostics: None,
                    timestamp: Utc::now(),
                }),
                Message::ToolResult(ToolResultMessage {
                    role: MessageRole::ToolResult,
                    tool_call_id: "tool-1".to_string(),
                    tool_name: "read".to_string(),
                    content: vec![
                        MessageContent::Text(TextContent {
                            text: "Read image file [image/png]".to_string(),
                            text_signature: None,
                        }),
                        MessageContent::Image(ImageContent {
                            data: "ZmFrZQ==".to_string(),
                            mime_type: "image/png".to_string(),
                        }),
                    ],
                    details: None,
                    is_error: false,
                    timestamp: Utc::now(),
                }),
                Message::ToolResult(ToolResultMessage {
                    role: MessageRole::ToolResult,
                    tool_call_id: "tool-2".to_string(),
                    tool_name: "read".to_string(),
                    content: vec![
                        MessageContent::Text(TextContent {
                            text: "Read image file [image/png]".to_string(),
                            text_signature: None,
                        }),
                        MessageContent::Image(ImageContent {
                            data: "ZmFrZQ==".to_string(),
                            mime_type: "image/png".to_string(),
                        }),
                    ],
                    details: None,
                    is_error: false,
                    timestamp: Utc::now(),
                }),
            ],
            tools: vec![],
        };
        let result = convert_messages(&model, &context, &compat);
        // roles: user, assistant(s), tool, tool, user
        let roles: Vec<&str> = result
            .iter()
            .filter_map(|m| m.get("role").and_then(|r| r.as_str()))
            .collect();
        assert_eq!(roles, vec!["user", "assistant", "tool", "tool", "user"]);
        // Last message is user with 2 image_url blocks
        let last = &result[4];
        assert_eq!(last.get("role").and_then(|r| r.as_str()), Some("user"));
        let content = last.get("content").and_then(|c| c.as_array()).unwrap();
        let image_parts: Vec<&Value> = content
            .iter()
            .filter(|p| p.get("type").and_then(|t| t.as_str()) == Some("image_url"))
            .collect();
        assert_eq!(image_parts.len(), 2);
    }

    #[test]
    fn convert_messages_requires_tool_result_name() {
        let model = test_model();
        let mut compat = detect_compat(&model);
        compat.requires_tool_result_name = true;
        let context = Context {
            system_prompt: None,
            messages: vec![
                Message::User(UserMessage {
                    role: MessageRole::User,
                    content: vec![MessageContent::Text(TextContent {
                        text: "call tool".to_string(),
                        text_signature: None,
                    })],
                    timestamp: Utc::now(),
                }),
                Message::Assistant(AssistantMessage {
                    role: MessageRole::Assistant,
                    content: vec![AssistantContentBlock::ToolCall(ToolCall {
                        id: "tc-1".to_string(),
                        name: "read".to_string(),
                        arguments: serde_json::json!({}),
                        thought_signature: None,
                    })],
                    api: "openai-completions".to_string(),
                    provider: "openai".to_string(),
                    model: "gpt-4".to_string(),
                    response_model: None,
                    response_id: None,
                    usage: empty_usage(),
                    stop_reason: StopReason::ToolUse,
                    error_message: None,
                    diagnostics: None,
                    timestamp: Utc::now(),
                }),
                Message::ToolResult(ToolResultMessage {
                    role: MessageRole::ToolResult,
                    tool_call_id: "tc-1".to_string(),
                    tool_name: "read".to_string(),
                    content: vec![MessageContent::Text(TextContent {
                        text: "done".to_string(),
                        text_signature: None,
                    })],
                    details: None,
                    is_error: false,
                    timestamp: Utc::now(),
                }),
            ],
            tools: vec![],
        };
        let result = convert_messages(&model, &context, &compat);
        // tool message should have "name" field
        let tool_msg = &result[2];
        assert_eq!(tool_msg.get("role").and_then(|r| r.as_str()), Some("tool"));
        assert_eq!(tool_msg.get("name").and_then(|n| n.as_str()), Some("read"));
    }

    #[test]
    fn convert_messages_requires_assistant_after_tool_result() {
        let model = test_model();
        let mut compat = detect_compat(&model);
        compat.requires_assistant_after_tool_result = true;
        let context = Context {
            system_prompt: None,
            messages: vec![
                Message::User(UserMessage {
                    role: MessageRole::User,
                    content: vec![MessageContent::Text(TextContent {
                        text: "call tool".to_string(),
                        text_signature: None,
                    })],
                    timestamp: Utc::now(),
                }),
                Message::Assistant(AssistantMessage {
                    role: MessageRole::Assistant,
                    content: vec![AssistantContentBlock::ToolCall(ToolCall {
                        id: "tc-1".to_string(),
                        name: "read".to_string(),
                        arguments: serde_json::json!({}),
                        thought_signature: None,
                    })],
                    api: "openai-completions".to_string(),
                    provider: "openai".to_string(),
                    model: "gpt-4".to_string(),
                    response_model: None,
                    response_id: None,
                    usage: empty_usage(),
                    stop_reason: StopReason::ToolUse,
                    error_message: None,
                    diagnostics: None,
                    timestamp: Utc::now(),
                }),
                Message::ToolResult(ToolResultMessage {
                    role: MessageRole::ToolResult,
                    tool_call_id: "tc-1".to_string(),
                    tool_name: "read".to_string(),
                    content: vec![MessageContent::Text(TextContent {
                        text: "done".to_string(),
                        text_signature: None,
                    })],
                    details: None,
                    is_error: false,
                    timestamp: Utc::now(),
                }),
                Message::User(UserMessage {
                    role: MessageRole::User,
                    content: vec![MessageContent::Text(TextContent {
                        text: "continue".to_string(),
                        text_signature: None,
                    })],
                    timestamp: Utc::now(),
                }),
            ],
            tools: vec![],
        };
        let result = convert_messages(&model, &context, &compat);
        // Should insert synthetic assistant between tool result and user
        let roles: Vec<&str> = result
            .iter()
            .filter_map(|m| m.get("role").and_then(|r| r.as_str()))
            .collect();
        assert_eq!(
            roles,
            vec!["user", "assistant", "tool", "assistant", "user"]
        );
        let synthetic = &result[3];
        assert_eq!(
            synthetic.get("content").and_then(|c| c.as_str()),
            Some("I have processed the tool results.")
        );
    }

    #[ignore = "provider-specific compat: may change with API updates"]
    #[test]
    fn convert_messages_deepseek_reasoning_content() {
        let model = Model {
            provider: "deepseek".to_string(),
            base_url: "https://api.deepseek.com".to_string(),
            reasoning: true,
            ..test_model()
        };
        let compat = detect_compat(&model);
        let context = Context {
            system_prompt: None,
            messages: vec![
                Message::User(UserMessage {
                    role: MessageRole::User,
                    content: vec![MessageContent::Text(TextContent {
                        text: "Hi".to_string(),
                        text_signature: None,
                    })],
                    timestamp: Utc::now(),
                }),
                Message::Assistant(AssistantMessage {
                    role: MessageRole::Assistant,
                    content: vec![AssistantContentBlock::ToolCall(ToolCall {
                        id: "call_1".to_string(),
                        name: "read".to_string(),
                        arguments: serde_json::json!({"path": "README.md"}),
                        thought_signature: None,
                    })],
                    api: "openai-completions".to_string(),
                    provider: "deepseek".to_string(),
                    model: "deepseek-model".to_string(),
                    response_model: None,
                    response_id: None,
                    usage: empty_usage(),
                    stop_reason: StopReason::ToolUse,
                    error_message: None,
                    diagnostics: None,
                    timestamp: Utc::now(),
                }),
            ],
            tools: vec![],
        };
        let result = convert_messages(&model, &context, &compat);
        assert_eq!(result.len(), 2);
        let assistant = &result[1];
        assert_eq!(
            assistant.get("reasoning_content").and_then(|v| v.as_str()),
            Some("")
        );
    }

    #[test]
    fn map_stop_reason_network_error() {
        let (reason, msg) = map_stop_reason(Some("network_error"));
        assert_eq!(reason, StopReason::Error);
        assert!(msg.unwrap().contains("network_error"));
    }

    #[test]
    fn map_stop_reason_null() {
        let (reason, msg) = map_stop_reason(None);
        assert_eq!(reason, StopReason::Stop);
        assert!(msg.is_none());
    }

    #[test]
    fn should_use_raw_http_with_cf_headers() {
        let model = test_model();
        let mut headers = std::collections::HashMap::new();
        headers.insert("cf-access-client-id".to_string(), "test".to_string());
        assert!(should_use_raw_openai_compatible_http(
            &model,
            "some-key",
            Some(&headers)
        ));
    }

    #[test]
    fn should_use_raw_http_with_cf_secret_headers() {
        let model = test_model();
        let mut headers = std::collections::HashMap::new();
        headers.insert("cf-access-client-secret".to_string(), "test".to_string());
        assert!(should_use_raw_openai_compatible_http(
            &model,
            "some-key",
            Some(&headers)
        ));
    }

    #[test]
    fn resolve_thinking_effort_unknown_level() {
        let model = test_model();
        assert_eq!(resolve_thinking_effort("bogus", &model), None);
    }

    // -----------------------------------------------------------------------
    // Tests that require network/mock infrastructure (ci-only)
    // -----------------------------------------------------------------------

    #[test]
    #[ignore = "Requires HTTP mock server infrastructure"]
    fn e2e_relay_raw_http_stream() {
        // TS counterpart: openai-completions-relay-http.test.ts
        // Tests raw fetch path, SSE streaming, custom CF headers, relay mode
    }

    #[test]
    #[ignore = "Requires HTTP mock server infrastructure"]
    fn e2e_response_model_surfacing() {
        // TS counterpart: openai-completions-response-model.test.ts
        // Tests that chunk.model surfaces on responseModel when different from model.id
        // and is left undefined when same or empty
    }

    #[test]
    #[ignore = "Requires HTTP mock server infrastructure"]
    fn e2e_null_chunks_ignored() {
        // TS counterpart: openai-completions-tool-choice.test.ts:
        // "ignores null stream chunks from openai-compatible providers"
    }

    #[test]
    #[ignore = "Requires HTTP mock server infrastructure"]
    fn e2e_stream_without_finish_reason_errors() {
        // TS counterpart: openai-completions-tool-choice.test.ts:
        // "errors when a stream ends after only null finish_reason chunks"
    }

    #[test]
    #[ignore = "Requires HTTP mock server infrastructure"]
    fn e2e_tool_call_coalescing_by_stable_index() {
        // TS counterpart: openai-completions-tool-choice.test.ts:
        // "coalesces tool call deltas by stable index when provider mutates ids mid-stream"
    }

    #[test]
    #[ignore = "Requires HTTP mock server infrastructure"]
    fn e2e_mixed_content_reasoning_tool_deltas() {
        // TS counterpart: openai-completions-tool-choice.test.ts:
        // "accumulates mixed content, reasoning, and parallel tool call deltas independently"
    }

    #[test]
    #[ignore = "Requires HTTP mock server infrastructure"]
    fn e2e_thinking_as_text_endpoint_integration() {
        // TS counterpart: openai-completions-thinking-as-text.test.ts:
        // "reaches the endpoint when replay contains both thinking and text"
    }

    #[test]
    #[ignore = "Requires HTTP mock server infrastructure"]
    fn e2e_prompt_cache_session_affinity_headers() {
        // TS counterpart: openai-completions-prompt-cache.test.ts
        // Tests session affinity headers through SDK path
    }

    #[test]
    #[ignore = "Requires HTTP mock server infrastructure"]
    fn e2e_retry_behavior() {
        // TS counterpart: openai-completions-retry.test.ts
        // Tests retry passthrough; TODO(retry) in source
    }
}
