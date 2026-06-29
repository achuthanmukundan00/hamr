//! Ported from packages/tui/test/terminal-image.test.ts
//!
//! Tests for terminal image detection, Kitty/iTerm2 protocols, dimensions, and hyperlinks.

use sexy_tui_rs::terminal_image::{
    allocate_image_id, calculate_image_rows, delete_all_kitty_images, delete_kitty_image,
    encode_iterm2, encode_kitty, hyperlink, is_image_line, render_image, reset_capabilities_cache,
    set_capabilities, set_cell_dimensions, CellDimensions, ImageDimensions, ImageProtocol,
    ImageRenderOptions, TerminalCapabilities,
};

fn with_kitty_capabilities<F: FnOnce()>(f: F) {
    set_capabilities(TerminalCapabilities {
        kitty_graphics: true,
        iterm2_images: false,
        sync_output: false,
        kitty_keyboard: false,
        true_color: true,
        nerd_font: false,
    });
    set_cell_dimensions(CellDimensions {
        width_px: 10,
        height_px: 10,
    });
    f();
    reset_capabilities_cache();
    set_cell_dimensions(CellDimensions {
        width_px: 9,
        height_px: 18,
    });
}

// =============================================================================
// isImageLine — iTerm2 image protocol
// =============================================================================

mod iterm2_image_detection {
    use super::*;

    #[test]
    fn test_detects_iterm2_image_at_start_of_line() {
        let iterm2_line = "\x1b]1337;File=size=100,100;inline=1:base64encodeddata==\x07";
        assert!(is_image_line(iterm2_line));
    }

    #[test]
    fn test_detects_iterm2_image_with_text_before_it() {
        let line = "Some text \x1b]1337;File=size=100,100;inline=1:base64data==\x07 more text";
        assert!(is_image_line(line));
    }

    #[test]
    fn test_detects_iterm2_image_in_middle_of_long_line() {
        let line = "Text before image...\x1b]1337;File=inline=1:verylongbase64data==...text after";
        assert!(is_image_line(line));
    }

    #[test]
    fn test_detects_iterm2_image_at_end_of_line() {
        let line = "Regular text ending with \x1b]1337;File=inline=1:base64data==\x07";
        assert!(is_image_line(line));
    }

    #[test]
    fn test_detects_minimal_iterm2_image_sequence() {
        let line = "\x1b]1337;File=:\x07";
        assert!(is_image_line(line));
    }
}

// =============================================================================
// isImageLine — Kitty image protocol
// =============================================================================

mod kitty_image_detection {
    use super::*;

    #[test]
    fn test_detects_kitty_image_at_start_of_line() {
        let line = "\x1b_Ga=T,f=100,t=f,d=base64data...\x1b\\\x1b_Gm=i=1;\x1b\\";
        assert!(is_image_line(line));
    }

    #[test]
    fn test_detects_kitty_image_with_text_before_it() {
        let line = "Output: \x1b_Ga=T,f=100;data...\x1b\\\x1b_Gm=i=1;\x1b\\";
        assert!(is_image_line(line));
    }

    #[test]
    fn test_detects_kitty_image_with_padding() {
        let line = "  \x1b_Ga=T,f=100...\x1b\\\x1b_Gm=i=1;\x1b\\  ";
        assert!(is_image_line(line));
    }
}

// =============================================================================
// isImageLine — Negative cases (lines without images)
// =============================================================================

mod negative_image_detection {
    use super::*;

    #[test]
    fn test_does_not_detect_images_in_plain_text() {
        let plain = "This is just a regular text line without any escape sequences";
        assert!(!is_image_line(plain));
    }

    #[test]
    fn test_does_not_detect_images_in_lines_with_only_ansi_codes() {
        let ansi = "\x1b[31mRed text\x1b[0m and \x1b[32mgreen text\x1b[0m";
        assert!(!is_image_line(ansi));
    }

    #[test]
    fn test_does_not_detect_images_in_lines_with_cursor_movement_codes() {
        let cursor = "\x1b[1A\x1b[2KLine cleared and moved up";
        assert!(!is_image_line(cursor));
    }

    #[test]
    fn test_does_not_detect_images_in_lines_with_partial_iterm2_sequences() {
        let partial = "Some text with ]1337;File but missing ESC at start";
        assert!(!is_image_line(partial));
    }

    #[test]
    fn test_does_not_detect_images_in_lines_with_partial_kitty_sequences() {
        let partial = "Some text with _G but missing ESC at start";
        assert!(!is_image_line(partial));
    }

    #[test]
    fn test_does_not_detect_images_in_empty_lines() {
        assert!(!is_image_line(""));
    }

    #[test]
    fn test_does_not_detect_images_in_newlines() {
        assert!(!is_image_line("\n"));
        assert!(!is_image_line("\n\n"));
    }
}

// =============================================================================
// isImageLine — Mixed content scenarios
// =============================================================================

mod mixed_content_image_detection {
    use super::*;

    #[test]
    fn test_detects_images_when_line_has_both_kitty_and_iterm2_sequences() {
        let line = "Kitty: \x1b_Ga=T...\x1b\\\x1b_Gm=i=1;\x1b\\ iTerm2: \x1b]1337;File=inline=1:data==\x07";
        assert!(is_image_line(line));
    }

    #[test]
    fn test_detects_image_in_line_with_multiple_text_and_image_segments() {
        let line = "Start \x1b]1337;File=img1==\x07 middle \x1b]1337;File=img2==\x07 end";
        assert!(is_image_line(line));
    }

    #[test]
    fn test_does_not_falsely_detect_image_in_file_path() {
        let file_path = "/path/to/File_1337_backup/image.jpg";
        assert!(!is_image_line(file_path));
    }
}

