//! Port of `packages/ai/src/providers/images/openrouter.ts`.
//!
//! OpenRouter image generation via the OpenAI-compatible `/v1/chat/completions`
//! endpoint, with `modalities: ["image"]` to request image output.
//!
//! OpenRouter returns images as a special `images` array on the chat completion
//! choice message. We parse that, extract base64-encoded `data:` URIs, and
//! return [`AssistantImages`] with `ImagesOutputContent::Image` entries.

use std::sync::LazyLock;

use chrono::Utc;
use regex::Regex;
use reqwest::header::{CONTENT_TYPE, HeaderMap, HeaderValue};
use serde::{Deserialize, Serialize};

use crate::images::local_types::{
    AssistantImages, ImagesContext, ImagesInputContent, ImagesModel, ImagesModelCost,
    ImagesOptions, ImagesOutputContent, ImagesStopReason, ImagesUsage, ImagesUsageCost,
};
use crate::utils::headers::headers_to_record;
use crate::utils::sanitize_unicode::sanitize_surrogates;

// ---------------------------------------------------------------------------
// Request types (mirror JSON shapes sent to /v1/chat/completions)
// ---------------------------------------------------------------------------

/// Single message in the OpenAI-format chat request.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct OpenRouterRequestMessage {
    role: String,
    content: Vec<OpenRouterRequestContentPart>,
}

/// Content part within an OpenRouter chat request message.
#[derive(Debug, Serialize)]
#[serde(tag = "type")]
enum OpenRouterRequestContentPart {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "image_url")]
    ImageUrl {
        image_url: OpenRouterRequestImageUrl,
    },
}

/// URL wrapper for image content parts.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct OpenRouterRequestImageUrl {
    url: String,
}

/// The full JSON body for the OpenRouter /v1/chat/completions request.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct OpenRouterRequest {
    model: String,
    messages: Vec<OpenRouterRequestMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stream: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    modalities: Option<Vec<String>>,
}

// ---------------------------------------------------------------------------
// Response types (mirror OpenRouter image generation response shape)
// ---------------------------------------------------------------------------

/// Top-level response from OpenRouter chat completions (with image support).
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OpenRouterImageGenerationResponse {
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    usage: Option<OpenRouterUsage>,
    choices: Vec<OpenRouterImageGenerationChoice>,
}

/// A choice from the image generation response.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OpenRouterImageGenerationChoice {
    #[serde(default)]
    message: OpenRouterImageGenerationMessage,
}

/// The assistant message within a choice, which may have an `images` array.
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct OpenRouterImageGenerationMessage {
    #[serde(default)]
    content: Option<serde_json::Value>,
    #[serde(default)]
    images: Option<Vec<OpenRouterGeneratedImage>>,
}

/// A single generated image from OpenRouter.
#[derive(Debug, Deserialize)]
struct OpenRouterGeneratedImage {
    #[serde(default)]
    image_url: Option<serde_json::Value>,
}

/// Usage data from the OpenRouter response.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OpenRouterUsage {
    #[serde(default)]
    prompt_tokens: Option<u64>,
    #[serde(default)]
    completion_tokens: Option<u64>,
    #[serde(default)]
    prompt_tokens_details: Option<OpenRouterPromptTokensDetails>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OpenRouterPromptTokensDetails {
    #[serde(default)]
    cached_tokens: Option<u64>,
    #[serde(default)]
    cache_write_tokens: Option<u64>,
}

// ---------------------------------------------------------------------------
// Regex for parsing data URIs
// ---------------------------------------------------------------------------

static DATA_URI_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^data:([^;]+);base64,(.+)$").unwrap());

// ---------------------------------------------------------------------------
// Provider implementation
// ---------------------------------------------------------------------------

