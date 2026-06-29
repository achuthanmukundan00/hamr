//! Multi-line editor component for extensions.
//!
//! Port of `packages/coding-agent/src/modes/interactive/components/extension-editor.ts`.
//!
//! Supports Ctrl+G for external editor.

use crate::modes::interactive::components::dynamic_border::DynamicBorder;
use crate::modes::interactive::components::keybinding_hints::key_hint;
use crate::modes::interactive::components::tui_shim::{
    Component, Container, Editor, Focusable, Spacer, Text,
};
use crate::modes::interactive::theme::theme::theme;

/// Keybindings manager stub.
pub struct KeybindingsManager;

impl KeybindingsManager {
    pub fn matches(&self, _data: &str, _binding: &str) -> bool {
        false
    }
}

/// Multi-line editor component for extension input.
pub struct ExtensionEditorComponent {
    container: Container,
    editor: Editor,
    title: String,
    _on_submit: Option<Box<dyn Fn(String) + Send + Sync>>,
    _on_cancel: Option<Box<dyn Fn() + Send + Sync>>,
    _keybindings: KeybindingsManager,
    has_external_editor: bool,
}

impl ExtensionEditorComponent {
    pub fn new(
        keybindings: KeybindingsManager,
        title: impl Into<String>,
        prefill: Option<String>,
        on_submit: Option<Box<dyn Fn(String) + Send + Sync>>,
        on_cancel: Option<Box<dyn Fn() + Send + Sync>>,
        has_external_editor: bool,
    ) -> Self {
        let title = title.into();
        let mut container = Container::new();
        let mut editor = Editor::new();

        if let Some(ref text) = prefill {
            editor.set_text(text);
        }

        // Top border
        container.add_child(Box::new(DynamicBorder::new(None)));
        container.add_child(Box::new(Spacer::new(1)));

        // Title
        container.add_child(Box::new(Text::new(theme().fg("accent", &title), 1, 0)));
        container.add_child(Box::new(Spacer::new(1)));

        // Editorial area (placeholder)
        container.add_child(Box::new(Container::new()));

        container.add_child(Box::new(Spacer::new(1)));

        // Hint
        let hint = {
            let mut parts = Vec::new();
            parts.push(key_hint("tui.select.confirm", "submit"));
            parts.push(key_hint("tui.input.newLine", "newline"));
            parts.push(key_hint("tui.select.cancel", "cancel"));
            if has_external_editor {
                parts.push(key_hint("app.editor.external", "external editor"));
            }
            parts.join("  ")
        };
        container.add_child(Box::new(Text::new(hint, 1, 0)));
        container.add_child(Box::new(Spacer::new(1)));

        // Bottom border
        container.add_child(Box::new(DynamicBorder::new(None)));

        Self {
            container,
            editor,
            title,
            _on_submit: on_submit,
            _on_cancel: on_cancel,
            _keybindings: keybindings,
            has_external_editor,
        }
    }

    /// Handle input key data. In the real TUI, this interprets keybindings.
    pub fn handle_input(&mut self, _key_data: &str) {
        // Stub: in real TUI integration, this matches keybindings and delegates
        // to editor or calls on_cancel/on_submit callbacks.
    }

    /// Get the current editor text.
    pub fn get_text(&self) -> &str {
        self.editor.get_text()
    }

    /// Set the editor text.
    pub fn set_text(&mut self, text: impl Into<String>) {
        self.editor.set_text(text);
    }
}

impl Component for ExtensionEditorComponent {
    fn render(&self, width: u16) -> Vec<String> {
        self.container.render(width)
    }

    fn invalidate(&mut self) {
        self.container.invalidate();
    }
}

impl Focusable for ExtensionEditorComponent {
    fn is_focused(&self) -> bool {
        self.editor.is_focused()
    }

    fn set_focused(&mut self, focused: bool) {
        self.editor.set_focused(focused);
    }
}
