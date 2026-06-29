//! Port of `packages/coding-agent/src/hamr/startup-config.ts`.
//!
//! Hamr startup configuration — loads provider and active model settings
//! from `~/.config/hamr/config.toml` (global) and `.hamr.toml` (local/project)
//! with active config merging.
//!
//! Also provides relay model auto-discovery and provider registration building.
//! This is the bridge between human-authored config files and the internal
//! provider registration format used by the extension system.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

// ─── HamrModelConfig ──────────────────────────────────────────────────────────

/// Per-model configuration from a hamr config file.
/// Accepts both camelCase and snake_case field names for user convenience.
#[derive(Debug, Clone, serde::Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct HamrModelConfig {
    pub id: String,
    #[serde(alias = "displayName")]
    pub display_name: Option<String>,
    #[serde(alias = "contextWindow")]
    pub context_window: Option<u64>,
    #[serde(alias = "maxOutputTokens")]
    pub max_output_tokens: Option<u64>,
    #[serde(alias = "supportsThinking")]
    pub supports_thinking: Option<bool>,
    #[serde(alias = "supportsVision")]
    pub supports_vision: Option<bool>,
    #[serde(alias = "thinkingLevels")]
    pub thinking_levels: Option<Vec<String>>,
    #[serde(alias = "defaultThinking")]
    pub default_thinking: Option<String>,
    #[serde(alias = "toolCallParser")]
    pub tool_call_parser: Option<String>,
    pub cost: Option<ModelCost>,
}

/// Per-model cost configuration.
#[derive(Debug, Clone, Copy, serde::Deserialize, serde::Serialize)]
pub struct ModelCost {
    pub input: f64,
    pub output: f64,
    #[serde(rename = "cacheRead")]
    pub cache_read: f64,
    #[serde(rename = "cacheWrite")]
    pub cache_write: f64,
}

// ─── HamrProviderConfig ──────────────────────────────────────────────────────

/// Per-provider configuration from a hamr config file.
#[derive(Debug, Clone, serde::Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct HamrProviderConfig {
    pub enabled: Option<bool>,
    pub name: Option<String>,
    #[serde(alias = "compatibility")]
    pub compatibility: Option<String>, // "openai-compatible" or "anthropic-compatible"
    #[serde(alias = "baseUrl")]
    pub base_url: Option<String>,
    #[serde(alias = "apiKey")]
    pub api_key: Option<String>,
    #[serde(alias = "apiKeyEnv")]
    pub api_key_env: Option<String>,
    #[serde(alias = "headers", alias = "customHeaders")]
    pub custom_headers: Option<HashMap<String, String>>,
    pub models: Option<Vec<HamrModelConfig>>,
    #[serde(alias = "toolCallParser")]
    pub tool_call_parser: Option<String>,
    /// Whether this provider is a cloud provider (defaults to false for configured
    /// providers, true for built-in providers not in config).
    pub cloud: Option<bool>,
}

// ─── HamrStartupConfig ───────────────────────────────────────────────────────

/// Full startup configuration loaded from config files.
#[derive(Debug, Clone, serde::Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct HamrStartupConfig {
    #[serde(default)]
    pub active: Option<ActiveConfig>,
    #[serde(default)]
    pub providers: HashMap<String, HamrProviderConfig>,
    #[serde(skip)]
    pub source_paths: Vec<String>,
}

/// Active model/thinking configuration.
#[derive(Debug, Clone, serde::Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ActiveConfig {
    pub provider: Option<String>,
    pub model: Option<String>,
    pub thinking: Option<String>,
}

// ─── HamrProviderRegistration ────────────────────────────────────────────────

/// A built provider registration ready for the extension system.
#[derive(Debug, Clone)]
pub struct HamrProviderRegistration {
    pub name: String,
    /// Placeholder: when ProviderConfig is ported, this will be the real type.
    pub config: serde_json::Value,
    pub parser_by_model: HashMap<String, String>,
}

// ─── Constants ───────────────────────────────────────────────────────────────

/// Default relay endpoint (llama.cpp / LM Studio / Ollama local default).
pub const DEFAULT_RELAY_BASE_URL: &str = "http://127.0.0.1:1234/v1";

/// API key placeholder for local endpoints that don't require authentication.
pub const LOCAL_API_KEY: &str = "not-needed";

/// Env override for the relay endpoint base URL.
pub const RELAY_BASE_URL_ENV: &str = "HAMR_RELAY_BASE_URL";

/// When set to "1", skip network model discovery (test gate).
pub const SKIP_NETWORK_ENV: &str = "HAMR_TEST_SKIP_NETWORK";

