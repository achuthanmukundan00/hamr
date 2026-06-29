//! Ported from packages/tui/test/truncate-to-width.test.ts
//!
//! Tests for truncateToWidth, visibleWidth, and normalizeTerminalOutput.

use sexy_tui_rs::utils::{normalize_terminal_output, truncate_to_width, visible_width};

// =============================================================================
// truncateToWidth
// =============================================================================

mod truncate_to_width_tests {
    use super::*;

    #[test]
    fn test_keeps_output_within_width_for_large_unicode_input() {
        let text = "🙂界".repeat(100_000);
        let truncated = truncate_to_width(&text, 40, Some("…"));

        assert!(visible_width(&truncated) <= 40);
        assert!(truncated.ends_with("…\x1b[0m"));
    }

    #[test]
    fn test_preserves_ansi_styling_and_resets_before_and_after_ellipsis() {
        let styled = format!("\x1b[31m{}\x1b[0m", "hello ".repeat(1000));
        let truncated = truncate_to_width(&styled, 20, Some("…"));

        assert!(visible_width(&truncated) <= 20);
        assert!(truncated.contains("\x1b[31m"));
        assert!(truncated.ends_with("\x1b[0m…\x1b[0m"));
    }

    #[test]
    fn test_handles_malformed_ansi_escape_prefixes_without_hanging() {
        let text = format!("abc\x1bnot-ansi {}", "🙂".repeat(1000));
        let truncated = truncate_to_width(&text, 20, Some("…"));

        assert!(visible_width(&truncated) <= 20);
    }

    #[test]
    fn test_clips_wide_ellipsis_safely_and_brackets_with_resets() {
        assert_eq!(truncate_to_width("abcdef", 1, Some("🙂")), "");
        assert_eq!(
            truncate_to_width("abcdef", 2, Some("🙂")),
            "\x1b[0m🙂\x1b[0m"
        );
        assert!(visible_width(&truncate_to_width("abcdef", 2, Some("🙂"))) <= 2);
    }

    #[test]
    fn test_returns_original_text_when_it_fits_even_if_ellipsis_is_wide() {
        assert_eq!(truncate_to_width("a", 2, Some("🙂")), "a");
        assert_eq!(truncate_to_width("界", 2, Some("🙂")), "界");
    }

    #[test]
    fn test_adds_trailing_reset_when_truncating_without_ellipsis() {
        let truncated =
            truncate_to_width(&format!("\x1b[31m{}", "hello".repeat(100)), 10, Some(""));
        assert!(visible_width(&truncated) <= 10);
        assert!(truncated.ends_with("\x1b[0m"));
    }
}

// =============================================================================
// visibleWidth
// =============================================================================

mod visible_width_tests {
    use super::*;

    #[test]
    fn test_counts_tabs_inline_and_skips_ansi() {
        assert_eq!(visible_width("\t\x1b[31m界\x1b[0m"), 5);
    }

    #[test]
    fn test_keeps_thai_and_lao_am_clusters_at_normal_cell_width() {
        assert_eq!(visible_width("ำ"), 1);
        assert_eq!(visible_width("ຳ"), 1);
        assert_eq!(visible_width("กำ"), 2);
        assert_eq!(visible_width("ກຳ"), 2);
    }

    #[test]
    fn test_normalizes_thai_and_lao_am_vowels() {
        assert_eq!(normalize_terminal_output("ำ"), "ํา");
        assert_eq!(normalize_terminal_output("ຳ"), "ໍາ");
        assert_eq!(
            visible_width(&normalize_terminal_output("ำabc")),
            visible_width("ำabc")
        );
        assert_eq!(
            visible_width(&normalize_terminal_output("ຳabc")),
            visible_width("ຳabc")
        );
    }
}
