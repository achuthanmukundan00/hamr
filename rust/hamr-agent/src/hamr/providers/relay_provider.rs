//! Port of `packages/coding-agent/src/hamr/providers/relay-provider.ts`.
//!
//! Relay model auto-detection — queries an OpenAI-compatible `GET /v1/models`
//! endpoint and discovers whatever models the relay is currently serving.
//!
//! Scans many context-window field names from different OpenAI-compatible
//! servers, detects vision capability, and derives readable display names.
//! All failures are swallowed — an unreachable endpoint yields an empty list.

use std::collections::HashMap;

// ─── DiscoveredRelayModel ────────────────────────────────────────────────────

/// A model discovered from a relay / OpenAI-compatible endpoint.
/// Mirror of the TS `DiscoveredRelayModel` interface.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscoveredRelayModel {
    pub id: String,
    pub display_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_window: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_output_tokens: Option<u64>,
    pub supports_thinking: bool,
    pub thinking_levels: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supports_vision: Option<bool>,
}

// ─── Context-window field names ──────────────────────────────────────────────

/// Common field names used by various OpenAI-compatible servers for context window.
/// Order roughly by likelihood: llama.cpp forks → vLLM/SGLang → generic → others.
pub const CONTEXT_WINDOW_FIELDS: &[&str] = &[
    "max_context_length",
    "max_model_len",
    "context_length",
    "max_total_tokens",
    "total_tokens_capacity",
    "n_ctx",
    "max_position_embeddings",
    "model_max_length",
    "max_seq_len",
    "max_sequence_length",
];

/// Known thinking levels across OpenAI-compatible servers.
pub const KNOWN_THINKING_LEVELS: &[&str] = &["off", "on", "low", "medium", "high", "xhigh"];

/// Discovery is best-effort and must never block startup for long.
pub const DISCOVERY_TIMEOUT_MS: u64 = 5000;

// ─── Header construction ─────────────────────────────────────────────────────

/// Build request headers for the model discovery endpoint.
/// Mirror of `buildHeaders` in the TS source.
pub fn build_headers(
    api_key: Option<&str>,
    custom_headers: Option<&HashMap<String, String>>,
) -> HashMap<String, String> {
    let mut headers = HashMap::new();
    headers.insert("Accept".to_string(), "application/json".to_string());
    headers.insert("User-Agent".to_string(), "hamr/1.0".to_string());
    if let Some(key) = api_key {
        headers.insert("Authorization".to_string(), format!("Bearer {}", key));
    }
    if let Some(ch) = custom_headers {
        for (k, v) in ch {
            headers.insert(k.clone(), v.clone());
        }
    }
    headers
}

// ─── Context window extraction ───────────────────────────────────────────────

/// Extract a context window value from a model entry.
///
/// Checks top-level fields first, then falls back to the `meta` sub-object.
/// Accepts both numeric values and numeric strings.
///
/// Mirror of `extractContextWindow` in the TS source.
pub fn extract_context_window(entry: &serde_json::Value) -> Option<u64> {
    // Check top-level fields
    if let Some(obj) = entry.as_object() {
        for field in CONTEXT_WINDOW_FIELDS {
            if let Some(value) = obj.get(*field) {
                if let Some(n) = value.as_u64() {
                    if n > 0 {
                        return Some(n);
                    }
                }
                if let Some(n) = value.as_f64() {
                    if n.is_finite() && n > 0.0 {
                        return Some(n as u64);
                    }
                }
                if let Some(s) = value.as_str() {
                    if let Ok(n) = s.parse::<u64>() {
                        if n > 0 {
                            return Some(n);
                        }
                    }
                }
            }
        }

        // Check meta sub-object
        if let Some(meta) = obj.get("meta").and_then(|v| v.as_object()) {
            for field in CONTEXT_WINDOW_FIELDS {
                if let Some(value) = meta.get(*field) {
                    if let Some(n) = value.as_u64() {
                        if n > 0 {
                            return Some(n);
                        }
                    }
                    if let Some(n) = value.as_f64() {
                        if n.is_finite() && n > 0.0 {
                            return Some(n as u64);
                        }
                    }
                    if let Some(s) = value.as_str() {
                        if let Ok(n) = s.parse::<u64>() {
                            if n > 0 {
                                return Some(n);
                            }
                        }
                    }
                }
            }
        }
    }
    None
}