// ─── Env expansion ───────────────────────────────────────────────────────────

/// Expand `${VAR}` and `$VAR` references in a config string from `std::env::var`.
/// Unset vars expand to "".
///
/// Mirror of `expandEnv` in the TS source.
pub fn expand_env(value: Option<&str>) -> Option<String> {
    let value = value?;
    let expanded = expand_env_string(value);
    Some(expanded)
}

/// Expand env vars in a string.
fn expand_env_string(value: &str) -> String {
    let re_braced = regex::Regex::new(r"\$\{(\w+)\}").unwrap();
    let re_bare = regex::Regex::new(r"\$(\w+)").unwrap();

    // First replace ${VAR}
    let intermediate = re_braced.replace_all(value, |caps: &regex::Captures| {
        let name = &caps[1];
        std::env::var(name).unwrap_or_default()
    });

    // Then replace $VAR (but don't re-match on already-expanded content)
    let result = re_bare.replace_all(&intermediate, |caps: &regex::Captures| {
        let name = &caps[1];
        std::env::var(name).unwrap_or_default()
    });

    result.to_string()
}

// ─── Config file discovery ───────────────────────────────────────────────────

/// Get the global hamr config path (`~/.config/hamr/config.toml`).
/// Returns `None` if `HOME` is not set.
///
/// Mirror of `globalHamrConfigPath` in the TS source.
pub fn global_hamr_config_path() -> Option<PathBuf> {
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .ok()?;
    Some(
        PathBuf::from(home)
            .join(".config")
            .join("hamr")
            .join("config.toml"),
    )
}

/// Walk up from `base_dir` looking for `.hamr.toml`.
/// Returns `None` if no local config file is found.
///
/// Mirror of `discoverLocalConfigPath` in the TS source.
pub fn discover_local_config_path(base_dir: &Path) -> Option<PathBuf> {
    let candidate = base_dir.join(".hamr.toml");
    if candidate.exists() {
        return Some(candidate);
    }
    let parent = base_dir.parent()?;
    if parent == base_dir {
        return None;
    }
    discover_local_config_path(parent)
}

// ─── Config loading ──────────────────────────────────────────────────────────

/// Parse a TOML config file into partial startup config.
///
/// Mirror of `parseConfigFile` in the TS source.
pub fn parse_config_file(path: &Path) -> Option<HamrStartupConfig> {
    if !path.exists() {
        return None;
    }
    let contents = std::fs::read_to_string(path).ok()?;
    // Parse TOML into a serde_json::Value first, then convert to our types
    // (this handles the toml->rust deserialization)
    let config: HamrStartupConfig = toml::from_str(&contents)
        .or_else(|_| {
            // Try serde_json as fallback (some configs may use JSON)
            serde_json::from_str(&contents)
        })
        .ok()?;
    Some(config)
}

/// Merge a partial config over a base config.
/// Active config is shallow-merged, providers are overlaid.
///
/// Mirror of `mergeConfig` in the TS source.
pub fn merge_config(
    base: &HamrStartupConfig,
    next: &HamrStartupConfig,
    source_path: String,
) -> HamrStartupConfig {
    let active = if let (Some(base_active), Some(next_active)) = (&base.active, &next.active) {
        let mut merged = base_active.clone();
        if next_active.provider.is_some() {
            merged.provider = next_active.provider.clone();
        }
        if next_active.model.is_some() {
            merged.model = next_active.model.clone();
        }
        if next_active.thinking.is_some() {
            merged.thinking = next_active.thinking.clone();
        }
        Some(merged)
    } else {
        next.active.clone().or_else(|| base.active.clone())
    };

    let mut providers = base.providers.clone();
    for (k, v) in &next.providers {
        providers.insert(k.clone(), v.clone());
    }

    let mut source_paths = base.source_paths.clone();
    source_paths.push(source_path);

    HamrStartupConfig {
        active,
        providers,
        source_paths,
    }
}

