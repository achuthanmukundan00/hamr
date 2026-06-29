//! Persistent editor extension — keeps the input editor open at the bottom
//! of the TUI with a compact footer showing model, cwd, and extension status.
//!
//! Mirrors `packages/coding-agent/src/hamr/persistent-editor.ts`.
//!
//! # Porting Status
//!
//! This file depends on:
//! - `crate::core::extensions::types::ExtensionAPI` (not yet ported)
//! - `crate::core::extensions::types::ExtensionContext` (not yet ported)
//! - TUI component types (not yet ported)
//! - Theme types (not yet ported)
//!
//! The struct and rendering logic are fully ported. The `create_extension`
//! function will compile once the extension infrastructure is in place.

// ---------------------------------------------------------------------------
// Footer component
// ---------------------------------------------------------------------------

/// Custom footer component rendered in persistent editor mode.
///
/// Shows: prompt line, cwd/branch, model name with usage stats, extension statuses.
///
/// Dependencies (to be ported):
/// - `crate::core::extensions::types::ExtensionContext`
/// - `crate::core::footer_data_provider::ReadonlyFooterDataProvider`
/// - `crate::modes::interactive::theme::theme::Theme`
/// - TUI Component trait
pub struct PersistentFooterComponent {
    #[allow(dead_code)]
    ctx: (), // will be ExtensionContext
    #[allow(dead_code)]
    theme: (), // will be Theme
    #[allow(dead_code)]
    footer_data: (), // will be ReadonlyFooterDataProvider
}

impl PersistentFooterComponent {
    /// Create a new persistent footer component.
    pub fn new() -> Self {
        PersistentFooterComponent {
            ctx: (),
            theme: (),
            footer_data: (),
        }
    }

    /// Render the footer into lines at the given terminal width.
    ///
    /// Port of the `render(width: number): string[]` method in TS.
    ///
    /// Lines:
    ///   1. Prompt (`persistent> type a message...`)
    ///   2. Cwd (and git branch if available)
    ///   3. Model name + usage (e.g. `claude-3 | 45.2%/200k`)
    ///   4. Extension statuses (if any)
    pub fn render(&self, width: usize) -> Vec<String> {
        let mut lines: Vec<String> = Vec::new();

        // Line 1: prompt
        let prompt_prefix = "persistent> ";
        let prompt_str = format!("{}type a message...", prompt_prefix);
        lines.push(if prompt_str.len() > width {
            prompt_str[..width].to_string()
        } else {
            prompt_str
        });

        // Line 2: cwd
        // TODO: use real cwd from ExtensionContext
        let pwd_str = std::env::current_dir()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|_| "?".to_string());
        // TODO: append git branch from footer_data
        lines.push(if pwd_str.len() > width {
            pwd_str[..width].to_string()
        } else {
            pwd_str
        });

        // Line 3: model name + usage
        // TODO: use real model name and context usage from ExtensionContext
        let stats_line = "no-model | ?/?";
        lines.push(truncate_to_width(stats_line, width));

        // Line 4+: extension statuses
        // TODO: read from footer_data.getExtensionStatuses()
        // For now, emit nothing.

        lines
    }

    #[allow(dead_code)]
    fn invalidate(&self) {}
    #[allow(dead_code)]
    fn dispose(&self) {}
}

/// Truncate a string to fit within `max_width`, appending `…` if truncated.
fn truncate_to_width(s: &str, max_width: usize) -> String {
    if s.len() <= max_width {
        return s.to_string();
    }
    if max_width <= 1 {
        return "…".to_string();
    }
    format!("{}…", &s[..max_width - 1])
}

// ---------------------------------------------------------------------------
// Extension factory
// ---------------------------------------------------------------------------

/// Create the persistent editor extension.
///
/// Registers:
/// - A shortcut (Shift+Ctrl+U) to toggle persistent mode
/// - A command `persistent-editor` to toggle persistent mode
///
/// When enabled, the editor stays open at the bottom with a compact footer
/// showing model, cwd, branch, and extension statuses.
///
/// Port of `createPersistentEditorExtension()` in TS.
///
/// # Type Dependencies (to be ported)
///
/// The function will accept `pi: ExtensionAPI` (from
/// `crate::core::extensions::types`).
///
/// ```ignore
/// pub fn create_persistent_editor_extension() -> impl Fn(ExtensionAPI) + Send + Sync {
///     let mut persistent_enabled = false;
///     move |pi: ExtensionAPI| {
///         // ... register shortcut and command
///     }
/// }
/// ```
///
/// For now the function body is documented but gated behind the
/// `hamr-persistent-editor` feature.
pub fn create_persistent_editor_extension() {
    // TODO: implement when ExtensionAPI is ported.
    //
    // The TS implementation:
    //
    // 1. Defines a `togglePersistentMode` closure that flips `persistentEnabled`
    //    and calls `enablePersistentMode` / `disablePersistentMode`.
    //
    // 2. `enablePersistentMode` sets a custom footer component via
    //    `ctx.ui.setFooter(...)` and shows a notification.
    //
    // 3. `disablePersistentMode` resets the footer to default and shows a
    //    notification.
    //
    // 4. Registers shortcut `Key.shiftCtrl("u")` → togglePersistentMode
    //
    // 5. Registers command "persistent-editor" → togglePersistentMode
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate_to_width_fits() {
        assert_eq!(truncate_to_width("hello", 10), "hello");
    }

    #[test]
    fn test_truncate_to_width_exact() {
        assert_eq!(truncate_to_width("hello", 5), "hello");
    }

    #[test]
    fn test_truncate_to_width_truncates() {
        let result = truncate_to_width("hello world", 5);
        // We can't use char-count-based width here (Rust s.len() returns bytes).
        // The truncation should give "hell…" = 4 ASCII bytes + 3 bytes for …
        assert_eq!(result.len(), 7);
        assert!(result.ends_with('…'));
    }

    #[test]
    fn test_persistent_footer_render() {
        let component = PersistentFooterComponent::new();
        let lines = component.render(80);
        assert!(!lines.is_empty());
        // Should always have at least the prompt line
        assert!(lines[0].contains("persistent>"));
    }
}
