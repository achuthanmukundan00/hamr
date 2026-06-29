//! Component for enabling/disabling models for Ctrl+P cycling.
//!
//! Port of `packages/coding-agent/src/modes/interactive/components/scoped-models-selector.ts`.
//!
//! Changes are session-only until explicitly persisted.

use crate::modes::interactive::components::dynamic_border::DynamicBorder;
use crate::modes::interactive::components::keybinding_hints::key_text;
use crate::modes::interactive::components::model_selector::ModelItem;
use crate::modes::interactive::components::tui_shim::{
    Component, Container, Focusable, Input, Key, Spacer, Text, fuzzy_filter, get_keybindings,
    matches_key,
};
use crate::modes::interactive::theme::theme::THEME;
use std::collections::HashSet;

/// Enabled IDs: null = all enabled, Vec<String> = explicit list
type EnabledIds = Option<Vec<String>>;

fn is_enabled(enabled_ids: &EnabledIds, id: &str) -> bool {
    match enabled_ids {
        None => true,
        Some(ids) => ids.contains(&id.to_string()),
    }
}

fn toggle(enabled_ids: &EnabledIds, id: &str) -> EnabledIds {
    match enabled_ids {
        None => Some(vec![id.to_string()]),
        Some(ids) => {
            let pos = ids.iter().position(|x| x == id);
            match pos {
                Some(idx) => {
                    let mut result = ids.clone();
                    result.remove(idx);
                    Some(result)
                }
                None => {
                    let mut result = ids.clone();
                    result.push(id.to_string());
                    Some(result)
                }
            }
        }
    }
}

fn enable_all(
    enabled_ids: &EnabledIds,
    all_ids: &[String],
    target_ids: Option<&[String]>,
) -> EnabledIds {
    match enabled_ids {
        None => None, // Already all enabled
        Some(ids) => {
            let targets: HashSet<_> = target_ids.unwrap_or(all_ids).iter().collect();
            let mut result: HashSet<_> = ids.iter().cloned().collect();
            for t in &targets {
                result.insert((*t).clone());
            }
            // Sort by order in all_ids
            let sorted: Vec<_> = all_ids
                .iter()
                .filter(|id| result.contains(*id))
                .cloned()
                .collect();
            if sorted.len() == all_ids.len() {
                None
            } else {
                Some(sorted)
            }
        }
    }
}

fn clear_all(
    enabled_ids: &EnabledIds,
    all_ids: &[String],
    target_ids: Option<&[String]>,
) -> EnabledIds {
    match enabled_ids {
        None => {
            // All are currently enabled; clear the targets
            let targets: HashSet<_> = target_ids.unwrap_or(all_ids).iter().collect();
            let result: Vec<_> = all_ids
                .iter()
                .filter(|id| !targets.contains(*id))
                .cloned()
                .collect();
            if result.is_empty() {
                Some(result)
            } else {
                Some(result)
            }
        }
        Some(ids) => {
            let targets: HashSet<_> = target_ids.unwrap_or(ids).iter().collect();
            let result: Vec<_> = ids
                .iter()
                .filter(|id| !targets.contains(*id))
                .cloned()
                .collect();
            Some(result)
        }
    }
}

fn get_sorted_ids(enabled_ids: &EnabledIds, all_ids: &[String]) -> Vec<String> {
    match enabled_ids {
        None => all_ids.to_vec(),
        Some(ids) => {
            let enabled_set: HashSet<_> = ids.iter().collect();
            let mut result = ids.clone();
            for id in all_ids {
                if !enabled_set.contains(id) {
                    result.push(id.clone());
                }
            }
            result
        }
    }
}

/// A model entry with enabled/disabled state.
#[derive(Clone)]
struct ModelEntry {
    full_id: String,
    model: ModelItem,
    enabled: bool,
}

/// Configuration for the scoped models selector.
pub struct ModelsConfig {
    pub all_models: Vec<ModelItem>,
    pub enabled_model_ids: Option<Vec<String>>,
}

