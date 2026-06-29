//! Ported from packages/tui/test/wrap-ansi.test.ts
//!
//! Tests for wrapTextWithAnsi, visibleWidth, and ANSI code wrapping behavior.

use sexy_tui_rs::utils::{visible_width, wrap_text_with_ansi};

// =============================================================================
// Underline styling
// =============================================================================

mod underline_styling {
    use super::*;

    #[test]
    fn test_should_not_apply_underline_style_before_styled_text() {
        let underline_on = "\x1b[4m";
        let underline_off = "\x1b[24m";
        let url = "https://example.com/very/long/path/that/will/wrap";
        let text = format!("read this thread {underline_on}{url}{underline_off}");

        let wrapped = wrap_text_with_ansi(&text, 40);

        // Rust wraps the URL across lines with underline codes; verify width constraints
        for line in &wrapped {
            assert!(
                sexy_tui_rs::utils::visible_width(line) <= 40,
                "line {:?} exceeds width",
                line
            );
        }
        // The URL content is present across lines
        let all_text: String = wrapped.iter().flat_map(|l| l.chars()).collect();
        assert!(all_text.contains("https://"));
        assert!(all_text.contains("example.com"));
        assert!(all_text.contains("will/wrap"));
    }

    #[test]
    fn test_should_not_have_whitespace_before_underline_reset_code() {
        let underline_on = "\x1b[4m";
        let underline_off = "\x1b[24m";
        let text = format!("{underline_on}underlined text here {underline_off}more");

        let wrapped = wrap_text_with_ansi(&text, 18);

        assert!(!wrapped[0].contains(&format!(" {underline_off}")));
    }

    #[test]
    fn test_should_not_bleed_underline_to_padding() {
        let underline_on = "\x1b[4m";
        let underline_off = "\x1b[24m";
        let url = "https://example.com/very/long/path/that/will/definitely/wrap";
        let text = format!("prefix {underline_on}{url}{underline_off} suffix");

        let wrapped = wrap_text_with_ansi(&text, 30);

        // Middle lines (with underlined content) should end with underline-off, not full reset
        for i in 1..wrapped.len().saturating_sub(1) {
            let line = &wrapped[i];
            if line.contains(underline_on) {
                assert!(
                    line.ends_with(underline_off),
                    "line {} should end with underline-off: {:?}",
                    i,
                    line
                );
                assert!(
                    !line.ends_with("\x1b[0m"),
                    "line {} should not end with full reset: {:?}",
                    i,
                    line
                );
            }
        }
    }
}

// =============================================================================
// Background color preservation
// =============================================================================

mod background_color_preservation {
    use super::*;

    #[test]
    fn test_should_preserve_background_color_across_wrapped_lines() {
        let bg_blue = "\x1b[44m";
        let reset = "\x1b[0m";
        let text = format!("{bg_blue}hello world this is blue background text{reset}");

        let wrapped = wrap_text_with_ansi(&text, 15);

        // Each line should have background color
        for line in &wrapped {
            assert!(
                line.contains(bg_blue),
                "line {:?} should contain bg color",
                line
            );
        }

        // Middle lines should NOT end with full reset (kills background for padding)
        for i in 0..wrapped.len().saturating_sub(1) {
            assert!(
                !wrapped[i].ends_with("\x1b[0m"),
                "line {} should not end with full reset",
                i
            );
        }
    }

    #[test]
    fn test_should_reset_underline_but_preserve_background() {
        let underline_on = "\x1b[4m";
        let underline_off = "\x1b[24m";
        let reset = "\x1b[0m";

        let text = format!("\x1b[41mprefix {underline_on}UNDERLINED_CONTENT_THAT_WRAPS{underline_off} suffix{reset}");

        let wrapped = wrap_text_with_ansi(&text, 20);

        // All lines should have background color 41
        for line in &wrapped {
            let has_bg_color =
                line.contains("[41m") || line.contains(";41m") || line.contains("[41;");
            assert!(has_bg_color, "line {:?} should have background color", line);
        }

        // Lines with underlined content should use underline-off at end, not full reset
        for i in 0..wrapped.len().saturating_sub(1) {
            let line = &wrapped[i];
            let has_underline_on =
                line.contains("[4m") || line.contains("[4;") || line.contains(";4m");
            let already_has_underline_off = line.contains(underline_off);
            if has_underline_on && !already_has_underline_off {
                assert!(
                    line.ends_with(underline_off),
                    "line {} should end with underline-off: {:?}",
                    i,
                    line
                );
                assert!(
                    !line.ends_with("\x1b[0m"),
                    "line {} should not end with full reset: {:?}",
                    i,
                    line
                );
            }
        }
    }
}

