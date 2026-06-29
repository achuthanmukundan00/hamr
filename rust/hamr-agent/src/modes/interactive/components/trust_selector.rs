//! Port of `packages/coding-agent/src/modes/interactive/components/trust-selector.ts`.
//!
//! Component that renders a project trust selector with borders.

use crate::core::trust_manager::{
    ProjectTrustOption, ProjectTrustStoreEntry, get_project_trust_options,
};
use crate::modes::interactive::components::dynamic_border::DynamicBorder;
use crate::modes::interactive::components::keybinding_hints::{key_hint, raw_key_hint};
use crate::modes::interactive::components::tui_shim::{Component, Spacer, Text, get_keybindings};
use crate::modes::interactive::theme::theme::theme;

/// The selection made by the user.
pub struct TrustSelection {
    pub trusted: bool,
    pub updates: Vec<crate::core::trust_manager::ProjectTrustUpdate>,
}

/// Options for constructing a TrustSelectorComponent.
pub struct TrustSelectorOptions {
    pub cwd: String,
    pub saved_decision: Option<ProjectTrustStoreEntry>,
    pub project_trusted: bool,
    pub on_select: Box<dyn Fn(TrustSelection) + Send + Sync>,
    pub on_cancel: Box<dyn Fn() + Send + Sync>,
}

fn format_decision(trust_path: Option<&str>, decision: &Option<ProjectTrustStoreEntry>) -> String {
    match decision {
        None => "none".to_string(),
        Some(d) => {
            let label = if d.decision { "trusted" } else { "untrusted" };
            if let Some(tp) = trust_path {
                if d.path != tp {
                    return format!("{} (inherited from {})", label, d.path);
                }
            }
            format!("{} ({})", label, d.path)
        }
    }
}

/// Component that renders a project trust selector.
pub struct TrustSelectorComponent {
    selected_index: usize,
    trust_options: Vec<ProjectTrustOption>,
    saved_decision: Option<ProjectTrustStoreEntry>,
    on_select: Box<dyn Fn(TrustSelection) + Send + Sync>,
    on_cancel: Box<dyn Fn() + Send + Sync>,
    cwd: String,
    project_trusted: bool,
}

impl TrustSelectorComponent {
    pub fn new(options: TrustSelectorOptions) -> Self {
        let trust_options = get_project_trust_options(&options.cwd, None);
        let saved_decision = options.saved_decision;

        let selected_index = trust_options
            .iter()
            .position(|opt| {
                opt.saved_path.as_deref() == saved_decision.as_ref().map(|d| d.path.as_str())
                    && Some(opt.trusted) == saved_decision.as_ref().map(|d| d.decision)
            })
            .unwrap_or(0)
            .max(0);

        TrustSelectorComponent {
            selected_index,
            trust_options,
            saved_decision,
            on_select: options.on_select,
            on_cancel: options.on_cancel,
            cwd: options.cwd,
            project_trusted: options.project_trusted,
        }
    }

    fn is_saved_option(&self, option: &ProjectTrustOption) -> bool {
        option.saved_path.is_some()
            && self.saved_decision.as_ref().map(|d| d.decision) == Some(option.trusted)
            && self.saved_decision.as_ref().map(|d| d.path.as_str()) == option.saved_path.as_deref()
    }

    pub fn handle_input(&mut self, key_data: &str) {
        let kb = get_keybindings();
        if kb.matches(key_data, "tui.select.up") || key_data == "k" {
            self.selected_index = self.selected_index.saturating_sub(1);
        } else if kb.matches(key_data, "tui.select.down") || key_data == "j" {
            self.selected_index =
                (self.selected_index + 1).min(self.trust_options.len().saturating_sub(1));
        } else if kb.matches(key_data, "tui.select.confirm") || key_data == "\n" {
            if let Some(selected) = self.trust_options.get(self.selected_index) {
                (self.on_select)(TrustSelection {
                    trusted: selected.trusted,
                    updates: selected.updates.clone(),
                });
            }
        } else if kb.matches(key_data, "tui.select.cancel") {
            (self.on_cancel)();
        }
    }
}

impl Component for TrustSelectorComponent {
    fn render(&self, width: u16) -> Vec<String> {
        let t = theme();
        let mut lines: Vec<String> = Vec::new();

        // Top border
        lines.extend(DynamicBorder::new(None).render(width));
        lines.extend(Spacer::new(1).render(width));
        lines.extend(Text::new(t.fg("accent", &t.bold("Project trust")), 1, 0).render(width));
        lines.extend(Text::new(t.fg("muted", &self.cwd), 1, 0).render(width));
        lines.extend(Spacer::new(1).render(width));
        lines.extend(
            Text::new(
                t.fg(
                    "muted",
                    &format!(
                        "Saved decision: {}",
                        format_decision(
                            self.trust_options
                                .first()
                                .and_then(|o| o.saved_path.as_deref()),
                            &self.saved_decision,
                        )
                    ),
                ),
                1,
                0,
            )
            .render(width),
        );
        lines.extend(
            Text::new(
                t.fg(
                    "muted",
                    &format!(
                        "Current session: {}",
                        if self.project_trusted {
                            "trusted"
                        } else {
                            "untrusted"
                        }
                    ),
                ),
                1,
                0,
            )
            .render(width),
        );
        lines.extend(Spacer::new(1).render(width));

        // Option list
        for i in 0..self.trust_options.len() {
            let option = &self.trust_options[i];
            let is_selected = i == self.selected_index;
            let is_current = self.is_saved_option(option);
            let checkmark = if is_current {
                t.fg("success", " ✓")
            } else {
                String::new()
            };
            let prefix = if is_selected {
                t.fg("accent", "→ ")
            } else {
                "  ".to_string()
            };
            let label = if is_selected {
                t.fg("accent", &option.label)
            } else {
                t.fg("text", &option.label)
            };
            lines
                .extend(Text::new(format!("{}{}{}", prefix, label, checkmark), 1, 0).render(width));
        }

        lines.extend(Spacer::new(1).render(width));
        lines.extend(
            Text::new(
                format!(
                    "{}{}  {}{}  {}{}",
                    raw_key_hint("↑↓", "navigate"),
                    "",
                    key_hint("tui.select.confirm", "save"),
                    "",
                    key_hint("tui.select.cancel", "cancel"),
                    "",
                ),
                1,
                0,
            )
            .render(width),
        );
        lines.extend(Spacer::new(1).render(width));
        lines.extend(DynamicBorder::new(None).render(width));

        lines
    }

    fn invalidate(&mut self) {}
}
