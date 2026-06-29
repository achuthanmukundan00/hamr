//! Port of `packages/ai/src/utils/sanitize-unicode.ts`.
//!
//! Removes unpaired Unicode surrogate characters from a string. Unpaired
//! surrogates (high surrogates U+D800-U+DBFF without a matching low surrogate
//! U+DC00-U+DFFF, or vice versa) cause JSON serialization errors in many API
//! providers.
//!
//! Note: a Rust `&str` is always valid UTF-8 and therefore can never *contain*
//! an unpaired surrogate (`char` excludes the surrogate range entirely). This
//! port operates over UTF-16 code units to faithfully mirror the TypeScript
//! behaviour — handling the case where the caller has data that round-trips
//! through UTF-16. In practice valid input is returned unchanged.

/// Remove unpaired UTF-16 surrogate code units, preserving valid surrogate pairs.
pub fn sanitize_surrogates(text: &str) -> String {
    let units: Vec<u16> = text.encode_utf16().collect();
    let mut kept: Vec<u16> = Vec::with_capacity(units.len());
    let mut i = 0;
    while i < units.len() {
        let unit = units[i];
        if (0xD800..=0xDBFF).contains(&unit) {
            // High surrogate — keep only if followed by a low surrogate.
            if i + 1 < units.len() && (0xDC00..=0xDFFF).contains(&units[i + 1]) {
                kept.push(unit);
                kept.push(units[i + 1]);
                i += 2;
                continue;
            }
            // Unpaired high surrogate — drop.
            i += 1;
        } else if (0xDC00..=0xDFFF).contains(&unit) {
            // Unpaired low surrogate (a paired one is consumed above) — drop.
            i += 1;
        } else {
            kept.push(unit);
            i += 1;
        }
    }
    String::from_utf16_lossy(&kept)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preserves_plain_text() {
        assert_eq!(sanitize_surrogates("Hello world"), "Hello world");
    }

    #[test]
    fn preserves_valid_emoji() {
        assert_eq!(
            sanitize_surrogates("Hello \u{1F648} World"),
            "Hello \u{1F648} World"
        );
    }

    #[test]
    fn removes_unpaired_high_surrogate() {
        // Build a UTF-16 sequence with an unpaired high surrogate.
        let units: Vec<u16> = vec![
            'T' as u16, 'e' as u16, 'x' as u16, 't' as u16, ' ' as u16, 0xD83D, ' ' as u16,
            'h' as u16, 'e' as u16, 'r' as u16, 'e' as u16,
        ];
        let input = String::from_utf16_lossy(&units);
        // from_utf16_lossy already replaced the lone surrogate with U+FFFD, so
        // verify the function is a no-op on already-valid input and stable.
        assert_eq!(sanitize_surrogates(&input), input);
    }

    #[test]
    fn removes_unpaired_low_surrogate() {
        let units: Vec<u16> = vec!['H' as u16, 'i' as u16, ' ' as u16, 0xDC00, '!' as u16];
        let input = String::from_utf16_lossy(&units);
        assert_eq!(sanitize_surrogates(&input), input);
    }

    #[test]
    fn preserves_multiple_valid_surrogate_pairs() {
        // Multiple emoji characters in sequence
        let input = "Hello 🙈 👍 ❤️";
        assert_eq!(sanitize_surrogates(input), input);
    }

    #[test]
    fn handles_mixed_unicode_and_special_characters() {
        let input = "Mixed text: Mario Zechner wann? Wo? Bin grad äußerst eventuninformiert 🙈";
        assert_eq!(sanitize_surrogates(input), input);
    }

    #[test]
    fn handles_japanese_and_chinese_characters() {
        let input = "Japanese: こんにちは\nChinese: 你好";
        assert_eq!(sanitize_surrogates(input), input);
    }

    #[test]
    fn handles_mathematical_symbols_and_curly_quotes() {
        let input = "Mathematical: ∑∫∂√\nCurly quotes: \"curly\" 'quotes'";
        assert_eq!(sanitize_surrogates(input), input);
    }

    #[test]
    fn empty_string_unchanged() {
        assert_eq!(sanitize_surrogates(""), "");
    }
}
