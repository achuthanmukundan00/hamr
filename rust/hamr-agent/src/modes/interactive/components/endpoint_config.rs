//! Endpoint configuration component — TUI form for adding custom/self-hosted
//! OpenAI-compatible or Anthropic-compatible endpoints.
//!
//! Port of `packages/coding-agent/src/modes/interactive/components/endpoint-config.ts`.

use crate::modes::interactive::components::dynamic_border::DynamicBorder;
use crate::modes::interactive::components::tui_shim::{Component, Container, Input, Spacer, Text};
use crate::modes::interactive::theme::theme::theme;

/// API protocol for an endpoint.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EndpointApi {
    OpenAICompletions,
    AnthropicMessages,
}

impl EndpointApi {
    pub fn as_str(&self) -> &'static str {
        match self {
            EndpointApi::OpenAICompletions => "openai-completions",
            EndpointApi::AnthropicMessages => "anthropic-messages",
        }
    }

    pub fn display(&self) -> &'static str {
        match self {
            EndpointApi::OpenAICompletions => "OpenAI Compatible",
            EndpointApi::AnthropicMessages => "Anthropic Messages",
        }
    }
}

/// A custom header in the endpoint configuration.
#[derive(Debug, Clone)]
pub struct EndpointHeader {
    pub key: String,
    pub value: String,
    pub secret: bool,
}

/// Complete endpoint configuration.
#[derive(Debug, Clone)]
pub struct EndpointConfig {
    pub name: String,
    pub base_url: String,
    pub api: EndpointApi,
    pub api_key: String,
    pub headers: Vec<EndpointHeader>,
}

impl Default for EndpointConfig {
    fn default() -> Self {
        Self {
            name: "custom".to_string(),
            base_url: "http://".to_string(),
            api: EndpointApi::OpenAICompletions,
            api_key: "not-needed".to_string(),
            headers: Vec::new(),
        }
    }
}

/// A preset endpoint template.
#[derive(Debug, Clone)]
pub struct EndpointPreset {
    pub label: String,
    pub id: String,
    pub base_url: String,
    pub api: EndpointApi,
}

/// Pre-defined endpoint presets.
pub fn endpoint_presets() -> Vec<EndpointPreset> {
    vec![
        EndpointPreset {
            label: "Relay".to_string(),
            id: "relay".to_string(),
            base_url: "http://127.0.0.1:1234/v1".to_string(),
            api: EndpointApi::OpenAICompletions,
        },
        EndpointPreset {
            label: "LM Studio".to_string(),
            id: "lm-studio".to_string(),
            base_url: "http://localhost:1234/v1".to_string(),
            api: EndpointApi::OpenAICompletions,
        },
        EndpointPreset {
            label: "llama.cpp".to_string(),
            id: "llama-cpp".to_string(),
            base_url: "http://localhost:8080/v1".to_string(),
            api: EndpointApi::OpenAICompletions,
        },
        EndpointPreset {
            label: "Ollama".to_string(),
            id: "ollama".to_string(),
            base_url: "http://localhost:11434/v1".to_string(),
            api: EndpointApi::OpenAICompletions,
        },
        EndpointPreset {
            label: "vLLM".to_string(),
            id: "vllm".to_string(),
            base_url: "http://localhost:8000/v1".to_string(),
            api: EndpointApi::OpenAICompletions,
        },
        EndpointPreset {
            label: "Custom".to_string(),
            id: "custom".to_string(),
            base_url: "http://".to_string(),
            api: EndpointApi::OpenAICompletions,
        },
    ]
}

/// Form mode for the endpoint config component.
#[derive(Debug, Clone, Copy, PartialEq)]
enum FormMode {
    Navigate,
    EditUrl,
    EditKey,
    EditHeaders,
}

/// Header edit sub-field.
#[derive(Debug, Clone, Copy, PartialEq)]
enum HeaderEditField {
    Name,
    Value,
    Secret,
    Delete,
    Save,
}

