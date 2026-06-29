//! Port of `packages/coding-agent/src/hamr/shimmer.ts`.
//!
//! Rainbow-colored "● thinking" animation frames for the TUI shimmer.

/// Rainbow-colored "● thinking" animation frames, each using an ANSI color
/// sequence. Mirrors the TS `RAINBOW_WORD_FRAMES` constant exactly.
pub const RAINBOW_WORD_FRAMES: &[&str] = &[
    "\x1b[31m● thinking\x1b[0m",
    "\x1b[33m● thinking\x1b[0m",
    "\x1b[32m● thinking\x1b[0m",
    "\x1b[36m● thinking\x1b[0m",
    "\x1b[34m● thinking\x1b[0m",
    "\x1b[35m● thinking\x1b[0m",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rainbow_word_frames_count() {
        assert_eq!(RAINBOW_WORD_FRAMES.len(), 6);
    }

    #[test]
    fn test_rainbow_word_frames_contain_thinking() {
        for frame in RAINBOW_WORD_FRAMES {
            // Strip ANSI escape sequences for a simple check
            let stripped = strip_ansi(frame);
            assert!(
                stripped.contains("thinking"),
                "frame '{frame}' should contain 'thinking'"
            );
        }
    }

    #[test]
    fn test_rainbow_word_frames_have_color_codes() {
        // Each frame should have at least one \x1b[...m color code
        for frame in RAINBOW_WORD_FRAMES {
            assert!(
                frame.contains("\x1b["),
                "frame '{frame}' should contain color code"
            );
        }
    }

    fn strip_ansi(s: &str) -> String {
        let re = regex::Regex::new("\x1b\\[[0-9;]*m").unwrap();
        re.replace_all(s, "").to_string()
    }
}