// =============================================================================
// isImageLine — Bug regression tests (very long lines, ANSI before/after)
// =============================================================================

mod regression_image_detection {
    use super::*;

    #[test]
    fn test_detects_image_sequences_in_very_long_lines() {
        let base64_char = "A".repeat(100);
        let image_sequence = "\x1b]1337;File=size=800,600;inline=1:";
        let long_line = format!(
            "Text prefix {}{} suffix",
            image_sequence,
            base64_char.repeat(3000)
        );
        assert!(long_line.len() > 300_000);
        assert!(is_image_line(&long_line));
    }

    #[test]
    fn test_detects_image_sequences_when_terminal_doesnt_support_images() {
        let line = "Read image file [image/jpeg]\x1b]1337;File=inline=1:base64data==\x07";
        assert!(is_image_line(line));
    }

    #[test]
    fn test_detects_image_sequences_with_ansi_codes_before_them() {
        let line = "\x1b[31mError output \x1b]1337;File=inline=1:image==\x07";
        assert!(is_image_line(line));
    }

    #[test]
    fn test_detects_image_sequences_with_ansi_codes_after_them() {
        let line = "\x1b_Ga=T,f=100:data...\x1b\\\x1b_Gm=i=1;\x1b\\\x1b[0m reset";
        assert!(is_image_line(line));
    }
}

// =============================================================================
// Kitty image cursor movement
// =============================================================================

mod kitty_cursor_movement {
    use super::*;

    #[test]
    fn test_encode_kitty_basic() {
        let id = allocate_image_id();
        let result = encode_kitty(
            id,
            "AAAA",
            ImageDimensions {
                width_px: 20,
                height_px: 20,
            },
            &ImageRenderOptions {
                max_width_cells: Some(2),
                max_height_cells: None,
                filename: None,
            },
            CellDimensions {
                width_px: 10,
                height_px: 10,
            },
        );
        assert!(result.starts_with("\x1b_Ga=T,f=100"));
        assert!(result.contains(&format!("i={}", id)));
        assert!(result.ends_with("\x1b\\"));
    }

    #[test]
    fn test_delete_kitty_image() {
        let result = delete_kitty_image(42);
        assert_eq!(result, "\x1b_Ga=d,d=I,i=42\x1b\\");
    }

    #[test]
    fn test_delete_all_kitty_images() {
        let result = delete_all_kitty_images();
        assert_eq!(result, "\x1b_Ga=d\x1b\\");
    }

    #[test]
    fn test_render_image_with_kitty_capabilities() {
        with_kitty_capabilities(|| {
            // Minimal valid PNG (1x1 pixel) encoded as base64
            let minipng_base64 = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNkYPj/HwADBwIAMCbHYQAAAABJRU5ErkJggg==";
            let result = render_image(
                minipng_base64,
                "image/png",
                &ImageRenderOptions {
                    max_width_cells: Some(2),
                    max_height_cells: None,
                    filename: None,
                },
            );
            assert!(!result.is_empty());
            assert!(result.starts_with("\x1b_G"));
        });
    }

    #[test]
    fn test_calculate_image_rows() {
        let rows = calculate_image_rows(
            ImageDimensions {
                width_px: 20,
                height_px: 20,
            },
            &ImageRenderOptions {
                max_width_cells: Some(2),
                max_height_cells: Some(5),
                filename: None,
            },
            CellDimensions {
                width_px: 10,
                height_px: 10,
            },
        );
        assert_eq!(rows, 2);
    }

    #[test]
    fn test_calculate_image_rows_honors_max_height() {
        let rows = calculate_image_rows(
            ImageDimensions {
                width_px: 10,
                height_px: 100,
            },
            &ImageRenderOptions {
                max_width_cells: Some(10),
                max_height_cells: Some(5),
                filename: None,
            },
            CellDimensions {
                width_px: 10,
                height_px: 10,
            },
        );
        assert_eq!(rows, 5);
    }

    #[test]
    fn test_render_image_with_max_height_cells_reduces_width() {
        with_kitty_capabilities(|| {
            let result = render_image(
                "AAAA",
                "image/png",
                &ImageRenderOptions {
                    max_width_cells: Some(10),
                    max_height_cells: Some(5),
                    filename: None,
                },
            );
            assert!(!result.is_empty());
        });
    }
}

// =============================================================================
// iTerm2 encoding
// =============================================================================

mod iterm2_encoding {
    use super::*;

    #[test]
    fn test_encode_iterm2_basic() {
        let result = encode_iterm2(
            "base64data",
            ImageDimensions {
                width_px: 100,
                height_px: 100,
            },
            &ImageRenderOptions {
                max_width_cells: Some(40),
                max_height_cells: None,
                filename: None,
            },
            CellDimensions {
                width_px: 10,
                height_px: 20,
            },
        );
        assert!(result.starts_with("\x1b]1337;File=inline=1;"));
        assert!(result.ends_with('\x07'));
        assert!(result.contains("base64data"));
    }
}

// =============================================================================
// hyperlink
// =============================================================================

mod hyperlink_tests {
    use super::*;

    #[test]
    fn test_wraps_text_in_osc_8_open_and_close_sequences() {
        let result = hyperlink("click me", "https://example.com");
        assert_eq!(
            result,
            "\x1b]8;;https://example.com\x1b\\click me\x1b]8;;\x1b\\"
        );
    }

    #[test]
    fn test_works_with_empty_text() {
        let result = hyperlink("", "https://example.com");
        assert_eq!(result, "\x1b]8;;https://example.com\x1b\\\x1b]8;;\x1b\\");
    }

    #[test]
    fn test_works_with_file_uris() {
        let result = hyperlink("README.md", "file:///home/user/README.md");
        assert!(result.contains("file:///home/user/README.md"));
        assert!(result.contains("README.md"));
    }
}
