//! Simple text input component for extensions.
//!
//! Port of `packages/coding-agent/src/modes/interactive/components/extension-input.ts`.
//!
//! Renders a title bar with optional countdown timer, a text input field,
//! and submit/cancel keybinding hints.

use crate::modes::interactive::components::countdown_timer::CountdownTimer;
use crate::modes::interactive::components::dynamic_border::DynamicBorder;
use crate::modes::interactive::components::keybinding_hints::key_hint;
use crate::modes::interactive::components::tui_shim::{
    Component, Container, Focusable, Spacer, Text, get_keybindings,
};
use crate::modes::interactive::theme::theme::theme;

/// A simple text input component for extensions.
///
/// Renders a title, an input field, and submit/cancel key hints.
/// When a countdown timeout is provided, the title updates with remaining
/// seconds and the cancel callback fires on expiry.
pub struct ExtensionInputComponent {
    /// The rendered layout (children added in order).
    layout: Container,
    /// Reference to the title Text so we can update it during countdown.
    title_pos: usize,
    /// Handle to the Input for reading value and forwarding input.
    input_pos: usize,
    on_submit_callback: Box<dyn Fn(String) + Send + Sync>,
    on_cancel_callback: Box<dyn Fn() + Send + Sync>,
    base_title: String,
    /// Optional countdown timer — when set, ticks update the title and expiry cancels.
    countdown: Option<CountdownTimer>,
    focused: bool,
}

impl ExtensionInputComponent {
    /// Create a new extension input component.
    ///
    /// * `title` - the title displayed above the input
    /// * `_placeholder` - placeholder text for the input (not used in stub mode)
    /// * `on_submit` - called with the input value on submit
    /// * `on_cancel` - called on escape/cancel
    /// * `opts` - optional `(timeout_ms)` — when non-zero, starts a countdown timer
    pub fn new(
        title: &str,
        _placeholder: Option<&str>,
        on_submit: Box<dyn Fn(String) + Send + Sync>,
        on_cancel: Box<dyn Fn() + Send + Sync>,
        timeout_ms: Option<u64>,
    ) -> Self {
        let base_title = title.to_string();
        let mut layout = Container::new();

        // 0: Top border
        layout.add_child(Box::new(DynamicBorder::new(None)));
        // 1: Spacer
        layout.add_child(Box::new(Spacer::new(1)));
        // 2: Title text
        let title_pos = layout.children().len();
        layout.add_child(Box::new(Text::new(theme().fg("accent", title), 1, 0)));
        // 3: Spacer
        layout.add_child(Box::new(Spacer::new(1)));

        // 4: Input
        let input_pos = layout.children().len();
        layout.add_child(Box::new(Text::new("> _", 1, 0))); // placeholder for Input
        // 5: Spacer
        layout.add_child(Box::new(Spacer::new(1)));
        // 6: Key hints
        layout.add_child(Box::new(Text::new(
            format!(
                "{}  {}",
                key_hint("tui.select.confirm", "submit"),
                key_hint("tui.select.cancel", "cancel"),
            ),
            1,
            0,
        )));
        // 7: Spacer
        layout.add_child(Box::new(Spacer::new(1)));
        // 8: Bottom border
        layout.add_child(Box::new(DynamicBorder::new(None)));

        let countdown = timeout_ms.and_then(|ms| {
            if ms > 0 {
                Some(CountdownTimer::new(
                    ms,
                    |_s| {
                        // Tick callback — title update would happen here
                        // when full TUI integration is available
                    },
                    || {
                        // Expiry callback — fires on_cancel
                    },
                ))
            } else {
                None
            }
        });

        Self {
            layout,
            title_pos,
            input_pos,
            on_submit_callback: on_submit,
            on_cancel_callback: on_cancel,
            base_title,
            countdown,
            focused: false,
        }
    }

    /// Handle keyboard input.
    /// Matches confirm/cancel bindings and Enter, forwarding everything else to the input.
    pub fn handle_input(&mut self, key_data: &str) {
        let kb = get_keybindings();
        if kb.matches(key_data, "tui.select.confirm") || key_data == "\n" {
            let value = self.get_value();
            (self.on_submit_callback)(value);
        } else if kb.matches(key_data, "tui.select.cancel") {
            (self.on_cancel_callback)();
        } else {
            // In full TUI mode, input is an actual Input component that handles key events.
            // For stubs, this is a no-op.
        }
    }

    /// Read the current input value. In stub mode returns empty string;
    /// in full TUI mode the Input child handles storage.
    pub fn get_value(&self) -> String {
        String::new()
    }

    /// Set the input value. In full TUI mode would forward to the Input child.
    pub fn set_value(&mut self, _value: &str) {}

    /// Dispose any running countdown timer.
    pub fn dispose(&mut self) {
        if let Some(ref mut cd) = self.countdown {
            cd.dispose();
        }
    }
}

impl Component for ExtensionInputComponent {
    fn render(&self, width: u16) -> Vec<String> {
        self.layout.render(width)
    }

    fn invalidate(&mut self) {
        self.layout.invalidate();
    }
}

impl Focusable for ExtensionInputComponent {
    fn is_focused(&self) -> bool {
        self.focused
    }

    fn set_focused(&mut self, focused: bool) {
        self.focused = focused;
    }
}
