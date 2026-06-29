//! Auto-generated model catalogue for `amazon-bedrock`.
//! Do not edit manually — re-run `scripts/generate_models.py`.

use std::collections::HashMap;

use crate::types::{Api, InputModality, Model, ModelCost, ModelThinkingLevel};

/// Model catalogue for the `amazon-bedrock` provider.
pub fn models() -> HashMap<String, Model> {
    let mut map: HashMap<String, Model> = HashMap::new();

    map.insert(
        "amazon.nova-2-lite-v1:0".into(),
        Model {
            id: "amazon.nova-2-lite-v1:0".into(),
            name: "Nova 2 Lite".into(),
            api: Api::BedrockConverseStream,
            provider: "amazon-bedrock".into(),
            base_url: "https://bedrock-runtime.us-east-1.amazonaws.com".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.33,
                output: 2.75,
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
        "amazon.nova-lite-v1:0".into(),
        Model {
            id: "amazon.nova-lite-v1:0".into(),
            name: "Nova Lite".into(),
            api: Api::BedrockConverseStream,
            provider: "amazon-bedrock".into(),
            base_url: "https://bedrock-runtime.us-east-1.amazonaws.com".into(),
            reasoning: false,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.06,
                output: 0.24,
                cache_read: 0.015,
                cache_write: 0.0,
            },
            context_window: 300000,
            max_tokens: 8192,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "amazon.nova-micro-v1:0".into(),
        Model {
            id: "amazon.nova-micro-v1:0".into(),
            name: "Nova Micro".into(),
            api: Api::BedrockConverseStream,
            provider: "amazon-bedrock".into(),
            base_url: "https://bedrock-runtime.us-east-1.amazonaws.com".into(),
            reasoning: false,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.035,
                output: 0.14,
                cache_read: 0.00875,
                cache_write: 0.0,
            },
            context_window: 128000,
            max_tokens: 8192,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "amazon.nova-pro-v1:0".into(),
        Model {
            id: "amazon.nova-pro-v1:0".into(),
            name: "Nova Pro".into(),
            api: Api::BedrockConverseStream,
            provider: "amazon-bedrock".into(),
            base_url: "https://bedrock-runtime.us-east-1.amazonaws.com".into(),
            reasoning: false,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.8,
                output: 3.2,
                cache_read: 0.2,
                cache_write: 0.0,
            },
            context_window: 300000,
            max_tokens: 8192,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "anthropic.claude-haiku-4-5-20251001-v1:0".into(),
        Model {
            id: "anthropic.claude-haiku-4-5-20251001-v1:0".into(),
            name: "Claude Haiku 4.5".into(),
            api: Api::BedrockConverseStream,
            provider: "amazon-bedrock".into(),
            base_url: "https://bedrock-runtime.us-east-1.amazonaws.com".into(),
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
        "anthropic.claude-opus-4-1-20250805-v1:0".into(),
        Model {
            id: "anthropic.claude-opus-4-1-20250805-v1:0".into(),
            name: "Claude Opus 4.1".into(),
            api: Api::BedrockConverseStream,
            provider: "amazon-bedrock".into(),
            base_url: "https://bedrock-runtime.us-east-1.amazonaws.com".into(),
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
        "anthropic.claude-opus-4-5-20251101-v1:0".into(),
        Model {
            id: "anthropic.claude-opus-4-5-20251101-v1:0".into(),
            name: "Claude Opus 4.5".into(),
            api: Api::BedrockConverseStream,
            provider: "amazon-bedrock".into(),
            base_url: "https://bedrock-runtime.us-east-1.amazonaws.com".into(),
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
        "anthropic.claude-opus-4-6-v1".into(),
        Model {
            id: "anthropic.claude-opus-4-6-v1".into(),
            name: "Claude Opus 4.6".into(),
            api: Api::BedrockConverseStream,
            provider: "amazon-bedrock".into(),
            base_url: "https://bedrock-runtime.us-east-1.amazonaws.com".into(),
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
        "anthropic.claude-opus-4-7".into(),
        Model {
            id: "anthropic.claude-opus-4-7".into(),
            name: "Claude Opus 4.7".into(),
            api: Api::BedrockConverseStream,
            provider: "amazon-bedrock".into(),
            base_url: "https://bedrock-runtime.us-east-1.amazonaws.com".into(),
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
        "anthropic.claude-opus-4-8".into(),
        Model {
            id: "anthropic.claude-opus-4-8".into(),
            name: "Claude Opus 4.8".into(),
            api: Api::BedrockConverseStream,
            provider: "amazon-bedrock".into(),
            base_url: "https://bedrock-runtime.us-east-1.amazonaws.com".into(),
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
        "anthropic.claude-sonnet-4-5-20250929-v1:0".into(),
        Model {
            id: "anthropic.claude-sonnet-4-5-20250929-v1:0".into(),
            name: "Claude Sonnet 4.5".into(),
            api: Api::BedrockConverseStream,
            provider: "amazon-bedrock".into(),
            base_url: "https://bedrock-runtime.us-east-1.amazonaws.com".into(),
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
        "anthropic.claude-sonnet-4-6".into(),
        Model {
            id: "anthropic.claude-sonnet-4-6".into(),
            name: "Claude Sonnet 4.6".into(),
            api: Api::BedrockConverseStream,
            provider: "amazon-bedrock".into(),
            base_url: "https://bedrock-runtime.us-east-1.amazonaws.com".into(),
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
        "au.anthropic.claude-haiku-4-5-20251001-v1:0".into(),
        Model {
            id: "au.anthropic.claude-haiku-4-5-20251001-v1:0".into(),
            name: "Claude Haiku 4.5 (AU)".into(),
            api: Api::BedrockConverseStream,
            provider: "amazon-bedrock".into(),
            base_url: "https://bedrock-runtime.us-east-1.amazonaws.com".into(),
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
        "au.anthropic.claude-opus-4-6-v1".into(),
        Model {
            id: "au.anthropic.claude-opus-4-6-v1".into(),
            name: "AU Anthropic Claude Opus 4.6".into(),
            api: Api::BedrockConverseStream,
            provider: "amazon-bedrock".into(),
            base_url: "https://bedrock-runtime.us-east-1.amazonaws.com".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 16.5,
                output: 82.5,
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
        "au.anthropic.claude-opus-4-8".into(),
        Model {
            id: "au.anthropic.claude-opus-4-8".into(),
            name: "Claude Opus 4.8 (AU)".into(),
            api: Api::BedrockConverseStream,
            provider: "amazon-bedrock".into(),
            base_url: "https://bedrock-runtime.us-east-1.amazonaws.com".into(),
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
        "au.anthropic.claude-sonnet-4-5-20250929-v1:0".into(),
        Model {
            id: "au.anthropic.claude-sonnet-4-5-20250929-v1:0".into(),
            name: "Claude Sonnet 4.5 (AU)".into(),
            api: Api::BedrockConverseStream,
            provider: "amazon-bedrock".into(),
            base_url: "https://bedrock-runtime.us-east-1.amazonaws.com".into(),
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
        "au.anthropic.claude-sonnet-4-6".into(),
        Model {
            id: "au.anthropic.claude-sonnet-4-6".into(),
            name: "AU Anthropic Claude Sonnet 4.6".into(),
            api: Api::BedrockConverseStream,
            provider: "amazon-bedrock".into(),
            base_url: "https://bedrock-runtime.us-east-1.amazonaws.com".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 3.3,
                output: 16.5,
                cache_read: 0.33,
                cache_write: 4.125,
            },
            context_window: 1000000,
            max_tokens: 128000,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "deepseek.r1-v1:0".into(),
        Model {
            id: "deepseek.r1-v1:0".into(),
            name: "DeepSeek-R1".into(),
            api: Api::BedrockConverseStream,
            provider: "amazon-bedrock".into(),
            base_url: "https://bedrock-runtime.us-east-1.amazonaws.com".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 1.35,
                output: 5.4,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 128000,
            max_tokens: 32768,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "deepseek.v3-v1:0".into(),
        Model {
            id: "deepseek.v3-v1:0".into(),
            name: "DeepSeek-V3.1".into(),
            api: Api::BedrockConverseStream,
            provider: "amazon-bedrock".into(),
            base_url: "https://bedrock-runtime.us-east-1.amazonaws.com".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.58,
                output: 1.68,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 163840,
            max_tokens: 81920,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "deepseek.v3.2".into(),
        Model {
            id: "deepseek.v3.2".into(),
            name: "DeepSeek-V3.2".into(),
            api: Api::BedrockConverseStream,
            provider: "amazon-bedrock".into(),
            base_url: "https://bedrock-runtime.us-east-1.amazonaws.com".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.62,
                output: 1.85,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 163840,
            max_tokens: 81920,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "eu.anthropic.claude-fable-5".into(),
        Model {
            id: "eu.anthropic.claude-fable-5".into(),
            name: "Claude Fable 5 (EU)".into(),
            api: Api::BedrockConverseStream,
            provider: "amazon-bedrock".into(),
            base_url: "https://bedrock-runtime.eu-central-1.amazonaws.com".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 11.0,
                output: 55.0,
                cache_read: 1.1,
                cache_write: 13.75,
            },
            context_window: 1000000,
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
        "eu.anthropic.claude-haiku-4-5-20251001-v1:0".into(),
        Model {
            id: "eu.anthropic.claude-haiku-4-5-20251001-v1:0".into(),
            name: "Claude Haiku 4.5 (EU)".into(),
            api: Api::BedrockConverseStream,
            provider: "amazon-bedrock".into(),
            base_url: "https://bedrock-runtime.eu-central-1.amazonaws.com".into(),
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
        "eu.anthropic.claude-opus-4-5-20251101-v1:0".into(),
        Model {
            id: "eu.anthropic.claude-opus-4-5-20251101-v1:0".into(),
            name: "Claude Opus 4.5 (EU)".into(),
            api: Api::BedrockConverseStream,
            provider: "amazon-bedrock".into(),
            base_url: "https://bedrock-runtime.eu-central-1.amazonaws.com".into(),
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
        "eu.anthropic.claude-opus-4-6-v1".into(),
        Model {
            id: "eu.anthropic.claude-opus-4-6-v1".into(),
            name: "Claude Opus 4.6 (EU)".into(),
            api: Api::BedrockConverseStream,
            provider: "amazon-bedrock".into(),
            base_url: "https://bedrock-runtime.eu-central-1.amazonaws.com".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 5.5,
                output: 27.5,
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
        "eu.anthropic.claude-opus-4-7".into(),
        Model {
            id: "eu.anthropic.claude-opus-4-7".into(),
            name: "Claude Opus 4.7 (EU)".into(),
            api: Api::BedrockConverseStream,
            provider: "amazon-bedrock".into(),
            base_url: "https://bedrock-runtime.eu-central-1.amazonaws.com".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 5.5,
                output: 27.5,
                cache_read: 0.55,
                cache_write: 6.875,
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
        "eu.anthropic.claude-opus-4-8".into(),
        Model {
            id: "eu.anthropic.claude-opus-4-8".into(),
            name: "Claude Opus 4.8 (EU)".into(),
            api: Api::BedrockConverseStream,
            provider: "amazon-bedrock".into(),
            base_url: "https://bedrock-runtime.eu-central-1.amazonaws.com".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 5.5,
                output: 27.5,
                cache_read: 0.55,
                cache_write: 6.875,
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
        "eu.anthropic.claude-sonnet-4-5-20250929-v1:0".into(),
        Model {
            id: "eu.anthropic.claude-sonnet-4-5-20250929-v1:0".into(),
            name: "Claude Sonnet 4.5 (EU)".into(),
            api: Api::BedrockConverseStream,
            provider: "amazon-bedrock".into(),
            base_url: "https://bedrock-runtime.eu-central-1.amazonaws.com".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 3.3,
                output: 16.5,
                cache_read: 0.33,
                cache_write: 4.125,
            },
            context_window: 200000,
            max_tokens: 64000,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "eu.anthropic.claude-sonnet-4-6".into(),
        Model {
            id: "eu.anthropic.claude-sonnet-4-6".into(),
            name: "Claude Sonnet 4.6 (EU)".into(),
            api: Api::BedrockConverseStream,
            provider: "amazon-bedrock".into(),
            base_url: "https://bedrock-runtime.eu-central-1.amazonaws.com".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 3.3,
                output: 16.5,
                cache_read: 0.33,
                cache_write: 4.125,
            },
            context_window: 1000000,
            max_tokens: 64000,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "global.anthropic.claude-fable-5".into(),
        Model {
            id: "global.anthropic.claude-fable-5".into(),
            name: "Claude Fable 5 (Global)".into(),
            api: Api::BedrockConverseStream,
            provider: "amazon-bedrock".into(),
            base_url: "https://bedrock-runtime.us-east-1.amazonaws.com".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 10.0,
                output: 50.0,
                cache_read: 1.0,
                cache_write: 12.5,
            },
            context_window: 1000000,
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
        "global.anthropic.claude-haiku-4-5-20251001-v1:0".into(),
        Model {
            id: "global.anthropic.claude-haiku-4-5-20251001-v1:0".into(),
            name: "Claude Haiku 4.5 (Global)".into(),
            api: Api::BedrockConverseStream,
            provider: "amazon-bedrock".into(),
            base_url: "https://bedrock-runtime.us-east-1.amazonaws.com".into(),
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
        "global.anthropic.claude-opus-4-5-20251101-v1:0".into(),
        Model {
            id: "global.anthropic.claude-opus-4-5-20251101-v1:0".into(),
            name: "Claude Opus 4.5 (Global)".into(),
            api: Api::BedrockConverseStream,
            provider: "amazon-bedrock".into(),
            base_url: "https://bedrock-runtime.us-east-1.amazonaws.com".into(),
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
        "global.anthropic.claude-opus-4-6-v1".into(),
        Model {
            id: "global.anthropic.claude-opus-4-6-v1".into(),
            name: "Claude Opus 4.6 (Global)".into(),
            api: Api::BedrockConverseStream,
            provider: "amazon-bedrock".into(),
            base_url: "https://bedrock-runtime.us-east-1.amazonaws.com".into(),
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
        "global.anthropic.claude-opus-4-7".into(),
        Model {
            id: "global.anthropic.claude-opus-4-7".into(),
            name: "Claude Opus 4.7 (Global)".into(),
            api: Api::BedrockConverseStream,
            provider: "amazon-bedrock".into(),
            base_url: "https://bedrock-runtime.us-east-1.amazonaws.com".into(),
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
        "global.anthropic.claude-opus-4-8".into(),
        Model {
            id: "global.anthropic.claude-opus-4-8".into(),
            name: "Claude Opus 4.8 (Global)".into(),
            api: Api::BedrockConverseStream,
            provider: "amazon-bedrock".into(),
            base_url: "https://bedrock-runtime.us-east-1.amazonaws.com".into(),
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
        "global.anthropic.claude-sonnet-4-5-20250929-v1:0".into(),
        Model {
            id: "global.anthropic.claude-sonnet-4-5-20250929-v1:0".into(),
            name: "Claude Sonnet 4.5 (Global)".into(),
            api: Api::BedrockConverseStream,
            provider: "amazon-bedrock".into(),
            base_url: "https://bedrock-runtime.us-east-1.amazonaws.com".into(),
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
        "global.anthropic.claude-sonnet-4-6".into(),
        Model {
            id: "global.anthropic.claude-sonnet-4-6".into(),
            name: "Claude Sonnet 4.6 (Global)".into(),
            api: Api::BedrockConverseStream,
            provider: "amazon-bedrock".into(),
            base_url: "https://bedrock-runtime.us-east-1.amazonaws.com".into(),
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
        "google.gemma-3-27b-it".into(),
        Model {
            id: "google.gemma-3-27b-it".into(),
            name: "Google Gemma 3 27B Instruct".into(),
            api: Api::BedrockConverseStream,
            provider: "amazon-bedrock".into(),
            base_url: "https://bedrock-runtime.us-east-1.amazonaws.com".into(),
            reasoning: false,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.12,
                output: 0.2,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 202752,
            max_tokens: 8192,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "google.gemma-3-4b-it".into(),
        Model {
            id: "google.gemma-3-4b-it".into(),
            name: "Gemma 3 4B IT".into(),
            api: Api::BedrockConverseStream,
            provider: "amazon-bedrock".into(),
            base_url: "https://bedrock-runtime.us-east-1.amazonaws.com".into(),
            reasoning: false,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.04,
                output: 0.08,
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
        "jp.anthropic.claude-opus-4-7".into(),
        Model {
            id: "jp.anthropic.claude-opus-4-7".into(),
            name: "Claude Opus 4.7 (JP)".into(),
            api: Api::BedrockConverseStream,
            provider: "amazon-bedrock".into(),
            base_url: "https://bedrock-runtime.us-east-1.amazonaws.com".into(),
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
        "jp.anthropic.claude-opus-4-8".into(),
        Model {
            id: "jp.anthropic.claude-opus-4-8".into(),
            name: "Claude Opus 4.8 (JP)".into(),
            api: Api::BedrockConverseStream,
            provider: "amazon-bedrock".into(),
            base_url: "https://bedrock-runtime.us-east-1.amazonaws.com".into(),
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
        "jp.anthropic.claude-sonnet-4-5-20250929-v1:0".into(),
        Model {
            id: "jp.anthropic.claude-sonnet-4-5-20250929-v1:0".into(),
            name: "Claude Sonnet 4.5 (JP)".into(),
            api: Api::BedrockConverseStream,
            provider: "amazon-bedrock".into(),
            base_url: "https://bedrock-runtime.us-east-1.amazonaws.com".into(),
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
        "jp.anthropic.claude-sonnet-4-6".into(),
        Model {
            id: "jp.anthropic.claude-sonnet-4-6".into(),
            name: "Claude Sonnet 4.6 (JP)".into(),
            api: Api::BedrockConverseStream,
            provider: "amazon-bedrock".into(),
            base_url: "https://bedrock-runtime.us-east-1.amazonaws.com".into(),
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
        "meta.llama3-1-70b-instruct-v1:0".into(),
        Model {
            id: "meta.llama3-1-70b-instruct-v1:0".into(),
            name: "Llama 3.1 70B Instruct".into(),
            api: Api::BedrockConverseStream,
            provider: "amazon-bedrock".into(),
            base_url: "https://bedrock-runtime.us-east-1.amazonaws.com".into(),
            reasoning: false,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.72,
                output: 0.72,
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
        "meta.llama3-1-8b-instruct-v1:0".into(),
        Model {
            id: "meta.llama3-1-8b-instruct-v1:0".into(),
            name: "Llama 3.1 8B Instruct".into(),
            api: Api::BedrockConverseStream,
            provider: "amazon-bedrock".into(),
            base_url: "https://bedrock-runtime.us-east-1.amazonaws.com".into(),
            reasoning: false,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.22,
                output: 0.22,
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
        "meta.llama3-3-70b-instruct-v1:0".into(),
        Model {
            id: "meta.llama3-3-70b-instruct-v1:0".into(),
            name: "Llama 3.3 70B Instruct".into(),
            api: Api::BedrockConverseStream,
            provider: "amazon-bedrock".into(),
            base_url: "https://bedrock-runtime.us-east-1.amazonaws.com".into(),
            reasoning: false,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.72,
                output: 0.72,
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
        "meta.llama4-maverick-17b-instruct-v1:0".into(),
        Model {
            id: "meta.llama4-maverick-17b-instruct-v1:0".into(),
            name: "Llama 4 Maverick 17B Instruct".into(),
            api: Api::BedrockConverseStream,
            provider: "amazon-bedrock".into(),
            base_url: "https://bedrock-runtime.us-east-1.amazonaws.com".into(),
            reasoning: false,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.24,
                output: 0.97,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 1000000,
            max_tokens: 16384,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "meta.llama4-scout-17b-instruct-v1:0".into(),
        Model {
            id: "meta.llama4-scout-17b-instruct-v1:0".into(),
            name: "Llama 4 Scout 17B Instruct".into(),
            api: Api::BedrockConverseStream,
            provider: "amazon-bedrock".into(),
            base_url: "https://bedrock-runtime.us-east-1.amazonaws.com".into(),
            reasoning: false,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.17,
                output: 0.66,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 3500000,
            max_tokens: 16384,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "minimax.minimax-m2".into(),
        Model {
            id: "minimax.minimax-m2".into(),
            name: "MiniMax M2".into(),
            api: Api::BedrockConverseStream,
            provider: "amazon-bedrock".into(),
            base_url: "https://bedrock-runtime.us-east-1.amazonaws.com".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.3,
                output: 1.2,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 204608,
            max_tokens: 128000,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "minimax.minimax-m2.1".into(),
        Model {
            id: "minimax.minimax-m2.1".into(),
            name: "MiniMax M2.1".into(),
            api: Api::BedrockConverseStream,
            provider: "amazon-bedrock".into(),
            base_url: "https://bedrock-runtime.us-east-1.amazonaws.com".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.3,
                output: 1.2,
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
        "minimax.minimax-m2.5".into(),
        Model {
            id: "minimax.minimax-m2.5".into(),
            name: "MiniMax M2.5".into(),
            api: Api::BedrockConverseStream,
            provider: "amazon-bedrock".into(),
            base_url: "https://bedrock-runtime.us-east-1.amazonaws.com".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.3,
                output: 1.2,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 196608,
            max_tokens: 98304,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "mistral.devstral-2-123b".into(),
        Model {
            id: "mistral.devstral-2-123b".into(),
            name: "Devstral 2 123B".into(),
            api: Api::BedrockConverseStream,
            provider: "amazon-bedrock".into(),
            base_url: "https://bedrock-runtime.us-east-1.amazonaws.com".into(),
            reasoning: false,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.4,
                output: 2.0,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 256000,
            max_tokens: 8192,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "mistral.magistral-small-2509".into(),
        Model {
            id: "mistral.magistral-small-2509".into(),
            name: "Magistral Small 1.2".into(),
            api: Api::BedrockConverseStream,
            provider: "amazon-bedrock".into(),
            base_url: "https://bedrock-runtime.us-east-1.amazonaws.com".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.5,
                output: 1.5,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 128000,
            max_tokens: 40000,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "mistral.ministral-3-14b-instruct".into(),
        Model {
            id: "mistral.ministral-3-14b-instruct".into(),
            name: "Ministral 14B 3.0".into(),
            api: Api::BedrockConverseStream,
            provider: "amazon-bedrock".into(),
            base_url: "https://bedrock-runtime.us-east-1.amazonaws.com".into(),
            reasoning: false,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.2,
                output: 0.2,
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
        "mistral.ministral-3-3b-instruct".into(),
        Model {
            id: "mistral.ministral-3-3b-instruct".into(),
            name: "Ministral 3 3B".into(),
            api: Api::BedrockConverseStream,
            provider: "amazon-bedrock".into(),
            base_url: "https://bedrock-runtime.us-east-1.amazonaws.com".into(),
            reasoning: false,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.1,
                output: 0.1,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 256000,
            max_tokens: 8192,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "mistral.ministral-3-8b-instruct".into(),
        Model {
            id: "mistral.ministral-3-8b-instruct".into(),
            name: "Ministral 3 8B".into(),
            api: Api::BedrockConverseStream,
            provider: "amazon-bedrock".into(),
            base_url: "https://bedrock-runtime.us-east-1.amazonaws.com".into(),
            reasoning: false,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.15,
                output: 0.15,
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
        "mistral.mistral-large-3-675b-instruct".into(),
        Model {
            id: "mistral.mistral-large-3-675b-instruct".into(),
            name: "Mistral Large 3".into(),
            api: Api::BedrockConverseStream,
            provider: "amazon-bedrock".into(),
            base_url: "https://bedrock-runtime.us-east-1.amazonaws.com".into(),
            reasoning: false,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.5,
                output: 1.5,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 256000,
            max_tokens: 8192,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "mistral.pixtral-large-2502-v1:0".into(),
        Model {
            id: "mistral.pixtral-large-2502-v1:0".into(),
            name: "Pixtral Large (25.02)".into(),
            api: Api::BedrockConverseStream,
            provider: "amazon-bedrock".into(),
            base_url: "https://bedrock-runtime.us-east-1.amazonaws.com".into(),
            reasoning: false,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 2.0,
                output: 6.0,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 128000,
            max_tokens: 8192,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "mistral.voxtral-mini-3b-2507".into(),
        Model {
            id: "mistral.voxtral-mini-3b-2507".into(),
            name: "Voxtral Mini 3B 2507".into(),
            api: Api::BedrockConverseStream,
            provider: "amazon-bedrock".into(),
            base_url: "https://bedrock-runtime.us-east-1.amazonaws.com".into(),
            reasoning: false,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.04,
                output: 0.04,
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
        "mistral.voxtral-small-24b-2507".into(),
        Model {
            id: "mistral.voxtral-small-24b-2507".into(),
            name: "Voxtral Small 24B 2507".into(),
            api: Api::BedrockConverseStream,
            provider: "amazon-bedrock".into(),
            base_url: "https://bedrock-runtime.us-east-1.amazonaws.com".into(),
            reasoning: false,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.15,
                output: 0.35,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 32000,
            max_tokens: 8192,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "moonshot.kimi-k2-thinking".into(),
        Model {
            id: "moonshot.kimi-k2-thinking".into(),
            name: "Kimi K2 Thinking".into(),
            api: Api::BedrockConverseStream,
            provider: "amazon-bedrock".into(),
            base_url: "https://bedrock-runtime.us-east-1.amazonaws.com".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.6,
                output: 2.5,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 262143,
            max_tokens: 16000,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "moonshotai.kimi-k2.5".into(),
        Model {
            id: "moonshotai.kimi-k2.5".into(),
            name: "Kimi K2.5".into(),
            api: Api::BedrockConverseStream,
            provider: "amazon-bedrock".into(),
            base_url: "https://bedrock-runtime.us-east-1.amazonaws.com".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.6,
                output: 3.0,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 262143,
            max_tokens: 16000,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "nvidia.nemotron-nano-12b-v2".into(),
        Model {
            id: "nvidia.nemotron-nano-12b-v2".into(),
            name: "NVIDIA Nemotron Nano 12B v2 VL BF16".into(),
            api: Api::BedrockConverseStream,
            provider: "amazon-bedrock".into(),
            base_url: "https://bedrock-runtime.us-east-1.amazonaws.com".into(),
            reasoning: false,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.2,
                output: 0.6,
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
        "nvidia.nemotron-nano-3-30b".into(),
        Model {
            id: "nvidia.nemotron-nano-3-30b".into(),
            name: "NVIDIA Nemotron Nano 3 30B".into(),
            api: Api::BedrockConverseStream,
            provider: "amazon-bedrock".into(),
            base_url: "https://bedrock-runtime.us-east-1.amazonaws.com".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.06,
                output: 0.24,
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
        "nvidia.nemotron-nano-9b-v2".into(),
        Model {
            id: "nvidia.nemotron-nano-9b-v2".into(),
            name: "NVIDIA Nemotron Nano 9B v2".into(),
            api: Api::BedrockConverseStream,
            provider: "amazon-bedrock".into(),
            base_url: "https://bedrock-runtime.us-east-1.amazonaws.com".into(),
            reasoning: false,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.06,
                output: 0.23,
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
        "nvidia.nemotron-super-3-120b".into(),
        Model {
            id: "nvidia.nemotron-super-3-120b".into(),
            name: "NVIDIA Nemotron 3 Super 120B A12B".into(),
            api: Api::BedrockConverseStream,
            provider: "amazon-bedrock".into(),
            base_url: "https://bedrock-runtime.us-east-1.amazonaws.com".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.15,
                output: 0.65,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 262144,
            max_tokens: 131072,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "openai.gpt-5.4".into(),
        Model {
            id: "openai.gpt-5.4".into(),
            name: "GPT-5.4".into(),
            api: Api::BedrockConverseStream,
            provider: "amazon-bedrock".into(),
            base_url: "https://bedrock-runtime.us-east-1.amazonaws.com".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 2.75,
                output: 16.5,
                cache_read: 0.275,
                cache_write: 0.0,
            },
            context_window: 272000,
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
        "openai.gpt-5.5".into(),
        Model {
            id: "openai.gpt-5.5".into(),
            name: "GPT-5.5".into(),
            api: Api::BedrockConverseStream,
            provider: "amazon-bedrock".into(),
            base_url: "https://bedrock-runtime.us-east-1.amazonaws.com".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 5.5,
                output: 33.0,
                cache_read: 0.55,
                cache_write: 0.0,
            },
            context_window: 272000,
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
        "openai.gpt-oss-120b".into(),
        Model {
            id: "openai.gpt-oss-120b".into(),
            name: "gpt-oss-120b".into(),
            api: Api::BedrockConverseStream,
            provider: "amazon-bedrock".into(),
            base_url: "https://bedrock-runtime.us-east-1.amazonaws.com".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.15,
                output: 0.6,
                cache_read: 0.0,
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
        "openai.gpt-oss-120b-1:0".into(),
        Model {
            id: "openai.gpt-oss-120b-1:0".into(),
            name: "gpt-oss-120b".into(),
            api: Api::BedrockConverseStream,
            provider: "amazon-bedrock".into(),
            base_url: "https://bedrock-runtime.us-east-1.amazonaws.com".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.15,
                output: 0.6,
                cache_read: 0.0,
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
        "openai.gpt-oss-20b".into(),
        Model {
            id: "openai.gpt-oss-20b".into(),
            name: "gpt-oss-20b".into(),
            api: Api::BedrockConverseStream,
            provider: "amazon-bedrock".into(),
            base_url: "https://bedrock-runtime.us-east-1.amazonaws.com".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.07,
                output: 0.3,
                cache_read: 0.0,
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
        "openai.gpt-oss-20b-1:0".into(),
        Model {
            id: "openai.gpt-oss-20b-1:0".into(),
            name: "gpt-oss-20b".into(),
            api: Api::BedrockConverseStream,
            provider: "amazon-bedrock".into(),
            base_url: "https://bedrock-runtime.us-east-1.amazonaws.com".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.07,
                output: 0.3,
                cache_read: 0.0,
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
        "openai.gpt-oss-safeguard-120b".into(),
        Model {
            id: "openai.gpt-oss-safeguard-120b".into(),
            name: "GPT OSS Safeguard 120B".into(),
            api: Api::BedrockConverseStream,
            provider: "amazon-bedrock".into(),
            base_url: "https://bedrock-runtime.us-east-1.amazonaws.com".into(),
            reasoning: false,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.15,
                output: 0.6,
                cache_read: 0.0,
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
        "openai.gpt-oss-safeguard-20b".into(),
        Model {
            id: "openai.gpt-oss-safeguard-20b".into(),
            name: "GPT OSS Safeguard 20B".into(),
            api: Api::BedrockConverseStream,
            provider: "amazon-bedrock".into(),
            base_url: "https://bedrock-runtime.us-east-1.amazonaws.com".into(),
            reasoning: false,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.07,
                output: 0.2,
                cache_read: 0.0,
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
        "qwen.qwen3-235b-a22b-2507-v1:0".into(),
        Model {
            id: "qwen.qwen3-235b-a22b-2507-v1:0".into(),
            name: "Qwen3 235B A22B 2507".into(),
            api: Api::BedrockConverseStream,
            provider: "amazon-bedrock".into(),
            base_url: "https://bedrock-runtime.us-east-1.amazonaws.com".into(),
            reasoning: false,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.22,
                output: 0.88,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 262144,
            max_tokens: 131072,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "qwen.qwen3-32b-v1:0".into(),
        Model {
            id: "qwen.qwen3-32b-v1:0".into(),
            name: "Qwen3 32B (dense)".into(),
            api: Api::BedrockConverseStream,
            provider: "amazon-bedrock".into(),
            base_url: "https://bedrock-runtime.us-east-1.amazonaws.com".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.15,
                output: 0.6,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 16384,
            max_tokens: 16384,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "qwen.qwen3-coder-30b-a3b-v1:0".into(),
        Model {
            id: "qwen.qwen3-coder-30b-a3b-v1:0".into(),
            name: "Qwen3 Coder 30B A3B Instruct".into(),
            api: Api::BedrockConverseStream,
            provider: "amazon-bedrock".into(),
            base_url: "https://bedrock-runtime.us-east-1.amazonaws.com".into(),
            reasoning: false,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.15,
                output: 0.6,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 262144,
            max_tokens: 131072,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "qwen.qwen3-coder-480b-a35b-v1:0".into(),
        Model {
            id: "qwen.qwen3-coder-480b-a35b-v1:0".into(),
            name: "Qwen3 Coder 480B A35B Instruct".into(),
            api: Api::BedrockConverseStream,
            provider: "amazon-bedrock".into(),
            base_url: "https://bedrock-runtime.us-east-1.amazonaws.com".into(),
            reasoning: false,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.22,
                output: 1.8,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 131072,
            max_tokens: 65536,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "qwen.qwen3-coder-next".into(),
        Model {
            id: "qwen.qwen3-coder-next".into(),
            name: "Qwen3 Coder Next".into(),
            api: Api::BedrockConverseStream,
            provider: "amazon-bedrock".into(),
            base_url: "https://bedrock-runtime.us-east-1.amazonaws.com".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.22,
                output: 1.8,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 131072,
            max_tokens: 65536,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "qwen.qwen3-next-80b-a3b".into(),
        Model {
            id: "qwen.qwen3-next-80b-a3b".into(),
            name: "Qwen/Qwen3-Next-80B-A3B-Instruct".into(),
            api: Api::BedrockConverseStream,
            provider: "amazon-bedrock".into(),
            base_url: "https://bedrock-runtime.us-east-1.amazonaws.com".into(),
            reasoning: false,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.14,
                output: 1.4,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 262000,
            max_tokens: 262000,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "qwen.qwen3-vl-235b-a22b".into(),
        Model {
            id: "qwen.qwen3-vl-235b-a22b".into(),
            name: "Qwen/Qwen3-VL-235B-A22B-Instruct".into(),
            api: Api::BedrockConverseStream,
            provider: "amazon-bedrock".into(),
            base_url: "https://bedrock-runtime.us-east-1.amazonaws.com".into(),
            reasoning: false,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.3,
                output: 1.5,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 262000,
            max_tokens: 262000,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "us.anthropic.claude-fable-5".into(),
        Model {
            id: "us.anthropic.claude-fable-5".into(),
            name: "Claude Fable 5 (US)".into(),
            api: Api::BedrockConverseStream,
            provider: "amazon-bedrock".into(),
            base_url: "https://bedrock-runtime.us-east-1.amazonaws.com".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 10.0,
                output: 50.0,
                cache_read: 1.0,
                cache_write: 12.5,
            },
            context_window: 1000000,
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
        "us.anthropic.claude-haiku-4-5-20251001-v1:0".into(),
        Model {
            id: "us.anthropic.claude-haiku-4-5-20251001-v1:0".into(),
            name: "Claude Haiku 4.5 (US)".into(),
            api: Api::BedrockConverseStream,
            provider: "amazon-bedrock".into(),
            base_url: "https://bedrock-runtime.us-east-1.amazonaws.com".into(),
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
        "us.anthropic.claude-opus-4-1-20250805-v1:0".into(),
        Model {
            id: "us.anthropic.claude-opus-4-1-20250805-v1:0".into(),
            name: "Claude Opus 4.1 (US)".into(),
            api: Api::BedrockConverseStream,
            provider: "amazon-bedrock".into(),
            base_url: "https://bedrock-runtime.us-east-1.amazonaws.com".into(),
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
        "us.anthropic.claude-opus-4-5-20251101-v1:0".into(),
        Model {
            id: "us.anthropic.claude-opus-4-5-20251101-v1:0".into(),
            name: "Claude Opus 4.5 (US)".into(),
            api: Api::BedrockConverseStream,
            provider: "amazon-bedrock".into(),
            base_url: "https://bedrock-runtime.us-east-1.amazonaws.com".into(),
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
        "us.anthropic.claude-opus-4-6-v1".into(),
        Model {
            id: "us.anthropic.claude-opus-4-6-v1".into(),
            name: "Claude Opus 4.6 (US)".into(),
            api: Api::BedrockConverseStream,
            provider: "amazon-bedrock".into(),
            base_url: "https://bedrock-runtime.us-east-1.amazonaws.com".into(),
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
        "us.anthropic.claude-opus-4-7".into(),
        Model {
            id: "us.anthropic.claude-opus-4-7".into(),
            name: "Claude Opus 4.7 (US)".into(),
            api: Api::BedrockConverseStream,
            provider: "amazon-bedrock".into(),
            base_url: "https://bedrock-runtime.us-east-1.amazonaws.com".into(),
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
        "us.anthropic.claude-opus-4-8".into(),
        Model {
            id: "us.anthropic.claude-opus-4-8".into(),
            name: "Claude Opus 4.8 (US)".into(),
            api: Api::BedrockConverseStream,
            provider: "amazon-bedrock".into(),
            base_url: "https://bedrock-runtime.us-east-1.amazonaws.com".into(),
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
        "us.anthropic.claude-sonnet-4-5-20250929-v1:0".into(),
        Model {
            id: "us.anthropic.claude-sonnet-4-5-20250929-v1:0".into(),
            name: "Claude Sonnet 4.5 (US)".into(),
            api: Api::BedrockConverseStream,
            provider: "amazon-bedrock".into(),
            base_url: "https://bedrock-runtime.us-east-1.amazonaws.com".into(),
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
        "us.anthropic.claude-sonnet-4-6".into(),
        Model {
            id: "us.anthropic.claude-sonnet-4-6".into(),
            name: "Claude Sonnet 4.6 (US)".into(),
            api: Api::BedrockConverseStream,
            provider: "amazon-bedrock".into(),
            base_url: "https://bedrock-runtime.us-east-1.amazonaws.com".into(),
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
        "us.deepseek.r1-v1:0".into(),
        Model {
            id: "us.deepseek.r1-v1:0".into(),
            name: "DeepSeek-R1 (US)".into(),
            api: Api::BedrockConverseStream,
            provider: "amazon-bedrock".into(),
            base_url: "https://bedrock-runtime.us-east-1.amazonaws.com".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 1.35,
                output: 5.4,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 128000,
            max_tokens: 32768,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "us.meta.llama4-maverick-17b-instruct-v1:0".into(),
        Model {
            id: "us.meta.llama4-maverick-17b-instruct-v1:0".into(),
            name: "Llama 4 Maverick 17B Instruct (US)".into(),
            api: Api::BedrockConverseStream,
            provider: "amazon-bedrock".into(),
            base_url: "https://bedrock-runtime.us-east-1.amazonaws.com".into(),
            reasoning: false,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.24,
                output: 0.97,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 1000000,
            max_tokens: 16384,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "us.meta.llama4-scout-17b-instruct-v1:0".into(),
        Model {
            id: "us.meta.llama4-scout-17b-instruct-v1:0".into(),
            name: "Llama 4 Scout 17B Instruct (US)".into(),
            api: Api::BedrockConverseStream,
            provider: "amazon-bedrock".into(),
            base_url: "https://bedrock-runtime.us-east-1.amazonaws.com".into(),
            reasoning: false,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.17,
                output: 0.66,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 3500000,
            max_tokens: 16384,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "writer.palmyra-x4-v1:0".into(),
        Model {
            id: "writer.palmyra-x4-v1:0".into(),
            name: "Palmyra X4".into(),
            api: Api::BedrockConverseStream,
            provider: "amazon-bedrock".into(),
            base_url: "https://bedrock-runtime.us-east-1.amazonaws.com".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 2.5,
                output: 10.0,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 122880,
            max_tokens: 8192,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "writer.palmyra-x5-v1:0".into(),
        Model {
            id: "writer.palmyra-x5-v1:0".into(),
            name: "Palmyra X5".into(),
            api: Api::BedrockConverseStream,
            provider: "amazon-bedrock".into(),
            base_url: "https://bedrock-runtime.us-east-1.amazonaws.com".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.6,
                output: 6.0,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 1040000,
            max_tokens: 8192,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "zai.glm-4.7".into(),
        Model {
            id: "zai.glm-4.7".into(),
            name: "GLM-4.7".into(),
            api: Api::BedrockConverseStream,
            provider: "amazon-bedrock".into(),
            base_url: "https://bedrock-runtime.us-east-1.amazonaws.com".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.6,
                output: 2.2,
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
        "zai.glm-4.7-flash".into(),
        Model {
            id: "zai.glm-4.7-flash".into(),
            name: "GLM-4.7-Flash".into(),
            api: Api::BedrockConverseStream,
            provider: "amazon-bedrock".into(),
            base_url: "https://bedrock-runtime.us-east-1.amazonaws.com".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.07,
                output: 0.4,
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
        "zai.glm-5".into(),
        Model {
            id: "zai.glm-5".into(),
            name: "GLM-5".into(),
            api: Api::BedrockConverseStream,
            provider: "amazon-bedrock".into(),
            base_url: "https://bedrock-runtime.us-east-1.amazonaws.com".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 1.0,
                output: 3.2,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 202752,
            max_tokens: 101376,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map
}
