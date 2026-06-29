//! Auto-generated model catalogue for `openrouter`.
//! Do not edit manually — re-run `scripts/generate_models.py`.

use std::collections::HashMap;

use crate::types::{Api, InputModality, Model, ModelCost, ModelThinkingLevel};

/// Model catalogue for the `openrouter` provider.
pub fn models() -> HashMap<String, Model> {
    let mut map: HashMap<String, Model> = HashMap::new();

    map.insert(
        "ai21/jamba-large-1.7".into(),
        Model {
            id: "ai21/jamba-large-1.7".into(),
            name: "AI21: Jamba Large 1.7".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: false,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 2.0,
                output: 8.0,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 256000,
            max_tokens: 4096,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "amazon/nova-2-lite-v1".into(),
        Model {
            id: "amazon/nova-2-lite-v1".into(),
            name: "Amazon: Nova 2 Lite".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.3,
                output: 2.5,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 1000000,
            max_tokens: 65535,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "amazon/nova-lite-v1".into(),
        Model {
            id: "amazon/nova-lite-v1".into(),
            name: "Amazon: Nova Lite 1.0".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: false,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.06,
                output: 0.24,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 300000,
            max_tokens: 5120,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "amazon/nova-micro-v1".into(),
        Model {
            id: "amazon/nova-micro-v1".into(),
            name: "Amazon: Nova Micro 1.0".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: false,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.035,
                output: 0.14,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 128000,
            max_tokens: 5120,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "amazon/nova-premier-v1".into(),
        Model {
            id: "amazon/nova-premier-v1".into(),
            name: "Amazon: Nova Premier 1.0".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: false,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 2.5,
                output: 12.5,
                cache_read: 0.625,
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
        "amazon/nova-pro-v1".into(),
        Model {
            id: "amazon/nova-pro-v1".into(),
            name: "Amazon: Nova Pro 1.0".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: false,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.8,
                output: 3.2,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 300000,
            max_tokens: 5120,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "anthropic/claude-3-haiku".into(),
        Model {
            id: "anthropic/claude-3-haiku".into(),
            name: "Anthropic: Claude 3 Haiku".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
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
        "anthropic/claude-fable-5".into(),
        Model {
            id: "anthropic/claude-fable-5".into(),
            name: "Anthropic: Claude Fable 5".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 10.0,
                output: 50.0,
                cache_read: 1.0,
                cache_write: 12.5,
            },
            context_window: 1000000,
            max_tokens: 128000,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "anthropic/claude-haiku-4.5".into(),
        Model {
            id: "anthropic/claude-haiku-4.5".into(),
            name: "Anthropic: Claude Haiku 4.5".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
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
            name: "Anthropic: Claude Opus 4".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
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
            name: "Anthropic: Claude Opus 4.1".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
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
            name: "Anthropic: Claude Opus 4.5".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
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
            name: "Anthropic: Claude Opus 4.6".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
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
        "anthropic/claude-opus-4.6-fast".into(),
        Model {
            id: "anthropic/claude-opus-4.6-fast".into(),
            name: "Anthropic: Claude Opus 4.6 (Fast)".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 30.0,
                output: 150.0,
                cache_read: 3.0,
                cache_write: 37.5,
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
            name: "Anthropic: Claude Opus 4.7".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
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
        "anthropic/claude-opus-4.7-fast".into(),
        Model {
            id: "anthropic/claude-opus-4.7-fast".into(),
            name: "Anthropic: Claude Opus 4.7 (Fast)".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 30.0,
                output: 150.0,
                cache_read: 3.0,
                cache_write: 37.5,
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
            name: "Anthropic: Claude Opus 4.8".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
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
        "anthropic/claude-opus-4.8-fast".into(),
        Model {
            id: "anthropic/claude-opus-4.8-fast".into(),
            name: "Anthropic: Claude Opus 4.8 (Fast)".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 10.0,
                output: 50.0,
                cache_read: 1.0,
                cache_write: 12.5,
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
            name: "Anthropic: Claude Sonnet 4".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
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
            name: "Anthropic: Claude Sonnet 4.5".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
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
            name: "Anthropic: Claude Sonnet 4.6".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
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
        "arcee-ai/trinity-large-thinking".into(),
        Model {
            id: "arcee-ai/trinity-large-thinking".into(),
            name: "Arcee AI: Trinity Large Thinking".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.25,
                output: 0.8,
                cache_read: 0.06,
                cache_write: 0.0,
            },
            context_window: 262144,
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
            name: "Arcee AI: Trinity Mini".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: true,
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
        "arcee-ai/virtuoso-large".into(),
        Model {
            id: "arcee-ai/virtuoso-large".into(),
            name: "Arcee AI: Virtuoso Large".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: false,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.75,
                output: 1.2,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 131072,
            max_tokens: 64000,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "bytedance-seed/seed-1.6".into(),
        Model {
            id: "bytedance-seed/seed-1.6".into(),
            name: "ByteDance Seed: Seed 1.6".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.25,
                output: 2.0,
                cache_read: 0.0,
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
        "bytedance-seed/seed-1.6-flash".into(),
        Model {
            id: "bytedance-seed/seed-1.6-flash".into(),
            name: "ByteDance Seed: Seed 1.6 Flash".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.075,
                output: 0.3,
                cache_read: 0.0,
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
        "bytedance-seed/seed-2.0-lite".into(),
        Model {
            id: "bytedance-seed/seed-2.0-lite".into(),
            name: "ByteDance Seed: Seed-2.0-Lite".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.25,
                output: 2.0,
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
        "bytedance-seed/seed-2.0-mini".into(),
        Model {
            id: "bytedance-seed/seed-2.0-mini".into(),
            name: "ByteDance Seed: Seed-2.0-Mini".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.1,
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
        "cohere/command-r-08-2024".into(),
        Model {
            id: "cohere/command-r-08-2024".into(),
            name: "Cohere: Command R (08-2024)".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: false,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.15,
                output: 0.6,
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
        "cohere/command-r-plus-08-2024".into(),
        Model {
            id: "cohere/command-r-plus-08-2024".into(),
            name: "Cohere: Command R+ (08-2024)".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: false,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 2.5,
                output: 10.0,
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
        "cohere/north-mini-code:free".into(),
        Model {
            id: "cohere/north-mini-code:free".into(),
            name: "Cohere: North Mini Code (free)".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.0,
                output: 0.0,
                cache_read: 0.0,
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
        "deepseek/deepseek-chat".into(),
        Model {
            id: "deepseek/deepseek-chat".into(),
            name: "DeepSeek: DeepSeek V3".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: false,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.2002,
                output: 0.8001,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 131072,
            max_tokens: 16000,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "deepseek/deepseek-chat-v3-0324".into(),
        Model {
            id: "deepseek/deepseek-chat-v3-0324".into(),
            name: "DeepSeek: DeepSeek V3 0324".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: false,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.2,
                output: 0.77,
                cache_read: 0.135,
                cache_write: 0.0,
            },
            context_window: 163840,
            max_tokens: 16384,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "deepseek/deepseek-chat-v3.1".into(),
        Model {
            id: "deepseek/deepseek-chat-v3.1".into(),
            name: "DeepSeek: DeepSeek V3.1".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.21,
                output: 0.79,
                cache_read: 0.13,
                cache_write: 0.0,
            },
            context_window: 163840,
            max_tokens: 32768,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "deepseek/deepseek-r1".into(),
        Model {
            id: "deepseek/deepseek-r1".into(),
            name: "DeepSeek: R1".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.7,
                output: 2.5,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 163840,
            max_tokens: 16000,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "deepseek/deepseek-r1-0528".into(),
        Model {
            id: "deepseek/deepseek-r1-0528".into(),
            name: "DeepSeek: R1 0528".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.5,
                output: 2.15,
                cache_read: 0.35,
                cache_write: 0.0,
            },
            context_window: 163840,
            max_tokens: 32768,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "deepseek/deepseek-v3.1-terminus".into(),
        Model {
            id: "deepseek/deepseek-v3.1-terminus".into(),
            name: "DeepSeek: DeepSeek V3.1 Terminus".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.27,
                output: 0.95,
                cache_read: 0.13,
                cache_write: 0.0,
            },
            context_window: 163840,
            max_tokens: 32768,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "deepseek/deepseek-v3.2".into(),
        Model {
            id: "deepseek/deepseek-v3.2".into(),
            name: "DeepSeek: DeepSeek V3.2".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.2288,
                output: 0.3432,
                cache_read: 0.02288,
                cache_write: 0.0,
            },
            context_window: 131072,
            max_tokens: 64000,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "deepseek/deepseek-v3.2-exp".into(),
        Model {
            id: "deepseek/deepseek-v3.2-exp".into(),
            name: "DeepSeek: DeepSeek V3.2 Exp".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.27,
                output: 0.41,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 163840,
            max_tokens: 65536,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "deepseek/deepseek-v4-flash".into(),
        Model {
            id: "deepseek/deepseek-v4-flash".into(),
            name: "DeepSeek: DeepSeek V4 Flash".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.09,
                output: 0.18,
                cache_read: 0.02,
                cache_write: 0.0,
            },
            context_window: 1048576,
            max_tokens: 65536,
            thinking_level_map: Some(HashMap::from([
                (ModelThinkingLevel::Minimal, None),
                (ModelThinkingLevel::Low, None),
                (ModelThinkingLevel::Medium, None),
                (ModelThinkingLevel::High, Some("high".into())),
                (ModelThinkingLevel::XHigh, Some("xhigh".into())),
            ])),
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "deepseek/deepseek-v4-pro".into(),
        Model {
            id: "deepseek/deepseek-v4-pro".into(),
            name: "DeepSeek: DeepSeek V4 Pro".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.435,
                output: 0.87,
                cache_read: 0.003625,
                cache_write: 0.0,
            },
            context_window: 1048576,
            max_tokens: 384000,
            thinking_level_map: Some(HashMap::from([
                (ModelThinkingLevel::Minimal, None),
                (ModelThinkingLevel::Low, None),
                (ModelThinkingLevel::Medium, None),
                (ModelThinkingLevel::High, Some("high".into())),
                (ModelThinkingLevel::XHigh, Some("xhigh".into())),
            ])),
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "google/gemini-2.5-flash".into(),
        Model {
            id: "google/gemini-2.5-flash".into(),
            name: "Google: Gemini 2.5 Flash".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.3,
                output: 2.5,
                cache_read: 0.03,
                cache_write: 0.083333,
            },
            context_window: 1048576,
            max_tokens: 65535,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "google/gemini-2.5-flash-lite".into(),
        Model {
            id: "google/gemini-2.5-flash-lite".into(),
            name: "Google: Gemini 2.5 Flash Lite".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.1,
                output: 0.4,
                cache_read: 0.01,
                cache_write: 0.083333,
            },
            context_window: 1048576,
            max_tokens: 65535,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "google/gemini-2.5-flash-lite-preview-09-2025".into(),
        Model {
            id: "google/gemini-2.5-flash-lite-preview-09-2025".into(),
            name: "Google: Gemini 2.5 Flash Lite Preview 09-2025".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.1,
                output: 0.4,
                cache_read: 0.01,
                cache_write: 0.083333,
            },
            context_window: 1048576,
            max_tokens: 65535,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "google/gemini-2.5-pro".into(),
        Model {
            id: "google/gemini-2.5-pro".into(),
            name: "Google: Gemini 2.5 Pro".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 1.25,
                output: 10.0,
                cache_read: 0.125,
                cache_write: 0.375,
            },
            context_window: 1048576,
            max_tokens: 65536,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "google/gemini-2.5-pro-preview".into(),
        Model {
            id: "google/gemini-2.5-pro-preview".into(),
            name: "Google: Gemini 2.5 Pro Preview 06-05".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 1.25,
                output: 10.0,
                cache_read: 0.125,
                cache_write: 0.375,
            },
            context_window: 1048576,
            max_tokens: 65536,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "google/gemini-2.5-pro-preview-05-06".into(),
        Model {
            id: "google/gemini-2.5-pro-preview-05-06".into(),
            name: "Google: Gemini 2.5 Pro Preview 05-06".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 1.25,
                output: 10.0,
                cache_read: 0.125,
                cache_write: 0.375,
            },
            context_window: 1048576,
            max_tokens: 65535,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "google/gemini-3-flash-preview".into(),
        Model {
            id: "google/gemini-3-flash-preview".into(),
            name: "Google: Gemini 3 Flash Preview".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.5,
                output: 3.0,
                cache_read: 0.05,
                cache_write: 0.083333,
            },
            context_window: 1048576,
            max_tokens: 65535,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "google/gemini-3-pro-image".into(),
        Model {
            id: "google/gemini-3-pro-image".into(),
            name: "Google: Nano Banana Pro (Gemini 3 Pro Image)".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 2.0,
                output: 12.0,
                cache_read: 0.2,
                cache_write: 0.375,
            },
            context_window: 65536,
            max_tokens: 32768,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "google/gemini-3.1-flash-lite".into(),
        Model {
            id: "google/gemini-3.1-flash-lite".into(),
            name: "Google: Gemini 3.1 Flash Lite".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.25,
                output: 1.5,
                cache_read: 0.025,
                cache_write: 0.083333,
            },
            context_window: 1048576,
            max_tokens: 65536,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "google/gemini-3.1-flash-lite-preview".into(),
        Model {
            id: "google/gemini-3.1-flash-lite-preview".into(),
            name: "Google: Gemini 3.1 Flash Lite Preview".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.25,
                output: 1.5,
                cache_read: 0.025,
                cache_write: 0.083333,
            },
            context_window: 1048576,
            max_tokens: 65536,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "google/gemini-3.1-pro-preview".into(),
        Model {
            id: "google/gemini-3.1-pro-preview".into(),
            name: "Google: Gemini 3.1 Pro Preview".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 2.0,
                output: 12.0,
                cache_read: 0.2,
                cache_write: 0.375,
            },
            context_window: 1048576,
            max_tokens: 65536,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "google/gemini-3.1-pro-preview-customtools".into(),
        Model {
            id: "google/gemini-3.1-pro-preview-customtools".into(),
            name: "Google: Gemini 3.1 Pro Preview Custom Tools".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 2.0,
                output: 12.0,
                cache_read: 0.2,
                cache_write: 0.375,
            },
            context_window: 1048756,
            max_tokens: 65536,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "google/gemini-3.5-flash".into(),
        Model {
            id: "google/gemini-3.5-flash".into(),
            name: "Google: Gemini 3.5 Flash".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 1.5,
                output: 9.0,
                cache_read: 0.15,
                cache_write: 0.083333,
            },
            context_window: 1048576,
            max_tokens: 65536,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "google/gemma-3-12b-it".into(),
        Model {
            id: "google/gemma-3-12b-it".into(),
            name: "Google: Gemma 3 12B".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: false,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.05,
                output: 0.15,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 131072,
            max_tokens: 16384,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "google/gemma-3-27b-it".into(),
        Model {
            id: "google/gemma-3-27b-it".into(),
            name: "Google: Gemma 3 27B".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: false,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.08,
                output: 0.16,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 131072,
            max_tokens: 16384,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "google/gemma-4-26b-a4b-it".into(),
        Model {
            id: "google/gemma-4-26b-a4b-it".into(),
            name: "Google: Gemma 4 26B A4B ".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.06,
                output: 0.33,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 262144,
            max_tokens: 4096,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "google/gemma-4-26b-a4b-it:free".into(),
        Model {
            id: "google/gemma-4-26b-a4b-it:free".into(),
            name: "Google: Gemma 4 26B A4B  (free)".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.0,
                output: 0.0,
                cache_read: 0.0,
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
        "google/gemma-4-31b-it".into(),
        Model {
            id: "google/gemma-4-31b-it".into(),
            name: "Google: Gemma 4 31B".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.12,
                output: 0.35,
                cache_read: 0.09,
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
        "google/gemma-4-31b-it:free".into(),
        Model {
            id: "google/gemma-4-31b-it:free".into(),
            name: "Google: Gemma 4 31B (free)".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.0,
                output: 0.0,
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
        "ibm-granite/granite-4.1-8b".into(),
        Model {
            id: "ibm-granite/granite-4.1-8b".into(),
            name: "IBM: Granite 4.1 8B".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: false,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.05,
                output: 0.1,
                cache_read: 0.05,
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
        "inception/mercury-2".into(),
        Model {
            id: "inception/mercury-2".into(),
            name: "Inception: Mercury 2".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.25,
                output: 0.75,
                cache_read: 0.025,
                cache_write: 0.0,
            },
            context_window: 128000,
            max_tokens: 50000,
            thinking_level_map: Some(HashMap::from([(ModelThinkingLevel::Off, None)])),
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "inclusionai/ling-2.6-1t".into(),
        Model {
            id: "inclusionai/ling-2.6-1t".into(),
            name: "inclusionAI: Ling-2.6-1T".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: false,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.075,
                output: 0.625,
                cache_read: 0.015,
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
        "inclusionai/ling-2.6-flash".into(),
        Model {
            id: "inclusionai/ling-2.6-flash".into(),
            name: "inclusionAI: Ling-2.6-flash".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: false,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.01,
                output: 0.03,
                cache_read: 0.002,
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
        "inclusionai/ring-2.6-1t".into(),
        Model {
            id: "inclusionai/ring-2.6-1t".into(),
            name: "inclusionAI: Ring-2.6-1T".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.075,
                output: 0.625,
                cache_read: 0.015,
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
        "kwaipilot/kat-coder-pro-v2".into(),
        Model {
            id: "kwaipilot/kat-coder-pro-v2".into(),
            name: "Kwaipilot: KAT-Coder-Pro V2".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: false,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.3,
                output: 1.2,
                cache_read: 0.06,
                cache_write: 0.0,
            },
            context_window: 256000,
            max_tokens: 80000,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "liquid/lfm-2.5-1.2b-thinking:free".into(),
        Model {
            id: "liquid/lfm-2.5-1.2b-thinking:free".into(),
            name: "LiquidAI: LFM2.5-1.2B-Thinking (free)".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.0,
                output: 0.0,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 32768,
            max_tokens: 4096,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "meta-llama/llama-3.1-70b-instruct".into(),
        Model {
            id: "meta-llama/llama-3.1-70b-instruct".into(),
            name: "Meta: Llama 3.1 70B Instruct".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: false,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.4,
                output: 0.4,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 131072,
            max_tokens: 16384,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "meta-llama/llama-3.1-8b-instruct".into(),
        Model {
            id: "meta-llama/llama-3.1-8b-instruct".into(),
            name: "Meta: Llama 3.1 8B Instruct".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: false,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.02,
                output: 0.03,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 131072,
            max_tokens: 16384,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "meta-llama/llama-3.3-70b-instruct".into(),
        Model {
            id: "meta-llama/llama-3.3-70b-instruct".into(),
            name: "Meta: Llama 3.3 70B Instruct".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: false,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.1,
                output: 0.32,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 131072,
            max_tokens: 16384,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "meta-llama/llama-3.3-70b-instruct:free".into(),
        Model {
            id: "meta-llama/llama-3.3-70b-instruct:free".into(),
            name: "Meta: Llama 3.3 70B Instruct (free)".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: false,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.0,
                output: 0.0,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 131072,
            max_tokens: 4096,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "meta-llama/llama-4-maverick".into(),
        Model {
            id: "meta-llama/llama-4-maverick".into(),
            name: "Meta: Llama 4 Maverick".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: false,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.15,
                output: 0.6,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 1048576,
            max_tokens: 16384,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "meta-llama/llama-4-scout".into(),
        Model {
            id: "meta-llama/llama-4-scout".into(),
            name: "Meta: Llama 4 Scout".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: false,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.1,
                output: 0.3,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 10000000,
            max_tokens: 16384,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "minimax/minimax-m1".into(),
        Model {
            id: "minimax/minimax-m1".into(),
            name: "MiniMax: MiniMax M1".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.4,
                output: 2.2,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 1000000,
            max_tokens: 40000,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "minimax/minimax-m2".into(),
        Model {
            id: "minimax/minimax-m2".into(),
            name: "MiniMax: MiniMax M2".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.255,
                output: 1.0,
                cache_read: 0.03,
                cache_write: 0.0,
            },
            context_window: 204800,
            max_tokens: 196608,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "minimax/minimax-m2.1".into(),
        Model {
            id: "minimax/minimax-m2.1".into(),
            name: "MiniMax: MiniMax M2.1".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.29,
                output: 0.95,
                cache_read: 0.03,
                cache_write: 0.0,
            },
            context_window: 204800,
            max_tokens: 196608,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "minimax/minimax-m2.5".into(),
        Model {
            id: "minimax/minimax-m2.5".into(),
            name: "MiniMax: MiniMax M2.5".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.12,
                output: 0.48,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 204800,
            max_tokens: 196608,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "minimax/minimax-m2.7".into(),
        Model {
            id: "minimax/minimax-m2.7".into(),
            name: "MiniMax: MiniMax M2.7".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.18,
                output: 0.72,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 204800,
            max_tokens: 196608,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "minimax/minimax-m3".into(),
        Model {
            id: "minimax/minimax-m3".into(),
            name: "MiniMax: MiniMax M3".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.3,
                output: 1.2,
                cache_read: 0.06,
                cache_write: 0.0,
            },
            context_window: 1048576,
            max_tokens: 512000,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "mistralai/codestral-2508".into(),
        Model {
            id: "mistralai/codestral-2508".into(),
            name: "Mistral: Codestral 2508".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: false,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.3,
                output: 0.9,
                cache_read: 0.03,
                cache_write: 0.0,
            },
            context_window: 256000,
            max_tokens: 4096,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "mistralai/devstral-2512".into(),
        Model {
            id: "mistralai/devstral-2512".into(),
            name: "Mistral: Devstral 2 2512".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: false,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.4,
                output: 2.0,
                cache_read: 0.04,
                cache_write: 0.0,
            },
            context_window: 262144,
            max_tokens: 4096,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "mistralai/ministral-14b-2512".into(),
        Model {
            id: "mistralai/ministral-14b-2512".into(),
            name: "Mistral: Ministral 3 14B 2512".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: false,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.2,
                output: 0.2,
                cache_read: 0.02,
                cache_write: 0.0,
            },
            context_window: 262144,
            max_tokens: 4096,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "mistralai/ministral-3b-2512".into(),
        Model {
            id: "mistralai/ministral-3b-2512".into(),
            name: "Mistral: Ministral 3 3B 2512".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: false,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.1,
                output: 0.1,
                cache_read: 0.01,
                cache_write: 0.0,
            },
            context_window: 131072,
            max_tokens: 4096,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "mistralai/ministral-8b-2512".into(),
        Model {
            id: "mistralai/ministral-8b-2512".into(),
            name: "Mistral: Ministral 3 8B 2512".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: false,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.15,
                output: 0.15,
                cache_read: 0.015,
                cache_write: 0.0,
            },
            context_window: 262144,
            max_tokens: 4096,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "mistralai/mistral-large".into(),
        Model {
            id: "mistralai/mistral-large".into(),
            name: "Mistral Large".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: false,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 2.0,
                output: 6.0,
                cache_read: 0.2,
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
        "mistralai/mistral-large-2407".into(),
        Model {
            id: "mistralai/mistral-large-2407".into(),
            name: "Mistral Large 2407".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: false,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 2.0,
                output: 6.0,
                cache_read: 0.2,
                cache_write: 0.0,
            },
            context_window: 131072,
            max_tokens: 4096,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "mistralai/mistral-large-2512".into(),
        Model {
            id: "mistralai/mistral-large-2512".into(),
            name: "Mistral: Mistral Large 3 2512".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: false,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.5,
                output: 1.5,
                cache_read: 0.05,
                cache_write: 0.0,
            },
            context_window: 262144,
            max_tokens: 4096,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "mistralai/mistral-medium-3".into(),
        Model {
            id: "mistralai/mistral-medium-3".into(),
            name: "Mistral: Mistral Medium 3".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: false,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.4,
                output: 2.0,
                cache_read: 0.04,
                cache_write: 0.0,
            },
            context_window: 131072,
            max_tokens: 4096,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "mistralai/mistral-medium-3-5".into(),
        Model {
            id: "mistralai/mistral-medium-3-5".into(),
            name: "Mistral: Mistral Medium 3.5".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 1.5,
                output: 7.5,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 262144,
            max_tokens: 4096,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "mistralai/mistral-medium-3.1".into(),
        Model {
            id: "mistralai/mistral-medium-3.1".into(),
            name: "Mistral: Mistral Medium 3.1".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: false,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.4,
                output: 2.0,
                cache_read: 0.04,
                cache_write: 0.0,
            },
            context_window: 131072,
            max_tokens: 4096,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "mistralai/mistral-nemo".into(),
        Model {
            id: "mistralai/mistral-nemo".into(),
            name: "Mistral: Mistral Nemo".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: false,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.02,
                output: 0.03,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 131072,
            max_tokens: 4096,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "mistralai/mistral-saba".into(),
        Model {
            id: "mistralai/mistral-saba".into(),
            name: "Mistral: Saba".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: false,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.2,
                output: 0.6,
                cache_read: 0.02,
                cache_write: 0.0,
            },
            context_window: 32768,
            max_tokens: 4096,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "mistralai/mistral-small-2603".into(),
        Model {
            id: "mistralai/mistral-small-2603".into(),
            name: "Mistral: Mistral Small 4".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.15,
                output: 0.6,
                cache_read: 0.015,
                cache_write: 0.0,
            },
            context_window: 262144,
            max_tokens: 4096,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "mistralai/mistral-small-3.2-24b-instruct".into(),
        Model {
            id: "mistralai/mistral-small-3.2-24b-instruct".into(),
            name: "Mistral: Mistral Small 3.2 24B".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: false,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.075,
                output: 0.2,
                cache_read: 0.0,
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
        "mistralai/mixtral-8x22b-instruct".into(),
        Model {
            id: "mistralai/mixtral-8x22b-instruct".into(),
            name: "Mistral: Mixtral 8x22B Instruct".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: false,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 2.0,
                output: 6.0,
                cache_read: 0.2,
                cache_write: 0.0,
            },
            context_window: 65536,
            max_tokens: 4096,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "mistralai/voxtral-small-24b-2507".into(),
        Model {
            id: "mistralai/voxtral-small-24b-2507".into(),
            name: "Mistral: Voxtral Small 24B 2507".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: false,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.1,
                output: 0.3,
                cache_read: 0.01,
                cache_write: 0.0,
            },
            context_window: 32000,
            max_tokens: 4096,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "moonshotai/kimi-k2".into(),
        Model {
            id: "moonshotai/kimi-k2".into(),
            name: "MoonshotAI: Kimi K2 0711".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: false,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.57,
                output: 2.3,
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
        "moonshotai/kimi-k2-0905".into(),
        Model {
            id: "moonshotai/kimi-k2-0905".into(),
            name: "MoonshotAI: Kimi K2 0905".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: false,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.6,
                output: 2.5,
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
        "moonshotai/kimi-k2-thinking".into(),
        Model {
            id: "moonshotai/kimi-k2-thinking".into(),
            name: "MoonshotAI: Kimi K2 Thinking".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.6,
                output: 2.5,
                cache_read: 0.6,
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
        "moonshotai/kimi-k2.5".into(),
        Model {
            id: "moonshotai/kimi-k2.5".into(),
            name: "MoonshotAI: Kimi K2.5".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.41,
                output: 2.06,
                cache_read: 0.07,
                cache_write: 0.0,
            },
            context_window: 262144,
            max_tokens: 4096,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "moonshotai/kimi-k2.6".into(),
        Model {
            id: "moonshotai/kimi-k2.6".into(),
            name: "MoonshotAI: Kimi K2.6".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.66,
                output: 3.41,
                cache_read: 0.144,
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
        "moonshotai/kimi-k2.7-code".into(),
        Model {
            id: "moonshotai/kimi-k2.7-code".into(),
            name: "MoonshotAI: Kimi K2.7 Code".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.74,
                output: 3.5,
                cache_read: 0.15,
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
        "nvidia/llama-3.3-nemotron-super-49b-v1.5".into(),
        Model {
            id: "nvidia/llama-3.3-nemotron-super-49b-v1.5".into(),
            name: "NVIDIA: Llama 3.3 Nemotron Super 49B V1.5".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.4,
                output: 0.4,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 131072,
            max_tokens: 16384,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "nvidia/nemotron-3-nano-30b-a3b".into(),
        Model {
            id: "nvidia/nemotron-3-nano-30b-a3b".into(),
            name: "NVIDIA: Nemotron 3 Nano 30B A3B".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.05,
                output: 0.2,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 262144,
            max_tokens: 228000,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "nvidia/nemotron-3-nano-30b-a3b:free".into(),
        Model {
            id: "nvidia/nemotron-3-nano-30b-a3b:free".into(),
            name: "NVIDIA: Nemotron 3 Nano 30B A3B (free)".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.0,
                output: 0.0,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 256000,
            max_tokens: 4096,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "nvidia/nemotron-3-nano-omni-30b-a3b-reasoning:free".into(),
        Model {
            id: "nvidia/nemotron-3-nano-omni-30b-a3b-reasoning:free".into(),
            name: "NVIDIA: Nemotron 3 Nano Omni (free)".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.0,
                output: 0.0,
                cache_read: 0.0,
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
        "nvidia/nemotron-3-super-120b-a12b".into(),
        Model {
            id: "nvidia/nemotron-3-super-120b-a12b".into(),
            name: "NVIDIA: Nemotron 3 Super".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.085,
                output: 0.4,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 1000000,
            max_tokens: 16384,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "nvidia/nemotron-3-super-120b-a12b:free".into(),
        Model {
            id: "nvidia/nemotron-3-super-120b-a12b:free".into(),
            name: "NVIDIA: Nemotron 3 Super (free)".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.0,
                output: 0.0,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 1000000,
            max_tokens: 262144,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "nvidia/nemotron-3-ultra-550b-a55b".into(),
        Model {
            id: "nvidia/nemotron-3-ultra-550b-a55b".into(),
            name: "NVIDIA: Nemotron 3 Ultra".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.5,
                output: 2.2,
                cache_read: 0.1,
                cache_write: 0.0,
            },
            context_window: 1000000,
            max_tokens: 16384,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "nvidia/nemotron-3-ultra-550b-a55b:free".into(),
        Model {
            id: "nvidia/nemotron-3-ultra-550b-a55b:free".into(),
            name: "NVIDIA: Nemotron 3 Ultra (free)".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.0,
                output: 0.0,
                cache_read: 0.0,
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
        "nvidia/nemotron-nano-12b-v2-vl:free".into(),
        Model {
            id: "nvidia/nemotron-nano-12b-v2-vl:free".into(),
            name: "NVIDIA: Nemotron Nano 12B 2 VL (free)".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.0,
                output: 0.0,
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
        "nvidia/nemotron-nano-9b-v2:free".into(),
        Model {
            id: "nvidia/nemotron-nano-9b-v2:free".into(),
            name: "NVIDIA: Nemotron Nano 9B V2 (free)".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.0,
                output: 0.0,
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
        "openai/gpt-3.5-turbo".into(),
        Model {
            id: "openai/gpt-3.5-turbo".into(),
            name: "OpenAI: GPT-3.5 Turbo".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
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
        "openai/gpt-3.5-turbo-0613".into(),
        Model {
            id: "openai/gpt-3.5-turbo-0613".into(),
            name: "OpenAI: GPT-3.5 Turbo (older v0613)".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: false,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 1.0,
                output: 2.0,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 4095,
            max_tokens: 4096,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "openai/gpt-3.5-turbo-16k".into(),
        Model {
            id: "openai/gpt-3.5-turbo-16k".into(),
            name: "OpenAI: GPT-3.5 Turbo 16k".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: false,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 3.0,
                output: 4.0,
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
        "openai/gpt-4".into(),
        Model {
            id: "openai/gpt-4".into(),
            name: "OpenAI: GPT-4".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: false,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 30.0,
                output: 60.0,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 8191,
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
            name: "OpenAI: GPT-4 Turbo".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
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
        "openai/gpt-4-turbo-preview".into(),
        Model {
            id: "openai/gpt-4-turbo-preview".into(),
            name: "OpenAI: GPT-4 Turbo Preview".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: false,
            input: vec![InputModality::Text],
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
            name: "OpenAI: GPT-4.1".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: false,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 2.0,
                output: 8.0,
                cache_read: 0.5,
                cache_write: 0.0,
            },
            context_window: 1047576,
            max_tokens: 4096,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "openai/gpt-4.1-mini".into(),
        Model {
            id: "openai/gpt-4.1-mini".into(),
            name: "OpenAI: GPT-4.1 Mini".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
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
            name: "OpenAI: GPT-4.1 Nano".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
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
            name: "OpenAI: GPT-4o".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: false,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 2.5,
                output: 10.0,
                cache_read: 0.0,
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
        "openai/gpt-4o-2024-05-13".into(),
        Model {
            id: "openai/gpt-4o-2024-05-13".into(),
            name: "OpenAI: GPT-4o (2024-05-13)".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: false,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 5.0,
                output: 15.0,
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
        "openai/gpt-4o-2024-08-06".into(),
        Model {
            id: "openai/gpt-4o-2024-08-06".into(),
            name: "OpenAI: GPT-4o (2024-08-06)".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
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
        "openai/gpt-4o-2024-11-20".into(),
        Model {
            id: "openai/gpt-4o-2024-11-20".into(),
            name: "OpenAI: GPT-4o (2024-11-20)".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
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
            name: "OpenAI: GPT-4o-mini".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
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
        "openai/gpt-4o-mini-2024-07-18".into(),
        Model {
            id: "openai/gpt-4o-mini-2024-07-18".into(),
            name: "OpenAI: GPT-4o-mini (2024-07-18)".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
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
            name: "OpenAI: GPT-5".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
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
        "openai/gpt-5-codex".into(),
        Model {
            id: "openai/gpt-5-codex".into(),
            name: "OpenAI: GPT-5 Codex".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
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
            name: "OpenAI: GPT-5 Mini".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
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
            name: "OpenAI: GPT-5 Nano".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.05,
                output: 0.4,
                cache_read: 0.01,
                cache_write: 0.0,
            },
            context_window: 400000,
            max_tokens: 4096,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "openai/gpt-5-pro".into(),
        Model {
            id: "openai/gpt-5-pro".into(),
            name: "OpenAI: GPT-5 Pro".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 15.0,
                output: 120.0,
                cache_read: 0.0,
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
        "openai/gpt-5.1".into(),
        Model {
            id: "openai/gpt-5.1".into(),
            name: "OpenAI: GPT-5.1".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 1.25,
                output: 10.0,
                cache_read: 0.13,
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
        "openai/gpt-5.1-chat".into(),
        Model {
            id: "openai/gpt-5.1-chat".into(),
            name: "OpenAI: GPT-5.1 Chat".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: false,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 1.25,
                output: 10.0,
                cache_read: 0.13,
                cache_write: 0.0,
            },
            context_window: 128000,
            max_tokens: 32000,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "openai/gpt-5.1-codex".into(),
        Model {
            id: "openai/gpt-5.1-codex".into(),
            name: "OpenAI: GPT-5.1-Codex".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 1.25,
                output: 10.0,
                cache_read: 0.13,
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
            name: "OpenAI: GPT-5.1-Codex-Max".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
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
            name: "OpenAI: GPT-5.1-Codex-Mini".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.25,
                output: 2.0,
                cache_read: 0.025,
                cache_write: 0.0,
            },
            context_window: 400000,
            max_tokens: 100000,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "openai/gpt-5.2".into(),
        Model {
            id: "openai/gpt-5.2".into(),
            name: "OpenAI: GPT-5.2".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
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
            name: "OpenAI: GPT-5.2 Chat".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
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
            name: "OpenAI: GPT-5.2-Codex".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
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
            name: "OpenAI: GPT-5.2 Pro".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
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
            name: "OpenAI: GPT-5.3 Chat".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
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
            name: "OpenAI: GPT-5.3-Codex".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
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
            name: "OpenAI: GPT-5.4".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
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
            name: "OpenAI: GPT-5.4 Mini".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
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
            name: "OpenAI: GPT-5.4 Nano".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
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
            name: "OpenAI: GPT-5.4 Pro".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
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
            name: "OpenAI: GPT-5.5".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 5.0,
                output: 30.0,
                cache_read: 0.5,
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
        "openai/gpt-5.5-pro".into(),
        Model {
            id: "openai/gpt-5.5-pro".into(),
            name: "OpenAI: GPT-5.5 Pro".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
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
        "openai/gpt-audio".into(),
        Model {
            id: "openai/gpt-audio".into(),
            name: "OpenAI: GPT Audio".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: false,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 2.5,
                output: 10.0,
                cache_read: 0.0,
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
        "openai/gpt-audio-mini".into(),
        Model {
            id: "openai/gpt-audio-mini".into(),
            name: "OpenAI: GPT Audio Mini".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: false,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.6,
                output: 2.4,
                cache_read: 0.0,
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
        "openai/gpt-chat-latest".into(),
        Model {
            id: "openai/gpt-chat-latest".into(),
            name: "OpenAI: GPT Chat Latest".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: false,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 5.0,
                output: 30.0,
                cache_read: 0.5,
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
        "openai/gpt-oss-120b".into(),
        Model {
            id: "openai/gpt-oss-120b".into(),
            name: "OpenAI: gpt-oss-120b".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.03,
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
        "openai/gpt-oss-120b:free".into(),
        Model {
            id: "openai/gpt-oss-120b:free".into(),
            name: "OpenAI: gpt-oss-120b (free)".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.0,
                output: 0.0,
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
            name: "OpenAI: gpt-oss-20b".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.029,
                output: 0.14,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 131072,
            max_tokens: 4096,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "openai/gpt-oss-20b:free".into(),
        Model {
            id: "openai/gpt-oss-20b:free".into(),
            name: "OpenAI: gpt-oss-20b (free)".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.0,
                output: 0.0,
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
        "openai/gpt-oss-safeguard-20b".into(),
        Model {
            id: "openai/gpt-oss-safeguard-20b".into(),
            name: "OpenAI: gpt-oss-safeguard-20b".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.075,
                output: 0.3,
                cache_read: 0.0375,
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
            name: "OpenAI: o1".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
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
            name: "OpenAI: o3".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
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
            name: "OpenAI: o3 Deep Research".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
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
            name: "OpenAI: o3 Mini".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
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
        "openai/o3-mini-high".into(),
        Model {
            id: "openai/o3-mini-high".into(),
            name: "OpenAI: o3 Mini High".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
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
            name: "OpenAI: o3 Pro".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
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
            name: "OpenAI: o4 Mini".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
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
        "openai/o4-mini-deep-research".into(),
        Model {
            id: "openai/o4-mini-deep-research".into(),
            name: "OpenAI: o4 Mini Deep Research".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
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
        "openai/o4-mini-high".into(),
        Model {
            id: "openai/o4-mini-high".into(),
            name: "OpenAI: o4 Mini High".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
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
        "openrouter/auto".into(),
        Model {
            id: "openrouter/auto".into(),
            name: "Auto Router".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: -1000000.0,
                output: -1000000.0,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 2000000,
            max_tokens: 4096,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "openrouter/free".into(),
        Model {
            id: "openrouter/free".into(),
            name: "Free Models Router".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.0,
                output: 0.0,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 200000,
            max_tokens: 4096,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "openrouter/owl-alpha".into(),
        Model {
            id: "openrouter/owl-alpha".into(),
            name: "Owl Alpha".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: false,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.0,
                output: 0.0,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 1048756,
            max_tokens: 262144,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "poolside/laguna-m.1".into(),
        Model {
            id: "poolside/laguna-m.1".into(),
            name: "Poolside: Laguna M.1".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.2,
                output: 0.4,
                cache_read: 0.1,
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
        "poolside/laguna-m.1:free".into(),
        Model {
            id: "poolside/laguna-m.1:free".into(),
            name: "Poolside: Laguna M.1 (free)".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.0,
                output: 0.0,
                cache_read: 0.0,
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
        "poolside/laguna-xs.2".into(),
        Model {
            id: "poolside/laguna-xs.2".into(),
            name: "Poolside: Laguna XS.2".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.1,
                output: 0.2,
                cache_read: 0.05,
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
        "poolside/laguna-xs.2:free".into(),
        Model {
            id: "poolside/laguna-xs.2:free".into(),
            name: "Poolside: Laguna XS.2 (free)".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.0,
                output: 0.0,
                cache_read: 0.0,
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
        "qwen/qwen-2.5-72b-instruct".into(),
        Model {
            id: "qwen/qwen-2.5-72b-instruct".into(),
            name: "Qwen2.5 72B Instruct".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: false,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.36,
                output: 0.4,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 131072,
            max_tokens: 16384,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "qwen/qwen-2.5-7b-instruct".into(),
        Model {
            id: "qwen/qwen-2.5-7b-instruct".into(),
            name: "Qwen: Qwen2.5 7B Instruct".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: false,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.04,
                output: 0.1,
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
        "qwen/qwen-plus".into(),
        Model {
            id: "qwen/qwen-plus".into(),
            name: "Qwen: Qwen-Plus".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: false,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.26,
                output: 0.78,
                cache_read: 0.052,
                cache_write: 0.325,
            },
            context_window: 1000000,
            max_tokens: 32768,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "qwen/qwen-plus-2025-07-28".into(),
        Model {
            id: "qwen/qwen-plus-2025-07-28".into(),
            name: "Qwen: Qwen Plus 0728".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: false,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.26,
                output: 0.78,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 1000000,
            max_tokens: 32768,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "qwen/qwen-plus-2025-07-28:thinking".into(),
        Model {
            id: "qwen/qwen-plus-2025-07-28:thinking".into(),
            name: "Qwen: Qwen Plus 0728 (thinking)".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.26,
                output: 0.78,
                cache_read: 0.0,
                cache_write: 0.325,
            },
            context_window: 1000000,
            max_tokens: 32768,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "qwen/qwen3-14b".into(),
        Model {
            id: "qwen/qwen3-14b".into(),
            name: "Qwen: Qwen3 14B".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.1,
                output: 0.24,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 131702,
            max_tokens: 40960,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "qwen/qwen3-235b-a22b".into(),
        Model {
            id: "qwen/qwen3-235b-a22b".into(),
            name: "Qwen: Qwen3 235B A22B".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.455,
                output: 1.82,
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
        "qwen/qwen3-235b-a22b-2507".into(),
        Model {
            id: "qwen/qwen3-235b-a22b-2507".into(),
            name: "Qwen: Qwen3 235B A22B Instruct 2507".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: false,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.09,
                output: 0.1,
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
        "qwen/qwen3-235b-a22b-thinking-2507".into(),
        Model {
            id: "qwen/qwen3-235b-a22b-thinking-2507".into(),
            name: "Qwen: Qwen3 235B A22B Thinking 2507".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.1,
                output: 0.1,
                cache_read: 0.1,
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
        "qwen/qwen3-30b-a3b".into(),
        Model {
            id: "qwen/qwen3-30b-a3b".into(),
            name: "Qwen: Qwen3 30B A3B".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.12,
                output: 0.5,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 131072,
            max_tokens: 16384,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "qwen/qwen3-30b-a3b-instruct-2507".into(),
        Model {
            id: "qwen/qwen3-30b-a3b-instruct-2507".into(),
            name: "Qwen: Qwen3 30B A3B Instruct 2507".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: false,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.04815,
                output: 0.19305,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 131072,
            max_tokens: 32000,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "qwen/qwen3-30b-a3b-thinking-2507".into(),
        Model {
            id: "qwen/qwen3-30b-a3b-thinking-2507".into(),
            name: "Qwen: Qwen3 30B A3B Thinking 2507".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.08,
                output: 0.4,
                cache_read: 0.08,
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
        "qwen/qwen3-32b".into(),
        Model {
            id: "qwen/qwen3-32b".into(),
            name: "Qwen: Qwen3 32B".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.08,
                output: 0.28,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 131072,
            max_tokens: 16384,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "qwen/qwen3-8b".into(),
        Model {
            id: "qwen/qwen3-8b".into(),
            name: "Qwen: Qwen3 8B".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.05,
                output: 0.4,
                cache_read: 0.05,
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
        "qwen/qwen3-coder".into(),
        Model {
            id: "qwen/qwen3-coder".into(),
            name: "Qwen: Qwen3 Coder 480B A35B".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: false,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.22,
                output: 1.8,
                cache_read: 0.0,
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
        "qwen/qwen3-coder-30b-a3b-instruct".into(),
        Model {
            id: "qwen/qwen3-coder-30b-a3b-instruct".into(),
            name: "Qwen: Qwen3 Coder 30B A3B Instruct".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: false,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.07,
                output: 0.27,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 160000,
            max_tokens: 32768,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "qwen/qwen3-coder-flash".into(),
        Model {
            id: "qwen/qwen3-coder-flash".into(),
            name: "Qwen: Qwen3 Coder Flash".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: false,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.195,
                output: 0.975,
                cache_read: 0.039,
                cache_write: 0.24375,
            },
            context_window: 1000000,
            max_tokens: 65536,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "qwen/qwen3-coder-next".into(),
        Model {
            id: "qwen/qwen3-coder-next".into(),
            name: "Qwen: Qwen3 Coder Next".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: false,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.11,
                output: 0.8,
                cache_read: 0.07,
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
        "qwen/qwen3-coder-plus".into(),
        Model {
            id: "qwen/qwen3-coder-plus".into(),
            name: "Qwen: Qwen3 Coder Plus".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: false,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.65,
                output: 3.25,
                cache_read: 0.13,
                cache_write: 0.8125,
            },
            context_window: 1000000,
            max_tokens: 65536,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "qwen/qwen3-coder:free".into(),
        Model {
            id: "qwen/qwen3-coder:free".into(),
            name: "Qwen: Qwen3 Coder 480B A35B (free)".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: false,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.0,
                output: 0.0,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 1048576,
            max_tokens: 262000,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "qwen/qwen3-max".into(),
        Model {
            id: "qwen/qwen3-max".into(),
            name: "Qwen: Qwen3 Max".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: false,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.78,
                output: 3.9,
                cache_read: 0.156,
                cache_write: 0.975,
            },
            context_window: 262144,
            max_tokens: 32768,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "qwen/qwen3-max-thinking".into(),
        Model {
            id: "qwen/qwen3-max-thinking".into(),
            name: "Qwen: Qwen3 Max Thinking".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.78,
                output: 3.9,
                cache_read: 0.0,
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
        "qwen/qwen3-next-80b-a3b-instruct".into(),
        Model {
            id: "qwen/qwen3-next-80b-a3b-instruct".into(),
            name: "Qwen: Qwen3 Next 80B A3B Instruct".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: false,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.09,
                output: 1.1,
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
        "qwen/qwen3-next-80b-a3b-instruct:free".into(),
        Model {
            id: "qwen/qwen3-next-80b-a3b-instruct:free".into(),
            name: "Qwen: Qwen3 Next 80B A3B Instruct (free)".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: false,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.0,
                output: 0.0,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 262144,
            max_tokens: 4096,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "qwen/qwen3-next-80b-a3b-thinking".into(),
        Model {
            id: "qwen/qwen3-next-80b-a3b-thinking".into(),
            name: "Qwen: Qwen3 Next 80B A3B Thinking".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.0975,
                output: 0.78,
                cache_read: 0.0,
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
        "qwen/qwen3-vl-235b-a22b-instruct".into(),
        Model {
            id: "qwen/qwen3-vl-235b-a22b-instruct".into(),
            name: "Qwen: Qwen3 VL 235B A22B Instruct".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: false,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.2,
                output: 0.88,
                cache_read: 0.11,
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
        "qwen/qwen3-vl-235b-a22b-thinking".into(),
        Model {
            id: "qwen/qwen3-vl-235b-a22b-thinking".into(),
            name: "Qwen: Qwen3 VL 235B A22B Thinking".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.26,
                output: 2.6,
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
        "qwen/qwen3-vl-30b-a3b-instruct".into(),
        Model {
            id: "qwen/qwen3-vl-30b-a3b-instruct".into(),
            name: "Qwen: Qwen3 VL 30B A3B Instruct".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: false,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.13,
                output: 0.52,
                cache_read: 0.0,
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
        "qwen/qwen3-vl-30b-a3b-thinking".into(),
        Model {
            id: "qwen/qwen3-vl-30b-a3b-thinking".into(),
            name: "Qwen: Qwen3 VL 30B A3B Thinking".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.13,
                output: 1.56,
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
        "qwen/qwen3-vl-32b-instruct".into(),
        Model {
            id: "qwen/qwen3-vl-32b-instruct".into(),
            name: "Qwen: Qwen3 VL 32B Instruct".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: false,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.104,
                output: 0.416,
                cache_read: 0.0,
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
        "qwen/qwen3-vl-8b-instruct".into(),
        Model {
            id: "qwen/qwen3-vl-8b-instruct".into(),
            name: "Qwen: Qwen3 VL 8B Instruct".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: false,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.08,
                output: 0.5,
                cache_read: 0.0,
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
        "qwen/qwen3-vl-8b-thinking".into(),
        Model {
            id: "qwen/qwen3-vl-8b-thinking".into(),
            name: "Qwen: Qwen3 VL 8B Thinking".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.117,
                output: 1.365,
                cache_read: 0.0,
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
        "qwen/qwen3.5-122b-a10b".into(),
        Model {
            id: "qwen/qwen3.5-122b-a10b".into(),
            name: "Qwen: Qwen3.5-122B-A10B".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.26,
                output: 2.08,
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
        "qwen/qwen3.5-27b".into(),
        Model {
            id: "qwen/qwen3.5-27b".into(),
            name: "Qwen: Qwen3.5-27B".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.195,
                output: 1.56,
                cache_read: 0.0,
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
        "qwen/qwen3.5-35b-a3b".into(),
        Model {
            id: "qwen/qwen3.5-35b-a3b".into(),
            name: "Qwen: Qwen3.5-35B-A3B".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.14,
                output: 1.0,
                cache_read: 0.05,
                cache_write: 0.0,
            },
            context_window: 262144,
            max_tokens: 81920,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "qwen/qwen3.5-397b-a17b".into(),
        Model {
            id: "qwen/qwen3.5-397b-a17b".into(),
            name: "Qwen: Qwen3.5 397B A17B".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.385,
                output: 2.45,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 256000,
            max_tokens: 4096,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "qwen/qwen3.5-9b".into(),
        Model {
            id: "qwen/qwen3.5-9b".into(),
            name: "Qwen: Qwen3.5-9B".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.1,
                output: 0.15,
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
        "qwen/qwen3.5-flash-02-23".into(),
        Model {
            id: "qwen/qwen3.5-flash-02-23".into(),
            name: "Qwen: Qwen3.5-Flash".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.065,
                output: 0.26,
                cache_read: 0.0,
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
        "qwen/qwen3.5-plus-02-15".into(),
        Model {
            id: "qwen/qwen3.5-plus-02-15".into(),
            name: "Qwen: Qwen3.5 Plus 2026-02-15".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.26,
                output: 1.56,
                cache_read: 0.0,
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
        "qwen/qwen3.5-plus-20260420".into(),
        Model {
            id: "qwen/qwen3.5-plus-20260420".into(),
            name: "Qwen: Qwen3.5 Plus 2026-04-20".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.3,
                output: 1.8,
                cache_read: 0.0,
                cache_write: 0.375,
            },
            context_window: 1000000,
            max_tokens: 65536,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "qwen/qwen3.6-27b".into(),
        Model {
            id: "qwen/qwen3.6-27b".into(),
            name: "Qwen: Qwen3.6 27B".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.2885,
                output: 2.65,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 262144,
            max_tokens: 262140,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "qwen/qwen3.6-35b-a3b".into(),
        Model {
            id: "qwen/qwen3.6-35b-a3b".into(),
            name: "Qwen: Qwen3.6 35B A3B".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.14,
                output: 1.0,
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
        "qwen/qwen3.6-flash".into(),
        Model {
            id: "qwen/qwen3.6-flash".into(),
            name: "Qwen: Qwen3.6 Flash".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.1875,
                output: 1.125,
                cache_read: 0.0,
                cache_write: 0.234375,
            },
            context_window: 1000000,
            max_tokens: 65536,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "qwen/qwen3.6-max-preview".into(),
        Model {
            id: "qwen/qwen3.6-max-preview".into(),
            name: "Qwen: Qwen3.6 Max Preview".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 1.04,
                output: 6.24,
                cache_read: 0.0,
                cache_write: 1.3,
            },
            context_window: 262144,
            max_tokens: 65536,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "qwen/qwen3.6-plus".into(),
        Model {
            id: "qwen/qwen3.6-plus".into(),
            name: "Qwen: Qwen3.6 Plus".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.325,
                output: 1.95,
                cache_read: 0.0,
                cache_write: 0.40625,
            },
            context_window: 1000000,
            max_tokens: 65536,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "qwen/qwen3.7-max".into(),
        Model {
            id: "qwen/qwen3.7-max".into(),
            name: "Qwen: Qwen3.7 Max".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 1.25,
                output: 3.75,
                cache_read: 0.25,
                cache_write: 1.5625,
            },
            context_window: 1000000,
            max_tokens: 65536,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "qwen/qwen3.7-plus".into(),
        Model {
            id: "qwen/qwen3.7-plus".into(),
            name: "Qwen: Qwen3.7 Plus".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.32,
                output: 1.28,
                cache_read: 0.064,
                cache_write: 0.4,
            },
            context_window: 1000000,
            max_tokens: 65536,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "rekaai/reka-edge".into(),
        Model {
            id: "rekaai/reka-edge".into(),
            name: "Reka Edge".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: false,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.1,
                output: 0.1,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 16384,
            max_tokens: 16384,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "relace/relace-search".into(),
        Model {
            id: "relace/relace-search".into(),
            name: "Relace: Relace Search".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: false,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 1.0,
                output: 3.0,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 256000,
            max_tokens: 128000,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "sakana/fugu-ultra".into(),
        Model {
            id: "sakana/fugu-ultra".into(),
            name: "Sakana: Fugu Ultra".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
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
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "sao10k/l3.1-euryale-70b".into(),
        Model {
            id: "sao10k/l3.1-euryale-70b".into(),
            name: "Sao10K: Llama 3.1 Euryale 70B v2.2".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: false,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.85,
                output: 0.85,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 131072,
            max_tokens: 16384,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "stepfun/step-3.5-flash".into(),
        Model {
            id: "stepfun/step-3.5-flash".into(),
            name: "StepFun: Step 3.5 Flash".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.09,
                output: 0.3,
                cache_read: 0.02,
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
        "stepfun/step-3.7-flash".into(),
        Model {
            id: "stepfun/step-3.7-flash".into(),
            name: "StepFun: Step 3.7 Flash".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
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
        "tencent/hy3-preview".into(),
        Model {
            id: "tencent/hy3-preview".into(),
            name: "Tencent: Hy3 preview".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.063,
                output: 0.21,
                cache_read: 0.021,
                cache_write: 0.0,
            },
            context_window: 262144,
            max_tokens: 4096,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "thedrummer/unslopnemo-12b".into(),
        Model {
            id: "thedrummer/unslopnemo-12b".into(),
            name: "TheDrummer: UnslopNemo 12B".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: false,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.4,
                output: 0.4,
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
        "upstage/solar-pro-3".into(),
        Model {
            id: "upstage/solar-pro-3".into(),
            name: "Upstage: Solar Pro 3".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.15,
                output: 0.6,
                cache_read: 0.015,
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
        "x-ai/grok-4.20".into(),
        Model {
            id: "x-ai/grok-4.20".into(),
            name: "xAI: Grok 4.20".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 1.25,
                output: 2.5,
                cache_read: 0.2,
                cache_write: 0.0,
            },
            context_window: 2000000,
            max_tokens: 4096,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "x-ai/grok-4.3".into(),
        Model {
            id: "x-ai/grok-4.3".into(),
            name: "xAI: Grok 4.3".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 1.25,
                output: 2.5,
                cache_read: 0.2,
                cache_write: 0.0,
            },
            context_window: 1000000,
            max_tokens: 4096,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "x-ai/grok-build-0.1".into(),
        Model {
            id: "x-ai/grok-build-0.1".into(),
            name: "xAI: Grok Build 0.1".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 1.0,
                output: 2.0,
                cache_read: 0.2,
                cache_write: 0.0,
            },
            context_window: 256000,
            max_tokens: 4096,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "xiaomi/mimo-v2.5".into(),
        Model {
            id: "xiaomi/mimo-v2.5".into(),
            name: "Xiaomi: MiMo-V2.5".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.105,
                output: 0.28,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 1048576,
            max_tokens: 4096,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "xiaomi/mimo-v2.5-pro".into(),
        Model {
            id: "xiaomi/mimo-v2.5-pro".into(),
            name: "Xiaomi: MiMo-V2.5-Pro".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.435,
                output: 0.87,
                cache_read: 0.0036,
                cache_write: 0.0,
            },
            context_window: 1048576,
            max_tokens: 131072,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "z-ai/glm-4.5".into(),
        Model {
            id: "z-ai/glm-4.5".into(),
            name: "Z.ai: GLM 4.5".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.6,
                output: 2.2,
                cache_read: 0.11,
                cache_write: 0.0,
            },
            context_window: 131072,
            max_tokens: 98304,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "z-ai/glm-4.5-air".into(),
        Model {
            id: "z-ai/glm-4.5-air".into(),
            name: "Z.ai: GLM 4.5 Air".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.13,
                output: 0.85,
                cache_read: 0.025,
                cache_write: 0.0,
            },
            context_window: 131072,
            max_tokens: 98304,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "z-ai/glm-4.5v".into(),
        Model {
            id: "z-ai/glm-4.5v".into(),
            name: "Z.ai: GLM 4.5V".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.6,
                output: 1.8,
                cache_read: 0.11,
                cache_write: 0.0,
            },
            context_window: 65536,
            max_tokens: 16384,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "z-ai/glm-4.6".into(),
        Model {
            id: "z-ai/glm-4.6".into(),
            name: "Z.ai: GLM 4.6".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.43,
                output: 1.74,
                cache_read: 0.08,
                cache_write: 0.0,
            },
            context_window: 202752,
            max_tokens: 131072,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "z-ai/glm-4.6v".into(),
        Model {
            id: "z-ai/glm-4.6v".into(),
            name: "Z.ai: GLM 4.6V".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.3,
                output: 0.9,
                cache_read: 0.055,
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
        "z-ai/glm-4.7".into(),
        Model {
            id: "z-ai/glm-4.7".into(),
            name: "Z.ai: GLM 4.7".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.4,
                output: 1.75,
                cache_read: 0.08,
                cache_write: 0.0,
            },
            context_window: 202752,
            max_tokens: 131072,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "z-ai/glm-4.7-flash".into(),
        Model {
            id: "z-ai/glm-4.7-flash".into(),
            name: "Z.ai: GLM 4.7 Flash".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.06,
                output: 0.4,
                cache_read: 0.01,
                cache_write: 0.0,
            },
            context_window: 202752,
            max_tokens: 16384,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "z-ai/glm-5".into(),
        Model {
            id: "z-ai/glm-5".into(),
            name: "Z.ai: GLM 5".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.6,
                output: 1.9,
                cache_read: 0.119,
                cache_write: 0.0,
            },
            context_window: 202752,
            max_tokens: 4096,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "z-ai/glm-5-turbo".into(),
        Model {
            id: "z-ai/glm-5-turbo".into(),
            name: "Z.ai: GLM 5 Turbo".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 1.2,
                output: 4.0,
                cache_read: 0.24,
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
        "z-ai/glm-5.1".into(),
        Model {
            id: "z-ai/glm-5.1".into(),
            name: "Z.ai: GLM 5.1".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.98,
                output: 3.08,
                cache_read: 0.182,
                cache_write: 0.0,
            },
            context_window: 202752,
            max_tokens: 4096,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "z-ai/glm-5.2".into(),
        Model {
            id: "z-ai/glm-5.2".into(),
            name: "Z.ai: GLM 5.2".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.95,
                output: 3.0,
                cache_read: 0.18,
                cache_write: 0.0,
            },
            context_window: 1048576,
            max_tokens: 32768,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "z-ai/glm-5v-turbo".into(),
        Model {
            id: "z-ai/glm-5v-turbo".into(),
            name: "Z.ai: GLM 5V Turbo".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 1.2,
                output: 4.0,
                cache_read: 0.24,
                cache_write: 0.0,
            },
            context_window: 202752,
            max_tokens: 131072,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "~anthropic/claude-fable-latest".into(),
        Model {
            id: "~anthropic/claude-fable-latest".into(),
            name: "Anthropic: Claude Fable Latest".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 10.0,
                output: 50.0,
                cache_read: 1.0,
                cache_write: 12.5,
            },
            context_window: 1000000,
            max_tokens: 128000,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "~anthropic/claude-haiku-latest".into(),
        Model {
            id: "~anthropic/claude-haiku-latest".into(),
            name: "Anthropic Claude Haiku Latest".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
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
        "~anthropic/claude-opus-latest".into(),
        Model {
            id: "~anthropic/claude-opus-latest".into(),
            name: "Anthropic: Claude Opus Latest".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
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
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "~anthropic/claude-sonnet-latest".into(),
        Model {
            id: "~anthropic/claude-sonnet-latest".into(),
            name: "Anthropic Claude Sonnet Latest".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
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
        "~google/gemini-flash-latest".into(),
        Model {
            id: "~google/gemini-flash-latest".into(),
            name: "Google Gemini Flash Latest".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 1.5,
                output: 9.0,
                cache_read: 0.15,
                cache_write: 0.083333,
            },
            context_window: 1048576,
            max_tokens: 65536,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "~google/gemini-pro-latest".into(),
        Model {
            id: "~google/gemini-pro-latest".into(),
            name: "Google Gemini Pro Latest".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 2.0,
                output: 12.0,
                cache_read: 0.2,
                cache_write: 0.375,
            },
            context_window: 1048576,
            max_tokens: 65536,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "~moonshotai/kimi-latest".into(),
        Model {
            id: "~moonshotai/kimi-latest".into(),
            name: "MoonshotAI Kimi Latest".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.66,
                output: 3.41,
                cache_read: 0.144,
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
        "~openai/gpt-latest".into(),
        Model {
            id: "~openai/gpt-latest".into(),
            name: "OpenAI GPT Latest".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 5.0,
                output: 30.0,
                cache_read: 0.5,
                cache_write: 0.0,
            },
            context_window: 1050000,
            max_tokens: 128000,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "~openai/gpt-mini-latest".into(),
        Model {
            id: "~openai/gpt-mini-latest".into(),
            name: "OpenAI GPT Mini Latest".into(),
            api: Api::OpenAiCompletions,
            provider: "openrouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
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
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map
}
