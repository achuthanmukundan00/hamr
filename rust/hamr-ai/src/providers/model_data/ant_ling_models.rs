//! Auto-generated model catalogue for `ant-ling`.
//! Do not edit manually — re-run `scripts/generate_models.py`.

use std::collections::HashMap;

use crate::types::{Api, InputModality, Model, ModelCost, ModelThinkingLevel};

/// Model catalogue for the `ant-ling` provider.
pub fn models() -> HashMap<String, Model> {
    let mut map: HashMap<String, Model> = HashMap::new();

    map.insert(
        "Ling-2.6-1T".into(),
        Model {
            id: "Ling-2.6-1T".into(),
            name: "Ling 2.6 1T".into(),
            api: Api::OpenAiCompletions,
            provider: "ant-ling".into(),
            base_url: "https://api.ant-ling.com/v1".into(),
            reasoning: false,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.06,
                output: 0.25,
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
        "Ling-2.6-flash".into(),
        Model {
            id: "Ling-2.6-flash".into(),
            name: "Ling 2.6 Flash".into(),
            api: Api::OpenAiCompletions,
            provider: "ant-ling".into(),
            base_url: "https://api.ant-ling.com/v1".into(),
            reasoning: false,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.01,
                output: 0.02,
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
        "Ring-2.6-1T".into(),
        Model {
            id: "Ring-2.6-1T".into(),
            name: "Ring 2.6 1T".into(),
            api: Api::OpenAiCompletions,
            provider: "ant-ling".into(),
            base_url: "https://api.ant-ling.com/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.06,
                output: 0.25,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 262144,
            max_tokens: 65536,
            thinking_level_map: Some(HashMap::from([
                (ModelThinkingLevel::Off, None),
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

    map
}
