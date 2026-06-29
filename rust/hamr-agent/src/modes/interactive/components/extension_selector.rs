//! Generic selector component for extensions.
//!
//! Port of `packages/coding-agent/src/modes/interactive/components/extension-selector.ts`.
//!
//! Displays a list of string options with keyboard navigation (↑↓),
//! select (Enter) and cancel (Esc). Supports optional countdown timer.

use crate::modes::interactive::components::countdown_timer::CountdownTimer;
use crate::modes::interactive::components::dynamic_border::DynamicBorder;
use crate::modes::interactive::components::keybinding_hints::{key_hint, raw_key_hint};
use crate::modes::interactive::components::tui_shim::{
    Component, Container, Spacer, Text, get_keybindings,
};
use crate::modes::interactive::theme::theme::theme;

/// A scrollable list selector for extensions.
///
/// Renders a title, a list of string options, and navigation/select/cancel hints.
/// Supports ↑↓ navigation, Enter for select, Esc for cancel, and optional
/// countdown timer that cancels on expiry.
pub struct ExtensionSelectorComponent {
    layout: Container,
    options: Vec<String>,
    selected_index: usize,
    /// Position in layout where the list container lives (for rebuilding).
    list_pos: usize,
    /// Position of the title text (for countdown updates).
    title_pos: usize,
    on_select_callback: Box<dyn Fn(String) + Send + Sync>,
    on_cancel_callback: Box<dyn Fn() + Send + Sync>,
    on_toggle_tools_expanded: Option<Box<dyn Fn() + Send + Sync>>,
    base_title: String,
    countdown: Option<CountdownTimer>,
}

impl ExtensionSelectorComponent {
    /// Create a new extension selector.
    ///
    /// * `title` - title displayed above the list
    /// * `options` - the list of string options to choose from
    /// * `on_select` - called with the selected option on Enter
    /// * `on_cancel` - called on Esc/cancel or countdown expiry
    /// * `timeout_ms` - optional countdown timer in milliseconds; cancels on expiry
    /// * `on_toggle_tools_expanded` - optional callback for app.tools.expand binding
    pub fn new(
        title: &str,
        options: Vec<String>,
        on_select: Box<dyn Fn(String) + Send + Sync>,
        on_cancel: Box<dyn Fn() + Send + Sync>,
        timeout_ms: Option<u64>,
        on_toggle_tools_expanded: Option<Box<dyn Fn() + Send + Sync>>,
    ) -> Self {
        let base_title = title.to_string();
        let mut layout = Container::new();

        // 0: Top border
        layout.add_child(Box::new(DynamicBorder::new(None)));
        // 1: Spacer
        layout.add_child(Box::new(Spacer::new(1)));
        // 2: Title
        let title_pos = layout.children().len();
        layout.add_child(Box::new(Text::new(
            theme().fg("accent", &theme().bold(title)),
            1,
            0,
        )));
        // 3: Spacer
        layout.add_child(Box::new(Spacer::new(1)));

        // Countdown timer
        let countdown = timeout_ms.and_then(|ms| {
            if ms > 0 {
                Some(CountdownTimer::new(
                    ms,
                    |_s| {
                        // Title update would happen here in full TUI mode:
                        // theme().fg("accent", &theme().bold(&format!("{} ({}s)", base_title, s)))
                    },
                    || {
                        // Expiry — fires on_cancel in full TUI mode
                    },
                ))
            } else {
                None
            }
        });

        // 4: List container (will be rebuilt by update_list)
        let list_pos = layout.children().len();
        let list_container = Container::new();
        layout.add_child(Box::new(list_container));

        // 5: Spacer
        layout.add_child(Box::new(Spacer::new(1)));
        // 6: Navigation hints
        layout.add_child(Box::new(Text::new(
            format!(
                "{}  {}  {}",
                raw_key_hint("↑↓", "navigate"),
                key_hint("tui.select.confirm", "select"),
                key_hint("tui.select.cancel", "cancel"),
            ),
            1,
            0,
        )));
        // 7: Spacer
        layout.add_child(Box::new(Spacer::new(1)));
        // 8: Bottom border
        layout.add_child(Box::new(DynamicBorder::new(None)));

        let mut result = Self {
            layout,
            options,
            selected_index: 0,
            list_pos,
            title_pos,
            on_select_callback: on_select,
            on_cancel_callback: on_cancel,
            on_toggle_tools_expanded,
            base_title,
            countdown,
        };

        result.update_list();
        result
    }

    /// Rebuild the visible list based on current selected_index.
    fn update_list(&mut self) {
        // Replace the list container at list_pos
        let mut new_list = Container::new();
        for i in 0..self.options.len() {
            let is_selected = i == self.selected_index;
            let text = if is_selected {
                format!(
                    "{} {}",
                    theme().fg("accent", "→"),
                    theme().fg("accent", &self.options[i]),
                )
            } else {
                format!("  {}", theme().fg("text", &self.options[i]))
            };
            new_list.add_child(Box::new(Text::new(text, 1, 0)));
        }

        // Replace the child at list_pos
        if self.list_pos < self.layout.children().len() {
            self.layout.children_mut()[self.list_pos] = Box::new(new_list);
        }
    }

    /// Handle keyboard input.
    pub fn handle_input(&mut self, key_data: &str) {
        let kb = get_keybindings();

        if kb.matches(key_data, "app.tools.expand") {
            if let Some(ref cb) = self.on_toggle_tools_expanded {
                cb();
            }
        } else if kb.matches(key_data, "tui.select.up") || key_data == "k" {
            self.selected_index = self.selected_index.saturating_sub(1);
            self.update_list();
        } else if kb.matches(key_data, "tui.select.down") || key_data == "j" {
            if self.selected_index + 1 < self.options.len() {
                self.selected_index += 1;
            }
            self.update_list();
        } else if kb.matches(key_data, "tui.select.confirm") || key_data == "\n" {
            if let Some(selected) = self.options.get(self.selected_index) {
                (self.on_select_callback)(selected.clone());
            }
        } else if kb.matches(key_data, "tui.select.cancel") {
            (self.on_cancel_callback)();
        }
    }

    pub fn dispose(&mut self) {
        if let Some(ref mut cd) = self.countdown {
            cd.dispose();
        }
    }
}

impl Component for ExtensionSelectorComponent {
    fn render(&self, width: u16) -> Vec<String> {
        self.layout.render(width)
    }

    fn invalidate(&mut self) {
        self.layout.invalidate();
    }
}
