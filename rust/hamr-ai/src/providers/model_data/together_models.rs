//! Auto-generated model catalogue for `together`.
//! Do not edit manually — re-run `scripts/generate_models.py`.

use std::collections::HashMap;

use crate::types::{Api, InputModality, Model, ModelCost, ModelThinkingLevel};

/// Model catalogue for the `together` provider.
pub fn models() -> HashMap<String, Model> {
    let mut map: HashMap<String, Model> = HashMap::new();

    map.insert(
        "MiniMaxAI/MiniMax-M2.7".into(),
        Model {
            id: "MiniMaxAI/MiniMax-M2.7".into(),
            name: "MiniMax-M2.7".into(),
            api: Api::OpenAiCompletions,
            provider: "together".into(),
            base_url: "https://api.together.ai/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.3,
                output: 1.2,
                cache_read: 0.06,
                cache_write: 0.0,
            },
            context_window: 202752,
            max_tokens: 131072,
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
        "MiniMaxAI/MiniMax-M3".into(),
        Model {
            id: "MiniMaxAI/MiniMax-M3".into(),
            name: "MiniMax-M3".into(),
            api: Api::OpenAiCompletions,
            provider: "together".into(),
            base_url: "https://api.together.ai/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.3,
                output: 1.2,
                cache_read: 0.06,
                cache_write: 0.0,
            },
            context_window: 524288,
            max_tokens: 250000,
            thinking_level_map: Some(HashMap::from([
                (ModelThinkingLevel::Minimal, None),
                (ModelThinkingLevel::Low, None),
                (ModelThinkingLevel::Medium, None),
            ])),
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "Qwen/Qwen2.5-7B-Instruct-Turbo".into(),
        Model {
            id: "Qwen/Qwen2.5-7B-Instruct-Turbo".into(),
            name: "Qwen 2.5 7B Instruct Turbo".into(),
            api: Api::OpenAiCompletions,
            provider: "together".into(),
            base_url: "https://api.together.ai/v1".into(),
            reasoning: false,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.3,
                output: 0.3,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 32768,
            max_tokens: 32768,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "Qwen/Qwen3-235B-A22B-Instruct-2507-tput".into(),
        Model {
            id: "Qwen/Qwen3-235B-A22B-Instruct-2507-tput".into(),
            name: "Qwen3 235B A22B Instruct 2507 FP8".into(),
            api: Api::OpenAiCompletions,
            provider: "together".into(),
            base_url: "https://api.together.ai/v1".into(),
            reasoning: false,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.2,
                output: 0.6,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 262144,
            max_tokens: 262144,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "Qwen/Qwen3.5-397B-A17B".into(),
        Model {
            id: "Qwen/Qwen3.5-397B-A17B".into(),
            name: "Qwen3.5 397B A17B".into(),
            api: Api::OpenAiCompletions,
            provider: "together".into(),
            base_url: "https://api.together.ai/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.6,
                output: 3.6,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 262144,
            max_tokens: 130000,
            thinking_level_map: Some(HashMap::from([
                (ModelThinkingLevel::Minimal, None),
                (ModelThinkingLevel::Low, None),
                (ModelThinkingLevel::Medium, None),
            ])),
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "Qwen/Qwen3.5-9B".into(),
        Model {
            id: "Qwen/Qwen3.5-9B".into(),
            name: "Qwen3.5 9B".into(),
            api: Api::OpenAiCompletions,
            provider: "together".into(),
            base_url: "https://api.together.ai/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.17,
                output: 0.25,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 262144,
            max_tokens: 65536,
            thinking_level_map: Some(HashMap::from([
                (ModelThinkingLevel::Minimal, None),
                (ModelThinkingLevel::Low, None),
                (ModelThinkingLevel::Medium, None),
            ])),
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "Qwen/Qwen3.6-Plus".into(),
        Model {
            id: "Qwen/Qwen3.6-Plus".into(),
            name: "Qwen3.6 Plus".into(),
            api: Api::OpenAiCompletions,
            provider: "together".into(),
            base_url: "https://api.together.ai/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.5,
                output: 3.0,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 1000000,
            max_tokens: 500000,
            thinking_level_map: Some(HashMap::from([
                (ModelThinkingLevel::Minimal, None),
                (ModelThinkingLevel::Low, None),
                (ModelThinkingLevel::Medium, None),
            ])),
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "Qwen/Qwen3.7-Max".into(),
        Model {
            id: "Qwen/Qwen3.7-Max".into(),
            name: "Qwen3.7 Max".into(),
            api: Api::OpenAiCompletions,
            provider: "together".into(),
            base_url: "https://api.together.ai/v1".into(),
            reasoning: false,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 1.25,
                output: 3.75,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 1000000,
            max_tokens: 500000,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "deepseek-ai/DeepSeek-V4-Pro".into(),
        Model {
            id: "deepseek-ai/DeepSeek-V4-Pro".into(),
            name: "DeepSeek V4 Pro".into(),
            api: Api::OpenAiCompletions,
            provider: "together".into(),
            base_url: "https://api.together.ai/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 1.74,
                output: 3.48,
                cache_read: 0.2,
                cache_write: 0.0,
            },
            context_window: 512000,
            max_tokens: 384000,
            thinking_level_map: Some(HashMap::from([
                (ModelThinkingLevel::Minimal, None),
                (ModelThinkingLevel::Low, None),
                (ModelThinkingLevel::Medium, None),
                (ModelThinkingLevel::High, Some("high".into())),
                (ModelThinkingLevel::XHigh, None),
            ])),
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "essentialai/Rnj-1-Instruct".into(),
        Model {
            id: "essentialai/Rnj-1-Instruct".into(),
            name: "Rnj-1 Instruct".into(),
            api: Api::OpenAiCompletions,
            provider: "together".into(),
            base_url: "https://api.together.ai/v1".into(),
            reasoning: false,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.15,
                output: 0.15,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 32768,
            max_tokens: 32768,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "google/gemma-4-31B-it".into(),
        Model {
            id: "google/gemma-4-31B-it".into(),
            name: "Gemma 4 31B Instruct".into(),
            api: Api::OpenAiCompletions,
            provider: "together".into(),
            base_url: "https://api.together.ai/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.39,
                output: 0.97,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 262144,
            max_tokens: 131072,
            thinking_level_map: Some(HashMap::from([
                (ModelThinkingLevel::Minimal, None),
                (ModelThinkingLevel::Low, None),
                (ModelThinkingLevel::Medium, None),
            ])),
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "meta-llama/Llama-3.3-70B-Instruct-Turbo".into(),
        Model {
            id: "meta-llama/Llama-3.3-70B-Instruct-Turbo".into(),
            name: "Llama 3.3 70B".into(),
            api: Api::OpenAiCompletions,
            provider: "together".into(),
            base_url: "https://api.together.ai/v1".into(),
            reasoning: false,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.88,
                output: 0.88,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 131072,
            max_tokens: 131072,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "moonshotai/Kimi-K2.6".into(),
        Model {
            id: "moonshotai/Kimi-K2.6".into(),
            name: "Kimi K2.6".into(),
            api: Api::OpenAiCompletions,
            provider: "together".into(),
            base_url: "https://api.together.ai/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 1.2,
                output: 4.5,
                cache_read: 0.2,
                cache_write: 0.0,
            },
            context_window: 262144,
            max_tokens: 131000,
            thinking_level_map: Some(HashMap::from([
                (ModelThinkingLevel::Minimal, None),
                (ModelThinkingLevel::Low, None),
                (ModelThinkingLevel::Medium, None),
            ])),
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "moonshotai/Kimi-K2.7-Code".into(),
        Model {
            id: "moonshotai/Kimi-K2.7-Code".into(),
            name: "Kimi K2.7 Code".into(),
            api: Api::OpenAiCompletions,
            provider: "together".into(),
            base_url: "https://api.together.ai/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.95,
                output: 4.0,
                cache_read: 0.19,
                cache_write: 0.0,
            },
            context_window: 262144,
            max_tokens: 131072,
            thinking_level_map: Some(HashMap::from([
                (ModelThinkingLevel::Minimal, None),
                (ModelThinkingLevel::Low, None),
                (ModelThinkingLevel::Medium, None),
            ])),
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "nvidia/nemotron-3-ultra-550b-a55b".into(),
        Model {
            id: "nvidia/nemotron-3-ultra-550b-a55b".into(),
            name: "Nemotron 3 Ultra 550B A55B".into(),
            api: Api::OpenAiCompletions,
            provider: "together".into(),
            base_url: "https://api.together.ai/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.6,
                output: 3.6,
                cache_read: 0.2,
                cache_write: 0.0,
            },
            context_window: 512300,
            max_tokens: 512300,
            thinking_level_map: Some(HashMap::from([
                (ModelThinkingLevel::Minimal, None),
                (ModelThinkingLevel::Low, None),
                (ModelThinkingLevel::Medium, None),
            ])),
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "openai/gpt-oss-120b".into(),
        Model {
            id: "openai/gpt-oss-120b".into(),
            name: "GPT OSS 120B".into(),
            api: Api::OpenAiCompletions,
            provider: "together".into(),
            base_url: "https://api.together.ai/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.15,
                output: 0.6,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 131072,
            max_tokens: 131072,
            thinking_level_map: Some(HashMap::from([
                (ModelThinkingLevel::Off, None),
                (ModelThinkingLevel::Minimal, None),
            ])),
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "openai/gpt-oss-20b".into(),
        Model {
            id: "openai/gpt-oss-20b".into(),
            name: "GPT OSS 20B".into(),
            api: Api::OpenAiCompletions,
            provider: "together".into(),
            base_url: "https://api.together.ai/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.05,
                output: 0.2,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 131072,
            max_tokens: 131072,
            thinking_level_map: Some(HashMap::from([
                (ModelThinkingLevel::Off, None),
                (ModelThinkingLevel::Minimal, None),
            ])),
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "zai-org/GLM-5".into(),
        Model {
            id: "zai-org/GLM-5".into(),
            name: "GLM-5".into(),
            api: Api::OpenAiCompletions,
            provider: "together".into(),
            base_url: "https://api.together.ai/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 1.0,
                output: 3.2,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 202752,
            max_tokens: 131072,
            thinking_level_map: Some(HashMap::from([
                (ModelThinkingLevel::Minimal, None),
                (ModelThinkingLevel::Low, None),
                (ModelThinkingLevel::Medium, None),
            ])),
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "zai-org/GLM-5.1".into(),
        Model {
            id: "zai-org/GLM-5.1".into(),
            name: "GLM-5.1".into(),
            api: Api::OpenAiCompletions,
            provider: "together".into(),
            base_url: "https://api.together.ai/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 1.4,
                output: 4.4,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 202752,
            max_tokens: 131072,
            thinking_level_map: Some(HashMap::from([
                (ModelThinkingLevel::Minimal, None),
                (ModelThinkingLevel::Low, None),
                (ModelThinkingLevel::Medium, None),
            ])),
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "zai-org/GLM-5.2".into(),
        Model {
            id: "zai-org/GLM-5.2".into(),
            name: "GLM-5.2".into(),
            api: Api::OpenAiCompletions,
            provider: "together".into(),
            base_url: "https://api.together.ai/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 1.4,
                output: 4.4,
                cache_read: 0.26,
                cache_write: 0.0,
            },
            context_window: 262144,
            max_tokens: 164000,
            thinking_level_map: Some(HashMap::from([
                (ModelThinkingLevel::Minimal, None),
                (ModelThinkingLevel::Low, None),
                (ModelThinkingLevel::Medium, None),
            ])),
            headers: None,
            compat: None,
        },
    );

    map
}