// ─── Display name derivation ─────────────────────────────────────────────────

/// Derive a human-readable display name from a model ID.
///
/// Strips common file extensions and quantization suffixes
/// (gguf, IQ*, Q*_K variants, f16/f32/q4_0 etc.).
///
/// Mirror of `deriveDisplayName` in the TS source.
pub fn derive_display_name(id: &str) -> String {
    let re_gguf = regex::Regex::new(r"(?i)\.gguf$").unwrap();
    let re_iq = regex::Regex::new(r"(?i)[-_]IQ[23456]_\w+$").unwrap();
    let re_q = regex::Regex::new(r"(?i)[-_]Q[234568]_K_\w+$").unwrap();
    let re_dtype = regex::Regex::new(r"(?i)[-_](f16|f32|q4_0|q4_1|q5_0|q5_1|q8_0)$").unwrap();

    let cleaned = re_gguf.replace(id, "");
    let cleaned = re_iq.replace(&cleaned, "");
    let cleaned = re_q.replace(&cleaned, "");
    let cleaned = re_dtype.replace(&cleaned, "");

    // Replace - and _ with spaces, capitalize each word
    let re_sep = regex::Regex::new(r"[-_]").unwrap();
    let spaced = re_sep.replace_all(&cleaned, " ");
    // Capitalize first letter of each word
    let re_word = regex::Regex::new(r"\b\w").unwrap();
    re_word
        .replace_all(&spaced, |caps: &regex::Captures| caps[0].to_uppercase())
        .to_string()
}

// ─── String list helper ──────────────────────────────────────────────────────

/// Convert a JSON value to a list of lowercase strings.
/// Mirror of `stringList` in the TS source.
fn string_list(value: &serde_json::Value) -> Vec<String> {
    value
        .as_array()
        .map(|arr| {
            arr.iter()
                .map(|v| match v.as_str() {
                    Some(s) => s.to_lowercase(),
                    None => v.to_string().to_lowercase(),
                })
                .collect()
        })
        .unwrap_or_default()
}

// ─── Boolean field extraction ────────────────────────────────────────────────

/// Try to extract a boolean from any of the given fields.
///
/// Handles boolean values, and string values "true"/"yes"/"1" → true,
/// "false"/"no"/"0" → false.
///
/// Mirror of `booleanField` in the TS source.
fn boolean_field(
    entry: &serde_json::Map<String, serde_json::Value>,
    fields: &[&str],
) -> Option<bool> {
    for field in fields {
        if let Some(value) = entry.get(*field) {
            if let Some(b) = value.as_bool() {
                return Some(b);
            }
            if let Some(s) = value.as_str() {
                let lower = s.to_lowercase();
                if ["true", "yes", "1"].contains(&lower.as_str()) {
                    return Some(true);
                }
                if ["false", "no", "0"].contains(&lower.as_str()) {
                    return Some(false);
                }
            }
        }
    }
    None
}

/// Fields checked for vision support.
const VISION_FIELDS: &[&str] = &[
    "supports_vision",
    "supportsVision",
    "vision",
    "multimodal",
    "supports_images",
    "supportsImages",
];

