//! Port of `../../packages/ai/src/env-api-keys.ts`.
//!
//! Resolves API keys from environment variables per provider.
//!
//! ## Divergence from TS
//!
//! The TS file lazily imports `node:fs`/`node:os`/`node:path` to detect Google
//! Vertex Application Default Credentials. In Rust these are available directly,
//! so [`has_vertex_adc_credentials`] uses `std::path` / `dirs`-style logic with
//! `std::fs` directly (no async import race, no permanent caching needed).

use crate::types::ProviderEnv;
use crate::utils::provider_env::get_provider_env_value;

/// Returns true when `ANTHROPIC_BASE_URL` points to DeepSeek's Anthropic-compatible endpoint.
pub fn is_anthropic_base_url_deepseek(env: Option<&ProviderEnv>) -> bool {
    match get_provider_env_value("ANTHROPIC_BASE_URL", env) {
        Some(base_url) => base_url.contains("deepseek.com"),
        None => false,
    }
}

/// Whether Google Vertex Application Default Credentials exist.
///
/// Checks `GOOGLE_APPLICATION_CREDENTIALS` first, then the default ADC path
/// `~/.config/gcloud/application_default_credentials.json`.
fn has_vertex_adc_credentials(env: Option<&ProviderEnv>) -> bool {
    if let Some(gac_path) = get_provider_env_value("GOOGLE_APPLICATION_CREDENTIALS", env) {
        return std::path::Path::new(&gac_path).exists();
    }
    // Fall back to the default ADC path under the home directory.
    match std::env::var("HOME").ok() {
        Some(home) => {
            let path = std::path::Path::new(&home)
                .join(".config")
                .join("gcloud")
                .join("application_default_credentials.json");
            path.exists()
        }
        None => false,
    }
}

/// Map a provider to the ordered list of env vars that can provide its API key.
///
/// Mirrors the TS `getApiKeyEnvVars`. Returns `None` for providers with no
/// known API-key env var (e.g. those that rely on OAuth/ambient credentials).
fn get_api_key_env_vars(provider: &str, env: Option<&ProviderEnv>) -> Option<Vec<&'static str>> {
    if provider == "github-copilot" {
        return Some(vec!["COPILOT_GITHUB_TOKEN"]);
    }

    // ANTHROPIC_OAUTH_TOKEN takes precedence over ANTHROPIC_API_KEY.
    // If ANTHROPIC_BASE_URL points to DeepSeek, ANTHROPIC_API_KEY is a DeepSeek key.
    if provider == "anthropic" {
        if is_anthropic_base_url_deepseek(env) {
            return Some(vec!["ANTHROPIC_OAUTH_TOKEN"]);
        }
        return Some(vec!["ANTHROPIC_OAUTH_TOKEN", "ANTHROPIC_API_KEY"]);
    }

    let env_var = match provider {
        "ant-ling" => "ANT_LING_API_KEY",
        "openai" => "OPENAI_API_KEY",
        "azure-openai-responses" => "AZURE_OPENAI_API_KEY",
        "nvidia" => "NVIDIA_API_KEY",
        "deepseek" => "DEEPSEEK_API_KEY",
        "google" => "GEMINI_API_KEY",
        "google-vertex" => "GOOGLE_CLOUD_API_KEY",
        "groq" => "GROQ_API_KEY",
        "cerebras" => "CEREBRAS_API_KEY",
        "xai" => "XAI_API_KEY",
        "openrouter" => "OPENROUTER_API_KEY",
        "vercel-ai-gateway" => "AI_GATEWAY_API_KEY",
        "zai" => "ZAI_API_KEY",
        "zai-coding-cn" => "ZAI_CODING_CN_API_KEY",
        "mistral" => "MISTRAL_API_KEY",
        "minimax" => "MINIMAX_API_KEY",
        "minimax-cn" => "MINIMAX_CN_API_KEY",
        "moonshotai" => "MOONSHOT_API_KEY",
        "moonshotai-cn" => "MOONSHOT_API_KEY",
        "huggingface" => "HF_TOKEN",
        "fireworks" => "FIREWORKS_API_KEY",
        "together" => "TOGETHER_API_KEY",
        "opencode" => "OPENCODE_API_KEY",
        "opencode-go" => "OPENCODE_API_KEY",
        "kimi-coding" => "KIMI_API_KEY",
        "cloudflare-workers-ai" => "CLOUDFLARE_API_KEY",
        "cloudflare-ai-gateway" => "CLOUDFLARE_API_KEY",
        "xiaomi" => "XIAOMI_API_KEY",
        "xiaomi-token-plan-cn" => "XIAOMI_TOKEN_PLAN_CN_API_KEY",
        "xiaomi-token-plan-ams" => "XIAOMI_TOKEN_PLAN_AMS_API_KEY",
        "xiaomi-token-plan-sgp" => "XIAOMI_TOKEN_PLAN_SGP_API_KEY",
        _ => return None,
    };
    Some(vec![env_var])
}

