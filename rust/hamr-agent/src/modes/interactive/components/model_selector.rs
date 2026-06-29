//! Component that renders a model selector with search.
//!
//! Port of `packages/coding-agent/src/modes/interactive/components/model-selector.ts`.

use crate::modes::interactive::components::dynamic_border::DynamicBorder;
use crate::modes::interactive::components::tui_shim::{
    Component, Container, Focusable, Input, Spacer, Text, TuiHandle, fuzzy_filter, get_keybindings,
};
use crate::modes::interactive::theme::theme::THEME;

/// A model item for the selector.
#[derive(Clone)]
pub struct ModelItem {
    pub provider: String,
    pub id: String,
    pub name: Option<String>,
}

/// A scoped model with optional thinking level.
#[derive(Clone)]
pub struct ScopedModelItem {
    pub model: ModelItem,
    pub thinking_level: Option<String>,
}

/// The scope of models to show.
#[derive(Clone, PartialEq)]
pub enum ModelScope {
    All,
    Scoped,
}

/// A component that renders a searchable model selector list.
pub struct ModelSelectorComponent {
    layout: Container,
    search_input: Input,
    all_models: Vec<ModelItem>,
    scoped_model_items: Vec<ModelItem>,
    active_models: Vec<ModelItem>,
    filtered_models: Vec<ModelItem>,
    selected_index: usize,
    current_model: Option<ModelItem>,
    scope: ModelScope,
    on_select_callback: Box<dyn Fn(ModelItem) + Send + Sync>,
    on_cancel_callback: Box<dyn Fn() + Send + Sync>,
    scope_text_pos: Option<usize>,
    list_pos: usize,
    focused: bool,
}

impl ModelSelectorComponent {
    /// Create a new model selector.
    ///
    /// * `current_model` - the currently active model (if any)
    /// * `all_models` - all available models
    /// * `scoped_models` - subset of models for scoped view
    /// * `on_select` - called with the chosen model
    /// * `on_cancel` - called on cancel
    /// * `initial_search_input` - optional initial search text
    pub fn new(
        _tui: &dyn TuiHandle,
        current_model: Option<ModelItem>,
        all_models: Vec<ModelItem>,
        scoped_models: Vec<ModelItem>,
        on_select: Box<dyn Fn(ModelItem) + Send + Sync>,
        on_cancel: Box<dyn Fn() + Send + Sync>,
        initial_search_input: Option<&str>,
    ) -> Self {
        let scope = if !scoped_models.is_empty() {
            ModelScope::Scoped
        } else {
            ModelScope::All
        };

        let active_models = if scope == ModelScope::Scoped {
            scoped_models.clone()
        } else {
            all_models.clone()
        };

        let filtered_models = active_models.clone();

        let mut layout = Container::new();

        // Top border
        layout.add_child(Box::new(DynamicBorder::new(None)));
        layout.add_child(Box::new(Spacer::new(1)));

        // Scope hint
        let scope_text_pos = if !scoped_models.is_empty() {
            let pos = layout.children().len();
            layout.add_child(Box::new(Text::new("", 0, 0)));
            Some(pos)
        } else {
            let hint =
                "Only showing models from configured providers. Use /login to add providers.";
            layout.add_child(Box::new(Text::new(THEME.fg("warning", hint), 0, 0)));
            None
        };

        layout.add_child(Box::new(Spacer::new(1)));

        // Search input
        let mut search_input = Input::new();
        if let Some(val) = initial_search_input {
            search_input.set_value(val);
        }
        layout.add_child(Box::new(Spacer::new(1))); // placeholder

        layout.add_child(Box::new(Spacer::new(1)));

        // List container
        let list_pos = layout.children().len();
        layout.add_child(Box::new(Container::new()));

        layout.add_child(Box::new(Spacer::new(1)));
        // Bottom border
        layout.add_child(Box::new(DynamicBorder::new(None)));

        let mut result = Self {
            layout,
            search_input,
            all_models,
            scoped_model_items: scoped_models,
            active_models,
            filtered_models,
            selected_index: 0,
            current_model,
            scope,
            on_select_callback: on_select,
            on_cancel_callback: on_cancel,
            scope_text_pos,
            list_pos,
            focused: false,
        };

        result.update_list();
        result
    }

    /// Filter models by search query.
    fn filter_models(&mut self, query: &str) {
        self.filtered_models = if query.is_empty() {
            self.active_models.clone()
        } else {
            fuzzy_filter(&self.active_models, query, |m: &ModelItem| {
                format!(
                    "{} {} {}/{} {} {}",
                    m.id, m.provider, m.provider, m.id, m.provider, m.id
                )
            })
        };
        self.selected_index = self
            .selected_index
            .min(self.filtered_models.len().saturating_sub(1));
        self.update_list();
    }