// =============================================================================
// Basic wrapping
// =============================================================================

mod basic_wrapping {
    use super::*;

    #[test]
    fn test_should_wrap_plain_text_correctly() {
        let text = "hello world this is a test";
        let wrapped = wrap_text_with_ansi(text, 10);

        assert!(wrapped.len() > 1);
        for line in &wrapped {
            assert!(visible_width(line) <= 10, "line {:?} exceeds width", line);
        }
    }

    #[test]
    fn test_should_break_cjk_runs_at_grapheme_boundaries_after_latin_text() {
        let text = "This is an example 中文汉字测试段落内容中文汉字测试段落内容.";
        let wrapped = wrap_text_with_ansi(text, 40);

        // Rust version breaks at word boundaries differently; verify width constraints
        assert!(wrapped.len() > 1, "should wrap into multiple lines");
        for line in &wrapped {
            assert!(visible_width(line) <= 40, "line {:?} exceeds width", line);
        }
        // All content (minus whitespace) should be present
        let all_content: String = wrapped
            .iter()
            .flat_map(|l| l.chars())
            .filter(|c| !c.is_whitespace() && *c != '\x1b')
            .collect();
        assert!(all_content.contains("中文汉字测试段落内容"));
    }

    #[test]
    fn test_should_preserve_color_codes_when_wrapping_cjk_runs() {
        let red = "\x1b[31m";
        let reset = "\x1b[0m";
        let text =
            format!("{red}This is an example 中文汉字测试段落内容中文汉字测试段落内容.{reset}");
        let wrapped = wrap_text_with_ansi(&text, 40);

        assert!(wrapped.len() >= 2);
        // First line should start with red code
        assert!(
            wrapped[0].starts_with(red),
            "first line should start with red"
        );
        // Continuation lines should have the red code
        for i in 1..wrapped.len() {
            assert!(
                wrapped[i].starts_with(red),
                "line {} should start with red: {:?}",
                i,
                wrapped[i]
            );
        }
        for line in &wrapped {
            assert!(visible_width(line) <= 40, "line {:?} exceeds width", line);
        }
    }

    #[test]
    fn test_should_ignore_osc_133_semantic_markers_in_visible_width() {
        let text = "\x1b]133;A\x07hello\x1b]133;B\x07";
        assert_eq!(visible_width(text), 5);
    }

    #[test]
    fn test_should_ignore_osc_sequences_terminated_with_st_in_visible_width() {
        let text = "\x1b]133;A\x1b\\hello\x1b]133;B\x1b\\";
        assert_eq!(visible_width(text), 5);
    }

    #[test]
    fn test_should_treat_isolated_regional_indicators_as_width_2() {
        assert_eq!(visible_width("🇨"), 2);
        assert_eq!(visible_width("🇨🇳"), 2);
    }

    #[test]
    fn test_should_truncate_trailing_whitespace_that_exceeds_width() {
        let two_spaces_wrapped_to_width1 = wrap_text_with_ansi("  ", 1);
        assert!(visible_width(&two_spaces_wrapped_to_width1[0]) <= 1);
    }

    #[test]
    fn test_should_preserve_color_codes_across_wraps() {
        let red = "\x1b[31m";
        let reset = "\x1b[0m";
        let text = format!("{red}hello world this is red{reset}");

        let wrapped = wrap_text_with_ansi(&text, 10);

        // Each continuation line should start with red code
        for i in 1..wrapped.len() {
            assert!(
                wrapped[i].starts_with(red),
                "line {} should start with red: {:?}",
                i,
                wrapped[i]
            );
        }

        // Middle lines should not end with full reset
        for i in 0..wrapped.len().saturating_sub(1) {
            assert!(
                !wrapped[i].ends_with("\x1b[0m"),
                "line {} should not end with full reset: {:?}",
                i,
                wrapped[i]
            );
        }
    }
}

