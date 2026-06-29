//! Auto-generated model catalogue for `openai-codex`.
//! Do not edit manually — re-run `scripts/generate_models.py`.

use std::collections::HashMap;

use crate::types::{Api, InputModality, Model, ModelCost, ModelThinkingLevel};

/// Model catalogue for the `openai-codex` provider.
pub fn models() -> HashMap<String, Model> {
    let mut map: HashMap<String, Model> = HashMap::new();

    map.insert(
        "gpt-5.3-codex-spark".into(),
        Model {
            id: "gpt-5.3-codex-spark".into(),
            name: "GPT-5.3 Codex Spark".into(),
            api: Api::OpenAiCodexResponses,
            provider: "openai-codex".into(),
            base_url: "https://chatgpt.com/backend-api".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 1.75,
                output: 14.0,
                cache_read: 0.175,
                cache_write: 0.0,
            },
            context_window: 128000,
            max_tokens: 128000,
            thinking_level_map: Some(HashMap::from([
                (ModelThinkingLevel::XHigh, Some("xhigh".into())),
                (ModelThinkingLevel::Minimal, Some("low".into())),
            ])),
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "gpt-5.4".into(),
        Model {
            id: "gpt-5.4".into(),
            name: "GPT-5.4".into(),
            api: Api::OpenAiCodexResponses,
            provider: "openai-codex".into(),
            base_url: "https://chatgpt.com/backend-api".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 2.5,
                output: 15.0,
                cache_read: 0.25,
                cache_write: 0.0,
            },
            context_window: 272000,
            max_tokens: 128000,
            thinking_level_map: Some(HashMap::from([
                (ModelThinkingLevel::XHigh, Some("xhigh".into())),
                (ModelThinkingLevel::Minimal, Some("low".into())),
            ])),
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "gpt-5.4-mini".into(),
        Model {
            id: "gpt-5.4-mini".into(),
            name: "GPT-5.4 mini".into(),
            api: Api::OpenAiCodexResponses,
            provider: "openai-codex".into(),
            base_url: "https://chatgpt.com/backend-api".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.75,
                output: 4.5,
                cache_read: 0.075,
                cache_write: 0.0,
            },
            context_window: 272000,
            max_tokens: 128000,
            thinking_level_map: Some(HashMap::from([
                (ModelThinkingLevel::XHigh, Some("xhigh".into())),
                (ModelThinkingLevel::Minimal, Some("low".into())),
            ])),
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "gpt-5.5".into(),
        Model {
            id: "gpt-5.5".into(),
            name: "GPT-5.5".into(),
            api: Api::OpenAiCodexResponses,
            provider: "openai-codex".into(),
            base_url: "https://chatgpt.com/backend-api".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 5.0,
                output: 30.0,
                cache_read: 0.5,
                cache_write: 0.0,
            },
            context_window: 272000,
            max_tokens: 128000,
            thinking_level_map: Some(HashMap::from([
                (ModelThinkingLevel::XHigh, Some("xhigh".into())),
                (ModelThinkingLevel::Minimal, Some("low".into())),
            ])),
            headers: None,
            compat: None,
        },
    );

    map
}
