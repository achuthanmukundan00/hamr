//! Port of `packages/coding-agent/src/core/model-registry.ts`.
//!
//! Model registry — manages built-in and custom models, provides API key resolution.
//!
//! 1. Loads built-in models via `hamr_ai::models::get_models`/`get_providers`
//! 2. Reads custom models from `models.json` (validates with manual schema checks)
//! 3. Merges custom models with built-ins (custom wins on provider+id conflicts)
//! 4. Provides auth resolution via [`AuthStorage`]
//! 5. Supports dynamic provider registration/unregistration

use hamr_ai::models::{get_models, get_providers};
use hamr_ai::types::{Api, Model, ModelCost, ThinkingLevelMap};
use std::collections::HashMap;
use std::fs;
use std::future::Future;
use std::path::Path;
use std::pin::Pin;
use std::sync::Arc;

/// Auth status for a provider (mirrors the TS `AuthStatus` interface).
#[derive(Debug, Clone, Default)]
pub struct AuthStatus {
    pub configured: bool,
    pub source: Option<String>,
    pub label: Option<String>,
}

// AuthStorage is a trait that the runtime implements.
// We use a minimal trait definition here to avoid circular dependencies.
pub mod auth_trait {
    use std::collections::HashMap;
    use std::future::Future;
    use std::pin::Pin;

    /// Auth credential (mirrors TS AuthCredential).
    #[derive(Debug, Clone)]
    pub enum AuthCredential {
        OAuth {
            provider_id: String,
            credentials: serde_json::Value,
        },
        ApiKey {
            key: String,
        },
    }

    /// Interface for auth storage — the runtime provides a concrete implementation.
    ///
    /// Uses `Pin<Box<dyn Future>>` instead of `async fn` for dyn-compatibility.
    pub trait AuthStorage: Send + Sync {
        fn get(&self, provider: &str) -> Option<AuthCredential>;
        fn get_provider_env(&self, provider: &str) -> Option<HashMap<String, String>>;
        fn has_auth(&self, provider: &str) -> bool;
        fn get_api_key(
            &self,
            provider: &str,
            include_fallback: bool,
        ) -> Pin<Box<dyn Future<Output = Option<String>> + Send + '_>>;
        fn get_auth_status(&self, provider: &str) -> super::AuthStatus;
        fn get_oauth_providers(&self) -> Vec<serde_json::Value>;
    }
}

use auth_trait::{AuthCredential, AuthStorage};

/// A no-op auth storage that returns None for everything.
/// Used as a placeholder when no real auth storage is available.
#[derive(Clone)]
pub struct NoopAuthStorage;

impl AuthStorage for NoopAuthStorage {
    fn get(&self, _provider: &str) -> Option<AuthCredential> {
        None
    }
    fn get_provider_env(&self, _provider: &str) -> Option<HashMap<String, String>> {
        None
    }
    fn has_auth(&self, _provider: &str) -> bool {
        false
    }
    fn get_api_key(
        &self,
        _provider: &str,
        _include_fallback: bool,
    ) -> Pin<Box<dyn Future<Output = Option<String>> + Send + '_>> {
        Box::pin(async { None })
    }
    fn get_auth_status(&self, _provider: &str) -> crate::core::model_registry::AuthStatus {
        crate::core::model_registry::AuthStatus {
            configured: false,
            source: None,
            label: None,
        }
    }
    fn get_oauth_providers(&self) -> Vec<serde_json::Value> {
        Vec::new()
    }
}

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// Provider override config (baseUrl, compat) without request auth/headers.
#[derive(Debug, Clone, Default)]
pub struct ProviderOverride {
    pub base_url: Option<String>,
    pub compat: Option<serde_json::Value>,
}

/// Per-request auth config for a provider (API key, headers, auth header mode).
#[derive(Debug, Clone, Default)]
pub struct ProviderRequestConfig {
    pub api_key: Option<String>,
    pub headers: Option<HashMap<String, String>>,
    pub auth_header: Option<bool>,
}

/// Arguments for a stream_simple function.
pub struct StreamSimpleArgs {
    pub model: Model,
    pub context: serde_json::Value,
    pub options: Option<serde_json::Value>,
}

/// Result from a stream_simple function.
pub type StreamSimpleResult = Result<serde_json::Value, String>;

/// Result of resolving request auth for a model.
#[derive(Debug, Clone)]
pub enum ResolvedRequestAuth {
    Ok {
        api_key: Option<String>,
        headers: Option<HashMap<String, String>>,
        env: Option<HashMap<String, String>>,
    },
    Err {
        error: String,
    },
}

impl ResolvedRequestAuth {
    pub fn ok(
        api_key: Option<String>,
        headers: Option<HashMap<String, String>>,
        env: Option<HashMap<String, String>>,
    ) -> Self {
        Self::Ok {
            api_key,
            headers,
            env,
        }
    }

    pub fn err(error: impl Into<String>) -> Self {
        Self::Err {
            error: error.into(),
        }
    }

    pub fn is_ok(&self) -> bool {
        matches!(self, Self::Ok { .. })
    }
}

/// Per-model override for built-in/custom models.
#[derive(Debug, Clone, Default)]
pub struct ModelOverride {
    pub name: Option<String>,
    pub reasoning: Option<bool>,
    pub thinking_level_map: Option<ThinkingLevelMap>,
    pub input: Option<Vec<hamr_ai::types::InputModality>>,
    pub cost: Option<PartialModelCost>,
    pub context_window: Option<u64>,
    pub max_tokens: Option<u64>,
    pub headers: Option<HashMap<String, String>>,
    pub compat: Option<serde_json::Value>,
}

/// Partial cost override (all fields optional).
#[derive(Debug, Clone, Default)]
pub struct PartialModelCost {
    pub input: Option<f64>,
    pub output: Option<f64>,
    pub cache_read: Option<f64>,
    pub cache_write: Option<f64>,
}

/// A model definition as it appears in models.json or the registerProvider API.
#[derive(Debug, Clone)]
pub struct ModelDefinition {
    pub id: String,
    pub name: Option<String>,
    pub api: Option<Api>,
    pub base_url: Option<String>,
    pub reasoning: Option<bool>,
    pub thinking_level_map: Option<ThinkingLevelMap>,
    pub input: Option<Vec<hamr_ai::types::InputModality>>,
    pub cost: Option<ModelCost>,
    pub context_window: Option<u64>,
    pub max_tokens: Option<u64>,
    pub headers: Option<HashMap<String, String>>,
    pub compat: Option<serde_json::Value>,
}

/// Input type for `registerProvider` API (dynamic extensions).
// Debug is implemented manually because `stream_simple` contains dyn Fn.
#[derive(Clone)]
pub struct ProviderConfigInput {
    pub name: Option<String>,
    pub base_url: Option<String>,
    pub api_key: Option<String>,
    pub api: Option<Api>,
    /// Simple stream function — callable box stored for dynamic dispatch.
    #[allow(clippy::type_complexity)]
    pub stream_simple: Option<
        Arc<
            dyn Fn(StreamSimpleArgs) -> Pin<Box<dyn Future<Output = StreamSimpleResult> + Send>>
                + Send
                + Sync,
        >,
    >,
    pub headers: Option<HashMap<String, String>>,
    pub auth_header: Option<bool>,
    pub oauth: Option<serde_json::Value>,
    pub models: Option<Vec<ModelDefinition>>,
}

// ---------------------------------------------------------------------------
// Config value resolution — delegates to resolve_config_value.rs
// ---------------------------------------------------------------------------

use crate::core::resolve_config_value::{
    get_config_value_env_var_names, is_command_config_value, is_config_value_configured,
    resolve_config_value_or_throw, resolve_config_value_uncached, resolve_headers_or_throw,
};

/// Re-export clearing for tests.
pub use crate::core::resolve_config_value::clear_config_value_cache as clear_api_key_cache;

// ---------------------------------------------------------------------------
// Provider display names — delegates to provider_display_names.rs
// ---------------------------------------------------------------------------

use crate::core::provider_display_names::built_in_provider_display_names;

// ---------------------------------------------------------------------------
// Helper: strip JSON comments (supports ANSI / C-style comments)
// ---------------------------------------------------------------------------

/// Primitive JSON comment stripper: removes `//` and `/* */` comments.
/// Mirrors `stripJsonComments` from `packages/coding-agent/src/utils/json.ts`.
fn strip_json_comments(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut chars = input.char_indices().peekable();
    let mut in_line = false;
    let mut in_block = false;
    let mut in_string = false;
    let mut escaped = false;

    while let Some((_, c)) = chars.next() {
        if in_line {
            if c == '\n' {
                in_line = false;
                out.push(c);
            }
            continue;
        }
        if in_block {
            if c == '\n' {
                out.push(c);
                continue;
            }
            if c == '*' && chars.peek().map(|(_, n)| *n) == Some('/') {
                chars.next();
                in_block = false;
            }
            continue;
        }
        if in_string {
            out.push(c);
            if escaped {
                escaped = false;
            } else if c == '\\' {
                escaped = true;
            } else if c == '"' {
                in_string = false;
            }
            continue;
        }
        if c == '"' {
            in_string = true;
            out.push(c);
            continue;
        }
        if c == '/' {
            if let Some(&(_, '/')) = chars.peek() {
                chars.next();
                in_line = true;
                continue;
            }
            if let Some(&(_, '*')) = chars.peek() {
                chars.next();
                in_block = true;
                continue;
            }
        }
        out.push(c);
    }
    out
}

// ---------------------------------------------------------------------------
// Schema validation helpers
// ---------------------------------------------------------------------------

fn json_type_name(val: &serde_json::Value) -> &'static str {
    match val {
        serde_json::Value::Null => "null",
        serde_json::Value::Bool(_) => "boolean",
        serde_json::Value::Number(_) => "number",
        serde_json::Value::String(_) => "string",
        serde_json::Value::Array(_) => "array",
        serde_json::Value::Object(_) => "object",
    }
}

