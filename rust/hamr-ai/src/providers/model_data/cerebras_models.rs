//! Auto-generated model catalogue for `cerebras`.
//! Do not edit manually — re-run `scripts/generate_models.py`.

use std::collections::HashMap;

use crate::types::{Api, InputModality, Model, ModelCost};

/// Model catalogue for the `cerebras` provider.
pub fn models() -> HashMap<String, Model> {
    let mut map: HashMap<String, Model> = HashMap::new();

    map.insert(
        "gpt-oss-120b".into(),
        Model {
            id: "gpt-oss-120b".into(),
            name: "GPT OSS 120B".into(),
            api: Api::OpenAiCompletions,
            provider: "cerebras".into(),
            base_url: "https://api.cerebras.ai/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.35,
                output: 0.75,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 131072,
            max_tokens: 40960,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "zai-glm-4.7".into(),
        Model {
            id: "zai-glm-4.7".into(),
            name: "Z.AI GLM-4.7".into(),
            api: Api::OpenAiCompletions,
            provider: "cerebras".into(),
            base_url: "https://api.cerebras.ai/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 2.25,
                output: 2.75,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 131072,
            max_tokens: 40960,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map
}