    /// Set the model scope (all vs scoped).
    fn set_scope(&mut self, scope: ModelScope) {
        if self.scope == scope {
            return;
        }
        self.scope = scope;
        self.active_models = if self.scope == ModelScope::Scoped {
            self.scoped_model_items.clone()
        } else {
            self.all_models.clone()
        };
        self.filter_models("");
    }

    /// Rebuild the visible list.
    fn update_list(&mut self) {
        let mut new_list = Container::new();

        let max_visible = 10usize;
        let start_index = self
            .selected_index
            .saturating_sub(max_visible / 2)
            .min(self.filtered_models.len().saturating_sub(max_visible));
        let end_index = (start_index + max_visible).min(self.filtered_models.len());

        for i in start_index..end_index {
            let item = &self.filtered_models[i];
            let is_selected = i == self.selected_index;
            let is_current = self
                .current_model
                .as_ref()
                .map_or(false, |cm| cm.id == item.id && cm.provider == item.provider);

            let line = if is_selected {
                let check = if is_current {
                    THEME.fg("success", " ✓")
                } else {
                    "".to_string()
                };
                format!(
                    "{} {} {}{}",
                    THEME.fg("accent", "→"),
                    THEME.fg("accent", &item.id),
                    THEME.fg("muted", &format!("[{}]", item.provider)),
                    check,
                )
            } else {
                let check = if is_current {
                    THEME.fg("success", " ✓")
                } else {
                    "".to_string()
                };
                format!(
                    "  {} {}{}",
                    item.id,
                    THEME.fg("muted", &format!("[{}]", item.provider)),
                    check,
                )
            };

            new_list.add_child(Box::new(Text::new(line, 0, 0)));
        }

        // Scroll indicator
        if start_index > 0 || end_index < self.filtered_models.len() {
            let scroll_info = THEME.fg(
                "muted",
                &format!(
                    "  ({}/{})",
                    self.selected_index + 1,
                    self.filtered_models.len()
                ),
            );
            new_list.add_child(Box::new(Text::new(scroll_info, 0, 0)));
        }

        // Empty state
        if self.filtered_models.is_empty() {
            new_list.add_child(Box::new(Text::new(
                THEME.fg("muted", "  No matching models"),
                0,
                0,
            )));
        }

        // Replace list
        if self.list_pos < self.layout.children().len() {
            self.layout.children_mut()[self.list_pos] = Box::new(new_list);
        }
    }

    /// Handle keyboard input.
    pub fn handle_input(&mut self, key_data: &str) {
        let kb = get_keybindings();

        if kb.matches(key_data, "tui.input.tab") && !self.scoped_model_items.is_empty() {
            let next = match self.scope {
                ModelScope::All => ModelScope::Scoped,
                ModelScope::Scoped => ModelScope::All,
            };
            self.set_scope(next);
            return;
        }

        if kb.matches(key_data, "tui.select.up") {
            if self.filtered_models.is_empty() {
                return;
            }
            self.selected_index = if self.selected_index == 0 {
                self.filtered_models.len().saturating_sub(1)
            } else {
                self.selected_index - 1
            };
            self.update_list();
        } else if kb.matches(key_data, "tui.select.down") {
            if self.filtered_models.is_empty() {
                return;
            }
            self.selected_index = if self.selected_index + 1 >= self.filtered_models.len() {
                0
            } else {
                self.selected_index + 1
            };
            self.update_list();
        } else if kb.matches(key_data, "tui.select.confirm") {
            if let Some(model) = self.filtered_models.get(self.selected_index) {
                (self.on_select_callback)(model.clone());
            }
        } else if kb.matches(key_data, "tui.select.cancel") {
            (self.on_cancel_callback)();
        } else {
            self.search_input.handle_input(key_data);
            let query = self.search_input.get_value().to_string();
            self.filter_models(&query);
        }
    }
}

impl Component for ModelSelectorComponent {
    fn render(&self, width: u16) -> Vec<String> {
        self.layout.render(width)
    }

    fn invalidate(&mut self) {
        self.layout.invalidate();
    }
}

impl Focusable for ModelSelectorComponent {
    fn is_focused(&self) -> bool {
        self.focused
    }

    fn set_focused(&mut self, focused: bool) {
        self.focused = focused;
        self.search_input.set_focused(focused);
    }
}