/// Parse an Api variant from a kebab-case string.
fn parse_api(s: &str) -> Result<Api, String> {
    match s {
        "openai-completions" => Ok(Api::OpenAiCompletions),
        "mistral-conversations" => Ok(Api::MistralConversations),
        "openai-responses" => Ok(Api::OpenAiResponses),
        "azure-openai-responses" => Ok(Api::AzureOpenAiResponses),
        "openai-codex-responses" => Ok(Api::OpenAiCodexResponses),
        "anthropic-messages" => Ok(Api::AnthropicMessages),
        "bedrock-converse-stream" => Ok(Api::BedrockConverseStream),
        "google-generative-ai" => Ok(Api::GoogleGenerativeAi),
        "google-vertex" => Ok(Api::GoogleVertex),
        other => Err(format!("unknown api \"{other}\"")),
    }
}

/// Validate models.json against the expected schema.
fn validate_models_json_schema(
    root: &serde_json::Map<String, serde_json::Value>,
    file_path: &str,
) -> Result<(), String> {
    let providers = match root.get("providers") {
        Some(serde_json::Value::Object(map)) => map,
        Some(other) => {
            return Err(format!(
                "Invalid models.json: \"providers\" must be an object, got {}\n\nFile: {file_path}",
                json_type_name(other)
            ));
        }
        None => {
            return Err(format!(
                "Invalid models.json: missing required field \"providers\"\n\nFile: {file_path}"
            ));
        }
    };

    let mut errors: Vec<String> = Vec::new();

    for (provider_name, provider_val) in providers {
        let provider_path = format!("providers.{provider_name}");
        let provider_map = match provider_val.as_object() {
            Some(m) => m,
            None => {
                errors.push(format!(
                    "  - {provider_path}: expected an object, got {}",
                    json_type_name(provider_val)
                ));
                continue;
            }
        };

        validate_provider_config(provider_map, &provider_path, &mut errors);

        // Validate models array
        if let Some(models_val) = provider_map.get("models") {
            if !models_val.is_null() {
                let models_array = match models_val.as_array() {
                    Some(a) => a,
                    None => {
                        errors.push(format!(
                            "  - {provider_path}.models: expected an array, got {}",
                            json_type_name(models_val)
                        ));
                        continue;
                    }
                };
                for (i, model_val) in models_array.iter().enumerate() {
                    let model_path = format!("{provider_path}.models[{i}]");
                    let model_map = match model_val.as_object() {
                        Some(o) => o,
                        None => {
                            errors.push(format!(
                                "  - {model_path}: expected an object, got {}",
                                json_type_name(model_val)
                            ));
                            continue;
                        }
                    };
                    validate_model_definition_fields(model_map, &model_path, &mut errors);
                }
            }
        }

        // Validate modelOverrides
        if let Some(ov_val) = provider_map.get("modelOverrides") {
            if !ov_val.is_null() {
                let ov_map = match ov_val.as_object() {
                    Some(o) => o,
                    None => {
                        errors.push(format!(
                            "  - {provider_path}.modelOverrides: expected an object, got {}",
                            json_type_name(ov_val)
                        ));
                        continue;
                    }
                };
                for (model_id, override_val) in ov_map {
                    let ov_path = format!("{provider_path}.modelOverrides.{model_id}");
                    let ov_map = match override_val.as_object() {
                        Some(o) => o,
                        None => {
                            errors.push(format!(
                                "  - {ov_path}: expected an object, got {}",
                                json_type_name(override_val)
                            ));
                            continue;
                        }
                    };
                    validate_model_override_fields(ov_map, &ov_path, &mut errors);
                }
            }
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "Invalid models.json schema:\n{}\n\nFile: {file_path}",
            errors.join("\n")
        ))
    }
}

fn validate_provider_config(
    map: &serde_json::Map<String, serde_json::Value>,
    path: &str,
    errors: &mut Vec<String>,
) {
    let allowed: &[&str] = &[
        "name",
        "baseUrl",
        "apiKey",
        "api",
        "headers",
        "compat",
        "authHeader",
        "models",
        "modelOverrides",
    ];
    for key in map.keys() {
        if !allowed.contains(&key.as_str()) {
            errors.push(format!("  - {path}: unknown field \"{key}\""));
        }
    }

    // Type-check simple fields
    type_check_opt_str(map, "name", path, errors);
    type_check_opt_str(map, "baseUrl", path, errors);
    type_check_opt_str(map, "apiKey", path, errors);
    type_check_opt_str(map, "api", path, errors);
    type_check_opt_bool(map, "authHeader", path, errors);
    type_check_string_map(map, "headers", path, errors);
}

fn type_check_opt_str(
    map: &serde_json::Map<String, serde_json::Value>,
    field: &str,
    parent: &str,
    errors: &mut Vec<String>,
) {
    if let Some(val) = map.get(field) {
        if !val.is_null() && !val.is_string() {
            errors.push(format!(
                "  - {parent}.{field}: expected a string, got {}",
                json_type_name(val)
            ));
        }
    }
}

fn type_check_opt_bool(
    map: &serde_json::Map<String, serde_json::Value>,
    field: &str,
    parent: &str,
    errors: &mut Vec<String>,
) {
    if let Some(val) = map.get(field) {
        if !val.is_null() && !val.is_boolean() {
            errors.push(format!(
                "  - {parent}.{field}: expected a boolean, got {}",
                json_type_name(val)
            ));
        }
    }
}

fn type_check_string_map(
    map: &serde_json::Map<String, serde_json::Value>,
    field: &str,
    parent: &str,
    errors: &mut Vec<String>,
) {
    if let Some(val) = map.get(field) {
        if val.is_null() {
            return;
        }
        let sub_map = match val.as_object() {
            Some(o) => o,
            None => {
                errors.push(format!(
                    "  - {parent}.{field}: expected an object, got {}",
                    json_type_name(val)
                ));
                return;
            }
        };
        for (k, v) in sub_map {
            if !v.is_string() {
                errors.push(format!(
                    "  - {parent}.{field}.{k}: expected a string, got {}",
                    json_type_name(v)
                ));
            }
        }
    }
}

fn validate_model_definition_fields(
    map: &serde_json::Map<String, serde_json::Value>,
    path: &str,
    errors: &mut Vec<String>,
) {
    // id is required
    match map.get("id") {
        Some(val) if val.is_string() && !val.as_str().unwrap_or("").is_empty() => {}
        Some(val) => {
            errors.push(format!(
                "  - {path}.id: must be a non-empty string, got {}",
                json_type_name(val)
            ));
        }
        None => {
            errors.push(format!("  - {path}: missing required field \"id\""));
        }
    }

    let allowed: &[&str] = &[
        "id",
        "name",
        "api",
        "baseUrl",
        "reasoning",
        "thinkingLevelMap",
        "input",
        "cost",
        "contextWindow",
        "maxTokens",
        "headers",
        "compat",
    ];
    for key in map.keys() {
        if !allowed.contains(&key.as_str()) {
            errors.push(format!("  - {path}: unknown field \"{key}\""));
        }
    }

    type_check_opt_str(map, "name", path, errors);
    type_check_opt_str(map, "api", path, errors);
    type_check_opt_str(map, "baseUrl", path, errors);
    type_check_opt_bool(map, "reasoning", path, errors);
    type_check_opt_str_map(map, "thinkingLevelMap", path, errors);
    type_check_input_array(map, "input", path, errors);
    type_check_cost(map, "cost", path, errors);
    type_check_opt_number(map, "contextWindow", path, errors);
    type_check_opt_number(map, "maxTokens", path, errors);
    type_check_string_map(map, "headers", path, errors);
}

fn validate_model_override_fields(
    map: &serde_json::Map<String, serde_json::Value>,
    path: &str,
    errors: &mut Vec<String>,
) {
    let allowed: &[&str] = &[
        "name",
        "reasoning",
        "thinkingLevelMap",
        "input",
        "cost",
        "contextWindow",
        "maxTokens",
        "headers",
        "compat",
    ];
    for key in map.keys() {
        if !allowed.contains(&key.as_str()) {
            errors.push(format!("  - {path}: unknown field \"{key}\""));
        }
    }

    type_check_opt_str(map, "name", path, errors);
    type_check_opt_bool(map, "reasoning", path, errors);
    type_check_opt_str_map(map, "thinkingLevelMap", path, errors);
    type_check_input_array(map, "input", path, errors);
    type_check_partial_cost(map, "cost", path, errors);
    type_check_opt_number(map, "contextWindow", path, errors);
    type_check_opt_number(map, "maxTokens", path, errors);
    type_check_string_map(map, "headers", path, errors);
}

fn type_check_opt_number(
    map: &serde_json::Map<String, serde_json::Value>,
    field: &str,
    parent: &str,
    errors: &mut Vec<String>,
) {
    if let Some(val) = map.get(field) {
        if val.is_null() {
            return;
        }
        if !val.is_number() {
            errors.push(format!(
                "  - {parent}.{field}: expected a number, got {}",
                json_type_name(val)
            ));
        } else if let Some(n) = val.as_f64() {
            if n < 0.0 {
                errors.push(format!(
                    "  - {parent}.{field}: must be non-negative, got {n}"
                ));
            }
        }
    }
}

fn type_check_opt_str_map(
    map: &serde_json::Map<String, serde_json::Value>,
    field: &str,
    parent: &str,
    errors: &mut Vec<String>,
) {
    if let Some(val) = map.get(field) {
        if val.is_null() {
            return;
        }
        let sub_map = match val.as_object() {
            Some(o) => o,
            None => {
                errors.push(format!(
                    "  - {parent}.{field}: expected an object, got {}",
                    json_type_name(val)
                ));
                return;
            }
        };
        let allowed_keys: &[&str] = &["off", "minimal", "low", "medium", "high", "xhigh"];
        for (k, v) in sub_map {
            if !allowed_keys.contains(&k.as_str()) {
                errors.push(format!("  - {parent}.{field}: unknown key \"{k}\""));
            }
            if !v.is_null() && !v.is_string() {
                errors.push(format!(
                    "  - {parent}.{field}.{k}: expected a string or null, got {}",
                    json_type_name(v)
                ));
            }
        }
    }
}

