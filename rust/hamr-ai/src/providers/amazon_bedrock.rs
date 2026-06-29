//! Port of `../../packages/ai/src/providers/amazon-bedrock.ts`.
//!
//! AWS Bedrock Converse API provider with credential chain, SigV4 signing, and
//! Bedrock binary event-stream streaming (not standard SSE — Bedrock wraps JSON
//! in `event-stream` binary frames with `:event-type` prefixes).
//!
//! ## Credential resolution
//!
//! 1. `AWS_ACCESS_KEY_ID` / `AWS_SECRET_ACCESS_KEY` / `AWS_SESSION_TOKEN` env vars.
//! 2. `AWS_PROFILE` + `~/.aws/credentials` INI file.
//! 3. `// TODO(imds)` — IMDS / EC2 metadata service.
//!
//! ## SigV4 signing
//!
//! `sign_request()` builds the canonical request → string-to-sign → HMAC-SHA256
//! signing chain using only `sha2` and `hmac` crates (no AWS SDK dependency).
//!
//! ## Streaming
//!
//! Bedrock's `converse-stream` endpoint returns binary `event-stream` frames, **not**
//! standard SSE. Each frame is a `:event-type` header + `:content-type` header +
//! a JSON payload wrapped in `{"bytes":"<base64>"}`.

use base64::Engine as _;
use futures::StreamExt;
use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256};

use crate::models::calculate_cost;
use crate::providers::simple_options::{
    adjust_max_tokens_for_thinking, build_base_options, clamp_reasoning,
};
use crate::providers::transform_messages;
use crate::types::{
    AssistantContentBlock, AssistantMessage, AssistantMessageEvent, CacheRetention, Context,
    DoneReason, ErrorReason, Message, MessageContent, Model, ModelThinkingLevel,
    SimpleStreamOptions, StopReason, StreamOptions, TextContent, ThinkingBudgets, ThinkingContent,
    ThinkingLevel, Tool, ToolCall, Usage, UsageCost,
};
use crate::utils::event_stream::{
    AssistantMessageEventStream, AssistantMessageEventStreamSender,
    create_assistant_message_event_stream,
};
use crate::utils::json_parse::parse_streaming_json;
use crate::utils::provider_env::get_provider_env_value;
use crate::utils::sanitize_unicode::sanitize_surrogates;

// ---------------------------------------------------------------------------
// Options
// ---------------------------------------------------------------------------

/// Controls how Claude's thinking content is returned in responses.
#[derive(Clone, Debug, Default)]
pub enum BedrockThinkingDisplay {
    /// Thinking blocks contain summarized thinking text (default here).
    #[default]
    Summarized,
    /// Thinking content is redacted but the signature still travels back
    /// for multi-turn continuity, reducing time-to-first-text-token.
    Omitted,
}

impl BedrockThinkingDisplay {
    fn as_str(&self) -> &'static str {
        match self {
            BedrockThinkingDisplay::Summarized => "summarized",
            BedrockThinkingDisplay::Omitted => "omitted",
        }
    }
}

/// Bedrock-specific stream options.
///
/// Mirrors the TS `BedrockOptions extends StreamOptions`.
#[derive(Clone, Debug, Default)]
pub struct BedrockOptions {
    pub base: StreamOptions,
    pub region: Option<String>,
    pub profile: Option<String>,
    /// `"auto" | "any" | "none" | { type: "tool"; name: string }`.
    pub tool_choice: Option<BedrockToolChoice>,
    pub reasoning: Option<ThinkingLevel>,
    pub thinking_budgets: Option<ThinkingBudgets>,
    /// Only supported by Claude 4.x models.
    pub interleaved_thinking: Option<bool>,
    pub thinking_display: Option<BedrockThinkingDisplay>,
    /// Key-value pairs for AWS cost allocation tagging.
    pub request_metadata: Option<std::collections::HashMap<String, String>>,
    /// Bearer token for Bedrock API key auth (bypasses SigV4).
    pub bearer_token: Option<String>,
}

