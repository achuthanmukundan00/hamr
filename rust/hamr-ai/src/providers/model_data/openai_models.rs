//! Auto-generated model catalogue for `openai`.
//! Do not edit manually — re-run `scripts/generate_models.py`.

use std::collections::HashMap;

use crate::types::{Api, InputModality, Model, ModelCost, ModelThinkingLevel};

/// Model catalogue for the `openai` provider.
pub fn models() -> HashMap<String, Model> {
    let mut map: HashMap<String, Model> = HashMap::new();

    map.insert(
        "gpt-4".into(),
        Model {
            id: "gpt-4".into(),
            name: "GPT-4".into(),
            api: Api::OpenAiResponses,
            provider: "openai".into(),
            base_url: "https://api.openai.com/v1".into(),
            reasoning: false,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 30.0,
                output: 60.0,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 8192,
            max_tokens: 8192,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "gpt-4-turbo".into(),
        Model {
            id: "gpt-4-turbo".into(),
            name: "GPT-4 Turbo".into(),
            api: Api::OpenAiResponses,
            provider: "openai".into(),
            base_url: "https://api.openai.com/v1".into(),
            reasoning: false,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 10.0,
                output: 30.0,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 128000,
            max_tokens: 4096,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "gpt-4.1".into(),
        Model {
            id: "gpt-4.1".into(),
            name: "GPT-4.1".into(),
            api: Api::OpenAiResponses,
            provider: "openai".into(),
            base_url: "https://api.openai.com/v1".into(),
            reasoning: false,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 2.0,
                output: 8.0,
                cache_read: 0.5,
                cache_write: 0.0,
            },
            context_window: 1047576,
            max_tokens: 32768,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "gpt-4.1-mini".into(),
        Model {
            id: "gpt-4.1-mini".into(),
            name: "GPT-4.1 mini".into(),
            api: Api::OpenAiResponses,
            provider: "openai".into(),
            base_url: "https://api.openai.com/v1".into(),
            reasoning: false,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.4,
                output: 1.6,
                cache_read: 0.1,
                cache_write: 0.0,
            },
            context_window: 1047576,
            max_tokens: 32768,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "gpt-4.1-nano".into(),
        Model {
            id: "gpt-4.1-nano".into(),
            name: "GPT-4.1 nano".into(),
            api: Api::OpenAiResponses,
            provider: "openai".into(),
            base_url: "https://api.openai.com/v1".into(),
            reasoning: false,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.1,
                output: 0.4,
                cache_read: 0.025,
                cache_write: 0.0,
            },
            context_window: 1047576,
            max_tokens: 32768,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "gpt-4o".into(),
        Model {
            id: "gpt-4o".into(),
            name: "GPT-4o".into(),
            api: Api::OpenAiResponses,
            provider: "openai".into(),
            base_url: "https://api.openai.com/v1".into(),
            reasoning: false,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 2.5,
                output: 10.0,
                cache_read: 1.25,
                cache_write: 0.0,
            },
            context_window: 128000,
            max_tokens: 16384,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "gpt-4o-2024-05-13".into(),
        Model {
            id: "gpt-4o-2024-05-13".into(),
            name: "GPT-4o (2024-05-13)".into(),
            api: Api::OpenAiResponses,
            provider: "openai".into(),
            base_url: "https://api.openai.com/v1".into(),
            reasoning: false,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 5.0,
                output: 15.0,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 128000,
            max_tokens: 4096,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "gpt-4o-2024-08-06".into(),
        Model {
            id: "gpt-4o-2024-08-06".into(),
            name: "GPT-4o (2024-08-06)".into(),
            api: Api::OpenAiResponses,
            provider: "openai".into(),
            base_url: "https://api.openai.com/v1".into(),
            reasoning: false,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 2.5,
                output: 10.0,
                cache_read: 1.25,
                cache_write: 0.0,
            },
            context_window: 128000,
            max_tokens: 16384,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "gpt-4o-2024-11-20".into(),
        Model {
            id: "gpt-4o-2024-11-20".into(),
            name: "GPT-4o (2024-11-20)".into(),
            api: Api::OpenAiResponses,
            provider: "openai".into(),
            base_url: "https://api.openai.com/v1".into(),
            reasoning: false,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 2.5,
                output: 10.0,
                cache_read: 1.25,
                cache_write: 0.0,
            },
            context_window: 128000,
            max_tokens: 16384,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "gpt-4o-mini".into(),
        Model {
            id: "gpt-4o-mini".into(),
            name: "GPT-4o mini".into(),
            api: Api::OpenAiResponses,
            provider: "openai".into(),
            base_url: "https://api.openai.com/v1".into(),
            reasoning: false,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.15,
                output: 0.6,
                cache_read: 0.075,
                cache_write: 0.0,
            },
            context_window: 128000,
            max_tokens: 16384,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "gpt-5".into(),
        Model {
            id: "gpt-5".into(),
            name: "GPT-5".into(),
            api: Api::OpenAiResponses,
            provider: "openai".into(),
            base_url: "https://api.openai.com/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 1.25,
                output: 10.0,
                cache_read: 0.125,
                cache_write: 0.0,
            },
            context_window: 400000,
            max_tokens: 128000,
            thinking_level_map: Some(HashMap::from([(ModelThinkingLevel::Off, None)])),
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "gpt-5-chat-latest".into(),
        Model {
            id: "gpt-5-chat-latest".into(),
            name: "GPT-5 Chat Latest".into(),
            api: Api::OpenAiResponses,
            provider: "openai".into(),
            base_url: "https://api.openai.com/v1".into(),
            reasoning: false,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 1.25,
                output: 10.0,
                cache_read: 0.125,
                cache_write: 0.0,
            },
            context_window: 128000,
            max_tokens: 16384,
            thinking_level_map: Some(HashMap::from([(ModelThinkingLevel::Off, None)])),
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "gpt-5-codex".into(),
        Model {
            id: "gpt-5-codex".into(),
            name: "GPT-5-Codex".into(),
            api: Api::OpenAiResponses,
            provider: "openai".into(),
            base_url: "https://api.openai.com/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 1.25,
                output: 10.0,
                cache_read: 0.125,
                cache_write: 0.0,
            },
            context_window: 400000,
            max_tokens: 128000,
            thinking_level_map: Some(HashMap::from([(ModelThinkingLevel::Off, None)])),
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "gpt-5-mini".into(),
        Model {
            id: "gpt-5-mini".into(),
            name: "GPT-5 Mini".into(),
            api: Api::OpenAiResponses,
            provider: "openai".into(),
            base_url: "https://api.openai.com/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.25,
                output: 2.0,
                cache_read: 0.025,
                cache_write: 0.0,
            },
            context_window: 400000,
            max_tokens: 128000,
            thinking_level_map: Some(HashMap::from([(ModelThinkingLevel::Off, None)])),
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "gpt-5-nano".into(),
        Model {
            id: "gpt-5-nano".into(),
            name: "GPT-5 Nano".into(),
            api: Api::OpenAiResponses,
            provider: "openai".into(),
            base_url: "https://api.openai.com/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.05,
                output: 0.4,
                cache_read: 0.005,
                cache_write: 0.0,
            },
            context_window: 400000,
            max_tokens: 128000,
            thinking_level_map: Some(HashMap::from([(ModelThinkingLevel::Off, None)])),
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "gpt-5-pro".into(),
        Model {
            id: "gpt-5-pro".into(),
            name: "GPT-5 Pro".into(),
            api: Api::OpenAiResponses,
            provider: "openai".into(),
            base_url: "https://api.openai.com/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 15.0,
                output: 120.0,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 400000,
            max_tokens: 128000,
            thinking_level_map: Some(HashMap::from([(ModelThinkingLevel::Off, None)])),
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "gpt-5.1".into(),
        Model {
            id: "gpt-5.1".into(),
            name: "GPT-5.1".into(),
            api: Api::OpenAiResponses,
            provider: "openai".into(),
            base_url: "https://api.openai.com/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 1.25,
                output: 10.0,
                cache_read: 0.125,
                cache_write: 0.0,
            },
            context_window: 400000,
            max_tokens: 128000,
            thinking_level_map: Some(HashMap::from([(
                ModelThinkingLevel::Off,
                Some("none".into()),
            )])),
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "gpt-5.1-chat-latest".into(),
        Model {
            id: "gpt-5.1-chat-latest".into(),
            name: "GPT-5.1 Chat".into(),
            api: Api::OpenAiResponses,
            provider: "openai".into(),
            base_url: "https://api.openai.com/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 1.25,
                output: 10.0,
                cache_read: 0.125,
                cache_write: 0.0,
            },
            context_window: 128000,
            max_tokens: 16384,
            thinking_level_map: Some(HashMap::from([(ModelThinkingLevel::Off, None)])),
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "gpt-5.1-codex".into(),
        Model {
            id: "gpt-5.1-codex".into(),
            name: "GPT-5.1 Codex".into(),
            api: Api::OpenAiResponses,
            provider: "openai".into(),
            base_url: "https://api.openai.com/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 1.25,
                output: 10.0,
                cache_read: 0.125,
                cache_write: 0.0,
            },
            context_window: 400000,
            max_tokens: 128000,
            thinking_level_map: Some(HashMap::from([(ModelThinkingLevel::Off, None)])),
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "gpt-5.1-codex-max".into(),
        Model {
            id: "gpt-5.1-codex-max".into(),
            name: "GPT-5.1 Codex Max".into(),
            api: Api::OpenAiResponses,
            provider: "openai".into(),
            base_url: "https://api.openai.com/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 1.25,
                output: 10.0,
                cache_read: 0.125,
                cache_write: 0.0,
            },
            context_window: 400000,
            max_tokens: 128000,
            thinking_level_map: Some(HashMap::from([(ModelThinkingLevel::Off, None)])),
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "gpt-5.1-codex-mini".into(),
        Model {
            id: "gpt-5.1-codex-mini".into(),
            name: "GPT-5.1 Codex mini".into(),
            api: Api::OpenAiResponses,
            provider: "openai".into(),
            base_url: "https://api.openai.com/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.25,
                output: 2.0,
                cache_read: 0.025,
                cache_write: 0.0,
            },
            context_window: 400000,
            max_tokens: 128000,
            thinking_level_map: Some(HashMap::from([(ModelThinkingLevel::Off, None)])),
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "gpt-5.2".into(),
        Model {
            id: "gpt-5.2".into(),
            name: "GPT-5.2".into(),
            api: Api::OpenAiResponses,
            provider: "openai".into(),
            base_url: "https://api.openai.com/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 1.75,
                output: 14.0,
                cache_read: 0.175,
                cache_write: 0.0,
            },
            context_window: 400000,
            max_tokens: 128000,
            thinking_level_map: Some(HashMap::from([
                (ModelThinkingLevel::Off, Some("none".into())),
                (ModelThinkingLevel::XHigh, Some("xhigh".into())),
            ])),
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "gpt-5.2-chat-latest".into(),
        Model {
            id: "gpt-5.2-chat-latest".into(),
            name: "GPT-5.2 Chat".into(),
            api: Api::OpenAiResponses,
            provider: "openai".into(),
            base_url: "https://api.openai.com/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 1.75,
                output: 14.0,
                cache_read: 0.175,
                cache_write: 0.0,
            },
            context_window: 128000,
            max_tokens: 16384,
            thinking_level_map: Some(HashMap::from([
                (ModelThinkingLevel::Off, None),
                (ModelThinkingLevel::XHigh, Some("xhigh".into())),
            ])),
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "gpt-5.2-codex".into(),
        Model {
            id: "gpt-5.2-codex".into(),
            name: "GPT-5.2 Codex".into(),
            api: Api::OpenAiResponses,
            provider: "openai".into(),
            base_url: "https://api.openai.com/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 1.75,
                output: 14.0,
                cache_read: 0.175,
                cache_write: 0.0,
            },
            context_window: 400000,
            max_tokens: 128000,
            thinking_level_map: Some(HashMap::from([
                (ModelThinkingLevel::Off, None),
                (ModelThinkingLevel::XHigh, Some("xhigh".into())),
            ])),
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "gpt-5.2-pro".into(),
        Model {
            id: "gpt-5.2-pro".into(),
            name: "GPT-5.2 Pro".into(),
            api: Api::OpenAiResponses,
            provider: "openai".into(),
            base_url: "https://api.openai.com/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 21.0,
                output: 168.0,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 400000,
            max_tokens: 128000,
            thinking_level_map: Some(HashMap::from([
                (ModelThinkingLevel::Off, None),
                (ModelThinkingLevel::XHigh, Some("xhigh".into())),
            ])),
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "gpt-5.3-chat-latest".into(),
        Model {
            id: "gpt-5.3-chat-latest".into(),
            name: "GPT-5.3 Chat (latest)".into(),
            api: Api::OpenAiResponses,
            provider: "openai".into(),
            base_url: "https://api.openai.com/v1".into(),
            reasoning: false,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 1.75,
                output: 14.0,
                cache_read: 0.175,
                cache_write: 0.0,
            },
            context_window: 128000,
            max_tokens: 16384,
            thinking_level_map: Some(HashMap::from([
                (ModelThinkingLevel::Off, None),
                (ModelThinkingLevel::XHigh, Some("xhigh".into())),
            ])),
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "gpt-5.3-codex".into(),
        Model {
            id: "gpt-5.3-codex".into(),
            name: "GPT-5.3 Codex".into(),
            api: Api::OpenAiResponses,
            provider: "openai".into(),
            base_url: "https://api.openai.com/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 1.75,
                output: 14.0,
                cache_read: 0.175,
                cache_write: 0.0,
            },
            context_window: 400000,
            max_tokens: 128000,
            thinking_level_map: Some(HashMap::from([
                (ModelThinkingLevel::Off, Some("none".into())),
                (ModelThinkingLevel::XHigh, Some("xhigh".into())),
            ])),
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "gpt-5.3-codex-spark".into(),
        Model {
            id: "gpt-5.3-codex-spark".into(),
            name: "GPT-5.3 Codex Spark".into(),
            api: Api::OpenAiResponses,
            provider: "openai".into(),
            base_url: "https://api.openai.com/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 1.75,
                output: 14.0,
                cache_read: 0.175,
                cache_write: 0.0,
            },
            context_window: 128000,
            max_tokens: 32000,
            thinking_level_map: Some(HashMap::from([
                (ModelThinkingLevel::Off, None),
                (ModelThinkingLevel::XHigh, Some("xhigh".into())),
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
            api: Api::OpenAiResponses,
            provider: "openai".into(),
            base_url: "https://api.openai.com/v1".into(),
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
                (ModelThinkingLevel::Off, Some("none".into())),
                (ModelThinkingLevel::XHigh, Some("xhigh".into())),
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
            api: Api::OpenAiResponses,
            provider: "openai".into(),
            base_url: "https://api.openai.com/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.75,
                output: 4.5,
                cache_read: 0.075,
                cache_write: 0.0,
            },
            context_window: 400000,
            max_tokens: 128000,
            thinking_level_map: Some(HashMap::from([
                (ModelThinkingLevel::Off, Some("none".into())),
                (ModelThinkingLevel::XHigh, Some("xhigh".into())),
            ])),
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "gpt-5.4-nano".into(),
        Model {
            id: "gpt-5.4-nano".into(),
            name: "GPT-5.4 nano".into(),
            api: Api::OpenAiResponses,
            provider: "openai".into(),
            base_url: "https://api.openai.com/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.2,
                output: 1.25,
                cache_read: 0.02,
                cache_write: 0.0,
            },
            context_window: 400000,
            max_tokens: 128000,
            thinking_level_map: Some(HashMap::from([
                (ModelThinkingLevel::Off, Some("none".into())),
                (ModelThinkingLevel::XHigh, Some("xhigh".into())),
            ])),
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "gpt-5.4-pro".into(),
        Model {
            id: "gpt-5.4-pro".into(),
            name: "GPT-5.4 Pro".into(),
            api: Api::OpenAiResponses,
            provider: "openai".into(),
            base_url: "https://api.openai.com/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 30.0,
                output: 180.0,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 1050000,
            max_tokens: 128000,
            thinking_level_map: Some(HashMap::from([
                (ModelThinkingLevel::Off, None),
                (ModelThinkingLevel::XHigh, Some("xhigh".into())),
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
            api: Api::OpenAiResponses,
            provider: "openai".into(),
            base_url: "https://api.openai.com/v1".into(),
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
                (ModelThinkingLevel::Off, Some("none".into())),
                (ModelThinkingLevel::XHigh, Some("xhigh".into())),
                (ModelThinkingLevel::Minimal, None),
            ])),
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "gpt-5.5-pro".into(),
        Model {
            id: "gpt-5.5-pro".into(),
            name: "GPT-5.5 Pro".into(),
            api: Api::OpenAiResponses,
            provider: "openai".into(),
            base_url: "https://api.openai.com/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 30.0,
                output: 180.0,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 1050000,
            max_tokens: 128000,
            thinking_level_map: Some(HashMap::from([
                (ModelThinkingLevel::Off, None),
                (ModelThinkingLevel::XHigh, Some("xhigh".into())),
                (ModelThinkingLevel::Minimal, None),
                (ModelThinkingLevel::Low, None),
            ])),
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "o1-pro".into(),
        Model {
            id: "o1-pro".into(),
            name: "o1-pro".into(),
            api: Api::OpenAiResponses,
            provider: "openai".into(),
            base_url: "https://api.openai.com/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 150.0,
                output: 600.0,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 200000,
            max_tokens: 100000,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "o3-deep-research".into(),
        Model {
            id: "o3-deep-research".into(),
            name: "o3-deep-research".into(),
            api: Api::OpenAiResponses,
            provider: "openai".into(),
            base_url: "https://api.openai.com/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 10.0,
                output: 40.0,
                cache_read: 2.5,
                cache_write: 0.0,
            },
            context_window: 200000,
            max_tokens: 100000,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "o3-mini".into(),
        Model {
            id: "o3-mini".into(),
            name: "o3-mini".into(),
            api: Api::OpenAiResponses,
            provider: "openai".into(),
            base_url: "https://api.openai.com/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 1.1,
                output: 4.4,
                cache_read: 0.55,
                cache_write: 0.0,
            },
            context_window: 200000,
            max_tokens: 100000,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "o3-pro".into(),
        Model {
            id: "o3-pro".into(),
            name: "o3-pro".into(),
            api: Api::OpenAiResponses,
            provider: "openai".into(),
            base_url: "https://api.openai.com/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 20.0,
                output: 80.0,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 200000,
            max_tokens: 100000,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "o4-mini".into(),
        Model {
            id: "o4-mini".into(),
            name: "o4-mini".into(),
            api: Api::OpenAiResponses,
            provider: "openai".into(),
            base_url: "https://api.openai.com/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 1.1,
                output: 4.4,
                cache_read: 0.275,
                cache_write: 0.0,
            },
            context_window: 200000,
            max_tokens: 100000,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "o4-mini-deep-research".into(),
        Model {
            id: "o4-mini-deep-research".into(),
            name: "o4-mini-deep-research".into(),
            api: Api::OpenAiResponses,
            provider: "openai".into(),
            base_url: "https://api.openai.com/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 2.0,
                output: 8.0,
                cache_read: 0.5,
                cache_write: 0.0,
            },
            context_window: 200000,
            max_tokens: 100000,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map
}
