//! Auto-generated model catalogue for `cloudflare-workers-ai`.
//! Do not edit manually — re-run `scripts/generate_models.py`.

use std::collections::HashMap;

use crate::types::{Api, InputModality, Model, ModelCost};

/// Model catalogue for the `cloudflare-workers-ai` provider.
pub fn models() -> HashMap<String, Model> {
    let mut map: HashMap<String, Model> = HashMap::new();

    map.insert(
        "@cf/google/gemma-4-26b-a4b-it".into(),
        Model {
            id: "@cf/google/gemma-4-26b-a4b-it".into(),
            name: "Gemma 4 26B A4B IT".into(),
            api: Api::OpenAiCompletions,
            provider: "cloudflare-workers-ai".into(),
            base_url: "https://api.cloudflare.com/client/v4/accounts/{CLOUDFLARE_ACCOUNT_ID}/ai/v1"
                .into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.1,
                output: 0.3,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 256000,
            max_tokens: 16384,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "@cf/ibm-granite/granite-4.0-h-micro".into(),
        Model {
            id: "@cf/ibm-granite/granite-4.0-h-micro".into(),
            name: "Granite 4.0 H Micro".into(),
            api: Api::OpenAiCompletions,
            provider: "cloudflare-workers-ai".into(),
            base_url: "https://api.cloudflare.com/client/v4/accounts/{CLOUDFLARE_ACCOUNT_ID}/ai/v1"
                .into(),
            reasoning: false,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.017,
                output: 0.112,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 131000,
            max_tokens: 131000,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "@cf/meta/llama-3.3-70b-instruct-fp8-fast".into(),
        Model {
            id: "@cf/meta/llama-3.3-70b-instruct-fp8-fast".into(),
            name: "Llama 3.3 70B Instruct fp8 Fast".into(),
            api: Api::OpenAiCompletions,
            provider: "cloudflare-workers-ai".into(),
            base_url: "https://api.cloudflare.com/client/v4/accounts/{CLOUDFLARE_ACCOUNT_ID}/ai/v1"
                .into(),
            reasoning: false,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.293,
                output: 2.253,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 24000,
            max_tokens: 24000,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "@cf/meta/llama-4-scout-17b-16e-instruct".into(),
        Model {
            id: "@cf/meta/llama-4-scout-17b-16e-instruct".into(),
            name: "Llama 4 Scout 17B 16E Instruct".into(),
            api: Api::OpenAiCompletions,
            provider: "cloudflare-workers-ai".into(),
            base_url: "https://api.cloudflare.com/client/v4/accounts/{CLOUDFLARE_ACCOUNT_ID}/ai/v1"
                .into(),
            reasoning: false,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.27,
                output: 0.85,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 131000,
            max_tokens: 16384,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "@cf/mistralai/mistral-small-3.1-24b-instruct".into(),
        Model {
            id: "@cf/mistralai/mistral-small-3.1-24b-instruct".into(),
            name: "Mistral Small 3.1 24B Instruct".into(),
            api: Api::OpenAiCompletions,
            provider: "cloudflare-workers-ai".into(),
            base_url: "https://api.cloudflare.com/client/v4/accounts/{CLOUDFLARE_ACCOUNT_ID}/ai/v1"
                .into(),
            reasoning: false,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.351,
                output: 0.555,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 128000,
            max_tokens: 128000,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "@cf/moonshotai/kimi-k2.6".into(),
        Model {
            id: "@cf/moonshotai/kimi-k2.6".into(),
            name: "Kimi K2.6".into(),
            api: Api::OpenAiCompletions,
            provider: "cloudflare-workers-ai".into(),
            base_url: "https://api.cloudflare.com/client/v4/accounts/{CLOUDFLARE_ACCOUNT_ID}/ai/v1"
                .into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.95,
                output: 4.0,
                cache_read: 0.16,
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
        "@cf/moonshotai/kimi-k2.7-code".into(),
        Model {
            id: "@cf/moonshotai/kimi-k2.7-code".into(),
            name: "Kimi K2.7 Code".into(),
            api: Api::OpenAiCompletions,
            provider: "cloudflare-workers-ai".into(),
            base_url: "https://api.cloudflare.com/client/v4/accounts/{CLOUDFLARE_ACCOUNT_ID}/ai/v1"
                .into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.95,
                output: 4.0,
                cache_read: 0.19,
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
        "@cf/nvidia/nemotron-3-120b-a12b".into(),
        Model {
            id: "@cf/nvidia/nemotron-3-120b-a12b".into(),
            name: "Nemotron 3 Super 120B".into(),
            api: Api::OpenAiCompletions,
            provider: "cloudflare-workers-ai".into(),
            base_url: "https://api.cloudflare.com/client/v4/accounts/{CLOUDFLARE_ACCOUNT_ID}/ai/v1"
                .into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.5,
                output: 1.5,
                cache_read: 0.0,
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
        "@cf/openai/gpt-oss-120b".into(),
        Model {
            id: "@cf/openai/gpt-oss-120b".into(),
            name: "GPT OSS 120B".into(),
            api: Api::OpenAiCompletions,
            provider: "cloudflare-workers-ai".into(),
            base_url: "https://api.cloudflare.com/client/v4/accounts/{CLOUDFLARE_ACCOUNT_ID}/ai/v1"
                .into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.35,
                output: 0.75,
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
        "@cf/openai/gpt-oss-20b".into(),
        Model {
            id: "@cf/openai/gpt-oss-20b".into(),
            name: "GPT OSS 20B".into(),
            api: Api::OpenAiCompletions,
            provider: "cloudflare-workers-ai".into(),
            base_url: "https://api.cloudflare.com/client/v4/accounts/{CLOUDFLARE_ACCOUNT_ID}/ai/v1"
                .into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.2,
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
        "@cf/qwen/qwen3-30b-a3b-fp8".into(),
        Model {
            id: "@cf/qwen/qwen3-30b-a3b-fp8".into(),
            name: "Qwen3 30B A3b fp8".into(),
            api: Api::OpenAiCompletions,
            provider: "cloudflare-workers-ai".into(),
            base_url: "https://api.cloudflare.com/client/v4/accounts/{CLOUDFLARE_ACCOUNT_ID}/ai/v1"
                .into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.0509,
                output: 0.335,
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
        "@cf/zai-org/glm-4.7-flash".into(),
        Model {
            id: "@cf/zai-org/glm-4.7-flash".into(),
            name: "GLM-4.7-Flash".into(),
            api: Api::OpenAiCompletions,
            provider: "cloudflare-workers-ai".into(),
            base_url: "https://api.cloudflare.com/client/v4/accounts/{CLOUDFLARE_ACCOUNT_ID}/ai/v1"
                .into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.0605,
                output: 0.4,
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
        "@cf/zai-org/glm-5.2".into(),
        Model {
            id: "@cf/zai-org/glm-5.2".into(),
            name: "Glm 5.2".into(),
            api: Api::OpenAiCompletions,
            provider: "cloudflare-workers-ai".into(),
            base_url: "https://api.cloudflare.com/client/v4/accounts/{CLOUDFLARE_ACCOUNT_ID}/ai/v1"
                .into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 1.4,
                output: 4.4,
                cache_read: 0.26,
                cache_write: 0.0,
            },
            context_window: 262144,
            max_tokens: 262144,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map
}
