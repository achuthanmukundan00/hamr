//! Auto-generated model catalogue for `opencode-go`.
//! Do not edit manually — re-run `scripts/generate_models.py`.

use std::collections::HashMap;

use crate::types::{Api, InputModality, Model, ModelCost, ModelThinkingLevel};

/// Model catalogue for the `opencode-go` provider.
pub fn models() -> HashMap<String, Model> {
    let mut map: HashMap<String, Model> = HashMap::new();

    map.insert(
        "deepseek-v4-flash".into(),
        Model {
            id: "deepseek-v4-flash".into(),
            name: "DeepSeek V4 Flash".into(),
            api: Api::OpenAiCompletions,
            provider: "opencode-go".into(),
            base_url: "https://opencode.ai/zen/go/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.14,
                output: 0.28,
                cache_read: 0.0028,
                cache_write: 0.0,
            },
            context_window: 1000000,
            max_tokens: 384000,
            thinking_level_map: Some(HashMap::from([
                (ModelThinkingLevel::Minimal, None),
                (ModelThinkingLevel::Low, None),
                (ModelThinkingLevel::Medium, None),
                (ModelThinkingLevel::High, Some("high".into())),
                (ModelThinkingLevel::XHigh, Some("max".into())),
            ])),
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "deepseek-v4-pro".into(),
        Model {
            id: "deepseek-v4-pro".into(),
            name: "DeepSeek V4 Pro".into(),
            api: Api::OpenAiCompletions,
            provider: "opencode-go".into(),
            base_url: "https://opencode.ai/zen/go/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 1.74,
                output: 3.48,
                cache_read: 0.0145,
                cache_write: 0.0,
            },
            context_window: 1000000,
            max_tokens: 384000,
            thinking_level_map: Some(HashMap::from([
                (ModelThinkingLevel::Minimal, None),
                (ModelThinkingLevel::Low, None),
                (ModelThinkingLevel::Medium, None),
                (ModelThinkingLevel::High, Some("high".into())),
                (ModelThinkingLevel::XHigh, Some("max".into())),
            ])),
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "glm-5.1".into(),
        Model {
            id: "glm-5.1".into(),
            name: "GLM-5.1".into(),
            api: Api::OpenAiCompletions,
            provider: "opencode-go".into(),
            base_url: "https://opencode.ai/zen/go/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 1.4,
                output: 4.4,
                cache_read: 0.26,
                cache_write: 0.0,
            },
            context_window: 202752,
            max_tokens: 32768,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "glm-5.2".into(),
        Model {
            id: "glm-5.2".into(),
            name: "GLM-5.2".into(),
            api: Api::OpenAiCompletions,
            provider: "opencode-go".into(),
            base_url: "https://opencode.ai/zen/go/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 1.4,
                output: 4.4,
                cache_read: 0.26,
                cache_write: 0.0,
            },
            context_window: 1000000,
            max_tokens: 131072,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "kimi-k2.6".into(),
        Model {
            id: "kimi-k2.6".into(),
            name: "Kimi K2.6".into(),
            api: Api::OpenAiCompletions,
            provider: "opencode-go".into(),
            base_url: "https://opencode.ai/zen/go/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.95,
                output: 4.0,
                cache_read: 0.16,
                cache_write: 0.0,
            },
            context_window: 262144,
            max_tokens: 65536,
            thinking_level_map: Some(HashMap::from([
                (ModelThinkingLevel::Minimal, None),
                (ModelThinkingLevel::Low, None),
                (ModelThinkingLevel::Medium, None),
            ])),
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "kimi-k2.7-code".into(),
        Model {
            id: "kimi-k2.7-code".into(),
            name: "Kimi K2.7 Code".into(),
            api: Api::OpenAiCompletions,
            provider: "opencode-go".into(),
            base_url: "https://opencode.ai/zen/go/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.95,
                output: 4.0,
                cache_read: 0.19,
                cache_write: 0.0,
            },
            context_window: 262144,
            max_tokens: 262144,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "mimo-v2.5".into(),
        Model {
            id: "mimo-v2.5".into(),
            name: "MiMo V2.5".into(),
            api: Api::OpenAiCompletions,
            provider: "opencode-go".into(),
            base_url: "https://opencode.ai/zen/go/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.14,
                output: 0.28,
                cache_read: 0.0028,
                cache_write: 0.0,
            },
            context_window: 1000000,
            max_tokens: 128000,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "mimo-v2.5-pro".into(),
        Model {
            id: "mimo-v2.5-pro".into(),
            name: "MiMo V2.5 Pro".into(),
            api: Api::OpenAiCompletions,
            provider: "opencode-go".into(),
            base_url: "https://opencode.ai/zen/go/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 1.74,
                output: 3.48,
                cache_read: 0.0145,
                cache_write: 0.0,
            },
            context_window: 1048576,
            max_tokens: 128000,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "minimax-m2.7".into(),
        Model {
            id: "minimax-m2.7".into(),
            name: "MiniMax M2.7".into(),
            api: Api::OpenAiCompletions,
            provider: "opencode-go".into(),
            base_url: "https://opencode.ai/zen/go/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.3,
                output: 1.2,
                cache_read: 0.06,
                cache_write: 0.0,
            },
            context_window: 204800,
            max_tokens: 131072,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "minimax-m3".into(),
        Model {
            id: "minimax-m3".into(),
            name: "MiniMax M3 (3x usage)".into(),
            api: Api::AnthropicMessages,
            provider: "opencode-go".into(),
            base_url: "https://opencode.ai/zen/go".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.1,
                output: 0.4,
                cache_read: 0.02,
                cache_write: 0.0,
            },
            context_window: 1000000,
            max_tokens: 131072,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "qwen3.6-plus".into(),
        Model {
            id: "qwen3.6-plus".into(),
            name: "Qwen3.6 Plus".into(),
            api: Api::OpenAiCompletions,
            provider: "opencode-go".into(),
            base_url: "https://opencode.ai/zen/go/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.5,
                output: 3.0,
                cache_read: 0.05,
                cache_write: 0.625,
            },
            context_window: 1000000,
            max_tokens: 65536,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "qwen3.7-max".into(),
        Model {
            id: "qwen3.7-max".into(),
            name: "Qwen3.7 Max".into(),
            api: Api::AnthropicMessages,
            provider: "opencode-go".into(),
            base_url: "https://opencode.ai/zen/go".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 2.5,
                output: 7.5,
                cache_read: 0.5,
                cache_write: 3.125,
            },
            context_window: 1000000,
            max_tokens: 65536,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "qwen3.7-plus".into(),
        Model {
            id: "qwen3.7-plus".into(),
            name: "Qwen3.7 Plus".into(),
            api: Api::AnthropicMessages,
            provider: "opencode-go".into(),
            base_url: "https://opencode.ai/zen/go".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.4,
                output: 1.6,
                cache_read: 0.04,
                cache_write: 0.5,
            },
            context_window: 1000000,
            max_tokens: 65536,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map
}
