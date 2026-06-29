//! Main settings selector component.
//!
//! Port of `packages/coding-agent/src/modes/interactive/components/settings-selector.ts`.
//!
//! Renders a scrollable settings list with sections for Model, Display, Session, and Network.

use crate::modes::interactive::components::dynamic_border::DynamicBorder;
use crate::modes::interactive::components::tui_shim::{
    Component, Container, SettingItem, SettingsList, Text, get_capabilities,
};

/// Thinking level options.
pub type ThinkingLevel = String;

/// Project trust levels.
pub type DefaultProjectTrust = String;

/// Warning settings that can be toggled.
pub struct WarningSettings {
    pub anthropic_extra_usage: Option<bool>,
}

/// Settings configuration passed to the component.
pub struct SettingsConfig {
    pub auto_compact: bool,
    pub show_images: bool,
    pub image_width_cells: u16,
    pub auto_resize_images: bool,
    pub block_images: bool,
    pub enable_skill_commands: bool,
    pub steering_mode: String,
    pub follow_up_mode: String,
    pub transport: String,
    pub http_idle_timeout_ms: u64,
    pub thinking_level: ThinkingLevel,
    pub available_thinking_levels: Vec<ThinkingLevel>,
    pub current_theme: String,
    pub available_themes: Vec<String>,
    pub hide_thinking_block: bool,
    pub collapse_changelog: bool,
    pub enable_install_telemetry: bool,
    pub double_escape_action: String,
    pub tree_filter_mode: String,
    pub show_hardware_cursor: bool,
    pub editor_padding_x: u16,
    pub autocomplete_max_visible: u16,
    pub quiet_startup: bool,
    pub default_project_trust: String,
    pub clear_on_shrink: bool,
    pub show_terminal_progress: bool,
    pub warnings: WarningSettings,
    pub model_accent: Option<String>,
}

/// Callbacks for settings changes.
pub struct SettingsCallbacks {
    pub on_auto_compact_change: Box<dyn Fn(bool) + Send + Sync>,
    pub on_show_images_change: Box<dyn Fn(bool) + Send + Sync>,
    pub on_image_width_cells_change: Box<dyn Fn(u16) + Send + Sync>,
    pub on_auto_resize_images_change: Box<dyn Fn(bool) + Send + Sync>,
    pub on_block_images_change: Box<dyn Fn(bool) + Send + Sync>,
    pub on_enable_skill_commands_change: Box<dyn Fn(bool) + Send + Sync>,
    pub on_steering_mode_change: Box<dyn Fn(String) + Send + Sync>,
    pub on_follow_up_mode_change: Box<dyn Fn(String) + Send + Sync>,
    pub on_transport_change: Box<dyn Fn(String) + Send + Sync>,
    pub on_http_idle_timeout_ms_change: Box<dyn Fn(u64) + Send + Sync>,
    pub on_thinking_level_change: Box<dyn Fn(ThinkingLevel) + Send + Sync>,
    pub on_theme_change: Box<dyn Fn(String) + Send + Sync>,
    pub on_theme_preview: Option<Box<dyn Fn(String) + Send + Sync>>,
    pub on_hide_thinking_block_change: Box<dyn Fn(bool) + Send + Sync>,
    pub on_collapse_changelog_change: Box<dyn Fn(bool) + Send + Sync>,
    pub on_enable_install_telemetry_change: Box<dyn Fn(bool) + Send + Sync>,
    pub on_double_escape_action_change: Box<dyn Fn(String) + Send + Sync>,
    pub on_tree_filter_mode_change: Box<dyn Fn(String) + Send + Sync>,
    pub on_show_hardware_cursor_change: Box<dyn Fn(bool) + Send + Sync>,
    pub on_editor_padding_x_change: Box<dyn Fn(u16) + Send + Sync>,
    pub on_autocomplete_max_visible_change: Box<dyn Fn(u16) + Send + Sync>,
    pub on_quiet_startup_change: Box<dyn Fn(bool) + Send + Sync>,
    pub on_default_project_trust_change: Box<dyn Fn(DefaultProjectTrust) + Send + Sync>,
    pub on_clear_on_shrink_change: Box<dyn Fn(bool) + Send + Sync>,
    pub on_show_terminal_progress_change: Box<dyn Fn(bool) + Send + Sync>,
    pub on_warnings_change: Box<dyn Fn(WarningSettings) + Send + Sync>,
    pub on_cancel: Box<dyn Fn() + Send + Sync>,
}

