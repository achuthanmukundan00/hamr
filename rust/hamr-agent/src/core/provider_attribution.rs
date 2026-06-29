//! Provider attribution headers for telemetry.
//!
//! Ported from packages/coding-agent/src/core/provider-attribution.ts

use std::collections::HashMap;

use hamr_ai::types::Model;
use url::Url;

// ---------------------------------------------------------------------------
// Host constants

const OPENROUTER_HOST: &str = "openrouter.ai";
const NVIDIA_NIM_HOST: &str = "integrate.api.nvidia.com";
const CLOUDFLARE_API_HOST: &str = "api.cloudflare.com";
const CLOUDFLARE_AI_GATEWAY_HOST: &str = "gateway.ai.cloudflare.com";
const OPENCODE_HOST: &str = "opencode.ai";
const VERCEL_GATEWAY_HOST: &str = "ai-gateway.vercel.sh";

// ---------------------------------------------------------------------------
// Settings manager trait

/// Minimal interface for checking telemetry opt-in.
pub trait SettingsManagerAttribution {
    fn is_install_telemetry_enabled(&self) -> bool;
}

// ---------------------------------------------------------------------------
// Helpers

fn matches_host(base_url: &str, expected_host: &str) -> bool {
    Url::parse(base_url)
        .ok()
        .map(|u| u.host_str().unwrap_or("").to_string())
        .is_some_and(|host| host == expected_host)
}

fn is_open_router_model(model: &Model) -> bool {
    model.provider == "openrouter" || model.base_url.contains(OPENROUTER_HOST)
}

fn is_nvidia_nim_model(model: &Model) -> bool {
    model.provider == "nvidia" || matches_host(&model.base_url, NVIDIA_NIM_HOST)
}

fn is_cloudflare_model(model: &Model) -> bool {
    model.provider == "cloudflare-workers-ai"
        || model.provider == "cloudflare-ai-gateway"
        || matches_host(&model.base_url, CLOUDFLARE_API_HOST)
        || matches_host(&model.base_url, CLOUDFLARE_AI_GATEWAY_HOST)
}

fn is_vercel_gateway_model(model: &Model) -> bool {
    model.provider == "vercel-ai-gateway" || matches_host(&model.base_url, VERCEL_GATEWAY_HOST)
}

// ---------------------------------------------------------------------------
// Public API

/// Returns default attribution headers for the given model, if telemetry is
/// enabled and the model is recognised.
pub fn get_default_attribution_headers(
    model: &Model,
    settings_manager: &impl SettingsManagerAttribution,
) -> Option<HashMap<String, String>> {
    if !settings_manager.is_install_telemetry_enabled() {
        return None;
    }

    if is_open_router_model(model) {
        let mut headers = HashMap::new();
        headers.insert("HTTP-Referer".into(), "https://hamr.dev".into());
        headers.insert("X-OpenRouter-Title".into(), "hamr".into());
        headers.insert("X-OpenRouter-Categories".into(), "cli-agent".into());
        return Some(headers);
    }

    if is_nvidia_nim_model(model) {
        let mut headers = HashMap::new();
        headers.insert("X-BILLING-INVOKE-ORIGIN".into(), "Hamr".into());
        return Some(headers);
    }

    if is_cloudflare_model(model) {
        let mut headers = HashMap::new();
        headers.insert("User-Agent".into(), "hamr-coding-agent".into());
        return Some(headers);
    }

    if is_vercel_gateway_model(model) {
        let mut headers = HashMap::new();
        headers.insert("http-referer".into(), "https://hamr.dev".into());
        headers.insert("x-title".into(), "hamr".into());
        return Some(headers);
    }

    None
}

/// Returns OpenCode session headers if the model is an OpenCode one and a
/// session id is provided.
pub fn get_session_headers(
    model: &Model,
    session_id: Option<&str>,
) -> Option<HashMap<String, String>> {
    let session_id = session_id?;

    if model.provider != "opencode"
        && model.provider != "opencode-go"
        && !matches_host(&model.base_url, OPENCODE_HOST)
    {
        return None;
    }

    let mut headers = HashMap::new();
    headers.insert("x-opencode-session".into(), session_id.to_owned());
    headers.insert("x-opencode-client".into(), "hamr".into());
    Some(headers)
}

/// Merge session headers, default attribution headers, and any additional
/// header sources into a single map. Returns `None` if the result is empty.
pub fn merge_provider_attribution_headers(
    model: &Model,
    settings_manager: &impl SettingsManagerAttribution,
    session_id: Option<&str>,
    header_sources: &[Option<HashMap<String, String>>],
) -> Option<HashMap<String, String>> {
    let mut merged = HashMap::new();

    if let Some(session_hdrs) = get_session_headers(model, session_id) {
        merged.extend(session_hdrs);
    }

    if let Some(default_hdrs) = get_default_attribution_headers(model, settings_manager) {
        merged.extend(default_hdrs);
    }

    for source in header_sources {
        if let Some(headers) = source {
            merged.extend(headers.iter().map(|(k, v)| (k.clone(), v.clone())));
        }
    }

    if merged.is_empty() {
        None
    } else {
        Some(merged)
    }
}

