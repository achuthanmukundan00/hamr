//! Auto-generated model catalogue for `xiaomi`.
//! Do not edit manually — re-run `scripts/generate_models.py`.

use std::collections::HashMap;

use crate::types::{Api, InputModality, Model, ModelCost};

/// Model catalogue for the `xiaomi` provider.
pub fn models() -> HashMap<String, Model> {
    let mut map: HashMap<String, Model> = HashMap::new();

    map.insert(
        "mimo-v2-flash".into(),
        Model {
            id: "mimo-v2-flash".into(),
            name: "MiMo-V2-Flash".into(),
            api: Api::OpenAiCompletions,
            provider: "xiaomi".into(),
            base_url: "https://api.xiaomimimo.com/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.1,
                output: 0.3,
                cache_read: 0.01,
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
        "mimo-v2-omni".into(),
        Model {
            id: "mimo-v2-omni".into(),
            name: "MiMo-V2-Omni".into(),
            api: Api::OpenAiCompletions,
            provider: "xiaomi".into(),
            base_url: "https://api.xiaomimimo.com/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.4,
                output: 2.0,
                cache_read: 0.08,
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
        "mimo-v2-pro".into(),
        Model {
            id: "mimo-v2-pro".into(),
            name: "MiMo-V2-Pro".into(),
            api: Api::OpenAiCompletions,
            provider: "xiaomi".into(),
            base_url: "https://api.xiaomimimo.com/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 1.0,
                output: 3.0,
                cache_read: 0.2,
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
        "mimo-v2.5".into(),
        Model {
            id: "mimo-v2.5".into(),
            name: "MiMo-V2.5".into(),
            api: Api::OpenAiCompletions,
            provider: "xiaomi".into(),
            base_url: "https://api.xiaomimimo.com/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.4,
                output: 2.0,
                cache_read: 0.08,
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
        "mimo-v2.5-pro".into(),
        Model {
            id: "mimo-v2.5-pro".into(),
            name: "MiMo-V2.5-Pro".into(),
            api: Api::OpenAiCompletions,
            provider: "xiaomi".into(),
            base_url: "https://api.xiaomimimo.com/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 1.0,
                output: 3.0,
                cache_read: 0.2,
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
        "mimo-v2.5-pro-ultraspeed".into(),
        Model {
            id: "mimo-v2.5-pro-ultraspeed".into(),
            name: "MiMo-V2.5-Pro-UltraSpeed".into(),
            api: Api::OpenAiCompletions,
            provider: "xiaomi".into(),
            base_url: "https://api.xiaomimimo.com/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 1.305,
                output: 2.61,
                cache_read: 0.0108,
                cache_write: 0.0,
            },
            context_window: 1048576,
            max_tokens: 131072,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map
}