/// Callbacks for the scoped models selector.
pub struct ModelsCallbacks {
    pub on_change: Box<dyn Fn(Option<Vec<String>>) + Send + Sync>,
    pub on_persist: Box<dyn Fn(Option<Vec<String>>) + Send + Sync>,
    pub on_cancel: Box<dyn Fn() + Send + Sync>,
}

/// Component for enabling/disabling models for Ctrl+P cycling.
pub struct ScopedModelsSelectorComponent {
    layout: Container,
    models_by_id: std::collections::HashMap<String, ModelItem>,
    all_ids: Vec<String>,
    enabled_ids: EnabledIds,
    filtered_items: Vec<ModelEntry>,
    selected_index: usize,
    search_input: Input,
    list_pos: usize,
    footer_pos: usize,
    callbacks: ModelsCallbacks,
    max_visible: usize,
    is_dirty: bool,
    focused: bool,
}

impl ScopedModelsSelectorComponent {
    pub fn new(config: ModelsConfig, callbacks: ModelsCallbacks) -> Self {
        let mut models_by_id = std::collections::HashMap::new();
        let mut all_ids = Vec::new();

        for model in &config.all_models {
            let full_id = format!("{}/{}", model.provider, model.id);
            models_by_id.insert(full_id.clone(), model.clone());
            all_ids.push(full_id);
        }

        let enabled_ids = match config.enabled_model_ids {
            None => None,
            Some(ids) if ids.is_empty() => None,
            Some(ids) => Some(ids),
        };

        let sorted_ids = get_sorted_ids(&enabled_ids, &all_ids);
        let filtered_items: Vec<ModelEntry> = sorted_ids
            .iter()
            .filter_map(|id| {
                models_by_id.get(id).map(|model| ModelEntry {
                    full_id: id.clone(),
                    model: model.clone(),
                    enabled: is_enabled(&enabled_ids, id),
                })
            })
            .collect();

        let mut layout = Container::new();

        // Header
        layout.add_child(Box::new(DynamicBorder::new(None)));
        layout.add_child(Box::new(Spacer::new(1)));
        layout.add_child(Box::new(Text::new(
            THEME.fg("accent", &THEME.bold("Model Configuration")),
            0,
            0,
        )));
        layout.add_child(Box::new(Text::new(
            THEME.fg(
                "muted",
                &format!(
                    "Session-only. {} to save to settings.",
                    key_text("app.models.save")
                ),
            ),
            0,
            0,
        )));
        layout.add_child(Box::new(Spacer::new(1)));

        // Search input
        let search_input = Input::new();
        layout.add_child(Box::new(Spacer::new(1))); // placeholder

        layout.add_child(Box::new(Spacer::new(1)));

        // List container
        let list_pos = layout.children().len();
        layout.add_child(Box::new(Container::new()));

        layout.add_child(Box::new(Spacer::new(1)));

        // Footer
        let footer_pos = layout.children().len();
        layout.add_child(Box::new(Text::new("", 0, 0)));

        layout.add_child(Box::new(DynamicBorder::new(None)));

        let mut result = Self {
            layout,
            models_by_id,
            all_ids,
            enabled_ids,
            filtered_items,
            selected_index: 0,
            search_input,
            list_pos,
            footer_pos,
            callbacks,
            max_visible: 8,
            is_dirty: false,
            focused: false,
        };

        result.update_list();
        result
    }

    fn get_footer_text(&self) -> String {
        let enabled_count = self
            .enabled_ids
            .as_ref()
            .map(|ids| ids.len())
            .unwrap_or(self.all_ids.len());
        let all_enabled = self.enabled_ids.is_none();
        let count_text = if all_enabled {
            "all enabled".to_string()
        } else {
            format!("{}/{} enabled", enabled_count, self.all_ids.len())
        };

        let parts = vec![
            format!("{} toggle", key_text("tui.select.confirm")),
            format!("{} all", key_text("app.models.enableAll")),
            format!("{} clear", key_text("app.models.clearAll")),
            format!("{} provider", key_text("app.models.toggleProvider")),
            format!(
                "{}/{} reorder",
                key_text("app.models.reorderUp"),
                key_text("app.models.reorderDown"),
            ),
            format!("{} save", key_text("app.models.save")),
            count_text,
        ];

        let joined = format!("  {}", parts.join(" · "));
        if self.is_dirty {
            format!(
                "{}{}",
                THEME.fg("dim", &joined),
                THEME.fg("warning", " (unsaved)")
            )
        } else {
            THEME.fg("dim", &joined)
        }
    }