/// Thinking level descriptions (mirrors TS THINKING_DESCRIPTIONS).
fn thinking_description(level: &str) -> &str {
    match level {
        "off" => "No reasoning",
        "minimal" => "Very brief reasoning (~1k tokens)",
        "low" => "Light reasoning (~2k tokens)",
        "medium" => "Moderate reasoning (~8k tokens)",
        "high" => "Deep reasoning (~16k tokens)",
        "xhigh" => "Maximum reasoning (~32k tokens)",
        _ => "",
    }
}

/// Default project trust labels.
const DEFAULT_PROJECT_TRUST_LABELS: &[(&str, &str)] = &[
    ("ask", "Ask"),
    ("always", "Always trust"),
    ("never", "Never trust"),
];

/// HTTP idle timeout choices in milliseconds.
const HTTP_IDLE_TIMEOUT_CHOICES: &[(u64, &str)] = &[
    (30_000, "30s"),
    (60_000, "60s"),
    (120_000, "2m"),
    (300_000, "5m"),
    (600_000, "10m"),
    (0, "disabled"),
];

fn format_http_idle_timeout_ms(ms: u64) -> String {
    for (val, label) in HTTP_IDLE_TIMEOUT_CHOICES {
        if *val == ms {
            return label.to_string();
        }
    }
    format!("{}ms", ms)
}

/// The main settings selector component.
pub struct SettingsSelectorComponent {
    layout: Container,
    settings_list: SettingsList,
}

