//! Auto-generated model catalogue for `deepseek`.
//! Do not edit manually — re-run `scripts/generate_models.py`.

use std::collections::HashMap;

use crate::types::{Api, InputModality, Model, ModelCost, ModelThinkingLevel};

/// Model catalogue for the `deepseek` provider.
pub fn models() -> HashMap<String, Model> {
    let mut map: HashMap<String, Model> = HashMap::new();

    map.insert(
        "deepseek-v4-flash".into(),
        Model {
            id: "deepseek-v4-flash".into(),
            name: "DeepSeek V4 Flash".into(),
            api: Api::OpenAiCompletions,
            provider: "deepseek".into(),
            base_url: "https://api.deepseek.com".into(),
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
            provider: "deepseek".into(),
            base_url: "https://api.deepseek.com".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.435,
                output: 0.87,
                cache_read: 0.003625,
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

    map
}
