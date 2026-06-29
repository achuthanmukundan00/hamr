//! Auto-generated model catalogue for `minimax`.
//! Do not edit manually — re-run `scripts/generate_models.py`.

use std::collections::HashMap;

use crate::types::{Api, InputModality, Model, ModelCost};

/// Model catalogue for the `minimax` provider.
pub fn models() -> HashMap<String, Model> {
    let mut map: HashMap<String, Model> = HashMap::new();

    map.insert(
        "MiniMax-M2.7".into(),
        Model {
            id: "MiniMax-M2.7".into(),
            name: "MiniMax-M2.7".into(),
            api: Api::AnthropicMessages,
            provider: "minimax".into(),
            base_url: "https://api.minimax.io/anthropic".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.3,
                output: 1.2,
                cache_read: 0.06,
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
        "MiniMax-M2.7-highspeed".into(),
        Model {
            id: "MiniMax-M2.7-highspeed".into(),
            name: "MiniMax-M2.7-highspeed".into(),
            api: Api::AnthropicMessages,
            provider: "minimax".into(),
            base_url: "https://api.minimax.io/anthropic".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.6,
                output: 2.4,
                cache_read: 0.06,
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
        "MiniMax-M3".into(),
        Model {
            id: "MiniMax-M3".into(),
            name: "MiniMax-M3".into(),
            api: Api::AnthropicMessages,
            provider: "minimax".into(),
            base_url: "https://api.minimax.io/anthropic".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.3,
                output: 1.2,
                cache_read: 0.06,
                cache_write: 0.0,
            },
            context_window: 1000000,
            max_tokens: 128000,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map
}
