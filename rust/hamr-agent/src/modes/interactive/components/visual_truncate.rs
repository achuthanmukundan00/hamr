//! Port of `packages/coding-agent/src/modes/interactive/components/visual-truncate.ts`.
//!
//! Shared utility for truncating text to visual lines (accounting for line wrapping).
//! Used by both tool-execution.rs and bash-execution.rs for consistent behavior.

use crate::modes::interactive::components::tui_shim::{Component, Text};

/// Result of truncating text to visual lines.
pub struct VisualTruncateResult {
    /// The visual lines to display
    pub visual_lines: Vec<String>,
    /// Number of visual lines that were skipped (hidden)
    pub skipped_count: usize,
}

/// Truncate text to a maximum number of visual lines (from the end).
/// This accounts for line wrapping based on terminal width.
///
/// - `text`: The text content (may contain newlines)
/// - `max_visual_lines`: Maximum number of visual lines to show
/// - `width`: Terminal/render width
/// - `padding_x`: Horizontal padding for Text component (default 0).
///                Use 0 when result will be placed in a Box (Box adds its own padding).
///                Use 1 when result will be placed in a plain Container.
pub fn truncate_to_visual_lines(
    text: &str,
    max_visual_lines: usize,
    width: u16,
    padding_x: u16,
) -> VisualTruncateResult {
    if text.is_empty() {
        return VisualTruncateResult {
            visual_lines: Vec::new(),
            skipped_count: 0,
        };
    }

    // Create a temporary Text component to render and get visual lines
    let temp_text = Text::new(text, padding_x, 0);
    let all_visual_lines = temp_text.render(width);

    if all_visual_lines.len() <= max_visual_lines {
        return VisualTruncateResult {
            visual_lines: all_visual_lines,
            skipped_count: 0,
        };
    }

    // Take the last N visual lines
    let skip = all_visual_lines.len() - max_visual_lines;
    let truncated_lines: Vec<String> = all_visual_lines.into_iter().skip(skip).collect();

    VisualTruncateResult {
        visual_lines: truncated_lines,
        skipped_count: skip,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_text() {
        let result = truncate_to_visual_lines("", 10, 80, 0);
        assert!(result.visual_lines.is_empty());
        assert_eq!(result.skipped_count, 0);
    }

    #[test]
    fn test_short_text_no_truncation() {
        let result = truncate_to_visual_lines("hello", 10, 80, 0);
        assert!(!result.visual_lines.is_empty());
        assert_eq!(result.skipped_count, 0);
    }

    #[test]
    fn test_long_text_truncation() {
        let long_text = "line\n".repeat(100);
        let result = truncate_to_visual_lines(&long_text, 5, 80, 0);
        assert_eq!(result.visual_lines.len(), 5);
        assert!(result.skipped_count > 0);
    }

    #[test]
    fn test_exact_fit() {
        let text = "line\n".repeat(3);
        let result = truncate_to_visual_lines(&text, 3, 80, 0);
        assert_eq!(result.visual_lines.len(), 3);
        assert_eq!(result.skipped_count, 0);
    }
}