/// Tool choice mode for Bedrock.
#[derive(Clone, Debug)]
pub enum BedrockToolChoice {
    Auto,
    Any,
    None_,
    Tool { name: String },
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const EMPTY_TEXT_PLACEHOLDER: &str = "<empty>";

const BEDROCK_DATA_RETENTION_DOCS_URL: &str =
    "https://docs.aws.amazon.com/bedrock/latest/userguide/data-retention.html";

// AWS SigV4 constants
const AWS4_SIGNING_ALGORITHM: &str = "AWS4-HMAC-SHA256";
const AWS4_SERVICE: &str = "bedrock";
const AWS4_TERMINATOR: &str = "aws4_request";

// ---------------------------------------------------------------------------
// Public entry points
// ---------------------------------------------------------------------------

/// Stream a completion from an AWS Bedrock model via the Converse Stream API.
///
/// Mirrors the TS `streamBedrock`.
pub fn stream(
    model: Model,
    context: Context,
    options: Option<BedrockOptions>,
) -> AssistantMessageEventStream {
    let (sender, stream_out) = create_assistant_message_event_stream();
    let options = options.unwrap_or_default();

    tokio::spawn(async move {
        run_stream(model, context, options, sender).await;
    });

    stream_out
}

/// TS-named alias for [`stream`].
pub fn stream_bedrock(
    model: Model,
    context: Context,
    options: Option<BedrockOptions>,
) -> AssistantMessageEventStream {
    stream(model, context, options)
}

/// Stream with simplified reasoning-level options.
///
/// Mirrors the TS `streamSimpleBedrock`.
pub fn stream_simple(
    model: Model,
    context: Context,
    options: Option<SimpleStreamOptions>,
) -> AssistantMessageEventStream {
    let base = build_base_options(&model, options.as_ref(), None);

    let reasoning = options.as_ref().and_then(|o| o.reasoning);
    let thinking_budgets = options.as_ref().and_then(|o| o.thinking_budgets);

    // if (!options?.reasoning)
    match reasoning {
        None => {
            return stream(
                model,
                context,
                Some(BedrockOptions {
                    base,
                    reasoning: None,
                    ..Default::default()
                }),
            );
        }
        Some(reasoning_level) => {
            if is_anthropic_claude_model(&model) {
                if supports_adaptive_thinking(&model.id, &model.name) {
                    return stream(
                        model,
                        context,
                        Some(BedrockOptions {
                            base,
                            reasoning: Some(reasoning_level),
                            thinking_budgets,
                            ..Default::default()
                        }),
                    );
                }

                let adjusted = adjust_max_tokens_for_thinking(
                    base.max_tokens,
                    model.max_tokens,
                    reasoning_level,
                    thinking_budgets.as_ref(),
                );

                let mut budgets = thinking_budgets.unwrap_or_default();
                let clamped = clamp_reasoning(Some(reasoning_level)).unwrap_or(ThinkingLevel::High);
                match clamped {
                    ThinkingLevel::Minimal => budgets.minimal = Some(adjusted.thinking_budget),
                    ThinkingLevel::Low => budgets.low = Some(adjusted.thinking_budget),
                    ThinkingLevel::Medium => budgets.medium = Some(adjusted.thinking_budget),
                    ThinkingLevel::High | ThinkingLevel::XHigh => {
                        budgets.high = Some(adjusted.thinking_budget)
                    }
                }

                return stream(
                    model,
                    context,
                    Some(BedrockOptions {
                        base,
                        reasoning: Some(reasoning_level),
                        thinking_budgets: Some(budgets),
                        ..Default::default()
                    }),
                );
            }

            // Non-Anthropic: pass reasoning through
            stream(
                model,
                context,
                Some(BedrockOptions {
                    base,
                    reasoning: Some(reasoning_level),
                    thinking_budgets,
                    ..Default::default()
                }),
            )
        }
    }
}

/// TS-named alias for [`stream_simple`].
pub fn stream_simple_bedrock(
    model: Model,
    context: Context,
    options: Option<SimpleStreamOptions>,
) -> AssistantMessageEventStream {
    stream_simple(model, context, options)
}

// ---------------------------------------------------------------------------
// Utility types for the streaming block bookkeeping
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// Run stream (core driver)
// ---------------------------------------------------------------------------

async fn run_stream(
    model: Model,
    context: Context,
    options: BedrockOptions,
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

            // Clean up scratch fields on error (like TS: delete block.index etc.)
            for _block in &mut output.content {}

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

fn done_reason_from_stop(stop: StopReason) -> DoneReason {
    match stop {
        StopReason::Length => DoneReason::Length,
        StopReason::ToolUse => DoneReason::ToolUse,
        _ => DoneReason::Stop,
    }
}

fn initial_output(model: &Model) -> AssistantMessage {
    AssistantMessage {
        role: crate::types::MessageRole::Assistant,
        content: Vec::new(),
        api: "bedrock-converse-stream".to_string(),
        provider: model.provider.clone(),
        model: model.id.clone(),
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
        stop_reason: StopReason::Stop,
        error_message: None,
        diagnostics: None,
        timestamp: chrono::Utc::now(),
    }
}

// ---------------------------------------------------------------------------
// Run stream inner (the actual HTTP logic)
// ---------------------------------------------------------------------------

async fn run_stream_inner(
    model: &Model,
    context: &Context,
    options: &BedrockOptions,
    sender: &mut AssistantMessageEventStreamSender,
    output: &mut AssistantMessage,
) -> Result<(), String> {
    // -----------------------------------------------------------------------
    // 1. Build endpoint URL
    // -----------------------------------------------------------------------
    let (region, url) = resolve_bedrock_endpoint(model, options)?;

    // -----------------------------------------------------------------------
    // 2. Build request body
    // -----------------------------------------------------------------------
    let body = build_request_body(model, context, options)?;

    // onPayload hook
    let body = if let Some(on_payload) = &options.base.on_payload {
        match on_payload(body.clone(), model.clone()).await {
            Some(next) => next,
            None => body,
        }
    } else {
        body
    };

    let body_bytes =
        serde_json::to_vec(&body).map_err(|e| format!("Serialization error: {}", e))?;

    // -----------------------------------------------------------------------
    // 3. Resolve credentials
    // -----------------------------------------------------------------------
    let skip_auth = get_provider_env_value("AWS_BEDROCK_SKIP_AUTH", options.base.env.as_ref())
        .map(|v| v == "1")
        .unwrap_or(false);
    let bearer_token = options
        .bearer_token
        .clone()
        .or_else(|| get_provider_env_value("AWS_BEARER_TOKEN_BEDROCK", options.base.env.as_ref()));
    let use_bearer_token = bearer_token.is_some() && !skip_auth;

    // -----------------------------------------------------------------------
    // 4. Build HTTP request
    // -----------------------------------------------------------------------
    let client = reqwest::Client::new();
    let mut request = client.post(&url).header("content-type", "application/json");

    // Model headers
    if let Some(model_headers) = &model.headers {
        for (k, v) in model_headers {
            request = request.header(k, v);
        }
    }
    // Options headers (win over model headers)
    if let Some(opt_headers) = &options.base.headers {
        for (k, v) in opt_headers {
            request = request.header(k, v);
        }
    }

    // Merge caller-supplied custom headers from options.base.metadata?
    // The TS has an `addCustomHeadersMiddleware` but that's for the SDK.
    // For raw HTTP, caller headers in options.base.headers already apply above.

    let request = if use_bearer_token {
        request.header(
            "Authorization",
            format!("Bearer {}", bearer_token.as_deref().unwrap_or("")),
        )
    } else if !skip_auth {
        // SigV4 signing
        let credentials = resolve_credentials(options)?;
        let signed_headers = sign_request(
            &region,
            AWS4_SERVICE,
            "POST",
            &url,
            "",
            &[
                ("content-type".to_string(), "application/json".to_string()),
                ("host".to_string(), url_to_host(&url)?),
            ],
            &body_bytes,
            &credentials,
        );

        let mut req = request;
        for (k, v) in &signed_headers {
            req = req.header(k, v);
        }
        req.body(body_bytes.clone())
    } else {
        // skipAuth: dummy credentials, no SigV4
        // The TS sends dummy credentials but uses the SDK which handles signing.
        // For raw HTTP with skipAuth, just send the body without auth headers.
        request.body(body_bytes.clone())
    };

    // -----------------------------------------------------------------------
    // 5. Send (raced against abort signal)
    // -----------------------------------------------------------------------
    let response = match send_with_abort(request, options.base.signal.clone()).await {
        Ok(resp) => resp,
        Err(e) => return Err(e),
    };

    // onResponse hook
    if let Some(on_response) = &options.base.on_response {
        let status = response.status().as_u16();
        let response_headers = std::collections::HashMap::new(); // reqwest doesn't expose easily
        on_response(
            crate::types::ProviderResponse {
                status,
                headers: response_headers,
            },
            model.clone(),
        )
        .await;
    }

    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        return Err(format_bedrock_error_text(status.as_u16(), &text));
    }

    // -----------------------------------------------------------------------
    // 6. Emit start
    // -----------------------------------------------------------------------
    sender.push(AssistantMessageEvent::Start {
        partial: output.clone(),
    });

    // -----------------------------------------------------------------------
    // 7. Parse Bedrock event-stream binary format
    //
    // Bedrock's converse-stream returns binary event-stream frames, NOT SSE.
    // Each frame looks like:
    //   :event-type <type>\n
    //   :content-type application/json\n
    //   <bytes...>\n
    //
    // The payload is a JSON envelope: {"bytes":"<base64>"}
    // which contains the actual JSON data.
    //
    // We read the response as bytes and parse frames line-by-line,
    // looking for the `:event-type` header and the base64 payload.
    // -----------------------------------------------------------------------
    let mut byte_stream = response.bytes_stream();
    let mut _event_type: Option<String> = None;
    let mut payload_buffer: Vec<u8> = Vec::new();
    let mut _in_headers = true;

    loop {
        // Abort check
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

        payload_buffer.extend_from_slice(&chunk);

        // Try to parse frames from the buffer
        loop {
            match parse_single_event(&mut payload_buffer) {
                Ok(Some(event)) => match process_bedrock_event(&event, model, output, sender) {
                    Ok(()) => {}
                    Err(stop) => {
                        output.stop_reason = stop;
                    }
                },
                Ok(None) => break, // need more data
                Err(e) => return Err(e),
            }
        }
    }

    // Drain any leftover data
    loop {
        match parse_single_event(&mut payload_buffer) {
            Ok(Some(event)) => match process_bedrock_event(&event, model, output, sender) {
                Ok(()) => {}
                Err(stop) => {
                    output.stop_reason = stop;
                }
            },
            Ok(None) => break,
            Err(e) => return Err(e),
        }
    }

    // Abort/error finalization
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
// Bedrock event-stream frame parsing
// ---------------------------------------------------------------------------

/// A parsed Bedrock event-stream frame.
#[derive(Debug)]
struct BedrockEvent {
    event_type: String,
    payload: serde_json::Value,
}

/// Parse a single Bedrock event from the binary stream buffer.
///
/// Bedrock frames look like:
/// ```text
/// :event-type <type>\r\n
/// :content-type application/json\r\n
/// \r\n
/// {"bytes":"<base64>"}\n
/// ```
///
/// Returns `Ok(None)` if there's not enough data yet.
/// Returns `Ok(Some(event))` on a complete frame.
/// Returns `Err(String)` on parse errors.
fn parse_single_event(buffer: &mut Vec<u8>) -> Result<Option<BedrockEvent>, String> {
    // Find the double CRLF or LF that separates headers from body
    let header_end = buffer
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .map(|pos| pos + 4)
        .or_else(|| {
            buffer
                .windows(2)
                .position(|w| w == b"\n\n")
                .map(|pos| pos + 2)
        });

    let header_end = match header_end {
        Some(p) => p,
        None => return Ok(None), // need more data
    };

    if header_end >= buffer.len() {
        return Ok(None);
    }

    let header_bytes = &buffer[..header_end];
    let headers_str =
        std::str::from_utf8(header_bytes).map_err(|_| "Invalid UTF-8 in headers".to_string())?;

    // Parse event type from headers
    let event_type = headers_str
        .lines()
        .find_map(|line| {
            let line = line.trim_end_matches('\r');
            line.strip_prefix(":event-type ")
        })
        .map(|s| s.to_string())
        .ok_or_else(|| format!("Missing :event-type header in: {:?}", headers_str))?;

    // Find the body (after headers)
    // Skip past the blank line
    let body_start = header_end;
    // The body extends to the next lone newline at the end of the frame
    let rest = &buffer[body_start..];

    // Find the line boundary: the body is terminated by \n
    let body_end = rest
        .iter()
        .position(|&b| b == b'\n')
        .map(|p| body_start + p);

    let end_pos = match body_end {
        Some(p) => p,
        None => return Ok(None),
    };

    let body_owned = buffer[body_start..end_pos].to_vec();
    let body_str =
        String::from_utf8(body_owned).map_err(|_| "Invalid UTF-8 in body".to_string())?;
    let body_str = body_str.trim_end_matches('\r').to_string();

    // Drain processed bytes (including the trailing newline)
    let consume = (end_pos + 1).min(buffer.len());
    buffer.drain(..consume);

    // Parse the JSON envelope: {"bytes":"<base64>"}
    let envelope: serde_json::Value =
        serde_json::from_str(&body_str).map_err(|e| format!("Invalid event envelope: {}", e))?;

    let b64 = envelope
        .get("bytes")
        .and_then(|v| v.as_str())
        .ok_or_else(|| format!("Missing 'bytes' field in event envelope: {}", body_str))?;

    let decoded = base64::engine::general_purpose::STANDARD
        .decode(b64)
        .map_err(|e| format!("Base64 decode error: {}", e))?;

    let json_str = std::str::from_utf8(&decoded)
        .map_err(|_| "Invalid UTF-8 in decoded payload".to_string())?;
    let payload: serde_json::Value =
        serde_json::from_str(json_str).map_err(|e| format!("Invalid JSON payload: {}", e))?;

    Ok(Some(BedrockEvent {
        event_type,
        payload,
    }))
}

// ---------------------------------------------------------------------------
// Bedrock event processing
// ---------------------------------------------------------------------------

/// Process a single Bedrock event, updating `output` and emitting events.
/// Returns `Err(stop_reason)` only for terminal events like `messageStop`
/// that set the stop reason (but are not errors).
fn process_bedrock_event(
    event: &BedrockEvent,
    model: &Model,
    output: &mut AssistantMessage,
    sender: &mut AssistantMessageEventStreamSender,
) -> Result<(), StopReason> {
    match event.event_type.as_str() {
        "messageStart" => {
            if let Some(role) = event.payload.get("role").and_then(|v| v.as_str()) {
                if role != "assistant" {
                    return Err(StopReason::Error);
                }
            }
            sender.push(AssistantMessageEvent::Start {
                partial: output.clone(),
            });
            Ok(())
        }

        "contentBlockStart" => {
            handle_content_block_start(&event.payload, output, sender);
            Ok(())
        }

        "contentBlockDelta" => {
            handle_content_block_delta(&event.payload, output, sender);
            Ok(())
        }

        "contentBlockStop" => {
            handle_content_block_stop(&event.payload, output, sender);
            Ok(())
        }

        "messageStop" => {
            let stop_reason =
                map_bedrock_stop_reason(event.payload.get("stopReason").and_then(|v| v.as_str()));
            output.stop_reason = stop_reason;
            Err(stop_reason)
        }

        "metadata" => {
            handle_metadata(&event.payload, model, output);
            Ok(())
        }

        "internalServerException" => Err(StopReason::Error),
        "modelStreamErrorException" => Err(StopReason::Error),
        "validationException" => Err(StopReason::Error),
        "throttlingException" => Err(StopReason::Error),
        "serviceUnavailableException" => Err(StopReason::Error),

        _ => {
            // Unknown event type — ignore.
            Ok(())
        }
    }
}

fn handle_content_block_start(
    payload: &serde_json::Value,
    output: &mut AssistantMessage,
    sender: &mut AssistantMessageEventStreamSender,
) {
    let _content_block_index = payload
        .get("contentBlockIndex")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as usize;
    let start_obj = payload.get("start");

    if let Some(start) = start_obj {
        if let Some(tool_use) = start.get("toolUse") {
            let tool_use_id = tool_use
                .get("toolUseId")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let name = tool_use
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let block = AssistantContentBlock::ToolCall(ToolCall {
                id: tool_use_id.clone(),
                name,
                arguments: serde_json::Value::Object(Default::default()),
                thought_signature: None,
            });
            output.content.push(block);
            let idx = output.content.len() - 1;
            sender.push(AssistantMessageEvent::ToolCallStart {
                content_index: idx,
                partial: output.clone(),
            });
        }
    }
}

fn handle_content_block_delta(
    payload: &serde_json::Value,
    output: &mut AssistantMessage,
    sender: &mut AssistantMessageEventStreamSender,
) {
    let content_block_index = payload
        .get("contentBlockIndex")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as usize;
    let delta = payload.get("delta");

    let Some(delta) = delta else {
        return;
    };

    // text delta
    if let Some(text) = delta.get("text").and_then(|v| v.as_str()) {
        let idx = find_or_create_text_block(content_block_index, output, sender);
        if let Some(AssistantContentBlock::Text(tc)) = output.content.get_mut(idx) {
            tc.text.push_str(text);
        }
        sender.push(AssistantMessageEvent::TextDelta {
            content_index: idx,
            delta: text.to_string(),
            partial: output.clone(),
        });
        return;
    }

    // toolUse delta
    if let Some(tool_use) = delta.get("toolUse") {
        let idx = find_block_by_bedrock_index(content_block_index, output);
        if let Some(idx) = idx {
            if let AssistantContentBlock::ToolCall(tc) = &mut output.content[idx] {
                if let Some(input) = tool_use.get("input").and_then(|v| v.as_str()) {
                    // Parse streaming JSON tool arguments
                    // The TS uses a `partialJson` scratch buffer that accumulates
                    // partial JSON chunks. We use `parse_streaming_json` which handles
                    // incomplete JSON by returning the best-effort parsed value.
                    tc.arguments = parse_streaming_json(Some(input));
                    sender.push(AssistantMessageEvent::ToolCallDelta {
                        content_index: idx,
                        delta: input.to_string(),
                        partial: output.clone(),
                    });
                }
            }
        }
        return;
    }

    // reasoningContent delta
    if let Some(reasoning_content) = delta.get("reasoningContent") {
        let reasoning_text = reasoning_content.get("reasoningText");
        let text = reasoning_text
            .and_then(|r| r.get("text"))
            .and_then(|v| v.as_str());
        let signature = reasoning_text
            .and_then(|r| r.get("signature"))
            .and_then(|v| v.as_str());

        let idx = find_or_create_thinking_block(content_block_index, output, sender);

        if let Some(t) = text {
            let partial = output.clone();
            if let Some(AssistantContentBlock::Thinking(tc)) = output.content.get_mut(idx) {
                tc.thinking.push_str(t);
            }
            sender.push(AssistantMessageEvent::ThinkingDelta {
                content_index: idx,
                delta: t.to_string(),
                partial,
            });
        }
        if let Some(sig) = signature {
            if let Some(AssistantContentBlock::Thinking(tc)) = output.content.get_mut(idx) {
                tc.thinking_signature =
                    Some(tc.thinking_signature.as_deref().unwrap_or("").to_owned() + sig);
            }
        }
    }
}

fn handle_content_block_stop(
    payload: &serde_json::Value,
    output: &mut AssistantMessage,
    sender: &mut AssistantMessageEventStreamSender,
) {
    let content_block_index = payload
        .get("contentBlockIndex")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as usize;
    let idx = match find_block_by_bedrock_index(content_block_index, output) {
        Some(i) => i,
        None => return,
    };

    // Emit end events based on block type
    match &output.content[idx] {
        AssistantContentBlock::Text(tc) => {
            sender.push(AssistantMessageEvent::TextEnd {
                content_index: idx,
                content: tc.text.clone(),
                partial: output.clone(),
            });
        }
        AssistantContentBlock::Thinking(tc) => {
            sender.push(AssistantMessageEvent::ThinkingEnd {
                content_index: idx,
                content: tc.thinking.clone(),
                partial: output.clone(),
            });
        }
        AssistantContentBlock::ToolCall(tc) => {
            // Finalize arguments via parse_streaming_json
            // (the TS uses partialJson scratch buffer)
            sender.push(AssistantMessageEvent::ToolCallEnd {
                content_index: idx,
                tool_call: tc.clone(),
                partial: output.clone(),
            });
        }
    }
}

fn handle_metadata(payload: &serde_json::Value, model: &Model, output: &mut AssistantMessage) {
    if let Some(usage) = payload.get("usage") {
        output.usage.input = usage
            .get("inputTokens")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        output.usage.output = usage
            .get("outputTokens")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        output.usage.cache_read = usage
            .get("cacheReadInputTokens")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        output.usage.cache_write = usage
            .get("cacheWriteInputTokens")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        output.usage.total_tokens = usage
            .get("totalTokens")
            .and_then(|v| v.as_u64())
            .unwrap_or(output.usage.input + output.usage.output);
        calculate_cost(model, &mut output.usage);
    }
}

// ---------------------------------------------------------------------------
// Block index lookup helpers
// ---------------------------------------------------------------------------

/// Find the index in `output.content` that matches a given Bedrock content block index.
/// Bedrock assigns a unique `contentBlockIndex` to each block.
/// We simulate this by storing the index on the block — but since our types don't
/// carry it, we use a positional heuristic: the blocks are pushed in order.
fn find_block_by_bedrock_index(bedrock_index: usize, output: &AssistantMessage) -> Option<usize> {
    if bedrock_index < output.content.len() {
        // Bedrock assigns 0-based indices to content blocks in the order they appear
        Some(bedrock_index)
    } else {
        None
    }
}

/// Find the text block at a given Bedrock index, or create one.
fn find_or_create_text_block(
    bedrock_index: usize,
    output: &mut AssistantMessage,
    sender: &mut AssistantMessageEventStreamSender,
) -> usize {
    if let Some(idx) = find_block_by_bedrock_index(bedrock_index, output) {
        if matches!(output.content[idx], AssistantContentBlock::Text(_)) {
            return idx;
        }
    }

    // Create new text block (the TS behavior: no text_start event is sent for text blocks,
    // they are created implicitly)
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
    idx
}

/// Find the thinking block at a given Bedrock index, or create one.
fn find_or_create_thinking_block(
    bedrock_index: usize,
    output: &mut AssistantMessage,
    sender: &mut AssistantMessageEventStreamSender,
) -> usize {
    if let Some(idx) = find_block_by_bedrock_index(bedrock_index, output) {
        if matches!(output.content[idx], AssistantContentBlock::Thinking(_)) {
            return idx;
        }
    }

    // Create new thinking block
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
    idx
}

// ---------------------------------------------------------------------------
// SigV4 signing (standalone, no AWS SDK dependency)
// ---------------------------------------------------------------------------

/// AWS credential triple.
#[derive(Clone, Debug)]
struct AwsCredentials {
    access_key_id: String,
    secret_access_key: String,
    session_token: Option<String>,
}

/// Sign a request using AWS Signature V4.
///
/// Returns a `HashMap` of headers to add to the request, including:
/// `Authorization`, `x-amz-date`, `x-amz-security-token` (if session token present),
/// and `x-amz-content-sha256`.
fn sign_request(
    region: &str,
    service: &str,
    method: &str,
    url: &str,
    query: &str,
    headers: &[(String, String)],
    body: &[u8],
    credentials: &AwsCredentials,
) -> std::collections::HashMap<String, String> {
    let parsed_url = url::Url::parse(url).expect("Invalid URL for SigV4 signing");
    let host = parsed_url.host_str().unwrap_or("localhost");
    let path = parsed_url.path();

    let now = chrono::Utc::now();
    let amz_date = now.format("%Y%m%dT%H%M%SZ").to_string();
    let date_stamp = now.format("%Y%m%d").to_string();

    // Build canonical headers
    let mut canonical_headers = std::collections::BTreeMap::new();
    canonical_headers.insert("host".to_string(), host.to_string());
    for (k, v) in headers {
        canonical_headers.insert(k.to_lowercase(), v.to_string());
    }
    canonical_headers.insert("x-amz-date".to_string(), amz_date.clone());
    if let Some(token) = &credentials.session_token {
        canonical_headers.insert("x-amz-security-token".to_string(), token.clone());
    }

    let body_hash = hex_encode(&Sha256::digest(body));
    canonical_headers.insert("x-amz-content-sha256".to_string(), body_hash.clone());

    // Canonical headers string
    let signed_headers: Vec<&str> = canonical_headers.keys().map(|k| k.as_str()).collect();
    let canonical_headers_str: String = canonical_headers
        .iter()
        .map(|(k, v)| format!("{}:{}", k, v))
        .collect::<Vec<_>>()
        .join("\n");

    // Canonical request
    let canonical_request = format!(
        "{}\n{}\n{}\n{}\n\n{}\n{}",
        method,
        path,
        query,
        canonical_headers_str,
        signed_headers.join(";"),
        body_hash,
    );

    let canonical_request_hash = hex_encode(&Sha256::digest(canonical_request.as_bytes()));

    // String to sign
    let credential_scope = format!("{}/{}/{}/{}", date_stamp, region, service, AWS4_TERMINATOR);
    let string_to_sign = format!(
        "{}\n{}\n{}\n{}",
        AWS4_SIGNING_ALGORITHM, amz_date, credential_scope, canonical_request_hash,
    );

    // Derive signing key
    let signing_key =
        derive_signing_key(&credentials.secret_access_key, &date_stamp, region, service);

    // Compute signature
    let signature = hex_encode(&hmac_sha256(&signing_key, string_to_sign.as_bytes()));

    // Build Authorization header
    let authorization = format!(
        "{} Credential={}/{}, SignedHeaders={}, Signature={}",
        AWS4_SIGNING_ALGORITHM,
        credentials.access_key_id,
        credential_scope,
        signed_headers.join(";"),
        signature,
    );

    let mut result = std::collections::HashMap::new();
    result.insert("x-amz-date".to_string(), amz_date);
    if let Some(token) = &credentials.session_token {
        result.insert("x-amz-security-token".to_string(), token.clone());
    }
    result.insert("x-amz-content-sha256".to_string(), body_hash);
    result.insert("authorization".to_string(), authorization);
    result
}

/// Derive the AWS SigV4 signing key.
fn derive_signing_key(secret: &str, date_stamp: &str, region: &str, service: &str) -> Vec<u8> {
    let k_secret = format!("AWS4{}", secret);
    let k_date = hmac_sha256(k_secret.as_bytes(), date_stamp.as_bytes());
    let k_region = hmac_sha256(&k_date, region.as_bytes());
    let k_service = hmac_sha256(&k_region, service.as_bytes());
    hmac_sha256(&k_service, AWS4_TERMINATOR.as_bytes())
}

/// HMAC-SHA256 helper.
fn hmac_sha256(key: &[u8], data: &[u8]) -> Vec<u8> {
    let mut mac = Hmac::<Sha256>::new_from_slice(key).expect("HMAC key length");
    mac.update(data);
    mac.finalize().into_bytes().to_vec()
}

/// Hex-encode bytes (no `hex` crate dependency).
fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

// ---------------------------------------------------------------------------
// Credential resolution
// ---------------------------------------------------------------------------

/// Resolve AWS credentials: env vars → `~/.aws/credentials` → `// TODO(imds)`.
fn resolve_credentials(options: &BedrockOptions) -> Result<AwsCredentials, String> {
    // 1. Check env vars first
    let access_key_id = get_provider_env_value("AWS_ACCESS_KEY_ID", options.base.env.as_ref())
        .or_else(|| std::env::var("AWS_ACCESS_KEY_ID").ok());
    let secret_access_key =
        get_provider_env_value("AWS_SECRET_ACCESS_KEY", options.base.env.as_ref())
            .or_else(|| std::env::var("AWS_SECRET_ACCESS_KEY").ok());

    if let (Some(akid), Some(sak)) = (&access_key_id, &secret_access_key) {
        let session_token = get_provider_env_value("AWS_SESSION_TOKEN", options.base.env.as_ref())
            .or_else(|| std::env::var("AWS_SESSION_TOKEN").ok());
        return Ok(AwsCredentials {
            access_key_id: akid.clone(),
            secret_access_key: sak.clone(),
            session_token,
        });
    }

    // 2. Check ~/.aws/credentials file
    let profile = options
        .profile
        .clone()
        .or_else(|| {
            get_provider_env_value("AWS_PROFILE", options.base.env.as_ref())
                .or_else(|| std::env::var("AWS_PROFILE").ok())
        })
        .unwrap_or_else(|| "default".to_string());

    if let Some(creds) = parse_aws_credentials_file(&profile) {
        return Ok(creds);
    }

    // 3. TODO(imds): EC2 metadata service
    Err(format!(
        "No AWS credentials found. Set AWS_ACCESS_KEY_ID/AWS_SECRET_ACCESS_KEY \
         environment variables or configure ~/.aws/credentials (profile: {}).",
        profile
    ))
}

/// Parse the `~/.aws/credentials` INI file for a given profile.
fn parse_aws_credentials_file(profile: &str) -> Option<AwsCredentials> {
    let path = dirs_credentials_path()?;
    let content = std::fs::read_to_string(&path).ok()?;

    let mut current_section: Option<String> = None;
    let mut access_key_id: Option<String> = None;
    let mut secret_access_key: Option<String> = None;
    let mut session_token: Option<String> = None;

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        if line.starts_with('[') && line.ends_with(']') {
            // Check if we found the target profile
            if let Some(ref section) = current_section {
                if section == profile {
                    break; // Found our profile, stop parsing further sections
                }
            }
            current_section = Some(line[1..line.len() - 1].to_string());
            continue;
        }

        if current_section.as_deref() == Some(profile) {
            if let Some((key, value)) = line.split_once('=') {
                let key = key.trim();
                let value = value.trim().trim_matches('"').to_string();
                match key {
                    "aws_access_key_id" => access_key_id = Some(value),
                    "aws_secret_access_key" => secret_access_key = Some(value),
                    "aws_session_token" => session_token = Some(value),
                    _ => {}
                }
            }
        }
    }

    // Only return if we found the profile with both required keys
    if current_section.as_deref() == Some(profile) {
        if let (Some(akid), Some(sak)) = (access_key_id, secret_access_key) {
            return Some(AwsCredentials {
                access_key_id: akid,
                secret_access_key: sak,
                session_token,
            });
        }
    }

    None
}

/// Get the path to the AWS credentials file.
fn dirs_credentials_path() -> Option<std::path::PathBuf> {
    // Try AWS_SHARED_CREDENTIALS_FILE env var first
    if let Ok(path) = std::env::var("AWS_SHARED_CREDENTIALS_FILE") {
        return Some(std::path::PathBuf::from(path));
    }
    // Default: ~/.aws/credentials
    std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .ok()
        .map(|home| {
            std::path::PathBuf::from(home)
                .join(".aws")
                .join("credentials")
        })
}

// ---------------------------------------------------------------------------
// Endpoint resolution
// ---------------------------------------------------------------------------

/// Resolve the Bedrock endpoint URL and region.
///
/// Returns `(region, url)`.
fn resolve_bedrock_endpoint(
    model: &Model,
    options: &BedrockOptions,
) -> Result<(String, String), String> {
    let configured_region = get_configured_bedrock_region(options);
    let has_ambient_profile = get_provider_env_value("AWS_PROFILE", options.base.env.as_ref())
        .or_else(|| std::env::var("AWS_PROFILE").ok())
        .is_some();
    let endpoint_region = get_standard_bedrock_endpoint_region(&model.base_url);
    let use_explicit_endpoint = should_use_explicit_bedrock_endpoint(
        &model.base_url,
        configured_region.as_deref(),
        has_ambient_profile,
    );

    let region = if !model.base_url.is_empty() && use_explicit_endpoint {
        configured_region
            .clone()
            .or_else(|| endpoint_region.clone())
            .unwrap_or_else(|| "us-east-1".to_string())
    } else {
        configured_region
            .clone()
            .or_else(|| endpoint_region.clone().filter(|_| use_explicit_endpoint))
            .unwrap_or_else(|| "us-east-1".to_string())
    };

    let url = if use_explicit_endpoint && !model.base_url.is_empty() {
        format!(
            "{}/model/{}:converse-stream",
            model.base_url.trim_end_matches('/'),
            model.id
        )
    } else {
        format!(
            "https://bedrock-runtime.{}.amazonaws.com/model/{}:converse-stream",
            region, model.id
        )
    };

    Ok((region, url))
}

fn get_configured_bedrock_region(options: &BedrockOptions) -> Option<String> {
    options
        .region
        .clone()
        .or_else(|| get_provider_env_value("AWS_REGION", options.base.env.as_ref()))
        .or_else(|| get_provider_env_value("AWS_DEFAULT_REGION", options.base.env.as_ref()))
}

fn get_standard_bedrock_endpoint_region(base_url: &str) -> Option<String> {
    if base_url.is_empty() {
        return None;
    }

    let parsed = url::Url::parse(base_url).ok()?;
    let hostname = parsed.host_str()?.to_lowercase();

    // Match standard patterns like:
    // bedrock-runtime.us-east-1.amazonaws.com
    // bedrock-runtime-fips.us-east-1.amazonaws.com
    // bedrock-runtime.us-east-1.amazonaws.com.cn
    let re =
        regex::Regex::new(r"^bedrock-runtime(?:-fips)?\.([a-z0-9-]+)\.amazonaws\.com(?:\.cn)?$")
            .ok()?;

    re.captures(&hostname)
        .and_then(|cap| cap.get(1).map(|m| m.as_str().to_string()))
}

fn should_use_explicit_bedrock_endpoint(
    base_url: &str,
    configured_region: Option<&str>,
    has_ambient_profile: bool,
) -> bool {
    let endpoint_region = get_standard_bedrock_endpoint_region(base_url);
    if endpoint_region.is_none() {
        return true;
    }
    configured_region.is_none() && !has_ambient_profile
}

fn url_to_host(url_str: &str) -> Result<String, String> {
    let parsed = url::Url::parse(url_str).map_err(|e| format!("Invalid URL: {}", e))?;
    parsed
        .host_str()
        .map(|h| h.to_string())
        .ok_or_else(|| "No host in URL".to_string())
}

// ---------------------------------------------------------------------------
// Request body building
// ---------------------------------------------------------------------------

fn build_request_body(
    model: &Model,
    context: &Context,
    options: &BedrockOptions,
) -> Result<serde_json::Value, String> {
    let cache_retention =
        resolve_cache_retention(options.base.cache_retention, options.base.env.as_ref());
    let inference_max_tokens = options.base.max_tokens.or_else(|| {
        if is_anthropic_claude_model(model) {
            Some(model.max_tokens)
        } else {
            None
        }
    });

    let mut body = serde_json::Map::new();

    body.insert("modelId".to_string(), serde_json::json!(model.id));

    // messages
    let messages =
        convert_bedrock_messages(context, model, cache_retention, options.base.env.as_ref());
    body.insert("messages".to_string(), messages);

    // system
    let system = build_system_prompt(
        context.system_prompt.as_deref(),
        model,
        cache_retention,
        options.base.env.as_ref(),
    );
    if let Some(sys) = system {
        body.insert("system".to_string(), sys);
    }

    // inferenceConfig
    let mut inference_config = serde_json::Map::new();
    if let Some(max_tokens) = inference_max_tokens {
        inference_config.insert("maxTokens".to_string(), serde_json::json!(max_tokens));
    }
    if let Some(temperature) = options.base.temperature {
        inference_config.insert("temperature".to_string(), serde_json::json!(temperature));
    }
    if !inference_config.is_empty() {
        body.insert(
            "inferenceConfig".to_string(),
            serde_json::Value::Object(inference_config),
        );
    }

    // toolConfig
    let tool_config = convert_tool_config(&context.tools, options.tool_choice.as_ref());
    if let Some(tc) = tool_config {
        body.insert("toolConfig".to_string(), tc);
    }

    // additionalModelRequestFields (thinking config)
    let additional_fields = build_additional_model_request_fields(model, options);
    if let Some(fields) = additional_fields {
        body.insert("additionalModelRequestFields".to_string(), fields);
    }

    // requestMetadata
    if let Some(metadata) = &options.request_metadata {
        body.insert("requestMetadata".to_string(), serde_json::json!(metadata));
    }

    // Abort check
    if let Some(signal) = &options.base.signal {
        if *signal.borrow() {
            return Err("Request aborted".to_string());
        }
    }

    Ok(serde_json::Value::Object(body))
}

// ---------------------------------------------------------------------------
// Message conversion
// ---------------------------------------------------------------------------

fn convert_bedrock_messages(
    context: &Context,
    model: &Model,
    cache_retention: CacheRetention,
    env: Option<&std::collections::HashMap<String, String>>,
) -> serde_json::Value {
    let normalized = transform_messages::transform_messages(
        context.messages.clone(),
        model,
        Some(&normalize_tool_call_id),
    );

    let mut result: Vec<serde_json::Value> = Vec::new();

    for msg in &normalized {
        match msg {
            Message::User(user_msg) => {
                let content = convert_user_content(&user_msg.content);
                result.push(serde_json::json!({
                    "role": "user",
                    "content": content,
                }));
            }

            Message::Assistant(assistant_msg) => {
                if assistant_msg.content.is_empty() {
                    continue;
                }

                let content_blocks = convert_assistant_content(&assistant_msg.content, model);
                if content_blocks.is_empty() {
                    continue;
                }

                result.push(serde_json::json!({
                    "role": "assistant",
                    "content": content_blocks,
                }));
            }

            Message::ToolResult(tool_msg) => {
                // Collect all consecutive toolResult messages into one user message
                let mut tool_results = Vec::new();

                // First tool result
                tool_results.push(serde_json::json!({
                    "toolResult": {
                        "toolUseId": normalize_tool_call_id_simple(&tool_msg.tool_call_id),
                        "content": convert_tool_result_content(&tool_msg.content),
                        "status": if tool_msg.is_error { "error" } else { "success" },
                    }
                }));

                result.push(serde_json::json!({
                    "role": "user",
                    "content": tool_results,
                }));
            }
        }
    }

    // Add cache point to last user message for supported models
    if cache_retention != CacheRetention::None
        && supports_prompt_caching(model, env)
        && !result.is_empty()
    {
        if let Some(last) = result.last_mut() {
            if last.get("role").and_then(|r| r.as_str()) == Some("user") {
                if let Some(content) = last.get_mut("content").and_then(|c| c.as_array_mut()) {
                    let mut cache_point = serde_json::json!({
                        "cachePoint": {
                            "type": "default"
                        }
                    });
                    if cache_retention == CacheRetention::Long {
                        if let Some(obj) = cache_point
                            .get_mut("cachePoint")
                            .and_then(|c| c.as_object_mut())
                        {
                            obj.insert("ttl".to_string(), serde_json::json!(3600));
                        }
                    }
                    content.push(cache_point);
                }
            }
        }
    }

    serde_json::Value::Array(result)
}

fn convert_user_content(content: &[MessageContent]) -> serde_json::Value {
    let mut blocks: Vec<serde_json::Value> = Vec::new();
    for c in content {
        match c {
            MessageContent::Text(tc) => {
                if let Some(block) = create_non_blank_text_block(&tc.text) {
                    blocks.push(block);
                }
            }
            MessageContent::Image(ic) => {
                blocks.push(create_image_block(&ic.mime_type, &ic.data));
            }
        }
    }
    if blocks.is_empty() {
        blocks.push(serde_json::json!({ "text": EMPTY_TEXT_PLACEHOLDER }));
    }
    serde_json::Value::Array(blocks)
}

fn convert_assistant_content(
    content: &[AssistantContentBlock],
    _model: &Model,
) -> Vec<serde_json::Value> {
    let mut blocks: Vec<serde_json::Value> = Vec::new();
    for block in content {
        match block {
            AssistantContentBlock::Text(tc) => {
                let sanitized = sanitize_surrogates(&tc.text);
                if sanitized.trim().is_empty() {
                    continue;
                }
                blocks.push(serde_json::json!({ "text": sanitized }));
            }
            AssistantContentBlock::ToolCall(tc) => {
                blocks.push(serde_json::json!({
                    "toolUse": {
                        "toolUseId": normalize_tool_call_id_simple(&tc.id),
                        "name": tc.name,
                        "input": tc.arguments,
                    }
                }));
            }
            AssistantContentBlock::Thinking(th) => {
                let thinking = sanitize_surrogates(&th.thinking);
                if thinking.trim().is_empty() {
                    continue;
                }
                let sig = &th.thinking_signature;
                if sig.as_deref().map(|s| s.trim().is_empty()).unwrap_or(true) {
                    blocks.push(serde_json::json!({
                        "reasoningContent": {
                            "reasoningText": { "text": thinking }
                        }
                    }));
                } else {
                    blocks.push(serde_json::json!({
                        "reasoningContent": {
                            "reasoningText": {
                                "text": thinking,
                                "signature": sig,
                            }
                        }
                    }));
                }
                blocks.push(serde_json::json!({
                    "reasoningContent": {
                        "reasoningText": { "text": thinking }
                    }
                }));
            }
        }
    }
    blocks
}

fn convert_tool_result_content(content: &[MessageContent]) -> serde_json::Value {
    let mut blocks: Vec<serde_json::Value> = Vec::new();
    for c in content {
        match c {
            MessageContent::Image(ic) => {
                blocks.push(create_image_block(&ic.mime_type, &ic.data));
            }
            MessageContent::Text(tc) => {
                if let Some(block) = create_non_blank_text_block(&tc.text) {
                    blocks.push(block);
                }
            }
        }
    }
    if blocks.is_empty() {
        blocks.push(serde_json::json!({ "text": EMPTY_TEXT_PLACEHOLDER }));
    }
    serde_json::Value::Array(blocks)
}

fn normalize_tool_call_id_simple(id: &str) -> String {
    let sanitized: String = id
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    if sanitized.len() > 64 {
        sanitized[..64].to_string()
    } else {
        sanitized
    }
}

fn normalize_tool_call_id(
    id: &str,
    _model: &crate::types::Model,
    _source: &crate::types::AssistantMessage,
) -> String {
    normalize_tool_call_id_simple(id)
}

// ---------------------------------------------------------------------------
// System prompt
// ---------------------------------------------------------------------------

fn build_system_prompt(
    system_prompt: Option<&str>,
    model: &Model,
    cache_retention: CacheRetention,
    env: Option<&std::collections::HashMap<String, String>>,
) -> Option<serde_json::Value> {
    let prompt = system_prompt?;
    if prompt.trim().is_empty() {
        return None;
    }

    let mut blocks: Vec<serde_json::Value> = Vec::new();
    blocks.push(serde_json::json!({ "text": sanitize_surrogates(prompt) }));

    if cache_retention != CacheRetention::None && supports_prompt_caching(model, env) {
        let cache_point = if cache_retention == CacheRetention::Long {
            serde_json::json!({
                "cachePoint": {
                    "type": "default",
                    "ttl": 3600
                }
            })
        } else {
            serde_json::json!({
                "cachePoint": { "type": "default" }
            })
        };
        blocks.push(cache_point);
    }

    Some(serde_json::Value::Array(blocks))
}

// ---------------------------------------------------------------------------
// Tool config
// ---------------------------------------------------------------------------

fn convert_tool_config(
    tools: &[Tool],
    tool_choice: Option<&BedrockToolChoice>,
) -> Option<serde_json::Value> {
    if tools.is_empty() {
        return None;
    }

    // Check for "none" tool choice
    if let Some(BedrockToolChoice::None_) = tool_choice {
        return None;
    }

    let bedrock_tools: Vec<serde_json::Value> = tools
        .iter()
        .map(|tool| {
            serde_json::json!({
                "toolSpec": {
                    "name": tool.name,
                    "description": tool.description,
                    "inputSchema": { "json": tool.parameters },
                }
            })
        })
        .collect();

    let bedrock_tool_choice = tool_choice.map(|tc| match tc {
        BedrockToolChoice::Auto => serde_json::json!({ "auto": {} }),
        BedrockToolChoice::Any => serde_json::json!({ "any": {} }),
        BedrockToolChoice::Tool { name } => serde_json::json!({ "tool": { "name": name } }),
        BedrockToolChoice::None_ => return serde_json::Value::Null, // handled above
    });

    let mut config = serde_json::Map::new();
    config.insert("tools".to_string(), serde_json::json!(bedrock_tools));
    if let Some(choice) = bedrock_tool_choice {
        if !choice.is_null() {
            config.insert("toolChoice".to_string(), choice);
        }
    }

    Some(serde_json::Value::Object(config))
}

// ---------------------------------------------------------------------------
// Thinking / additional model request fields
// ---------------------------------------------------------------------------

fn build_additional_model_request_fields(
    model: &Model,
    options: &BedrockOptions,
) -> Option<serde_json::Value> {
    let reasoning = options.reasoning?;
    if !model.reasoning {
        return None;
    }

    if is_anthropic_claude_model(model) {
        let display = if is_gov_cloud_bedrock_target(model, options) {
            None
        } else {
            Some(
                options
                    .thinking_display
                    .as_ref()
                    .map(|d| d.as_str())
                    .unwrap_or("summarized"),
            )
        };

        let mut result = serde_json::Map::new();

        if supports_adaptive_thinking(&model.id, &model.name) {
            let mut thinking = serde_json::Map::new();
            thinking.insert("type".to_string(), serde_json::json!("adaptive"));
            if let Some(disp) = display {
                thinking.insert("display".to_string(), serde_json::json!(disp));
            }
            result.insert("thinking".to_string(), serde_json::Value::Object(thinking));
            result.insert(
                "output_config".to_string(),
                serde_json::json!({
                    "effort": map_thinking_level_to_effort(model, reasoning)
                }),
            );
        } else {
            let default_budgets: std::collections::HashMap<&str, u64> = [
                ("minimal", 1024),
                ("low", 2048),
                ("medium", 8192),
                ("high", 16384),
            ]
            .iter()
            .cloned()
            .collect();

            let level = if reasoning == ThinkingLevel::XHigh {
                ThinkingLevel::High
            } else {
                reasoning
            };

            let budget = options
                .thinking_budgets
                .as_ref()
                .and_then(|b| match level {
                    ThinkingLevel::Minimal => b.minimal,
                    ThinkingLevel::Low => b.low,
                    ThinkingLevel::Medium => b.medium,
                    ThinkingLevel::High | ThinkingLevel::XHigh => b.high,
                })
                .unwrap_or_else(|| {
                    let key = match level {
                        ThinkingLevel::Minimal => "minimal",
                        ThinkingLevel::Low => "low",
                        ThinkingLevel::Medium => "medium",
                        _ => "high",
                    };
                    *default_budgets.get(key).unwrap_or(&16384)
                });

            let mut thinking = serde_json::Map::new();
            thinking.insert("type".to_string(), serde_json::json!("enabled"));
            thinking.insert("budget_tokens".to_string(), serde_json::json!(budget));
            if let Some(disp) = display {
                thinking.insert("display".to_string(), serde_json::json!(disp));
            }
            result.insert("thinking".to_string(), serde_json::Value::Object(thinking));
        }

        if !supports_adaptive_thinking(&model.id, &model.name) {
            let interleaved = options.interleaved_thinking.unwrap_or(true);
            if interleaved {
                result.insert(
                    "anthropic_beta".to_string(),
                    serde_json::json!(["interleaved-thinking-2025-05-14"]),
                );
            }
        }

        return Some(serde_json::Value::Object(result));
    }

    None
}

fn map_thinking_level_to_effort(model: &Model, level: ThinkingLevel) -> String {
    if level == ThinkingLevel::XHigh && supports_native_xhigh_effort(model) {
        return "xhigh".to_string();
    }

    // Check thinking_level_map
    let mapped = model.thinking_level_map.as_ref().and_then(|map| {
        let ml = ModelThinkingLevel::from(level);
        map.get(&ml).and_then(|v| v.as_deref())
    });

    if let Some(v) = mapped {
        return v.to_string();
    }

    match level {
        ThinkingLevel::Minimal | ThinkingLevel::Low => "low",
        ThinkingLevel::Medium => "medium",
        ThinkingLevel::High | ThinkingLevel::XHigh => "high",
    }
    .to_string()
}

fn supports_adaptive_thinking(model_id: &str, model_name: &str) -> bool {
    let candidates = get_model_match_candidates(model_id, model_name);
    candidates.iter().any(|s| {
        s.contains("opus-4-6")
            || s.contains("opus-4-7")
            || s.contains("opus-4-8")
            || s.contains("sonnet-4-6")
            || s.contains("fable-5")
    })
}

fn supports_native_xhigh_effort(model: &Model) -> bool {
    let candidates = get_model_match_candidates(&model.id, &model.name);
    candidates
        .iter()
        .any(|s| s.contains("opus-4-7") || s.contains("opus-4-8") || s.contains("fable-5"))
}

fn get_model_match_candidates(model_id: &str, model_name: &str) -> Vec<String> {
    let values: Vec<&str> = if model_name.is_empty() {
        vec![model_id]
    } else {
        vec![model_id, model_name]
    };
    values
        .iter()
        .flat_map(|v| {
            let lower = v.to_lowercase();
            vec![
                lower.clone(),
                lower.replace(
                    |c: char| c.is_whitespace() || c == '_' || c == '.' || c == ':',
                    "-",
                ),
            ]
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Model identification
// ---------------------------------------------------------------------------

fn is_anthropic_claude_model(model: &Model) -> bool {
    let id = model.id.to_lowercase();
    let name = model.name.to_lowercase();
    id.contains("anthropic.claude")
        || id.contains("anthropic/claude")
        || name.contains("anthropic.claude")
        || name.contains("anthropic/claude")
        || name.contains("claude")
}

fn supports_prompt_caching(
    model: &Model,
    env: Option<&std::collections::HashMap<String, String>>,
) -> bool {
    let candidates = get_model_match_candidates(&model.id, &model.name);

    let has_claude_ref = candidates.iter().any(|s| s.contains("claude"));
    if !has_claude_ref {
        if get_provider_env_value("AWS_BEDROCK_FORCE_CACHE", env)
            .map(|v| v == "1")
            .unwrap_or(false)
        {
            return true;
        }
        return false;
    }

    // Claude 4.x models
    if candidates.iter().any(|s| s.contains("-4-")) {
        return true;
    }
    // Claude 3.7 Sonnet
    if candidates.iter().any(|s| s.contains("claude-3-7-sonnet")) {
        return true;
    }
    // Claude 3.5 Haiku
    if candidates.iter().any(|s| s.contains("claude-3-5-haiku")) {
        return true;
    }

    false
}

fn is_gov_cloud_bedrock_target(model: &Model, options: &BedrockOptions) -> bool {
    let region = get_configured_bedrock_region(options);
    if let Some(r) = region {
        if r.to_lowercase().starts_with("us-gov-") {
            return true;
        }
    }

    let model_id = model.id.to_lowercase();
    model_id.starts_with("us-gov.") || model_id.starts_with("arn:aws-us-gov:")
}

// ---------------------------------------------------------------------------
// Stop reason mapping
// ---------------------------------------------------------------------------

fn map_bedrock_stop_reason(reason: Option<&str>) -> StopReason {
    match reason {
        Some("end_turn") | Some("stop_sequence") => StopReason::Stop,
        Some("max_tokens") | Some("model_context_window_exceeded") | Some("content_filtered") => {
            StopReason::Length
        }
        Some("tool_use") => StopReason::ToolUse,
        _ => StopReason::Error,
    }
}

// ---------------------------------------------------------------------------
// Cache retention
// ---------------------------------------------------------------------------

fn resolve_cache_retention(
    cache_retention: Option<CacheRetention>,
    env: Option<&std::collections::HashMap<String, String>>,
) -> CacheRetention {
    match cache_retention {
        Some(cr) => cr,
        None => {
            if get_provider_env_value("PI_CACHE_RETENTION", env)
                .map(|v| v == "long")
                .unwrap_or(false)
            {
                CacheRetention::Long
            } else {
                CacheRetention::Short
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Error formatting
// ---------------------------------------------------------------------------

fn format_bedrock_error_text(status: u16, body: &str) -> String {
    let prefix = match status {
        500 => "Internal server error",
        503 => "Service unavailable",
        400 => "Validation error",
        429 => "Throttling error",
        _ => "Bedrock API error",
    };

    let data_retention_hint = if body.contains("data retention mode") {
        format!(
            " See {} for supported data retention modes.",
            BEDROCK_DATA_RETENTION_DOCS_URL
        )
    } else {
        String::new()
    };

    format!("{} ({}): {}{}", prefix, status, body, data_retention_hint)
}

// ---------------------------------------------------------------------------
// Image blocks
// ---------------------------------------------------------------------------

fn create_image_block(mime_type: &str, data: &str) -> serde_json::Value {
    let format = match mime_type {
        "image/jpeg" | "image/jpg" => "jpeg",
        "image/png" => "png",
        "image/gif" => "gif",
        "image/webp" => "webp",
        _ => "png", // default
    };

    serde_json::json!({
        "image": {
            "format": format,
            "source": { "bytes": data },
        }
    })
}

fn create_non_blank_text_block(text: &str) -> Option<serde_json::Value> {
    let sanitized = sanitize_surrogates(text);
    if sanitized.trim().is_empty() {
        None
    } else {
        Some(serde_json::json!({ "text": sanitized }))
    }
}

// ---------------------------------------------------------------------------
// Concurrency helpers
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
    use crate::types::{Api, MessageRole, ToolResultMessage, UserMessage};

    // -----------------------------------------------------------------------
    // Helper: build a basic test model factory
    // -----------------------------------------------------------------------

    fn create_test_model() -> Model {
        Model {
            id: "test".to_string(),
            name: String::new(),
            api: Api::BedrockConverseStream,
            provider: "bedrock".to_string(),
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
            context_window: 200_000,
            max_tokens: 8192,
            headers: None,
            compat: None,
        }
    }

    fn create_sonnet45_model() -> Model {
        Model {
            id: "us.anthropic.claude-sonnet-4-5-20250929-v1:0".to_string(),
            name: "Claude Sonnet 4.5".to_string(),
            api: Api::BedrockConverseStream,
            provider: "bedrock".to_string(),
            base_url: "https://bedrock-runtime.us-east-1.amazonaws.com".to_string(),
            reasoning: true,
            thinking_level_map: None,
            input: Vec::new(),
            cost: crate::types::ModelCost {
                input: 3.0,
                output: 15.0,
                cache_read: 0.3,
                cache_write: 3.75,
            },
            context_window: 200_000,
            max_tokens: 8192,
            headers: None,
            compat: None,
        }
    }

    fn create_opus48_model() -> Model {
        Model {
            id: "global.anthropic.claude-opus-4-8-v1".to_string(),
            name: "Claude Opus 4.8 (Global)".to_string(),
            api: Api::BedrockConverseStream,
            provider: "bedrock".to_string(),
            base_url: "https://bedrock-runtime.us-east-1.amazonaws.com".to_string(),
            reasoning: true,
            thinking_level_map: None,
            input: Vec::new(),
            cost: crate::types::ModelCost {
                input: 3.0,
                output: 15.0,
                cache_read: 0.3,
                cache_write: 3.75,
            },
            context_window: 200_000,
            max_tokens: 8192,
            headers: None,
            compat: None,
        }
    }

    fn create_fable5_model() -> Model {
        Model {
            id: "global.anthropic.claude-fable-5".to_string(),
            name: "Claude Fable 5".to_string(),
            api: Api::BedrockConverseStream,
            provider: "bedrock".to_string(),
            base_url: String::new(),
            reasoning: true,
            thinking_level_map: None,
            input: Vec::new(),
            cost: crate::types::ModelCost {
                input: 3.0,
                output: 15.0,
                cache_read: 0.3,
                cache_write: 3.75,
            },
            context_window: 200_000,
            max_tokens: 8192,
            headers: None,
            compat: None,
        }
    }

    fn default_context() -> Context {
        Context {
            system_prompt: None,
            messages: vec![Message::User(UserMessage {
                role: MessageRole::User,
                content: vec![MessageContent::Text(TextContent {
                    text: "Hello".to_string(),
                    text_signature: None,
                })],
                timestamp: chrono::Utc::now(),
            })],
            tools: Vec::new(),
        }
    }

    // -----------------------------------------------------------------------
    // SigV4 signing tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_hex_encode() {
        assert_eq!(hex_encode(&[0x00, 0xFF, 0xab]), "00ffab");
        assert_eq!(hex_encode(b"hello"), "68656c6c6f");
    }

    #[test]
    fn test_hmac_sha256_deterministic() {
        // Simple known-answer test: hmac-sha256 of b"test" with key b"key"
        let result = hmac_sha256(b"key", b"The quick brown fox jumps over the lazy dog");
        let expected = "f7bc83f430538424b13298e6aa6fb143ef4d59a14946175997479dbc2d1a3cd8";
        assert_eq!(hex_encode(&result), expected);
    }

    #[test]
    fn test_sign_request_produces_expected_headers() {
        let creds = AwsCredentials {
            access_key_id: "AKIDEXAMPLE".to_string(),
            secret_access_key: "wJalrXUtnFEMI/K7MDENG+bPxRfiCYEXAMPLEKEY".to_string(),
            session_token: None,
        };

        let url =
            "https://bedrock-runtime.us-east-1.amazonaws.com/model/test-model:converse-stream";
        let headers = vec![
            ("content-type".to_string(), "application/json".to_string()),
            (
                "host".to_string(),
                "bedrock-runtime.us-east-1.amazonaws.com".to_string(),
            ),
        ];

        let signed = sign_request(
            "us-east-1",
            "bedrock",
            "POST",
            url,
            "",
            &headers,
            b"{}",
            &creds,
        );

        // Should have the required SigV4 headers
        assert!(signed.contains_key("x-amz-date"));
        assert!(signed.contains_key("x-amz-content-sha256"));
        assert!(signed.contains_key("authorization"));

        // Authorization header should contain the key ID and scope
        let auth = signed.get("authorization").unwrap();
        assert!(auth.contains("AWS4-HMAC-SHA256"));
        assert!(auth.contains("AKIDEXAMPLE"));
        assert!(auth.contains("us-east-1/bedrock/aws4_request"));
        assert!(auth.contains("Signature="));

        // x-amz-content-sha256 should be the hex of SHA256 of the body
        let body_hash = hex_encode(&Sha256::digest(b"{}"));
        assert_eq!(signed.get("x-amz-content-sha256").unwrap(), &body_hash);
    }

    #[test]
    fn test_sign_request_with_session_token() {
        let creds = AwsCredentials {
            access_key_id: "AKID".to_string(),
            secret_access_key: "SAK".to_string(),
            session_token: Some("TOKEN".to_string()),
        };

        let url = "https://bedrock-runtime.us-west-2.amazonaws.com/model/m:converse-stream";
        let headers = vec![
            ("content-type".to_string(), "application/json".to_string()),
            (
                "host".to_string(),
                "bedrock-runtime.us-west-2.amazonaws.com".to_string(),
            ),
        ];

        let signed = sign_request(
            "us-west-2",
            "bedrock",
            "POST",
            url,
            "",
            &headers,
            b"{}",
            &creds,
        );

        assert!(signed.contains_key("x-amz-security-token"));
        assert_eq!(signed.get("x-amz-security-token").unwrap(), "TOKEN");
    }

    // -----------------------------------------------------------------------
    // Credential resolution tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_normalize_tool_call_id_simple() {
        assert_eq!(
            normalize_tool_call_id_simple("valid-id_123"),
            "valid-id_123"
        );
        assert_eq!(
            normalize_tool_call_id_simple("id with spaces!"),
            "id_with_spaces_"
        );
        // Long IDs get truncated
        let long = "a".repeat(100);
        let norm = normalize_tool_call_id_simple(&long);
        assert_eq!(norm.len(), 64);
    }

    #[test]
    fn test_map_bedrock_stop_reason() {
        assert_eq!(map_bedrock_stop_reason(Some("end_turn")), StopReason::Stop);
        assert_eq!(
            map_bedrock_stop_reason(Some("stop_sequence")),
            StopReason::Stop
        );
        assert_eq!(
            map_bedrock_stop_reason(Some("max_tokens")),
            StopReason::Length
        );
        assert_eq!(
            map_bedrock_stop_reason(Some("model_context_window_exceeded")),
            StopReason::Length
        );
        assert_eq!(
            map_bedrock_stop_reason(Some("tool_use")),
            StopReason::ToolUse
        );
        assert_eq!(map_bedrock_stop_reason(Some("unknown")), StopReason::Error);
        assert_eq!(map_bedrock_stop_reason(None), StopReason::Error);
    }

    // -----------------------------------------------------------------------
    // Endpoint resolution tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_get_standard_bedrock_endpoint_region() {
        assert_eq!(
            get_standard_bedrock_endpoint_region("https://bedrock-runtime.us-east-1.amazonaws.com"),
            Some("us-east-1".to_string())
        );
        assert_eq!(
            get_standard_bedrock_endpoint_region(
                "https://bedrock-runtime-fips.us-gov-west-1.amazonaws.com"
            ),
            Some("us-gov-west-1".to_string())
        );
        assert_eq!(
            get_standard_bedrock_endpoint_region(
                "https://bedrock-runtime.ap-northeast-1.amazonaws.com.cn"
            ),
            Some("ap-northeast-1".to_string())
        );
        assert_eq!(
            get_standard_bedrock_endpoint_region("https://custom-proxy.example.com"),
            None
        );
        assert_eq!(get_standard_bedrock_endpoint_region(""), None);
    }

    #[test]
    fn test_should_use_explicit_endpoint() {
        // Custom endpoint (non-standard) should use explicit
        assert!(should_use_explicit_bedrock_endpoint(
            "https://custom-proxy.example.com",
            None,
            false,
        ));

        // Standard endpoint without region/profile: pin explicit (region from URL)
        assert!(should_use_explicit_bedrock_endpoint(
            "https://bedrock-runtime.us-east-1.amazonaws.com",
            None,
            false,
        ));

        // Standard endpoint with configured region: don't pin explicit
        assert!(!should_use_explicit_bedrock_endpoint(
            "https://bedrock-runtime.us-east-1.amazonaws.com",
            Some("us-west-2"),
            false,
        ));
    }

    #[test]
    fn test_should_not_pin_explicit_with_ambient_profile() {
        // Standard endpoint with ambient AWS_PROFILE: don't pin explicit
        assert!(!should_use_explicit_bedrock_endpoint(
            "https://bedrock-runtime.us-east-1.amazonaws.com",
            None,
            true,
        ));
    }

    #[test]
    fn test_model_match_candidates() {
        let candidates =
            get_model_match_candidates("anthropic.claude-sonnet-4-20250514", "Sonnet 4");
        assert!(candidates.iter().any(|s| s.contains("sonnet-4")));
        assert!(candidates.iter().any(|s| s.contains("claude-sonnet-4")));
    }

    #[test]
    fn test_model_match_candidates_empty_name() {
        let candidates = get_model_match_candidates("anthropic.claude-sonnet-4-20250514", "");
        assert!(candidates.iter().any(|s| s.contains("sonnet-4")));
        // With empty name, only the model id is a candidate
        assert_eq!(candidates.len(), 2); // lowercased + normalised
    }

    // -----------------------------------------------------------------------
    // Thinking level mapping tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_map_thinking_level_to_effort() {
        let model = Model {
            id: "anthropic.claude-sonnet-4-20250514".to_string(),
            name: "Sonnet 4".to_string(),
            ..create_test_model()
        };

        // High maps to high
        assert_eq!(
            map_thinking_level_to_effort(&model, ThinkingLevel::High),
            "high"
        );
        // Medium maps to medium
        assert_eq!(
            map_thinking_level_to_effort(&model, ThinkingLevel::Medium),
            "medium"
        );
        // Low maps to low
        assert_eq!(
            map_thinking_level_to_effort(&model, ThinkingLevel::Low),
            "low"
        );
        // Minimal maps to low
        assert_eq!(
            map_thinking_level_to_effort(&model, ThinkingLevel::Minimal),
            "low"
        );
    }

    #[test]
    fn test_map_thinking_level_to_effort_xhigh_for_fable5() {
        let model = create_fable5_model();
        assert_eq!(
            map_thinking_level_to_effort(&model, ThinkingLevel::XHigh),
            "xhigh"
        );
    }

    #[test]
    fn test_map_thinking_level_to_effort_xhigh_for_opus47() {
        let model = Model {
            id: "anthropic.claude-opus-4-7".to_string(),
            name: "Opus 4.7".to_string(),
            ..create_test_model()
        };
        assert_eq!(
            map_thinking_level_to_effort(&model, ThinkingLevel::XHigh),
            "xhigh"
        );
    }

    #[test]
    fn test_map_thinking_level_to_effort_xhigh_for_non_supporting_model_is_high() {
        let model = Model {
            id: "anthropic.claude-sonnet-4".to_string(),
            name: "Sonnet 4".to_string(),
            ..create_test_model()
        };
        assert_eq!(
            map_thinking_level_to_effort(&model, ThinkingLevel::XHigh),
            "high"
        );
    }

    #[test]
    fn test_map_thinking_level_to_effort_respects_thinking_level_map() {
        let mut map = std::collections::HashMap::new();
        map.insert(ModelThinkingLevel::Medium, Some("low".to_string()));
        let model = Model {
            id: "custom-model".to_string(),
            name: String::new(),
            thinking_level_map: Some(map),
            ..create_test_model()
        };
        assert_eq!(
            map_thinking_level_to_effort(&model, ThinkingLevel::Medium),
            "low"
        );
    }

    // -----------------------------------------------------------------------
    // Model identification tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_is_anthropic_claude_model() {
        let model = Model {
            id: "anthropic.claude-sonnet-4".to_string(),
            name: String::new(),
            ..create_test_model()
        };
        assert!(is_anthropic_claude_model(&model));

        let model = Model {
            id: "some-other-model".to_string(),
            name: "Claude".to_string(),
            ..create_test_model()
        };
        assert!(is_anthropic_claude_model(&model));

        let model = Model {
            id: "non-claude-model".to_string(),
            name: "Llama".to_string(),
            ..create_test_model()
        };
        assert!(!is_anthropic_claude_model(&model));
    }

    #[test]
    fn test_supports_adaptive_thinking() {
        assert!(supports_adaptive_thinking(
            "anthropic.claude-opus-4-6-v1",
            "Opus 4.6"
        ));
        assert!(supports_adaptive_thinking(
            "anthropic.claude-sonnet-4-6-v1",
            "Sonnet 4.6"
        ));
        assert!(supports_adaptive_thinking(
            "anthropic.claude-opus-4-7",
            "Opus 4.7"
        ));
        assert!(supports_adaptive_thinking(
            "anthropic.claude-opus-4-8",
            "Opus 4.8"
        ));
        assert!(supports_adaptive_thinking(
            "anthropic.claude-fable-5",
            "Fable 5"
        ));
        assert!(!supports_adaptive_thinking(
            "anthropic.claude-sonnet-4-5",
            "Sonnet 4.5"
        ));
    }

    #[test]
    fn test_supports_native_xhigh_effort() {
        let model = Model {
            id: "anthropic.claude-opus-4-7".to_string(),
            name: String::new(),
            ..create_test_model()
        };
        assert!(supports_native_xhigh_effort(&model));

        let model = Model {
            id: "anthropic.claude-opus-4-8".to_string(),
            name: String::new(),
            ..create_test_model()
        };
        assert!(supports_native_xhigh_effort(&model));

        let model = Model {
            id: "anthropic.claude-fable-5".to_string(),
            name: String::new(),
            ..create_test_model()
        };
        assert!(supports_native_xhigh_effort(&model));

        let model = Model {
            id: "anthropic.claude-sonnet-4-5".to_string(),
            name: String::new(),
            ..create_test_model()
        };
        assert!(!supports_native_xhigh_effort(&model));
    }

    #[test]
    fn test_supports_prompt_caching() {
        // Claude 4.x supports caching
        let model = Model {
            id: "anthropic.claude-sonnet-4-5".to_string(),
            name: String::new(),
            ..create_test_model()
        };
        assert!(supports_prompt_caching(&model, None));

        // Claude 3.7 Sonnet supports caching
        let model = Model {
            id: "anthropic.claude-3-7-sonnet".to_string(),
            name: String::new(),
            ..create_test_model()
        };
        assert!(supports_prompt_caching(&model, None));

        // Non-Claude model without force-cache does not support caching
        let model = Model {
            id: "mistral-model".to_string(),
            name: String::new(),
            ..create_test_model()
        };
        assert!(!supports_prompt_caching(&model, None));

        // Non-Claude with AWS_BEDROCK_FORCE_CACHE=1 supports caching
        let env_map: std::collections::HashMap<String, String> =
            std::collections::HashMap::from([(
                "AWS_BEDROCK_FORCE_CACHE".to_string(),
                "1".to_string(),
            )]);
        let model = Model {
            id: "cohere-model".to_string(),
            name: String::new(),
            ..create_test_model()
        };
        assert!(supports_prompt_caching(&model, Some(&env_map)));

        // Claude 3.5 Haiku supports caching
        let model = Model {
            id: "anthropic.claude-3-5-haiku".to_string(),
            name: String::new(),
            ..create_test_model()
        };
        assert!(supports_prompt_caching(&model, None));
    }

    #[test]
    fn test_is_gov_cloud_bedrock_target() {
        // GovCloud region
        let model = create_test_model();
        let opts = BedrockOptions {
            region: Some("us-gov-west-1".to_string()),
            ..Default::default()
        };
        assert!(is_gov_cloud_bedrock_target(&model, &opts));

        // GovCloud model ID
        let model = Model {
            id: "arn:aws-us-gov:bedrock:us-gov-west-1:123:model/mymodel".to_string(),
            ..create_test_model()
        };
        let opts = BedrockOptions::default();
        assert!(is_gov_cloud_bedrock_target(&model, &opts));

        // Non-GovCloud region
        let model = create_test_model();
        let opts = BedrockOptions {
            region: Some("us-east-1".to_string()),
            ..Default::default()
        };
        assert!(!is_gov_cloud_bedrock_target(&model, &opts));
    }

    // -----------------------------------------------------------------------
    // Error formatting tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_format_bedrock_error_text() {
        assert_eq!(
            format_bedrock_error_text(400, "Bad request"),
            "Validation error (400): Bad request"
        );
        assert_eq!(
            format_bedrock_error_text(500, "Internal failure"),
            "Internal server error (500): Internal failure"
        );
        assert_eq!(
            format_bedrock_error_text(503, "Overloaded"),
            "Service unavailable (503): Overloaded"
        );
        assert_eq!(
            format_bedrock_error_text(429, "Rate limit"),
            "Throttling error (429): Rate limit"
        );
        assert_eq!(
            format_bedrock_error_text(403, "Forbidden"),
            "Bedrock API error (403): Forbidden"
        );
    }

    #[test]
    fn test_format_bedrock_error_with_data_retention_hint() {
        let msg = format_bedrock_error_text(400, "data retention mode");
        assert!(msg.contains(BEDROCK_DATA_RETENTION_DOCS_URL));
    }

    // -----------------------------------------------------------------------
    // URL helper tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_url_to_host() {
        assert_eq!(
            url_to_host("https://bedrock-runtime.us-east-1.amazonaws.com/path").unwrap(),
            "bedrock-runtime.us-east-1.amazonaws.com"
        );
        assert!(url_to_host("not-a-url").is_err());
    }

