//! Auto-generated model catalogue for `mistral`.
//! Do not edit manually — re-run `scripts/generate_models.py`.

use std::collections::HashMap;

use crate::types::{Api, InputModality, Model, ModelCost};

/// Model catalogue for the `mistral` provider.
pub fn models() -> HashMap<String, Model> {
    let mut map: HashMap<String, Model> = HashMap::new();

    map.insert(
        "codestral-latest".into(),
        Model {
            id: "codestral-latest".into(),
            name: "Codestral (latest)".into(),
            api: Api::MistralConversations,
            provider: "mistral".into(),
            base_url: "https://api.mistral.ai".into(),
            reasoning: false,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.3,
                output: 0.9,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 256000,
            max_tokens: 4096,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "devstral-2512".into(),
        Model {
            id: "devstral-2512".into(),
            name: "Devstral 2".into(),
            api: Api::MistralConversations,
            provider: "mistral".into(),
            base_url: "https://api.mistral.ai".into(),
            reasoning: false,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.4,
                output: 2.0,
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
        "devstral-latest".into(),
        Model {
            id: "devstral-latest".into(),
            name: "Devstral 2".into(),
            api: Api::MistralConversations,
            provider: "mistral".into(),
            base_url: "https://api.mistral.ai".into(),
            reasoning: false,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.4,
                output: 2.0,
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
        "devstral-medium-2507".into(),
        Model {
            id: "devstral-medium-2507".into(),
            name: "Devstral Medium".into(),
            api: Api::MistralConversations,
            provider: "mistral".into(),
            base_url: "https://api.mistral.ai".into(),
            reasoning: false,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.4,
                output: 2.0,
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
        "devstral-medium-latest".into(),
        Model {
            id: "devstral-medium-latest".into(),
            name: "Devstral 2 (latest)".into(),
            api: Api::MistralConversations,
            provider: "mistral".into(),
            base_url: "https://api.mistral.ai".into(),
            reasoning: false,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.4,
                output: 2.0,
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
        "devstral-small-2505".into(),
        Model {
            id: "devstral-small-2505".into(),
            name: "Devstral Small 2505".into(),
            api: Api::MistralConversations,
            provider: "mistral".into(),
            base_url: "https://api.mistral.ai".into(),
            reasoning: false,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.1,
                output: 0.3,
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
        "devstral-small-2507".into(),
        Model {
            id: "devstral-small-2507".into(),
            name: "Devstral Small".into(),
            api: Api::MistralConversations,
            provider: "mistral".into(),
            base_url: "https://api.mistral.ai".into(),
            reasoning: false,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.1,
                output: 0.3,
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
        "labs-devstral-small-2512".into(),
        Model {
            id: "labs-devstral-small-2512".into(),
            name: "Devstral Small 2".into(),
            api: Api::MistralConversations,
            provider: "mistral".into(),
            base_url: "https://api.mistral.ai".into(),
            reasoning: false,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.0,
                output: 0.0,
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
        "magistral-medium-latest".into(),
        Model {
            id: "magistral-medium-latest".into(),
            name: "Magistral Medium (latest)".into(),
            api: Api::MistralConversations,
            provider: "mistral".into(),
            base_url: "https://api.mistral.ai".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 2.0,
                output: 5.0,
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
        "magistral-small".into(),
        Model {
            id: "magistral-small".into(),
            name: "Magistral Small".into(),
            api: Api::MistralConversations,
            provider: "mistral".into(),
            base_url: "https://api.mistral.ai".into(),
            reasoning: true,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.5,
                output: 1.5,
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
        "ministral-3b-latest".into(),
        Model {
            id: "ministral-3b-latest".into(),
            name: "Ministral 3B (latest)".into(),
            api: Api::MistralConversations,
            provider: "mistral".into(),
            base_url: "https://api.mistral.ai".into(),
            reasoning: false,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.04,
                output: 0.04,
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
        "ministral-8b-latest".into(),
        Model {
            id: "ministral-8b-latest".into(),
            name: "Ministral 8B (latest)".into(),
            api: Api::MistralConversations,
            provider: "mistral".into(),
            base_url: "https://api.mistral.ai".into(),
            reasoning: false,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.1,
                output: 0.1,
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
        "mistral-large-2411".into(),
        Model {
            id: "mistral-large-2411".into(),
            name: "Mistral Large 2.1".into(),
            api: Api::MistralConversations,
            provider: "mistral".into(),
            base_url: "https://api.mistral.ai".into(),
            reasoning: false,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 2.0,
                output: 6.0,
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
        "mistral-large-2512".into(),
        Model {
            id: "mistral-large-2512".into(),
            name: "Mistral Large 3".into(),
            api: Api::MistralConversations,
            provider: "mistral".into(),
            base_url: "https://api.mistral.ai".into(),
            reasoning: false,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.5,
                output: 1.5,
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
        "mistral-large-latest".into(),
        Model {
            id: "mistral-large-latest".into(),
            name: "Mistral Large (latest)".into(),
            api: Api::MistralConversations,
            provider: "mistral".into(),
            base_url: "https://api.mistral.ai".into(),
            reasoning: false,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.5,
                output: 1.5,
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
        "mistral-medium-2505".into(),
        Model {
            id: "mistral-medium-2505".into(),
            name: "Mistral Medium 3".into(),
            api: Api::MistralConversations,
            provider: "mistral".into(),
            base_url: "https://api.mistral.ai".into(),
            reasoning: false,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.4,
                output: 2.0,
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
        "mistral-medium-2508".into(),
        Model {
            id: "mistral-medium-2508".into(),
            name: "Mistral Medium 3.1".into(),
            api: Api::MistralConversations,
            provider: "mistral".into(),
            base_url: "https://api.mistral.ai".into(),
            reasoning: false,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.4,
                output: 2.0,
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
        "mistral-medium-2604".into(),
        Model {
            id: "mistral-medium-2604".into(),
            name: "Mistral Medium 3.5".into(),
            api: Api::MistralConversations,
            provider: "mistral".into(),
            base_url: "https://api.mistral.ai".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 1.5,
                output: 7.5,
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
        "mistral-medium-3.5".into(),
        Model {
            id: "mistral-medium-3.5".into(),
            name: "Mistral Medium 3.5".into(),
            api: Api::MistralConversations,
            provider: "mistral".into(),
            base_url: "https://api.mistral.ai".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 1.5,
                output: 7.5,
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
        "mistral-medium-latest".into(),
        Model {
            id: "mistral-medium-latest".into(),
            name: "Mistral Medium (latest)".into(),
            api: Api::MistralConversations,
            provider: "mistral".into(),
            base_url: "https://api.mistral.ai".into(),
            reasoning: false,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.4,
                output: 2.0,
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
        "mistral-nemo".into(),
        Model {
            id: "mistral-nemo".into(),
            name: "Mistral Nemo".into(),
            api: Api::MistralConversations,
            provider: "mistral".into(),
            base_url: "https://api.mistral.ai".into(),
            reasoning: false,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.15,
                output: 0.15,
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
        "mistral-small-2506".into(),
        Model {
            id: "mistral-small-2506".into(),
            name: "Mistral Small 3.2".into(),
            api: Api::MistralConversations,
            provider: "mistral".into(),
            base_url: "https://api.mistral.ai".into(),
            reasoning: false,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.1,
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
        "mistral-small-2603".into(),
        Model {
            id: "mistral-small-2603".into(),
            name: "Mistral Small 4".into(),
            api: Api::MistralConversations,
            provider: "mistral".into(),
            base_url: "https://api.mistral.ai".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.15,
                output: 0.6,
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
        "mistral-small-latest".into(),
        Model {
            id: "mistral-small-latest".into(),
            name: "Mistral Small (latest)".into(),
            api: Api::MistralConversations,
            provider: "mistral".into(),
            base_url: "https://api.mistral.ai".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.15,
                output: 0.6,
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
        "open-mistral-7b".into(),
        Model {
            id: "open-mistral-7b".into(),
            name: "Mistral 7B".into(),
            api: Api::MistralConversations,
            provider: "mistral".into(),
            base_url: "https://api.mistral.ai".into(),
            reasoning: false,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.25,
                output: 0.25,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 8000,
            max_tokens: 8000,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "open-mistral-nemo".into(),
        Model {
            id: "open-mistral-nemo".into(),
            name: "Open Mistral Nemo".into(),
            api: Api::MistralConversations,
            provider: "mistral".into(),
            base_url: "https://api.mistral.ai".into(),
            reasoning: false,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.15,
                output: 0.15,
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
        "open-mixtral-8x22b".into(),
        Model {
            id: "open-mixtral-8x22b".into(),
            name: "Mixtral 8x22B".into(),
            api: Api::MistralConversations,
            provider: "mistral".into(),
            base_url: "https://api.mistral.ai".into(),
            reasoning: false,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 2.0,
                output: 6.0,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 64000,
            max_tokens: 64000,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "open-mixtral-8x7b".into(),
        Model {
            id: "open-mixtral-8x7b".into(),
            name: "Mixtral 8x7B".into(),
            api: Api::MistralConversations,
            provider: "mistral".into(),
            base_url: "https://api.mistral.ai".into(),
            reasoning: false,
            input: vec![InputModality::Text],
            cost: ModelCost {
                input: 0.7,
                output: 0.7,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 32000,
            max_tokens: 32000,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "pixtral-12b".into(),
        Model {
            id: "pixtral-12b".into(),
            name: "Pixtral 12B".into(),
            api: Api::MistralConversations,
            provider: "mistral".into(),
            base_url: "https://api.mistral.ai".into(),
            reasoning: false,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.15,
                output: 0.15,
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
        "pixtral-large-latest".into(),
        Model {
            id: "pixtral-large-latest".into(),
            name: "Pixtral Large (latest)".into(),
            api: Api::MistralConversations,
            provider: "mistral".into(),
            base_url: "https://api.mistral.ai".into(),
            reasoning: false,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 2.0,
                output: 6.0,
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

    map
}