fn type_check_input_array(
    map: &serde_json::Map<String, serde_json::Value>,
    field: &str,
    parent: &str,
    errors: &mut Vec<String>,
) {
    if let Some(val) = map.get(field) {
        if val.is_null() {
            return;
        }
        let arr = match val.as_array() {
            Some(a) => a,
            None => {
                errors.push(format!(
                    "  - {parent}.{field}: expected an array, got {}",
                    json_type_name(val)
                ));
                return;
            }
        };
        for (i, item) in arr.iter().enumerate() {
            match item.as_str() {
                Some("text") | Some("image") => {}
                Some(other) => {
                    errors.push(format!(
                        "  - {parent}.{field}[{i}]: expected \"text\" or \"image\", got \"{other}\""
                    ));
                }
                None => {
                    errors.push(format!(
                        "  - {parent}.{field}[{i}]: expected a string, got {}",
                        json_type_name(item)
                    ));
                }
            }
        }
    }
}

fn type_check_cost(
    map: &serde_json::Map<String, serde_json::Value>,
    field: &str,
    parent: &str,
    errors: &mut Vec<String>,
) {
    if let Some(val) = map.get(field) {
        if val.is_null() {
            return;
        }
        let sub = match val.as_object() {
            Some(o) => o,
            None => {
                errors.push(format!(
                    "  - {parent}.{field}: expected an object, got {}",
                    json_type_name(val)
                ));
                return;
            }
        };
        let required = ["input", "output", "cacheRead", "cacheWrite"];
        let allowed = ["input", "output", "cacheRead", "cacheWrite"];
        for r in &required {
            match sub.get(*r) {
                Some(v) if v.is_number() => {}
                Some(v) => errors.push(format!(
                    "  - {parent}.{field}.{r}: expected a number, got {}",
                    json_type_name(v)
                )),
                None => errors.push(format!(
                    "  - {parent}.{field}: missing required field \"{r}\""
                )),
            }
        }
        for k in sub.keys() {
            if !allowed.contains(&k.as_str()) {
                errors.push(format!("  - {parent}.{field}: unknown field \"{k}\""));
            }
        }
    }
}

