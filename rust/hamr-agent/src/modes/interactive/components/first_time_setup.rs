//! First-time setup dialog: theme choice and analytics opt-in.
//!
//! Port of `packages/coding-agent/src/modes/interactive/components/first-time-setup.ts`.

use crate::modes::interactive::components::dynamic_border::DynamicBorder;
use crate::modes::interactive::components::keybinding_hints::{key_hint, raw_key_hint};
use crate::modes::interactive::components::tui_shim::{
    Component, Container, Spacer, Text, get_keybindings,
};
use crate::modes::interactive::theme::theme::theme;

/// Result of the first-time setup flow.
pub struct FirstTimeSetupResult {
    pub theme: String,
    pub share_analytics: bool,
}

/// Options for the first-time setup component.
pub struct FirstTimeSetupOptions {
    pub detected_theme: String,
    pub on_theme_preview: Box<dyn Fn(&str) + Send + Sync>,
    pub on_submit: Box<dyn Fn(FirstTimeSetupResult) + Send + Sync>,
    pub on_cancel: Box<dyn Fn() + Send + Sync>,
}

/// A theme option entry.
struct ThemeOption {
    value: &'static str,
    label: &'static str,
}

/// An analytics option entry.
struct AnalyticsOption {
    value: bool,
    label: &'static str,
}

const THEME_OPTIONS: &[ThemeOption] = &[
    ThemeOption {
        value: "hamr",
        label: "Hamr (recommended)",
    },
    ThemeOption {
        value: "dark",
        label: "Dark",
    },
    ThemeOption {
        value: "light",
        label: "Light",
    },
];

const ANALYTICS_OPTIONS: &[AnalyticsOption] = &[
    AnalyticsOption {
        value: true,
        label: "Share anonymous usage data",
    },
    AnalyticsOption {
        value: false,
        label: "Don't share",
    },
];

const SETUP_LOGO_LINES: &str = "██████\n██  ██\n████  ██\n██    ██";

/// First-time setup dialog: theme choice followed by analytics opt-in.
pub struct FirstTimeSetupComponent {
    layout: Container,
    step: SetupStep,
    theme_index: usize,
    analytics_index: usize,
    options: FirstTimeSetupOptions,
}

#[derive(PartialEq)]
enum SetupStep {
    Theme,
    Analytics,
}

impl FirstTimeSetupComponent {
    /// Create a new first-time setup component.
    ///
    /// * `options` - callbacks and detected theme info
    pub fn new(options: FirstTimeSetupOptions) -> Self {
        // Default to "hamr" theme (index 0) regardless of terminal detection.
        // The detected appearance is shown for info only.
        let theme_index = 0usize;
        let analytics_index = 0usize;

        let layout = Container::new();
        let mut result = Self {
            layout,
            step: SetupStep::Theme,
            theme_index,
            analytics_index,
            options,
        };

        result.update();
        result
    }

