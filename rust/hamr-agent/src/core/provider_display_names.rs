use std::collections::HashMap;
use std::sync::LazyLock;

/// Display names for built-in providers.
static BUILT_IN_PROVIDER_DISPLAY_NAMES: LazyLock<HashMap<String, String>> = LazyLock::new(|| {
    HashMap::from([
        ("anthropic".to_string(), "Anthropic".to_string()),
        ("amazon-bedrock".to_string(), "Amazon Bedrock".to_string()),
        ("ant-ling".to_string(), "Ant Ling".to_string()),
        (
            "azure-openai-responses".to_string(),
            "Azure OpenAI Responses".to_string(),
        ),
        ("cerebras".to_string(), "Cerebras".to_string()),
        (
            "cloudflare-ai-gateway".to_string(),
            "Cloudflare AI Gateway".to_string(),
        ),
        (
            "cloudflare-workers-ai".to_string(),
            "Cloudflare Workers AI".to_string(),
        ),
        ("deepseek".to_string(), "DeepSeek".to_string()),
        ("fireworks".to_string(), "Fireworks".to_string()),
        ("google".to_string(), "Google Gemini".to_string()),
        ("google-vertex".to_string(), "Google Vertex AI".to_string()),
        ("groq".to_string(), "Groq".to_string()),
        ("huggingface".to_string(), "Hugging Face".to_string()),
        ("kimi-coding".to_string(), "Kimi For Coding".to_string()),
        ("mistral".to_string(), "Mistral".to_string()),
        ("minimax".to_string(), "MiniMax".to_string()),
        ("minimax-cn".to_string(), "MiniMax (China)".to_string()),
        ("moonshotai".to_string(), "Moonshot AI".to_string()),
        (
            "moonshotai-cn".to_string(),
            "Moonshot AI (China)".to_string(),
        ),
        ("nvidia".to_string(), "NVIDIA NIM".to_string()),
        ("opencode".to_string(), "OpenCode Zen".to_string()),
        ("opencode-go".to_string(), "OpenCode Go".to_string()),
        ("openai".to_string(), "OpenAI".to_string()),
        ("openrouter".to_string(), "OpenRouter".to_string()),
        ("together".to_string(), "Together AI".to_string()),
        (
            "vercel-ai-gateway".to_string(),
            "Vercel AI Gateway".to_string(),
        ),
        ("xai".to_string(), "xAI".to_string()),
        ("zai".to_string(), "ZAI".to_string()),
        (
            "zai-coding-cn".to_string(),
            "ZAI Coding Plan (China)".to_string(),
        ),
        ("xiaomi".to_string(), "Xiaomi MiMo".to_string()),
        (
            "xiaomi-token-plan-cn".to_string(),
            "Xiaomi MiMo Token Plan (China)".to_string(),
        ),
        (
            "xiaomi-token-plan-ams".to_string(),
            "Xiaomi MiMo Token Plan (Amsterdam)".to_string(),
        ),
        (
            "xiaomi-token-plan-sgp".to_string(),
            "Xiaomi MiMo Token Plan (Singapore)".to_string(),
        ),
    ])
});

/// Returns a reference to the static built-in provider display name map.
pub fn built_in_provider_display_names() -> &'static HashMap<String, String> {
    &BUILT_IN_PROVIDER_DISPLAY_NAMES
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_built_in_providers_have_display_names() {
        let names = built_in_provider_display_names();
        assert!(names.contains_key("anthropic"));
        assert!(names.contains_key("openai"));
        assert!(names.contains_key("google"));
        assert!(names.contains_key("nvidia"));
        assert!(names.contains_key("openrouter"));
    }

    #[test]
    fn test_expected_display_name_values() {
        let names = built_in_provider_display_names();
        assert_eq!(names.get("anthropic").unwrap(), "Anthropic");
        assert_eq!(names.get("openai").unwrap(), "OpenAI");
        assert_eq!(names.get("google").unwrap(), "Google Gemini");
        assert_eq!(names.get("nvidia").unwrap(), "NVIDIA NIM");
    }

    #[test]
    fn test_unknown_provider_returns_none() {
        let names = built_in_provider_display_names();
        assert!(names.get("nonexistent-provider").is_none());
    }

    #[test]
    fn test_static_does_not_panic_on_concurrent_access() {
        let names = built_in_provider_display_names();
        assert!(!names.is_empty());
    }
}
