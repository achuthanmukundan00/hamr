//! Port of `packages/ai/src/providers/cloudflare.ts`.
//!
//! Cloudflare base URL templates and `{VAR}` placeholder resolution.

use std::sync::LazyLock;

use regex::Regex;

use crate::types::{Model, ProviderEnv};
use crate::utils::provider_env::get_provider_env_value;

/// Workers AI direct endpoint.
pub const CLOUDFLARE_WORKERS_AI_BASE_URL: &str =
    "https://api.cloudflare.com/client/v4/accounts/{CLOUDFLARE_ACCOUNT_ID}/ai/v1";

/// AI Gateway Unified API.
pub const CLOUDFLARE_AI_GATEWAY_COMPAT_BASE_URL: &str =
    "https://gateway.ai.cloudflare.com/v1/{CLOUDFLARE_ACCOUNT_ID}/{CLOUDFLARE_GATEWAY_ID}/compat";

/// AI Gateway → OpenAI passthrough.
pub const CLOUDFLARE_AI_GATEWAY_OPENAI_BASE_URL: &str =
    "https://gateway.ai.cloudflare.com/v1/{CLOUDFLARE_ACCOUNT_ID}/{CLOUDFLARE_GATEWAY_ID}/openai";

/// AI Gateway → Anthropic passthrough.
pub const CLOUDFLARE_AI_GATEWAY_ANTHROPIC_BASE_URL: &str = "https://gateway.ai.cloudflare.com/v1/{CLOUDFLARE_ACCOUNT_ID}/{CLOUDFLARE_GATEWAY_ID}/anthropic";

static PLACEHOLDER_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\{([A-Z_][A-Z0-9_]*)\}").expect("valid placeholder regex"));

/// Whether `provider` is one of the Cloudflare providers.
pub fn is_cloudflare_provider(provider: &str) -> bool {
    provider == "cloudflare-workers-ai" || provider == "cloudflare-ai-gateway"
}

/// Substitute `{VAR}` placeholders in a Cloudflare `base_url` from provider env
/// or the process environment.
///
/// Returns an error string (mirroring the thrown `Error`) when a referenced
/// variable is unset or empty.
pub fn resolve_cloudflare_base_url(
    model: &Model,
    env: Option<&ProviderEnv>,
) -> Result<String, String> {
    let url = &model.base_url;
    if !url.contains('{') {
        return Ok(url.clone());
    }

    let mut result = String::with_capacity(url.len());
    let mut last_end = 0;
    for caps in PLACEHOLDER_RE.captures_iter(url) {
        let whole = caps.get(0).unwrap();
        let name = &caps[1];
        match get_provider_env_value(name, env) {
            Some(value) if !value.is_empty() => {
                result.push_str(&url[last_end..whole.start()]);
                result.push_str(&value);
                last_end = whole.end();
            }
            _ => {
                return Err(format!(
                    "{name} is required for provider {} but is not set.",
                    model.provider
                ));
            }
        }
    }
    result.push_str(&url[last_end..]);
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Api, ModelCost};

    fn model_with_url(url: &str) -> Model {
        Model {
            id: "m".into(),
            name: "M".into(),
            api: Api::OpenAiCompletions,
            provider: "cloudflare-workers-ai".into(),
            base_url: url.into(),
            reasoning: false,
            thinking_level_map: None,
            input: vec![],
            cost: ModelCost {
                input: 0.0,
                output: 0.0,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 0,
            max_tokens: 0,
            headers: None,
            compat: None,
        }
    }

    #[test]
    fn provider_detection() {
        assert!(is_cloudflare_provider("cloudflare-workers-ai"));
        assert!(is_cloudflare_provider("cloudflare-ai-gateway"));
        assert!(!is_cloudflare_provider("openai"));
    }

    #[test]
    fn no_placeholder_returns_unchanged() {
        let m = model_with_url("https://example.com/v1");
        assert_eq!(
            resolve_cloudflare_base_url(&m, None).unwrap(),
            "https://example.com/v1"
        );
    }

    #[test]
    fn resolves_from_provider_env() {
        let m = model_with_url(CLOUDFLARE_WORKERS_AI_BASE_URL);
        let mut env = ProviderEnv::new();
        env.insert("CLOUDFLARE_ACCOUNT_ID".into(), "acct123".into());
        let resolved = resolve_cloudflare_base_url(&m, Some(&env)).unwrap();
        assert_eq!(
            resolved,
            "https://api.cloudflare.com/client/v4/accounts/acct123/ai/v1"
        );
    }

    #[test]
    fn missing_var_errors() {
        let m = model_with_url("https://x/{CLOUDFLARE_MISSING_VAR_XYZ}/v1");
        let env = ProviderEnv::new();
        let err = resolve_cloudflare_base_url(&m, Some(&env)).unwrap_err();
        assert!(
            err.contains("CLOUDFLARE_MISSING_VAR_XYZ is required"),
            "{err}"
        );
    }
}