/// Load the full startup config by merging global + local config files.
///
/// Mirror of `loadHamrStartupConfig` in the TS source.
pub fn load_hamr_startup_config(cwd: &Path) -> HamrStartupConfig {
    let mut config = HamrStartupConfig {
        active: Some(ActiveConfig {
            provider: Some("relay".to_string()),
            model: None,
            thinking: Some("off".to_string()),
        }),
        providers: {
            let mut providers = HashMap::new();
            providers.insert(
                "relay".to_string(),
                HamrProviderConfig {
                    enabled: Some(true),
                    name: Some("Relay".to_string()),
                    compatibility: Some("openai-compatible".to_string()),
                    ..Default::default()
                },
            );
            providers.insert(
                "google-vertex".to_string(),
                HamrProviderConfig {
                    enabled: Some(true),
                    name: Some("Google Vertex".to_string()),
                    compatibility: Some("google-vertex".to_string()),
                    cloud: Some(true),
                    ..Default::default()
                },
            );
            providers.insert(
                "openai-codex".to_string(),
                HamrProviderConfig {
                    enabled: Some(true),
                    name: Some("OpenAI Codex".to_string()),
                    compatibility: Some("openai-codex-responses".to_string()),
                    cloud: Some(true),
                    ..Default::default()
                },
            );
            providers
        },
        source_paths: Vec::new(),
    };

    // Load global config
    if let Some(global_path) = global_hamr_config_path() {
        if let Some(global_config) = parse_config_file(&global_path) {
            config = merge_config(&config, &global_config, global_path.display().to_string());
        }
    }

    // Load local config
    if let Some(local_path) = discover_local_config_path(cwd) {
        if let Some(local_config) = parse_config_file(&local_path) {
            config = merge_config(&config, &local_config, local_path.display().to_string());
        }
    }

    config
}

// ─── Helper accessors ────────────────────────────────────────────────────────

/// Get the effective base URL for a provider.
pub fn provider_base_url(provider: &HamrProviderConfig) -> Option<&str> {
    provider.base_url.as_deref()
}

/// Resolve the relay base URL in priority order: env override → config →
/// default. Uses `HAMR_RELAY_BASE_URL` env var.
pub fn resolve_relay_base_url(provider: &HamrProviderConfig) -> String {
    if let Ok(from_env) = std::env::var(RELAY_BASE_URL_ENV) {
        let trimmed = from_env.trim().to_string();
        if !trimmed.is_empty() {
            return trimmed;
        }
    }
    provider_base_url(provider)
        .map(|s| s.to_string())
        .unwrap_or_else(|| DEFAULT_RELAY_BASE_URL.to_string())
}

/// Resolve custom headers from a provider config, expanding env vars.
pub fn provider_headers(provider: &HamrProviderConfig) -> Option<HashMap<String, String>> {
    let headers = provider.custom_headers.as_ref()?;
    if headers.is_empty() {
        return None;
    }
    let expanded: HashMap<String, String> = headers
        .iter()
        .map(|(k, v)| (k.clone(), expand_env_string(v)))
        .collect();
    Some(expanded)
}

/// Resolve an API key from provider config.
///
/// Priority: literal (env-expanded) → api_key_env → None.
pub fn resolve_provider_api_key(provider: &HamrProviderConfig) -> Option<String> {
    // Try literal (with env expansion)
    if let Some(literal) = expand_env(provider.api_key.as_deref()) {
        let trimmed = literal.trim().to_string();
        if !trimmed.is_empty() {
            return Some(trimmed);
        }
    }
    // Try env name
    if let Some(env_name) = &provider.api_key_env {
        if let Ok(val) = std::env::var(env_name) {
            let trimmed = val.trim().to_string();
            if !trimmed.is_empty() {
                return Some(trimmed);
            }
        }
    }
    None
}

/// Check whether a provider should be treated as a cloud provider.
///
/// Configured providers default to non-cloud unless they set `cloud: true`.
/// Providers not in config (built-in) are treated as cloud by default.
///
/// Mirror of `isCloudProvider` in the TS source.
pub fn is_cloud_provider(config: &HamrStartupConfig, provider_id: &str) -> bool {
    match config.providers.get(provider_id) {
        Some(provider) => provider.cloud.unwrap_or(false),
        None => true, // Built-in (not in config) → cloud
    }
}

/// Normalize a thinking level string — strip "auto" to None.
pub fn normalize_thinking(level: Option<&str>) -> Option<String> {
    match level {
        None | Some("auto") => None,
        Some(other) => Some(other.to_string()),
    }
}

// ─── Accessor helpers for model config ───────────────────────────────────────

/// Get a model's context window, preferring snake_case alias.
pub fn model_context_window(model: &HamrModelConfig) -> Option<u64> {
    model.context_window
}

/// Get a model's max output tokens.
pub fn model_max_output_tokens(model: &HamrModelConfig) -> Option<u64> {
    model.max_output_tokens
}

/// Get a model's vision support.
pub fn model_supports_vision(model: &HamrModelConfig) -> Option<bool> {
    model.supports_vision
}

// ─── Relay model discovery ───────────────────────────────────────────────────

