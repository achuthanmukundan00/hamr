//! Ported from packages/tui/test/terminal-colors.test.ts
//!
//! Tests for terminal color parsing: OSC 11 background color, color scheme reports, RgbColor.

use sexy_tui_rs::terminal_colors::{
    parse_osc11_background_color, parse_terminal_color_scheme_report, RgbColor,
};

// =============================================================================
// parseOsc11BackgroundColor
// =============================================================================

mod parse_osc11_background_color_tests {
    use super::*;

    #[test]
    fn test_parses_16_bit_osc_11_rgb_responses() {
        assert_eq!(
            parse_osc11_background_color("\x1b]11;rgb:0000/8000/ffff\x07"),
            Some(RgbColor {
                r: 0,
                g: 128,
                b: 255
            })
        );
    }

    #[test]
    fn test_parses_osc_11_hex_responses() {
        assert_eq!(
            parse_osc11_background_color("\x1b]11;#ffffff\x1b\\"),
            Some(RgbColor {
                r: 255,
                g: 255,
                b: 255
            })
        );
        assert_eq!(
            parse_osc11_background_color("\x1b]11;#000000\x07"),
            Some(RgbColor { r: 0, g: 0, b: 0 })
        );
    }

    #[test]
    fn test_rejects_non_strict_osc_11_responses() {
        assert_eq!(parse_osc11_background_color("x\x1b]11;#ffffff\x07"), None);
        assert_eq!(parse_osc11_background_color("\x1b]10;#ffffff\x07"), None);
        assert_eq!(parse_osc11_background_color("\x1b]11;#ffffff\x07x"), None);
    }
}

// =============================================================================
// parseTerminalColorSchemeReport
// =============================================================================

mod parse_terminal_color_scheme_report_tests {
    use super::*;

    #[test]
    fn test_parses_dark_scheme() {
        assert_eq!(
            parse_terminal_color_scheme_report("\x1b[?997;1n"),
            Some("dark".into())
        );
    }

    #[test]
    fn test_parses_light_scheme() {
        assert_eq!(
            parse_terminal_color_scheme_report("\x1b[?997;2n"),
            Some("light".into())
        );
    }

    #[test]
    fn test_rejects_unknown_scheme_values() {
        assert_eq!(parse_terminal_color_scheme_report("\x1b[?997;3n"), None);
    }

    #[test]
    fn test_rejects_non_color_scheme_reports() {
        assert_eq!(parse_terminal_color_scheme_report("\x1b[?996n"), None);
    }

    #[test]
    fn test_rejects_non_strict_patterns() {
        assert_eq!(parse_terminal_color_scheme_report("x\x1b[?997;1n"), None);
    }
}
