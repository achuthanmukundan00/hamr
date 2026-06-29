//! Utilities for formatting keybinding hints in the UI.
//!
//! Port of `packages/coding-agent/src/modes/interactive/components/keybinding-hints.ts`.

use crate::modes::interactive::components::tui_shim::get_keybindings;
use crate::modes::interactive::theme::theme::theme;

/// Options for formatting key text display.
pub struct KeyTextFormatOptions {
    pub capitalize: bool,
}

impl Default for KeyTextFormatOptions {
    fn default() -> Self {
        Self { capitalize: false }
    }
}

/// Format a single key part. On macOS, "alt" becomes "option".
/// If capitalize is true, capitalizes the first letter.
fn format_key_part(part: &str, options: &KeyTextFormatOptions) -> String {
    let display_part = if cfg!(target_os = "macos") && part.to_lowercase() == "alt" {
        "option".to_string()
    } else {
        part.to_string()
    };

    if options.capitalize {
        let mut chars = display_part.chars();
        match chars.next() {
            Some(first) => {
                let rest: String = chars.collect();
                format!("{}{}", first.to_uppercase(), rest)
            }
            None => String::new(),
        }
    } else {
        display_part
    }
}

/// Format a key string like "ctrl+c" or "alt+enter/tab".
/// Splits by "/", then by "+", and formats each part.
pub fn format_key_text(key: &str, options: &KeyTextFormatOptions) -> String {
    key.split('/')
        .map(|k| {
            k.split('+')
                .map(|part| format_key_part(part, options))
                .collect::<Vec<_>>()
                .join("+")
        })
        .collect::<Vec<_>>()
        .join("/")
}

/// Join key IDs into a formatted key text string.
fn format_keys(keys: &[String], options: &KeyTextFormatOptions) -> String {
    if keys.is_empty() {
        return String::new();
    }
    format_key_text(&keys.join("/"), options)
}

/// Return the raw key text for a keybinding (lowercase).
pub fn key_text(binding: &str) -> String {
    format_keys(
        &get_keybindings().get_keys(binding),
        &KeyTextFormatOptions::default(),
    )
}

/// Return the display key text for a keybinding (capitalized).
pub fn key_display_text(binding: &str) -> String {
    format_keys(
        &get_keybindings().get_keys(binding),
        &KeyTextFormatOptions { capitalize: true },
    )
}

/// Return a styled key hint with dim key text and muted description.
/// e.g., "[Enter] submit"
pub fn key_hint(binding: &str, description: &str) -> String {
    format!(
        "{}{}",
        theme().fg("dim", &key_text(binding)),
        theme().fg("muted", &format!(" {}", description))
    )
}

/// Return an unstyled key hint - key text is formatted, description appended.
pub fn raw_key_hint(key: &str, description: &str) -> String {
    format!(
        "{}{}",
        theme().fg(
            "dim",
            &format_key_text(key, &KeyTextFormatOptions::default())
        ),
        theme().fg("muted", &format!(" {}", description))
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_key_text_basic() {
        let result = format_key_text("ctrl+c", &KeyTextFormatOptions::default());
        assert_eq!(result, "ctrl+c");
    }

    #[test]
    fn test_format_key_text_capitalize() {
        let result = format_key_text("ctrl+c", &KeyTextFormatOptions { capitalize: true });
        assert_eq!(result, "Ctrl+C");
    }

    #[test]
    fn test_format_key_text_multi_key() {
        let result = format_key_text("alt+enter/tab", &KeyTextFormatOptions::default());
        if cfg!(target_os = "macos") {
            assert_eq!(result, "option+enter/tab");
        } else {
            assert_eq!(result, "alt+enter/tab");
        }
    }

    #[test]
    fn test_raw_key_hint() {
        let result = raw_key_hint("↑↓", "navigate");
        // Both parts should use theme coloring - key text dimmed, description muted
        assert!(result.contains("↑↓"));
        assert!(result.contains("navigate"));
    }

    #[test]
    fn test_key_hint() {
        let result = key_hint("tui.select.confirm", "submit");
        assert!(result.contains("submit"));
    }

    #[test]
    fn test_format_key_text_alt_on_macos() {
        let result = format_key_text("alt+enter", &KeyTextFormatOptions::default());
        if cfg!(target_os = "macos") {
            assert_eq!(result, "option+enter");
        } else {
            assert_eq!(result, "alt+enter");
        }
    }
}
