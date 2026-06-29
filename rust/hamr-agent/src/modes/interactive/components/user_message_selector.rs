//! Port of `packages/coding-agent/src/modes/interactive/components/user-message-selector.ts`.
//!
//! Component that renders a user message selector for branching.

use crate::modes::interactive::components::dynamic_border::DynamicBorder;
use crate::modes::interactive::components::tui_shim::{
    Component, Spacer, Text, get_keybindings, truncate_to_width,
};
use crate::modes::interactive::theme::theme::theme;

/// A user message item in the selector list.
#[derive(Debug, Clone)]
pub struct UserMessageItem {
    /// Entry ID in the session
    pub id: String,
    /// The message text
    pub text: String,
    /// Optional timestamp
    pub timestamp: Option<String>,
}

/// Custom user message list component with selection.
struct UserMessageList {
    messages: Vec<UserMessageItem>,
    selected_index: usize,
    max_visible: usize,
    pub on_select: Option<Box<dyn Fn(String) + Send + Sync>>,
    pub on_cancel: Option<Box<dyn Fn() + Send + Sync>>,
}

impl UserMessageList {
    fn new(messages: Vec<UserMessageItem>, initial_selected_id: Option<&str>) -> Self {
        let selected_index = initial_selected_id
            .and_then(|id| messages.iter().position(|m| m.id == id))
            .unwrap_or_else(|| messages.len().saturating_sub(1).max(0));

        UserMessageList {
            messages,
            selected_index,
            max_visible: 10,
            on_select: None,
            on_cancel: None,
        }
    }

    fn handle_input(&mut self, key_data: &str) {
        let kb = get_keybindings();
        if kb.matches(key_data, "tui.select.up") {
            if self.selected_index == 0 {
                self.selected_index = self.messages.len().saturating_sub(1);
            } else {
                self.selected_index -= 1;
            }
        } else if kb.matches(key_data, "tui.select.down") {
            if self.selected_index >= self.messages.len().saturating_sub(1) {
                self.selected_index = 0;
            } else {
                self.selected_index += 1;
            }
        } else if kb.matches(key_data, "tui.select.confirm") {
            if let Some(selected) = self.messages.get(self.selected_index) {
                if let Some(ref cb) = self.on_select {
                    cb(selected.id.clone());
                }
            }
        } else if kb.matches(key_data, "tui.select.cancel") {
            if let Some(ref cb) = self.on_cancel {
                cb();
            }
        }
    }
}

impl Component for UserMessageList {
    fn render(&self, width: u16) -> Vec<String> {
        let t = theme();
        let mut lines: Vec<String> = Vec::new();

        if self.messages.is_empty() {
            lines.push(t.fg("muted", "  No user messages found"));
            return lines;
        }

        // Calculate visible range with scrolling
        let start_index = 0usize.max(
            self.selected_index
                .saturating_sub(self.max_visible / 2)
                .min(self.messages.len().saturating_sub(self.max_visible)),
        );
        let end_index = (start_index + self.max_visible).min(self.messages.len());

        // Render visible messages
        for i in start_index..end_index {
            let message = &self.messages[i];
            let is_selected = i == self.selected_index;

            // Normalize message to single line
            let normalized_message = message.text.replace('\n', " ").trim().to_string();

            // First line: cursor + message
            let cursor = if is_selected {
                t.fg("accent", "› ")
            } else {
                "  ".to_string()
            };
            let max_msg_width = (width as usize).saturating_sub(2);
            let truncated_msg = truncate_to_width(&normalized_message, max_msg_width as u16, "");
            let message_line = if is_selected {
                format!("{}{}", cursor, t.bold(&truncated_msg))
            } else {
                format!("{}{}", cursor, truncated_msg)
            };
            lines.push(message_line);

            // Second line: metadata
            let position = i + 1;
            let metadata = format!("  Message {} of {}", position, self.messages.len());
            lines.push(t.fg("muted", &metadata));
            lines.push(String::new()); // Blank line between messages
        }

        // Add scroll indicator if needed
        if start_index > 0 || end_index < self.messages.len() {
            let scroll_info = format!("  ({}/{})", self.selected_index + 1, self.messages.len());
            lines.push(t.fg("muted", &scroll_info));
        }

        lines
    }

    fn invalidate(&mut self) {}
}

/// Component that renders a user message selector for branching.
pub struct UserMessageSelectorComponent {
    message_list: UserMessageList,
    messages: Vec<UserMessageItem>,
}

impl UserMessageSelectorComponent {
    pub fn new(
        messages: Vec<UserMessageItem>,
        on_select: Box<dyn Fn(String) + Send + Sync>,
        on_cancel: Box<dyn Fn() + Send + Sync>,
        initial_selected_id: Option<&str>,
    ) -> Self {
        let mut message_list = UserMessageList::new(messages.clone(), initial_selected_id);
        message_list.on_select = Some(on_select);
        message_list.on_cancel = Some(on_cancel);

        UserMessageSelectorComponent {
            message_list,
            messages,
        }
    }

    pub fn handle_input(&mut self, key_data: &str) {
        self.message_list.handle_input(key_data);
    }
}

impl Component for UserMessageSelectorComponent {
    fn render(&self, width: u16) -> Vec<String> {
        let t = theme();
        let mut lines: Vec<String> = Vec::new();

        // Header
        lines.extend(Spacer::new(1).render(width));
        lines.extend(Text::new(t.bold("Fork from Message"), 1, 0).render(width));
        lines.extend(Text::new(
            t.fg(
                "muted",
                "Select a user message to copy the active path up to that point into a new session",
            ),
            1,
            0,
        ).render(width));
        lines.extend(Spacer::new(1).render(width));
        lines.extend(DynamicBorder::new(None).render(width));
        lines.extend(Spacer::new(1).render(width));

        // Message list
        lines.extend(self.message_list.render(width));

        // Bottom border
        lines.extend(Spacer::new(1).render(width));
        lines.extend(DynamicBorder::new(None).render(width));

        lines
    }

    fn invalidate(&mut self) {
        self.message_list.invalidate();
    }
}