/// State for editing a single header.
#[derive(Debug, Clone)]
struct HeaderEditState {
    key: String,
    value: String,
    secret: bool,
    is_new: bool,
}

/// Endpoint configuration TUI component.
pub struct EndpointConfigComponent {
    container: Container,
    /// Index into the presets list.
    preset_index: usize,
    config: EndpointConfig,
    mode: FormMode,
    selected_field: usize,
    input: Input,
    header_edit: Option<HeaderEditState>,
    header_edit_field: HeaderEditField,
    /// Callbacks — stored as closures (TODO: wire with real TUI events).
    _on_save: Option<Box<dyn Fn(&EndpointConfig) + Send + Sync>>,
    _on_cancel: Option<Box<dyn Fn() + Send + Sync>>,
}

impl EndpointConfigComponent {
    pub fn new(
        on_save: Option<Box<dyn Fn(&EndpointConfig) + Send + Sync>>,
        on_cancel: Option<Box<dyn Fn() + Send + Sync>>,
        initial_config: Option<EndpointConfig>,
    ) -> Self {
        let presets = endpoint_presets();
        let preset = if let Some(ref cfg) = initial_config {
            presets
                .iter()
                .find(|p| p.id == cfg.name)
                .cloned()
                .unwrap_or_else(|| presets[0].clone())
        } else {
            presets[0].clone()
        };
        let preset_index = presets.iter().position(|p| p.id == preset.id).unwrap_or(0);

        let config = initial_config.unwrap_or_else(|| EndpointConfig {
            name: preset.id.clone(),
            base_url: preset.base_url.clone(),
            api: preset.api,
            api_key: "not-needed".to_string(),
            headers: Vec::new(),
        });

        let mut container = Container::new();

        // Layout
        container.add_child(Box::new(DynamicBorder::new(None)));
        container.add_child(Box::new(Spacer::new(1)));
        container.add_child(Box::new(Text::new(
            theme().fg("accent", &theme().bold("Configure endpoint")),
            1,
            0,
        )));
        container.add_child(Box::new(Spacer::new(1)));

        // Content placeholder
        container.add_child(Box::new(Container::new()));

        container.add_child(Box::new(Spacer::new(1)));
        container.add_child(Box::new(Text::new("", 1, 0)));
        container.add_child(Box::new(Spacer::new(1)));
        container.add_child(Box::new(DynamicBorder::new(None)));

        let comp = Self {
            container,
            preset_index,
            config,
            mode: FormMode::Navigate,
            selected_field: 0,
            input: Input::new(),
            header_edit: None,
            header_edit_field: HeaderEditField::Name,
            _on_save: on_save,
            _on_cancel: on_cancel,
        };

        comp
    }

    fn field_count(&self) -> usize {
        // preset + url + api + key + headers(N) + [add header] + save + cancel
        4 + self.config.headers.len() + 1 + 2
    }

    fn cycle_preset(&mut self) {
        let presets = endpoint_presets();
        self.preset_index = (self.preset_index + 1) % presets.len();
        let preset = &presets[self.preset_index];
        self.config.name = preset.id.clone();
        self.config.base_url = preset.base_url.clone();
        self.config.api = preset.api;
    }

    fn cycle_api(&mut self) {
        self.config.api = match self.config.api {
            EndpointApi::OpenAICompletions => EndpointApi::AnthropicMessages,
            EndpointApi::AnthropicMessages => EndpointApi::OpenAICompletions,
        };
    }

    /// Handle input navigation. Returns true if event was consumed.
    pub fn handle_input(&mut self, _key_data: &str) -> bool {
        // In the real TUI integration, this would match keybindings.
        // For now, stub returns false (not consumed).
        false
    }

    /// Get the current config (for save callback).
    pub fn get_config(&self) -> &EndpointConfig {
        &self.config
    }
}

impl Component for EndpointConfigComponent {
    fn render(&self, width: u16) -> Vec<String> {
        self.container.render(width)
    }

    fn invalidate(&mut self) {
        self.container.invalidate();
    }
}