/// Generate images via OpenRouter's `/v1/chat/completions` endpoint.
///
/// Mirrors the TS [`generateImagesOpenRouter`].
pub async fn generate_images_open_router(
    model: ImagesModel,
    context: ImagesContext,
    options: Option<ImagesOptions>,
) -> Result<AssistantImages, String> {
    let opts = options.unwrap_or_default();

    // Build the output structure early (mirrors TS: populate fields as we go).
    let mut output = AssistantImages {
        api: model.api.clone(),
        provider: model.provider.clone(),
        model: model.id.clone(),
        output: Vec::new(),
        stop_reason: ImagesStopReason::Stop,
        error_message: None,
        response_id: None,
        usage: None,
        timestamp: Utc::now().timestamp_millis(),
    };

    // Resolve API key.
    let api_key = match &opts.api_key {
        Some(k) => k.clone(),
        None => match resolve_api_key(&model, &opts) {
            Some(k) => k,
            None => {
                output.stop_reason = ImagesStopReason::Error;
                output.error_message = Some(format!("No API key for provider: {}", model.provider));
                return Ok(output);
            }
        },
    };

    // Build the HTTP request.
    let client = reqwest::Client::new();
    let base_url = &model.base_url;
    let url = format!("{}/v1/chat/completions", base_url.trim_end_matches('/'));

    let request_body = build_request_json(&model, &context);

    // Allow payload inspection/replacement via on_payload callback.
    let final_body = if let Some(ref on_payload) = opts.on_payload {
        let payload_value = serde_json::to_value(&request_body)
            .map_err(|e| format!("Failed to serialize request: {}", e))?;
        match on_payload(payload_value, model.clone()).await {
            Some(modified) => modified,
            None => serde_json::to_value(&request_body)
                .map_err(|e| format!("Failed to serialize request: {}", e))?,
        }
    } else {
        serde_json::to_value(&request_body)
            .map_err(|e| format!("Failed to serialize request: {}", e))?
    };

    // Build headers.
    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    let auth_value = format!("Bearer {}", api_key);
    headers.insert(
        reqwest::header::AUTHORIZATION,
        HeaderValue::from_str(&auth_value)
            .map_err(|_| "Invalid API key header value".to_string())?,
    );
    // Merge model-level headers.
    if let Some(ref model_headers) = model.headers {
        for (key, value) in model_headers {
            if let (Ok(k), Ok(v)) = (
                reqwest::header::HeaderName::from_bytes(key.as_bytes()),
                HeaderValue::from_str(value),
            ) {
                headers.insert(k, v);
            }
        }
    }
    // Merge per-request headers (override model headers).
    if let Some(ref extra_headers) = opts.headers {
        for (key, value) in extra_headers {
            if let (Ok(k), Ok(v)) = (
                reqwest::header::HeaderName::from_bytes(key.as_bytes()),
                HeaderValue::from_str(value),
            ) {
                headers.insert(k, v);
            }
        }
    }

    // Build the HTTP request.
    let body_json = serde_json::to_string(&final_body)
        .map_err(|e| format!("Failed to serialize request body: {}", e))?;
    let mut request_builder = client.post(&url).headers(headers).body(body_json);

    // Apply timeout if set.
    if let Some(timeout_ms) = opts.timeout_ms {
        request_builder = request_builder.timeout(std::time::Duration::from_millis(timeout_ms));
    }

    // Execute.
    let response = match request_builder.send().await {
        Ok(resp) => resp,
        Err(e) => {
            output.stop_reason = ImagesStopReason::Error;
            output.error_message = Some(format!("HTTP request failed: {}", e));
            return Ok(output);
        }
    };

    let status = response.status().as_u16();
    let response_headers = response.headers().clone();

    // Fire on_response callback.
    if let Some(ref on_response) = opts.on_response {
        let provider_resp = crate::types::ProviderResponse {
            status,
            headers: headers_to_record(&response_headers),
        };
        on_response(provider_resp, model.clone()).await;
    }

    // Read body.
    let body: Vec<u8> = match response.bytes().await {
        Ok(b) => b.to_vec(),
        Err(e) => {
            output.stop_reason = ImagesStopReason::Error;
            output.error_message = Some(format!("Failed to read response body: {}", e));
            return Ok(output);
        }
    };

    // Check for HTTP error status.
    if status >= 400 {
        let error_text = String::from_utf8_lossy(&body);
        output.stop_reason = ImagesStopReason::Error;
        output.error_message = Some(format!("OpenRouter API error ({}): {}", status, error_text));
        return Ok(output);
    }

    // Parse response JSON.
    let image_response: OpenRouterImageGenerationResponse = match serde_json::from_slice(&body) {
        Ok(r) => r,
        Err(e) => {
            output.stop_reason = ImagesStopReason::Error;
            output.error_message = Some(format!("Failed to parse OpenRouter response: {}", e));
            return Ok(output);
        }
    };

    // Populate response metadata.
    output.response_id = image_response.id;

    // Parse usage if present.
    if let Some(usage) = image_response.usage {
        output.usage = Some(parse_usage(usage, &model.cost));
    }

    // Parse choice content.
    if let Some(choice) = image_response.choices.into_iter().next() {
        // Text content.
        if let Some(content) = choice.message.content {
            let text = match &content {
                serde_json::Value::String(s) => s.clone(),
                _ => content.to_string(),
            };
            if !text.is_empty() {
                output.output.push(ImagesOutputContent::Text(
                    crate::images::local_types::ImagesOutputTextContent { text },
                ));
            }
        }

        // Image content from the special `images` array.
        if let Some(images) = choice.message.images {
            for image in images {
                let image_url_str = extract_image_url_string(image.image_url.as_ref());
                let Some(url) = image_url_str else { continue };
                if !url.starts_with("data:") {
                    continue;
                }
                if let Some(caps) = DATA_URI_RE.captures(&url) {
                    let mime_type = caps[1].to_string();
                    let data = caps[2].to_string();
                    output.output.push(ImagesOutputContent::Image(
                        crate::images::local_types::ImagesOutputImageContent { data, mime_type },
                    ));
                }
            }
        }
    }

    Ok(output)
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Resolve the API key from options or environment.
fn resolve_api_key(model: &ImagesModel, opts: &ImagesOptions) -> Option<String> {
    // Try provider-scoped env first.
    if let Some(ref env) = opts.env {
        for var_name in openrouter_api_key_env_vars(&model.provider) {
            if let Some(val) = env.get(var_name) {
                return Some(val.clone());
            }
        }
    }
    // Fall back to process env.
    for var_name in openrouter_api_key_env_vars(&model.provider) {
        if let Ok(val) = std::env::var(var_name) {
            return Some(val);
        }
    }
    None
}

/// Return the environment variable names to check for an OpenRouter API key.
fn openrouter_api_key_env_vars(_provider: &str) -> Vec<&'static str> {
    vec!["OPENROUTER_API_KEY"]
}