/// Extract whether the model supports vision.
///
/// Checks explicit boolean fields first, then falls back to scanning
/// `capabilities`, `modalities`, `input_modalities`, `inputModalities`,
/// and `features` arrays (both at top level and in `meta`).
///
/// Mirror of `extractSupportsVision` in the TS source.
pub fn extract_supports_vision(entry: &serde_json::Value) -> Option<bool> {
    let obj = entry.as_object()?;

    // Check explicit boolean fields
    let explicit = boolean_field(obj, VISION_FIELDS);
    if explicit.is_some() {
        return explicit;
    }

    // Check meta sub-object
    let meta = obj.get("meta").and_then(|v| v.as_object());
    if let Some(m) = meta {
        let meta_explicit = boolean_field(m, VISION_FIELDS);
        if meta_explicit.is_some() {
            return meta_explicit;
        }
    }

    // Scan array fields
    let mut haystack: Vec<String> = Vec::new();
    haystack.extend(string_list(
        entry
            .get("capabilities")
            .unwrap_or(&serde_json::Value::Null),
    ));
    haystack.extend(string_list(
        entry.get("modalities").unwrap_or(&serde_json::Value::Null),
    ));
    haystack.extend(string_list(
        entry
            .get("input_modalities")
            .unwrap_or(&serde_json::Value::Null),
    ));
    haystack.extend(string_list(
        entry
            .get("inputModalities")
            .unwrap_or(&serde_json::Value::Null),
    ));
    haystack.extend(string_list(
        entry.get("features").unwrap_or(&serde_json::Value::Null),
    ));
    if let Some(m) = meta {
        haystack.extend(string_list(
            m.get("capabilities").unwrap_or(&serde_json::Value::Null),
        ));
        haystack.extend(string_list(
            m.get("modalities").unwrap_or(&serde_json::Value::Null),
        ));
        haystack.extend(string_list(
            m.get("input_modalities")
                .unwrap_or(&serde_json::Value::Null),
        ));
        haystack.extend(string_list(
            m.get("inputModalities").unwrap_or(&serde_json::Value::Null),
        ));
        haystack.extend(string_list(
            m.get("features").unwrap_or(&serde_json::Value::Null),
        ));
    }

    let vision_keywords = ["multimodal", "vision", "image", "images"];
    if haystack
        .iter()
        .any(|item| vision_keywords.contains(&item.as_str()))
    {
        return Some(true);
    }

    let text_only_keywords = ["text-only", "text_only"];
    if haystack
        .iter()
        .any(|item| text_only_keywords.contains(&item.as_str()))
    {
        return Some(false);
    }

    None
}

// ─── Model discovery ─────────────────────────────────────────────────────────

/// Fetch available models from an OpenAI-compatible `GET /v1/models` endpoint.
///
/// Returns the discovered models, or an empty vector if the endpoint could not
/// be reached or returned an unrecognized payload. Never panics.
///
/// Mirror of `discoverRelayModels` in the TS source.
///
pub async fn discover_relay_models(
    base_url: &str,
    api_key: Option<&str>,
    custom_headers: Option<&HashMap<String, String>>,
) -> Vec<DiscoveredRelayModel> {
    let clean_base_url = base_url.trim_end_matches('/');
    let headers = build_headers(api_key, custom_headers);

    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_millis(DISCOVERY_TIMEOUT_MS))
        .build()
    {
        Ok(c) => c,
        Err(_) => return vec![],
    };

    let url = format!("{}/models", clean_base_url);
    let mut req = client.get(&url);
    for (k, v) in &headers {
        req = req.header(k.as_str(), v.as_str());
    }

    let res = match req.send().await {
        Ok(r) if r.status().is_success() => r,
        _ => return vec![],
    };

    let body: serde_json::Value = match res.json().await {
        Ok(b) => b,
        Err(_) => return vec![],
    };

    parse_models_response(&body)
}

// ─── Parse helper: extract models from response body ─────────────────────────

