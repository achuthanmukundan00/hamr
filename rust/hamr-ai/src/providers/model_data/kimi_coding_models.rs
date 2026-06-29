//! Auto-generated model catalogue for `kimi-coding`.
//! Do not edit manually — re-run `scripts/generate_models.py`.

use std::collections::HashMap;

use crate::types::{Api, InputModality, Model, ModelCost};

/// Model catalogue for the `kimi-coding` provider.
pub fn models() -> HashMap<String, Model> {
    let mut map: HashMap<String, Model> = HashMap::new();

    map.insert(
        "kimi-for-coding".into(),
        Model {
            id: "kimi-for-coding".into(),
            name: "Kimi For Coding".into(),
            api: Api::AnthropicMessages,
            provider: "kimi-coding".into(),
            base_url: "https://api.kimi.com/coding".into(),
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
            headers: Some(HashMap::from([("User-Agent".into(), "KimiCLI/1.5".into())])),
            compat: None,
        },
    );

    map.insert(
        "kimi-k2-thinking".into(),
        Model {
            id: "kimi-k2-thinking".into(),
            name: "Kimi K2 Thinking".into(),
            api: Api::AnthropicMessages,
            provider: "kimi-coding".into(),
            base_url: "https://api.kimi.com/coding".into(),
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
            headers: Some(HashMap::from([("User-Agent".into(), "KimiCLI/1.5".into())])),
            compat: None,
        },
    );

    map
}