/// Discover models from a relay endpoint and convert to hamr model configs.
///
/// Returns an empty list when the endpoint is unreachable or discovery is
/// skipped via `HAMR_TEST_SKIP_NETWORK=1`.
///
/// Mirror of `discoverRelayModels` in the TS source (the local wrapper).
pub async fn discover_relay_models_for_config(
    base_url: &str,
    api_key: Option<&str>,
    headers: Option<&HashMap<String, String>>,
) -> Vec<HamrModelConfig> {
    if std::env::var(SKIP_NETWORK_ENV).unwrap_or_default() == "1" {
        return vec![];
    }

    let discovered =
        super::providers::relay_provider::discover_relay_models(base_url, api_key, headers).await;

    discovered
        .into_iter()
        .map(|model| HamrModelConfig {
            id: model.id,
            display_name: Some(model.display_name),
            context_window: model.context_window,
            max_output_tokens: model.max_output_tokens,
            supports_thinking: Some(model.supports_thinking),
            thinking_levels: Some(model.thinking_levels),
            supports_vision: model.supports_vision,
            ..Default::default()
        })
        .collect()
}

// ─── Model merging ───────────────────────────────────────────────────────────

/// Merge a discovered model into a configured model.
///
/// Configured values take priority over discovered values.
///
/// Mirror of `mergeDiscoveredModel` in the TS source.
pub fn merge_discovered_model(
    configured: &HamrModelConfig,
    discovered: &HamrModelConfig,
) -> HamrModelConfig {
    HamrModelConfig {
        id: configured.id.clone(),
        display_name: configured
            .display_name
            .clone()
            .or_else(|| discovered.display_name.clone()),
        context_window: model_context_window(configured)
            .or_else(|| model_context_window(discovered)),
        max_output_tokens: model_max_output_tokens(configured)
            .or_else(|| model_max_output_tokens(discovered)),
        supports_vision: model_supports_vision(configured)
            .or_else(|| model_supports_vision(discovered)),
        supports_thinking: configured
            .supports_thinking
            .or(discovered.supports_thinking),
        thinking_levels: configured
            .thinking_levels
            .clone()
            .or_else(|| discovered.thinking_levels.clone()),
        default_thinking: configured
            .default_thinking
            .clone()
            .or_else(|| discovered.default_thinking.clone()),
        tool_call_parser: configured
            .tool_call_parser
            .clone()
            .or_else(|| discovered.tool_call_parser.clone()),
        cost: configured.cost.or(discovered.cost),
    }
}

/// Merge two model lists: configured over discovered, adding new discoveries.
///
/// Mirror of `mergeProviderModels` in the TS source.
pub fn merge_provider_models(
    configured: &[HamrModelConfig],
    discovered: &[HamrModelConfig],
) -> Vec<HamrModelConfig> {
    if configured.is_empty() {
        return discovered.to_vec();
    }
    if discovered.is_empty() {
        return configured.to_vec();
    }

    let discovered_by_id: HashMap<String, &HamrModelConfig> = discovered
        .iter()
        .map(|m| (m.id.to_lowercase(), m))
        .collect();

    let configured_ids: std::collections::HashSet<String> =
        configured.iter().map(|m| m.id.to_lowercase()).collect();

    let mut result: Vec<HamrModelConfig> = configured
        .iter()
        .map(|model| {
            let key = model.id.to_lowercase();
            if let Some(discovered_model) = discovered_by_id.get(&key) {
                merge_discovered_model(model, discovered_model)
            } else {
                model.clone()
            }
        })
        .collect();

    // Add newly discovered models not in config
    for model in discovered {
        if !configured_ids.contains(&model.id.to_lowercase()) {
            result.push(model.clone());
        }
    }

    result
}

// ─── Build provider registrations ────────────────────────────────────────────

/// Auto-detect tool-call parser for a model id.
pub fn detect_parser_for_model(model_id: &str) -> Option<String> {
    crate::hamr::providers::parsers::types::detect_parser_id(model_id).map(|s| s.to_string())
}