/// Parse a `GET /v1/models` JSON response body into discovered models.
///
/// Handles both standard `{ object: "list", data: [...] }` and bare-array
/// response formats. Missing or unrecognized payloads return an empty vector.
///
/// This is the pure parsing function, separated from the async HTTP fetch.
pub fn parse_models_response(body: &serde_json::Value) -> Vec<DiscoveredRelayModel> {
    // Standard OpenAI /v1/models response: { object: "list", data: [...] }
    // Some servers return a bare array instead.
    let entries: Vec<&serde_json::Value> =
        if let Some(data) = body.get("data").and_then(|d| d.as_array()) {
            data.iter().collect()
        } else if let Some(arr) = body.as_array() {
            arr.iter().collect()
        } else {
            return vec![];
        };

    let mut models: Vec<DiscoveredRelayModel> = Vec::new();
    for entry in entries {
        let obj = match entry.as_object() {
            Some(o) => o,
            None => continue,
        };

        // ID: prefer `id` over `name`
        let raw_id = obj
            .get("id")
            .and_then(|v| v.as_str())
            .or_else(|| obj.get("name").and_then(|v| v.as_str()))
            .unwrap_or("");
        let id = raw_id.trim();
        if id.is_empty() {
            continue;
        }

        let context_window = extract_context_window(entry);

        // Server-provided display name
        let server_name = obj
            .get("display_name")
            .and_then(|v| v.as_str())
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .or_else(|| {
                obj.get("name")
                    .and_then(|v| v.as_str())
                    .map(|s| s.trim())
                    .filter(|s| !s.is_empty())
            });
        let display_name = server_name
            .map(|s| s.to_string())
            .unwrap_or_else(|| derive_display_name(id));

        let supports_vision = extract_supports_vision(entry);

        let supports_thinking = obj
            .get("supports_thinking")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let api_levels: Vec<String> = obj
            .get("thinking_levels")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str())
                    .filter(|l| KNOWN_THINKING_LEVELS.contains(l))
                    .map(|s| s.to_string())
                    .collect()
            })
            .unwrap_or_default();

        // When the server says thinking is supported but doesn't advertise
        // specific levels, default to boolean on/off (standard for local models).
        let thinking_levels = if !api_levels.is_empty() {
            api_levels
        } else if supports_thinking {
            vec!["off".to_string(), "on".to_string()]
        } else {
            vec![]
        };

        let max_output_tokens = obj
            .get("max_output_tokens")
            .and_then(|v| v.as_u64())
            .filter(|&n| n > 0);

        models.push(DiscoveredRelayModel {
            id: id.to_string(),
            display_name,
            context_window,
            max_output_tokens,
            supports_thinking,
            thinking_levels,
            supports_vision,
        });
    }

    models
}

// ─── Extension integration ───────────────────────────────────────────────────

/// Extension integration — when ExtensionAPI is ported, the relay provider
/// will register itself and provide discovered models.
pub const EXTENSION_NAME: &str = "hamr-relay-provider";

#[cfg(test)]
mod tests {
    use super::*;

    // ─── Header tests ────────────────────────────────────────────

    #[test]
    fn test_build_headers_minimal() {
        let headers = build_headers(None, None);
        assert_eq!(headers.get("Accept").unwrap(), "application/json");
        assert_eq!(headers.get("User-Agent").unwrap(), "hamr/1.0");
        assert!(!headers.contains_key("Authorization"));
    }

    #[test]
    fn test_build_headers_with_api_key() {
        let headers = build_headers(Some("my-key"), None);
        assert_eq!(headers.get("Authorization").unwrap(), "Bearer my-key");
    }

    #[test]
    fn test_build_headers_with_custom() {
        let mut custom = HashMap::new();
        custom.insert("X-Custom".to_string(), "value".to_string());
        let headers = build_headers(None, Some(&custom));
        assert_eq!(headers.get("X-Custom").unwrap(), "value");
    }

    // ─── Context window extraction ───────────────────────────────

    #[test]
    fn test_extract_context_window_top_level_numeric() {
        let entry = serde_json::json!({
            "id": "model1",
            "max_context_length": 32768
        });
        assert_eq!(extract_context_window(&entry), Some(32768));
    }

    #[test]
    fn test_extract_context_window_string() {
        let entry = serde_json::json!({
            "id": "model1",
            "context_length": "4096"
        });
        assert_eq!(extract_context_window(&entry), Some(4096));
    }

    #[test]
    fn test_extract_context_window_meta() {
        let entry = serde_json::json!({
            "id": "model1",
            "meta": {
                "max_model_len": 8192
            }
        });
        assert_eq!(extract_context_window(&entry), Some(8192));
    }

    #[test]
    fn test_extract_context_window_none() {
        let entry = serde_json::json!({"id": "model1"});
        assert_eq!(extract_context_window(&entry), None);
    }

    // ─── Display name derivation ─────────────────────────────────

    #[test]
    fn test_derive_display_name_basic() {
        assert_eq!(derive_display_name("llama-3-8B"), "Llama 3 8B");
    }

    #[test]
    fn test_derive_display_name_strips_gguf() {
        let name = derive_display_name("mistral-7b.Q4_K_M.gguf");
        assert!(!name.contains(".gguf"));
        assert!(!name.contains("Q4_K_M"));
        assert!(name.contains("Mistral"));
        assert!(name.contains("7b"));
    }

