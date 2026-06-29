//! Auto-generated model catalogue for `xai`.
//! Do not edit manually — re-run `scripts/generate_models.py`.

use std::collections::HashMap;

use crate::types::{Api, InputModality, Model, ModelCost};

/// Model catalogue for the `xai` provider.
pub fn models() -> HashMap<String, Model> {
    let mut map: HashMap<String, Model> = HashMap::new();

    map.insert(
        "grok-3".into(),
        Model {
            id: "grok-3".into(),
            name: "Grok 3".into(),
            api: Api::OpenAiCompletions,
            provider: "xai".into(),
            base_url: "https://api.x.ai/v1".into(),
            reasoning: false,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 3.0,
                output: 15.0,
                cache_read: 0.75,
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
        "grok-3-fast".into(),
        Model {
            id: "grok-3-fast".into(),
            name: "Grok 3 Fast".into(),
            api: Api::OpenAiCompletions,
            provider: "xai".into(),
            base_url: "https://api.x.ai/v1".into(),
            reasoning: false,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 5.0,
                output: 25.0,
                cache_read: 1.25,
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
        "grok-4.20-0309-non-reasoning".into(),
        Model {
            id: "grok-4.20-0309-non-reasoning".into(),
            name: "Grok 4.20 (Non-Reasoning)".into(),
            api: Api::OpenAiCompletions,
            provider: "xai".into(),
            base_url: "https://api.x.ai/v1".into(),
            reasoning: false,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 1.25,
                output: 2.5,
                cache_read: 0.2,
                cache_write: 0.0,
            },
            context_window: 1000000,
            max_tokens: 30000,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "grok-4.20-0309-reasoning".into(),
        Model {
            id: "grok-4.20-0309-reasoning".into(),
            name: "Grok 4.20 (Reasoning)".into(),
            api: Api::OpenAiCompletions,
            provider: "xai".into(),
            base_url: "https://api.x.ai/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 1.25,
                output: 2.5,
                cache_read: 0.2,
                cache_write: 0.0,
            },
            context_window: 1000000,
            max_tokens: 30000,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "grok-4.3".into(),
        Model {
            id: "grok-4.3".into(),
            name: "Grok 4.3".into(),
            api: Api::OpenAiCompletions,
            provider: "xai".into(),
            base_url: "https://api.x.ai/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 1.25,
                output: 2.5,
                cache_read: 0.2,
                cache_write: 0.0,
            },
            context_window: 1000000,
            max_tokens: 30000,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "grok-build-0.1".into(),
        Model {
            id: "grok-build-0.1".into(),
            name: "Grok Build 0.1".into(),
            api: Api::OpenAiCompletions,
            provider: "xai".into(),
            base_url: "https://api.x.ai/v1".into(),
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
        "grok-code-fast-1".into(),
        Model {
            id: "grok-code-fast-1".into(),
            name: "Grok Code Fast 1".into(),
            api: Api::OpenAiCompletions,
            provider: "xai".into(),
            base_url: "https://api.x.ai/v1".into(),
            reasoning: false,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.2,
                output: 1.5,
                cache_read: 0.02,
                cache_write: 0.0,
            },
            context_window: 32768,
            max_tokens: 8192,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map
}