/// Build the request JSON body for the OpenRouter chat completions endpoint.
fn build_request_json(model: &ImagesModel, context: &ImagesContext) -> OpenRouterRequest {
    let content: Vec<OpenRouterRequestContentPart> = context
        .input
        .iter()
        .map(|item| match item {
            ImagesInputContent::Text(t) => OpenRouterRequestContentPart::Text {
                text: sanitize_surrogates(&t.text),
            },
            ImagesInputContent::Image(img) => OpenRouterRequestContentPart::ImageUrl {
                image_url: OpenRouterRequestImageUrl {
                    url: format!("data:{};base64,{}", img.mime_type, img.data),
                },
            },
        })
        .collect();

    let modalities = if model.output.iter().any(|o| o == "text") {
        Some(vec!["image".to_string(), "text".to_string()])
    } else {
        Some(vec!["image".to_string()])
    };

    OpenRouterRequest {
        model: model.id.clone(),
        messages: vec![OpenRouterRequestMessage {
            role: "user".to_string(),
            content,
        }],
        stream: Some(false),
        modalities,
    }
}

/// Extract the URL string from the various shapes OpenRouter returns for
/// `image_url` (can be a string, or an object `{ url: string }`, or missing).
fn extract_image_url_string(value: Option<&serde_json::Value>) -> Option<String> {
    match value {
        Some(serde_json::Value::String(s)) => Some(s.clone()),
        Some(serde_json::Value::Object(obj)) => obj
            .get("url")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        _ => None,
    }
}