/// Build the full list of provider registrations from startup config.
///
/// For each enabled provider in the config, this:
/// 1. Resolves the base URL
/// 2. Discovers relay models (for OpenAI-compatible endpoints)
/// 3. Merges configured models with discovered ones
/// 4. Builds a `ProviderConfig` registration with per-model parser mappings
///
/// Mirror of `buildHamrProviderRegistrations` in the TS source.
pub async fn build_hamr_provider_registrations(
    config: &HamrStartupConfig,
) -> Vec<HamrProviderRegistration> {
    let mut registrations: Vec<HamrProviderRegistration> = Vec::new();

    for (provider_id, provider) in &config.providers {
        if provider.enabled == Some(false) {
            continue;
        }

        let compatibility = provider
            .compatibility
            .as_deref()
            .unwrap_or("openai-compatible");

        // Resolve base URL
        let resolved_base_url: Option<String> = if provider_id == "relay" {
            Some(resolve_relay_base_url(provider))
        } else {
            provider_base_url(provider).map(|s| s.to_string())
        };

        let base_url = match &resolved_base_url {
            Some(url) => expand_env_string(url),
            None => continue,
        };

        if base_url.is_empty() {
            continue;
        }

        // Resolve API key and headers
        let api_key = resolve_provider_api_key(provider);
        let headers = provider_headers(provider);

        // Discover relay models for OpenAI-compatible endpoints
        let configured = provider.models.as_deref().unwrap_or(&[]);
        let models = if compatibility == "openai-compatible" {
            let discovered =
                discover_relay_models_for_config(&base_url, api_key.as_deref(), headers.as_ref())
                    .await;
            merge_provider_models(configured, &discovered)
        } else {
            configured.to_vec()
        };

        if models.is_empty() {
            continue;
        }

        // Per-model parser mappings
        let mut parser_by_model: HashMap<String, String> = HashMap::new();
        for model in &models {
            let explicit = model
                .tool_call_parser
                .as_deref()
                .or_else(|| provider.tool_call_parser.as_deref());
            let parser = explicit
                .map(|s| s.to_string())
                .or_else(|| detect_parser_for_model(&model.id))
                .unwrap_or_else(|| "generic".to_string());
            parser_by_model.insert(model.id.clone(), parser);
        }

        // Build registration
        let api = match compatibility {
            "anthropic-compatible" => "anthropic-messages",
            "google-vertex" => "google-vertex",
            "openai-codex-responses" => "openai-codex-responses",
            _ => "openai-completions",
        };

        let effective_api_key = api_key
            .or_else(|| provider.api_key.clone())
            .or_else(|| provider.api_key_env.as_ref().map(|e| format!("${}", e)))
            .unwrap_or_else(|| LOCAL_API_KEY.to_string());

        let registration = HamrProviderRegistration {
            name: provider_id.clone(),
            config: serde_json::json!({
                "name": provider.name.as_deref().unwrap_or(provider_id),
                "baseUrl": base_url,
                "api": api,
                "apiKey": effective_api_key,
                "authHeader": false,
                "headers": headers,
                "models": models.iter().map(|m| {
                    serde_json::json!({
                        "id": m.id,
                        "name": m.display_name.as_deref().unwrap_or(&m.id),
                        "reasoning": m.supports_thinking.unwrap_or(false),
                        "input": if m.supports_vision.unwrap_or(true) {
                            vec!["text", "image"]
                        } else {
                            vec!["text"]
                        },
                        "cost": m.cost.map(|c| serde_json::json!({
                            "input": c.input,
                            "output": c.output,
                            "cacheRead": c.cache_read,
                            "cacheWrite": c.cache_write,
                        })).unwrap_or_else(|| serde_json::json!({
                            "input": 0,
                            "output": 0,
                            "cacheRead": 0,
                            "cacheWrite": 0,
                        })),
                        "contextWindow": m.context_window.unwrap_or(0),
                        "maxTokens": m.max_output_tokens.unwrap_or(16384),
                    })
                }).collect::<Vec<_>>(),
            }),
            parser_by_model,
        };

        registrations.push(registration);
    }

    registrations
}

/// Get the default model from the startup config.
///
/// Mirror of `getHamrDefaultModel` in the TS source.
/// When `ModelRegistry` is ported, this will use the actual registry.
pub fn get_hamr_default_model(
    config: &HamrStartupConfig,
) -> Option<(String, String, Option<String>)> {
    let provider = config
        .active
        .as_ref()
        .and_then(|a| a.provider.as_deref())
        .unwrap_or("relay")
        .to_string();

    let model_id = config
        .active
        .as_ref()
        .and_then(|a| a.model.as_deref())
        .map(|s| s.to_string());

    let thinking = normalize_thinking(config.active.as_ref().and_then(|a| a.thinking.as_deref()));

    // Note: Model verification against the registry is done at the caller
    // level (e.g., in sdk.rs) where the ModelRegistry instance is available.
    model_id.map(|id| (provider, id, thinking))
}

#[cfg(test)]
mod tests {
    use super::*;

    // ─── Env expansion tests ─────────────────────────────────────

    #[test]
    fn test_expand_env_string_no_vars() {
        assert_eq!(expand_env_string("hello"), "hello");
    }

    #[test]
    fn test_expand_env_string_with_var() {
        unsafe {
            std::env::set_var("TEST_VAR_1", "world");
        }
        assert_eq!(expand_env_string("hello ${TEST_VAR_1}"), "hello world");
    }

