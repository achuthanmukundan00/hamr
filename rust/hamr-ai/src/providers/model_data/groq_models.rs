//! Auto-generated model catalogue for `groq`.
//! Do not edit manually — re-run `scripts/generate_models.py`.

use std::collections::HashMap;

use crate::types::{Api, InputModality, Model, ModelCost, ModelThinkingLevel};

/// Model catalogue for the `groq` provider.
pub fn models() -> HashMap<String, Model> {
    let mut map: HashMap<String, Model> = HashMap::new();

    map.insert(
        "llama-3.1-8b-instant".into(),
        Model {
            id: "llama-3.1-8b-instant".into(),
            name: "Llama 3.1 8B".into(),
            api: Api::OpenAiCompletions,
            provider: "groq".into(),
            base_url: "https://api.groq.com/openai/v1".into(),
            reasoning: false,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.05,
                output: 0.08,
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
        "llama-3.3-70b-versatile".into(),
        Model {
            id: "llama-3.3-70b-versatile".into(),
            name: "Llama 3.3 70B".into(),
            api: Api::OpenAiCompletions,
            provider: "groq".into(),
            base_url: "https://api.groq.com/openai/v1".into(),
            reasoning: false,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.59,
                output: 0.79,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 131072,
            max_tokens: 32768,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "meta-llama/llama-4-scout-17b-16e-instruct".into(),
        Model {
            id: "meta-llama/llama-4-scout-17b-16e-instruct".into(),
            name: "Llama 4 Scout 17B 16E".into(),
            api: Api::OpenAiCompletions,
            provider: "groq".into(),
            base_url: "https://api.groq.com/openai/v1".into(),
            reasoning: false,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.11,
                output: 0.34,
                cache_read: 0.0,
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
        "openai/gpt-oss-120b".into(),
        Model {
            id: "openai/gpt-oss-120b".into(),
            name: "GPT OSS 120B".into(),
            api: Api::OpenAiCompletions,
            provider: "groq".into(),
            base_url: "https://api.groq.com/openai/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.15,
                output: 0.6,
                cache_read: 0.075,
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
        "openai/gpt-oss-20b".into(),
        Model {
            id: "openai/gpt-oss-20b".into(),
            name: "GPT OSS 20B".into(),
            api: Api::OpenAiCompletions,
            provider: "groq".into(),
            base_url: "https://api.groq.com/openai/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.075,
                output: 0.3,
                cache_read: 0.0375,
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
        "openai/gpt-oss-safeguard-20b".into(),
        Model {
            id: "openai/gpt-oss-safeguard-20b".into(),
            name: "Safety GPT OSS 20B".into(),
            api: Api::OpenAiCompletions,
            provider: "groq".into(),
            base_url: "https://api.groq.com/openai/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.075,
                output: 0.3,
                cache_read: 0.037,
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
        "qwen/qwen3-32b".into(),
        Model {
            id: "qwen/qwen3-32b".into(),
            name: "Qwen3-32B".into(),
            api: Api::OpenAiCompletions,
            provider: "groq".into(),
            base_url: "https://api.groq.com/openai/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.29,
                output: 0.59,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 131072,
            max_tokens: 40960,
            thinking_level_map: Some(HashMap::from([
                (ModelThinkingLevel::Minimal, None),
                (ModelThinkingLevel::Low, None),
                (ModelThinkingLevel::Medium, None),
                (ModelThinkingLevel::High, Some("default".into())),
            ])),
            headers: None,
            compat: None,
        },
    );

    map
}
