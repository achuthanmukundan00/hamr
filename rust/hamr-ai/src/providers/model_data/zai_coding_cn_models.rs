//! Auto-generated model catalogue for `zai-coding-cn`.
//! Do not edit manually — re-run `scripts/generate_models.py`.

use std::collections::HashMap;

use crate::types::{Api, InputModality, Model, ModelCost, ModelThinkingLevel};

/// Model catalogue for the `zai-coding-cn` provider.
pub fn models() -> HashMap<String, Model> {
    let mut map: HashMap<String, Model> = HashMap::new();

    map.insert(
        "glm-4.5-air".into(),
        Model {
            id: "glm-4.5-air".into(),
            name: "GLM-4.5-Air".into(),
            api: Api::OpenAiCompletions,
            provider: "zai-coding-cn".into(),
            base_url: "https://open.bigmodel.cn/api/coding/paas/v4".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.0,
                output: 0.0,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 131072,
            max_tokens: 98304,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "glm-4.7".into(),
        Model {
            id: "glm-4.7".into(),
            name: "GLM-4.7".into(),
            api: Api::OpenAiCompletions,
            provider: "zai-coding-cn".into(),
            base_url: "https://open.bigmodel.cn/api/coding/paas/v4".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.0,
                output: 0.0,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 204800,
            max_tokens: 131072,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "glm-5-turbo".into(),
        Model {
            id: "glm-5-turbo".into(),
            name: "GLM-5-Turbo".into(),
            api: Api::OpenAiCompletions,
            provider: "zai-coding-cn".into(),
            base_url: "https://open.bigmodel.cn/api/coding/paas/v4".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.0,
                output: 0.0,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 200000,
            max_tokens: 131072,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "glm-5.1".into(),
        Model {
            id: "glm-5.1".into(),
            name: "GLM-5.1".into(),
            api: Api::OpenAiCompletions,
            provider: "zai-coding-cn".into(),
            base_url: "https://open.bigmodel.cn/api/coding/paas/v4".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.0,
                output: 0.0,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 200000,
            max_tokens: 131072,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "glm-5.2".into(),
        Model {
            id: "glm-5.2".into(),
            name: "GLM-5.2".into(),
            api: Api::OpenAiCompletions,
            provider: "zai-coding-cn".into(),
            base_url: "https://open.bigmodel.cn/api/coding/paas/v4".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.0,
                output: 0.0,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 1000000,
            max_tokens: 131072,
            thinking_level_map: Some(HashMap::from([
                (ModelThinkingLevel::Minimal, None),
                (ModelThinkingLevel::Low, Some("high".into())),
                (ModelThinkingLevel::Medium, Some("high".into())),
                (ModelThinkingLevel::High, Some("high".into())),
                (ModelThinkingLevel::XHigh, Some("max".into())),
            ])),
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "glm-5v-turbo".into(),
        Model {
            id: "glm-5v-turbo".into(),
            name: "GLM-5V-Turbo".into(),
            api: Api::OpenAiCompletions,
            provider: "zai-coding-cn".into(),
            base_url: "https://open.bigmodel.cn/api/coding/paas/v4".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.0,
                output: 0.0,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 200000,
            max_tokens: 131072,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map
}