    fn refresh(&mut self) {
        let query = self.search_input.get_value();
        let sorted_ids = get_sorted_ids(&self.enabled_ids, &self.all_ids);
        let items: Vec<ModelEntry> = sorted_ids
            .iter()
            .filter_map(|id| {
                self.models_by_id.get(id).map(|model| ModelEntry {
                    full_id: id.clone(),
                    model: model.clone(),
                    enabled: is_enabled(&self.enabled_ids, id),
                })
            })
            .collect();

        self.filtered_items = if query.is_empty() {
            items
        } else {
            fuzzy_filter(&items, query, |e: &ModelEntry| {
                format!("{} {}", e.model.id, e.model.provider)
            })
        };
        self.selected_index = self
            .selected_index
            .min(self.filtered_items.len().saturating_sub(1));
        self.update_list();
        self.update_footer();
    }

    fn notify_change(&self) {
        (self.callbacks.on_change)(self.enabled_ids.clone());
    }

    fn update_list(&mut self) {
        let mut new_list = Container::new();

        if self.filtered_items.is_empty() {
            new_list.add_child(Box::new(Text::new(
                THEME.fg("muted", "  No matching models"),
                0,
                0,
            )));
        } else {
            let start_index = self
                .selected_index
                .saturating_sub(self.max_visible / 2)
                .min(self.filtered_items.len().saturating_sub(self.max_visible));
            let end_index = (start_index + self.max_visible).min(self.filtered_items.len());

            let all_enabled = self.enabled_ids.is_none();

            for i in start_index..end_index {
                let item = &self.filtered_items[i];
                let is_selected = i == self.selected_index;
                let prefix = if is_selected {
                    THEME.fg("accent", "→ ")
                } else {
                    "  ".to_string()
                };
                let model_text = if is_selected {
                    THEME.fg("accent", &item.model.id)
                } else {
                    item.model.id.clone()
                };
                let provider_badge = THEME.fg("muted", &format!(" [{}]", item.model.provider));
                let status = if all_enabled {
                    "".to_string()
                } else if item.enabled {
                    THEME.fg("success", " ✓")
                } else {
                    THEME.fg("dim", " ✗")
                };

                new_list.add_child(Box::new(Text::new(
                    format!("{}{}{}{}", prefix, model_text, provider_badge, status),
                    0,
                    0,
                )));
            }

            if start_index > 0 || end_index < self.filtered_items.len() {
                let scroll_info = THEME.fg(
                    "muted",
                    &format!(
                        "  ({}/{})",
                        self.selected_index + 1,
                        self.filtered_items.len()
                    ),
                );
                new_list.add_child(Box::new(Text::new(scroll_info, 0, 0)));
            }
        }

        if self.list_pos < self.layout.children().len() {
            self.layout.children_mut()[self.list_pos] = Box::new(new_list);
        }
    }

    fn update_footer(&mut self) {
        let text = self.get_footer_text();
        if self.footer_pos < self.layout.children().len() {
            self.layout.children_mut()[self.footer_pos] = Box::new(Text::new(text, 0, 0));
        }
    }

