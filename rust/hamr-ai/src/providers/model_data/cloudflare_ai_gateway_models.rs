//! Auto-generated model catalogue for `cloudflare-ai-gateway`.
//! Do not edit manually — re-run `scripts/generate_models.py`.

use std::collections::HashMap;

use crate::types::{Api, InputModality, Model, ModelCost, ModelThinkingLevel};

/// Model catalogue for the `cloudflare-ai-gateway` provider.
pub fn models() -> HashMap<String, Model> {
    let mut map: HashMap<String, Model> = HashMap::new();

    map.insert(
        "claude-3-5-haiku".into(),
        Model {
        id: "claude-3-5-haiku".into(),
        name: "Claude Haiku 3.5 (latest)".into(),
        api: Api::AnthropicMessages,
        provider: "cloudflare-ai-gateway".into(),
        base_url: "https://gateway.ai.cloudflare.com/v1/{CLOUDFLARE_ACCOUNT_ID}/{CLOUDFLARE_GATEWAY_ID}/anthropic".into(),
        reasoning: false,
        input: vec![InputModality::Text, InputModality::Image],
        cost: ModelCost { input: 0.8, output: 4.0, cache_read: 0.08, cache_write: 1.0 },
        context_window: 200000,
        max_tokens: 8192,
        thinking_level_map: None,
        headers: None,
            compat: None
    },
    );

    map.insert(
        "claude-3-haiku".into(),
        Model {
        id: "claude-3-haiku".into(),
        name: "Claude Haiku 3".into(),
        api: Api::AnthropicMessages,
        provider: "cloudflare-ai-gateway".into(),
        base_url: "https://gateway.ai.cloudflare.com/v1/{CLOUDFLARE_ACCOUNT_ID}/{CLOUDFLARE_GATEWAY_ID}/anthropic".into(),
        reasoning: false,
        input: vec![InputModality::Text, InputModality::Image],
        cost: ModelCost { input: 0.25, output: 1.25, cache_read: 0.03, cache_write: 0.3 },
        context_window: 200000,
        max_tokens: 4096,
        thinking_level_map: None,
        headers: None,
            compat: None
    },
    );

    map.insert(
        "claude-3-opus".into(),
        Model {
        id: "claude-3-opus".into(),
        name: "Claude Opus 3".into(),
        api: Api::AnthropicMessages,
        provider: "cloudflare-ai-gateway".into(),
        base_url: "https://gateway.ai.cloudflare.com/v1/{CLOUDFLARE_ACCOUNT_ID}/{CLOUDFLARE_GATEWAY_ID}/anthropic".into(),
        reasoning: false,
        input: vec![InputModality::Text, InputModality::Image],
        cost: ModelCost { input: 15.0, output: 75.0, cache_read: 1.5, cache_write: 18.75 },
        context_window: 200000,
        max_tokens: 4096,
        thinking_level_map: None,
        headers: None,
            compat: None
    },
    );

    map.insert(
        "claude-3-sonnet".into(),
        Model {
        id: "claude-3-sonnet".into(),
        name: "Claude Sonnet 3".into(),
        api: Api::AnthropicMessages,
        provider: "cloudflare-ai-gateway".into(),
        base_url: "https://gateway.ai.cloudflare.com/v1/{CLOUDFLARE_ACCOUNT_ID}/{CLOUDFLARE_GATEWAY_ID}/anthropic".into(),
        reasoning: false,
        input: vec![InputModality::Text, InputModality::Image],
        cost: ModelCost { input: 3.0, output: 15.0, cache_read: 0.3, cache_write: 0.3 },
        context_window: 200000,
        max_tokens: 4096,
        thinking_level_map: None,
        headers: None,
            compat: None
    },
    );

    map.insert(
        "claude-3.5-haiku".into(),
        Model {
        id: "claude-3.5-haiku".into(),
        name: "Claude Haiku 3.5 (latest)".into(),
        api: Api::AnthropicMessages,
        provider: "cloudflare-ai-gateway".into(),
        base_url: "https://gateway.ai.cloudflare.com/v1/{CLOUDFLARE_ACCOUNT_ID}/{CLOUDFLARE_GATEWAY_ID}/anthropic".into(),
        reasoning: false,
        input: vec![InputModality::Text, InputModality::Image],
        cost: ModelCost { input: 0.8, output: 4.0, cache_read: 0.08, cache_write: 1.0 },
        context_window: 200000,
        max_tokens: 8192,
        thinking_level_map: None,
        headers: None,
            compat: None
    },
    );

    map.insert(
        "claude-3.5-sonnet".into(),
        Model {
        id: "claude-3.5-sonnet".into(),
        name: "Claude Sonnet 3.5 v2".into(),
        api: Api::AnthropicMessages,
        provider: "cloudflare-ai-gateway".into(),
        base_url: "https://gateway.ai.cloudflare.com/v1/{CLOUDFLARE_ACCOUNT_ID}/{CLOUDFLARE_GATEWAY_ID}/anthropic".into(),
        reasoning: false,
        input: vec![InputModality::Text, InputModality::Image],
        cost: ModelCost { input: 3.0, output: 15.0, cache_read: 0.3, cache_write: 3.75 },
        context_window: 200000,
        max_tokens: 8192,
        thinking_level_map: None,
        headers: None,
            compat: None
    },
    );

    map.insert(
        "claude-fable-5".into(),
        Model {
        id: "claude-fable-5".into(),
        name: "Claude Fable 5".into(),
        api: Api::AnthropicMessages,
        provider: "cloudflare-ai-gateway".into(),
        base_url: "https://gateway.ai.cloudflare.com/v1/{CLOUDFLARE_ACCOUNT_ID}/{CLOUDFLARE_GATEWAY_ID}/anthropic".into(),
        reasoning: true,
        input: vec![InputModality::Text, InputModality::Image],
        cost: ModelCost { input: 10.0, output: 50.0, cache_read: 1.0, cache_write: 12.5 },
        context_window: 1000000,
        max_tokens: 128000,
        thinking_level_map: Some(HashMap::from([(ModelThinkingLevel::Off, None), (ModelThinkingLevel::XHigh, Some("xhigh".into()))])),
        headers: None,
            compat: None
    },
    );

    map.insert(
        "claude-haiku-4-5".into(),
        Model {
        id: "claude-haiku-4-5".into(),
        name: "Claude Haiku 4.5 (latest)".into(),
        api: Api::AnthropicMessages,
        provider: "cloudflare-ai-gateway".into(),
        base_url: "https://gateway.ai.cloudflare.com/v1/{CLOUDFLARE_ACCOUNT_ID}/{CLOUDFLARE_GATEWAY_ID}/anthropic".into(),
        reasoning: true,
        input: vec![InputModality::Text, InputModality::Image],
        cost: ModelCost { input: 1.0, output: 5.0, cache_read: 0.1, cache_write: 1.25 },
        context_window: 200000,
        max_tokens: 64000,
        thinking_level_map: None,
        headers: None,
            compat: None
    },
    );

    map.insert(
        "claude-opus-4".into(),
        Model {
        id: "claude-opus-4".into(),
        name: "Claude Opus 4 (latest)".into(),
        api: Api::AnthropicMessages,
        provider: "cloudflare-ai-gateway".into(),
        base_url: "https://gateway.ai.cloudflare.com/v1/{CLOUDFLARE_ACCOUNT_ID}/{CLOUDFLARE_GATEWAY_ID}/anthropic".into(),
        reasoning: true,
        input: vec![InputModality::Text, InputModality::Image],
        cost: ModelCost { input: 15.0, output: 75.0, cache_read: 1.5, cache_write: 18.75 },
        context_window: 200000,
        max_tokens: 32000,
        thinking_level_map: None,
        headers: None,
            compat: None
    },
    );

    map.insert(
        "claude-opus-4-1".into(),
        Model {
        id: "claude-opus-4-1".into(),
        name: "Claude Opus 4.1 (latest)".into(),
        api: Api::AnthropicMessages,
        provider: "cloudflare-ai-gateway".into(),
        base_url: "https://gateway.ai.cloudflare.com/v1/{CLOUDFLARE_ACCOUNT_ID}/{CLOUDFLARE_GATEWAY_ID}/anthropic".into(),
        reasoning: true,
        input: vec![InputModality::Text, InputModality::Image],
        cost: ModelCost { input: 15.0, output: 75.0, cache_read: 1.5, cache_write: 18.75 },
        context_window: 200000,
        max_tokens: 32000,
        thinking_level_map: None,
        headers: None,
            compat: None
    },
    );

    map.insert(
        "claude-opus-4-5".into(),
        Model {
        id: "claude-opus-4-5".into(),
        name: "Claude Opus 4.5 (latest)".into(),
        api: Api::AnthropicMessages,
        provider: "cloudflare-ai-gateway".into(),
        base_url: "https://gateway.ai.cloudflare.com/v1/{CLOUDFLARE_ACCOUNT_ID}/{CLOUDFLARE_GATEWAY_ID}/anthropic".into(),
        reasoning: true,
        input: vec![InputModality::Text, InputModality::Image],
        cost: ModelCost { input: 5.0, output: 25.0, cache_read: 0.5, cache_write: 6.25 },
        context_window: 200000,
        max_tokens: 64000,
        thinking_level_map: None,
        headers: None,
            compat: None
    },
    );

    map.insert(
        "claude-opus-4-6".into(),
        Model {
        id: "claude-opus-4-6".into(),
        name: "Claude Opus 4.6 (latest)".into(),
        api: Api::AnthropicMessages,
        provider: "cloudflare-ai-gateway".into(),
        base_url: "https://gateway.ai.cloudflare.com/v1/{CLOUDFLARE_ACCOUNT_ID}/{CLOUDFLARE_GATEWAY_ID}/anthropic".into(),
        reasoning: true,
        input: vec![InputModality::Text, InputModality::Image],
        cost: ModelCost { input: 5.0, output: 25.0, cache_read: 0.5, cache_write: 6.25 },
        context_window: 1000000,
        max_tokens: 128000,
        thinking_level_map: Some(HashMap::from([(ModelThinkingLevel::XHigh, Some("max".into()))])),
        headers: None,
            compat: None
    },
    );

    map.insert(
        "claude-opus-4-7".into(),
        Model {
        id: "claude-opus-4-7".into(),
        name: "Claude Opus 4.7".into(),
        api: Api::AnthropicMessages,
        provider: "cloudflare-ai-gateway".into(),
        base_url: "https://gateway.ai.cloudflare.com/v1/{CLOUDFLARE_ACCOUNT_ID}/{CLOUDFLARE_GATEWAY_ID}/anthropic".into(),
        reasoning: true,
        input: vec![InputModality::Text, InputModality::Image],
        cost: ModelCost { input: 5.0, output: 25.0, cache_read: 0.5, cache_write: 6.25 },
        context_window: 1000000,
        max_tokens: 128000,
        thinking_level_map: Some(HashMap::from([(ModelThinkingLevel::XHigh, Some("xhigh".into()))])),
        headers: None,
            compat: None
    },
    );

    map.insert(
        "claude-opus-4-8".into(),
        Model {
        id: "claude-opus-4-8".into(),
        name: "Claude Opus 4.8".into(),
        api: Api::AnthropicMessages,
        provider: "cloudflare-ai-gateway".into(),
        base_url: "https://gateway.ai.cloudflare.com/v1/{CLOUDFLARE_ACCOUNT_ID}/{CLOUDFLARE_GATEWAY_ID}/anthropic".into(),
        reasoning: true,
        input: vec![InputModality::Text, InputModality::Image],
        cost: ModelCost { input: 5.0, output: 25.0, cache_read: 0.5, cache_write: 6.25 },
        context_window: 1000000,
        max_tokens: 128000,
        thinking_level_map: Some(HashMap::from([(ModelThinkingLevel::XHigh, Some("xhigh".into()))])),
        headers: None,
            compat: None
    },
    );

    map.insert(
        "claude-sonnet-4".into(),
        Model {
        id: "claude-sonnet-4".into(),
        name: "Claude Sonnet 4 (latest)".into(),
        api: Api::AnthropicMessages,
        provider: "cloudflare-ai-gateway".into(),
        base_url: "https://gateway.ai.cloudflare.com/v1/{CLOUDFLARE_ACCOUNT_ID}/{CLOUDFLARE_GATEWAY_ID}/anthropic".into(),
        reasoning: true,
        input: vec![InputModality::Text, InputModality::Image],
        cost: ModelCost { input: 3.0, output: 15.0, cache_read: 0.3, cache_write: 3.75 },
        context_window: 200000,
        max_tokens: 64000,
        thinking_level_map: None,
        headers: None,
            compat: None
    },
    );

    map.insert(
        "claude-sonnet-4-5".into(),
        Model {
        id: "claude-sonnet-4-5".into(),
        name: "Claude Sonnet 4.5 (latest)".into(),
        api: Api::AnthropicMessages,
        provider: "cloudflare-ai-gateway".into(),
        base_url: "https://gateway.ai.cloudflare.com/v1/{CLOUDFLARE_ACCOUNT_ID}/{CLOUDFLARE_GATEWAY_ID}/anthropic".into(),
        reasoning: true,
        input: vec![InputModality::Text, InputModality::Image],
        cost: ModelCost { input: 3.0, output: 15.0, cache_read: 0.3, cache_write: 3.75 },
        context_window: 200000,
        max_tokens: 64000,
        thinking_level_map: None,
        headers: None,
            compat: None
    },
    );

    map.insert(
        "claude-sonnet-4-6".into(),
        Model {
        id: "claude-sonnet-4-6".into(),
        name: "Claude Sonnet 4.6".into(),
        api: Api::AnthropicMessages,
        provider: "cloudflare-ai-gateway".into(),
        base_url: "https://gateway.ai.cloudflare.com/v1/{CLOUDFLARE_ACCOUNT_ID}/{CLOUDFLARE_GATEWAY_ID}/anthropic".into(),
        reasoning: true,
        input: vec![InputModality::Text, InputModality::Image],
        cost: ModelCost { input: 3.0, output: 15.0, cache_read: 0.3, cache_write: 3.75 },
        context_window: 1000000,
        max_tokens: 64000,
        thinking_level_map: None,
        headers: None,
            compat: None
    },
    );

    map.insert(
        "gpt-4".into(),
        Model {
        id: "gpt-4".into(),
        name: "GPT-4".into(),
        api: Api::OpenAiResponses,
        provider: "cloudflare-ai-gateway".into(),
        base_url: "https://gateway.ai.cloudflare.com/v1/{CLOUDFLARE_ACCOUNT_ID}/{CLOUDFLARE_GATEWAY_ID}/openai".into(),
        reasoning: false,
        input: vec![InputModality::Text],
        cost: ModelCost { input: 30.0, output: 60.0, cache_read: 0.0, cache_write: 0.0 },
        context_window: 8192,
        max_tokens: 8192,
        thinking_level_map: None,
        headers: None,
            compat: None
    },
    );

    map.insert(
        "gpt-4-turbo".into(),
        Model {
        id: "gpt-4-turbo".into(),
        name: "GPT-4 Turbo".into(),
        api: Api::OpenAiResponses,
        provider: "cloudflare-ai-gateway".into(),
        base_url: "https://gateway.ai.cloudflare.com/v1/{CLOUDFLARE_ACCOUNT_ID}/{CLOUDFLARE_GATEWAY_ID}/openai".into(),
        reasoning: false,
        input: vec![InputModality::Text, InputModality::Image],
        cost: ModelCost { input: 10.0, output: 30.0, cache_read: 0.0, cache_write: 0.0 },
        context_window: 128000,
        max_tokens: 4096,
        thinking_level_map: None,
        headers: None,
            compat: None
    },
    );

    map.insert(
        "gpt-4o".into(),
        Model {
        id: "gpt-4o".into(),
        name: "GPT-4o".into(),
        api: Api::OpenAiResponses,
        provider: "cloudflare-ai-gateway".into(),
        base_url: "https://gateway.ai.cloudflare.com/v1/{CLOUDFLARE_ACCOUNT_ID}/{CLOUDFLARE_GATEWAY_ID}/openai".into(),
        reasoning: false,
        input: vec![InputModality::Text, InputModality::Image],
        cost: ModelCost { input: 2.5, output: 10.0, cache_read: 1.25, cache_write: 0.0 },
        context_window: 128000,
        max_tokens: 16384,
        thinking_level_map: None,
        headers: None,
            compat: None
    },
    );

    map.insert(
        "gpt-4o-mini".into(),
        Model {
        id: "gpt-4o-mini".into(),
        name: "GPT-4o mini".into(),
        api: Api::OpenAiResponses,
        provider: "cloudflare-ai-gateway".into(),
        base_url: "https://gateway.ai.cloudflare.com/v1/{CLOUDFLARE_ACCOUNT_ID}/{CLOUDFLARE_GATEWAY_ID}/openai".into(),
        reasoning: false,
        input: vec![InputModality::Text, InputModality::Image],
        cost: ModelCost { input: 0.15, output: 0.6, cache_read: 0.08, cache_write: 0.0 },
        context_window: 128000,
        max_tokens: 16384,
        thinking_level_map: None,
        headers: None,
            compat: None
    },
    );

    map.insert(
        "gpt-5.1".into(),
        Model {
        id: "gpt-5.1".into(),
        name: "GPT-5.1".into(),
        api: Api::OpenAiResponses,
        provider: "cloudflare-ai-gateway".into(),
        base_url: "https://gateway.ai.cloudflare.com/v1/{CLOUDFLARE_ACCOUNT_ID}/{CLOUDFLARE_GATEWAY_ID}/openai".into(),
        reasoning: true,
        input: vec![InputModality::Text, InputModality::Image],
        cost: ModelCost { input: 1.25, output: 10.0, cache_read: 0.13, cache_write: 0.0 },
        context_window: 400000,
        max_tokens: 128000,
        thinking_level_map: Some(HashMap::from([(ModelThinkingLevel::Off, None)])),
        headers: None,
            compat: None
    },
    );

    map.insert(
        "gpt-5.1-codex".into(),
        Model {
        id: "gpt-5.1-codex".into(),
        name: "GPT-5.1 Codex".into(),
        api: Api::OpenAiResponses,
        provider: "cloudflare-ai-gateway".into(),
        base_url: "https://gateway.ai.cloudflare.com/v1/{CLOUDFLARE_ACCOUNT_ID}/{CLOUDFLARE_GATEWAY_ID}/openai".into(),
        reasoning: true,
        input: vec![InputModality::Text, InputModality::Image],
        cost: ModelCost { input: 1.25, output: 10.0, cache_read: 0.125, cache_write: 0.0 },
        context_window: 400000,
        max_tokens: 128000,
        thinking_level_map: Some(HashMap::from([(ModelThinkingLevel::Off, None)])),
        headers: None,
            compat: None
    },
    );

    map.insert(
        "gpt-5.2".into(),
        Model {
        id: "gpt-5.2".into(),
        name: "GPT-5.2".into(),
        api: Api::OpenAiResponses,
        provider: "cloudflare-ai-gateway".into(),
        base_url: "https://gateway.ai.cloudflare.com/v1/{CLOUDFLARE_ACCOUNT_ID}/{CLOUDFLARE_GATEWAY_ID}/openai".into(),
        reasoning: true,
        input: vec![InputModality::Text, InputModality::Image],
        cost: ModelCost { input: 1.75, output: 14.0, cache_read: 0.175, cache_write: 0.0 },
        context_window: 400000,
        max_tokens: 128000,
        thinking_level_map: Some(HashMap::from([(ModelThinkingLevel::Off, None), (ModelThinkingLevel::XHigh, Some("xhigh".into()))])),
        headers: None,
            compat: None
    },
    );

    map.insert(
        "gpt-5.2-codex".into(),
        Model {
        id: "gpt-5.2-codex".into(),
        name: "GPT-5.2 Codex".into(),
        api: Api::OpenAiResponses,
        provider: "cloudflare-ai-gateway".into(),
        base_url: "https://gateway.ai.cloudflare.com/v1/{CLOUDFLARE_ACCOUNT_ID}/{CLOUDFLARE_GATEWAY_ID}/openai".into(),
        reasoning: true,
        input: vec![InputModality::Text, InputModality::Image],
        cost: ModelCost { input: 1.75, output: 14.0, cache_read: 0.175, cache_write: 0.0 },
        context_window: 400000,
        max_tokens: 128000,
        thinking_level_map: Some(HashMap::from([(ModelThinkingLevel::Off, None), (ModelThinkingLevel::XHigh, Some("xhigh".into()))])),
        headers: None,
            compat: None
    },
    );

    map.insert(
        "gpt-5.3-codex".into(),
        Model {
        id: "gpt-5.3-codex".into(),
        name: "GPT-5.3 Codex".into(),
        api: Api::OpenAiResponses,
        provider: "cloudflare-ai-gateway".into(),
        base_url: "https://gateway.ai.cloudflare.com/v1/{CLOUDFLARE_ACCOUNT_ID}/{CLOUDFLARE_GATEWAY_ID}/openai".into(),
        reasoning: true,
        input: vec![InputModality::Text, InputModality::Image],
        cost: ModelCost { input: 1.75, output: 14.0, cache_read: 0.175, cache_write: 0.0 },
        context_window: 400000,
        max_tokens: 128000,
        thinking_level_map: Some(HashMap::from([(ModelThinkingLevel::Off, None), (ModelThinkingLevel::XHigh, Some("xhigh".into()))])),
        headers: None,
            compat: None
    },
    );

    map.insert(
        "gpt-5.4".into(),
        Model {
        id: "gpt-5.4".into(),
        name: "GPT-5.4".into(),
        api: Api::OpenAiResponses,
        provider: "cloudflare-ai-gateway".into(),
        base_url: "https://gateway.ai.cloudflare.com/v1/{CLOUDFLARE_ACCOUNT_ID}/{CLOUDFLARE_GATEWAY_ID}/openai".into(),
        reasoning: true,
        input: vec![InputModality::Text, InputModality::Image],
        cost: ModelCost { input: 2.5, output: 15.0, cache_read: 0.25, cache_write: 0.0 },
        context_window: 1050000,
        max_tokens: 128000,
        thinking_level_map: Some(HashMap::from([(ModelThinkingLevel::Off, None), (ModelThinkingLevel::XHigh, Some("xhigh".into()))])),
        headers: None,
            compat: None
    },
    );

    map.insert(
        "gpt-5.5".into(),
        Model {
        id: "gpt-5.5".into(),
        name: "GPT-5.5".into(),
        api: Api::OpenAiResponses,
        provider: "cloudflare-ai-gateway".into(),
        base_url: "https://gateway.ai.cloudflare.com/v1/{CLOUDFLARE_ACCOUNT_ID}/{CLOUDFLARE_GATEWAY_ID}/openai".into(),
        reasoning: true,
        input: vec![InputModality::Text, InputModality::Image],
        cost: ModelCost { input: 5.0, output: 30.0, cache_read: 0.5, cache_write: 0.0 },
        context_window: 1050000,
        max_tokens: 128000,
        thinking_level_map: Some(HashMap::from([(ModelThinkingLevel::Off, None), (ModelThinkingLevel::XHigh, Some("xhigh".into()))])),
        headers: None,
            compat: None
    },
    );

    map.insert(
        "o3-mini".into(),
        Model {
        id: "o3-mini".into(),
        name: "o3-mini".into(),
        api: Api::OpenAiResponses,
        provider: "cloudflare-ai-gateway".into(),
        base_url: "https://gateway.ai.cloudflare.com/v1/{CLOUDFLARE_ACCOUNT_ID}/{CLOUDFLARE_GATEWAY_ID}/openai".into(),
        reasoning: true,
        input: vec![InputModality::Text],
        cost: ModelCost { input: 1.1, output: 4.4, cache_read: 0.55, cache_write: 0.0 },
        context_window: 200000,
        max_tokens: 100000,
        thinking_level_map: None,
        headers: None,
            compat: None
    },
    );

    map.insert(
        "o3-pro".into(),
        Model {
        id: "o3-pro".into(),
        name: "o3-pro".into(),
        api: Api::OpenAiResponses,
        provider: "cloudflare-ai-gateway".into(),
        base_url: "https://gateway.ai.cloudflare.com/v1/{CLOUDFLARE_ACCOUNT_ID}/{CLOUDFLARE_GATEWAY_ID}/openai".into(),
        reasoning: true,
        input: vec![InputModality::Text, InputModality::Image],
        cost: ModelCost { input: 20.0, output: 80.0, cache_read: 0.0, cache_write: 0.0 },
        context_window: 200000,
        max_tokens: 100000,
        thinking_level_map: None,
        headers: None,
            compat: None
    },
    );

    map.insert(
        "o4-mini".into(),
        Model {
        id: "o4-mini".into(),
        name: "o4-mini".into(),
        api: Api::OpenAiResponses,
        provider: "cloudflare-ai-gateway".into(),
        base_url: "https://gateway.ai.cloudflare.com/v1/{CLOUDFLARE_ACCOUNT_ID}/{CLOUDFLARE_GATEWAY_ID}/openai".into(),
        reasoning: true,
        input: vec![InputModality::Text, InputModality::Image],
        cost: ModelCost { input: 1.1, output: 4.4, cache_read: 0.28, cache_write: 0.0 },
        context_window: 200000,
        max_tokens: 100000,
        thinking_level_map: None,
        headers: None,
            compat: None
    },
    );

    map.insert(
        "workers-ai/@cf/moonshotai/kimi-k2.5".into(),
        Model {
        id: "workers-ai/@cf/moonshotai/kimi-k2.5".into(),
        name: "Kimi K2.5".into(),
        api: Api::OpenAiCompletions,
        provider: "cloudflare-ai-gateway".into(),
        base_url: "https://gateway.ai.cloudflare.com/v1/{CLOUDFLARE_ACCOUNT_ID}/{CLOUDFLARE_GATEWAY_ID}/compat".into(),
        reasoning: true,
        input: vec![InputModality::Text, InputModality::Image],
        cost: ModelCost { input: 0.6, output: 3.0, cache_read: 0.1, cache_write: 0.0 },
        context_window: 256000,
        max_tokens: 256000,
        thinking_level_map: None,
        headers: None,
            compat: None
    },
    );

    map.insert(
        "workers-ai/@cf/moonshotai/kimi-k2.6".into(),
        Model {
        id: "workers-ai/@cf/moonshotai/kimi-k2.6".into(),
        name: "Kimi K2.6".into(),
        api: Api::OpenAiCompletions,
        provider: "cloudflare-ai-gateway".into(),
        base_url: "https://gateway.ai.cloudflare.com/v1/{CLOUDFLARE_ACCOUNT_ID}/{CLOUDFLARE_GATEWAY_ID}/compat".into(),
        reasoning: true,
        input: vec![InputModality::Text, InputModality::Image],
        cost: ModelCost { input: 0.95, output: 4.0, cache_read: 0.16, cache_write: 0.0 },
        context_window: 256000,
        max_tokens: 256000,
        thinking_level_map: None,
        headers: None,
            compat: None
    },
    );

    map.insert(
        "workers-ai/@cf/nvidia/nemotron-3-120b-a12b".into(),
        Model {
        id: "workers-ai/@cf/nvidia/nemotron-3-120b-a12b".into(),
        name: "Nemotron 3 Super 120B".into(),
        api: Api::OpenAiCompletions,
        provider: "cloudflare-ai-gateway".into(),
        base_url: "https://gateway.ai.cloudflare.com/v1/{CLOUDFLARE_ACCOUNT_ID}/{CLOUDFLARE_GATEWAY_ID}/compat".into(),
        reasoning: true,
        input: vec![InputModality::Text],
        cost: ModelCost { input: 0.5, output: 1.5, cache_read: 0.0, cache_write: 0.0 },
        context_window: 256000,
        max_tokens: 256000,
        thinking_level_map: None,
        headers: None,
            compat: None
    },
    );

    map.insert(
        "workers-ai/@cf/zai-org/glm-4.7-flash".into(),
        Model {
        id: "workers-ai/@cf/zai-org/glm-4.7-flash".into(),
        name: "GLM-4.7-Flash".into(),
        api: Api::OpenAiCompletions,
        provider: "cloudflare-ai-gateway".into(),
        base_url: "https://gateway.ai.cloudflare.com/v1/{CLOUDFLARE_ACCOUNT_ID}/{CLOUDFLARE_GATEWAY_ID}/compat".into(),
        reasoning: true,
        input: vec![InputModality::Text],
        cost: ModelCost { input: 0.06, output: 0.4, cache_read: 0.0, cache_write: 0.0 },
        context_window: 131072,
        max_tokens: 131072,
        thinking_level_map: None,
        headers: None,
            compat: None
    },
    );

    map
}