    #[test]
    fn test_expand_env_string_unset_var() {
        assert_eq!(expand_env_string("hello ${UNSET_VAR_THING}"), "hello ");
    }

    // ─── Model accessor tests ────────────────────────────────────

    #[test]
    fn test_model_context_window() {
        let model = HamrModelConfig {
            context_window: Some(8192),
            ..Default::default()
        };
        assert_eq!(model_context_window(&model), Some(8192));
    }

    #[test]
    fn test_model_default() {
        let model = HamrModelConfig::default();
        assert_eq!(model_context_window(&model), None);
    }

    // ─── Merge config tests ──────────────────────────────────────

    #[test]
    fn test_merge_config_providers_overlay() {
        let mut base_providers = HashMap::new();
        base_providers.insert(
            "a".to_string(),
            HamrProviderConfig {
                name: Some("Provider A".to_string()),
                ..Default::default()
            },
        );
        let base = HamrStartupConfig {
            providers: base_providers,
            ..Default::default()
        };

        let mut next_providers = HashMap::new();
        next_providers.insert(
            "b".to_string(),
            HamrProviderConfig {
                name: Some("Provider B".to_string()),
                ..Default::default()
            },
        );
        let next = HamrStartupConfig {
            providers: next_providers,
            ..Default::default()
        };

        let merged = merge_config(&base, &next, "/test".to_string());
        assert!(merged.providers.contains_key("a"));
        assert!(merged.providers.contains_key("b"));
        assert_eq!(merged.source_paths, vec!["/test"]);
    }

    // ─── Cloud provider tests ────────────────────────────────────

    #[test]
    fn test_is_cloud_provider_configured_cloud_true() {
        let mut providers = HashMap::new();
        providers.insert(
            "my-provider".to_string(),
            HamrProviderConfig {
                cloud: Some(true),
                ..Default::default()
            },
        );
        let config = HamrStartupConfig {
            providers,
            ..Default::default()
        };
        assert!(is_cloud_provider(&config, "my-provider"));
    }

    #[test]
    fn test_is_cloud_provider_configured_cloud_default() {
        let mut providers = HashMap::new();
        providers.insert("my-provider".to_string(), HamrProviderConfig::default());
        let config = HamrStartupConfig {
            providers,
            ..Default::default()
        };
        assert!(!is_cloud_provider(&config, "my-provider"));
    }

    #[test]
    fn test_is_cloud_provider_not_in_config() {
        let config = HamrStartupConfig::default();
        // Built-in providers not in config are cloud by default
        assert!(is_cloud_provider(&config, "anthropic"));
    }

    // ─── Thinking normalization ──────────────────────────────────

    #[test]
    fn test_normalize_thinking_auto() {
        assert_eq!(normalize_thinking(Some("auto")), None);
    }

    #[test]
    fn test_normalize_thinking_none() {
        assert_eq!(normalize_thinking(None), None);
    }

    #[test]
    fn test_normalize_thinking_value() {
        assert_eq!(normalize_thinking(Some("high")), Some("high".to_string()));
    }

    // ─── Merge provider models ───────────────────────────────────

