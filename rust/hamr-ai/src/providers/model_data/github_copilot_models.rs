//! Auto-generated model catalogue for `github-copilot`.
//! Do not edit manually — re-run `scripts/generate_models.py`.

use std::collections::HashMap;

use crate::types::{Api, InputModality, Model, ModelCost, ModelThinkingLevel};

/// Model catalogue for the `github-copilot` provider.
pub fn models() -> HashMap<String, Model> {
    let mut map: HashMap<String, Model> = HashMap::new();

    map.insert(
        "claude-fable-5".into(),
        Model {
            id: "claude-fable-5".into(),
            name: "Claude Fable 5".into(),
            api: Api::OpenAiCompletions,
            provider: "github-copilot".into(),
            base_url: "https://api.individual.githubcopilot.com".into(),
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
            thinking_level_map: None,
            headers: Some(HashMap::from([
                ("User-Agent".into(), "GitHubCopilotChat/0.35.0".into()),
                ("Editor-Version".into(), "vscode/1.107.0".into()),
                ("Editor-Plugin-Version".into(), "copilot-chat/0.35.0".into()),
                ("Copilot-Integration-Id".into(), "vscode-chat".into()),
            ])),
            compat: None,
        },
    );

    map.insert(
        "claude-haiku-4.5".into(),
        Model {
            id: "claude-haiku-4.5".into(),
            name: "Claude Haiku 4.5 (latest)".into(),
            api: Api::AnthropicMessages,
            provider: "github-copilot".into(),
            base_url: "https://api.individual.githubcopilot.com".into(),
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
            headers: Some(HashMap::from([
                ("User-Agent".into(), "GitHubCopilotChat/0.35.0".into()),
                ("Editor-Version".into(), "vscode/1.107.0".into()),
                ("Editor-Plugin-Version".into(), "copilot-chat/0.35.0".into()),
                ("Copilot-Integration-Id".into(), "vscode-chat".into()),
            ])),
            compat: None,
        },
    );

    map.insert(
        "claude-opus-4.5".into(),
        Model {
            id: "claude-opus-4.5".into(),
            name: "Claude Opus 4.5 (latest)".into(),
            api: Api::AnthropicMessages,
            provider: "github-copilot".into(),
            base_url: "https://api.individual.githubcopilot.com".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 5.0,
                output: 25.0,
                cache_read: 0.5,
                cache_write: 6.25,
            },
            context_window: 200000,
            max_tokens: 32000,
            thinking_level_map: None,
            headers: Some(HashMap::from([
                ("User-Agent".into(), "GitHubCopilotChat/0.35.0".into()),
                ("Editor-Version".into(), "vscode/1.107.0".into()),
                ("Editor-Plugin-Version".into(), "copilot-chat/0.35.0".into()),
                ("Copilot-Integration-Id".into(), "vscode-chat".into()),
            ])),
            compat: None,
        },
    );

    map.insert(
        "claude-opus-4.6".into(),
        Model {
            id: "claude-opus-4.6".into(),
            name: "Claude Opus 4.6".into(),
            api: Api::AnthropicMessages,
            provider: "github-copilot".into(),
            base_url: "https://api.individual.githubcopilot.com".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 5.0,
                output: 25.0,
                cache_read: 0.5,
                cache_write: 6.25,
            },
            context_window: 1000000,
            max_tokens: 32000,
            thinking_level_map: Some(HashMap::from([(
                ModelThinkingLevel::XHigh,
                Some("max".into()),
            )])),
            headers: Some(HashMap::from([
                ("User-Agent".into(), "GitHubCopilotChat/0.35.0".into()),
                ("Editor-Version".into(), "vscode/1.107.0".into()),
                ("Editor-Plugin-Version".into(), "copilot-chat/0.35.0".into()),
                ("Copilot-Integration-Id".into(), "vscode-chat".into()),
            ])),
            compat: None,
        },
    );

    map.insert(
        "claude-opus-4.7".into(),
        Model {
            id: "claude-opus-4.7".into(),
            name: "Claude Opus 4.7".into(),
            api: Api::AnthropicMessages,
            provider: "github-copilot".into(),
            base_url: "https://api.individual.githubcopilot.com".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 5.0,
                output: 25.0,
                cache_read: 0.5,
                cache_write: 6.25,
            },
            context_window: 200000,
            max_tokens: 32000,
            thinking_level_map: Some(HashMap::from([
                (ModelThinkingLevel::XHigh, Some("xhigh".into())),
                (ModelThinkingLevel::Minimal, Some("low".into())),
            ])),
            headers: Some(HashMap::from([
                ("User-Agent".into(), "GitHubCopilotChat/0.35.0".into()),
                ("Editor-Version".into(), "vscode/1.107.0".into()),
                ("Editor-Plugin-Version".into(), "copilot-chat/0.35.0".into()),
                ("Copilot-Integration-Id".into(), "vscode-chat".into()),
            ])),
            compat: None,
        },
    );

    map.insert(
        "claude-opus-4.8".into(),
        Model {
            id: "claude-opus-4.8".into(),
            name: "Claude Opus 4.8".into(),
            api: Api::AnthropicMessages,
            provider: "github-copilot".into(),
            base_url: "https://api.individual.githubcopilot.com".into(),
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
            thinking_level_map: Some(HashMap::from([
                (ModelThinkingLevel::XHigh, Some("xhigh".into())),
                (ModelThinkingLevel::Minimal, Some("low".into())),
            ])),
            headers: Some(HashMap::from([
                ("User-Agent".into(), "GitHubCopilotChat/0.35.0".into()),
                ("Editor-Version".into(), "vscode/1.107.0".into()),
                ("Editor-Plugin-Version".into(), "copilot-chat/0.35.0".into()),
                ("Copilot-Integration-Id".into(), "vscode-chat".into()),
            ])),
            compat: None,
        },
    );

    map.insert(
        "claude-sonnet-4".into(),
        Model {
            id: "claude-sonnet-4".into(),
            name: "Claude Sonnet 4 (latest)".into(),
            api: Api::AnthropicMessages,
            provider: "github-copilot".into(),
            base_url: "https://api.individual.githubcopilot.com".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 3.0,
                output: 15.0,
                cache_read: 0.3,
                cache_write: 3.75,
            },
            context_window: 216000,
            max_tokens: 16000,
            thinking_level_map: None,
            headers: Some(HashMap::from([
                ("User-Agent".into(), "GitHubCopilotChat/0.35.0".into()),
                ("Editor-Version".into(), "vscode/1.107.0".into()),
                ("Editor-Plugin-Version".into(), "copilot-chat/0.35.0".into()),
                ("Copilot-Integration-Id".into(), "vscode-chat".into()),
            ])),
            compat: None,
        },
    );

    map.insert(
        "claude-sonnet-4.5".into(),
        Model {
            id: "claude-sonnet-4.5".into(),
            name: "Claude Sonnet 4.5 (latest)".into(),
            api: Api::AnthropicMessages,
            provider: "github-copilot".into(),
            base_url: "https://api.individual.githubcopilot.com".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 3.0,
                output: 15.0,
                cache_read: 0.3,
                cache_write: 3.75,
            },
            context_window: 200000,
            max_tokens: 32000,
            thinking_level_map: None,
            headers: Some(HashMap::from([
                ("User-Agent".into(), "GitHubCopilotChat/0.35.0".into()),
                ("Editor-Version".into(), "vscode/1.107.0".into()),
                ("Editor-Plugin-Version".into(), "copilot-chat/0.35.0".into()),
                ("Copilot-Integration-Id".into(), "vscode-chat".into()),
            ])),
            compat: None,
        },
    );

    map.insert(
        "claude-sonnet-4.6".into(),
        Model {
            id: "claude-sonnet-4.6".into(),
            name: "Claude Sonnet 4.6".into(),
            api: Api::AnthropicMessages,
            provider: "github-copilot".into(),
            base_url: "https://api.individual.githubcopilot.com".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 3.0,
                output: 15.0,
                cache_read: 0.3,
                cache_write: 3.75,
            },
            context_window: 1000000,
            max_tokens: 32000,
            thinking_level_map: Some(HashMap::from([
                (ModelThinkingLevel::Minimal, Some("low".into())),
                (ModelThinkingLevel::XHigh, Some("max".into())),
            ])),
            headers: Some(HashMap::from([
                ("User-Agent".into(), "GitHubCopilotChat/0.35.0".into()),
                ("Editor-Version".into(), "vscode/1.107.0".into()),
                ("Editor-Plugin-Version".into(), "copilot-chat/0.35.0".into()),
                ("Copilot-Integration-Id".into(), "vscode-chat".into()),
            ])),
            compat: None,
        },
    );

    map.insert(
        "gemini-2.5-pro".into(),
        Model {
            id: "gemini-2.5-pro".into(),
            name: "Gemini 2.5 Pro".into(),
            api: Api::OpenAiCompletions,
            provider: "github-copilot".into(),
            base_url: "https://api.individual.githubcopilot.com".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 1.25,
                output: 10.0,
                cache_read: 0.125,
                cache_write: 0.0,
            },
            context_window: 128000,
            max_tokens: 64000,
            thinking_level_map: None,
            headers: Some(HashMap::from([
                ("User-Agent".into(), "GitHubCopilotChat/0.35.0".into()),
                ("Editor-Version".into(), "vscode/1.107.0".into()),
                ("Editor-Plugin-Version".into(), "copilot-chat/0.35.0".into()),
                ("Copilot-Integration-Id".into(), "vscode-chat".into()),
            ])),
            compat: None,
        },
    );

    map.insert(
        "gemini-3-flash-preview".into(),
        Model {
            id: "gemini-3-flash-preview".into(),
            name: "Gemini 3 Flash Preview".into(),
            api: Api::OpenAiCompletions,
            provider: "github-copilot".into(),
            base_url: "https://api.individual.githubcopilot.com".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.5,
                output: 3.0,
                cache_read: 0.05,
                cache_write: 0.0,
            },
            context_window: 128000,
            max_tokens: 64000,
            thinking_level_map: None,
            headers: Some(HashMap::from([
                ("User-Agent".into(), "GitHubCopilotChat/0.35.0".into()),
                ("Editor-Version".into(), "vscode/1.107.0".into()),
                ("Editor-Plugin-Version".into(), "copilot-chat/0.35.0".into()),
                ("Copilot-Integration-Id".into(), "vscode-chat".into()),
            ])),
            compat: None,
        },
    );

    map.insert(
        "gemini-3.1-pro-preview".into(),
        Model {
            id: "gemini-3.1-pro-preview".into(),
            name: "Gemini 3.1 Pro Preview".into(),
            api: Api::OpenAiCompletions,
            provider: "github-copilot".into(),
            base_url: "https://api.individual.githubcopilot.com".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 2.0,
                output: 12.0,
                cache_read: 0.2,
                cache_write: 0.0,
            },
            context_window: 200000,
            max_tokens: 64000,
            thinking_level_map: None,
            headers: Some(HashMap::from([
                ("User-Agent".into(), "GitHubCopilotChat/0.35.0".into()),
                ("Editor-Version".into(), "vscode/1.107.0".into()),
                ("Editor-Plugin-Version".into(), "copilot-chat/0.35.0".into()),
                ("Copilot-Integration-Id".into(), "vscode-chat".into()),
            ])),
            compat: None,
        },
    );

    map.insert(
        "gemini-3.5-flash".into(),
        Model {
            id: "gemini-3.5-flash".into(),
            name: "Gemini 3.5 Flash".into(),
            api: Api::OpenAiCompletions,
            provider: "github-copilot".into(),
            base_url: "https://api.individual.githubcopilot.com".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 1.5,
                output: 9.0,
                cache_read: 0.15,
                cache_write: 0.0,
            },
            context_window: 200000,
            max_tokens: 64000,
            thinking_level_map: None,
            headers: Some(HashMap::from([
                ("User-Agent".into(), "GitHubCopilotChat/0.35.0".into()),
                ("Editor-Version".into(), "vscode/1.107.0".into()),
                ("Editor-Plugin-Version".into(), "copilot-chat/0.35.0".into()),
                ("Copilot-Integration-Id".into(), "vscode-chat".into()),
            ])),
            compat: None,
        },
    );

    map.insert(
        "gpt-4.1".into(),
        Model {
            id: "gpt-4.1".into(),
            name: "GPT-4.1".into(),
            api: Api::OpenAiCompletions,
            provider: "github-copilot".into(),
            base_url: "https://api.individual.githubcopilot.com".into(),
            reasoning: false,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 2.0,
                output: 8.0,
                cache_read: 0.5,
                cache_write: 0.0,
            },
            context_window: 128000,
            max_tokens: 16384,
            thinking_level_map: None,
            headers: Some(HashMap::from([
                ("User-Agent".into(), "GitHubCopilotChat/0.35.0".into()),
                ("Editor-Version".into(), "vscode/1.107.0".into()),
                ("Editor-Plugin-Version".into(), "copilot-chat/0.35.0".into()),
                ("Copilot-Integration-Id".into(), "vscode-chat".into()),
            ])),
            compat: None,
        },
    );

    map.insert(
        "gpt-5-mini".into(),
        Model {
            id: "gpt-5-mini".into(),
            name: "GPT-5 Mini".into(),
            api: Api::OpenAiResponses,
            provider: "github-copilot".into(),
            base_url: "https://api.individual.githubcopilot.com".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.25,
                output: 2.0,
                cache_read: 0.025,
                cache_write: 0.0,
            },
            context_window: 264000,
            max_tokens: 64000,
            thinking_level_map: Some(HashMap::from([
                (ModelThinkingLevel::Off, None),
                (ModelThinkingLevel::Minimal, Some("low".into())),
            ])),
            headers: Some(HashMap::from([
                ("User-Agent".into(), "GitHubCopilotChat/0.35.0".into()),
                ("Editor-Version".into(), "vscode/1.107.0".into()),
                ("Editor-Plugin-Version".into(), "copilot-chat/0.35.0".into()),
                ("Copilot-Integration-Id".into(), "vscode-chat".into()),
            ])),
            compat: None,
        },
    );

    map.insert(
        "gpt-5.2".into(),
        Model {
            id: "gpt-5.2".into(),
            name: "GPT-5.2".into(),
            api: Api::OpenAiResponses,
            provider: "github-copilot".into(),
            base_url: "https://api.individual.githubcopilot.com".into(),
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
                (ModelThinkingLevel::Minimal, Some("low".into())),
                (ModelThinkingLevel::XHigh, Some("xhigh".into())),
            ])),
            headers: Some(HashMap::from([
                ("User-Agent".into(), "GitHubCopilotChat/0.35.0".into()),
                ("Editor-Version".into(), "vscode/1.107.0".into()),
                ("Editor-Plugin-Version".into(), "copilot-chat/0.35.0".into()),
                ("Copilot-Integration-Id".into(), "vscode-chat".into()),
            ])),
            compat: None,
        },
    );

    map.insert(
        "gpt-5.2-codex".into(),
        Model {
            id: "gpt-5.2-codex".into(),
            name: "GPT-5.2 Codex".into(),
            api: Api::OpenAiResponses,
            provider: "github-copilot".into(),
            base_url: "https://api.individual.githubcopilot.com".into(),
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
                (ModelThinkingLevel::Minimal, Some("low".into())),
                (ModelThinkingLevel::XHigh, Some("xhigh".into())),
            ])),
            headers: Some(HashMap::from([
                ("User-Agent".into(), "GitHubCopilotChat/0.35.0".into()),
                ("Editor-Version".into(), "vscode/1.107.0".into()),
                ("Editor-Plugin-Version".into(), "copilot-chat/0.35.0".into()),
                ("Copilot-Integration-Id".into(), "vscode-chat".into()),
            ])),
            compat: None,
        },
    );

    map.insert(
        "gpt-5.3-codex".into(),
        Model {
            id: "gpt-5.3-codex".into(),
            name: "GPT-5.3 Codex".into(),
            api: Api::OpenAiResponses,
            provider: "github-copilot".into(),
            base_url: "https://api.individual.githubcopilot.com".into(),
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
                (ModelThinkingLevel::Minimal, Some("low".into())),
                (ModelThinkingLevel::XHigh, Some("xhigh".into())),
            ])),
            headers: Some(HashMap::from([
                ("User-Agent".into(), "GitHubCopilotChat/0.35.0".into()),
                ("Editor-Version".into(), "vscode/1.107.0".into()),
                ("Editor-Plugin-Version".into(), "copilot-chat/0.35.0".into()),
                ("Copilot-Integration-Id".into(), "vscode-chat".into()),
            ])),
            compat: None,
        },
    );

    map.insert(
        "gpt-5.4".into(),
        Model {
            id: "gpt-5.4".into(),
            name: "GPT-5.4".into(),
            api: Api::OpenAiResponses,
            provider: "github-copilot".into(),
            base_url: "https://api.individual.githubcopilot.com".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 2.5,
                output: 15.0,
                cache_read: 0.25,
                cache_write: 0.0,
            },
            context_window: 400000,
            max_tokens: 128000,
            thinking_level_map: Some(HashMap::from([
                (ModelThinkingLevel::Off, None),
                (ModelThinkingLevel::Minimal, Some("low".into())),
                (ModelThinkingLevel::XHigh, Some("xhigh".into())),
            ])),
            headers: Some(HashMap::from([
                ("User-Agent".into(), "GitHubCopilotChat/0.35.0".into()),
                ("Editor-Version".into(), "vscode/1.107.0".into()),
                ("Editor-Plugin-Version".into(), "copilot-chat/0.35.0".into()),
                ("Copilot-Integration-Id".into(), "vscode-chat".into()),
            ])),
            compat: None,
        },
    );

    map.insert(
        "gpt-5.4-mini".into(),
        Model {
            id: "gpt-5.4-mini".into(),
            name: "GPT-5.4 mini".into(),
            api: Api::OpenAiResponses,
            provider: "github-copilot".into(),
            base_url: "https://api.individual.githubcopilot.com".into(),
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
                (ModelThinkingLevel::Minimal, Some("low".into())),
                (ModelThinkingLevel::XHigh, Some("xhigh".into())),
            ])),
            headers: Some(HashMap::from([
                ("User-Agent".into(), "GitHubCopilotChat/0.35.0".into()),
                ("Editor-Version".into(), "vscode/1.107.0".into()),
                ("Editor-Plugin-Version".into(), "copilot-chat/0.35.0".into()),
                ("Copilot-Integration-Id".into(), "vscode-chat".into()),
            ])),
            compat: None,
        },
    );

    map.insert(
        "gpt-5.4-nano".into(),
        Model {
            id: "gpt-5.4-nano".into(),
            name: "GPT-5.4 nano".into(),
            api: Api::OpenAiResponses,
            provider: "github-copilot".into(),
            base_url: "https://api.individual.githubcopilot.com".into(),
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
                (ModelThinkingLevel::Minimal, Some("low".into())),
                (ModelThinkingLevel::XHigh, Some("xhigh".into())),
            ])),
            headers: Some(HashMap::from([
                ("User-Agent".into(), "GitHubCopilotChat/0.35.0".into()),
                ("Editor-Version".into(), "vscode/1.107.0".into()),
                ("Editor-Plugin-Version".into(), "copilot-chat/0.35.0".into()),
                ("Copilot-Integration-Id".into(), "vscode-chat".into()),
            ])),
            compat: None,
        },
    );

    map.insert(
        "gpt-5.5".into(),
        Model {
            id: "gpt-5.5".into(),
            name: "GPT-5.5".into(),
            api: Api::OpenAiResponses,
            provider: "github-copilot".into(),
            base_url: "https://api.individual.githubcopilot.com".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 5.0,
                output: 30.0,
                cache_read: 0.5,
                cache_write: 0.0,
            },
            context_window: 400000,
            max_tokens: 128000,
            thinking_level_map: Some(HashMap::from([
                (ModelThinkingLevel::Off, None),
                (ModelThinkingLevel::Minimal, Some("low".into())),
                (ModelThinkingLevel::XHigh, Some("xhigh".into())),
            ])),
            headers: Some(HashMap::from([
                ("User-Agent".into(), "GitHubCopilotChat/0.35.0".into()),
                ("Editor-Version".into(), "vscode/1.107.0".into()),
                ("Editor-Plugin-Version".into(), "copilot-chat/0.35.0".into()),
                ("Copilot-Integration-Id".into(), "vscode-chat".into()),
            ])),
            compat: None,
        },
    );

    map
}