/// Parse usage from the OpenRouter response and compute costs.
///
/// Mirrors the TS `parseUsage`.
fn parse_usage(raw: OpenRouterUsage, cost: &ImagesModelCost) -> ImagesUsage {
    let prompt_tokens = raw.prompt_tokens.unwrap_or(0);
    let reported_cached_tokens = raw
        .prompt_tokens_details
        .as_ref()
        .and_then(|d| d.cached_tokens)
        .unwrap_or(0);
    let cache_write_tokens = raw
        .prompt_tokens_details
        .as_ref()
        .and_then(|d| d.cache_write_tokens)
        .unwrap_or(0);
    let cache_read_tokens = if cache_write_tokens > 0 {
        reported_cached_tokens.saturating_sub(cache_write_tokens)
    } else {
        reported_cached_tokens
    };
    let input =
        (prompt_tokens as i64 - cache_read_tokens as i64 - cache_write_tokens as i64).max(0) as u64;
    let output = raw.completion_tokens.unwrap_or(0);

    let cost_input = (cost.input / 1_000_000.0) * input as f64;
    let cost_output = (cost.output / 1_000_000.0) * output as f64;
    let cost_cache_read = (cost.cache_read / 1_000_000.0) * cache_read_tokens as f64;
    let cost_cache_write = (cost.cache_write / 1_000_000.0) * cache_write_tokens as f64;
    let cost_total = cost_input + cost_output + cost_cache_read + cost_cache_write;

    ImagesUsage {
        input,
        output,
        cache_read: cache_read_tokens,
        cache_write: cache_write_tokens,
        total_tokens: input + output + cache_read_tokens + cache_write_tokens,
        cost: ImagesUsageCost {
            input: cost_input,
            output: cost_output,
            cache_read: cost_cache_read,
            cache_write: cost_cache_write,
            total: cost_total,
        },
    }
}