/// Find configured environment variables that can provide an API key for a provider.
///
/// Only reports actual API key variables; intentionally excludes ambient credential
/// sources (AWS profiles/IAM, Google ADC). Mirrors the TS `findEnvKeys`.
pub fn find_env_keys(provider: &str, env: Option<&ProviderEnv>) -> Option<Vec<String>> {
    let mut env_vars: Vec<String> = get_api_key_env_vars(provider, env)?
        .into_iter()
        .map(|s| s.to_string())
        .collect();

    // When ANTHROPIC_BASE_URL points to DeepSeek, ANTHROPIC_API_KEY is a DeepSeek
    // credential; make it available for the deepseek provider.
    if provider == "deepseek" && is_anthropic_base_url_deepseek(env) {
        if get_provider_env_value("ANTHROPIC_API_KEY", env).is_some() {
            env_vars.push("ANTHROPIC_API_KEY".to_string());
        }
    }

    let found: Vec<String> = env_vars
        .into_iter()
        .filter(|var| get_provider_env_value(var, env).is_some())
        .collect();

    if found.is_empty() { None } else { Some(found) }
}

/// Get the API key for a provider from known environment variables.
///
/// Will not return API keys for providers that require OAuth tokens. Mirrors the
/// TS `getEnvApiKey`.
pub fn get_env_api_key(provider: &str, env: Option<&ProviderEnv>) -> Option<String> {
    if let Some(env_keys) = find_env_keys(provider, env) {
        if let Some(first) = env_keys.first() {
            return get_provider_env_value(first, env);
        }
    }

    // Vertex AI supports either an explicit API key or Application Default Credentials.
    if provider == "google-vertex" {
        let has_credentials = has_vertex_adc_credentials(env);
        let has_project = get_provider_env_value("GOOGLE_CLOUD_PROJECT", env).is_some()
            || get_provider_env_value("GCLOUD_PROJECT", env).is_some();
        let has_location = get_provider_env_value("GOOGLE_CLOUD_LOCATION", env).is_some();
        if has_credentials && has_project && has_location {
            return Some("<authenticated>".to_string());
        }
    }

    if provider == "amazon-bedrock" {
        // Amazon Bedrock supports multiple credential sources.
        let authenticated = get_provider_env_value("AWS_PROFILE", env).is_some()
            || (get_provider_env_value("AWS_ACCESS_KEY_ID", env).is_some()
                && get_provider_env_value("AWS_SECRET_ACCESS_KEY", env).is_some())
            || get_provider_env_value("AWS_BEARER_TOKEN_BEDROCK", env).is_some()
            || get_provider_env_value("AWS_CONTAINER_CREDENTIALS_RELATIVE_URI", env).is_some()
            || get_provider_env_value("AWS_CONTAINER_CREDENTIALS_FULL_URI", env).is_some()
            || get_provider_env_value("AWS_WEB_IDENTITY_TOKEN_FILE", env).is_some();
        if authenticated {
            return Some("<authenticated>".to_string());
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_openai_from_provider_env() {
        let mut env = ProviderEnv::new();
        env.insert("OPENAI_API_KEY".to_string(), "sk-test".to_string());
        assert_eq!(
            get_env_api_key("openai", Some(&env)),
            Some("sk-test".to_string())
        );
    }

    #[test]
    fn anthropic_oauth_token_precedence() {
        let mut env = ProviderEnv::new();
        env.insert("ANTHROPIC_OAUTH_TOKEN".to_string(), "oauth".to_string());
        env.insert("ANTHROPIC_API_KEY".to_string(), "apikey".to_string());
        // OAuth token is listed first → preferred.
        assert_eq!(
            get_env_api_key("anthropic", Some(&env)),
            Some("oauth".to_string())
        );
    }

    #[test]
    fn anthropic_deepseek_base_url_excludes_api_key() {
        let mut env = ProviderEnv::new();
        env.insert(
            "ANTHROPIC_BASE_URL".to_string(),
            "https://api.deepseek.com/anthropic".to_string(),
        );
        env.insert("ANTHROPIC_API_KEY".to_string(), "ds-key".to_string());
        // Only ANTHROPIC_OAUTH_TOKEN is considered for anthropic → none set → None.
        assert_eq!(get_env_api_key("anthropic", Some(&env)), None);

        // deepseek provider picks up ANTHROPIC_API_KEY as a candidate in this case.
        // (Assert membership rather than the resolved key, since an ambient
        // DEEPSEEK_API_KEY in the process env would otherwise take precedence.)
        let keys = find_env_keys("deepseek", Some(&env)).expect("at least one key");
        assert!(keys.contains(&"ANTHROPIC_API_KEY".to_string()));
    }

    #[test]
    fn github_copilot_uses_copilot_token() {
        let mut env = ProviderEnv::new();
        env.insert("COPILOT_GITHUB_TOKEN".to_string(), "ghp".to_string());
        assert_eq!(
            get_env_api_key("github-copilot", Some(&env)),
            Some("ghp".to_string())
        );
    }

    #[test]
    fn unknown_provider_returns_none() {
        let env = ProviderEnv::new();
        assert_eq!(get_env_api_key("does-not-exist", Some(&env)), None);
    }

    #[test]
    fn generic_github_tokens_not_treated_as_github_copilot_creds() {
        let mut env = ProviderEnv::new();
        env.insert("GH_TOKEN".to_string(), "gh-token".to_string());
        env.insert("GITHUB_TOKEN".to_string(), "github-token".to_string());
        // find_env_keys should return None since COPILOT_GITHUB_TOKEN is not set
        assert_eq!(find_env_keys("github-copilot", Some(&env)), None);
        assert_eq!(get_env_api_key("github-copilot", Some(&env)), None);
    }

    #[test]
    fn resolves_github_copilot_from_copilot_github_token() {
        let mut env = ProviderEnv::new();
        env.insert(
            "COPILOT_GITHUB_TOKEN".to_string(),
            "copilot-token".to_string(),
        );
        env.insert("GH_TOKEN".to_string(), "gh-token".to_string());
        env.insert("GITHUB_TOKEN".to_string(), "github-token".to_string());

        assert_eq!(
            find_env_keys("github-copilot", Some(&env)),
            Some(vec!["COPILOT_GITHUB_TOKEN".to_string()])
        );
        assert_eq!(
            get_env_api_key("github-copilot", Some(&env)),
            Some("copilot-token".to_string())
        );
    }

    #[test]
    fn resolves_zai_coding_cn_from_zai_coding_cn_api_key() {
        let mut env = ProviderEnv::new();
        env.insert(
            "ZAI_CODING_CN_API_KEY".to_string(),
            "zai-coding-cn-token".to_string(),
        );

        assert_eq!(
            find_env_keys("zai-coding-cn", Some(&env)),
            Some(vec!["ZAI_CODING_CN_API_KEY".to_string()])
        );
        assert_eq!(
            get_env_api_key("zai-coding-cn", Some(&env)),
            Some("zai-coding-cn-token".to_string())
        );
    }

    #[test]
    fn together_api_key_resolved_from_env() {
        let mut env = ProviderEnv::new();
        env.insert(
            "TOGETHER_API_KEY".to_string(),
            "test-together-key".to_string(),
        );

        assert_eq!(
            find_env_keys("together", Some(&env)),
            Some(vec!["TOGETHER_API_KEY".to_string()])
        );
        assert_eq!(
            get_env_api_key("together", Some(&env)),
            Some("test-together-key".to_string())
        );
    }

    #[test]
    fn fireworks_api_key_resolved_from_env() {
        let mut env = ProviderEnv::new();
        env.insert(
            "FIREWORKS_API_KEY".to_string(),
            "test-fireworks-key".to_string(),
        );

        assert_eq!(
            find_env_keys("fireworks", Some(&env)),
            Some(vec!["FIREWORKS_API_KEY".to_string()])
        );
        assert_eq!(
            get_env_api_key("fireworks", Some(&env)),
            Some("test-fireworks-key".to_string())
        );
    }
}
