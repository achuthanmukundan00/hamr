//! Component that renders a show images selector with borders.
//!
//! Port of `packages/coding-agent/src/modes/interactive/components/show-images-selector.ts`.

use crate::modes::interactive::components::dynamic_border::DynamicBorder;
use crate::modes::interactive::components::tui_shim::{
    Component, Container, SelectItem, SelectList, SelectListLayoutOptions,
};

const SHOW_IMAGES_SELECT_LIST_LAYOUT: SelectListLayoutOptions = SelectListLayoutOptions {
    min_primary_column_width: 12,
    max_primary_column_width: 32,
};

/// A component that renders a "Show Images" yes/no selector with borders.
pub struct ShowImagesSelectorComponent {
    layout: Container,
}

impl ShowImagesSelectorComponent {
    /// Create a new show images selector.
    ///
    /// * `current_value` - whether images are currently shown
    /// * `on_select` - called with the new boolean value
    /// * `on_cancel` - called on cancel
    pub fn new(
        current_value: bool,
        on_select: Box<dyn Fn(bool) + Send + Sync>,
        on_cancel: Box<dyn Fn() + Send + Sync>,
    ) -> Self {
        let items = vec![
            SelectItem {
                value: "yes".to_string(),
                label: "Yes".to_string(),
                description: Some("Show images inline in terminal".to_string()),
            },
            SelectItem {
                value: "no".to_string(),
                label: "No".to_string(),
                description: Some("Show text placeholder instead".to_string()),
            },
        ];

        let mut layout = Container::new();

        // Top border
        layout.add_child(Box::new(DynamicBorder::new(None)));

        // Create selector with yes/no items
        let mut select_list = SelectList::new(
            items,
            5,
            {
                let on_select = on_select;
                move |item: SelectItem| {
                    on_select(item.value == "yes");
                }
            },
            {
                move || {
                    on_cancel();
                }
            },
        );

        // Pre-select current value
        select_list.set_selected_index(if current_value { 0 } else { 1 });

        layout.add_child(Box::new(select_list));

        // Bottom border
        layout.add_child(Box::new(DynamicBorder::new(None)));

        Self { layout }
    }
}

impl Component for ShowImagesSelectorComponent {
    fn render(&self, width: u16) -> Vec<String> {
        self.layout.render(width)
    }

    fn invalidate(&mut self) {
        self.layout.invalidate();
    }
}
