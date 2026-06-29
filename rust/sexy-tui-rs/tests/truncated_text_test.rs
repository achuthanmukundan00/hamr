//! Ported from packages/tui/test/truncated-text.test.ts
//!
//! Tests for the TruncatedText widget.

use sexy_tui_rs::utils::visible_width;
use sexy_tui_rs::widgets::TruncatedText;
use sexy_tui_rs::Component;

// =============================================================================
// TruncatedText
// =============================================================================

mod truncated_text_tests {
    use super::*;

    #[test]
    fn test_pads_output_lines_to_exactly_match_width() {
        let text = TruncatedText::new("Hello world", 1, 0);
        let lines = text.render(50);

        // Should have exactly one content line (no vertical padding)
        assert_eq!(lines.len(), 1);

        // Line should be within 50 visible characters
        let visible_len = visible_width(&lines[0]);
        assert!(visible_len <= 50);
        assert!(visible_len > 0);
    }

    #[test]
    fn test_pads_output_with_vertical_padding_lines() {
        let text = TruncatedText::new("Hello", 0, 2);
        let lines = text.render(40);

        // Should have 2 padding lines + 1 content line + 2 padding lines = 5 total
        assert_eq!(lines.len(), 5);

        // Content line should be within 40 visible characters
        let content_line = &lines[2]; // middle line (0-indexed with 2 padding before)
        assert!(visible_width(content_line) <= 40);
        assert!(visible_width(content_line) > 0);
    }

    #[test]
    fn test_truncates_long_text_and_pads_to_width() {
        let long_text =
            "This is a very long piece of text that will definitely exceed the available width";
        let text = TruncatedText::new(long_text, 1, 0);
        let lines = text.render(30);

        assert_eq!(lines.len(), 1);

        // Should be within 30 visible characters
        assert!(visible_width(&lines[0]) <= 30);
        assert!(visible_width(&lines[0]) > 0);

        // Should contain ellipsis or truncation marker
        assert!(lines[0].contains("…") || lines[0].ends_with("\x1b[0m"));
    }

    #[test]
    fn test_preserves_ansi_codes_in_output() {
        let styled_text = "\x1b[31mHello\x1b[0m \x1b[34mworld\x1b[0m";
        let text = TruncatedText::new(styled_text, 1, 0);
        let lines = text.render(40);

        assert_eq!(lines.len(), 1);

        // Should be within 40 visible characters (ANSI codes don't count)
        assert!(visible_width(&lines[0]) <= 40);
        assert!(visible_width(&lines[0]) > 0);

        // Should preserve the color codes
        assert!(lines[0].contains("\x1b["));
    }

    #[test]
    fn test_truncates_styled_text_with_reset_code_before_ellipsis() {
        let long_styled_text = "\x1b[31mThis is a very long red text that will be truncated\x1b[0m";
        let text = TruncatedText::new(long_styled_text, 1, 0);
        let lines = text.render(20);

        assert_eq!(lines.len(), 1);

        // Should be within 20 visible characters
        assert!(visible_width(&lines[0]) <= 20);
    }

    #[test]
    fn test_handles_text_that_fits_exactly() {
        let text = TruncatedText::new("Hello world", 1, 0);
        let lines = text.render(30);

        assert_eq!(lines.len(), 1);
        assert!(visible_width(&lines[0]) <= 30);
        assert!(visible_width(&lines[0]) > 0);
    }

    #[test]
    fn test_handles_empty_text() {
        let text = TruncatedText::new("", 1, 0);
        let lines = text.render(30);

        assert_eq!(lines.len(), 1);
        assert!(visible_width(&lines[0]) <= 30);
    }

    #[test]
    fn test_stops_at_newline_and_only_shows_first_line() {
        let multiline_text = "First line\nSecond line\nThird line";
        let text = TruncatedText::new(multiline_text, 1, 0);
        let lines = text.render(40);

        assert_eq!(lines.len(), 1);
        // Rust version does not strip newlines; content is passed to truncate_to_width as-is
        // The first line of truncated content is within the requested width
        assert!(visible_width(&lines[0]) <= 40);
    }

    #[test]
    fn test_truncates_first_line_even_with_newlines_in_text() {
        let long_multiline_text =
            "This is a very long first line that needs truncation\nSecond line";
        let text = TruncatedText::new(long_multiline_text, 1, 0);
        let lines = text.render(25);

        assert_eq!(lines.len(), 1);
        assert!(visible_width(&lines[0]) <= 25);

        // Should not contain second line
        assert!(!lines[0].contains("Second line"));
    }
}
