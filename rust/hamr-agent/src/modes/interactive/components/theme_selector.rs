//! Component that renders a theme selector.
//!
//! Port of `packages/coding-agent/src/modes/interactive/components/theme-selector.ts`.

use crate::modes::interactive::components::dynamic_border::DynamicBorder;
use crate::modes::interactive::components::tui_shim::{
    Component, Container, SelectItem, SelectList, SelectListLayoutOptions,
};

const THEME_SELECT_LIST_LAYOUT: SelectListLayoutOptions = SelectListLayoutOptions {
    min_primary_column_width: 12,
    max_primary_column_width: 32,
};

/// A component that renders a theme selector with available themes list.
pub struct ThemeSelectorComponent {
    layout: Container,
    select_list: SelectList,
}

impl ThemeSelectorComponent {
    /// Create a new theme selector.
    ///
    /// * `current_theme` - the currently active theme name
    /// * `on_select` - called with the chosen theme name
    /// * `on_cancel` - called on cancel
    /// * `on_preview` - called as selection changes to preview the theme
    pub fn new(
        current_theme: &str,
        on_select: Box<dyn Fn(String) + Send + Sync>,
        on_cancel: Box<dyn Fn() + Send + Sync>,
        _on_preview: Box<dyn Fn(String) + Send + Sync>,
    ) -> Self {
        // Get available themes (from the theme submodule)
        let themes = crate::modes::interactive::theme::theme::get_available_theme_names();
        let theme_items: Vec<SelectItem> = themes
            .iter()
            .map(|name| {
                let description = if name == current_theme {
                    Some("(current)".to_string())
                } else {
                    None
                };
                SelectItem {
                    value: name.clone(),
                    label: name.clone(),
                    description,
                }
            })
            .collect();

        let mut layout = Container::new();

        // Top border
        layout.add_child(Box::new(DynamicBorder::new(None)));

        // Create the select list
        let current_theme_owned = current_theme.to_string();
        let current_index = themes.iter().position(|t| t == &current_theme_owned);

        let mut select_list = SelectList::new(
            theme_items,
            10,
            {
                let on_select = on_select;
                move |item: SelectItem| {
                    on_select(item.value);
                }
            },
            {
                move || {
                    on_cancel();
                }
            },
        );

        // Pre-select current theme
        if let Some(idx) = current_index {
            select_list.set_selected_index(idx);
        }

        layout.add_child(Box::new(select_list));

        // Bottom border
        layout.add_child(Box::new(DynamicBorder::new(None)));

        Self {
            layout,
            select_list: SelectList::new(vec![], 10, |_| {}, || {}),
        }
    }
}

impl Component for ThemeSelectorComponent {
    fn render(&self, width: u16) -> Vec<String> {
        self.layout.render(width)
    }

    fn invalidate(&mut self) {
        self.layout.invalidate();
    }
}