    #[test]
    fn test_merge_provider_models_configured_only() {
        let configured = vec![HamrModelConfig {
            id: "model-a".to_string(),
            ..Default::default()
        }];
        let result = merge_provider_models(&configured, &[]);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, "model-a");
    }

    #[test]
    fn test_merge_provider_models_discovered_only() {
        let discovered = vec![HamrModelConfig {
            id: "model-b".to_string(),
            ..Default::default()
        }];
        let result = merge_provider_models(&[], &discovered);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, "model-b");
    }

    #[test]
    fn test_merge_provider_models_overlay() {
        let configured = vec![HamrModelConfig {
            id: "model-a".to_string(),
            display_name: Some("Custom Name".to_string()),
            context_window: Some(10000),
            ..Default::default()
        }];
        let discovered = vec![HamrModelConfig {
            id: "model-a".to_string(),
            display_name: Some("Discovery Name".to_string()),
            context_window: Some(8192),
            supports_vision: Some(true),
            ..Default::default()
        }];
        let result = merge_provider_models(&configured, &discovered);
        assert_eq!(result.len(), 1);
        // Configured display_name wins
        assert_eq!(result[0].display_name, Some("Custom Name".to_string()));
        // Configured context_window wins
        assert_eq!(result[0].context_window, Some(10000));
        // Configured supports_vision is None, so discovered's Some(true) fills in
        assert_eq!(result[0].supports_vision, Some(true));
    }

    // ─── Resolve relay base URL ──────────────────────────────────

    #[test]
    fn test_resolve_relay_base_url_default() {
        let provider = HamrProviderConfig::default();
        let url = resolve_relay_base_url(&provider);
        assert_eq!(url, DEFAULT_RELAY_BASE_URL);
    }

    #[test]
    fn test_resolve_relay_base_url_from_config() {
        let provider = HamrProviderConfig {
            base_url: Some("http://custom:8080/v1".to_string()),
            ..Default::default()
        };
        let url = resolve_relay_base_url(&provider);
        assert_eq!(url, "http://custom:8080/v1");
    }

    // ─── Env expansion edge cases ────────────────────────────────

    #[test]
    fn test_expand_env_string_bare_var() {
        unsafe {
            std::env::set_var("TEST_BARE_1", "value");
        }
        assert_eq!(expand_env_string("$TEST_BARE_1"), "value");
    }

    #[test]
    fn test_expand_env_string_bare_unset() {
        // underscore is valid in a var name, so MISSING_BARE_VAR_suffix is one var name
        assert_eq!(
            expand_env_string("prefix_$MISSING_BARE_VAR_suffix"),
            "prefix_"
        );
    }

    #[test]
    fn test_expand_env_none() {
        assert_eq!(expand_env(None), None);
    }

    // ─── Provider helpers ─────────────────────────────────────────

    #[test]
    fn test_provider_base_url_none() {
        let provider = HamrProviderConfig::default();
        assert_eq!(provider_base_url(&provider), None);
    }

    #[test]
    fn test_provider_headers_empty() {
        let provider = HamrProviderConfig::default();
        assert_eq!(provider_headers(&provider), None);
    }

    #[test]
    fn test_provider_headers_with_values() {
        let mut headers = std::collections::HashMap::new();
        headers.insert("Authorization".to_string(), "Bearer token".to_string());
        let provider = HamrProviderConfig {
            custom_headers: Some(headers),
            ..Default::default()
        };
        let result = provider_headers(&provider);
        assert!(result.is_some());
        assert_eq!(
            result.unwrap().get("Authorization").map(|s| s.as_str()),
            Some("Bearer token")
        );
    }

    // ─── API key resolution ───────────────────────────────────────

    #[test]
    fn test_resolve_provider_api_key_literal() {
        let provider = HamrProviderConfig {
            api_key: Some("sk-literal".to_string()),
            ..Default::default()
        };
        assert_eq!(
            resolve_provider_api_key(&provider),
            Some("sk-literal".to_string())
        );
    }

    #[test]
    fn test_resolve_provider_api_key_env_var() {
        unsafe {
            std::env::set_var("HAMR_TEST_KEY_ENV", "sk-from-env");
        }
        let provider = HamrProviderConfig {
            api_key_env: Some("HAMR_TEST_KEY_ENV".to_string()),
            ..Default::default()
        };
        assert_eq!(
            resolve_provider_api_key(&provider),
            Some("sk-from-env".to_string())
        );
    }

    #[test]
    fn test_resolve_provider_api_key_neither() {
        let provider = HamrProviderConfig::default();
        assert_eq!(resolve_provider_api_key(&provider), None);
    }

    // ─── Default model accessor ────────────────────────────────────

    #[test]
    fn test_get_hamr_default_model_relay_default() {
        let config = HamrStartupConfig::default();
        // active is None, so provider defaults to "relay", model is None
        let result = get_hamr_default_model(&config);
        assert!(result.is_none());
    }

    #[test]
    fn test_get_hamr_default_model_with_active() {
        let config = HamrStartupConfig {
            active: Some(ActiveConfig {
                provider: Some("relay".to_string()),
                model: Some("qwen3-coder-7b".to_string()),
                thinking: Some("high".to_string()),
            }),
            ..Default::default()
        };
        let result = get_hamr_default_model(&config);
        assert_eq!(
            result,
            Some((
                "relay".to_string(),
                "qwen3-coder-7b".to_string(),
                Some("high".to_string())
            ))
        );
    }

    #[test]
    fn test_get_hamr_default_model_normalize_thinking() {
        let config = HamrStartupConfig {
            active: Some(ActiveConfig {
                provider: Some("relay".to_string()),
                model: Some("m1".to_string()),
                thinking: Some("auto".to_string()),
            }),
            ..Default::default()
        };
        let result = get_hamr_default_model(&config);
        // "auto" normalizes to None
        assert_eq!(result, Some(("relay".to_string(), "m1".to_string(), None)));
    }

    // ─── Merge discovered model ────────────────────────────────────

    #[test]
    fn test_merge_discovered_model_configured_wins() {
        let configured = HamrModelConfig {
            id: "m1".to_string(),
            display_name: Some("Configured Name".to_string()),
            context_window: Some(10000),
            max_output_tokens: Some(5000),
            ..Default::default()
        };
        let discovered = HamrModelConfig {
            id: "m1".to_string(),
            display_name: Some("Discovered Name".to_string()),
            context_window: Some(8000),
            max_output_tokens: Some(4000),
            supports_vision: Some(true),
            supports_thinking: Some(true),
            ..Default::default()
        };
        let merged = merge_discovered_model(&configured, &discovered);
        assert_eq!(merged.display_name, Some("Configured Name".to_string()));
        assert_eq!(merged.context_window, Some(10000));
        assert_eq!(merged.max_output_tokens, Some(5000));
        // Configured didn't set supports_vision, so discovered fills in
        assert_eq!(merged.supports_vision, Some(true));
        // Configured didn't set supports_thinking, so discovered fills in
        assert_eq!(merged.supports_thinking, Some(true));
    }

    #[test]
    fn test_merge_discovered_model_discovered_fills_blanks() {
        let configured = HamrModelConfig {
            id: "m2".to_string(),
            ..Default::default()
        };
        let discovered = HamrModelConfig {
            id: "m2".to_string(),
            display_name: Some("Discovery Only".to_string()),
            context_window: Some(16000),
            supports_thinking: Some(true),
            thinking_levels: Some(vec!["off".to_string(), "high".to_string()]),
            ..Default::default()
        };
        let merged = merge_discovered_model(&configured, &discovered);
        assert_eq!(merged.display_name, Some("Discovery Only".to_string()));
        assert_eq!(merged.context_window, Some(16000));
        assert_eq!(merged.supports_thinking, Some(true));
    }

    // ─── Config file discovery ─────────────────────────────────────

    #[test]
    fn test_global_hamr_config_path() {
        let path = global_hamr_config_path();
        assert!(path.is_some());
        let path_str = path.unwrap();
        assert!(
            path_str
                .to_string_lossy()
                .ends_with(".config/hamr/config.toml")
        );
    }

    // ─── Parse config file ────────────────────────────────────────

    #[test]
    fn test_parse_config_file_not_found() {
        let result = parse_config_file(std::path::Path::new("/nonexistent/hamr/config.toml"));
        assert!(result.is_none());
    }

    // ─── Merge config active overlay ──────────────────────────────

    #[test]
    fn test_merge_config_active_merged() {
        let base = HamrStartupConfig {
            active: Some(ActiveConfig {
                provider: Some("relay".to_string()),
                model: Some("base-model".to_string()),
                thinking: Some("medium".to_string()),
            }),
            ..Default::default()
        };
        let next = HamrStartupConfig {
            active: Some(ActiveConfig {
                provider: None,
                model: Some("next-model".to_string()),
                thinking: None,
            }),
            ..Default::default()
        };
        let merged = merge_config(&base, &next, "/test/path".to_string());
        assert_eq!(
            merged.active.as_ref().unwrap().provider.as_deref(),
            Some("relay")
        );
        // "next-model" overrides "base-model"
        assert_eq!(
            merged.active.as_ref().unwrap().model.as_deref(),
            Some("next-model")
        );
        // base "medium" survives because next thinking is None
        assert_eq!(
            merged.active.as_ref().unwrap().thinking.as_deref(),
            Some("medium")
        );
    }

    #[test]
    fn test_merge_config_no_base_active() {
        let base = HamrStartupConfig::default();
        let next = HamrStartupConfig {
            active: Some(ActiveConfig {
                provider: Some("relay".to_string()),
                model: None,
                thinking: None,
            }),
            ..Default::default()
        };
        let merged = merge_config(&base, &next, "/p2".to_string());
        assert_eq!(
            merged.active.as_ref().unwrap().provider.as_deref(),
            Some("relay")
        );
    }

    // ─── Detect parser for model ──────────────────────────────────

    #[test]
    fn test_detect_parser_for_model_known() {
        let result = detect_parser_for_model("qwen3-coder-7b");
        assert!(result.is_some());
    }

    #[test]
    fn test_detect_parser_for_model_unknown() {
        let result = detect_parser_for_model("totally-unknown-model-v99");
        assert!(result.is_none());
    }

    // ─── Model accessor defaults ──────────────────────────────────

    #[test]
    fn test_model_max_output_tokens_none() {
        let model = HamrModelConfig::default();
        assert_eq!(model_max_output_tokens(&model), None);
    }

    #[test]
    fn test_model_supports_vision_none() {
        let model = HamrModelConfig::default();
        assert_eq!(model_supports_vision(&model), None);
    }
}