    /// Rebuild the whole dialog on every change so theme previews recolor all text.
    fn update(&mut self) {
        self.layout.clear();
        self.layout.add_child(Box::new(DynamicBorder::new(None)));
        self.layout.add_child(Box::new(Spacer::new(1)));
        self.layout.add_child(Box::new(Text::new(
            theme().fg("accent", SETUP_LOGO_LINES),
            1,
            0,
        )));
        self.layout.add_child(Box::new(Spacer::new(1)));

        // Welcome message with APP_NAME
        let app_name = "Hamr"; // APP_NAME constant
        self.layout.add_child(Box::new(Text::new(
            theme().fg(
                "accent",
                &theme().bold(&format!(
                    "Welcome to {}, the minimal coding agent.",
                    app_name
                )),
            ),
            1,
            0,
        )));
        self.layout.add_child(Box::new(Spacer::new(1)));

        if self.step == SetupStep::Theme {
            self.layout.add_child(Box::new(Text::new(
                theme().fg("text", "Pick a theme."),
                1,
                0,
            )));
            self.layout.add_child(Box::new(Text::new(
                theme().fg(
                    "muted",
                    &format!(
                        "Detected system appearance: {}",
                        self.options.detected_theme
                    ),
                ),
                1,
                0,
            )));
            self.layout.add_child(Box::new(Spacer::new(1)));
            self.add_option_list(
                &THEME_OPTIONS
                    .iter()
                    .map(|o| o.label.to_string())
                    .collect::<Vec<_>>(),
                self.theme_index,
            );
        } else {
            self.layout.add_child(Box::new(Text::new(
                theme().fg("text", "Opt-in to anonymous usage data sharing?"),
                1,
                0,
            )));

            let analytics_text = format!(
                "Opting in stores a tracking identifier in settings.json and enables anonymous\n\
                 usage analytics. This helps us to better debug, reproduce, and resolve issues\n\
                 and bugs within {}. You can observe what is shared using /privacy and make\n\
                 changes anytime in settings.json.",
                app_name
            );
            self.layout.add_child(Box::new(Text::new(
                theme().fg("muted", &analytics_text),
                1,
                0,
            )));
            self.layout.add_child(Box::new(Spacer::new(1)));
            self.add_option_list(
                &ANALYTICS_OPTIONS
                    .iter()
                    .map(|o| o.label.to_string())
                    .collect::<Vec<_>>(),
                self.analytics_index,
            );
        }

        self.layout.add_child(Box::new(Spacer::new(1)));

        let step_hint = if self.step == SetupStep::Theme {
            "continue"
        } else {
            "finish"
        };
        self.layout.add_child(Box::new(Text::new(
            format!(
                "{}  {}  {}",
                raw_key_hint("↑↓", "navigate"),
                key_hint("tui.select.confirm", step_hint),
                key_hint("tui.select.cancel", "skip setup"),
            ),
            1,
            0,
        )));
        self.layout.add_child(Box::new(Spacer::new(1)));
        self.layout.add_child(Box::new(DynamicBorder::new(None)));
    }

    /// Add a list of options with the selected index highlighted.
    fn add_option_list(&mut self, labels: &[String], selected_index: usize) {
        for (i, label) in labels.iter().enumerate() {
            let is_selected = i == selected_index;
            let prefix = if is_selected {
                theme().fg("accent", "→ ")
            } else {
                "  ".to_string()
            };
            let label_styled = if is_selected {
                theme().fg("accent", label)
            } else {
                theme().fg("text", label)
            };
            self.layout.add_child(Box::new(Text::new(
                format!("{}{}", prefix, label_styled),
                1,
                0,
            )));
        }
    }

    /// Move the selection up or down by delta.
    fn move_selection(&mut self, delta: isize) {
        if self.step == SetupStep::Theme {
            let next = (self.theme_index as isize + delta)
                .max(0)
                .min(THEME_OPTIONS.len() as isize - 1) as usize;
            if next != self.theme_index {
                self.theme_index = next;
                (self.options.on_theme_preview)(THEME_OPTIONS[self.theme_index].value);
            }
        } else {
            self.analytics_index = (self.analytics_index as isize + delta)
                .max(0)
                .min(ANALYTICS_OPTIONS.len() as isize - 1)
                as usize;
        }
        self.update();
    }

    /// Handle keyboard input.
    pub fn handle_input(&mut self, key_data: &str) {
        let kb = get_keybindings();
        if kb.matches(key_data, "tui.select.up") || key_data == "k" {
            self.move_selection(-1);
        } else if kb.matches(key_data, "tui.select.down") || key_data == "j" {
            self.move_selection(1);
        } else if kb.matches(key_data, "tui.select.confirm") || key_data == "\n" {
            if self.step == SetupStep::Theme {
                self.step = SetupStep::Analytics;
                self.update();
            } else {
                (self.options.on_submit)(FirstTimeSetupResult {
                    theme: THEME_OPTIONS[self.theme_index].value.to_string(),
                    share_analytics: ANALYTICS_OPTIONS[self.analytics_index].value,
                });
            }
        } else if kb.matches(key_data, "tui.select.cancel") {
            (self.options.on_cancel)();
        }
    }
}

impl Component for FirstTimeSetupComponent {
    fn render(&self, width: u16) -> Vec<String> {
        self.layout.render(width)
    }

    fn invalidate(&mut self) {
        self.layout.invalidate();
    }
}
