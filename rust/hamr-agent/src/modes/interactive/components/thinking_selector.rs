//! Port of `packages/coding-agent/src/modes/interactive/components/thinking-selector.ts`.
//!
//! Component that renders a thinking level selector with borders.

use crate::modes::interactive::components::dynamic_border::DynamicBorder;
use crate::modes::interactive::components::tui_shim::{
    Component, Container, SelectItem, SelectList, SelectListLayoutOptions,
};
use crate::modes::interactive::theme::theme::theme;

/// Available thinking levels (mirrors TS `ThinkingLevel`).
pub const THINKING_OFF: &str = "off";
pub const THINKING_MINIMAL: &str = "minimal";
pub const THINKING_LOW: &str = "low";
pub const THINKING_MEDIUM: &str = "medium";
pub const THINKING_HIGH: &str = "high";
pub const THINKING_XHIGH: &str = "xhigh";

const THINKING_SELECT_LIST_LAYOUT: SelectListLayoutOptions = SelectListLayoutOptions {
    min_primary_column_width: 12,
    max_primary_column_width: 32,
};

/// Descriptions for each thinking level.
fn level_description(level: &str) -> &'static str {
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

/// Component that renders a thinking level selector with borders.
pub struct ThinkingSelectorComponent {
    container: Container,
    select_list: SelectList,
}

impl ThinkingSelectorComponent {
    pub fn new(
        current_level: &str,
        available_levels: &[&str],
        on_select: Box<dyn Fn(String) + Send + Sync>,
        on_cancel: Box<dyn Fn() + Send + Sync>,
    ) -> Self {
        let mut container = Container::new();

        let thinking_items: Vec<SelectItem> = available_levels
            .iter()
            .map(|level| SelectItem {
                value: level.to_string(),
                label: level.to_string(),
                description: Some(level_description(level).to_string()),
            })
            .collect();

        // Add top border
        container.add_child(Box::new(DynamicBorder::new(None)));

        // Build callbacks for the select list
        let on_select_clone = move |item: SelectItem| on_select(item.value);
        let on_cancel_clone = on_cancel;

        // Create selector
        let mut select_list = SelectList::new(
            thinking_items.clone(),
            thinking_items.len(),
            on_select_clone,
            on_cancel_clone,
        );

        // Preselect current level
        if let Some(current_index) = thinking_items
            .iter()
            .position(|item| item.value == current_level)
        {
            select_list.set_selected_index(current_index);
        }

        // We add the select_list as a child of the container.
        // In the real TUI, this would be a complex component. For the stub,
        // we render the list items directly.
        container.add_child(Box::new(SelectListStub {
            items: thinking_items.clone(),
            selected_index: thinking_items
                .iter()
                .position(|item| item.value == current_level)
                .unwrap_or(0),
        }));

        // Add bottom border
        container.add_child(Box::new(DynamicBorder::new(None)));

        ThinkingSelectorComponent {
            container,
            select_list,
        }
    }

    pub fn get_select_list(&self) -> &SelectList {
        &self.select_list
    }

    pub fn get_select_list_mut(&mut self) -> &mut SelectList {
        &mut self.select_list
    }
}

impl Component for ThinkingSelectorComponent {
    fn render(&self, width: u16) -> Vec<String> {
        self.container.render(width)
    }

    fn invalidate(&mut self) {
        self.container.invalidate();
    }
}

/// A simple stub to render SelectItems without the real SelectList component.
struct SelectListStub {
    items: Vec<SelectItem>,
    selected_index: usize,
}

impl Component for SelectListStub {
    fn render(&self, _width: u16) -> Vec<String> {
        let t = theme();
        let mut lines = Vec::new();
        for (i, item) in self.items.iter().enumerate() {
            let is_selected = i == self.selected_index;
            let prefix = if is_selected { "→ " } else { "  " };
            let styled = if is_selected {
                t.fg("accent", &format!("{}{}", prefix, item.label))
            } else {
                format!("{}{}", prefix, item.label)
            };
            if let Some(ref desc) = item.description {
                lines.push(format!("{}  {}", styled, t.fg("muted", desc)));
            } else {
                lines.push(styled);
            }
        }
        lines
    }

    fn invalidate(&mut self) {}
}
