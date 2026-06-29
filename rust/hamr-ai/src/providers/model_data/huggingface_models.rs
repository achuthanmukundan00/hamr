//! Auto-generated model catalogue for `huggingface`.
//! Do not edit manually — re-run `scripts/generate_models.py`.

use std::collections::HashMap;

use crate::types::{Api, InputModality, Model, ModelCost};

/// Model catalogue for the `huggingface` provider.
pub fn models() -> HashMap<String, Model> {
    let mut map: HashMap<String, Model> = HashMap::new();

    map.insert(
        "MiniMaxAI/MiniMax-M2".into(),
        Model {
            id: "MiniMaxAI/MiniMax-M2".into(),
            name: "MiniMax-M2".into(),
            api: Api::OpenAiCompletions,
            provider: "huggingface".into(),
            base_url: "https://router.huggingface.co/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.3,
                output: 1.2,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 204800,
            max_tokens: 128000,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "MiniMaxAI/MiniMax-M2.1".into(),
        Model {
            id: "MiniMaxAI/MiniMax-M2.1".into(),
            name: "MiniMax-M2.1".into(),
            api: Api::OpenAiCompletions,
            provider: "huggingface".into(),
            base_url: "https://router.huggingface.co/v1".into(),
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
        "MiniMaxAI/MiniMax-M2.5".into(),
        Model {
            id: "MiniMaxAI/MiniMax-M2.5".into(),
            name: "MiniMax-M2.5".into(),
            api: Api::OpenAiCompletions,
            provider: "huggingface".into(),
            base_url: "https://router.huggingface.co/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.3,
                output: 1.2,
                cache_read: 0.03,
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
        "MiniMaxAI/MiniMax-M2.7".into(),
        Model {
            id: "MiniMaxAI/MiniMax-M2.7".into(),
            name: "MiniMax-M2.7".into(),
            api: Api::OpenAiCompletions,
            provider: "huggingface".into(),
            base_url: "https://router.huggingface.co/v1".into(),
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
        "MiniMaxAI/MiniMax-M3".into(),
        Model {
            id: "MiniMaxAI/MiniMax-M3".into(),
            name: "MiniMax-M3".into(),
            api: Api::OpenAiCompletions,
            provider: "huggingface".into(),
            base_url: "https://router.huggingface.co/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.3,
                output: 1.2,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 524288,
            max_tokens: 128000,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "Qwen/Qwen3-235B-A22B".into(),
        Model {
            id: "Qwen/Qwen3-235B-A22B".into(),
            name: "Qwen3 235B-A22B".into(),
            api: Api::OpenAiCompletions,
            provider: "huggingface".into(),
            base_url: "https://router.huggingface.co/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.2,
                output: 0.8,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 40960,
            max_tokens: 16384,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "Qwen/Qwen3-235B-A22B-Thinking-2507".into(),
        Model {
            id: "Qwen/Qwen3-235B-A22B-Thinking-2507".into(),
            name: "Qwen3-235B-A22B-Thinking-2507".into(),
            api: Api::OpenAiCompletions,
            provider: "huggingface".into(),
            base_url: "https://router.huggingface.co/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.3,
                output: 3.0,
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
        "Qwen/Qwen3-32B".into(),
        Model {
            id: "Qwen/Qwen3-32B".into(),
            name: "Qwen3 32B".into(),
            api: Api::OpenAiCompletions,
            provider: "huggingface".into(),
            base_url: "https://router.huggingface.co/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.29,
                output: 0.59,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 131072,
            max_tokens: 16384,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "Qwen/Qwen3-Coder-30B-A3B-Instruct".into(),
        Model {
            id: "Qwen/Qwen3-Coder-30B-A3B-Instruct".into(),
            name: "Qwen3-Coder 30B-A3B Instruct".into(),
            api: Api::OpenAiCompletions,
            provider: "huggingface".into(),
            base_url: "https://router.huggingface.co/v1".into(),
            reasoning: false,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.07,
                output: 0.26,
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
        "Qwen/Qwen3-Coder-480B-A35B-Instruct".into(),
        Model {
            id: "Qwen/Qwen3-Coder-480B-A35B-Instruct".into(),
            name: "Qwen3-Coder-480B-A35B-Instruct".into(),
            api: Api::OpenAiCompletions,
            provider: "huggingface".into(),
            base_url: "https://router.huggingface.co/v1".into(),
            reasoning: false,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 2.0,
                output: 2.0,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 262144,
            max_tokens: 66536,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "Qwen/Qwen3-Coder-Next".into(),
        Model {
            id: "Qwen/Qwen3-Coder-Next".into(),
            name: "Qwen3-Coder-Next".into(),
            api: Api::OpenAiCompletions,
            provider: "huggingface".into(),
            base_url: "https://router.huggingface.co/v1".into(),
            reasoning: false,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.2,
                output: 1.5,
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
        "Qwen/Qwen3-Next-80B-A3B-Instruct".into(),
        Model {
            id: "Qwen/Qwen3-Next-80B-A3B-Instruct".into(),
            name: "Qwen3-Next-80B-A3B-Instruct".into(),
            api: Api::OpenAiCompletions,
            provider: "huggingface".into(),
            base_url: "https://router.huggingface.co/v1".into(),
            reasoning: false,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.25,
                output: 1.0,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 262144,
            max_tokens: 66536,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "Qwen/Qwen3-Next-80B-A3B-Thinking".into(),
        Model {
            id: "Qwen/Qwen3-Next-80B-A3B-Thinking".into(),
            name: "Qwen3-Next-80B-A3B-Thinking".into(),
            api: Api::OpenAiCompletions,
            provider: "huggingface".into(),
            base_url: "https://router.huggingface.co/v1".into(),
            reasoning: false,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.3,
                output: 2.0,
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
        "Qwen/Qwen3.5-122B-A10B".into(),
        Model {
            id: "Qwen/Qwen3.5-122B-A10B".into(),
            name: "Qwen3.5 122B-A10B".into(),
            api: Api::OpenAiCompletions,
            provider: "huggingface".into(),
            base_url: "https://router.huggingface.co/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.4,
                output: 3.2,
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
        "Qwen/Qwen3.5-27B".into(),
        Model {
            id: "Qwen/Qwen3.5-27B".into(),
            name: "Qwen3.5 27B".into(),
            api: Api::OpenAiCompletions,
            provider: "huggingface".into(),
            base_url: "https://router.huggingface.co/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.3,
                output: 2.4,
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
        "Qwen/Qwen3.5-35B-A3B".into(),
        Model {
            id: "Qwen/Qwen3.5-35B-A3B".into(),
            name: "Qwen3.5 35B-A3B".into(),
            api: Api::OpenAiCompletions,
            provider: "huggingface".into(),
            base_url: "https://router.huggingface.co/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.25,
                output: 2.0,
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
        "Qwen/Qwen3.5-397B-A17B".into(),
        Model {
            id: "Qwen/Qwen3.5-397B-A17B".into(),
            name: "Qwen3.5-397B-A17B".into(),
            api: Api::OpenAiCompletions,
            provider: "huggingface".into(),
            base_url: "https://router.huggingface.co/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.6,
                output: 3.6,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 262144,
            max_tokens: 32768,
            thinking_level_map: None,
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
            provider: "huggingface".into(),
            base_url: "https://router.huggingface.co/v1".into(),
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
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "Qwen/Qwen3.6-27B".into(),
        Model {
            id: "Qwen/Qwen3.6-27B".into(),
            name: "Qwen3.6 27B".into(),
            api: Api::OpenAiCompletions,
            provider: "huggingface".into(),
            base_url: "https://router.huggingface.co/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.47,
                output: 3.19,
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
        "Qwen/Qwen3.6-35B-A3B".into(),
        Model {
            id: "Qwen/Qwen3.6-35B-A3B".into(),
            name: "Qwen3.6 35B-A3B".into(),
            api: Api::OpenAiCompletions,
            provider: "huggingface".into(),
            base_url: "https://router.huggingface.co/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.15,
                output: 0.95,
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
        "XiaomiMiMo/MiMo-V2-Flash".into(),
        Model {
            id: "XiaomiMiMo/MiMo-V2-Flash".into(),
            name: "MiMo-V2-Flash".into(),
            api: Api::OpenAiCompletions,
            provider: "huggingface".into(),
            base_url: "https://router.huggingface.co/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.1,
                output: 0.3,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 262144,
            max_tokens: 4096,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "XiaomiMiMo/MiMo-V2.5-Pro".into(),
        Model {
            id: "XiaomiMiMo/MiMo-V2.5-Pro".into(),
            name: "MiMo-V2.5-Pro".into(),
            api: Api::OpenAiCompletions,
            provider: "huggingface".into(),
            base_url: "https://router.huggingface.co/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 1.0,
                output: 3.0,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 1048576,
            max_tokens: 131072,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "deepseek-ai/DeepSeek-R1".into(),
        Model {
            id: "deepseek-ai/DeepSeek-R1".into(),
            name: "DeepSeek-R1".into(),
            api: Api::OpenAiCompletions,
            provider: "huggingface".into(),
            base_url: "https://router.huggingface.co/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.7,
                output: 2.5,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 64000,
            max_tokens: 32768,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "deepseek-ai/DeepSeek-R1-0528".into(),
        Model {
            id: "deepseek-ai/DeepSeek-R1-0528".into(),
            name: "DeepSeek-R1-0528".into(),
            api: Api::OpenAiCompletions,
            provider: "huggingface".into(),
            base_url: "https://router.huggingface.co/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 3.0,
                output: 5.0,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 163840,
            max_tokens: 163840,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "deepseek-ai/DeepSeek-V3.2".into(),
        Model {
            id: "deepseek-ai/DeepSeek-V3.2".into(),
            name: "DeepSeek-V3.2".into(),
            api: Api::OpenAiCompletions,
            provider: "huggingface".into(),
            base_url: "https://router.huggingface.co/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.28,
                output: 0.4,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 163840,
            max_tokens: 65536,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "deepseek-ai/DeepSeek-V4-Flash".into(),
        Model {
            id: "deepseek-ai/DeepSeek-V4-Flash".into(),
            name: "DeepSeek V4 Flash".into(),
            api: Api::OpenAiCompletions,
            provider: "huggingface".into(),
            base_url: "https://router.huggingface.co/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.14,
                output: 0.28,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 1048576,
            max_tokens: 384000,
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
            provider: "huggingface".into(),
            base_url: "https://router.huggingface.co/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.435,
                output: 0.87,
                cache_read: 0.003625,
                cache_write: 0.0,
            },
            context_window: 1048576,
            max_tokens: 393216,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "google/gemma-4-26B-A4B-it".into(),
        Model {
            id: "google/gemma-4-26B-A4B-it".into(),
            name: "Gemma 4 26B A4B IT".into(),
            api: Api::OpenAiCompletions,
            provider: "huggingface".into(),
            base_url: "https://router.huggingface.co/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.13,
                output: 0.4,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 262144,
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
            name: "Gemma 4 31B IT".into(),
            api: Api::OpenAiCompletions,
            provider: "huggingface".into(),
            base_url: "https://router.huggingface.co/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.14,
                output: 0.4,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 262144,
            max_tokens: 32768,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "meta-llama/Llama-3.3-70B-Instruct".into(),
        Model {
            id: "meta-llama/Llama-3.3-70B-Instruct".into(),
            name: "Llama-3.3-70B-Instruct".into(),
            api: Api::OpenAiCompletions,
            provider: "huggingface".into(),
            base_url: "https://router.huggingface.co/v1".into(),
            reasoning: false,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.59,
                output: 0.79,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 131072,
            max_tokens: 4096,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "moonshotai/Kimi-K2-Instruct".into(),
        Model {
            id: "moonshotai/Kimi-K2-Instruct".into(),
            name: "Kimi-K2-Instruct".into(),
            api: Api::OpenAiCompletions,
            provider: "huggingface".into(),
            base_url: "https://router.huggingface.co/v1".into(),
            reasoning: false,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 1.0,
                output: 3.0,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 131072,
            max_tokens: 16384,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "moonshotai/Kimi-K2-Instruct-0905".into(),
        Model {
            id: "moonshotai/Kimi-K2-Instruct-0905".into(),
            name: "Kimi-K2-Instruct-0905".into(),
            api: Api::OpenAiCompletions,
            provider: "huggingface".into(),
            base_url: "https://router.huggingface.co/v1".into(),
            reasoning: false,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 1.0,
                output: 3.0,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 262144,
            max_tokens: 16384,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "moonshotai/Kimi-K2-Thinking".into(),
        Model {
            id: "moonshotai/Kimi-K2-Thinking".into(),
            name: "Kimi-K2-Thinking".into(),
            api: Api::OpenAiCompletions,
            provider: "huggingface".into(),
            base_url: "https://router.huggingface.co/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.6,
                output: 2.5,
                cache_read: 0.15,
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
        "moonshotai/Kimi-K2.5".into(),
        Model {
            id: "moonshotai/Kimi-K2.5".into(),
            name: "Kimi-K2.5".into(),
            api: Api::OpenAiCompletions,
            provider: "huggingface".into(),
            base_url: "https://router.huggingface.co/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.6,
                output: 3.0,
                cache_read: 0.1,
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
        "moonshotai/Kimi-K2.6".into(),
        Model {
            id: "moonshotai/Kimi-K2.6".into(),
            name: "Kimi-K2.6".into(),
            api: Api::OpenAiCompletions,
            provider: "huggingface".into(),
            base_url: "https://router.huggingface.co/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.95,
                output: 4.0,
                cache_read: 0.16,
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
        "moonshotai/Kimi-K2.7-Code".into(),
        Model {
            id: "moonshotai/Kimi-K2.7-Code".into(),
            name: "Kimi K2.7 Code".into(),
            api: Api::OpenAiCompletions,
            provider: "huggingface".into(),
            base_url: "https://router.huggingface.co/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.95,
                output: 4.0,
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
        "stepfun-ai/Step-3.5-Flash".into(),
        Model {
            id: "stepfun-ai/Step-3.5-Flash".into(),
            name: "Step 3.5 Flash".into(),
            api: Api::OpenAiCompletions,
            provider: "huggingface".into(),
            base_url: "https://router.huggingface.co/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.1,
                output: 0.3,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 262144,
            max_tokens: 256000,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "stepfun-ai/Step-3.7-Flash".into(),
        Model {
            id: "stepfun-ai/Step-3.7-Flash".into(),
            name: "Step 3.7 Flash".into(),
            api: Api::OpenAiCompletions,
            provider: "huggingface".into(),
            base_url: "https://router.huggingface.co/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.2,
                output: 1.15,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 262144,
            max_tokens: 256000,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "zai-org/GLM-4.5".into(),
        Model {
            id: "zai-org/GLM-4.5".into(),
            name: "GLM-4.5".into(),
            api: Api::OpenAiCompletions,
            provider: "huggingface".into(),
            base_url: "https://router.huggingface.co/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.6,
                output: 2.2,
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
        "zai-org/GLM-4.5-Air".into(),
        Model {
            id: "zai-org/GLM-4.5-Air".into(),
            name: "GLM-4.5-Air".into(),
            api: Api::OpenAiCompletions,
            provider: "huggingface".into(),
            base_url: "https://router.huggingface.co/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.13,
                output: 0.85,
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
        "zai-org/GLM-4.5V".into(),
        Model {
            id: "zai-org/GLM-4.5V".into(),
            name: "GLM-4.5V".into(),
            api: Api::OpenAiCompletions,
            provider: "huggingface".into(),
            base_url: "https://router.huggingface.co/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.6,
                output: 1.8,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 65536,
            max_tokens: 16384,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "zai-org/GLM-4.6".into(),
        Model {
            id: "zai-org/GLM-4.6".into(),
            name: "GLM-4.6".into(),
            api: Api::OpenAiCompletions,
            provider: "huggingface".into(),
            base_url: "https://router.huggingface.co/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.55,
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
        "zai-org/GLM-4.7".into(),
        Model {
            id: "zai-org/GLM-4.7".into(),
            name: "GLM-4.7".into(),
            api: Api::OpenAiCompletions,
            provider: "huggingface".into(),
            base_url: "https://router.huggingface.co/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.6,
                output: 2.2,
                cache_read: 0.11,
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
        "zai-org/GLM-4.7-Flash".into(),
        Model {
            id: "zai-org/GLM-4.7-Flash".into(),
            name: "GLM-4.7-Flash".into(),
            api: Api::OpenAiCompletions,
            provider: "huggingface".into(),
            base_url: "https://router.huggingface.co/v1".into(),
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
            thinking_level_map: None,
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
            provider: "huggingface".into(),
            base_url: "https://router.huggingface.co/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 1.0,
                output: 3.2,
                cache_read: 0.2,
                cache_write: 0.0,
            },
            context_window: 202752,
            max_tokens: 131072,
            thinking_level_map: None,
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
            provider: "huggingface".into(),
            base_url: "https://router.huggingface.co/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 1.0,
                output: 3.2,
                cache_read: 0.2,
                cache_write: 0.0,
            },
            context_window: 202752,
            max_tokens: 131072,
            thinking_level_map: None,
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
            provider: "huggingface".into(),
            base_url: "https://router.huggingface.co/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 1.4,
                output: 4.4,
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

    map
}
