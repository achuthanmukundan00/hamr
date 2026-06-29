//! Auto-generated model catalogue for `nvidia`.
//! Do not edit manually — re-run `scripts/generate_models.py`.

use std::collections::HashMap;

use crate::types::{Api, InputModality, Model, ModelCost};

/// Model catalogue for the `nvidia` provider.
pub fn models() -> HashMap<String, Model> {
    let mut map: HashMap<String, Model> = HashMap::new();

    map.insert(
        "meta/llama-3.1-70b-instruct".into(),
        Model {
            id: "meta/llama-3.1-70b-instruct".into(),
            name: "Llama 3.1 70b Instruct".into(),
            api: Api::OpenAiCompletions,
            provider: "nvidia".into(),
            base_url: "https://integrate.api.nvidia.com/v1".into(),
            reasoning: false,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.0,
                output: 0.0,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 128000,
            max_tokens: 4096,
            thinking_level_map: None,
            headers: Some(HashMap::from([("NVCF-POLL-SECONDS".into(), "3600".into())])),
            compat: None,
        },
    );

    map.insert(
        "meta/llama-3.1-8b-instruct".into(),
        Model {
            id: "meta/llama-3.1-8b-instruct".into(),
            name: "Llama 3.1 8B Instruct".into(),
            api: Api::OpenAiCompletions,
            provider: "nvidia".into(),
            base_url: "https://integrate.api.nvidia.com/v1".into(),
            reasoning: false,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.0,
                output: 0.0,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 16000,
            max_tokens: 4096,
            thinking_level_map: None,
            headers: Some(HashMap::from([("NVCF-POLL-SECONDS".into(), "3600".into())])),
            compat: None,
        },
    );

    map.insert(
        "meta/llama-3.2-11b-vision-instruct".into(),
        Model {
            id: "meta/llama-3.2-11b-vision-instruct".into(),
            name: "Llama 3.2 11b Vision Instruct".into(),
            api: Api::OpenAiCompletions,
            provider: "nvidia".into(),
            base_url: "https://integrate.api.nvidia.com/v1".into(),
            reasoning: false,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.0,
                output: 0.0,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 128000,
            max_tokens: 4096,
            thinking_level_map: None,
            headers: Some(HashMap::from([("NVCF-POLL-SECONDS".into(), "3600".into())])),
            compat: None,
        },
    );

    map.insert(
        "meta/llama-3.2-90b-vision-instruct".into(),
        Model {
            id: "meta/llama-3.2-90b-vision-instruct".into(),
            name: "Llama-3.2-90B-Vision-Instruct".into(),
            api: Api::OpenAiCompletions,
            provider: "nvidia".into(),
            base_url: "https://integrate.api.nvidia.com/v1".into(),
            reasoning: false,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.0,
                output: 0.0,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 128000,
            max_tokens: 8192,
            thinking_level_map: None,
            headers: Some(HashMap::from([("NVCF-POLL-SECONDS".into(), "3600".into())])),
            compat: None,
        },
    );

    map.insert(
        "meta/llama-3.3-70b-instruct".into(),
        Model {
            id: "meta/llama-3.3-70b-instruct".into(),
            name: "Llama 3.3 70b Instruct".into(),
            api: Api::OpenAiCompletions,
            provider: "nvidia".into(),
            base_url: "https://integrate.api.nvidia.com/v1".into(),
            reasoning: false,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.0,
                output: 0.0,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 128000,
            max_tokens: 4096,
            thinking_level_map: None,
            headers: Some(HashMap::from([("NVCF-POLL-SECONDS".into(), "3600".into())])),
            compat: None,
        },
    );

    map.insert(
        "mistralai/mistral-large-3-675b-instruct-2512".into(),
        Model {
            id: "mistralai/mistral-large-3-675b-instruct-2512".into(),
            name: "Mistral Large 3 675B Instruct 2512".into(),
            api: Api::OpenAiCompletions,
            provider: "nvidia".into(),
            base_url: "https://integrate.api.nvidia.com/v1".into(),
            reasoning: false,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.0,
                output: 0.0,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 262144,
            max_tokens: 262144,
            thinking_level_map: None,
            headers: Some(HashMap::from([("NVCF-POLL-SECONDS".into(), "3600".into())])),
            compat: None,
        },
    );

    map.insert(
        "mistralai/mistral-small-4-119b-2603".into(),
        Model {
            id: "mistralai/mistral-small-4-119b-2603".into(),
            name: "mistral-small-4-119b-2603".into(),
            api: Api::OpenAiCompletions,
            provider: "nvidia".into(),
            base_url: "https://integrate.api.nvidia.com/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.0,
                output: 0.0,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 128000,
            max_tokens: 8192,
            thinking_level_map: None,
            headers: Some(HashMap::from([("NVCF-POLL-SECONDS".into(), "3600".into())])),
            compat: None,
        },
    );

    map.insert(
        "moonshotai/kimi-k2.6".into(),
        Model {
            id: "moonshotai/kimi-k2.6".into(),
            name: "Kimi K2.6".into(),
            api: Api::OpenAiCompletions,
            provider: "nvidia".into(),
            base_url: "https://integrate.api.nvidia.com/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.0,
                output: 0.0,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 262144,
            max_tokens: 262144,
            thinking_level_map: None,
            headers: Some(HashMap::from([("NVCF-POLL-SECONDS".into(), "3600".into())])),
            compat: None,
        },
    );

    map.insert(
        "nvidia/nemotron-3-nano-30b-a3b".into(),
        Model {
            id: "nvidia/nemotron-3-nano-30b-a3b".into(),
            name: "nemotron-3-nano-30b-a3b".into(),
            api: Api::OpenAiCompletions,
            provider: "nvidia".into(),
            base_url: "https://integrate.api.nvidia.com/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.0,
                output: 0.0,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 131072,
            max_tokens: 131072,
            thinking_level_map: None,
            headers: Some(HashMap::from([("NVCF-POLL-SECONDS".into(), "3600".into())])),
            compat: None,
        },
    );

    map.insert(
        "nvidia/nemotron-3-nano-omni-30b-a3b-reasoning".into(),
        Model {
            id: "nvidia/nemotron-3-nano-omni-30b-a3b-reasoning".into(),
            name: "Nemotron 3 Nano Omni".into(),
            api: Api::OpenAiCompletions,
            provider: "nvidia".into(),
            base_url: "https://integrate.api.nvidia.com/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.0,
                output: 0.0,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 256000,
            max_tokens: 65536,
            thinking_level_map: None,
            headers: Some(HashMap::from([("NVCF-POLL-SECONDS".into(), "3600".into())])),
            compat: None,
        },
    );

    map.insert(
        "nvidia/nemotron-3-super-120b-a12b".into(),
        Model {
            id: "nvidia/nemotron-3-super-120b-a12b".into(),
            name: "Nemotron 3 Super".into(),
            api: Api::OpenAiCompletions,
            provider: "nvidia".into(),
            base_url: "https://integrate.api.nvidia.com/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.2,
                output: 0.8,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 262144,
            max_tokens: 262144,
            thinking_level_map: None,
            headers: Some(HashMap::from([("NVCF-POLL-SECONDS".into(), "3600".into())])),
            compat: None,
        },
    );

    map.insert(
        "nvidia/nemotron-3-ultra-550b-a55b".into(),
        Model {
            id: "nvidia/nemotron-3-ultra-550b-a55b".into(),
            name: "Nemotron 3 Ultra 550B A55B".into(),
            api: Api::OpenAiCompletions,
            provider: "nvidia".into(),
            base_url: "https://integrate.api.nvidia.com/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.5,
                output: 2.5,
                cache_read: 0.15,
                cache_write: 0.0,
            },
            context_window: 1000000,
            max_tokens: 65536,
            thinking_level_map: None,
            headers: Some(HashMap::from([("NVCF-POLL-SECONDS".into(), "3600".into())])),
            compat: None,
        },
    );

    map.insert(
        "nvidia/nvidia-nemotron-nano-9b-v2".into(),
        Model {
            id: "nvidia/nvidia-nemotron-nano-9b-v2".into(),
            name: "nvidia-nemotron-nano-9b-v2".into(),
            api: Api::OpenAiCompletions,
            provider: "nvidia".into(),
            base_url: "https://integrate.api.nvidia.com/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.0,
                output: 0.0,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 131072,
            max_tokens: 131072,
            thinking_level_map: None,
            headers: Some(HashMap::from([("NVCF-POLL-SECONDS".into(), "3600".into())])),
            compat: None,
        },
    );

    map.insert(
        "openai/gpt-oss-120b".into(),
        Model {
            id: "openai/gpt-oss-120b".into(),
            name: "GPT-OSS-120B".into(),
            api: Api::OpenAiCompletions,
            provider: "nvidia".into(),
            base_url: "https://integrate.api.nvidia.com/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.0,
                output: 0.0,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 128000,
            max_tokens: 8192,
            thinking_level_map: None,
            headers: Some(HashMap::from([("NVCF-POLL-SECONDS".into(), "3600".into())])),
            compat: None,
        },
    );

    map.insert(
        "openai/gpt-oss-20b".into(),
        Model {
            id: "openai/gpt-oss-20b".into(),
            name: "GPT OSS 20B".into(),
            api: Api::OpenAiCompletions,
            provider: "nvidia".into(),
            base_url: "https://integrate.api.nvidia.com/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.0,
                output: 0.0,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 131072,
            max_tokens: 32768,
            thinking_level_map: None,
            headers: Some(HashMap::from([("NVCF-POLL-SECONDS".into(), "3600".into())])),
            compat: None,
        },
    );

    map.insert(
        "qwen/qwen3.5-122b-a10b".into(),
        Model {
            id: "qwen/qwen3.5-122b-a10b".into(),
            name: "Qwen3.5 122B-A10B".into(),
            api: Api::OpenAiCompletions,
            provider: "nvidia".into(),
            base_url: "https://integrate.api.nvidia.com/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.0,
                output: 0.0,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 262144,
            max_tokens: 65536,
            thinking_level_map: None,
            headers: Some(HashMap::from([("NVCF-POLL-SECONDS".into(), "3600".into())])),
            compat: None,
        },
    );

    map.insert(
        "stepfun-ai/step-3.5-flash".into(),
        Model {
            id: "stepfun-ai/step-3.5-flash".into(),
            name: "Step 3.5 Flash".into(),
            api: Api::OpenAiCompletions,
            provider: "nvidia".into(),
            base_url: "https://integrate.api.nvidia.com/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.0,
                output: 0.0,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 256000,
            max_tokens: 16384,
            thinking_level_map: None,
            headers: Some(HashMap::from([("NVCF-POLL-SECONDS".into(), "3600".into())])),
            compat: None,
        },
    );

    map.insert(
        "stepfun-ai/step-3.7-flash".into(),
        Model {
            id: "stepfun-ai/step-3.7-flash".into(),
            name: "Step 3.7 Flash".into(),
            api: Api::OpenAiCompletions,
            provider: "nvidia".into(),
            base_url: "https://integrate.api.nvidia.com/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.0,
                output: 0.0,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 256000,
            max_tokens: 16384,
            thinking_level_map: None,
            headers: Some(HashMap::from([("NVCF-POLL-SECONDS".into(), "3600".into())])),
            compat: None,
        },
    );

    map.insert(
        "z-ai/glm-5.1".into(),
        Model {
            id: "z-ai/glm-5.1".into(),
            name: "GLM-5.1".into(),
            api: Api::OpenAiCompletions,
            provider: "nvidia".into(),
            base_url: "https://integrate.api.nvidia.com/v1".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.0,
                output: 0.0,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 131072,
            max_tokens: 131072,
            thinking_level_map: None,
            headers: Some(HashMap::from([("NVCF-POLL-SECONDS".into(), "3600".into())])),
            compat: None,
        },
    );

    map
}

