//! Ported from packages/tui/test/bug-regression-isimageline-startswith-bug.test.ts
//!
//! Regression tests for the isImageLine startsWith bug.
//! The old implementation used startsWith() and would return false when the
//! terminal didn't support images, causing TUI crash "Rendered line exceeds terminal width".
//! Fixed: uses includes() to detect escape sequences anywhere in the line.

use sexy_tui_rs::terminal_image::is_image_line;

// =============================================================================
// Bug regression: isImageLine crash with image escape sequences
// =============================================================================

mod bug_scenario_terminal_without_image_support {
    use super::*;

    #[test]
    fn test_new_implementation_detects_sequences_in_any_position() {
        // Line containing image escape sequence with text before it (common bug scenario)
        let line =
            "Read image file [image/jpeg]\x1b]1337;File=size=800,600;inline=1:base64data...\x07";
        assert!(
            is_image_line(line),
            "Fix: should detect image sequence even when text precedes it"
        );
    }

    #[test]
    fn test_detects_kitty_sequences_in_any_position() {
        let scenarios = [
            "At start: \x1b_Ga=T,f=100,data...\x1b\\",
            "Prefix \x1b_Ga=T,data...\x1b\\",
            "Suffix text \x1b_Ga=T,data...\x1b\\ suffix",
            "Middle \x1b_Ga=T,data...\x1b\\ more text",
        ];

        for line in &scenarios {
            assert!(
                is_image_line(line),
                "Should detect Kitty sequence in: {line}"
            );
        }
    }

    #[test]
    fn test_detects_iterm2_sequences_in_any_position() {
        let scenarios = [
            "At start: \x1b]1337;File=size=100,100:base64...\x07",
            "Prefix \x1b]1337;File=inline=1:data==\x07",
            "Suffix text \x1b]1337;File=inline=1:data==\x07 suffix",
            "Middle \x1b]1337;File=inline=1:data==\x07 more text",
        ];

        for line in &scenarios {
            assert!(
                is_image_line(line),
                "Should detect iTerm2 sequence in: {line}"
            );
        }
    }
}

mod integration_tool_execution_scenario {
    use super::*;

    #[test]
    fn test_detects_image_sequences_in_read_tool_output() {
        let line =
            "Read image file [image/jpeg]\x1b]1337;File=size=800,600;inline=1:base64image...\x07";
        assert!(
            is_image_line(line),
            "Should detect image sequence in tool output line"
        );
    }

    #[test]
    fn test_detects_kitty_sequences_from_image_component() {
        let line = "\x1b_Ga=T,f=100,t=f,d=base64data...\x1b\\\x1b_Gm=i=1;\x1b\\";
        assert!(
            is_image_line(line),
            "Should detect Kitty image component output"
        );
    }

    #[test]
    fn test_handles_ansi_codes_before_image_sequences() {
        let lines = [
            "\x1b[31mError\x1b[0m: \x1b]1337;File=inline=1:base64==\x07",
            "\x1b[33mWarning\x1b[0m: \x1b_Ga=T,data...\x1b\\",
            "\x1b[1mBold\x1b[0m \x1b]1337;File=:base64==\x07\x1b[0m",
        ];

        for line in &lines {
            assert!(
                is_image_line(line),
                "Should detect image sequence after ANSI codes"
            );
        }
    }
}

mod crash_scenario_simulation {
    use super::*;

    #[test]
    fn test_does_not_crash_on_very_long_lines_with_image_sequences() {
        let base64_char = "A".repeat(100);
        let iterm2_sequence = "\x1b]1337;File=size=800,600;inline=1:";

        let crash_line = format!(
            "Output: {}{} end of output",
            iterm2_sequence,
            base64_char.repeat(3040)
        );

        // Verify line is very long
        assert!(crash_line.len() > 300_000, "Test line should be > 300KB");

        // New implementation should detect it (prevents crash)
        let detected = is_image_line(&crash_line);
        assert!(
            detected,
            "Should detect image sequence in very long line, preventing TUI crash"
        );
    }
}

mod negative_cases_no_false_positive {
    use super::*;

    #[test]
    fn test_does_not_detect_images_in_regular_long_text() {
        let long_text = "A".repeat(100_000);
        assert!(
            !is_image_line(&long_text),
            "Should not detect images in plain long text"
        );
    }

    #[test]
    fn test_does_not_detect_images_in_lines_with_file_paths() {
        let file_paths = [
            "/path/to/1337/image.jpg",
            "/usr/local/bin/File_converter",
            "~/Documents/1337File_backup.png",
            "./_G_test_file.txt",
        ];

        for path in &file_paths {
            assert!(
                !is_image_line(path),
                "Should not falsely detect image sequence in path: {path}"
            );
        }
    }
}
