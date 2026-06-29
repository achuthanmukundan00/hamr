//! Port of `packages/coding-agent/src/utils/ansi.ts`.
//!
//! Strip ANSI escape sequences from strings.
//!
//! Portions derived from:
//! - ansi-regex (https://github.com/chalk/ansi-regex)
//! - strip-ansi (https://github.com/chalk/strip-ansi)
//! MIT License © Sindre Sorhus

use regex::Regex;

/// Build the ANSI escape sequence regex once (lazy static).
fn ansi_regex() -> &'static Regex {
    static RE: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
    RE.get_or_init(|| {
        // ST (string terminator): BEL, ESC\, or 0x9C
        // OSC: ESC ] ... ST (non-greedy)
        // CSI and related: ESC/C1, optional intermediates, optional params,
        //                  then final byte
        let pattern = concat!(
            "(?:\\x1B\\][\\s\\S]*?(?:\\x07|\\x1B\\\\|\\x9C))",
            "|",
            "[\\x1B\\x9B][\\[\\]()#;?]*(?:\\d{1,4}(?:[;:]\\d{0,4})*)?[\\dA-PR-TZcf-nq-uy=><~]",
        );
        Regex::new(pattern).expect("invalid ANSI regex pattern")
    })
}

/// Strip ANSI escape sequences from `value`.
///
/// Fast path: if the string contains no ESC (`\x1B`) or CSI (`\x9B`) bytes,
/// returns the input unchanged.
pub fn strip_ansi(value: &str) -> String {
    if !value.contains('\u{1b}') && !value.contains('\u{9b}') {
        return value.to_string();
    }
    ansi_regex().replace_all(value, "").into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_simple() {
        assert_eq!(strip_ansi("\x1B[31mhello\x1B[0m"), "hello");
    }

    #[test]
    fn test_strip_no_ansi() {
        assert_eq!(strip_ansi("hello world"), "hello world");
    }

    #[test]
    fn test_strip_osc() {
        // OSC sequence: ESC ] 0 ; title \x07
        let input = "\x1B]0;My Title\x07content";
        assert_eq!(strip_ansi(input), "content");
    }

    #[test]
    fn test_strip_csi_8bit() {
        assert_eq!(strip_ansi("\u{9b}32mgreen\u{1b}[0m"), "green");
    }

    #[test]
    fn test_empty() {
        assert_eq!(strip_ansi(""), "");
    }

    #[test]
    fn test_strip_hyperlink_osc() {
        let input = "\x1b]8;;https://example.com\x07link\x1b]8;;\x07";
        assert_eq!(strip_ansi(input), "link");
    }

    #[test]
    fn test_strip_ris() {
        assert_eq!(strip_ansi("\x1bcdone"), "done");
    }

    #[test]
    fn test_strip_combined_sequences() {
        let input = "a\x1b[31mred\x1b[0m\x1b]8;;https://example.com\x07link\x1b]8;;\x07z";
        assert_eq!(strip_ansi(input), "aredlinkz");
    }

    #[test]
    fn test_strip_single_byte_esc() {
        for code in 'g'..='m' {
            let input = format!("\x1b{}ok", code);
            assert_eq!(strip_ansi(&input), "ok");
        }
        for code in 'r'..='t' {
            let input = format!("\x1b{}ok", code);
            assert_eq!(strip_ansi(&input), "ok");
        }
    }

    #[test]
    fn test_no_esc_or_csi_fast_path() {
        // Must return the same string reference (fast path)
        let s = "hello world";
        assert_eq!(strip_ansi(s), s);
    }
}
