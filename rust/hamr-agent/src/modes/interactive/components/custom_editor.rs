//! Multi-line editor component for extensions.
//!
//! Port of `packages/coding-agent/src/modes/interactive/components/custom-editor.ts`.
//!
//! Custom editor that handles app-level keybindings for coding-agent.

use crate::modes::interactive::components::tui_shim::{Component, Editor, Focusable, Keybindings};
use std::collections::HashMap;

/// Application-level keybinding identifier.
pub type AppKeybinding = String;

/// Keybindings manager — delegates to `tui_shim::Keybindings` which
/// wraps `sexy-tui-rs::KeybindingsManager` when the `tui` feature is active.
pub struct KeybindingsManager {
    inner: Keybindings,
}

impl KeybindingsManager {
    pub fn new(inner: Keybindings) -> Self {
        Self { inner }
    }

    pub fn matches(&self, data: &str, binding: &str) -> bool {
        self.inner.matches(data, binding)
    }

    pub fn get_keys(&self, binding: &str) -> Vec<String> {
        self.inner.get_keys(binding)
    }
}

/// Custom editor that handles app-level keybindings for coding-agent.
pub struct CustomEditor {
    editor: Editor,
    keybindings: KeybindingsManager,
    pub action_handlers: HashMap<AppKeybinding, Box<dyn Fn() + Send + Sync>>,
    pub on_escape: Option<Box<dyn Fn() + Send + Sync>>,
    pub on_ctrl_d: Option<Box<dyn Fn() + Send + Sync>>,
    pub on_paste_image: Option<Box<dyn Fn() + Send + Sync>>,
    pub on_extension_shortcut: Option<Box<dyn Fn(&str) -> bool + Send + Sync>>,
}

impl CustomEditor {
    pub fn new(keybindings: KeybindingsManager) -> Self {
        Self {
            editor: Editor::new(),
            keybindings,
            action_handlers: HashMap::new(),
            on_escape: None,
            on_ctrl_d: None,
            on_paste_image: None,
            on_extension_shortcut: None,
        }
    }

    /// Register a handler for an app action.
    pub fn on_action(&mut self, action: AppKeybinding, handler: Box<dyn Fn() + Send + Sync>) {
        self.action_handlers.insert(action, handler);
    }

    /// Handle input key data. Overrides Editor::handle_input to intercept app keybindings.
    pub fn handle_input(&mut self, data: &str) {
        // Check extension-registered shortcuts first
        if let Some(ref handler) = self.on_extension_shortcut {
            if handler(data) {
                return;
            }
        }

        // Check for paste image keybinding
        if self.keybindings.matches(data, "app.clipboard.pasteImage") {
            if let Some(ref handler) = self.on_paste_image {
                handler();
                return;
            }
        }

        // Escape/interrupt - only if autocomplete is NOT active
        if self.keybindings.matches(data, "app.interrupt") {
            if !self.editor.is_showing_autocomplete() {
                if let Some(ref handler) = self.on_escape {
                    handler();
                    return;
                }
                if let Some(handler) = self.action_handlers.get("app.interrupt") {
                    handler();
                    return;
                }
            }
            self.editor.handle_input(data);
            return;
        }

        // Exit (Ctrl+D) - only when editor is empty
        if self.keybindings.matches(data, "app.exit") {
            if self.editor.get_text().is_empty() {
                if let Some(ref handler) = self.on_ctrl_d {
                    handler();
                    return;
                }
                if let Some(handler) = self.action_handlers.get("app.exit") {
                    handler();
                    return;
                }
            }
        }

        // Check all other app actions
        let action_keys: Vec<String> = self.action_handlers.keys().cloned().collect();
        for action in &action_keys {
            if action != "app.interrupt" && action != "app.exit" {
                if self.keybindings.matches(data, action) {
                    if let Some(handler) = self.action_handlers.get(action) {
                        handler();
                        return;
                    }
                }
            }
        }

        // Pass to editor
        self.editor.handle_input(data);
    }

    pub fn set_text(&mut self, text: impl Into<String>) {
        self.editor.set_text(text);
    }

    pub fn get_text(&self) -> &str {
        self.editor.get_text()
    }

    pub fn is_showing_autocomplete(&self) -> bool {
        self.editor.is_showing_autocomplete()
    }
}

impl Component for CustomEditor {
    fn render(&self, width: u16) -> Vec<String> {
        self.editor.render(width)
    }

    fn invalidate(&mut self) {
        self.editor.invalidate();
    }
}

impl Focusable for CustomEditor {
    fn is_focused(&self) -> bool {
        self.editor.is_focused()
    }

    fn set_focused(&mut self, focused: bool) {
        self.editor.set_focused(focused);
    }
}
