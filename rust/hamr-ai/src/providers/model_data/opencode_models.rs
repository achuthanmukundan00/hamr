//! Auto-generated model catalogue for `opencode`.
//! Do not edit manually — re-run `scripts/generate_models.py`.

use std::collections::HashMap;

use crate::types::{Api, InputModality, Model, ModelCost, ModelThinkingLevel};

/// Model catalogue for the `opencode` provider.
pub fn models() -> HashMap<String, Model> {
    let mut map: HashMap<String, Model> = HashMap::new();

    map.insert(
        "big-pickle".into(),
        Model {
            id: "big-pickle".into(),
            name: "Big Pickle".into(),
            api: Api::OpenAiCompletions,
            provider: "opencode".into(),
            base_url: "https://opencode.ai/zen/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.0,
                output: 0.0,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 200000,
            max_tokens: 32000,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "claude-haiku-4-5".into(),
        Model {
            id: "claude-haiku-4-5".into(),
            name: "Claude Haiku 4.5".into(),
            api: Api::AnthropicMessages,
            provider: "opencode".into(),
            base_url: "https://opencode.ai/zen".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 1.0,
                output: 5.0,
                cache_read: 0.1,
                cache_write: 1.25,
            },
            context_window: 200000,
            max_tokens: 64000,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "claude-opus-4-1".into(),
        Model {
            id: "claude-opus-4-1".into(),
            name: "Claude Opus 4.1".into(),
            api: Api::AnthropicMessages,
            provider: "opencode".into(),
            base_url: "https://opencode.ai/zen".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 15.0,
                output: 75.0,
                cache_read: 1.5,
                cache_write: 18.75,
            },
            context_window: 200000,
            max_tokens: 32000,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "claude-opus-4-5".into(),
        Model {
            id: "claude-opus-4-5".into(),
            name: "Claude Opus 4.5".into(),
            api: Api::AnthropicMessages,
            provider: "opencode".into(),
            base_url: "https://opencode.ai/zen".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 5.0,
                output: 25.0,
                cache_read: 0.5,
                cache_write: 6.25,
            },
            context_window: 200000,
            max_tokens: 64000,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "claude-opus-4-6".into(),
        Model {
            id: "claude-opus-4-6".into(),
            name: "Claude Opus 4.6".into(),
            api: Api::AnthropicMessages,
            provider: "opencode".into(),
            base_url: "https://opencode.ai/zen".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 5.0,
                output: 25.0,
                cache_read: 0.5,
                cache_write: 6.25,
            },
            context_window: 1000000,
            max_tokens: 128000,
            thinking_level_map: Some(HashMap::from([(
                ModelThinkingLevel::XHigh,
                Some("max".into()),
            )])),
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "claude-opus-4-7".into(),
        Model {
            id: "claude-opus-4-7".into(),
            name: "Claude Opus 4.7".into(),
            api: Api::AnthropicMessages,
            provider: "opencode".into(),
            base_url: "https://opencode.ai/zen".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 5.0,
                output: 25.0,
                cache_read: 0.5,
                cache_write: 6.25,
            },
            context_window: 1000000,
            max_tokens: 128000,
            thinking_level_map: Some(HashMap::from([(
                ModelThinkingLevel::XHigh,
                Some("xhigh".into()),
            )])),
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "claude-opus-4-8".into(),
        Model {
            id: "claude-opus-4-8".into(),
            name: "Claude Opus 4.8".into(),
            api: Api::AnthropicMessages,
            provider: "opencode".into(),
            base_url: "https://opencode.ai/zen".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 5.0,
                output: 25.0,
                cache_read: 0.5,
                cache_write: 6.25,
            },
            context_window: 1000000,
            max_tokens: 128000,
            thinking_level_map: Some(HashMap::from([(
                ModelThinkingLevel::XHigh,
                Some("xhigh".into()),
            )])),
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "claude-sonnet-4".into(),
        Model {
            id: "claude-sonnet-4".into(),
            name: "Claude Sonnet 4".into(),
            api: Api::AnthropicMessages,
            provider: "opencode".into(),
            base_url: "https://opencode.ai/zen".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 3.0,
                output: 15.0,
                cache_read: 0.3,
                cache_write: 3.75,
            },
            context_window: 200000,
            max_tokens: 64000,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "claude-sonnet-4-5".into(),
        Model {
            id: "claude-sonnet-4-5".into(),
            name: "Claude Sonnet 4.5".into(),
            api: Api::AnthropicMessages,
            provider: "opencode".into(),
            base_url: "https://opencode.ai/zen".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 3.0,
                output: 15.0,
                cache_read: 0.3,
                cache_write: 3.75,
            },
            context_window: 200000,
            max_tokens: 64000,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "claude-sonnet-4-6".into(),
        Model {
            id: "claude-sonnet-4-6".into(),
            name: "Claude Sonnet 4.6".into(),
            api: Api::AnthropicMessages,
            provider: "opencode".into(),
            base_url: "https://opencode.ai/zen".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 3.0,
                output: 15.0,
                cache_read: 0.3,
                cache_write: 3.75,
            },
            context_window: 1000000,
            max_tokens: 64000,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "deepseek-v4-flash".into(),
        Model {
            id: "deepseek-v4-flash".into(),
            name: "DeepSeek V4 Flash".into(),
            api: Api::OpenAiCompletions,
            provider: "opencode".into(),
            base_url: "https://opencode.ai/zen/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.14,
                output: 0.28,
                cache_read: 0.028,
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
        "deepseek-v4-flash-free".into(),
        Model {
            id: "deepseek-v4-flash-free".into(),
            name: "DeepSeek V4 Flash Free".into(),
            api: Api::OpenAiCompletions,
            provider: "opencode".into(),
            base_url: "https://opencode.ai/zen/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.0,
                output: 0.0,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 200000,
            max_tokens: 128000,
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
            provider: "opencode".into(),
            base_url: "https://opencode.ai/zen/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 1.74,
                output: 3.84,
                cache_read: 0.145,
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
        "gemini-3-flash".into(),
        Model {
            id: "gemini-3-flash".into(),
            name: "Gemini 3 Flash".into(),
            api: Api::GoogleGenerativeAi,
            provider: "opencode".into(),
            base_url: "https://opencode.ai/zen/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.5,
                output: 3.0,
                cache_read: 0.05,
                cache_write: 0.0,
            },
            context_window: 1048576,
            max_tokens: 65536,
            thinking_level_map: Some(HashMap::from([(ModelThinkingLevel::Off, None)])),
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "gemini-3.1-pro".into(),
        Model {
            id: "gemini-3.1-pro".into(),
            name: "Gemini 3.1 Pro Preview".into(),
            api: Api::GoogleGenerativeAi,
            provider: "opencode".into(),
            base_url: "https://opencode.ai/zen/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 2.0,
                output: 12.0,
                cache_read: 0.2,
                cache_write: 0.0,
            },
            context_window: 1048576,
            max_tokens: 65536,
            thinking_level_map: Some(HashMap::from([
                (ModelThinkingLevel::Off, None),
                (ModelThinkingLevel::Minimal, None),
                (ModelThinkingLevel::Low, Some("LOW".into())),
                (ModelThinkingLevel::Medium, None),
                (ModelThinkingLevel::High, Some("HIGH".into())),
            ])),
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "gemini-3.5-flash".into(),
        Model {
            id: "gemini-3.5-flash".into(),
            name: "Gemini 3.5 Flash".into(),
            api: Api::GoogleGenerativeAi,
            provider: "opencode".into(),
            base_url: "https://opencode.ai/zen/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 1.5,
                output: 9.0,
                cache_read: 0.15,
                cache_write: 0.0,
            },
            context_window: 1048576,
            max_tokens: 65536,
            thinking_level_map: Some(HashMap::from([(ModelThinkingLevel::Off, None)])),
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "glm-5".into(),
        Model {
            id: "glm-5".into(),
            name: "GLM-5".into(),
            api: Api::OpenAiCompletions,
            provider: "opencode".into(),
            base_url: "https://opencode.ai/zen/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 1.0,
                output: 3.2,
                cache_read: 0.2,
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
        "glm-5.1".into(),
        Model {
            id: "glm-5.1".into(),
            name: "GLM-5.1".into(),
            api: Api::OpenAiCompletions,
            provider: "opencode".into(),
            base_url: "https://opencode.ai/zen/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 1.4,
                output: 4.4,
                cache_read: 0.26,
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
        "glm-5.2".into(),
        Model {
            id: "glm-5.2".into(),
            name: "GLM-5.2".into(),
            api: Api::OpenAiCompletions,
            provider: "opencode".into(),
            base_url: "https://opencode.ai/zen/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 1.4,
                output: 4.4,
                cache_read: 0.26,
                cache_write: 0.0,
            },
            context_window: 1000000,
            max_tokens: 131072,
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
            provider: "opencode".into(),
            base_url: "https://opencode.ai/zen/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 1.07,
                output: 8.5,
                cache_read: 0.107,
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
        "gpt-5-codex".into(),
        Model {
            id: "gpt-5-codex".into(),
            name: "GPT-5 Codex".into(),
            api: Api::OpenAiResponses,
            provider: "opencode".into(),
            base_url: "https://opencode.ai/zen/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 1.07,
                output: 8.5,
                cache_read: 0.107,
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
            provider: "opencode".into(),
            base_url: "https://opencode.ai/zen/v1".into(),
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
        "gpt-5.1".into(),
        Model {
            id: "gpt-5.1".into(),
            name: "GPT-5.1".into(),
            api: Api::OpenAiResponses,
            provider: "opencode".into(),
            base_url: "https://opencode.ai/zen/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 1.07,
                output: 8.5,
                cache_read: 0.107,
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
        "gpt-5.1-codex".into(),
        Model {
            id: "gpt-5.1-codex".into(),
            name: "GPT-5.1 Codex".into(),
            api: Api::OpenAiResponses,
            provider: "opencode".into(),
            base_url: "https://opencode.ai/zen/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 1.07,
                output: 8.5,
                cache_read: 0.107,
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
            provider: "opencode".into(),
            base_url: "https://opencode.ai/zen/v1".into(),
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
            name: "GPT-5.1 Codex Mini".into(),
            api: Api::OpenAiResponses,
            provider: "opencode".into(),
            base_url: "https://opencode.ai/zen/v1".into(),
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
            provider: "opencode".into(),
            base_url: "https://opencode.ai/zen/v1".into(),
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
        "gpt-5.2-codex".into(),
        Model {
            id: "gpt-5.2-codex".into(),
            name: "GPT-5.2 Codex".into(),
            api: Api::OpenAiResponses,
            provider: "opencode".into(),
            base_url: "https://opencode.ai/zen/v1".into(),
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
        "gpt-5.3-codex".into(),
        Model {
            id: "gpt-5.3-codex".into(),
            name: "GPT-5.3 Codex".into(),
            api: Api::OpenAiResponses,
            provider: "opencode".into(),
            base_url: "https://opencode.ai/zen/v1".into(),
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
        "gpt-5.4".into(),
        Model {
            id: "gpt-5.4".into(),
            name: "GPT-5.4".into(),
            api: Api::OpenAiResponses,
            provider: "opencode".into(),
            base_url: "https://opencode.ai/zen/v1".into(),
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
                (ModelThinkingLevel::Off, None),
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
            name: "GPT-5.4 Mini".into(),
            api: Api::OpenAiResponses,
            provider: "opencode".into(),
            base_url: "https://opencode.ai/zen/v1".into(),
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
                (ModelThinkingLevel::Off, None),
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
            name: "GPT-5.4 Nano".into(),
            api: Api::OpenAiResponses,
            provider: "opencode".into(),
            base_url: "https://opencode.ai/zen/v1".into(),
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
                (ModelThinkingLevel::Off, None),
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
            provider: "opencode".into(),
            base_url: "https://opencode.ai/zen/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 30.0,
                output: 180.0,
                cache_read: 30.0,
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
            provider: "opencode".into(),
            base_url: "https://opencode.ai/zen/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 5.0,
                output: 30.0,
                cache_read: 0.5,
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
        "gpt-5.5-pro".into(),
        Model {
            id: "gpt-5.5-pro".into(),
            name: "GPT-5.5 Pro".into(),
            api: Api::OpenAiResponses,
            provider: "opencode".into(),
            base_url: "https://opencode.ai/zen/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 30.0,
                output: 180.0,
                cache_read: 30.0,
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
        "grok-build-0.1".into(),
        Model {
            id: "grok-build-0.1".into(),
            name: "Grok Build 0.1".into(),
            api: Api::OpenAiCompletions,
            provider: "opencode".into(),
            base_url: "https://opencode.ai/zen/v1".into(),
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
            thinking_level_map: Some(HashMap::from([
                (ModelThinkingLevel::Off, None),
                (ModelThinkingLevel::Minimal, None),
                (ModelThinkingLevel::Low, None),
                (ModelThinkingLevel::Medium, None),
            ])),
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "kimi-k2.5".into(),
        Model {
            id: "kimi-k2.5".into(),
            name: "Kimi K2.5".into(),
            api: Api::OpenAiCompletions,
            provider: "opencode".into(),
            base_url: "https://opencode.ai/zen/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.6,
                output: 3.0,
                cache_read: 0.08,
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
        "kimi-k2.6".into(),
        Model {
            id: "kimi-k2.6".into(),
            name: "Kimi K2.6".into(),
            api: Api::OpenAiCompletions,
            provider: "opencode".into(),
            base_url: "https://opencode.ai/zen/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.95,
                output: 4.0,
                cache_read: 0.16,
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
        "mimo-v2.5-free".into(),
        Model {
            id: "mimo-v2.5-free".into(),
            name: "MiMo V2.5 Free".into(),
            api: Api::OpenAiCompletions,
            provider: "opencode".into(),
            base_url: "https://opencode.ai/zen/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.0,
                output: 0.0,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 200000,
            max_tokens: 32000,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "minimax-m2.5".into(),
        Model {
            id: "minimax-m2.5".into(),
            name: "MiniMax M2.5".into(),
            api: Api::OpenAiCompletions,
            provider: "opencode".into(),
            base_url: "https://opencode.ai/zen/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.3,
                output: 1.2,
                cache_read: 0.06,
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
        "minimax-m2.7".into(),
        Model {
            id: "minimax-m2.7".into(),
            name: "MiniMax M2.7".into(),
            api: Api::OpenAiCompletions,
            provider: "opencode".into(),
            base_url: "https://opencode.ai/zen/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.3,
                output: 1.2,
                cache_read: 0.06,
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
        "nemotron-3-ultra-free".into(),
        Model {
            id: "nemotron-3-ultra-free".into(),
            name: "Nemotron 3 Ultra Free".into(),
            api: Api::OpenAiCompletions,
            provider: "opencode".into(),
            base_url: "https://opencode.ai/zen/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.0,
                output: 0.0,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 1000000,
            max_tokens: 128000,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "north-mini-code-free".into(),
        Model {
            id: "north-mini-code-free".into(),
            name: "North Mini Code Free".into(),
            api: Api::OpenAiCompletions,
            provider: "opencode".into(),
            base_url: "https://opencode.ai/zen/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.0,
                output: 0.0,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 256000,
            max_tokens: 64000,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "qwen3.5-plus".into(),
        Model {
            id: "qwen3.5-plus".into(),
            name: "Qwen3.5 Plus".into(),
            api: Api::AnthropicMessages,
            provider: "opencode".into(),
            base_url: "https://opencode.ai/zen".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.2,
                output: 1.2,
                cache_read: 0.02,
                cache_write: 0.25,
            },
            context_window: 262144,
            max_tokens: 65536,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "qwen3.6-plus".into(),
        Model {
            id: "qwen3.6-plus".into(),
            name: "Qwen3.6 Plus".into(),
            api: Api::AnthropicMessages,
            provider: "opencode".into(),
            base_url: "https://opencode.ai/zen".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.5,
                output: 3.0,
                cache_read: 0.05,
                cache_write: 0.625,
            },
            context_window: 262144,
            max_tokens: 65536,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map
}