fn type_check_partial_cost(
    map: &serde_json::Map<String, serde_json::Value>,
    field: &str,
    parent: &str,
    errors: &mut Vec<String>,
) {
    if let Some(val) = map.get(field) {
        if val.is_null() {
            return;
        }
        let sub = match val.as_object() {
            Some(o) => o,
            None => {
                errors.push(format!(
                    "  - {parent}.{field}: expected an object, got {}",
                    json_type_name(val)
                ));
                return;
            }
        };
        let allowed = ["input", "output", "cacheRead", "cacheWrite"];
        for k in sub.keys() {
            if !allowed.contains(&k.as_str()) {
                errors.push(format!("  - {parent}.{field}: unknown field \"{k}\""));
            } else if !sub[k].is_null() && !sub[k].is_number() {
                errors.push(format!(
                    "  - {parent}.{field}.{k}: expected a number or null, got {}",
                    json_type_name(&sub[k])
                ));
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Parsing helpers
// ---------------------------------------------------------------------------

fn parse_thinking_level(s: &str) -> Option<hamr_ai::types::ModelThinkingLevel> {
    match s {
        "off" => Some(hamr_ai::types::ModelThinkingLevel::Off),
        "minimal" => Some(hamr_ai::types::ModelThinkingLevel::Minimal),
        "low" => Some(hamr_ai::types::ModelThinkingLevel::Low),
        "medium" => Some(hamr_ai::types::ModelThinkingLevel::Medium),
        "high" => Some(hamr_ai::types::ModelThinkingLevel::High),
        "xhigh" => Some(hamr_ai::types::ModelThinkingLevel::XHigh),
        _ => None,
    }
}

fn parse_thinking_level_map(val: &serde_json::Value) -> Option<ThinkingLevelMap> {
    let map = val.as_object()?;
    let mut result = ThinkingLevelMap::new();
    for (key, v) in map {
        if let Some(level) = parse_thinking_level(key) {
            match v {
                serde_json::Value::String(s) if !s.is_empty() => {
                    result.insert(level, Some(s.clone()));
                }
                serde_json::Value::Null => {
                    result.insert(level, None);
                }
                _ => {}
            }
        }
    }
    if result.is_empty() {
        None
    } else {
        Some(result)
    }
}

fn parse_input_modalities(val: &serde_json::Value) -> Option<Vec<hamr_ai::types::InputModality>> {
    let arr = val.as_array()?;
    let mut result = Vec::new();
    for item in arr {
        match item.as_str() {
            Some("text") => result.push(hamr_ai::types::InputModality::Text),
            Some("image") => result.push(hamr_ai::types::InputModality::Image),
            _ => {}
        }
    }
    if result.is_empty() {
        None
    } else {
        Some(result)
    }
}

fn parse_model_cost(val: &serde_json::Value) -> Option<ModelCost> {
    let map = val.as_object()?;
    Some(ModelCost {
        input: map.get("input")?.as_f64()?,
        output: map.get("output")?.as_f64()?,
        cache_read: map.get("cacheRead")?.as_f64()?,
        cache_write: map.get("cacheWrite")?.as_f64()?,
    })
}

fn parse_partial_cost(val: &serde_json::Value) -> PartialModelCost {
    let mut pc = PartialModelCost::default();
    if let Some(map) = val.as_object() {
        if let Some(n) = map.get("input").and_then(|v| v.as_f64()) {
            pc.input = Some(n);
        }
        if let Some(n) = map.get("output").and_then(|v| v.as_f64()) {
            pc.output = Some(n);
        }
        if let Some(n) = map.get("cacheRead").and_then(|v| v.as_f64()) {
            pc.cache_read = Some(n);
        }
        if let Some(n) = map.get("cacheWrite").and_then(|v| v.as_f64()) {
            pc.cache_write = Some(n);
        }
    }
    pc
}

fn parse_string_map(val: &serde_json::Value) -> Option<HashMap<String, String>> {
    let map = val.as_object()?;
    let mut result = HashMap::new();
    for (k, v) in map {
        if let Some(s) = v.as_str() {
            result.insert(k.clone(), s.to_string());
        }
    }
    if result.is_empty() {
        None
    } else {
        Some(result)
    }
}

// ---------------------------------------------------------------------------
// Custom models loading result
// ---------------------------------------------------------------------------

struct CustomModelsResult {
    models: Vec<ModelWithCompat>,
    overrides: HashMap<String, ProviderOverride>,
    model_overrides: HashMap<String, HashMap<String, ModelOverride>>,
    error: Option<String>,
}

fn empty_custom_models_result(error: Option<String>) -> CustomModelsResult {
    CustomModelsResult {
        models: Vec::new(),
        overrides: HashMap::new(),
        model_overrides: HashMap::new(),
        error,
    }
}

// ---------------------------------------------------------------------------
// Compat merging
// ---------------------------------------------------------------------------

/// Deep merge two JSON values (object-level merge).
fn deep_merge_json(base: &serde_json::Value, over: &serde_json::Value) -> serde_json::Value {
    match (base, over) {
        (serde_json::Value::Object(b_map), serde_json::Value::Object(o_map)) => {
            let mut result = b_map.clone();
            for (k, v) in o_map {
                if let Some(bv) = b_map.get(k) {
                    result.insert(k.clone(), deep_merge_json(bv, v));
                } else {
                    result.insert(k.clone(), v.clone());
                }
            }
            serde_json::Value::Object(result)
        }
        (_, over) => over.clone(),
    }
}

/// Merge compat objects, with special handling for nested routing objects.
fn merge_compat(
    base_compat: Option<serde_json::Value>,
    override_compat: Option<serde_json::Value>,
) -> Option<serde_json::Value> {
    match (base_compat, override_compat) {
        (None, None) => None,
        (Some(b), None) => Some(b),
        (None, Some(o)) => Some(o),
        (Some(base), Some(over)) => {
            match (base, over) {
                (serde_json::Value::Object(mut b_map), serde_json::Value::Object(o_map)) => {
                    // Deep-merge nested routing configs
                    for nested_key in &["openRouterRouting", "vercelGatewayRouting"] {
                        if let Some(or_val) = o_map.get(*nested_key) {
                            if let Some(br_val) = b_map.get(*nested_key) {
                                b_map.insert(
                                    nested_key.to_string(),
                                    deep_merge_json(br_val, or_val),
                                );
                            } else {
                                b_map.insert(nested_key.to_string(), or_val.clone());
                            }
                        }
                    }
                    // Override wins for all other fields.
                    for (k, v) in o_map {
                        if k != "openRouterRouting" && k != "vercelGatewayRouting" {
                            b_map.insert(k, v.clone());
                        }
                    }
                    Some(serde_json::Value::Object(b_map))
                }
                (_, over) => Some(over),
            }
        }
    }
}

/// Apply a model override to a model (deep merge).
fn apply_model_override(model: &Model, override_: &ModelOverride) -> Model {
    let mut result = model.clone();

    if let Some(name) = &override_.name {
        result.name = name.clone();
    }
    if let Some(reasoning) = override_.reasoning {
        result.reasoning = reasoning;
    }
    if let Some(tlm) = &override_.thinking_level_map {
        let mut merged = model.thinking_level_map.clone().unwrap_or_default();
        merged.extend(tlm.iter().map(|(k, v)| (*k, v.clone())));
        result.thinking_level_map = Some(merged);
    }
    if let Some(input) = &override_.input {
        result.input = input.clone();
    }
    if let Some(cost) = &override_.cost {
        result.cost = ModelCost {
            input: cost.input.unwrap_or(model.cost.input),
            output: cost.output.unwrap_or(model.cost.output),
            cache_read: cost.cache_read.unwrap_or(model.cost.cache_read),
            cache_write: cost.cache_write.unwrap_or(model.cost.cache_write),
        };
    }
    if let Some(ctx) = override_.context_window {
        result.context_window = ctx;
    }
    if let Some(mt) = override_.max_tokens {
        result.max_tokens = mt;
    }
    // Deep-merge compat: if both are objects, merge keys (override wins);
    // otherwise override replaces entirely.
    if let Some(ref compat) = override_.compat {
        match (&result.compat, compat) {
            (Some(serde_json::Value::Object(base)), serde_json::Value::Object(ov)) => {
                let mut merged = base.clone();
                for (k, v) in ov {
                    merged.insert(k.clone(), v.clone());
                }
                result.compat = Some(serde_json::Value::Object(merged));
            }
            _ => {
                result.compat = Some(compat.clone());
            }
        }
    }

    result
}

// ---------------------------------------------------------------------------
// Agent directory helpers
// ---------------------------------------------------------------------------

const HAMR_CONFIG_DIR: &str = ".hamr";
const AGENT_DIR_NAME: &str = "agent";

fn get_agent_dir() -> String {
    if let Ok(dir) = std::env::var("HAMR_AGENT_DIR") {
        return dir;
    }
    if let Ok(dir) = std::env::var("PI_AGENT_DIR") {
        return dir;
    }
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    format!("{home}/{HAMR_CONFIG_DIR}/{AGENT_DIR_NAME}")
}

// ---------------------------------------------------------------------------
// Parsed config types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default)]
struct ModelsJsonConfig {
    providers: HashMap<String, ProviderJsonConfig>,
}

#[derive(Debug, Clone, Default)]
struct ProviderJsonConfig {
    name: Option<String>,
    base_url: Option<String>,
    api_key: Option<String>,
    api: Option<Api>,
    headers: Option<HashMap<String, String>>,
    compat: Option<serde_json::Value>,
    auth_header: Option<bool>,
    models: Vec<JsonModelDefinition>,
    model_overrides: Option<HashMap<String, ModelOverride>>,
}

#[derive(Debug, Clone, Default)]
struct JsonModelDefinition {
    id: String,
    name: Option<String>,
    api: Option<Api>,
    base_url: Option<String>,
    reasoning: Option<bool>,
    thinking_level_map: Option<ThinkingLevelMap>,
    input: Option<Vec<hamr_ai::types::InputModality>>,
    cost: Option<ModelCost>,
    context_window: Option<u64>,
    max_tokens: Option<u64>,
    headers: Option<HashMap<String, String>>,
    compat: Option<serde_json::Value>,
}

impl ModelsJsonConfig {
    fn from_value(root: &serde_json::Value) -> Self {
        let mut config = ModelsJsonConfig::default();

        if let Some(providers) = root.get("providers").and_then(|v| v.as_object()) {
            for (provider_name, provider_val) in providers {
                let mut pc = ProviderJsonConfig::default();

                if let Some(map) = provider_val.as_object() {
                    if let Some(val) = map.get("name") {
                        pc.name = val.as_str().map(String::from);
                    }
                    if let Some(val) = map.get("baseUrl") {
                        pc.base_url = val.as_str().map(String::from);
                    }
                    if let Some(val) = map.get("apiKey") {
                        pc.api_key = val.as_str().map(String::from);
                    }
                    if let Some(val) = map.get("api") {
                        pc.api = val.as_str().and_then(|s| parse_api(s).ok());
                    }
                    if let Some(val) = map.get("headers") {
                        pc.headers = parse_string_map(val);
                    }
                    if let Some(val) = map.get("compat") {
                        if !val.is_null() {
                            pc.compat = Some(val.clone());
                        }
                    }
                    if let Some(val) = map.get("authHeader") {
                        pc.auth_header = val.as_bool();
                    }
                    if let Some(models_val) = map.get("models") {
                        if let Some(models_arr) = models_val.as_array() {
                            for model_val in models_arr {
                                if let Some(model_map) = model_val.as_object() {
                                    let mut md = JsonModelDefinition::default();
                                    if let Some(val) = model_map.get("id") {
                                        md.id = val.as_str().unwrap_or("").to_string();
                                    }
                                    if let Some(val) = model_map.get("name") {
                                        md.name = val.as_str().map(String::from);
                                    }
                                    if let Some(val) = model_map.get("api") {
                                        md.api = val.as_str().and_then(|s| parse_api(s).ok());
                                    }
                                    if let Some(val) = model_map.get("baseUrl") {
                                        md.base_url = val.as_str().map(String::from);
                                    }
                                    if let Some(val) = model_map.get("reasoning") {
                                        md.reasoning = val.as_bool();
                                    }
                                    if let Some(val) = model_map.get("thinkingLevelMap") {
                                        md.thinking_level_map = parse_thinking_level_map(val);
                                    }
                                    if let Some(val) = model_map.get("input") {
                                        md.input = parse_input_modalities(val);
                                    }
                                    if let Some(val) = model_map.get("cost") {
                                        md.cost = parse_model_cost(val);
                                    }
                                    if let Some(val) = model_map.get("contextWindow") {
                                        md.context_window = val.as_u64();
                                    }
                                    if let Some(val) = model_map.get("maxTokens") {
                                        md.max_tokens = val.as_u64();
                                    }
                                    if let Some(val) = model_map.get("headers") {
                                        md.headers = parse_string_map(val);
                                    }
                                    if let Some(val) = model_map.get("compat") {
                                        if !val.is_null() {
                                            md.compat = Some(val.clone());
                                        }
                                    }
                                    pc.models.push(md);
                                }
                            }
                        }
                    }
                    if let Some(ov_val) = map.get("modelOverrides") {
                        if let Some(ov_map) = ov_val.as_object() {
                            let mut overrides = HashMap::new();
                            for (model_id, override_val) in ov_map {
                                if let Some(om) = override_val.as_object() {
                                    let mut mo = ModelOverride::default();
                                    if let Some(val) = om.get("name") {
                                        mo.name = val.as_str().map(String::from);
                                    }
                                    if let Some(val) = om.get("reasoning") {
                                        mo.reasoning = val.as_bool();
                                    }
                                    if let Some(val) = om.get("thinkingLevelMap") {
                                        mo.thinking_level_map = parse_thinking_level_map(val);
                                    }
                                    if let Some(val) = om.get("input") {
                                        mo.input = parse_input_modalities(val);
                                    }
                                    if let Some(val) = om.get("cost") {
                                        mo.cost = Some(parse_partial_cost(val));
                                    }
                                    if let Some(val) = om.get("contextWindow") {
                                        mo.context_window = val.as_u64();
                                    }
                                    if let Some(val) = om.get("maxTokens") {
                                        mo.max_tokens = val.as_u64();
                                    }
                                    if let Some(val) = om.get("headers") {
                                        mo.headers = parse_string_map(val);
                                    }
                                    if let Some(val) = om.get("compat") {
                                        if !val.is_null() {
                                            mo.compat = Some(val.clone());
                                        }
                                    }
                                    overrides.insert(model_id.clone(), mo);
                                }
                            }
                            if !overrides.is_empty() {
                                pc.model_overrides = Some(overrides);
                            }
                        }
                    }
                }

                config.providers.insert(provider_name.clone(), pc);
            }
        }

        config
    }
}

// ---------------------------------------------------------------------------
// ModelRegistry
// ---------------------------------------------------------------------------

/// Model registry — loads and manages models, resolves API keys via AuthStorage.
///
/// This is the central model catalogue for the agent. It holds both built-in
/// and custom models, merges provider-level and per-model overrides from
/// `models.json`, and resolves request auth (API keys + headers) at runtime.
pub struct ModelRegistry {
    models: Vec<ModelWithCompat>,
    provider_request_configs: HashMap<String, ProviderRequestConfig>,
    model_request_headers: HashMap<String, HashMap<String, String>>,
    registered_providers: HashMap<String, ProviderConfigInput>,
    load_error: Option<String>,
    auth_storage: Arc<dyn AuthStorage>,
    models_json_path: Option<String>,
}

/// Wrapper around Model that carries compat JSON and headers for auth resolution.
#[derive(Debug, Clone)]
struct ModelWithCompat {
    model: Model,
    /// Raw compat JSON (kept as serde_json::Value since the Rust Model type
    /// doesn't have a `compat` field yet).
    compat: Option<serde_json::Value>,
}

impl ModelWithCompat {
    fn to_model(&self) -> Model {
        self.model.clone()
    }
}

impl ModelRegistry {
    // -----------------------------------------------------------------------
    // Constructors
    // -----------------------------------------------------------------------

    /// Create a registry that reads custom models from `models.json`.
    pub fn create(auth_storage: Arc<dyn AuthStorage>, models_json_path: String) -> Self {
        let mut registry = ModelRegistry {
            models: Vec::new(),
            provider_request_configs: HashMap::new(),
            model_request_headers: HashMap::new(),
            registered_providers: HashMap::new(),
            load_error: None,
            auth_storage,
            models_json_path: Some(models_json_path),
        };
        registry.load_models();
        registry
    }

    /// Create a registry without any models.json file (in-memory only).
    pub fn in_memory(auth_storage: Arc<dyn AuthStorage>) -> Self {
        let mut registry = ModelRegistry {
            models: Vec::new(),
            provider_request_configs: HashMap::new(),
            model_request_headers: HashMap::new(),
            registered_providers: HashMap::new(),
            load_error: None,
            auth_storage,
            models_json_path: None,
        };
        registry.load_models();
        registry
    }

    // -----------------------------------------------------------------------
    // Refresh
    // -----------------------------------------------------------------------

    /// Reload models from disk and re-apply registered providers.
    ///
    /// Resets API/OAuth registrations and rebuilds the model catalogue from
    /// built-in data and `models.json` (if present), then re-applies all
    /// dynamically registered providers.
    pub fn refresh(&mut self) {
        self.provider_request_configs.clear();
        self.model_request_headers.clear();
        self.load_error = None;

        // Reset API and OAuth registrations.
        hamr_ai::providers::register_builtins::reset_api_providers();
        hamr_ai::oauth::reset_oauth_providers();

        self.load_models();

        // Re-apply all dynamically registered providers.
        let snap = self.registered_providers.clone();
        for (provider_name, config) in snap {
            self.apply_provider_config(&provider_name, &config);
        }
    }

    /// Get any error from loading `models.json` (None if successful).
    pub fn get_error(&self) -> Option<&str> {
        self.load_error.as_deref()
    }

    // -----------------------------------------------------------------------
    // Model loading
    // -----------------------------------------------------------------------

    fn load_models(&mut self) {
        let path_opt = self.models_json_path.clone();
        let result = match &path_opt {
            Some(path) => self.load_custom_models(path),
            None => empty_custom_models_result(None),
        };

        if let Some(ref err) = result.error {
            self.load_error = Some(err.clone());
        }

        let built_in = self.load_built_in_models(&result.overrides, &result.model_overrides);
        let combined = self.merge_custom_models(built_in, result.models);

        // Let OAuth providers modify their models (e.g. update baseUrl).
        for oauth_provider in self.auth_storage.get_oauth_providers() {
            let pid = oauth_provider
                .get("id")
                .and_then(|v| v.as_str())
                .map(String::from);
            if let Some(ref provider_id) = pid {
                let cred = self.auth_storage.get(provider_id);
                if let Some(AuthCredential::OAuth { .. }) = cred {
                    // TODO: call oauth_provider.modifyModels once wired.
                }
            }
        }

        self.models = combined;
    }

    /// Load built-in models and apply provider/model overrides.
    fn load_built_in_models(
        &self,
        overrides: &HashMap<String, ProviderOverride>,
        model_overrides: &HashMap<String, HashMap<String, ModelOverride>>,
    ) -> Vec<ModelWithCompat> {
        let providers = get_providers();
        let mut result = Vec::new();

        for provider in &providers {
            let models = get_models(provider);
            let provider_override = overrides.get(provider);
            let per_model_overrides = model_overrides.get(provider);

            for m in models {
                let mut model_wc = ModelWithCompat {
                    model: m.clone(),
                    compat: None,
                };

                // Apply provider-level baseUrl/headers/compat override.
                if let Some(po) = provider_override {
                    if let Some(ref bu) = po.base_url {
                        model_wc.model.base_url = bu.clone();
                    }
                    model_wc.compat = merge_compat(model_wc.compat.clone(), po.compat.clone());
                }

                // Apply per-model override.
                if let Some(model_override) =
                    per_model_overrides.and_then(|m| m.get(&model_wc.model.id))
                {
                    model_wc.model = apply_model_override(&model_wc.model, model_override);
                }

                result.push(model_wc);
            }
        }

        result
    }

    /// Merge custom models into built-in list by provider+id (custom wins on conflicts).
    fn merge_custom_models(
        &self,
        built_in: Vec<ModelWithCompat>,
        custom: Vec<ModelWithCompat>,
    ) -> Vec<ModelWithCompat> {
        let mut merged = built_in;
        for custom_model in custom {
            if let Some(idx) = merged.iter().position(|m| {
                m.model.provider == custom_model.model.provider
                    && m.model.id == custom_model.model.id
            }) {
                merged[idx] = custom_model;
            } else {
                merged.push(custom_model);
            }
        }
        merged
    }

    fn load_custom_models(&mut self, models_json_path: &str) -> CustomModelsResult {
        let path = Path::new(models_json_path);
        if !path.exists() {
            return empty_custom_models_result(None);
        }

        let content = match fs::read_to_string(path) {
            Ok(c) => c,
            Err(e) => {
                return empty_custom_models_result(Some(format!(
                    "Failed to load models.json: {}\n\nFile: {}",
                    e, models_json_path
                )));
            }
        };

        let stripped = strip_json_comments(&content);
        let parsed: serde_json::Value = match serde_json::from_str(&stripped) {
            Ok(v) => v,
            Err(e) => {
                return empty_custom_models_result(Some(format!(
                    "Failed to parse models.json: {}\n\nFile: {}",
                    e, models_json_path
                )));
            }
        };

        // Validate schema.
        if let Some(root_map) = parsed.as_object() {
            if let Err(err) = validate_models_json_schema(root_map, models_json_path) {
                return empty_custom_models_result(Some(err));
            }
        }

        let config = ModelsJsonConfig::from_value(&parsed);
        let providers = get_providers();

        if let Err(err) = self.validate_json_config(&config, &providers) {
            return empty_custom_models_result(Some(format!(
                "Invalid models.json configuration:\n{}\n\nFile: {}",
                err, models_json_path
            )));
        }

        let mut overrides = HashMap::new();
        let mut model_overrides: HashMap<String, HashMap<String, ModelOverride>> = HashMap::new();

        for (provider_name, provider_config) in &config.providers {
            if provider_config.base_url.is_some() || provider_config.compat.is_some() {
                overrides.insert(
                    provider_name.clone(),
                    ProviderOverride {
                        base_url: provider_config.base_url.clone(),
                        compat: provider_config.compat.clone(),
                    },
                );
            }

            self.store_provider_request_config(provider_name, provider_config);

            if let Some(ref mov) = provider_config.model_overrides {
                model_overrides.insert(provider_name.clone(), mov.clone());
                for (model_id, model_override) in mov {
                    self.store_model_headers(provider_name, model_id, &model_override.headers);
                }
            }
        }

        let models = self.parse_models(&config);

        CustomModelsResult {
            models,
            overrides,
            model_overrides,
            error: None,
        }
    }

    fn validate_json_config(
        &self,
        config: &ModelsJsonConfig,
        built_in_providers: &[String],
    ) -> Result<(), String> {
        let built_in_set: std::collections::HashSet<&str> =
            built_in_providers.iter().map(|s| s.as_str()).collect();

        for (provider_name, provider_config) in &config.providers {
            let is_built_in = built_in_set.contains(provider_name.as_str());
            let models = &provider_config.models;
            let has_model_overrides = provider_config
                .model_overrides
                .as_ref()
                .map(|m| !m.is_empty())
                .unwrap_or(false);

            if models.is_empty() {
                // Override-only config: needs at least one of baseUrl, headers, compat, modelOverrides.
                if provider_config.base_url.is_none()
                    && provider_config.headers.is_none()
                    && provider_config.compat.is_none()
                    && !has_model_overrides
                {
                    return Err(format!(
                        "Provider {provider_name}: must specify \"baseUrl\", \"headers\", \"compat\", \"modelOverrides\", or \"models\".",
                    ));
                }
            } else if !is_built_in {
                // Non-built-in providers with custom models require endpoint + auth.
                if provider_config.base_url.is_none() {
                    return Err(format!(
                        "Provider {provider_name}: \"baseUrl\" is required when defining custom models.",
                    ));
                }
                if provider_config.api_key.is_none() {
                    return Err(format!(
                        "Provider {provider_name}: \"apiKey\" is required when defining custom models.",
                    ));
                }
            }

            for model_def in models {
                let has_model_api = model_def.api.is_some();

                if !provider_config.api.is_some() && !has_model_api && !is_built_in {
                    return Err(format!(
                        "Provider {provider_name}, model {}: no \"api\" specified. Set at provider or model level.",
                        model_def.id,
                    ));
                }

                if model_def.id.is_empty() {
                    return Err(format!("Provider {provider_name}: model missing \"id\""));
                }
                if let Some(ctx) = model_def.context_window {
                    if ctx == 0 {
                        return Err(format!(
                            "Provider {provider_name}, model {}: invalid contextWindow",
                            model_def.id,
                        ));
                    }
                }
                if let Some(mt) = model_def.max_tokens {
                    if mt == 0 {
                        return Err(format!(
                            "Provider {provider_name}, model {}: invalid maxTokens",
                            model_def.id,
                        ));
                    }
                }
            }
        }

        Ok(())
    }

    fn parse_models(&mut self, config: &ModelsJsonConfig) -> Vec<ModelWithCompat> {
        let mut models = Vec::new();
        let providers = get_providers();
        let built_in_set: std::collections::HashSet<&str> =
            providers.iter().map(|s| s.as_str()).collect();

        // Cache built-in defaults (api, baseUrl) per provider.
        let mut built_in_defaults_cache: HashMap<String, (Api, String)> = HashMap::new();
        let get_built_in_defaults = |cache: &mut HashMap<String, (Api, String)>,
                                     provider_name: &str|
         -> Option<(Api, String)> {
            if !built_in_set.contains(provider_name) {
                return None;
            }
            if let Some(def) = cache.get(provider_name) {
                return Some(def.clone());
            }
            let built_in = get_models(provider_name);
            if let Some(first) = built_in.first() {
                let defaults = (first.api, first.base_url.clone());
                cache.insert(provider_name.to_string(), defaults.clone());
                Some(defaults)
            } else {
                None
            }
        };

        for (provider_name, provider_config) in &config.providers {
            let model_defs = &provider_config.models;
            if model_defs.is_empty() {
                continue; // Override-only, no custom models.
            }

            let built_in_defaults =
                get_built_in_defaults(&mut built_in_defaults_cache, provider_name);

            for model_def in model_defs {
                let api = model_def
                    .api
                    .or(provider_config.api)
                    .or(built_in_defaults.as_ref().map(|(a, _)| *a));
                let api = match api {
                    Some(a) => a,
                    None => continue,
                };

                let base_url = model_def
                    .base_url
                    .clone()
                    .or_else(|| provider_config.base_url.clone())
                    .or_else(|| built_in_defaults.as_ref().map(|(_, b)| b.clone()));
                let base_url = match base_url {
                    Some(bu) => bu,
                    None => continue,
                };

                let compat = merge_compat(provider_config.compat.clone(), model_def.compat.clone());
                self.store_model_headers(provider_name, &model_def.id, &model_def.headers);

                let name = model_def
                    .name
                    .clone()
                    .unwrap_or_else(|| model_def.id.clone());
                let reasoning = model_def.reasoning.unwrap_or(false);
                let input = model_def
                    .input
                    .clone()
                    .unwrap_or_else(|| vec![hamr_ai::types::InputModality::Text]);
                let cost = model_def.cost.clone().unwrap_or(ModelCost {
                    input: 0.0,
                    output: 0.0,
                    cache_read: 0.0,
                    cache_write: 0.0,
                });
                let context_window = model_def.context_window.unwrap_or(128_000);
                let max_tokens = model_def.max_tokens.unwrap_or(16_384);

                models.push(ModelWithCompat {
                    model: Model {
                        id: model_def.id.clone(),
                        name,
                        api,
                        provider: provider_name.clone(),
                        base_url,
                        reasoning,
                        thinking_level_map: model_def.thinking_level_map.clone(),
                        input,
                        cost,
                        context_window,
                        max_tokens,
                        headers: None,
            compat: None,
                    },
                    compat,
                });
            }
        }

        models
    }

    // -----------------------------------------------------------------------
    // Query methods
    // -----------------------------------------------------------------------

    /// Get all models (built-in + custom).
    /// If models.json had errors, returns only built-in models.
    pub fn get_all(&self) -> Vec<Model> {
        self.models.iter().map(|mwc| mwc.to_model()).collect()
    }

    /// Get only models that have auth configured.
    pub fn get_available(&self) -> Vec<Model> {
        self.models
            .iter()
            .filter(|mwc| self.has_configured_auth(mwc))
            .map(|mwc| mwc.to_model())
            .collect()
    }

    /// Find a model by provider and ID.
    pub fn find(&self, provider: &str, model_id: &str) -> Option<Model> {
        self.models
            .iter()
            .find(|mwc| mwc.model.provider == provider && mwc.model.id == model_id)
            .map(|mwc| mwc.to_model())
    }

    // -----------------------------------------------------------------------
    // Auth checks
    // -----------------------------------------------------------------------

    /// Check if auth is configured for a model's provider.
    fn has_configured_auth(&self, mwc: &ModelWithCompat) -> bool {
        let provider_api_key = self
            .provider_request_configs
            .get(&mwc.model.provider)
            .and_then(|c| c.api_key.as_deref());

        self.auth_storage.has_auth(&mwc.model.provider)
            || (provider_api_key.is_some()
                && is_config_value_configured(
                    provider_api_key.unwrap(),
                    self.auth_storage
                        .get_provider_env(&mwc.model.provider)
                        .as_ref(),
                ))
    }

    // -----------------------------------------------------------------------
    // API key and header resolution
    // -----------------------------------------------------------------------

    /// Get API key and request headers for a model.
    pub async fn get_api_key_and_headers(&self, model: &Model) -> ResolvedRequestAuth {
        self.get_api_key_and_headers_impl(model)
            .await
            .unwrap_or_else(|err| ResolvedRequestAuth::err(err))
    }

    /// Internal implementation that returns errors via Result.
    async fn get_api_key_and_headers_impl(
        &self,
        model: &Model,
    ) -> Result<ResolvedRequestAuth, String> {
        let provider_config = self.provider_request_configs.get(&model.provider);
        let provider_env = self.auth_storage.get_provider_env(&model.provider);

        let api_key_from_auth = self.auth_storage.get_api_key(&model.provider, false).await;

        let api_key = match api_key_from_auth {
            Some(k) => Some(k),
            None => match provider_config.and_then(|c| c.api_key.as_deref()) {
                Some(cfg_key) => Some(resolve_config_value_or_throw(
                    cfg_key,
                    &format!("API key for provider \"{}\"", model.provider),
                    provider_env.as_ref(),
                )?),
                None => None,
            },
        };

        let provider_headers = resolve_headers_or_throw(
            provider_config.and_then(|c| c.headers.as_ref()),
            &format!("provider \"{}\"", model.provider),
            provider_env.as_ref(),
        )?;

        let model_headers_key = self.get_model_request_key(&model.provider, &model.id);
        let model_headers = resolve_headers_or_throw(
            self.model_request_headers.get(&model_headers_key),
            &format!("model \"{}/{}\"", model.provider, model.id),
            provider_env.as_ref(),
        )?;

        let mut headers: HashMap<String, String> = HashMap::new();
        // Merge: model.headers → provider_headers → model_headers (later wins on conflict).
        if let Some(ref h) = model.headers {
            headers.extend(h.iter().map(|(k, v)| (k.clone(), v.clone())));
        }
        if let Some(ref h) = provider_headers {
            headers.extend(h.iter().map(|(k, v)| (k.clone(), v.clone())));
        }
        if let Some(ref h) = model_headers {
            headers.extend(h.iter().map(|(k, v)| (k.clone(), v.clone())));
        }

        if provider_config.and_then(|c| c.auth_header).unwrap_or(false) {
            let key = match &api_key {
                Some(k) => k.clone(),
                None => {
                    return Ok(ResolvedRequestAuth::err(format!(
                        "No API key found for \"{}\"",
                        model.provider
                    )));
                }
            };
            headers.insert("Authorization".to_string(), format!("Bearer {key}"));
        }

        Ok(ResolvedRequestAuth::ok(
            api_key,
            if headers.is_empty() {
                None
            } else {
                Some(headers)
            },
            if let Some(ref env) = provider_env {
                if env.is_empty() {
                    None
                } else {
                    Some(env.clone())
                }
            } else {
                None
            },
        ))
    }

    /// Get auth status for a provider, including request auth from models.json.
    pub fn get_provider_auth_status(&self, provider: &str) -> AuthStatus {
        let auth_status = self.auth_storage.get_auth_status(provider);
        if auth_status.source.is_some() {
            return auth_status;
        }

        let provider_api_key = self
            .provider_request_configs
            .get(provider)
            .and_then(|c| c.api_key.as_deref());

        let Some(api_key) = provider_api_key else {
            return auth_status;
        };

        if is_command_config_value(api_key) {
            return AuthStatus {
                configured: true,
                source: Some("models_json_command".to_string()),
                label: None,
            };
        }

        let env_var_names = get_config_value_env_var_names(api_key);
        if !env_var_names.is_empty() {
            return if is_config_value_configured(
                api_key,
                self.auth_storage.get_provider_env(provider).as_ref(),
            ) {
                AuthStatus {
                    configured: true,
                    source: Some("environment".to_string()),
                    label: Some(env_var_names.join(", ")),
                }
            } else {
                AuthStatus {
                    configured: false,
                    ..Default::default()
                }
            };
        }

        AuthStatus {
            configured: true,
            source: Some("models_json_key".to_string()),
            label: None,
        }
    }

    // -----------------------------------------------------------------------
    // Display names
    // -----------------------------------------------------------------------

    /// Get display name for a provider.
    pub fn get_provider_display_name(&self, provider: &str) -> String {
        // Check registered providers first.
        if let Some(rp) = self.registered_providers.get(provider) {
            if let Some(ref name) = rp.name {
                return name.clone();
            }
            // TODO: check rp.oauth.name once oauth is wired.
        }

        // Check OAuth providers.
        for oauth_provider in &self.auth_storage.get_oauth_providers() {
            let pid = oauth_provider.get("id").and_then(|v| v.as_str());
            let pname = oauth_provider.get("name").and_then(|v| v.as_str());
            if pid == Some(provider) {
                if let Some(n) = pname {
                    return n.to_string();
                }
            }
        }

        // Fall back to built-in display names.
        built_in_provider_display_names()
            .get(provider)
            .cloned()
            .unwrap_or_else(|| provider.to_string())
    }

    // -----------------------------------------------------------------------
    // API key for provider
    // -----------------------------------------------------------------------

    /// Get API key for a provider (no model context needed).
    pub async fn get_api_key_for_provider(&self, provider: &str) -> Option<String> {
        let api_key = self.auth_storage.get_api_key(provider, false).await;
        if api_key.is_some() {
            return api_key;
        }

        let provider_api_key = self
            .provider_request_configs
            .get(provider)
            .and_then(|c| c.api_key.as_deref());

        provider_api_key.and_then(|key| {
            resolve_config_value_uncached(
                key,
                self.auth_storage.get_provider_env(provider).as_ref(),
            )
        })
    }

    // -----------------------------------------------------------------------
    // OAuth check
    // -----------------------------------------------------------------------

    /// Check if a model is using OAuth credentials.
    pub fn is_using_oauth(&self, model: &Model) -> bool {
        matches!(
            self.auth_storage.get(&model.provider),
            Some(AuthCredential::OAuth { .. })
        )
    }

    // -----------------------------------------------------------------------
    // Dynamic provider registration
    // -----------------------------------------------------------------------

    /// Register a provider dynamically (from extensions).
    ///
    /// If provider has models: replaces all existing models for this provider.
    /// If provider has only baseUrl/headers: overrides existing models' URLs.
    /// If provider has oauth: registers OAuth provider for /login support.
    pub fn register_provider(&mut self, provider_name: &str, config: ProviderConfigInput) {
        self.validate_provider_config(provider_name, &config);
        self.apply_provider_config(provider_name, &config);
        self.upsert_registered_provider(provider_name, config);
    }

    /// Unregister a previously registered provider.
    ///
    /// Removes the provider from the registry and reloads models from disk so
    /// built-in models overridden by this provider are restored. Also resets
    /// dynamic OAuth and API stream registrations before reapplying remaining
    /// dynamic providers. Has no effect if the provider was never registered.
    pub fn unregister_provider(&mut self, provider_name: &str) {
        if !self.registered_providers.contains_key(provider_name) {
            return;
        }
        self.registered_providers.remove(provider_name);
        self.refresh();
    }

    /// Upsert a provider config into registered_providers.
    /// Overriding defined values win; undefined values are preserved from the stored config.
    fn upsert_registered_provider(&mut self, provider_name: &str, config: ProviderConfigInput) {
        self.registered_providers
            .entry(provider_name.to_string())
            .and_modify(|existing| {
                if config.name.is_some() {
                    existing.name = config.name.clone();
                }
                if config.base_url.is_some() {
                    existing.base_url = config.base_url.clone();
                }
                if config.api_key.is_some() {
                    existing.api_key = config.api_key.clone();
                }
                if config.api.is_some() {
                    existing.api = config.api;
                }
                if config.stream_simple.is_some() {
                    existing.stream_simple = config.stream_simple.clone();
                }
                if config.headers.is_some() {
                    existing.headers = config.headers.clone();
                }
                if config.auth_header.is_some() {
                    existing.auth_header = config.auth_header;
                }
                if config.oauth.is_some() {
                    existing.oauth = config.oauth.clone();
                }
                if config.models.is_some() {
                    existing.models = config.models.clone();
                }
            })
            .or_insert(config);
    }

    fn validate_provider_config(&self, provider_name: &str, config: &ProviderConfigInput) {
        if config.stream_simple.is_some() && config.api.is_none() {
            panic!("Provider {provider_name}: \"api\" is required when registering streamSimple.");
        }

        let Some(models) = &config.models else {
            return;
        };
        if models.is_empty() {
            return;
        }

        if config.base_url.is_none() {
            panic!("Provider {provider_name}: \"baseUrl\" is required when defining models.");
        }
        if config.api_key.is_none() && config.oauth.is_none() {
            panic!(
                "Provider {provider_name}: \"apiKey\" or \"oauth\" is required when defining models."
            );
        }

        for model_def in models {
            let api = model_def.api.or(config.api);
            if api.is_none() {
                panic!(
                    "Provider {provider_name}, model {}: no \"api\" specified.",
                    model_def.id,
                );
            }
        }
    }

    fn apply_provider_config(&mut self, provider_name: &str, config: &ProviderConfigInput) {
        // Register OAuth provider if provided.
        if config.oauth.is_some() {
            // TODO: call register_oauth_provider() once wired.
        }

        // Register stream function if provided.
        // NOTE: Full dynamic provider registration requires bridging the
        // StreamSimpleArgs closure signature with the ApiStreamFunction/ApiStreamSimpleFunction
        // signatures expected by the API registry. This is deferred until the
        // extension/Rhai scripting infrastructure is fully wired.
        if config.stream_simple.is_some() {
            // Dynamic provider registration placeholder.
            // The stream_simple closure will be called directly from the
            // agent session runtime when the model is used.
        }

        self.store_provider_request_config(provider_name, config);

        if let Some(ref models) = config.models {
            if !models.is_empty() {
                // Full replacement: remove existing models for this provider.
                self.models
                    .retain(|mwc| mwc.model.provider != provider_name);

                // Parse and add new models.
                for model_def in models {
                    let api = model_def.api.or(config.api);
                    let Some(api) = api else {
                        continue;
                    };

                    self.store_model_headers(provider_name, &model_def.id, &model_def.headers);

                    self.models.push(ModelWithCompat {
                        model: Model {
                            id: model_def.id.clone(),
                            name: model_def
                                .name
                                .clone()
                                .unwrap_or_else(|| model_def.id.clone()),
                            api,
                            provider: provider_name.to_string(),
                            base_url: model_def
                                .base_url
                                .clone()
                                .unwrap_or_else(|| config.base_url.clone().unwrap_or_default()),
                            reasoning: model_def.reasoning.unwrap_or(false),
                            thinking_level_map: model_def.thinking_level_map.clone(),
                            input: model_def.input.clone().unwrap_or_default(),
                            cost: model_def.cost.clone().unwrap_or(ModelCost {
                                input: 0.0,
                                output: 0.0,
                                cache_read: 0.0,
                                cache_write: 0.0,
                            }),
                            context_window: model_def.context_window.unwrap_or(128_000),
                            max_tokens: model_def.max_tokens.unwrap_or(16_384),
                            headers: None,
            compat: None,
                        },
                        compat: model_def.compat.clone(),
                    });
                }

                // Apply OAuth modifyModels if credentials exist.
                // TODO: once OAuth providers have modifyModels wired.
            }
        } else if config.base_url.is_some() || config.headers.is_some() {
            // Override-only: update baseUrl for existing models.
            for mwc in &mut self.models {
                if mwc.model.provider != provider_name {
                    continue;
                }
                if let Some(ref bu) = config.base_url {
                    mwc.model.base_url = bu.clone();
                }
            }
        }
    }

    // -----------------------------------------------------------------------
    // Internal helpers
    // -----------------------------------------------------------------------

    fn get_model_request_key(&self, provider: &str, model_id: &str) -> String {
        format!("{provider}:{model_id}")
    }

    fn store_provider_request_config(
        &mut self,
        provider_name: &str,
        config: &impl ProviderConfigLike,
    ) {
        if config.api_key().is_none()
            && config.headers().is_none()
            && config.auth_header().is_none()
        {
            return;
        }
        self.provider_request_configs.insert(
            provider_name.to_string(),
            ProviderRequestConfig {
                api_key: config.api_key().cloned(),
                headers: config.headers().cloned(),
                auth_header: config.auth_header(),
            },
        );
    }

    fn store_model_headers(
        &mut self,
        provider_name: &str,
        model_id: &str,
        headers: &Option<HashMap<String, String>>,
    ) {
        let key = self.get_model_request_key(provider_name, model_id);
        match headers {
            Some(h) if !h.is_empty() => {
                self.model_request_headers.insert(key, h.clone());
            }
            _ => {
                self.model_request_headers.remove(&key);
            }
        }
    }
}

/// Trait so both ProviderJsonConfig and ProviderConfigInput can be passed to
/// store_provider_request_config.
trait ProviderConfigLike {
    fn api_key(&self) -> Option<&String>;
    fn headers(&self) -> Option<&HashMap<String, String>>;
    fn auth_header(&self) -> Option<bool>;
}

impl ProviderConfigLike for ProviderJsonConfig {
    fn api_key(&self) -> Option<&String> {
        self.api_key.as_ref()
    }
    fn headers(&self) -> Option<&HashMap<String, String>> {
        self.headers.as_ref()
    }
    fn auth_header(&self) -> Option<bool> {
        self.auth_header
    }
}

impl ProviderConfigLike for ProviderConfigInput {
    fn api_key(&self) -> Option<&String> {
        self.api_key.as_ref()
    }
    fn headers(&self) -> Option<&HashMap<String, String>> {
        self.headers.as_ref()
    }
    fn auth_header(&self) -> Option<bool> {
        self.auth_header
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_api_anthropic_messages() {
        let api = parse_api("anthropic-messages");
        assert!(matches!(api, Ok(Api::AnthropicMessages)));
    }

    #[test]
    fn test_parse_api_openai_completions() {
        let api = parse_api("openai-completions");
        assert!(matches!(api, Ok(Api::OpenAiCompletions)));
    }

    #[test]
    fn test_parse_api_unknown() {
        let api = parse_api("unknown-api");
        assert!(api.is_err());
    }

    #[test]
    fn test_strip_json_comments_no_comments() {
        let input = r#"{"key": "value"}"#;
        assert_eq!(strip_json_comments(input), r#"{"key": "value"}"#);
    }

    #[test]
    fn test_strip_line_comment() {
        let input = "// this is a comment\n{\"key\": \"value\"}";
        assert_eq!(strip_json_comments(input), "\n{\"key\": \"value\"}");
    }

    #[test]
    fn test_strip_block_comment() {
        let input = "/* block comment */\n{\"key\": \"value\"}";
        assert_eq!(strip_json_comments(input), "\n{\"key\": \"value\"}");
    }

    #[test]
    fn test_strip_comments_preserves_url_and_comment_markers_in_strings() {
        let input =
            r#"{"url":"http://127.0.0.1/v1","literal":"/* keep */","escaped":"\"//\""} // remove"#;
        assert_eq!(
            strip_json_comments(input),
            r#"{"url":"http://127.0.0.1/v1","literal":"/* keep */","escaped":"\"//\""} "#
        );
    }

    #[test]
    fn test_json_type_name_null() {
        assert_eq!(json_type_name(&serde_json::Value::Null), "null");
    }

    #[test]
    fn test_json_type_name_string() {
        assert_eq!(
            json_type_name(&serde_json::Value::String("hi".into())),
            "string"
        );
    }

    #[test]
    fn test_json_type_name_number() {
        assert_eq!(json_type_name(&serde_json::json!(42)), "number");
    }

    #[test]
    fn test_json_type_name_bool() {
        assert_eq!(json_type_name(&serde_json::json!(true)), "boolean");
    }

    #[test]
    fn test_json_type_name_array() {
        assert_eq!(json_type_name(&serde_json::json!([1, 2, 3])), "array");
    }

    #[test]
    fn test_json_type_name_object() {
        assert_eq!(json_type_name(&serde_json::json!({"a": 1})), "object");
    }

    #[test]
    fn test_validate_models_json_missing_providers() {
        let root: serde_json::Map<String, serde_json::Value> = serde_json::Map::new();
        let result = validate_models_json_schema(&root, "test.json");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("missing required field"));
    }

    // --- deep_merge_json ---

    #[test]
    fn test_deep_merge_json_both_objects() {
        let base = serde_json::json!({"a": 1, "b": 2});
        let over = serde_json::json!({"b": 3, "c": 4});
        let merged = deep_merge_json(&base, &over);
        assert_eq!(merged, serde_json::json!({"a": 1, "b": 3, "c": 4}));
    }

    #[test]
    fn test_deep_merge_json_over_is_non_object() {
        let base = serde_json::json!({"a": 1});
        let over = serde_json::json!(42);
        assert_eq!(deep_merge_json(&base, &over), serde_json::json!(42));
    }

    #[test]
    fn test_deep_merge_json_both_scalars_over_wins() {
        assert_eq!(
            deep_merge_json(&serde_json::json!("hello"), &serde_json::json!("world")),
            serde_json::json!("world")
        );
    }

    // --- merge_compat ---

    #[test]
    fn test_merge_compat_both_none() {
        assert_eq!(merge_compat(None, None), None);
    }

    #[test]
    fn test_merge_compat_only_base() {
        let compat = serde_json::json!({"key": "val"});
        assert_eq!(merge_compat(Some(compat.clone()), None), Some(compat));
    }

    #[test]
    fn test_merge_compat_only_overrides() {
        let compat = serde_json::json!({"key": "val"});
        assert_eq!(merge_compat(None, Some(compat.clone())), Some(compat));
    }

    #[test]
    fn test_merge_compat_deep_merge_routing() {
        let base = serde_json::json!({
            "openRouterRouting": {"order": ["provider1"]},
            "common": "base"
        });
        let over = serde_json::json!({
            "openRouterRouting": {"only": ["provider2"]},
            "common": "over"
        });
        let merged = merge_compat(Some(base), Some(over)).unwrap();
        // openRouterRouting is deep-merged; both order and only are arrays
        assert_eq!(
            merged["openRouterRouting"]["order"],
            serde_json::json!(["provider1"])
        );
        assert_eq!(
            merged["openRouterRouting"]["only"],
            serde_json::json!(["provider2"])
        );
        // common should be overridden
        assert_eq!(merged["common"], "over");
    }

    #[test]
    fn test_merge_compat_over_wins_for_non_object_base() {
        let base = serde_json::json!("string_base");
        let over = serde_json::json!({"key": "val"});
        assert_eq!(
            merge_compat(Some(base), Some(over)),
            Some(serde_json::json!({"key": "val"}))
        );
    }

    // --- parse_thinking_level_map ---

    #[test]
    fn test_parse_thinking_level_map_valid() {
        let val = serde_json::json!({
            "low": "budget_low",
            "high": "budget_high"
        });
        let result = parse_thinking_level_map(&val);
        assert!(result.is_some());
        let map = result.unwrap();
        assert_eq!(
            map.get(&hamr_ai::types::ModelThinkingLevel::Low),
            Some(&Some("budget_low".to_string()))
        );
        assert_eq!(
            map.get(&hamr_ai::types::ModelThinkingLevel::High),
            Some(&Some("budget_high".to_string()))
        );
    }

    #[test]
    fn test_parse_thinking_level_map_null_value() {
        let val = serde_json::json!({"off": null});
        let result = parse_thinking_level_map(&val);
        assert!(result.is_some());
        assert_eq!(
            result
                .unwrap()
                .get(&hamr_ai::types::ModelThinkingLevel::Off),
            Some(&None)
        );
    }

    #[test]
    fn test_parse_thinking_level_map_empty() {
        let val = serde_json::json!({});
        assert!(parse_thinking_level_map(&val).is_none());
    }

    #[test]
    fn test_parse_thinking_level_map_invalid_key_ignored() {
        let val = serde_json::json!({"invalid_key": "value"});
        assert!(parse_thinking_level_map(&val).is_none());
    }

    // --- parse_input_modalities ---

    #[test]
    fn test_parse_input_modalities_text_and_image() {
        let val = serde_json::json!(["text", "image"]);
        let result = parse_input_modalities(&val);
        assert!(result.is_some());
        let mods = result.unwrap();
        assert!(mods.contains(&hamr_ai::types::InputModality::Text));
        assert!(mods.contains(&hamr_ai::types::InputModality::Image));
    }

    #[test]
    fn test_parse_input_modalities_invalid_item_skipped() {
        let val = serde_json::json!(["text", "audio"]);
        let result = parse_input_modalities(&val);
        assert!(result.is_some());
        assert_eq!(result.unwrap(), vec![hamr_ai::types::InputModality::Text]);
    }

    #[test]
    fn test_parse_input_modalities_empty() {
        let val = serde_json::json!([]);
        assert!(parse_input_modalities(&val).is_none());
    }

    // --- parse_model_cost ---

    #[test]
    fn test_parse_model_cost_full() {
        let val = serde_json::json!({
            "input": 1.0,
            "output": 2.0,
            "cacheRead": 0.5,
            "cacheWrite": 0.25
        });
        let cost = parse_model_cost(&val);
        assert!(cost.is_some());
        let c = cost.unwrap();
        assert!((c.input - 1.0).abs() < f64::EPSILON);
        assert!((c.output - 2.0).abs() < f64::EPSILON);
        assert!((c.cache_read - 0.5).abs() < f64::EPSILON);
        assert!((c.cache_write - 0.25).abs() < f64::EPSILON);
    }

    #[test]
    fn test_parse_model_cost_missing_fields() {
        let val = serde_json::json!({"input": 1.0});
        assert!(parse_model_cost(&val).is_none());
    }

    // --- parse_partial_cost ---

    #[test]
    fn test_parse_partial_cost_all_fields() {
        let val = serde_json::json!({"input": 3.0, "output": 6.0});
        let cost = parse_partial_cost(&val);
        assert!((cost.input.unwrap() - 3.0).abs() < f64::EPSILON);
        assert!((cost.output.unwrap() - 6.0).abs() < f64::EPSILON);
        assert!(cost.cache_read.is_none());
        assert!(cost.cache_write.is_none());
    }

    #[test]
    fn test_parse_partial_cost_empty() {
        let cost = parse_partial_cost(&serde_json::json!({}));
        assert!(cost.input.is_none());
        assert!(cost.output.is_none());
    }

    // --- parse_string_map ---

    #[test]
    fn test_parse_string_map_valid() {
        let val = serde_json::json!({"k1": "v1", "k2": "v2"});
        let map = parse_string_map(&val).unwrap();
        assert_eq!(map.get("k1"), Some(&"v1".to_string()));
        assert_eq!(map.get("k2"), Some(&"v2".to_string()));
        assert_eq!(map.len(), 2);
    }

    #[test]
    fn test_parse_string_map_skips_non_string_values() {
        let val = serde_json::json!({"k1": "v1", "k2": 42});
        let map = parse_string_map(&val).unwrap();
        assert_eq!(map.get("k1"), Some(&"v1".to_string()));
        assert!(map.get("k2").is_none());
    }

    #[test]
    fn test_parse_string_map_empty() {
        let val = serde_json::json!({});
        assert!(parse_string_map(&val).is_none());
    }

    // --- apply_model_override ---

    #[test]
    fn test_apply_model_override_name() {
        let model = Model {
            id: "test-model".into(),
            name: "Original".into(),
            api: Api::OpenAiCompletions,
            provider: "test".into(),
            base_url: "https://example.com".into(),
            reasoning: false,
            thinking_level_map: None,
            input: vec![hamr_ai::types::InputModality::Text],
            cost: ModelCost {
                input: 1.0,
                output: 2.0,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 128000,
            max_tokens: 4096,
            headers: None,
            compat: None,
        };
        let override_ = ModelOverride {
            name: Some("Overridden".into()),
            ..Default::default()
        };
        let result = apply_model_override(&model, &override_);
        assert_eq!(result.name, "Overridden");
        assert_eq!(result.id, "test-model");
    }

    #[test]
    fn test_apply_model_override_partial_cost() {
        let model = Model {
            id: "test".into(),
            name: "Test".into(),
            api: Api::OpenAiCompletions,
            provider: "test".into(),
            base_url: "https://example.com".into(),
            reasoning: false,
            thinking_level_map: None,
            input: vec![hamr_ai::types::InputModality::Text],
            cost: ModelCost {
                input: 1.0,
                output: 2.0,
                cache_read: 0.5,
                cache_write: 0.25,
            },
            context_window: 128000,
            max_tokens: 4096,
            headers: None,
            compat: None,
        };
        let override_ = ModelOverride {
            cost: Some(PartialModelCost {
                input: Some(99.0),
                output: None,
                cache_read: None,
                cache_write: None,
            }),
            ..Default::default()
        };
        let result = apply_model_override(&model, &override_);
        assert!((result.cost.input - 99.0).abs() < f64::EPSILON);
        assert!((result.cost.output - 2.0).abs() < f64::EPSILON);
        assert!((result.cost.cache_read - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn test_apply_model_override_reasoning() {
        let model = Model {
            id: "test".into(),
            name: "Test".into(),
            api: Api::OpenAiCompletions,
            provider: "test".into(),
            base_url: "https://example.com".into(),
            reasoning: false,
            thinking_level_map: None,
            input: vec![hamr_ai::types::InputModality::Text],
            cost: ModelCost {
                input: 0.0,
                output: 0.0,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 128000,
            max_tokens: 4096,
            headers: None,
            compat: None,
        };
        let result = apply_model_override(
            &model,
            &ModelOverride {
                reasoning: Some(true),
                ..Default::default()
            },
        );
        assert!(result.reasoning);
    }

    // --- validate_models_json_schema (more cases) ---

    #[test]
    fn test_validate_models_json_providers_not_object() {
        let mut root = serde_json::Map::new();
        root.insert("providers".into(), serde_json::json!("string"));
        let err = validate_models_json_schema(&root, "test.json").unwrap_err();
        assert!(err.contains("must be an object"));
    }

    #[test]
    fn test_validate_models_json_model_missing_id() {
        let root = serde_json::json!({
            "providers": {
                "test": {
                    "models": [{"name": "no-id"}]
                }
            }
        });
        let root_map = root.as_object().unwrap();
        let err = validate_models_json_schema(root_map, "test.json").unwrap_err();
        assert!(err.contains("missing required field \"id\""));
    }

    #[test]
    fn test_validate_models_json_model_unknown_field() {
        let root = serde_json::json!({
            "providers": {
                "test": {
                    "models": [{"id": "m1", "unknownField": true}]
                }
            }
        });
        let root_map = root.as_object().unwrap();
        let err = validate_models_json_schema(root_map, "test.json").unwrap_err();
        assert!(err.contains("unknown field"));
    }

    #[test]
    fn test_validate_models_json_provider_unknown_field() {
        let root = serde_json::json!({
            "providers": {
                "test": {
                    "unknownProviderField": "value"
                }
            }
        });
        let root_map = root.as_object().unwrap();
        let err = validate_models_json_schema(root_map, "test.json").unwrap_err();
        assert!(err.contains("unknown field"));
    }

    // --- merge_custom_models ---

    #[test]
    fn test_merge_custom_models_empty() {
        let registry = ModelRegistry::in_memory(Arc::new(NoopAuthStorage));
        let built_in: Vec<ModelWithCompat> = Vec::new();
        let custom: Vec<ModelWithCompat> = Vec::new();
        let merged = registry.merge_custom_models(built_in, custom);
        assert!(merged.is_empty());
    }
}