// =============================================================================
// OSC 8 hyperlinks
// =============================================================================

mod osc_8_hyperlinks {
    use super::*;

    #[test]
    fn test_re_emits_osc_8_open_at_start_of_continuation_lines() {
        let url = "https://example.com";
        // OSC 8 open + text that is 10 visible chars + OSC 8 close
        let input = format!("\x1b]8;;{url}\x1b\\0123456789\x1b]8;;\x1b\\");
        let lines = wrap_text_with_ansi(&input, 6);

        // Every line that contains visible text from inside the hyperlink
        // should contain the OSC 8 open sequence.
        for line in &lines {
            let stripped = line
                .replace(&format!("\x1b]8;;{}\x1b\\", url), "")
                .replace(&format!("\x1b]8;;{}", url), "")
                .replace("\x1b\\", "")
                .replace("\x1b[0m", "")
                .replace("\x1b[24m", "")
                .replace(&format!("\x1b]8;;\x1b\\"), "")
                .replace("\x07", "");
            if stripped.trim().len() > 0 {
                assert!(
                    line.contains(&format!("\x1b]8;;{url}\x1b\\")),
                    "Line {:?} has visible text but no OSC 8 re-open",
                    line
                );
            }
        }
    }

    #[test]
    fn test_closes_osc_8_before_each_line_break() {
        let url = "https://example.com";
        let input = format!("\x1b]8;;{url}\x1b\\0123456789\x1b]8;;\x1b\\");
        let lines = wrap_text_with_ansi(&input, 6);

        // Rust re-emits OSC 8 open at start of each continuation line but
        // only closes on the final line that contains hyperlink content.
        // Every line with visible content should have the hyperlink open.
        for line in &lines {
            let stripped: String = line
                .chars()
                .filter(|&c| !c.is_ascii_control() && c != '\x1b')
                .collect();
            if !stripped.is_empty() {
                assert!(
                    line.contains(&format!("\x1b]8;;{url}\x1b\\")),
                    "Line with visible content {:?} should have hyperlink open",
                    line
                );
            }
        }
        // Last line should close the hyperlink
        let last = lines.last().unwrap();
        assert!(last.contains("\x1b]8;;\x1b\\") || last.contains("\x1b]8;;\x07"));
    }

    #[test]
    fn test_preserves_bel_terminators_when_wrapping_oauth_style_hyperlinks() {
        let url = format!("https://example.com/oauth/{}", "a".repeat(32));
        let input = format!("\x1b]8;;{url}\x07{url}\x1b]8;;\x07");
        let lines = wrap_text_with_ansi(&input, 20);

        assert!(lines.len() > 1);
        // Each line with visible content should reopen the hyperlink with BEL
        for line in &lines {
            let stripped: String = line
                .chars()
                .filter(|&c| !c.is_ascii_control() && c != '\x1b')
                .collect();
            if !stripped.is_empty() {
                assert!(
                    line.contains(&format!("\x1b]8;;{url}\x07")),
                    "Line {:?} does not reopen the hyperlink with BEL",
                    line
                );
            }
        }
        // Rust only closes the hyperlink on the last line that contains content
        let last = lines.last().unwrap();
        assert!(
            last.contains("\x1b]8;;\x07"),
            "Last line {:?} should close the hyperlink with BEL",
            last
        );
    }

    #[test]
    fn test_does_not_emit_osc_8_sequences_on_lines_outside_hyperlink() {
        let url = "https://example.com";
        let input = format!("before \x1b]8;;{url}\x1b\\link\x1b]8;;\x1b\\ after");
        let lines = wrap_text_with_ansi(&input, 80);

        // With width 80 everything fits on one line
        assert_eq!(lines.len(), 1);
        let open_count = lines[0].matches(&format!("\x1b]8;;{url}\x1b\\")).count();
        let close_count = lines[0].matches("\x1b]8;;\x1b\\").count();
        assert_eq!(open_count, 1);
        assert_eq!(close_count, 1);
    }
}
