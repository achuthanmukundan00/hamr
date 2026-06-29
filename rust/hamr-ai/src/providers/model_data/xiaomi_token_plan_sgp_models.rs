//! Auto-generated model catalogue for `xiaomi-token-plan-sgp`.
//! Do not edit manually — re-run `scripts/generate_models.py`.

use std::collections::HashMap;

use crate::types::{Api, InputModality, Model, ModelCost};

/// Model catalogue for the `xiaomi-token-plan-sgp` provider.
pub fn models() -> HashMap<String, Model> {
    let mut map: HashMap<String, Model> = HashMap::new();

    map.insert(
        "mimo-v2-omni".into(),
        Model {
            id: "mimo-v2-omni".into(),
            name: "MiMo-V2-Omni".into(),
            api: Api::OpenAiCompletions,
            provider: "xiaomi-token-plan-sgp".into(),
            base_url: "https://token-plan-sgp.xiaomimimo.com/v1".into(),
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
            provider: "xiaomi-token-plan-sgp".into(),
            base_url: "https://token-plan-sgp.xiaomimimo.com/v1".into(),
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
            provider: "xiaomi-token-plan-sgp".into(),
            base_url: "https://token-plan-sgp.xiaomimimo.com/v1".into(),
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
            provider: "xiaomi-token-plan-sgp".into(),
            base_url: "https://token-plan-sgp.xiaomimimo.com/v1".into(),
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
            provider: "xiaomi-token-plan-sgp".into(),
            base_url: "https://token-plan-sgp.xiaomimimo.com/v1".into(),
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
