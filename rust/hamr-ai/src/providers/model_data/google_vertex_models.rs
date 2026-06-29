//! Auto-generated model catalogue for `google-vertex`.
//! Do not edit manually — re-run `scripts/generate_models.py`.

use std::collections::HashMap;

use crate::types::{Api, InputModality, Model, ModelCost, ModelThinkingLevel};

/// Model catalogue for the `google-vertex` provider.
pub fn models() -> HashMap<String, Model> {
    let mut map: HashMap<String, Model> = HashMap::new();

    map.insert(
        "gemini-2.5-flash".into(),
        Model {
            id: "gemini-2.5-flash".into(),
            name: "Gemini 2.5 Flash".into(),
            api: Api::GoogleVertex,
            provider: "google-vertex".into(),
            base_url: "https://{location}-aiplatform.googleapis.com".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.3,
                output: 2.5,
                cache_read: 0.03,
                cache_write: 0.0,
            },
            context_window: 1048576,
            max_tokens: 65536,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "gemini-2.5-flash-lite".into(),
        Model {
            id: "gemini-2.5-flash-lite".into(),
            name: "Gemini 2.5 Flash-Lite".into(),
            api: Api::GoogleVertex,
            provider: "google-vertex".into(),
            base_url: "https://{location}-aiplatform.googleapis.com".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.1,
                output: 0.4,
                cache_read: 0.01,
                cache_write: 0.0,
            },
            context_window: 1048576,
            max_tokens: 65536,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "gemini-2.5-pro".into(),
        Model {
            id: "gemini-2.5-pro".into(),
            name: "Gemini 2.5 Pro".into(),
            api: Api::GoogleVertex,
            provider: "google-vertex".into(),
            base_url: "https://{location}-aiplatform.googleapis.com".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 1.25,
                output: 10.0,
                cache_read: 0.125,
                cache_write: 0.0,
            },
            context_window: 1048576,
            max_tokens: 65536,
            thinking_level_map: None,
            headers: None,
            compat: None,
        },
    );

    map.insert(
        "gemini-3-flash-preview".into(),
        Model {
            id: "gemini-3-flash-preview".into(),
            name: "Gemini 3 Flash Preview".into(),
            api: Api::GoogleVertex,
            provider: "google-vertex".into(),
            base_url: "https://{location}-aiplatform.googleapis.com".into(),
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
        "gemini-3.1-flash-lite".into(),
        Model {
            id: "gemini-3.1-flash-lite".into(),
            name: "Gemini 3.1 Flash Lite".into(),
            api: Api::GoogleVertex,
            provider: "google-vertex".into(),
            base_url: "https://{location}-aiplatform.googleapis.com".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.25,
                output: 1.5,
                cache_read: 0.025,
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
        "gemini-3.1-pro-preview".into(),
        Model {
            id: "gemini-3.1-pro-preview".into(),
            name: "Gemini 3.1 Pro Preview".into(),
            api: Api::GoogleVertex,
            provider: "google-vertex".into(),
            base_url: "https://{location}-aiplatform.googleapis.com".into(),
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
        "gemini-3.1-pro-preview-customtools".into(),
        Model {
            id: "gemini-3.1-pro-preview-customtools".into(),
            name: "Gemini 3.1 Pro Preview Custom Tools".into(),
            api: Api::GoogleVertex,
            provider: "google-vertex".into(),
            base_url: "https://{location}-aiplatform.googleapis.com".into(),
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
            api: Api::GoogleVertex,
            provider: "google-vertex".into(),
            base_url: "https://{location}-aiplatform.googleapis.com".into(),
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
        "gemini-flash-latest".into(),
        Model {
            id: "gemini-flash-latest".into(),
            name: "Gemini Flash Latest".into(),
            api: Api::GoogleVertex,
            provider: "google-vertex".into(),
            base_url: "https://{location}-aiplatform.googleapis.com".into(),
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
        "gemini-flash-lite-latest".into(),
        Model {
            id: "gemini-flash-lite-latest".into(),
            name: "Gemini Flash-Lite Latest".into(),
            api: Api::GoogleVertex,
            provider: "google-vertex".into(),
            base_url: "https://{location}-aiplatform.googleapis.com".into(),
            reasoning: true,
            input: vec![InputModality::Text, InputModality::Image],
            cost: ModelCost {
                input: 0.25,
                output: 1.5,
                cache_read: 0.025,
                cache_write: 0.0,
            },
            context_window: 1048576,
            max_tokens: 65536,
            thinking_level_map: Some(HashMap::from([(ModelThinkingLevel::Off, None)])),
            headers: None,
            compat: None,
        },
    );

    map
}