    pub fn handle_input(&mut self, data: &str) {
        let kb = get_keybindings();

        // Navigation
        if kb.matches(data, "tui.select.up") {
            if self.filtered_items.is_empty() {
                return;
            }
            self.selected_index = if self.selected_index == 0 {
                self.filtered_items.len().saturating_sub(1)
            } else {
                self.selected_index - 1
            };
            self.update_list();
            return;
        }

        if kb.matches(data, "tui.select.down") {
            if self.filtered_items.is_empty() {
                return;
            }
            self.selected_index = if self.selected_index + 1 >= self.filtered_items.len() {
                0
            } else {
                self.selected_index + 1
            };
            self.update_list();
            return;
        }

        // Toggle on Enter
        if kb.matches(data, "tui.select.confirm") {
            if let Some(item) = self.filtered_items.get(self.selected_index) {
                self.enabled_ids = toggle(&self.enabled_ids, &item.full_id);
                self.is_dirty = true;
                self.refresh();
                self.notify_change();
            }
            return;
        }

        // Enable all
        if kb.matches(data, "app.models.enableAll") {
            let target_ids: Option<Vec<String>> = if !self.search_input.get_value().is_empty() {
                Some(
                    self.filtered_items
                        .iter()
                        .map(|i| i.full_id.clone())
                        .collect(),
                )
            } else {
                None
            };
            self.enabled_ids = enable_all(
                &self.enabled_ids,
                &self.all_ids,
                target_ids.as_ref().map(|v| v.as_slice()),
            );
            self.is_dirty = true;
            self.refresh();
            self.notify_change();
            return;
        }

        // Clear all
        if kb.matches(data, "app.models.clearAll") {
            let target_ids: Option<Vec<String>> = if !self.search_input.get_value().is_empty() {
                Some(
                    self.filtered_items
                        .iter()
                        .map(|i| i.full_id.clone())
                        .collect(),
                )
            } else {
                None
            };
            self.enabled_ids = clear_all(
                &self.enabled_ids,
                &self.all_ids,
                target_ids.as_ref().map(|v| v.as_slice()),
            );
            self.is_dirty = true;
            self.refresh();
            self.notify_change();
            return;
        }

        // Toggle provider
        if kb.matches(data, "app.models.toggleProvider") {
            if let Some(item) = self.filtered_items.get(self.selected_index) {
                let provider = &item.model.provider;
                let provider_ids: Vec<String> = self
                    .all_ids
                    .iter()
                    .filter(|id| {
                        self.models_by_id
                            .get(*id)
                            .map_or(false, |m| &m.provider == provider)
                    })
                    .cloned()
                    .collect();
                let all_enabled = provider_ids
                    .iter()
                    .all(|id| is_enabled(&self.enabled_ids, id));
                self.enabled_ids = if all_enabled {
                    clear_all(&self.enabled_ids, &self.all_ids, Some(&provider_ids))
                } else {
                    enable_all(&self.enabled_ids, &self.all_ids, Some(&provider_ids))
                };
                self.is_dirty = true;
                self.refresh();
                self.notify_change();
            }
            return;
        }

        // Save
        if kb.matches(data, "app.models.save") {
            (self.callbacks.on_persist)(self.enabled_ids.clone());
            self.is_dirty = false;
            self.update_footer();
            return;
        }

        // Ctrl+C - clear search or cancel
        if matches_key(data, &Key::ctrl('c')) {
            if !self.search_input.get_value().is_empty() {
                self.search_input.set_value("");
                self.refresh();
            } else {
                (self.callbacks.on_cancel)();
            }
            return;
        }

        // Escape - cancel
        if matches_key(data, Key::escape()) {
            (self.callbacks.on_cancel)();
            return;
        }

        // Pass to search input
        self.search_input.handle_input(data);
        self.refresh();
    }
}

impl Component for ScopedModelsSelectorComponent {
    fn render(&self, width: u16) -> Vec<String> {
        self.layout.render(width)
    }

    fn invalidate(&mut self) {
        self.layout.invalidate();
    }
}

impl Focusable for ScopedModelsSelectorComponent {
    fn is_focused(&self) -> bool {
        self.focused
    }

    fn set_focused(&mut self, focused: bool) {
        self.focused = focused;
        self.search_input.set_focused(focused);
    }
}