    #[test]
    fn test_derive_display_name_strips_iq() {
        let name = derive_display_name("model-IQ4_XS");
        assert!(!name.to_lowercase().contains("iq4"));
    }

    #[test]
    fn test_derive_display_name_strips_f16() {
        let name = derive_display_name("model-f16");
        assert!(!name.to_lowercase().contains("f16"));
    }

    // ─── Vision detection ────────────────────────────────────────

    #[test]
    fn test_extract_supports_vision_explicit_true() {
        let entry = serde_json::json!({
            "id": "model1",
            "supports_vision": true
        });
        assert_eq!(extract_supports_vision(&entry), Some(true));
    }

    #[test]
    fn test_extract_supports_vision_explicit_false() {
        let entry = serde_json::json!({
            "id": "model1",
            "multimodal": false
        });
        assert_eq!(extract_supports_vision(&entry), Some(false));
    }

    #[test]
    fn test_extract_supports_vision_capabilities() {
        let entry = serde_json::json!({
            "id": "model1",
            "capabilities": ["completion", "multimodal"]
        });
        assert_eq!(extract_supports_vision(&entry), Some(true));
    }

    #[test]
    fn test_extract_supports_vision_text_only() {
        let entry = serde_json::json!({
            "id": "model1",
            "capabilities": ["text-only"]
        });
        assert_eq!(extract_supports_vision(&entry), Some(false));
    }

    #[test]
    fn test_extract_supports_vision_unknown() {
        let entry = serde_json::json!({"id": "model1"});
        assert_eq!(extract_supports_vision(&entry), None);
    }

    // ─── Parse models response ───────────────────────────────────

    #[test]
    fn test_parse_models_response_standard() {
        let body = serde_json::json!({
            "object": "list",
            "data": [
                {
                    "id": "llama-3-8b",
                    "name": "Llama 3 8B",
                    "max_context_length": 8192
                }
            ]
        });
        let models = parse_models_response(&body);
        assert_eq!(models.len(), 1);
        assert_eq!(models[0].id, "llama-3-8b");
        assert_eq!(models[0].display_name, "Llama 3 8B");
        assert_eq!(models[0].context_window, Some(8192));
    }

    #[test]
    fn test_parse_models_response_bare_array() {
        let body = serde_json::json!([
            {"id": "model-a"},
            {"id": "model-b"}
        ]);
        let models = parse_models_response(&body);
        assert_eq!(models.len(), 2);
        assert_eq!(models[0].id, "model-a");
        assert_eq!(models[1].id, "model-b");
    }

    #[test]
    fn test_parse_models_response_skips_empty_id() {
        let body = serde_json::json!({
            "data": [
                {"id": ""},
                {"id": "  "},
                {"id": "valid"}
            ]
        });
        let models = parse_models_response(&body);
        assert_eq!(models.len(), 1);
        assert_eq!(models[0].id, "valid");
    }

    #[test]
    fn test_parse_models_response_unknown_format() {
        let body = serde_json::json!({"error": "not found"});
        let models = parse_models_response(&body);
        assert!(models.is_empty());
    }

    #[test]
    fn test_parse_models_response_thinking_discovery() {
        let body = serde_json::json!({
            "data": [{
                "id": "model1",
                "supports_thinking": true,
                "thinking_levels": ["off", "on", "high"]
            }]
        });
        let models = parse_models_response(&body);
        assert_eq!(models.len(), 1);
        assert!(models[0].supports_thinking);
        assert_eq!(models[0].thinking_levels, vec!["off", "on", "high"]);
    }

    #[test]
    fn test_parse_models_response_thinking_no_levels() {
        let body = serde_json::json!({
            "data": [{
                "id": "model1",
                "supports_thinking": true
            }]
        });
        let models = parse_models_response(&body);
        assert_eq!(models.len(), 1);
        assert!(models[0].supports_thinking);
        assert_eq!(models[0].thinking_levels, vec!["off", "on"]);
    }

    #[test]
    fn test_extension_name() {
        assert_eq!(EXTENSION_NAME, "hamr-relay-provider");
    }
}
