//! Auto-generated model catalogue for `moonshotai-cn`.
//! Do not edit manually — re-run `scripts/generate_models.py`.

use std::collections::HashMap;

use crate::types::{Api, InputModality, Model, ModelCost, ModelThinkingLevel};

/// Model catalogue for the `moonshotai-cn` provider.
pub fn models() -> HashMap<String, Model> {
    let mut map: HashMap<String, Model> = HashMap::new();

    map.insert(
        "kimi-k2-0711-preview".into(),
        Model {
            id: "kimi-k2-0711-preview".into(),
            name: "Kimi K2 0711".into(),
            api: Api::OpenAiCompletions,
            provider: "moonshotai-cn".into(),
            base_url: "https://api.moonshot.cn/v1".into(),
            reasoning: false,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.6,
                output: 2.5,
                cache_read: 0.15,
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
        "kimi-k2-0905-preview".into(),
        Model {
            id: "kimi-k2-0905-preview".into(),
            name: "Kimi K2 0905".into(),
            api: Api::OpenAiCompletions,
            provider: "moonshotai-cn".into(),
            base_url: "https://api.moonshot.cn/v1".into(),
            reasoning: false,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.6,
                output: 2.5,
                cache_read: 0.15,
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
        "kimi-k2-thinking".into(),
        Model {
            id: "kimi-k2-thinking".into(),
            name: "Kimi K2 Thinking".into(),
            api: Api::OpenAiCompletions,
            provider: "moonshotai-cn".into(),
            base_url: "https://api.moonshot.cn/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.6,
                output: 2.5,
                cache_read: 0.15,
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
        "kimi-k2-thinking-turbo".into(),
        Model {
            id: "kimi-k2-thinking-turbo".into(),
            name: "Kimi K2 Thinking Turbo".into(),
            api: Api::OpenAiCompletions,
            provider: "moonshotai-cn".into(),
            base_url: "https://api.moonshot.cn/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 1.15,
                output: 8.0,
                cache_read: 0.15,
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
        "kimi-k2-turbo-preview".into(),
        Model {
            id: "kimi-k2-turbo-preview".into(),
            name: "Kimi K2 Turbo".into(),
            api: Api::OpenAiCompletions,
            provider: "moonshotai-cn".into(),
            base_url: "https://api.moonshot.cn/v1".into(),
            reasoning: false,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 2.4,
                output: 10.0,
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
        "kimi-k2.5".into(),
        Model {
            id: "kimi-k2.5".into(),
            name: "Kimi K2.5".into(),
            api: Api::OpenAiCompletions,
            provider: "moonshotai-cn".into(),
            base_url: "https://api.moonshot.cn/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.6,
                output: 3.0,
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
        "kimi-k2.6".into(),
        Model {
            id: "kimi-k2.6".into(),
            name: "Kimi K2.6".into(),
            api: Api::OpenAiCompletions,
            provider: "moonshotai-cn".into(),
            base_url: "https://api.moonshot.cn/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.95,
                output: 4.0,
                cache_read: 0.16,
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
        "kimi-k2.7-code".into(),
        Model {
            id: "kimi-k2.7-code".into(),
            name: "Kimi K2.7 Code".into(),
            api: Api::OpenAiCompletions,
            provider: "moonshotai-cn".into(),
            base_url: "https://api.moonshot.cn/v1".into(),
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
            thinking_level_map: Some(HashMap::from([(ModelThinkingLevel::Off, None)])),
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "kimi-k2.7-code-highspeed".into(),
        Model {
            id: "kimi-k2.7-code-highspeed".into(),
            name: "Kimi K2.7 Code HighSpeed".into(),
            api: Api::OpenAiCompletions,
            provider: "moonshotai-cn".into(),
            base_url: "https://api.moonshot.cn/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 1.9,
                output: 8.0,
                cache_read: 0.38,
                cache_write: 0.0,
            },
            context_window: 262144,
            max_tokens: 262144,
            thinking_level_map: Some(HashMap::from([(ModelThinkingLevel::Off, None)])),
            headers: None,
            compat: None,
        },
    );

    map
}
