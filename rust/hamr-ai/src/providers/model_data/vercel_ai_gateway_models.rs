//! Auto-generated model catalogue for `vercel-ai-gateway`.
//! Do not edit manually — re-run `scripts/generate_models.py`.

use std::collections::HashMap;

use crate::types::{Api, InputModality, Model, ModelCost, ModelThinkingLevel};

/// Model catalogue for the `vercel-ai-gateway` provider.
pub fn models() -> HashMap<String, Model> {
    let mut map: HashMap<String, Model> = HashMap::new();

    map.insert(
        "alibaba/qwen-3-14b".into(),
        Model {
            id: "alibaba/qwen-3-14b".into(),
            name: "Qwen3-14B".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.12,
                output: 0.24,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 40960,
            max_tokens: 16384,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "alibaba/qwen-3-235b".into(),
        Model {
            id: "alibaba/qwen-3-235b".into(),
            name: "Qwen3 235B A22B".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.22,
                output: 0.88,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 262144,
            max_tokens: 16384,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "alibaba/qwen-3-30b".into(),
        Model {
            id: "alibaba/qwen-3-30b".into(),
            name: "Qwen3-30B-A3B".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.12,
                output: 0.5,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 40960,
            max_tokens: 16384,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "alibaba/qwen-3-32b".into(),
        Model {
            id: "alibaba/qwen-3-32b".into(),
            name: "Qwen 3 32B".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.16,
                output: 0.64,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 128000,
            max_tokens: 8192,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "alibaba/qwen-3.6-max-preview".into(),
        Model {
            id: "alibaba/qwen-3.6-max-preview".into(),
            name: "Qwen 3.6 Max Preview".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 1.3,
                output: 7.8,
                cache_read: 0.26,
                cache_write: 1.625,
            },
            context_window: 240000,
            max_tokens: 64000,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "alibaba/qwen3-235b-a22b-thinking".into(),
        Model {
            id: "alibaba/qwen3-235b-a22b-thinking".into(),
            name: "Qwen3 VL 235B A22B Thinking".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.4,
                output: 4.0,
                cache_read: 0.0,
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
        "alibaba/qwen3-coder".into(),
        Model {
            id: "alibaba/qwen3-coder".into(),
            name: "Qwen3 Coder 480B A35B Instruct".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: false,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 1.5,
                output: 7.5,
                cache_read: 0.3,
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
        "alibaba/qwen3-coder-30b-a3b".into(),
        Model {
            id: "alibaba/qwen3-coder-30b-a3b".into(),
            name: "Qwen 3 Coder 30B A3B Instruct".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: false,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.15,
                output: 0.6,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 262144,
            max_tokens: 8192,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "alibaba/qwen3-coder-next".into(),
        Model {
            id: "alibaba/qwen3-coder-next".into(),
            name: "Qwen3 Coder Next".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: false,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.5,
                output: 1.2,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 256000,
            max_tokens: 256000,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "alibaba/qwen3-coder-plus".into(),
        Model {
            id: "alibaba/qwen3-coder-plus".into(),
            name: "Qwen3 Coder Plus".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: false,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 1.0,
                output: 5.0,
                cache_read: 0.2,
                cache_write: 0.0,
            },
            context_window: 1000000,
            max_tokens: 65536,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "alibaba/qwen3-max".into(),
        Model {
            id: "alibaba/qwen3-max".into(),
            name: "Qwen3 Max".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: false,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 1.2,
                output: 6.0,
                cache_read: 0.24,
                cache_write: 0.0,
            },
            context_window: 262144,
            max_tokens: 32768,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "alibaba/qwen3-max-preview".into(),
        Model {
            id: "alibaba/qwen3-max-preview".into(),
            name: "Qwen3 Max Preview".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: false,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 1.2,
                output: 6.0,
                cache_read: 0.24,
                cache_write: 0.0,
            },
            context_window: 262144,
            max_tokens: 32768,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "alibaba/qwen3-max-thinking".into(),
        Model {
            id: "alibaba/qwen3-max-thinking".into(),
            name: "Qwen 3 Max Thinking".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 1.2,
                output: 6.0,
                cache_read: 0.24,
                cache_write: 0.0,
            },
            context_window: 256000,
            max_tokens: 65536,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "alibaba/qwen3-next-80b-a3b-instruct".into(),
        Model {
            id: "alibaba/qwen3-next-80b-a3b-instruct".into(),
            name: "Qwen3 Next 80B A3B Instruct".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: false,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.15,
                output: 1.2,
                cache_read: 0.0,
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
        "alibaba/qwen3-next-80b-a3b-thinking".into(),
        Model {
            id: "alibaba/qwen3-next-80b-a3b-thinking".into(),
            name: "Qwen3 Next 80B A3B Thinking".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.15,
                output: 1.2,
                cache_read: 0.0,
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
        "alibaba/qwen3-vl-235b-a22b-instruct".into(),
        Model {
            id: "alibaba/qwen3-vl-235b-a22b-instruct".into(),
            name: "Qwen3 VL 235B A22B Instruct".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: false,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.4,
                output: 1.6,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 131072,
            max_tokens: 129024,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "alibaba/qwen3-vl-instruct".into(),
        Model {
            id: "alibaba/qwen3-vl-instruct".into(),
            name: "Qwen3 VL 235B A22B Instruct".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: false,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.4,
                output: 1.6,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 131072,
            max_tokens: 129024,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "alibaba/qwen3-vl-thinking".into(),
        Model {
            id: "alibaba/qwen3-vl-thinking".into(),
            name: "Qwen3 VL 235B A22B Thinking".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.4,
                output: 4.0,
                cache_read: 0.0,
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
        "alibaba/qwen3.5-flash".into(),
        Model {
            id: "alibaba/qwen3.5-flash".into(),
            name: "Qwen 3.5 Flash".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.1,
                output: 0.4,
                cache_read: 0.001,
                cache_write: 0.125,
            },
            context_window: 1000000,
            max_tokens: 64000,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "alibaba/qwen3.5-plus".into(),
        Model {
            id: "alibaba/qwen3.5-plus".into(),
            name: "Qwen 3.5 Plus".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.4,
                output: 2.4,
                cache_read: 0.04,
                cache_write: 0.5,
            },
            context_window: 1000000,
            max_tokens: 64000,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "alibaba/qwen3.6-27b".into(),
        Model {
            id: "alibaba/qwen3.6-27b".into(),
            name: "Qwen 3.6 27B".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.6,
                output: 3.6,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 256000,
            max_tokens: 256000,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "alibaba/qwen3.6-plus".into(),
        Model {
            id: "alibaba/qwen3.6-plus".into(),
            name: "Qwen 3.6 Plus".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.5,
                output: 3.0,
                cache_read: 0.1,
                cache_write: 0.625,
            },
            context_window: 1000000,
            max_tokens: 64000,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "alibaba/qwen3.7-max".into(),
        Model {
            id: "alibaba/qwen3.7-max".into(),
            name: "Qwen 3.7 Max".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 1.25,
                output: 3.75,
                cache_read: 0.25,
                cache_write: 1.5625,
            },
            context_window: 991000,
            max_tokens: 64000,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "alibaba/qwen3.7-plus".into(),
        Model {
            id: "alibaba/qwen3.7-plus".into(),
            name: "Qwen 3.7 Plus".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.4,
                output: 1.6,
                cache_read: 0.08,
                cache_write: 0.5,
            },
            context_window: 1000000,
            max_tokens: 64000,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "amazon/nova-2-lite".into(),
        Model {
            id: "amazon/nova-2-lite".into(),
            name: "Nova 2 Lite".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.3,
                output: 2.5,
                cache_read: 0.075,
                cache_write: 0.0,
            },
            context_window: 1000000,
            max_tokens: 1000000,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "amazon/nova-lite".into(),
        Model {
            id: "amazon/nova-lite".into(),
            name: "Nova Lite".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: false,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.06,
                output: 0.24,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 300000,
            max_tokens: 8192,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "amazon/nova-micro".into(),
        Model {
            id: "amazon/nova-micro".into(),
            name: "Nova Micro".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: false,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.035,
                output: 0.14,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 128000,
            max_tokens: 8192,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "amazon/nova-pro".into(),
        Model {
            id: "amazon/nova-pro".into(),
            name: "Nova Pro".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: false,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.8,
                output: 3.2,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 300000,
            max_tokens: 8192,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "anthropic/claude-3-haiku".into(),
        Model {
            id: "anthropic/claude-3-haiku".into(),
            name: "Claude 3 Haiku".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: false,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.25,
                output: 1.25,
                cache_read: 0.03,
                cache_write: 0.3,
            },
            context_window: 200000,
            max_tokens: 4096,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "anthropic/claude-3.5-haiku".into(),
        Model {
            id: "anthropic/claude-3.5-haiku".into(),
            name: "Claude 3.5 Haiku".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: false,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.8,
                output: 4.0,
                cache_read: 0.08,
                cache_write: 1.0,
            },
            context_window: 200000,
            max_tokens: 8192,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "anthropic/claude-haiku-4.5".into(),
        Model {
            id: "anthropic/claude-haiku-4.5".into(),
            name: "Claude Haiku 4.5".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 1.0,
                output: 5.0,
                cache_read: 0.1,
                cache_write: 1.25,
            },
            context_window: 200000,
            max_tokens: 64000,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "anthropic/claude-opus-4".into(),
        Model {
            id: "anthropic/claude-opus-4".into(),
            name: "Claude Opus 4".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 15.0,
                output: 75.0,
                cache_read: 1.5,
                cache_write: 18.75,
            },
            context_window: 200000,
            max_tokens: 32000,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "anthropic/claude-opus-4.1".into(),
        Model {
            id: "anthropic/claude-opus-4.1".into(),
            name: "Claude Opus 4.1".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 15.0,
                output: 75.0,
                cache_read: 1.5,
                cache_write: 18.75,
            },
            context_window: 200000,
            max_tokens: 32000,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "anthropic/claude-opus-4.5".into(),
        Model {
            id: "anthropic/claude-opus-4.5".into(),
            name: "Claude Opus 4.5".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 5.0,
                output: 25.0,
                cache_read: 0.5,
                cache_write: 6.25,
            },
            context_window: 200000,
            max_tokens: 64000,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "anthropic/claude-opus-4.6".into(),
        Model {
            id: "anthropic/claude-opus-4.6".into(),
            name: "Claude Opus 4.6".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 5.0,
                output: 25.0,
                cache_read: 0.5,
                cache_write: 6.25,
            },
            context_window: 1000000,
            max_tokens: 128000,
            thinking_level_map: Some(HashMap::from([(
                ModelThinkingLevel::XHigh,
                Some("max".into()),
            )])),
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "anthropic/claude-opus-4.7".into(),
        Model {
            id: "anthropic/claude-opus-4.7".into(),
            name: "Claude Opus 4.7".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 5.0,
                output: 25.0,
                cache_read: 0.5,
                cache_write: 6.25,
            },
            context_window: 1000000,
            max_tokens: 128000,
            thinking_level_map: Some(HashMap::from([(
                ModelThinkingLevel::XHigh,
                Some("xhigh".into()),
            )])),
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "anthropic/claude-opus-4.8".into(),
        Model {
            id: "anthropic/claude-opus-4.8".into(),
            name: "Claude Opus 4.8".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 5.0,
                output: 25.0,
                cache_read: 0.5,
                cache_write: 6.25,
            },
            context_window: 1000000,
            max_tokens: 128000,
            thinking_level_map: Some(HashMap::from([(
                ModelThinkingLevel::XHigh,
                Some("xhigh".into()),
            )])),
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "anthropic/claude-sonnet-4".into(),
        Model {
            id: "anthropic/claude-sonnet-4".into(),
            name: "Claude Sonnet 4".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 3.0,
                output: 15.0,
                cache_read: 0.3,
                cache_write: 3.75,
            },
            context_window: 1000000,
            max_tokens: 64000,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "anthropic/claude-sonnet-4.5".into(),
        Model {
            id: "anthropic/claude-sonnet-4.5".into(),
            name: "Claude Sonnet 4.5".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 3.0,
                output: 15.0,
                cache_read: 0.3,
                cache_write: 3.75,
            },
            context_window: 1000000,
            max_tokens: 64000,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "anthropic/claude-sonnet-4.6".into(),
        Model {
            id: "anthropic/claude-sonnet-4.6".into(),
            name: "Claude Sonnet 4.6".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 3.0,
                output: 15.0,
                cache_read: 0.3,
                cache_write: 3.75,
            },
            context_window: 1000000,
            max_tokens: 128000,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "arcee-ai/trinity-large-preview".into(),
        Model {
            id: "arcee-ai/trinity-large-preview".into(),
            name: "Trinity Large Preview".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: false,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.25,
                output: 1.0,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 131000,
            max_tokens: 131000,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "arcee-ai/trinity-large-thinking".into(),
        Model {
            id: "arcee-ai/trinity-large-thinking".into(),
            name: "Trinity Large Thinking".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.25,
                output: 0.9,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 262100,
            max_tokens: 80000,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "arcee-ai/trinity-mini".into(),
        Model {
            id: "arcee-ai/trinity-mini".into(),
            name: "Trinity Mini".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: false,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.045,
                output: 0.15,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 131072,
            max_tokens: 131072,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "bytedance/seed-1.6".into(),
        Model {
            id: "bytedance/seed-1.6".into(),
            name: "Seed 1.6".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.25,
                output: 2.0,
                cache_read: 0.05,
                cache_write: 0.0,
            },
            context_window: 256000,
            max_tokens: 32000,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "bytedance/seed-1.8".into(),
        Model {
            id: "bytedance/seed-1.8".into(),
            name: "Bytedance Seed 1.8".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.25,
                output: 2.0,
                cache_read: 0.05,
                cache_write: 0.0,
            },
            context_window: 256000,
            max_tokens: 64000,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "cohere/command-a".into(),
        Model {
            id: "cohere/command-a".into(),
            name: "Command A".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: false,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 2.5,
                output: 10.0,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 256000,
            max_tokens: 8000,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "deepseek/deepseek-r1".into(),
        Model {
            id: "deepseek/deepseek-r1".into(),
            name: "DeepSeek-R1".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 1.35,
                output: 5.4,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 128000,
            max_tokens: 8192,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "deepseek/deepseek-v3".into(),
        Model {
            id: "deepseek/deepseek-v3".into(),
            name: "DeepSeek V3 0324".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: false,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.27,
                output: 1.12,
                cache_read: 0.135,
                cache_write: 0.0,
            },
            context_window: 163840,
            max_tokens: 163840,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "deepseek/deepseek-v3.1".into(),
        Model {
            id: "deepseek/deepseek-v3.1".into(),
            name: "DeepSeek V3.1".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.56,
                output: 1.68,
                cache_read: 0.28,
                cache_write: 0.0,
            },
            context_window: 163840,
            max_tokens: 8192,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "deepseek/deepseek-v3.1-terminus".into(),
        Model {
            id: "deepseek/deepseek-v3.1-terminus".into(),
            name: "DeepSeek V3.1 Terminus".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.27,
                output: 1.0,
                cache_read: 0.135,
                cache_write: 0.0,
            },
            context_window: 131072,
            max_tokens: 65536,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "deepseek/deepseek-v3.2".into(),
        Model {
            id: "deepseek/deepseek-v3.2".into(),
            name: "DeepSeek V3.2".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: false,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.28,
                output: 0.42,
                cache_read: 0.028,
                cache_write: 0.0,
            },
            context_window: 128000,
            max_tokens: 8000,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "deepseek/deepseek-v3.2-thinking".into(),
        Model {
            id: "deepseek/deepseek-v3.2-thinking".into(),
            name: "DeepSeek V3.2 Thinking".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.62,
                output: 1.85,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 128000,
            max_tokens: 8000,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "deepseek/deepseek-v4-flash".into(),
        Model {
            id: "deepseek/deepseek-v4-flash".into(),
            name: "DeepSeek V4 Flash".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
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
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "deepseek/deepseek-v4-pro".into(),
        Model {
            id: "deepseek/deepseek-v4-pro".into(),
            name: "DeepSeek V4 Pro".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.435,
                output: 0.87,
                cache_read: 0.0036,
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
        "google/gemini-2.5-flash".into(),
        Model {
            id: "google/gemini-2.5-flash".into(),
            name: "Gemini 2.5 Flash".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.3,
                output: 2.5,
                cache_read: 0.03,
                cache_write: 0.0,
            },
            context_window: 1000000,
            max_tokens: 65536,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "google/gemini-2.5-flash-lite".into(),
        Model {
            id: "google/gemini-2.5-flash-lite".into(),
            name: "Gemini 2.5 Flash Lite".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.1,
                output: 0.4,
                cache_read: 0.01,
                cache_write: 0.0,
            },
            context_window: 1048576,
            max_tokens: 65536,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "google/gemini-2.5-pro".into(),
        Model {
            id: "google/gemini-2.5-pro".into(),
            name: "Gemini 2.5 Pro".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 1.25,
                output: 10.0,
                cache_read: 0.125,
                cache_write: 0.0,
            },
            context_window: 1048576,
            max_tokens: 65536,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "google/gemini-3-flash".into(),
        Model {
            id: "google/gemini-3-flash".into(),
            name: "Gemini 3 Flash".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.5,
                output: 3.0,
                cache_read: 0.05,
                cache_write: 0.0,
            },
            context_window: 1000000,
            max_tokens: 65000,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "google/gemini-3-pro-preview".into(),
        Model {
            id: "google/gemini-3-pro-preview".into(),
            name: "Gemini 3 Pro Preview".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 2.0,
                output: 12.0,
                cache_read: 0.2,
                cache_write: 0.0,
            },
            context_window: 1000000,
            max_tokens: 64000,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "google/gemini-3.1-flash-lite".into(),
        Model {
            id: "google/gemini-3.1-flash-lite".into(),
            name: "Gemini 3.1 Flash Lite".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.25,
                output: 1.5,
                cache_read: 0.03,
                cache_write: 0.0,
            },
            context_window: 1000000,
            max_tokens: 65000,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "google/gemini-3.1-flash-lite-preview".into(),
        Model {
            id: "google/gemini-3.1-flash-lite-preview".into(),
            name: "Gemini 3.1 Flash Lite Preview".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.25,
                output: 1.5,
                cache_read: 0.03,
                cache_write: 0.0,
            },
            context_window: 1000000,
            max_tokens: 65000,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "google/gemini-3.1-pro-preview".into(),
        Model {
            id: "google/gemini-3.1-pro-preview".into(),
            name: "Gemini 3.1 Pro Preview".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 2.0,
                output: 12.0,
                cache_read: 0.2,
                cache_write: 0.0,
            },
            context_window: 1000000,
            max_tokens: 64000,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "google/gemini-3.5-flash".into(),
        Model {
            id: "google/gemini-3.5-flash".into(),
            name: "Gemini 3.5 Flash".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 1.5,
                output: 9.0,
                cache_read: 0.15,
                cache_write: 0.0,
            },
            context_window: 1000000,
            max_tokens: 64000,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "google/gemma-4-26b-a4b-it".into(),
        Model {
            id: "google/gemma-4-26b-a4b-it".into(),
            name: "Gemma 4 26B A4B IT".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.15,
                output: 0.6,
                cache_read: 0.015,
                cache_write: 0.0,
            },
            context_window: 262144,
            max_tokens: 131072,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "google/gemma-4-31b-it".into(),
        Model {
            id: "google/gemma-4-31b-it".into(),
            name: "Gemma 4 31B IT".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.14,
                output: 0.4,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 262144,
            max_tokens: 131072,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "inception/mercury-2".into(),
        Model {
            id: "inception/mercury-2".into(),
            name: "Mercury 2".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.25,
                output: 0.75,
                cache_read: 0.025,
                cache_write: 0.0,
            },
            context_window: 128000,
            max_tokens: 128000,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "inception/mercury-coder-small".into(),
        Model {
            id: "inception/mercury-coder-small".into(),
            name: "Mercury Coder Small Beta".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: false,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.25,
                output: 1.0,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 32000,
            max_tokens: 16384,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "interfaze/interfaze-beta".into(),
        Model {
            id: "interfaze/interfaze-beta".into(),
            name: "Interfaze Beta".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 1.5,
                output: 3.5,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 1000000,
            max_tokens: 32000,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "kwaipilot/kat-coder-pro-v1".into(),
        Model {
            id: "kwaipilot/kat-coder-pro-v1".into(),
            name: "KAT-Coder-Pro V1".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: false,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.3,
                output: 1.2,
                cache_read: 0.06,
                cache_write: 0.0,
            },
            context_window: 256000,
            max_tokens: 32000,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "kwaipilot/kat-coder-pro-v2".into(),
        Model {
            id: "kwaipilot/kat-coder-pro-v2".into(),
            name: "Kat Coder Pro V2".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.3,
                output: 1.2,
                cache_read: 0.06,
                cache_write: 0.0,
            },
            context_window: 256000,
            max_tokens: 256000,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "meituan/longcat-flash-chat".into(),
        Model {
            id: "meituan/longcat-flash-chat".into(),
            name: "LongCat Flash Chat".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: false,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.0,
                output: 0.0,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 128000,
            max_tokens: 100000,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "meituan/longcat-flash-thinking-2601".into(),
        Model {
            id: "meituan/longcat-flash-thinking-2601".into(),
            name: "LongCat Flash Thinking 2601".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.0,
                output: 0.0,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 32768,
            max_tokens: 32768,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "meta/llama-3.1-70b".into(),
        Model {
            id: "meta/llama-3.1-70b".into(),
            name: "Llama 3.1 70B Instruct".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: false,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.72,
                output: 0.72,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 128000,
            max_tokens: 8192,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "meta/llama-3.1-8b".into(),
        Model {
            id: "meta/llama-3.1-8b".into(),
            name: "Llama 3.1 8B Instruct".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: false,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.22,
                output: 0.22,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 128000,
            max_tokens: 8192,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "meta/llama-3.2-11b".into(),
        Model {
            id: "meta/llama-3.2-11b".into(),
            name: "Llama 3.2 11B Vision Instruct".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: false,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.16,
                output: 0.16,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 128000,
            max_tokens: 8192,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "meta/llama-3.2-90b".into(),
        Model {
            id: "meta/llama-3.2-90b".into(),
            name: "Llama 3.2 90B Vision Instruct".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: false,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.72,
                output: 0.72,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 128000,
            max_tokens: 8192,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "meta/llama-3.3-70b".into(),
        Model {
            id: "meta/llama-3.3-70b".into(),
            name: "Llama 3.3 70B Instruct".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: false,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.72,
                output: 0.72,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 128000,
            max_tokens: 8192,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "meta/llama-4-maverick".into(),
        Model {
            id: "meta/llama-4-maverick".into(),
            name: "Llama 4 Maverick 17B Instruct".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: false,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.24,
                output: 0.97,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 128000,
            max_tokens: 8192,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "meta/llama-4-scout".into(),
        Model {
            id: "meta/llama-4-scout".into(),
            name: "Llama 4 Scout 17B Instruct".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: false,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.17,
                output: 0.66,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 128000,
            max_tokens: 8192,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "minimax/minimax-m2".into(),
        Model {
            id: "minimax/minimax-m2".into(),
            name: "MiniMax M2".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.3,
                output: 1.2,
                cache_read: 0.03,
                cache_write: 0.375,
            },
            context_window: 205000,
            max_tokens: 205000,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "minimax/minimax-m2.1".into(),
        Model {
            id: "minimax/minimax-m2.1".into(),
            name: "MiniMax M2.1".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.3,
                output: 1.2,
                cache_read: 0.03,
                cache_write: 0.375,
            },
            context_window: 204800,
            max_tokens: 131072,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "minimax/minimax-m2.1-lightning".into(),
        Model {
            id: "minimax/minimax-m2.1-lightning".into(),
            name: "MiniMax M2.1 Lightning".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.3,
                output: 2.4,
                cache_read: 0.03,
                cache_write: 0.375,
            },
            context_window: 204800,
            max_tokens: 131072,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "minimax/minimax-m2.5".into(),
        Model {
            id: "minimax/minimax-m2.5".into(),
            name: "MiniMax M2.5".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.3,
                output: 1.2,
                cache_read: 0.03,
                cache_write: 0.375,
            },
            context_window: 204800,
            max_tokens: 131000,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "minimax/minimax-m2.5-highspeed".into(),
        Model {
            id: "minimax/minimax-m2.5-highspeed".into(),
            name: "MiniMax M2.5 High Speed".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.6,
                output: 2.4,
                cache_read: 0.03,
                cache_write: 0.375,
            },
            context_window: 204800,
            max_tokens: 131000,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "minimax/minimax-m2.7".into(),
        Model {
            id: "minimax/minimax-m2.7".into(),
            name: "MiniMax M2.7".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.3,
                output: 1.2,
                cache_read: 0.06,
                cache_write: 0.375,
            },
            context_window: 204800,
            max_tokens: 131000,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "minimax/minimax-m2.7-highspeed".into(),
        Model {
            id: "minimax/minimax-m2.7-highspeed".into(),
            name: "MiniMax M2.7 High Speed".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.6,
                output: 2.4,
                cache_read: 0.06,
                cache_write: 0.375,
            },
            context_window: 204800,
            max_tokens: 131100,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "minimax/minimax-m3".into(),
        Model {
            id: "minimax/minimax-m3".into(),
            name: "MiniMax M3".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.3,
                output: 1.2,
                cache_read: 0.06,
                cache_write: 0.0,
            },
            context_window: 1000000,
            max_tokens: 1000000,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "mistral/codestral".into(),
        Model {
            id: "mistral/codestral".into(),
            name: "Mistral Codestral".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: false,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.3,
                output: 0.9,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 128000,
            max_tokens: 4000,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "mistral/devstral-2".into(),
        Model {
            id: "mistral/devstral-2".into(),
            name: "Devstral 2".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: false,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.4,
                output: 2.0,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 256000,
            max_tokens: 256000,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "mistral/devstral-small".into(),
        Model {
            id: "mistral/devstral-small".into(),
            name: "Devstral Small 1.1".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: false,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.1,
                output: 0.3,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 128000,
            max_tokens: 64000,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "mistral/devstral-small-2".into(),
        Model {
            id: "mistral/devstral-small-2".into(),
            name: "Devstral Small 2".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: false,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.1,
                output: 0.3,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 256000,
            max_tokens: 256000,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "mistral/magistral-medium".into(),
        Model {
            id: "mistral/magistral-medium".into(),
            name: "Magistral Medium 2509".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 2.0,
                output: 5.0,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 128000,
            max_tokens: 64000,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "mistral/magistral-small".into(),
        Model {
            id: "mistral/magistral-small".into(),
            name: "Magistral Small 2509".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.5,
                output: 1.5,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 128000,
            max_tokens: 64000,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "mistral/ministral-14b".into(),
        Model {
            id: "mistral/ministral-14b".into(),
            name: "Ministral 14B".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: false,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.2,
                output: 0.2,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 256000,
            max_tokens: 256000,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "mistral/ministral-3b".into(),
        Model {
            id: "mistral/ministral-3b".into(),
            name: "Ministral 3B".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: false,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.1,
                output: 0.1,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 128000,
            max_tokens: 4000,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "mistral/ministral-8b".into(),
        Model {
            id: "mistral/ministral-8b".into(),
            name: "Ministral 8B".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: false,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.15,
                output: 0.15,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 128000,
            max_tokens: 4000,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "mistral/mistral-large-3".into(),
        Model {
            id: "mistral/mistral-large-3".into(),
            name: "Mistral Large 3".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: false,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.5,
                output: 1.5,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 256000,
            max_tokens: 256000,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "mistral/mistral-medium".into(),
        Model {
            id: "mistral/mistral-medium".into(),
            name: "Mistral Medium 3.1".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: false,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.4,
                output: 2.0,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 128000,
            max_tokens: 64000,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "mistral/mistral-medium-3.5".into(),
        Model {
            id: "mistral/mistral-medium-3.5".into(),
            name: "Mistral Medium Latest".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 1.5,
                output: 7.5,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 256000,
            max_tokens: 256000,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "mistral/mistral-nemo".into(),
        Model {
            id: "mistral/mistral-nemo".into(),
            name: "Mistral Nemo 12B".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: false,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.15,
                output: 0.15,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 128000,
            max_tokens: 128000,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "mistral/mistral-small".into(),
        Model {
            id: "mistral/mistral-small".into(),
            name: "Mistral Small".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: false,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.1,
                output: 0.3,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 32000,
            max_tokens: 4000,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "mistral/pixtral-12b".into(),
        Model {
            id: "mistral/pixtral-12b".into(),
            name: "Pixtral 12B 2409".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: false,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.15,
                output: 0.15,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 128000,
            max_tokens: 4000,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "mistral/pixtral-large".into(),
        Model {
            id: "mistral/pixtral-large".into(),
            name: "Pixtral Large".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: false,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 2.0,
                output: 6.0,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 128000,
            max_tokens: 4000,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "moonshotai/kimi-k2".into(),
        Model {
            id: "moonshotai/kimi-k2".into(),
            name: "Kimi K2 Instruct".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: false,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.57,
                output: 2.3,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 131072,
            max_tokens: 131072,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "moonshotai/kimi-k2-thinking".into(),
        Model {
            id: "moonshotai/kimi-k2-thinking".into(),
            name: "Kimi K2 Thinking".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.6,
                output: 2.5,
                cache_read: 0.15,
                cache_write: 0.0,
            },
            context_window: 262114,
            max_tokens: 262114,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "moonshotai/kimi-k2.5".into(),
        Model {
            id: "moonshotai/kimi-k2.5".into(),
            name: "Kimi K2.5".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.6,
                output: 3.0,
                cache_read: 0.1,
                cache_write: 0.0,
            },
            context_window: 262114,
            max_tokens: 262114,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "moonshotai/kimi-k2.6".into(),
        Model {
            id: "moonshotai/kimi-k2.6".into(),
            name: "Kimi K2.6".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
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
        "moonshotai/kimi-k2.7-code".into(),
        Model {
            id: "moonshotai/kimi-k2.7-code".into(),
            name: "Kimi K2.7 Code".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.95,
                output: 4.0,
                cache_read: 0.19,
                cache_write: 0.0,
            },
            context_window: 256000,
            max_tokens: 32768,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "moonshotai/kimi-k2.7-code-highspeed".into(),
        Model {
            id: "moonshotai/kimi-k2.7-code-highspeed".into(),
            name: "Kimi K2.7 Code High Speed".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 1.9,
                output: 8.0,
                cache_read: 0.38,
                cache_write: 0.0,
            },
            context_window: 262144,
            max_tokens: 32768,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "nvidia/nemotron-3-nano-30b-a3b".into(),
        Model {
            id: "nvidia/nemotron-3-nano-30b-a3b".into(),
            name: "Nemotron 3 Nano 30B A3B".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.05,
                output: 0.24,
                cache_read: 0.0,
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
        "nvidia/nemotron-3-super-120b-a12b".into(),
        Model {
            id: "nvidia/nemotron-3-super-120b-a12b".into(),
            name: "NVIDIA Nemotron 3 Super 120B A12B".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.15,
                output: 0.65,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 256000,
            max_tokens: 32000,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "nvidia/nemotron-3-ultra-550b-a55b".into(),
        Model {
            id: "nvidia/nemotron-3-ultra-550b-a55b".into(),
            name: "Nemotron 3 Ultra".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.6,
                output: 2.4,
                cache_read: 0.12,
                cache_write: 0.0,
            },
            context_window: 1000000,
            max_tokens: 65000,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "nvidia/nemotron-nano-12b-v2-vl".into(),
        Model {
            id: "nvidia/nemotron-nano-12b-v2-vl".into(),
            name: "Nvidia Nemotron Nano 12B V2 VL".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.2,
                output: 0.6,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 131072,
            max_tokens: 131072,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "nvidia/nemotron-nano-9b-v2".into(),
        Model {
            id: "nvidia/nemotron-nano-9b-v2".into(),
            name: "Nvidia Nemotron Nano 9B V2".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.06,
                output: 0.23,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 131072,
            max_tokens: 131072,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "openai/gpt-3.5-turbo".into(),
        Model {
            id: "openai/gpt-3.5-turbo".into(),
            name: "GPT-3.5 Turbo".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: false,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.5,
                output: 1.5,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 16385,
            max_tokens: 4096,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "openai/gpt-4-turbo".into(),
        Model {
            id: "openai/gpt-4-turbo".into(),
            name: "GPT-4 Turbo".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: false,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 10.0,
                output: 30.0,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 128000,
            max_tokens: 4096,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "openai/gpt-4.1".into(),
        Model {
            id: "openai/gpt-4.1".into(),
            name: "GPT-4.1".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: false,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 2.0,
                output: 8.0,
                cache_read: 0.5,
                cache_write: 0.0,
            },
            context_window: 1047576,
            max_tokens: 32768,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "openai/gpt-4.1-mini".into(),
        Model {
            id: "openai/gpt-4.1-mini".into(),
            name: "GPT-4.1 mini".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: false,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.4,
                output: 1.6,
                cache_read: 0.1,
                cache_write: 0.0,
            },
            context_window: 1047576,
            max_tokens: 32768,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "openai/gpt-4.1-nano".into(),
        Model {
            id: "openai/gpt-4.1-nano".into(),
            name: "GPT-4.1 nano".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: false,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.1,
                output: 0.4,
                cache_read: 0.025,
                cache_write: 0.0,
            },
            context_window: 1047576,
            max_tokens: 32768,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "openai/gpt-4o".into(),
        Model {
            id: "openai/gpt-4o".into(),
            name: "GPT-4o".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: false,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 2.5,
                output: 10.0,
                cache_read: 1.25,
                cache_write: 0.0,
            },
            context_window: 128000,
            max_tokens: 16384,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "openai/gpt-4o-mini".into(),
        Model {
            id: "openai/gpt-4o-mini".into(),
            name: "GPT-4o mini".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: false,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.15,
                output: 0.6,
                cache_read: 0.075,
                cache_write: 0.0,
            },
            context_window: 128000,
            max_tokens: 16384,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "openai/gpt-5".into(),
        Model {
            id: "openai/gpt-5".into(),
            name: "GPT-5".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 1.25,
                output: 10.0,
                cache_read: 0.125,
                cache_write: 0.0,
            },
            context_window: 400000,
            max_tokens: 128000,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "openai/gpt-5-chat".into(),
        Model {
            id: "openai/gpt-5-chat".into(),
            name: "GPT 5 Chat".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: false,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 1.25,
                output: 10.0,
                cache_read: 0.125,
                cache_write: 0.0,
            },
            context_window: 128000,
            max_tokens: 16384,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "openai/gpt-5-codex".into(),
        Model {
            id: "openai/gpt-5-codex".into(),
            name: "GPT-5-Codex".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 1.25,
                output: 10.0,
                cache_read: 0.125,
                cache_write: 0.0,
            },
            context_window: 400000,
            max_tokens: 128000,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "openai/gpt-5-mini".into(),
        Model {
            id: "openai/gpt-5-mini".into(),
            name: "GPT-5 mini".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.25,
                output: 2.0,
                cache_read: 0.025,
                cache_write: 0.0,
            },
            context_window: 400000,
            max_tokens: 128000,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "openai/gpt-5-nano".into(),
        Model {
            id: "openai/gpt-5-nano".into(),
            name: "GPT-5 nano".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.05,
                output: 0.4,
                cache_read: 0.005,
                cache_write: 0.0,
            },
            context_window: 400000,
            max_tokens: 128000,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "openai/gpt-5-pro".into(),
        Model {
            id: "openai/gpt-5-pro".into(),
            name: "GPT-5 pro".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 15.0,
                output: 120.0,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 400000,
            max_tokens: 272000,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "openai/gpt-5.1-codex".into(),
        Model {
            id: "openai/gpt-5.1-codex".into(),
            name: "GPT-5.1-Codex".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 1.25,
                output: 10.0,
                cache_read: 0.125,
                cache_write: 0.0,
            },
            context_window: 400000,
            max_tokens: 128000,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "openai/gpt-5.1-codex-max".into(),
        Model {
            id: "openai/gpt-5.1-codex-max".into(),
            name: "GPT 5.1 Codex Max".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 1.25,
                output: 10.0,
                cache_read: 0.125,
                cache_write: 0.0,
            },
            context_window: 400000,
            max_tokens: 128000,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "openai/gpt-5.1-codex-mini".into(),
        Model {
            id: "openai/gpt-5.1-codex-mini".into(),
            name: "GPT 5.1 Codex Mini".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.25,
                output: 2.0,
                cache_read: 0.025,
                cache_write: 0.0,
            },
            context_window: 400000,
            max_tokens: 128000,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "openai/gpt-5.1-instant".into(),
        Model {
            id: "openai/gpt-5.1-instant".into(),
            name: "GPT-5.1 Instant".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: false,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 1.25,
                output: 10.0,
                cache_read: 0.125,
                cache_write: 0.0,
            },
            context_window: 128000,
            max_tokens: 16384,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "openai/gpt-5.1-thinking".into(),
        Model {
            id: "openai/gpt-5.1-thinking".into(),
            name: "GPT 5.1 Thinking".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 1.25,
                output: 10.0,
                cache_read: 0.125,
                cache_write: 0.0,
            },
            context_window: 400000,
            max_tokens: 128000,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "openai/gpt-5.2".into(),
        Model {
            id: "openai/gpt-5.2".into(),
            name: "GPT 5.2".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 1.75,
                output: 14.0,
                cache_read: 0.175,
                cache_write: 0.0,
            },
            context_window: 400000,
            max_tokens: 128000,
            thinking_level_map: Some(HashMap::from([(
                ModelThinkingLevel::XHigh,
                Some("xhigh".into()),
            )])),
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "openai/gpt-5.2-chat".into(),
        Model {
            id: "openai/gpt-5.2-chat".into(),
            name: "GPT 5.2 Chat".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: false,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 1.75,
                output: 14.0,
                cache_read: 0.175,
                cache_write: 0.0,
            },
            context_window: 128000,
            max_tokens: 16384,
            thinking_level_map: Some(HashMap::from([(
                ModelThinkingLevel::XHigh,
                Some("xhigh".into()),
            )])),
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "openai/gpt-5.2-codex".into(),
        Model {
            id: "openai/gpt-5.2-codex".into(),
            name: "GPT 5.2 Codex".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 1.75,
                output: 14.0,
                cache_read: 0.175,
                cache_write: 0.0,
            },
            context_window: 400000,
            max_tokens: 128000,
            thinking_level_map: Some(HashMap::from([(
                ModelThinkingLevel::XHigh,
                Some("xhigh".into()),
            )])),
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "openai/gpt-5.2-pro".into(),
        Model {
            id: "openai/gpt-5.2-pro".into(),
            name: "GPT 5.2 ".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 21.0,
                output: 168.0,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 400000,
            max_tokens: 128000,
            thinking_level_map: Some(HashMap::from([(
                ModelThinkingLevel::XHigh,
                Some("xhigh".into()),
            )])),
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "openai/gpt-5.3-chat".into(),
        Model {
            id: "openai/gpt-5.3-chat".into(),
            name: "GPT-5.3 Chat".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: false,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 1.75,
                output: 14.0,
                cache_read: 0.175,
                cache_write: 0.0,
            },
            context_window: 128000,
            max_tokens: 16384,
            thinking_level_map: Some(HashMap::from([(
                ModelThinkingLevel::XHigh,
                Some("xhigh".into()),
            )])),
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "openai/gpt-5.3-codex".into(),
        Model {
            id: "openai/gpt-5.3-codex".into(),
            name: "GPT 5.3 Codex".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 1.75,
                output: 14.0,
                cache_read: 0.175,
                cache_write: 0.0,
            },
            context_window: 400000,
            max_tokens: 128000,
            thinking_level_map: Some(HashMap::from([(
                ModelThinkingLevel::XHigh,
                Some("xhigh".into()),
            )])),
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "openai/gpt-5.4".into(),
        Model {
            id: "openai/gpt-5.4".into(),
            name: "GPT 5.4".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 2.5,
                output: 15.0,
                cache_read: 0.25,
                cache_write: 0.0,
            },
            context_window: 1050000,
            max_tokens: 128000,
            thinking_level_map: Some(HashMap::from([(
                ModelThinkingLevel::XHigh,
                Some("xhigh".into()),
            )])),
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "openai/gpt-5.4-mini".into(),
        Model {
            id: "openai/gpt-5.4-mini".into(),
            name: "GPT 5.4 Mini".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.75,
                output: 4.5,
                cache_read: 0.075,
                cache_write: 0.0,
            },
            context_window: 400000,
            max_tokens: 128000,
            thinking_level_map: Some(HashMap::from([(
                ModelThinkingLevel::XHigh,
                Some("xhigh".into()),
            )])),
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "openai/gpt-5.4-nano".into(),
        Model {
            id: "openai/gpt-5.4-nano".into(),
            name: "GPT 5.4 Nano".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.2,
                output: 1.25,
                cache_read: 0.02,
                cache_write: 0.0,
            },
            context_window: 400000,
            max_tokens: 128000,
            thinking_level_map: Some(HashMap::from([(
                ModelThinkingLevel::XHigh,
                Some("xhigh".into()),
            )])),
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "openai/gpt-5.4-pro".into(),
        Model {
            id: "openai/gpt-5.4-pro".into(),
            name: "GPT 5.4 Pro".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 30.0,
                output: 180.0,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 1050000,
            max_tokens: 128000,
            thinking_level_map: Some(HashMap::from([(
                ModelThinkingLevel::XHigh,
                Some("xhigh".into()),
            )])),
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "openai/gpt-5.5".into(),
        Model {
            id: "openai/gpt-5.5".into(),
            name: "GPT 5.5".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 5.0,
                output: 30.0,
                cache_read: 0.5,
                cache_write: 0.0,
            },
            context_window: 1000000,
            max_tokens: 128000,
            thinking_level_map: Some(HashMap::from([(
                ModelThinkingLevel::XHigh,
                Some("xhigh".into()),
            )])),
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "openai/gpt-5.5-pro".into(),
        Model {
            id: "openai/gpt-5.5-pro".into(),
            name: "GPT 5.5 Pro".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 30.0,
                output: 180.0,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 1000000,
            max_tokens: 128000,
            thinking_level_map: Some(HashMap::from([
                (ModelThinkingLevel::XHigh, Some("xhigh".into())),
                (ModelThinkingLevel::Off, None),
                (ModelThinkingLevel::Minimal, None),
                (ModelThinkingLevel::Low, None),
            ])),
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "openai/gpt-oss-120b".into(),
        Model {
            id: "openai/gpt-oss-120b".into(),
            name: "GPT OSS 120B".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.1,
                output: 0.5,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 131072,
            max_tokens: 131072,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "openai/gpt-oss-20b".into(),
        Model {
            id: "openai/gpt-oss-20b".into(),
            name: "GPT OSS 20B".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.05,
                output: 0.2,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 131072,
            max_tokens: 8192,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "openai/gpt-oss-safeguard-20b".into(),
        Model {
            id: "openai/gpt-oss-safeguard-20b".into(),
            name: "GPT OSS Safeguard 20B".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.075,
                output: 0.3,
                cache_read: 0.037,
                cache_write: 0.0,
            },
            context_window: 131072,
            max_tokens: 65536,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "openai/o1".into(),
        Model {
            id: "openai/o1".into(),
            name: "o1".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 15.0,
                output: 60.0,
                cache_read: 7.5,
                cache_write: 0.0,
            },
            context_window: 200000,
            max_tokens: 100000,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "openai/o3".into(),
        Model {
            id: "openai/o3".into(),
            name: "o3".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 2.0,
                output: 8.0,
                cache_read: 0.5,
                cache_write: 0.0,
            },
            context_window: 200000,
            max_tokens: 100000,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "openai/o3-deep-research".into(),
        Model {
            id: "openai/o3-deep-research".into(),
            name: "o3-deep-research".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 10.0,
                output: 40.0,
                cache_read: 2.5,
                cache_write: 0.0,
            },
            context_window: 200000,
            max_tokens: 100000,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "openai/o3-mini".into(),
        Model {
            id: "openai/o3-mini".into(),
            name: "o3-mini".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 1.1,
                output: 4.4,
                cache_read: 0.55,
                cache_write: 0.0,
            },
            context_window: 200000,
            max_tokens: 100000,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "openai/o3-pro".into(),
        Model {
            id: "openai/o3-pro".into(),
            name: "o3 Pro".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 20.0,
                output: 80.0,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 200000,
            max_tokens: 100000,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "openai/o4-mini".into(),
        Model {
            id: "openai/o4-mini".into(),
            name: "o4-mini".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 1.1,
                output: 4.4,
                cache_read: 0.275,
                cache_write: 0.0,
            },
            context_window: 200000,
            max_tokens: 100000,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "sakana/fugu-ultra".into(),
        Model {
            id: "sakana/fugu-ultra".into(),
            name: "Fugu Ultra".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 5.0,
                output: 30.0,
                cache_read: 0.5,
                cache_write: 0.0,
            },
            context_window: 1000000,
            max_tokens: 1000000,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "stepfun/step-3.5-flash".into(),
        Model {
            id: "stepfun/step-3.5-flash".into(),
            name: "StepFun 3.5 Flash".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.09,
                output: 0.3,
                cache_read: 0.02,
                cache_write: 0.0,
            },
            context_window: 262114,
            max_tokens: 262114,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "stepfun/step-3.7-flash".into(),
        Model {
            id: "stepfun/step-3.7-flash".into(),
            name: "Step 3.7 Flash".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.2,
                output: 1.15,
                cache_read: 0.04,
                cache_write: 0.0,
            },
            context_window: 256000,
            max_tokens: 256000,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "xai/grok-4.1-fast-non-reasoning".into(),
        Model {
            id: "xai/grok-4.1-fast-non-reasoning".into(),
            name: "Grok 4.1 Fast Non-Reasoning".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: false,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.2,
                output: 0.5,
                cache_read: 0.05,
                cache_write: 0.0,
            },
            context_window: 1000000,
            max_tokens: 1000000,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "xai/grok-4.1-fast-reasoning".into(),
        Model {
            id: "xai/grok-4.1-fast-reasoning".into(),
            name: "Grok 4.1 Fast Reasoning".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.2,
                output: 0.5,
                cache_read: 0.05,
                cache_write: 0.0,
            },
            context_window: 1000000,
            max_tokens: 1000000,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "xai/grok-4.20-multi-agent".into(),
        Model {
            id: "xai/grok-4.20-multi-agent".into(),
            name: "Grok 4.20 Multi-Agent".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 1.25,
                output: 2.5,
                cache_read: 0.2,
                cache_write: 0.0,
            },
            context_window: 2000000,
            max_tokens: 2000000,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "xai/grok-4.20-multi-agent-beta".into(),
        Model {
            id: "xai/grok-4.20-multi-agent-beta".into(),
            name: "Grok 4.20 Multi Agent Beta".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 1.25,
                output: 2.5,
                cache_read: 0.2,
                cache_write: 0.0,
            },
            context_window: 2000000,
            max_tokens: 2000000,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "xai/grok-4.20-non-reasoning".into(),
        Model {
            id: "xai/grok-4.20-non-reasoning".into(),
            name: "Grok 4.20 Non-Reasoning".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: false,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 1.25,
                output: 2.5,
                cache_read: 0.2,
                cache_write: 0.0,
            },
            context_window: 2000000,
            max_tokens: 2000000,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "xai/grok-4.20-non-reasoning-beta".into(),
        Model {
            id: "xai/grok-4.20-non-reasoning-beta".into(),
            name: "Grok 4.20 Beta Non-Reasoning".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: false,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 1.25,
                output: 2.5,
                cache_read: 0.2,
                cache_write: 0.0,
            },
            context_window: 2000000,
            max_tokens: 2000000,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "xai/grok-4.20-reasoning".into(),
        Model {
            id: "xai/grok-4.20-reasoning".into(),
            name: "Grok 4.20 Reasoning".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 1.25,
                output: 2.5,
                cache_read: 0.2,
                cache_write: 0.0,
            },
            context_window: 2000000,
            max_tokens: 2000000,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "xai/grok-4.20-reasoning-beta".into(),
        Model {
            id: "xai/grok-4.20-reasoning-beta".into(),
            name: "Grok 4.20 Beta Reasoning".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 1.25,
                output: 2.5,
                cache_read: 0.2,
                cache_write: 0.0,
            },
            context_window: 2000000,
            max_tokens: 2000000,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "xai/grok-4.3".into(),
        Model {
            id: "xai/grok-4.3".into(),
            name: "Grok 4.3".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 1.25,
                output: 2.5,
                cache_read: 0.2,
                cache_write: 0.0,
            },
            context_window: 1000000,
            max_tokens: 1000000,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "xai/grok-build-0.1".into(),
        Model {
            id: "xai/grok-build-0.1".into(),
            name: "Grok Build 0.1".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 1.0,
                output: 2.0,
                cache_read: 0.2,
                cache_write: 0.0,
            },
            context_window: 256000,
            max_tokens: 256000,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "xiaomi/mimo-v2-flash".into(),
        Model {
            id: "xiaomi/mimo-v2-flash".into(),
            name: "MiMo V2 Flash".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.1,
                output: 0.3,
                cache_read: 0.01,
                cache_write: 0.0,
            },
            context_window: 262144,
            max_tokens: 32000,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "xiaomi/mimo-v2-pro".into(),
        Model {
            id: "xiaomi/mimo-v2-pro".into(),
            name: "MiMo V2 Pro".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 1.0,
                output: 3.0,
                cache_read: 0.2,
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
        "xiaomi/mimo-v2.5".into(),
        Model {
            id: "xiaomi/mimo-v2.5".into(),
            name: "MiMo M2.5".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.14,
                output: 0.28,
                cache_read: 0.0028,
                cache_write: 0.0,
            },
            context_window: 1050000,
            max_tokens: 131100,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "xiaomi/mimo-v2.5-pro".into(),
        Model {
            id: "xiaomi/mimo-v2.5-pro".into(),
            name: "MiMo V2.5 Pro".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.435,
                output: 0.87,
                cache_read: 0.0036,
                cache_write: 0.0,
            },
            context_window: 1050000,
            max_tokens: 131000,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "zai/glm-4.5".into(),
        Model {
            id: "zai/glm-4.5".into(),
            name: "GLM-4.5".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.6,
                output: 2.2,
                cache_read: 0.11,
                cache_write: 0.0,
            },
            context_window: 128000,
            max_tokens: 96000,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "zai/glm-4.5-air".into(),
        Model {
            id: "zai/glm-4.5-air".into(),
            name: "GLM 4.5 Air".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.2,
                output: 1.1,
                cache_read: 0.03,
                cache_write: 0.0,
            },
            context_window: 128000,
            max_tokens: 96000,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "zai/glm-4.5v".into(),
        Model {
            id: "zai/glm-4.5v".into(),
            name: "GLM 4.5V".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.6,
                output: 1.8,
                cache_read: 0.11,
                cache_write: 0.0,
            },
            context_window: 66000,
            max_tokens: 16000,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "zai/glm-4.6".into(),
        Model {
            id: "zai/glm-4.6".into(),
            name: "GLM 4.6".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.6,
                output: 2.2,
                cache_read: 0.11,
                cache_write: 0.0,
            },
            context_window: 200000,
            max_tokens: 96000,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "zai/glm-4.6v".into(),
        Model {
            id: "zai/glm-4.6v".into(),
            name: "GLM-4.6V".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.3,
                output: 0.9,
                cache_read: 0.05,
                cache_write: 0.0,
            },
            context_window: 128000,
            max_tokens: 24000,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "zai/glm-4.6v-flash".into(),
        Model {
            id: "zai/glm-4.6v-flash".into(),
            name: "GLM-4.6V-Flash".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.0,
                output: 0.0,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 128000,
            max_tokens: 24000,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "zai/glm-4.7".into(),
        Model {
            id: "zai/glm-4.7".into(),
            name: "GLM 4.7".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.6,
                output: 2.2,
                cache_read: 0.12,
                cache_write: 0.0,
            },
            context_window: 200000,
            max_tokens: 120000,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "zai/glm-4.7-flash".into(),
        Model {
            id: "zai/glm-4.7-flash".into(),
            name: "GLM 4.7 Flash".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.07,
                output: 0.4,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 200000,
            max_tokens: 131000,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "zai/glm-4.7-flashx".into(),
        Model {
            id: "zai/glm-4.7-flashx".into(),
            name: "GLM 4.7 FlashX".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.06,
                output: 0.4,
                cache_read: 0.01,
                cache_write: 0.0,
            },
            context_window: 200000,
            max_tokens: 128000,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "zai/glm-5".into(),
        Model {
            id: "zai/glm-5".into(),
            name: "GLM 5".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.95,
                output: 3.15,
                cache_read: 0.2,
                cache_write: 0.0,
            },
            context_window: 202800,
            max_tokens: 131100,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "zai/glm-5-turbo".into(),
        Model {
            id: "zai/glm-5-turbo".into(),
            name: "GLM 5 Turbo".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 1.2,
                output: 4.0,
                cache_read: 0.24,
                cache_write: 0.0,
            },
            context_window: 202800,
            max_tokens: 131100,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "zai/glm-5.1".into(),
        Model {
            id: "zai/glm-5.1".into(),
            name: "GLM 5.1".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 1.3,
                output: 4.3,
                cache_read: 0.26,
                cache_write: 0.0,
            },
            context_window: 202000,
            max_tokens: 202000,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "zai/glm-5.2".into(),
        Model {
            id: "zai/glm-5.2".into(),
            name: "GLM 5.2".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 1.5,
                output: 4.5,
                cache_read: 0.3,
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
        "zai/glm-5.2-fast".into(),
        Model {
            id: "zai/glm-5.2-fast".into(),
            name: "GLM 5.2 Fast".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 3.0,
                output: 10.25,
                cache_read: 0.5,
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
        "zai/glm-5v-turbo".into(),
        Model {
            id: "zai/glm-5v-turbo".into(),
            name: "GLM 5V Turbo".into(),
            api: Api::AnthropicMessages,
            provider: "vercel-ai-gateway".into(),
            base_url: "https://ai-gateway.vercel.sh".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 1.2,
                output: 4.0,
                cache_read: 0.24,
                cache_write: 0.0,
            },
            context_window: 200000,
            max_tokens: 128000,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map
}
