//! Port of `packages/coding-agent/src/hamr/persistent-editor.ts`.
//!
//! Persistent editor extension — keeps the input editor open at the bottom
//! of the TUI with a compact footer showing prompt line, cwd/branch, model
//! name with usage stats, and extension statuses.
//!
//! Registers:
//! - Shortcut (Shift+Ctrl+U) to toggle persistent mode
//! - Command `persistent-editor` to toggle via slash command
//!
//! When `ExtensionAPI` and TUI types are fully ported, the `create` function
//! will accept `pi: &dyn ExtensionAPI` and register the shortcut + command.

use std::sync::Arc;

use crate::core::extensions::types::{ExtensionAPI, ExtensionFactory};

// ─── PersistentFooterComponent ───────────────────────────────────────────────

/// Custom footer component rendered in persistent editor mode.
///
/// Shows:
///   1. Prompt line: `persistent> <editor text or "type a message...">`
///   2. Cwd and git branch
///   3. Model name with context usage percentage and window size
///   4. Extension statuses from `footerData`
///
/// Port of the TS `PersistentFooterComponent` class.
pub struct PersistentFooterComponent;

impl PersistentFooterComponent {
    /// Create a new persistent footer component.
    pub fn new() -> Self {
        PersistentFooterComponent
    }

    /// Render the footer into lines at the given terminal width.
    ///
    /// Port of `render(width: number): string[]` in the TS class.
    ///
    /// When `ExtensionContext`, `Theme`, and `ReadonlyFooterDataProvider` are
    /// ported, this will accept them as arguments and use their data.
    pub fn render(&self, width: usize) -> Vec<String> {
        let mut lines: Vec<String> = Vec::new();

        // Line 1: prompt
        let prompt_prefix = "persistent> ";
        // TODO: read editor text from ctx.ui.getEditorText()
        let editor_text = "";
        let prompt_str = if !editor_text.is_empty() {
            format!("{}{}", prompt_prefix, editor_text)
        } else {
            format!("{}type a message...", prompt_prefix)
        };
        lines.push(truncate_str(&prompt_str, width));

        // Line 2: cwd + git branch
        // TODO: read cwd from ctx.cwd, branch from footerData.getGitBranch()
        let cwd = std::env::current_dir()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|_| "?".to_string());
        let branch: Option<String> = None; // TODO
        let pwd_str = match branch {
            Some(ref b) => format!("{} ({})", cwd, b),
            None => cwd,
        };
        lines.push(truncate_str(&pwd_str, width));

        // Line 3: model name + usage stats
        // TODO: read from ctx.model, ctx.getContextUsage()
        let model_name = "no-model";
        let usage = format!("{} | ?/?", model_name);
        lines.push(truncate_str(&usage, width));

        // Line 4+: extension statuses
        // TODO: read from footerData.getExtensionStatuses()
        // For now, no statuses.

        lines
    }
}

/// Truncate a string to fit within `max_width`.
/// Returns the string as-is if it fits, otherwise cuts it to width.
fn truncate_str(s: &str, max_width: usize) -> String {
    if s.len() <= max_width {
        s.to_string()
    } else {
        s[..max_width].to_string()
    }
}

// ─── Extension factory ───────────────────────────────────────────────────────

/// Extension name constant.
pub const EXTENSION_NAME: &str = "hamr-persistent-editor";

/// Creates the persistent editor extension.
///
/// Port of `createPersistentEditorExtension()` in the TS source.
///
/// In TS, this returns a closure `(pi: ExtensionAPI) => void` that closes
/// over `persistentEnabled`. In Rust, this will return an `ExtensionFactory`
/// (an `Arc<dyn Fn(&dyn ExtensionAPI) -> Result<()>>`) once the types are
/// defined.
///
/// Behavior:
/// 1. Defines `togglePersistentMode` closure that flips state
/// 2. `enablePersistentMode`: sets custom footer, shows "enabled" notification
/// 3. `disablePersistentMode`: resets footer, shows "disabled" notification
/// 4. Registers shortcut `Key.shiftCtrl("u")` → togglePersistentMode
/// 5. Registers command "persistent-editor" → togglePersistentMode
pub fn create_persistent_editor_extension() -> ExtensionFactory {
    Arc::new(|_pi: Arc<dyn ExtensionAPI>| {
        Box::pin(async move {
            // TODO: full implementation when TUI types are ported.
            // This will:
            // 1. Register shortcut Shift+Ctrl+U to toggle persistent mode
            // 2. Register command "persistent-editor" to toggle via slash command
            // 3. Set/unset custom footer via ExtensionContext.ui
            //
            // For now, this is a no-op factory that wires nothing.
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate_str_fits() {
        assert_eq!(truncate_str("hello", 10), "hello");
    }

    #[test]
    fn test_truncate_str_exact() {
        assert_eq!(truncate_str("hello", 5), "hello");
    }

    #[test]
    fn test_truncate_str_truncates() {
        assert_eq!(truncate_str("hello world", 5), "hello");
    }

    #[test]
    fn test_persistent_footer_render() {
        let component = PersistentFooterComponent::new();
        let lines = component.render(80);
        assert!(!lines.is_empty());
        assert!(lines[0].contains("persistent>"));
    }

    #[test]
    fn test_extension_name() {
        assert_eq!(EXTENSION_NAME, "hamr-persistent-editor");
    }

    #[test]
    fn test_create_returns_factory() {
        let factory = create_persistent_editor_extension();
        // Factory should be a valid ExtensionFactory (Arc'd closure)
        let _ = factory;
    }
}