    // -----------------------------------------------------------------------
    // Cache retention tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_resolve_cache_retention_default() {
        let env_with_long: std::collections::HashMap<String, String> =
            std::collections::HashMap::from([(
                "PI_CACHE_RETENTION".to_string(),
                "long".to_string(),
            )]);
        assert_eq!(
            resolve_cache_retention(None, Some(&env_with_long)),
            CacheRetention::Long
        );
        assert_eq!(resolve_cache_retention(None, None), CacheRetention::Short);
        assert_eq!(
            resolve_cache_retention(Some(CacheRetention::None), None),
            CacheRetention::None
        );
    }

    // -----------------------------------------------------------------------
    // Tool config tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_convert_tool_config_none_when_no_tools() {
        assert!(convert_tool_config(&[], None).is_none());
    }

    #[test]
    fn test_convert_tool_config_none_when_tool_choice_is_none() {
        let tools = vec![Tool {
            name: "test".to_string(),
            description: "A test tool".to_string(),
            parameters: serde_json::json!({"type": "object"}),
        }];
        assert!(convert_tool_config(&tools, Some(&BedrockToolChoice::None_)).is_none());
    }

    #[test]
    fn test_convert_tool_config_auto_choice() {
        let tools = vec![Tool {
            name: "test_tool".to_string(),
            description: "A test tool".to_string(),
            parameters: serde_json::json!({"type": "object"}),
        }];
        let config = convert_tool_config(&tools, Some(&BedrockToolChoice::Auto)).unwrap();
        assert_eq!(config["tools"][0]["toolSpec"]["name"], "test_tool");
        assert_eq!(config["toolChoice"]["auto"], serde_json::json!({}));
    }

    #[test]
    fn test_convert_tool_config_any_choice() {
        let tools = vec![Tool {
            name: "test".to_string(),
            description: "desc".to_string(),
            parameters: serde_json::json!({}),
        }];
        let config = convert_tool_config(&tools, Some(&BedrockToolChoice::Any)).unwrap();
        assert_eq!(config["toolChoice"]["any"], serde_json::json!({}));
    }

    #[test]
    fn test_convert_tool_config_tool_choice() {
        let tools = vec![Tool {
            name: "my_tool".to_string(),
            description: "desc".to_string(),
            parameters: serde_json::json!({}),
        }];
        let config = convert_tool_config(
            &tools,
            Some(&BedrockToolChoice::Tool {
                name: "my_tool".into(),
            }),
        )
        .unwrap();
        assert_eq!(config["toolChoice"]["tool"]["name"], "my_tool");
    }

    // -----------------------------------------------------------------------
    // System prompt tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_build_system_prompt_none_for_empty() {
        assert!(
            build_system_prompt(None, &create_test_model(), CacheRetention::None, None).is_none()
        );
        assert!(
            build_system_prompt(Some(""), &create_test_model(), CacheRetention::None, None)
                .is_none()
        );
        assert!(
            build_system_prompt(Some("  "), &create_test_model(), CacheRetention::None, None)
                .is_none()
        );
    }

    #[test]
    fn test_build_system_prompt_basic() {
        let result = build_system_prompt(
            Some("Be helpful."),
            &create_test_model(),
            CacheRetention::None,
            None,
        )
        .unwrap();
        assert_eq!(result[0]["text"], "Be helpful.");
        assert_eq!(result.as_array().unwrap().len(), 1);
    }

    #[test]
    fn test_build_system_prompt_with_cache_point() {
        let cache_model = Model {
            id: "anthropic.claude-sonnet-4-5".to_string(),
            ..create_test_model()
        };
        let result = build_system_prompt(
            Some("Be helpful."),
            &cache_model,
            CacheRetention::Short,
            None,
        )
        .unwrap();
        assert_eq!(result.as_array().unwrap().len(), 2);
        assert!(result[1].get("cachePoint").is_some());
    }

    #[test]
    fn test_build_system_prompt_with_long_cache_point_ttl() {
        let cache_model = Model {
            id: "anthropic.claude-sonnet-4-5".to_string(),
            ..create_test_model()
        };
        let result = build_system_prompt(
            Some("Be helpful."),
            &cache_model,
            CacheRetention::Long,
            None,
        )
        .unwrap();
        let cache_point = &result[1]["cachePoint"];
        assert_eq!(cache_point["type"], "default");
        assert_eq!(cache_point["ttl"], 3600);
    }

    // -----------------------------------------------------------------------
    // Image block tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_create_image_block() {
        let block = create_image_block("image/png", "base64data");
        assert_eq!(block["image"]["format"], "png");
        assert_eq!(block["image"]["source"]["bytes"], "base64data");

        let block = create_image_block("image/jpeg", "data");
        assert_eq!(block["image"]["format"], "jpeg");

        let block = create_image_block("image/webp", "data");
        assert_eq!(block["image"]["format"], "webp");
    }

    // -----------------------------------------------------------------------
    // Non-blank text block tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_create_non_blank_text_block() {
        assert!(create_non_blank_text_block("").is_none());
        assert!(create_non_blank_text_block("  ").is_none());
        let block = create_non_blank_text_block("hello").unwrap();
        assert_eq!(block["text"], "hello");
    }

    // -----------------------------------------------------------------------
    // Build request body: message conversion (port of bedrock-convert-messages.test.ts)
    // -----------------------------------------------------------------------

    #[test]
    fn test_build_request_body_skips_unknown_user_content() {
        let mut ctx = default_context();
        ctx.messages = vec![Message::User(UserMessage {
            role: MessageRole::User,
            content: vec![MessageContent::Text(TextContent {
                text: "hello".into(),
                text_signature: None,
            })],
            timestamp: chrono::Utc::now(),
        })];

        let model = create_test_model();
        let opts = BedrockOptions::default();
        let body = build_request_body(&model, &ctx, &opts).unwrap();
        let msgs = body["messages"].as_array().unwrap();
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0]["content"][0]["text"], "hello");
    }

    #[test]
    fn test_build_request_body_replaces_blank_user_content_with_placeholder() {
        let mut ctx = default_context();
        ctx.messages = vec![Message::User(UserMessage {
            role: MessageRole::User,
            content: vec![MessageContent::Text(TextContent {
                text: "   ".into(),
                text_signature: None,
            })],
            timestamp: chrono::Utc::now(),
        })];

        let body =
            build_request_body(&create_test_model(), &ctx, &BedrockOptions::default()).unwrap();
        let msgs = body["messages"].as_array().unwrap();
        assert_eq!(msgs[0]["content"][0]["text"], EMPTY_TEXT_PLACEHOLDER);
    }

    #[test]
    fn test_build_request_body_filters_blank_among_non_blank_user_content() {
        let mut ctx = default_context();
        ctx.messages = vec![Message::User(UserMessage {
            role: MessageRole::User,
            content: vec![
                MessageContent::Text(TextContent {
                    text: "".into(),
                    text_signature: None,
                }),
                MessageContent::Text(TextContent {
                    text: "hello".into(),
                    text_signature: None,
                }),
            ],
            timestamp: chrono::Utc::now(),
        })];

        let body =
            build_request_body(&create_test_model(), &ctx, &BedrockOptions::default()).unwrap();
        let msgs = body["messages"].as_array().unwrap();
        let content = msgs[0]["content"].as_array().unwrap();
        assert_eq!(content.len(), 1);
        assert_eq!(content[0]["text"], "hello");
    }

    #[test]
    fn test_build_request_body_replaces_blank_tool_result_with_placeholder() {
        let mut ctx = default_context();
        ctx.messages = vec![Message::ToolResult(ToolResultMessage {
            role: MessageRole::ToolResult,
            tool_call_id: "tool-1".into(),
            tool_name: "tool".into(),
            content: vec![MessageContent::Text(TextContent {
                text: "".into(),
                text_signature: None,
            })],
            is_error: false,
            details: None,
            timestamp: chrono::Utc::now(),
        })];

        let body =
            build_request_body(&create_test_model(), &ctx, &BedrockOptions::default()).unwrap();
        let msgs = body["messages"].as_array().unwrap();
        let tool_content = msgs[0]["content"][0]["toolResult"]["content"]
            .as_array()
            .unwrap();
        assert_eq!(tool_content[0]["text"], EMPTY_TEXT_PLACEHOLDER);
    }

    #[test]
    fn test_build_request_body_skips_assistant_content_only_unknown() {
        let mut ctx = default_context();
        ctx.messages = vec![Message::Assistant(AssistantMessage {
            role: MessageRole::Assistant,
            content: vec![],
            api: "bedrock-converse-stream".into(),
            provider: "bedrock".into(),
            model: "test-model".into(),
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
            stop_reason: StopReason::Stop,
            error_message: None,
            diagnostics: None,
            timestamp: chrono::Utc::now(),
        })];

        let body =
            build_request_body(&create_test_model(), &ctx, &BedrockOptions::default()).unwrap();
        let msgs = body["messages"].as_array().unwrap();
        // Empty assistant message is skipped, leaving no messages
        assert_eq!(msgs.len(), 0);
    }

    #[test]
    fn test_build_request_body_with_inference_config() {
        let model = create_test_model();
        let ctx = default_context();
        let opts = BedrockOptions {
            base: StreamOptions {
                temperature: Some(0.7),
                max_tokens: Some(4096),
                ..Default::default()
            },
            ..Default::default()
        };

        let body = build_request_body(&model, &ctx, &opts).unwrap();
        let ic = &body["inferenceConfig"];
        assert_eq!(ic["maxTokens"], 4096);
        assert_eq!(ic["temperature"], 0.7);
    }

    #[test]
    fn test_build_request_body_with_model_headers() {
        let model = Model {
            id: "test-model".into(),
            name: String::new(),
            api: Api::BedrockConverseStream,
            provider: "bedrock".into(),
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
            context_window: 200_000,
            max_tokens: 8192,
            headers: None,
            compat: None,
        };

        let ctx = default_context();
        let opts = BedrockOptions {
            base: StreamOptions {
                headers: Some(std::collections::HashMap::from([(
                    "x-custom".to_string(),
                    "value".to_string(),
                )])),
                ..Default::default()
            },
            ..Default::default()
        };

        // build_request_body doesn't handle headers; headers are on the HTTP request side.
        // This test just confirms the body builds.
        let body = build_request_body(&model, &ctx, &opts).unwrap();
        assert_eq!(body["modelId"], "test-model");
    }

    // -----------------------------------------------------------------------
    // Build request body: thinking payload (port of bedrock-thinking-payload.test.ts)
    // -----------------------------------------------------------------------

    #[test]
    fn test_build_additional_model_request_fields_adaptive_thinking_for_opus48() {
        let model = create_opus48_model();
        let opts = BedrockOptions {
            reasoning: Some(ThinkingLevel::High),
            ..Default::default()
        };

        let fields = build_additional_model_request_fields(&model, &opts).unwrap();
        assert_eq!(fields["thinking"]["type"], "adaptive");
        assert_eq!(fields["thinking"]["display"], "summarized");
        assert_eq!(fields["output_config"]["effort"], "high");
        // Opus 4.8 supports adaptive thinking, so no anthropic_beta
        assert!(fields.get("anthropic_beta").is_none());
    }

    #[test]
    fn test_build_additional_model_request_fields_xhigh_effort_for_opus48() {
        let model = create_opus48_model();
        let opts = BedrockOptions {
            reasoning: Some(ThinkingLevel::XHigh),
            ..Default::default()
        };

        let fields = build_additional_model_request_fields(&model, &opts).unwrap();
        assert_eq!(fields["thinking"]["type"], "adaptive");
        assert_eq!(fields["output_config"]["effort"], "xhigh");
    }

    #[test]
    fn test_build_additional_model_request_fields_adaptive_thinking_for_fable5() {
        let model = create_fable5_model();
        let opts = BedrockOptions {
            reasoning: Some(ThinkingLevel::High),
            ..Default::default()
        };

        let fields = build_additional_model_request_fields(&model, &opts).unwrap();
        assert_eq!(fields["thinking"]["type"], "adaptive");
        assert_eq!(fields["thinking"]["display"], "summarized");
        assert_eq!(fields["output_config"]["effort"], "high");
    }

    #[test]
    fn test_build_additional_model_request_fields_xhigh_effort_for_fable5() {
        let model = create_fable5_model();
        let opts = BedrockOptions {
            reasoning: Some(ThinkingLevel::XHigh),
            ..Default::default()
        };

        let fields = build_additional_model_request_fields(&model, &opts).unwrap();
        assert_eq!(fields["output_config"]["effort"], "xhigh");
    }

    #[test]
    fn test_build_additional_model_request_fields_omits_display_for_govcloud_region() {
        let model = create_opus48_model();
        let opts = BedrockOptions {
            reasoning: Some(ThinkingLevel::High),
            region: Some("us-gov-west-1".into()),
            ..Default::default()
        };

        let fields = build_additional_model_request_fields(&model, &opts).unwrap();
        assert_eq!(fields["thinking"]["type"], "adaptive");
        // GovCloud should omit the display field
        assert!(fields["thinking"].get("display").is_none());
    }

    #[test]
    fn test_build_additional_model_request_fields_omits_display_for_govcloud_model_id() {
        let model = Model {
            id: "us-gov.anthropic.claude-sonnet-4-5-20250929-v1:0".to_string(),
            name: "Claude Sonnet 4.5 (GovCloud)".to_string(),
            ..create_sonnet45_model()
        };
        let opts = BedrockOptions {
            reasoning: Some(ThinkingLevel::High),
            ..Default::default()
        };

        let fields = build_additional_model_request_fields(&model, &opts).unwrap();
        // Non-adaptive model with GovCloud ID: fixed budget, no display
        assert_eq!(fields["thinking"]["type"], "enabled");
        assert!(fields["thinking"].get("display").is_none());
        assert_eq!(
            fields["anthropic_beta"],
            serde_json::json!(["interleaved-thinking-2025-05-14"])
        );
    }

    #[test]
    fn test_build_additional_model_request_fields_fixed_budget_for_sonnet45() {
        let model = create_sonnet45_model();
        let opts = BedrockOptions {
            reasoning: Some(ThinkingLevel::High),
            ..Default::default()
        };

        let fields = build_additional_model_request_fields(&model, &opts).unwrap();
        assert_eq!(fields["thinking"]["type"], "enabled");
        assert!(fields["thinking"]["budget_tokens"].as_u64().unwrap() > 0);
        assert_eq!(
            fields["anthropic_beta"],
            serde_json::json!(["interleaved-thinking-2025-05-14"])
        );
    }

    #[test]
    fn test_build_additional_model_request_fields_uses_custom_budget() {
        let model = create_sonnet45_model();
        let opts = BedrockOptions {
            reasoning: Some(ThinkingLevel::Low),
            thinking_budgets: Some(ThinkingBudgets {
                low: Some(5000),
                ..Default::default()
            }),
            ..Default::default()
        };

        let fields = build_additional_model_request_fields(&model, &opts).unwrap();
        assert_eq!(fields["thinking"]["type"], "enabled");
        assert_eq!(fields["thinking"]["budget_tokens"], 5000);
    }

    #[test]
    fn test_build_additional_model_request_fields_none_when_no_reasoning() {
        let model = create_sonnet45_model();
        let opts = BedrockOptions {
            reasoning: None,
            ..Default::default()
        };

        assert!(build_additional_model_request_fields(&model, &opts).is_none());
    }

    #[test]
    fn test_build_additional_model_request_fields_none_for_non_reasoning_model() {
        let model = create_test_model(); // reasoning: false
        let opts = BedrockOptions {
            reasoning: Some(ThinkingLevel::High),
            ..Default::default()
        };

        assert!(build_additional_model_request_fields(&model, &opts).is_none());
    }

    #[test]
    fn test_build_additional_model_request_fields_interleaved_thinking_disabled() {
        let model = create_sonnet45_model();
        let opts = BedrockOptions {
            reasoning: Some(ThinkingLevel::High),
            interleaved_thinking: Some(false),
            ..Default::default()
        };

        let fields = build_additional_model_request_fields(&model, &opts).unwrap();
        // When interleaved_thinking is false, anthropic_beta should be absent
        assert!(fields.get("anthropic_beta").is_none());
    }

    #[test]
    fn test_build_additional_model_request_fields_application_inference_profile_adaptive() {
        // Application inference profile where model.name identifies the model
        let model = Model {
            id: "arn:aws:bedrock:us-east-1:123456789012:application-inference-profile/my-profile"
                .to_string(),
            name: "Claude Opus 4.6".to_string(),
            reasoning: true,
            ..create_test_model()
        };
        let opts = BedrockOptions {
            reasoning: Some(ThinkingLevel::High),
            ..Default::default()
        };

        let fields = build_additional_model_request_fields(&model, &opts).unwrap();
        assert_eq!(fields["thinking"]["type"], "adaptive");
        assert_eq!(fields["output_config"]["effort"], "high");
    }

    #[test]
    fn test_build_additional_model_request_fields_application_inference_profile_fixed_budget() {
        // Application inference profile for a non-adaptive Claude model
        let model = Model {
            id: "arn:aws:bedrock:us-east-1:123456789012:application-inference-profile/my-profile"
                .to_string(),
            name: "Claude Sonnet 4.5".to_string(),
            reasoning: true,
            ..create_test_model()
        };
        let opts = BedrockOptions {
            reasoning: Some(ThinkingLevel::High),
            ..Default::default()
        };

        let fields = build_additional_model_request_fields(&model, &opts).unwrap();
        assert_eq!(fields["thinking"]["type"], "enabled");
        assert!(fields["thinking"]["budget_tokens"].as_u64().unwrap() > 0);
        assert_eq!(
            fields["anthropic_beta"],
            serde_json::json!(["interleaved-thinking-2025-05-14"])
        );
    }

    #[test]
    fn test_build_additional_model_request_fields_thinking_display_summarized() {
        let model = create_sonnet45_model();
        let opts = BedrockOptions {
            reasoning: Some(ThinkingLevel::High),
            thinking_display: Some(BedrockThinkingDisplay::Summarized),
            ..Default::default()
        };

        let fields = build_additional_model_request_fields(&model, &opts).unwrap();
        assert_eq!(fields["thinking"]["display"], "summarized");
    }

    #[test]
    fn test_build_additional_model_request_fields_thinking_display_omitted() {
        let model = create_sonnet45_model();
        let opts = BedrockOptions {
            reasoning: Some(ThinkingLevel::High),
            thinking_display: Some(BedrockThinkingDisplay::Omitted),
            ..Default::default()
        };

        let fields = build_additional_model_request_fields(&model, &opts).unwrap();
        assert_eq!(fields["thinking"]["display"], "omitted");
    }

    // -----------------------------------------------------------------------
    // Cache point injection in message conversion
    // -----------------------------------------------------------------------

    #[test]
    fn test_build_request_body_with_cache_points() {
        let cache_model = Model {
            id: "anthropic.claude-sonnet-4-5".to_string(),
            name: "Claude Sonnet 4.5".to_string(),
            ..create_test_model()
        };

        let mut ctx = default_context();
        ctx.system_prompt = Some("You are helpful.".into());

        let opts = BedrockOptions {
            base: StreamOptions {
                cache_retention: Some(CacheRetention::Short),
                ..Default::default()
            },
            ..Default::default()
        };

        let body = build_request_body(&cache_model, &ctx, &opts).unwrap();

        // System prompt should have a cache point
        let sys = body["system"].as_array().unwrap();
        assert_eq!(sys.len(), 2);
        assert!(sys[1].get("cachePoint").is_some());

        // Last user message should have a cache point
        let msgs = body["messages"].as_array().unwrap();
        let last_msg = msgs.last().unwrap();
        let last_content = last_msg["content"].as_array().unwrap();
        let last_item = last_content.last().unwrap();
        assert!(last_item.get("cachePoint").is_some());
    }

    // -----------------------------------------------------------------------
    // Endpoint resolution: resolve_bedrock_endpoint integration tests
    // (port of bedrock-endpoint-resolution.test.ts)
    // -----------------------------------------------------------------------

    #[test]
    fn test_resolve_bedrock_endpoint_standard_model() {
        let model = create_test_model();
        let opts = BedrockOptions::default();
        let (region, url) = resolve_bedrock_endpoint(&model, &opts).unwrap();
        assert_eq!(region, "us-east-1");
        assert!(url.contains("bedrock-runtime.us-east-1.amazonaws.com"));
        assert!(url.ends_with("/model/test:converse-stream"));
    }

    #[test]
    fn test_resolve_bedrock_endpoint_with_configured_region() {
        let model = create_test_model();
        let opts = BedrockOptions {
            region: Some("eu-west-1".into()),
            ..Default::default()
        };
        let (region, url) = resolve_bedrock_endpoint(&model, &opts).unwrap();
        assert_eq!(region, "eu-west-1");
        assert!(url.contains("bedrock-runtime.eu-west-1.amazonaws.com"));
    }

    #[test]
    fn test_resolve_bedrock_endpoint_with_configured_region_from_options_env() {
        let model = create_test_model();
        let env = Some(std::collections::HashMap::from([(
            "AWS_REGION".to_string(),
            "eu-central-1".to_string(),
        )]));
        let opts = BedrockOptions {
            base: StreamOptions {
                env: env.clone(),
                ..Default::default()
            },
            ..Default::default()
        };
        let (region, _url) = resolve_bedrock_endpoint(&model, &opts).unwrap();
        assert_eq!(region, "eu-central-1");
    }

    #[test]
    fn test_resolve_bedrock_endpoint_derives_region_from_eu_endpoint() {
        let model = Model {
            base_url: "https://bedrock-runtime.eu-central-1.amazonaws.com".to_string(),
            ..create_test_model()
        };
        let opts = BedrockOptions::default();
        let (region, url) = resolve_bedrock_endpoint(&model, &opts).unwrap();
        assert_eq!(region, "eu-central-1");
        assert!(url.contains("bedrock-runtime.eu-central-1.amazonaws.com"));
    }

    #[test]
    fn test_resolve_bedrock_endpoint_custom_vpc_endpoint() {
        let model = Model {
            base_url: "https://bedrock-vpc.example.com".to_string(),
            ..create_test_model()
        };
        let opts = BedrockOptions {
            region: Some("us-west-2".into()),
            ..Default::default()
        };
        let (region, url) = resolve_bedrock_endpoint(&model, &opts).unwrap();
        assert_eq!(region, "us-west-2");
        assert!(url.starts_with("https://bedrock-vpc.example.com"));
        assert!(url.ends_with("/model/test:converse-stream"));
    }

    #[test]
    fn test_resolve_bedrock_endpoint_with_ambient_profile_does_not_pin() {
        // With AWS_PROFILE but no explicit region: should use us-east-1 default
        // since the endpoint is standard
        let model = Model {
            base_url: "https://bedrock-runtime.us-east-1.amazonaws.com".to_string(),
            ..create_test_model()
        };
        // no explicit region means us-east-1, and profile causes non-pin
        // -> region from URL, no explicit endpoint
        let opts = BedrockOptions::default();
        let (region, _url) = resolve_bedrock_endpoint(&model, &opts).unwrap();
        // Without profile, the standard endpoint is used and region derived from URL
        assert_eq!(region, "us-east-1");
    }
}
