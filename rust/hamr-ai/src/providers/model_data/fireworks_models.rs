//! Auto-generated model catalogue for `fireworks`.
//! Do not edit manually — re-run `scripts/generate_models.py`.

use std::collections::HashMap;

use crate::types::{Api, InputModality, Model, ModelCost};

/// Model catalogue for the `fireworks` provider.
pub fn models() -> HashMap<String, Model> {
    let mut map: HashMap<String, Model> = HashMap::new();

    map.insert(
        "accounts/fireworks/models/deepseek-v4-flash".into(),
        Model {
            id: "accounts/fireworks/models/deepseek-v4-flash".into(),
            name: "DeepSeek V4 Flash".into(),
            api: Api::AnthropicMessages,
            provider: "fireworks".into(),
            base_url: "https://api.fireworks.ai/inference".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.14,
                output: 0.28,
                cache_read: 0.028,
                cache_write: 0.0,
            },
            context_window: 1000000,
            max_tokens: 384000,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "accounts/fireworks/models/deepseek-v4-pro".into(),
        Model {
            id: "accounts/fireworks/models/deepseek-v4-pro".into(),
            name: "DeepSeek V4 Pro".into(),
            api: Api::AnthropicMessages,
            provider: "fireworks".into(),
            base_url: "https://api.fireworks.ai/inference".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 1.74,
                output: 3.48,
                cache_read: 0.145,
                cache_write: 0.0,
            },
            context_window: 1000000,
            max_tokens: 384000,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "accounts/fireworks/models/glm-5p1".into(),
        Model {
            id: "accounts/fireworks/models/glm-5p1".into(),
            name: "GLM 5.1".into(),
            api: Api::AnthropicMessages,
            provider: "fireworks".into(),
            base_url: "https://api.fireworks.ai/inference".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 1.4,
                output: 4.4,
                cache_read: 0.26,
                cache_write: 0.0,
            },
            context_window: 202800,
            max_tokens: 131072,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "accounts/fireworks/models/glm-5p2".into(),
        Model {
            id: "accounts/fireworks/models/glm-5p2".into(),
            name: "GLM 5.2".into(),
            api: Api::AnthropicMessages,
            provider: "fireworks".into(),
            base_url: "https://api.fireworks.ai/inference".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 1.4,
                output: 4.4,
                cache_read: 0.26,
                cache_write: 0.0,
            },
            context_window: 1048575,
            max_tokens: 131072,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "accounts/fireworks/models/gpt-oss-120b".into(),
        Model {
            id: "accounts/fireworks/models/gpt-oss-120b".into(),
            name: "GPT OSS 120B".into(),
            api: Api::AnthropicMessages,
            provider: "fireworks".into(),
            base_url: "https://api.fireworks.ai/inference".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.15,
                output: 0.6,
                cache_read: 0.015,
                cache_write: 0.0,
            },
            context_window: 131072,
            max_tokens: 32768,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "accounts/fireworks/models/gpt-oss-20b".into(),
        Model {
            id: "accounts/fireworks/models/gpt-oss-20b".into(),
            name: "GPT OSS 20B".into(),
            api: Api::AnthropicMessages,
            provider: "fireworks".into(),
            base_url: "https://api.fireworks.ai/inference".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.07,
                output: 0.3,
                cache_read: 0.035,
                cache_write: 0.0,
            },
            context_window: 131072,
            max_tokens: 32768,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "accounts/fireworks/models/kimi-k2p6".into(),
        Model {
            id: "accounts/fireworks/models/kimi-k2p6".into(),
            name: "Kimi K2.6".into(),
            api: Api::AnthropicMessages,
            provider: "fireworks".into(),
            base_url: "https://api.fireworks.ai/inference".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.95,
                output: 4.0,
                cache_read: 0.16,
                cache_write: 0.0,
            },
            context_window: 262000,
            max_tokens: 262000,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "accounts/fireworks/models/kimi-k2p7-code".into(),
        Model {
            id: "accounts/fireworks/models/kimi-k2p7-code".into(),
            name: "Kimi K2.7 Code".into(),
            api: Api::AnthropicMessages,
            provider: "fireworks".into(),
            base_url: "https://api.fireworks.ai/inference".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.95,
                output: 4.0,
                cache_read: 0.19,
                cache_write: 0.0,
            },
            context_window: 262000,
            max_tokens: 262000,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "accounts/fireworks/models/minimax-m2p7".into(),
        Model {
            id: "accounts/fireworks/models/minimax-m2p7".into(),
            name: "MiniMax-M2.7".into(),
            api: Api::AnthropicMessages,
            provider: "fireworks".into(),
            base_url: "https://api.fireworks.ai/inference".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.3,
                output: 1.2,
                cache_read: 0.06,
                cache_write: 0.0,
            },
            context_window: 196608,
            max_tokens: 196608,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "accounts/fireworks/models/minimax-m3".into(),
        Model {
            id: "accounts/fireworks/models/minimax-m3".into(),
            name: "MiniMax-M3".into(),
            api: Api::AnthropicMessages,
            provider: "fireworks".into(),
            base_url: "https://api.fireworks.ai/inference".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.3,
                output: 1.2,
                cache_read: 0.06,
                cache_write: 0.0,
            },
            context_window: 512000,
            max_tokens: 512000,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "accounts/fireworks/models/qwen3p7-plus".into(),
        Model {
            id: "accounts/fireworks/models/qwen3p7-plus".into(),
            name: "Qwen 3.7 Plus".into(),
            api: Api::AnthropicMessages,
            provider: "fireworks".into(),
            base_url: "https://api.fireworks.ai/inference".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.4,
                output: 1.6,
                cache_read: 0.08,
                cache_write: 0.0,
            },
            context_window: 262144,
            max_tokens: 65536,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "accounts/fireworks/routers/glm-5p1-fast".into(),
        Model {
            id: "accounts/fireworks/routers/glm-5p1-fast".into(),
            name: "GLM 5.1 Fast".into(),
            api: Api::AnthropicMessages,
            provider: "fireworks".into(),
            base_url: "https://api.fireworks.ai/inference".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 2.8,
                output: 8.8,
                cache_read: 0.52,
                cache_write: 0.0,
            },
            context_window: 202800,
            max_tokens: 131072,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "accounts/fireworks/routers/glm-5p2-fast".into(),
        Model {
            id: "accounts/fireworks/routers/glm-5p2-fast".into(),
            name: "GLM 5.2 Fast".into(),
            api: Api::AnthropicMessages,
            provider: "fireworks".into(),
            base_url: "https://api.fireworks.ai/inference".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 2.1,
                output: 6.6,
                cache_read: 0.21,
                cache_write: 0.0,
            },
            context_window: 1048575,
            max_tokens: 131072,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "accounts/fireworks/routers/kimi-k2p6-fast".into(),
        Model {
            id: "accounts/fireworks/routers/kimi-k2p6-fast".into(),
            name: "Kimi K2.6 Fast".into(),
            api: Api::AnthropicMessages,
            provider: "fireworks".into(),
            base_url: "https://api.fireworks.ai/inference".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 2.0,
                output: 8.0,
                cache_read: 0.3,
                cache_write: 0.0,
            },
            context_window: 262000,
            max_tokens: 262000,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "accounts/fireworks/routers/kimi-k2p6-turbo".into(),
        Model {
            id: "accounts/fireworks/routers/kimi-k2p6-turbo".into(),
            name: "Kimi K2.6 Turbo".into(),
            api: Api::AnthropicMessages,
            provider: "fireworks".into(),
            base_url: "https://api.fireworks.ai/inference".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 2.0,
                output: 8.0,
                cache_read: 0.3,
                cache_write: 0.0,
            },
            context_window: 262000,
            max_tokens: 262000,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "accounts/fireworks/routers/kimi-k2p7-code-fast".into(),
        Model {
            id: "accounts/fireworks/routers/kimi-k2p7-code-fast".into(),
            name: "Kimi K2.7 Code Fast".into(),
            api: Api::AnthropicMessages,
            provider: "fireworks".into(),
            base_url: "https://api.fireworks.ai/inference".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 1.9,
                output: 8.0,
                cache_read: 0.38,
                cache_write: 0.0,
            },
            context_window: 262000,
            max_tokens: 262000,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map
}