// ---------------------------------------------------------------------------
// Tests

#[cfg(test)]
mod tests {
    use super::*;

    struct TelemetryEnabled;
    impl SettingsManagerAttribution for TelemetryEnabled {
        fn is_install_telemetry_enabled(&self) -> bool {
            true
        }
    }

    struct TelemetryDisabled;
    impl SettingsManagerAttribution for TelemetryDisabled {
        fn is_install_telemetry_enabled(&self) -> bool {
            false
        }
    }

    fn make_model(provider: &str, base_url: &str) -> Model {
        Model {
            id: "test-model".into(),
            name: "Test Model".into(),
            api: hamr_ai::types::Api::OpenAiCompletions,
            provider: provider.into(),
            base_url: base_url.into(),
            reasoning: false,
            thinking_level_map: None,
            input: vec![],
            cost: hamr_ai::types::ModelCost {
                input: 0.0,
                output: 0.0,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 4096,
            max_tokens: 1024,
            headers: None,
            compat: None,
        }
    }

    // -----------------------------------------------------------------------
    // matches_host

    #[test]
    fn test_matches_host_exact_match() {
        assert!(matches_host("https://openrouter.ai/chat", OPENROUTER_HOST));
    }

    #[test]
    fn test_matches_host_no_match() {
        assert!(!matches_host("https://example.com/", OPENROUTER_HOST));
    }

    #[test]
    fn test_matches_host_invalid_url() {
        assert!(!matches_host("not a url", OPENROUTER_HOST));
    }

    // -----------------------------------------------------------------------
    // is_open_router_model

    #[test]
    fn test_openrouter_by_provider() {
        let m = make_model("openrouter", "https://example.com/");
        assert!(is_open_router_model(&m));
    }

    #[test]
    fn test_openrouter_by_base_url() {
        let m = make_model("some-other", "https://openrouter.ai/v1");
        assert!(is_open_router_model(&m));
    }

    #[test]
    fn test_openrouter_not() {
        let m = make_model("anthropic", "https://api.anthropic.com");
        assert!(!is_open_router_model(&m));
    }

    // -----------------------------------------------------------------------
    // is_nvidia_nim_model

    #[test]
    fn test_nvidia_by_provider() {
        let m = make_model("nvidia", "https://example.com/");
        assert!(is_nvidia_nim_model(&m));
    }

    #[test]
    fn test_nvidia_by_base_url() {
        let m = make_model("other", "https://integrate.api.nvidia.com/v1");
        assert!(is_nvidia_nim_model(&m));
    }

    #[test]
    fn test_nvidia_not() {
        let m = make_model("openai", "https://api.openai.com");
        assert!(!is_nvidia_nim_model(&m));
    }

    // -----------------------------------------------------------------------
    // is_cloudflare_model

    #[test]
    fn test_cloudflare_workers() {
        let m = make_model("cloudflare-workers-ai", "https://example.com");
        assert!(is_cloudflare_model(&m));
    }

    #[test]
    fn test_cloudflare_gateway_provider() {
        let m = make_model("cloudflare-ai-gateway", "https://example.com");
        assert!(is_cloudflare_model(&m));
    }

    #[test]
    fn test_cloudflare_by_api_host() {
        let m = make_model("other", "https://api.cloudflare.com/client/v4");
        assert!(is_cloudflare_model(&m));
    }

    #[test]
    fn test_cloudflare_by_gateway_host() {
        let m = make_model("other", "https://gateway.ai.cloudflare.com/v1");
        assert!(is_cloudflare_model(&m));
    }

    #[test]
    fn test_cloudflare_not() {
        let m = make_model("openai", "https://api.openai.com");
        assert!(!is_cloudflare_model(&m));
    }

    // -----------------------------------------------------------------------
    // is_vercel_gateway_model

    #[test]
    fn test_vercel_by_provider() {
        let m = make_model("vercel-ai-gateway", "https://example.com");
        assert!(is_vercel_gateway_model(&m));
    }

    #[test]
    fn test_vercel_by_base_url() {
        let m = make_model("other", "https://ai-gateway.vercel.sh/v1");
        assert!(is_vercel_gateway_model(&m));
    }

    #[test]
    fn test_vercel_not() {
        let m = make_model("openai", "https://api.openai.com");
        assert!(!is_vercel_gateway_model(&m));
    }

    // -----------------------------------------------------------------------
    // get_default_attribution_headers

    #[test]
    fn test_default_headers_telemetry_disabled() {
        let m = make_model("openrouter", "https://openrouter.ai");
        assert!(get_default_attribution_headers(&m, &TelemetryDisabled).is_none());
    }

    #[test]
    fn test_default_headers_openrouter() {
        let m = make_model("openrouter", "https://example.com");
        let hdrs = get_default_attribution_headers(&m, &TelemetryEnabled).unwrap();
        assert_eq!(hdrs.get("HTTP-Referer").unwrap(), "https://hamr.dev");
        assert_eq!(hdrs.get("X-OpenRouter-Title").unwrap(), "hamr");
        assert_eq!(hdrs.get("X-OpenRouter-Categories").unwrap(), "cli-agent");
    }

    #[test]
    fn test_default_headers_nvidia() {
        let m = make_model("nvidia", "https://example.com");
        let hdrs = get_default_attribution_headers(&m, &TelemetryEnabled).unwrap();
        assert_eq!(hdrs.get("X-BILLING-INVOKE-ORIGIN").unwrap(), "Hamr");
    }

    #[test]
    fn test_default_headers_cloudflare() {
        let m = make_model("cloudflare-workers-ai", "https://example.com");
        let hdrs = get_default_attribution_headers(&m, &TelemetryEnabled).unwrap();
        assert_eq!(hdrs.get("User-Agent").unwrap(), "hamr-coding-agent");
    }

    #[test]
    fn test_default_headers_vercel() {
        let m = make_model("vercel-ai-gateway", "https://example.com");
        let hdrs = get_default_attribution_headers(&m, &TelemetryEnabled).unwrap();
        assert_eq!(hdrs.get("http-referer").unwrap(), "https://hamr.dev");
        assert_eq!(hdrs.get("x-title").unwrap(), "hamr");
    }

    #[test]
    fn test_default_headers_unknown_model() {
        let m = make_model("anthropic", "https://api.anthropic.com");
        assert!(get_default_attribution_headers(&m, &TelemetryEnabled).is_none());
    }

    // -----------------------------------------------------------------------
    // get_session_headers

    #[test]
    fn test_session_headers_no_session_id() {
        let m = make_model("opencode", "https://example.com");
        assert!(get_session_headers(&m, None).is_none());
    }

    #[test]
    fn test_session_headers_opencode_provider() {
        let m = make_model("opencode", "https://example.com");
        let hdrs = get_session_headers(&m, Some("sess-1")).unwrap();
        assert_eq!(hdrs.get("x-opencode-session").unwrap(), "sess-1");
        assert_eq!(hdrs.get("x-opencode-client").unwrap(), "hamr");
    }

    #[test]
    fn test_session_headers_opencode_go_provider() {
        let m = make_model("opencode-go", "https://example.com");
        assert!(get_session_headers(&m, Some("sess-1")).is_some());
    }

    #[test]
    fn test_session_headers_by_base_url() {
        let m = make_model("other", "https://opencode.ai/v1");
        let hdrs = get_session_headers(&m, Some("sess-1")).unwrap();
        assert_eq!(hdrs.get("x-opencode-session").unwrap(), "sess-1");
    }

    #[test]
    fn test_session_headers_not_opencode() {
        let m = make_model("openai", "https://api.openai.com");
        assert!(get_session_headers(&m, Some("sess-1")).is_none());
    }

    // -----------------------------------------------------------------------
    // merge_provider_attribution_headers

    #[test]
    fn test_merge_empty_without_any_matches() {
        let m = make_model("anthropic", "https://api.anthropic.com");
        assert!(merge_provider_attribution_headers(&m, &TelemetryEnabled, None, &[]).is_none());
    }

    #[test]
    fn test_merge_session_and_default() {
        let m = make_model("openrouter", "https://opencode.ai");
        let result = merge_provider_attribution_headers(&m, &TelemetryEnabled, Some("sess-1"), &[]);
        let result = result.unwrap();
        // session headers
        assert_eq!(result.get("x-opencode-session").unwrap(), "sess-1");
        assert_eq!(result.get("x-opencode-client").unwrap(), "hamr");
        // default attribution
        assert_eq!(result.get("HTTP-Referer").unwrap(), "https://hamr.dev");
    }

    #[test]
    fn test_merge_overrides_from_later_sources() {
        let m = make_model("openrouter", "https://openrouter.ai");
        let extra = Some(HashMap::from([(
            "HTTP-Referer".into(),
            "https://example.com".into(),
        )]));
        let result = merge_provider_attribution_headers(&m, &TelemetryEnabled, None, &[extra]);
        let result = result.unwrap();
        // extra source overrides the default
        assert_eq!(result.get("HTTP-Referer").unwrap(), "https://example.com");
    }
}