impl SettingsSelectorComponent {
    /// Create a new settings selector.
    ///
    /// * `config` - initial settings values
    /// * `callbacks` - callbacks for each setting change
    pub fn new(config: SettingsConfig, _callbacks: SettingsCallbacks) -> Self {
        let supports_images = get_capabilities().images;

        let bool_str = |b: bool| if b { "true" } else { "false" }.to_string();

        let mut items: Vec<SettingItem> = vec![
            // ── Model ──
            SettingItem {
                id: "thinking".to_string(),
                label: "Thinking level".to_string(),
                description: Some("Reasoning depth for thinking-capable models".to_string()),
                current_value: config.thinking_level,
                values: Some(config.available_thinking_levels.clone()),
                section: Some("Model".to_string()),
            },
            SettingItem {
                id: "transport".to_string(),
                label: "Transport".to_string(),
                description: Some(
                    "Preferred transport for providers that support multiple transports"
                        .to_string(),
                ),
                current_value: config.transport,
                values: Some(
                    ["sse", "websocket", "websocket-cached", "auto"]
                        .iter()
                        .map(|s| s.to_string())
                        .collect(),
                ),
                section: None,
            },
            // ── Display ──
            SettingItem {
                id: "theme".to_string(),
                label: "Theme".to_string(),
                description: Some("Color theme for the interface".to_string()),
                current_value: config.current_theme,
                values: Some(config.available_themes),
                section: Some("Display".to_string()),
            },
            SettingItem {
                id: "hide-thinking".to_string(),
                label: "Hide thinking".to_string(),
                description: Some("Hide thinking blocks in assistant responses".to_string()),
                current_value: bool_str(config.hide_thinking_block),
                values: Some(vec!["true".to_string(), "false".to_string()]),
                section: None,
            },
            SettingItem {
                id: "collapse-changelog".to_string(),
                label: "Collapse changelog".to_string(),
                description: Some("Show condensed changelog after updates".to_string()),
                current_value: bool_str(config.collapse_changelog),
                values: Some(vec!["true".to_string(), "false".to_string()]),
                section: None,
            },
            SettingItem {
                id: "tree-filter-mode".to_string(),
                label: "Tree filter mode".to_string(),
                description: Some("Default filter when opening /tree".to_string()),
                current_value: config.tree_filter_mode,
                values: Some(
                    ["default", "no-tools", "user-only", "labeled-only", "all"]
                        .iter()
                        .map(|s| s.to_string())
                        .collect(),
                ),
                section: None,
            },
            SettingItem {
                id: "autocompact".to_string(),
                label: "Auto-compact".to_string(),
                description: Some(
                    "Automatically compact context when it gets too large".to_string(),
                ),
                current_value: bool_str(config.auto_compact),
                values: Some(vec!["true".to_string(), "false".to_string()]),
                section: None,
            },
            SettingItem {
                id: "clear-on-shrink".to_string(),
                label: "Clear on shrink".to_string(),
                description: Some(
                    "Clear empty rows when content shrinks (may cause flicker)".to_string(),
                ),
                current_value: bool_str(config.clear_on_shrink),
                values: Some(vec!["true".to_string(), "false".to_string()]),
                section: None,
            },
            // ── Session ──
            SettingItem {
                id: "steering-mode".to_string(),
                label: "Steering mode".to_string(),
                description: Some("Enter while streaming queues steering messages".to_string()),
                current_value: config.steering_mode,
                values: Some(vec!["one-at-a-time".to_string(), "all".to_string()]),
                section: Some("Session".to_string()),
            },
            SettingItem {
                id: "follow-up-mode".to_string(),
                label: "Follow-up mode".to_string(),
                description: Some("Queue follow-up messages until agent stops".to_string()),
                current_value: config.follow_up_mode,
                values: Some(vec!["one-at-a-time".to_string(), "all".to_string()]),
                section: None,
            },
            SettingItem {
                id: "double-escape-action".to_string(),
                label: "Double-escape action".to_string(),
                description: Some(
                    "Action when pressing Escape twice with empty editor".to_string(),
                ),
                current_value: config.double_escape_action,
                values: Some(
                    ["tree", "fork", "none"]
                        .iter()
                        .map(|s| s.to_string())
                        .collect(),
                ),
                section: None,
            },
            SettingItem {
                id: "quiet-startup".to_string(),
                label: "Quiet startup".to_string(),
                description: Some("Disable verbose printing at startup".to_string()),
                current_value: bool_str(config.quiet_startup),
                values: Some(vec!["true".to_string(), "false".to_string()]),
                section: None,
            },
            SettingItem {
                id: "skill-commands".to_string(),
                label: "Skill commands".to_string(),
                description: Some("Register skills as /skill:name commands".to_string()),
                current_value: bool_str(config.enable_skill_commands),
                values: Some(vec!["true".to_string(), "false".to_string()]),
                section: None,
            },
            SettingItem {
                id: "install-telemetry".to_string(),
                label: "Install telemetry".to_string(),
                description: Some(
                    "Send an anonymous version/update ping after updates".to_string(),
                ),
                current_value: bool_str(config.enable_install_telemetry),
                values: Some(vec!["true".to_string(), "false".to_string()]),
                section: None,
            },
            SettingItem {
                id: "default-project-trust".to_string(),
                label: "Default project trust".to_string(),
                description: Some(
                    "Fallback behavior when no extension decides project trust".to_string(),
                ),
                current_value: config.default_project_trust,
                values: Some(
                    DEFAULT_PROJECT_TRUST_LABELS
                        .iter()
                        .map(|(v, _)| v.to_string())
                        .collect(),
                ),
                section: None,
            },
            SettingItem {
                id: "warnings".to_string(),
                label: "Warnings".to_string(),
                description: Some("Enable or disable individual warnings".to_string()),
                current_value: "configure".to_string(),
                values: None,
                section: None,
            },
            // ── Network ──
            SettingItem {
                id: "http-idle-timeout".to_string(),
                label: "HTTP idle timeout".to_string(),
                description: Some(
                    "Maximum idle gap while waiting for HTTP headers or body chunks".to_string(),
                ),
                current_value: format_http_idle_timeout_ms(config.http_idle_timeout_ms),
                values: Some(
                    HTTP_IDLE_TIMEOUT_CHOICES
                        .iter()
                        .map(|(_, label)| label.to_string())
                        .collect(),
                ),
                section: Some("Network".to_string()),
            },
        ];

        // Insert display items conditionally
        let mut insert_at = items.iter().position(|i| i.id == "theme").unwrap_or(1) + 1;
        if supports_images {
            items.insert(
                insert_at,
                SettingItem {
                    id: "show-images".to_string(),
                    label: "Show images".to_string(),
                    description: Some("Render images inline in terminal".to_string()),
                    current_value: bool_str(config.show_images),
                    values: Some(vec!["true".to_string(), "false".to_string()]),
                    section: None,
                },
            );
            insert_at += 1;
            items.insert(
                insert_at,
                SettingItem {
                    id: "image-width-cells".to_string(),
                    label: "Image width".to_string(),
                    description: Some("Preferred inline image width in terminal cells".to_string()),
                    current_value: config.image_width_cells.to_string(),
                    values: Some(vec!["60".to_string(), "80".to_string(), "120".to_string()]),
                    section: None,
                },
            );
            insert_at += 1;
        }
        items.insert(
            insert_at,
            SettingItem {
                id: "auto-resize-images".to_string(),
                label: "Auto-resize images".to_string(),
                description: Some("Resize large images to 2000x2000 max".to_string()),
                current_value: bool_str(config.auto_resize_images),
                values: Some(vec!["true".to_string(), "false".to_string()]),
                section: None,
            },
        );
        insert_at += 1;
        items.insert(
            insert_at,
            SettingItem {
                id: "block-images".to_string(),
                label: "Block images".to_string(),
                description: Some("Prevent images from being sent to LLM providers".to_string()),
                current_value: bool_str(config.block_images),
                values: Some(vec!["true".to_string(), "false".to_string()]),
                section: None,
            },
        );
        insert_at += 1;
        items.insert(
            insert_at,
            SettingItem {
                id: "show-hardware-cursor".to_string(),
                label: "Show hardware cursor".to_string(),
                description: Some("Show the terminal cursor for IME support".to_string()),
                current_value: bool_str(config.show_hardware_cursor),
                values: Some(vec!["true".to_string(), "false".to_string()]),
                section: None,
            },
        );
        insert_at += 1;
        items.insert(
            insert_at,
            SettingItem {
                id: "editor-padding".to_string(),
                label: "Editor padding".to_string(),
                description: Some("Horizontal padding for input editor (0-3)".to_string()),
                current_value: config.editor_padding_x.to_string(),
                values: Some(["0", "1", "2", "3"].iter().map(|s| s.to_string()).collect()),
                section: None,
            },
        );
        insert_at += 1;
        items.insert(
            insert_at,
            SettingItem {
                id: "autocomplete-max-visible".to_string(),
                label: "Autocomplete max items".to_string(),
                description: Some("Max visible items in autocomplete dropdown (3-20)".to_string()),
                current_value: config.autocomplete_max_visible.to_string(),
                values: Some(
                    ["3", "5", "7", "10", "15", "20"]
                        .iter()
                        .map(|s| s.to_string())
                        .collect(),
                ),
                section: None,
            },
        );
        insert_at += 1;
        items.insert(
            insert_at,
            SettingItem {
                id: "terminal-progress".to_string(),
                label: "Terminal progress".to_string(),
                description: Some(
                    "Show OSC 9;4 progress indicators in the terminal tab bar".to_string(),
                ),
                current_value: bool_str(config.show_terminal_progress),
                values: Some(vec!["true".to_string(), "false".to_string()]),
                section: None,
            },
        );

        let mut layout = Container::new();

        // Top border
        layout.add_child(Box::new(DynamicBorder::new(None)));

        // Settings list
        let settings_list = SettingsList::new(items, 10, |_id, _new_value| {}, || {});
        layout.add_child(Box::new(Text::new("settings-list", 0, 0))); // placeholder

        // Bottom border
        layout.add_child(Box::new(DynamicBorder::new(None)));

        Self {
            layout,
            settings_list,
        }
    }
}

impl Component for SettingsSelectorComponent {
    fn render(&self, width: u16) -> Vec<String> {
        self.layout.render(width)
    }

    fn invalidate(&mut self) {
        self.layout.invalidate();
    }
}
