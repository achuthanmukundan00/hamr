//! FTS5 mark helpers.
//!
//! Mirrors `packages/coding-agent/src/hamr/memory/fts-marks.ts`.

/// Strip `<mark>` and `</mark>` tags from an FTS5 snippet.
pub fn strip_fts_marks(s: &str) -> String {
    s.replace("</mark>", "").replace("<mark>", "")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_fts_marks() {
        assert_eq!(strip_fts_marks("hello <mark>world</mark>"), "hello world");
        assert_eq!(strip_fts_marks("no marks here"), "no marks here");
        assert_eq!(strip_fts_marks(""), "");
    }

    #[test]
    fn test_strip_fts_marks_multiple() {
        assert_eq!(
            strip_fts_marks("<mark>foo</mark> bar <mark>baz</mark>"),
            "foo bar baz"
        );
    }
}