impl Default for ImagesOptions {
    fn default() -> Self {
        ImagesOptions {
            signal: None,
            api_key: None,
            on_payload: None,
            on_response: None,
            headers: None,
            timeout_ms: None,
            max_retries: None,
            env: None,
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::images::local_types::*;

    #[test]
    fn test_build_request_json_text_only() {
        let model = ImagesModel {
            id: "openai/qwen-vl".to_string(),
            name: "Qwen VL".to_string(),
            api: "openrouter-images".to_string(),
            provider: "openrouter".to_string(),
            base_url: "https://openrouter.ai/api".to_string(),
            output: vec!["image".to_string()],
            cost: ImagesModelCost {
                input: 0.0,
                output: 0.0,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            headers: None,
        };
        let context = ImagesContext {
            input: vec![ImagesInputContent::Text(ImagesInputTextContent {
                text: "Generate a cat".to_string(),
            })],
        };

        let request = build_request_json(&model, &context);
        assert_eq!(request.model, "openai/qwen-vl");
        assert_eq!(request.messages.len(), 1);
        assert_eq!(request.messages[0].role, "user");
        assert_eq!(request.stream, Some(false));
        // Only "image" modality since output doesn't include "text"
        assert_eq!(request.modalities, Some(vec!["image".to_string()]));
    }

    #[test]
    fn test_build_request_json_with_image_input() {
        let model = ImagesModel {
            id: "openai/qwen-vl".to_string(),
            name: "Qwen VL".to_string(),
            api: "openrouter-images".to_string(),
            provider: "openrouter".to_string(),
            base_url: "https://openrouter.ai/api".to_string(),
            output: vec!["text".to_string(), "image".to_string()],
            cost: ImagesModelCost {
                input: 0.0,
                output: 0.0,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            headers: None,
        };
        let context = ImagesContext {
            input: vec![
                ImagesInputContent::Text(ImagesInputTextContent {
                    text: "Describe this image".to_string(),
                }),
                ImagesInputContent::Image(ImagesInputImageContent {
                    data: "abc123".to_string(),
                    mime_type: "image/png".to_string(),
                }),
            ],
        };

        let request = build_request_json(&model, &context);
        assert_eq!(request.messages[0].content.len(), 2);
        // Modalities should include "text" since output has "text"
        assert_eq!(
            request.modalities,
            Some(vec!["image".to_string(), "text".to_string()])
        );

        // Check the image content part.
        let image_part = &request.messages[0].content[1];
        if let OpenRouterRequestContentPart::ImageUrl { image_url } = image_part {
            assert_eq!(image_url.url, "data:image/png;base64,abc123");
        } else {
            panic!("Expected ImageUrl content part");
        }
    }

    #[test]
    fn test_extract_image_url_string() {
        // String form.
        assert_eq!(
            extract_image_url_string(Some(&serde_json::json!("data:image/png;base64,xyz"))),
            Some("data:image/png;base64,xyz".to_string())
        );
        // Object form.
        assert_eq!(
            extract_image_url_string(Some(
                &serde_json::json!({"url": "data:image/png;base64,xyz"})
            )),
            Some("data:image/png;base64,xyz".to_string())
        );
        // Missing.
        assert_eq!(extract_image_url_string(None), None);
        // Invalid.
        assert_eq!(extract_image_url_string(Some(&serde_json::json!(42))), None);
    }

    #[test]
    fn test_parse_usage() {
        let cost = ImagesModelCost {
            input: 1.0,
            output: 2.0,
            cache_read: 0.5,
            cache_write: 1.5,
        };
        let raw = OpenRouterUsage {
            prompt_tokens: Some(100),
            completion_tokens: Some(50),
            prompt_tokens_details: Some(OpenRouterPromptTokensDetails {
                cached_tokens: Some(30),
                cache_write_tokens: Some(10),
            }),
        };
        let usage = parse_usage(raw, &cost);

        // cache_read = 30 - 10 = 20 (since cache_write_tokens > 0)
        assert_eq!(usage.cache_read, 20);
        assert_eq!(usage.cache_write, 10);
        // input = 100 - 20 - 10 = 70
        assert_eq!(usage.input, 70);
        assert_eq!(usage.output, 50);
        assert_eq!(usage.total_tokens, 70 + 50 + 20 + 10); // 150

        assert!((usage.cost.input - 70.0 * 1.0 / 1_000_000.0).abs() < f64::EPSILON);
        assert!((usage.cost.output - 50.0 * 2.0 / 1_000_000.0).abs() < f64::EPSILON);
        assert!((usage.cost.cache_read - 20.0 * 0.5 / 1_000_000.0).abs() < f64::EPSILON);
        assert!((usage.cost.cache_write - 10.0 * 1.5 / 1_000_000.0).abs() < f64::EPSILON);
        assert!(
            (usage.cost.total - (70.0 * 1.0 + 50.0 * 2.0 + 20.0 * 0.5 + 10.0 * 1.5) / 1_000_000.0)
                .abs()
                < f64::EPSILON
        );
    }

    #[test]
    fn test_parse_usage_no_details() {
        let cost = ImagesModelCost {
            input: 1.0,
            output: 2.0,
            cache_read: 0.5,
            cache_write: 1.5,
        };
        let raw = OpenRouterUsage {
            prompt_tokens: Some(200),
            completion_tokens: Some(100),
            prompt_tokens_details: None,
        };
        let usage = parse_usage(raw, &cost);

        assert_eq!(usage.input, 200);
        assert_eq!(usage.output, 100);
        assert_eq!(usage.cache_read, 0);
        assert_eq!(usage.cache_write, 0);
        assert_eq!(usage.total_tokens, 300);
    }

    #[test]
    fn test_parse_usage_cached_tokens_no_write() {
        let cost = ImagesModelCost {
            input: 1.0,
            output: 2.0,
            cache_read: 0.5,
            cache_write: 1.5,
        };
        // When there's no cache_write_tokens, cached_tokens goes entirely to cache_read.
        let raw = OpenRouterUsage {
            prompt_tokens: Some(150),
            completion_tokens: Some(60),
            prompt_tokens_details: Some(OpenRouterPromptTokensDetails {
                cached_tokens: Some(40),
                cache_write_tokens: None,
            }),
        };
        let usage = parse_usage(raw, &cost);

        assert_eq!(usage.cache_read, 40);
        assert_eq!(usage.cache_write, 0);
        assert_eq!(usage.input, 110); // 150 - 40 - 0
    }

    #[test]
    fn test_data_uri_regex() {
        let re = &DATA_URI_RE;
        let caps = re.captures("data:image/png;base64,abc123def456").unwrap();
        assert_eq!(&caps[1], "image/png");
        assert_eq!(&caps[2], "abc123def456");

        // Non-data URIs don't match.
        assert!(re.captures("https://example.com/image.png").is_none());

        // Stays-without- leadingdata don't match.
        assert!(re.captures("notadata:image/png;base64,x").is_none());
    }

    #[test]
    fn test_response_serialization_roundtrip() {
        let json = r#"{
            "id": "chatcmpl-123",
            "usage": { "promptTokens": 10, "completionTokens": 20 },
            "choices": [{
                "message": {
                    "content": "Here is your image",
                    "images": [
                        { "image_url": { "url": "data:image/png;base64,aGVsbG8=" } }
                    ]
                }
            }]
        }"#;

        let resp: OpenRouterImageGenerationResponse =
            serde_json::from_str(json).expect("should parse");
        assert_eq!(resp.id, Some("chatcmpl-123".to_string()));
        assert_eq!(resp.choices.len(), 1);

        let msg = &resp.choices[0].message;
        assert_eq!(
            msg.content.as_ref().and_then(|v| v.as_str()),
            Some("Here is your image")
        );
        assert!(msg.images.is_some());
        assert_eq!(msg.images.as_ref().unwrap().len(), 1);

        let usage = resp.usage.unwrap();
        assert_eq!(usage.prompt_tokens, Some(10));
        assert_eq!(usage.completion_tokens, Some(20));
    }

    #[test]
    fn test_api_key_env_vars() {
        let vars = openrouter_api_key_env_vars("openrouter");
        assert!(vars.contains(&"OPENROUTER_API_KEY"));
    }

    #[test]
    fn test_resolve_api_key_from_opts() {
        let model = ImagesModel {
            id: "m".to_string(),
            name: "m".to_string(),
            api: "openrouter-images".to_string(),
            provider: "openrouter".to_string(),
            base_url: "https://example.com".to_string(),
            output: vec!["image".to_string()],
            cost: ImagesModelCost {
                input: 0.0,
                output: 0.0,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            headers: None,
        };

        // When api_key is set in options directly, resolve_api_key is not called
        // (the main function checks opts.api_key first). But test the standalone
        // function with a provider env.
        let mut env = std::collections::HashMap::new();
        env.insert("OPENROUTER_API_KEY".to_string(), "sk-test-key".to_string());
        let opts = ImagesOptions {
            api_key: None,
            env: Some(env),
            ..Default::default()
        };

        let key = resolve_api_key(&model, &opts);
        assert_eq!(key, Some("sk-test-key".to_string()));
    }

    #[tokio::test]
    async fn test_http_error_returns_error_message() {
        // Use a URL that will fail immediately (no server running).
        let model = ImagesModel {
            id: "m".to_string(),
            name: "m".to_string(),
            api: "openrouter-images".to_string(),
            provider: "openrouter".to_string(),
            base_url: "http://127.0.0.1:1".to_string(), // unlikely to have anything here
            output: vec!["image".to_string()],
            cost: ImagesModelCost {
                input: 0.0,
                output: 0.0,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            headers: None,
        };
        let context = ImagesContext { input: vec![] };

        let result = generate_images_open_router(
            model,
            context,
            Some(ImagesOptions {
                api_key: Some("sk-test".to_string()),
                timeout_ms: Some(100),
                ..Default::default()
            }),
        )
        .await;

        // Should not panic; returns an error result (or Ok with error embedded).
        match result {
            Ok(images) => {
                assert_eq!(images.stop_reason, ImagesStopReason::Error);
                assert!(images.error_message.is_some());
            }
            Err(e) => {
                assert!(!e.is_empty());
            }
        }
    }
}
