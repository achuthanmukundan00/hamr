//! Port of `packages/coding-agent/src/utils/json.ts`.
//!
//! Strip `//` line comments and trailing commas from JSON, leaving string
//! literals untouched.

/// Strip `//` line comments and trailing commas from JSON-like text, leaving
/// string literals untouched.
///
/// The function processes the input in two passes:
/// 1. Remove `//` line comments (outside string literals).
/// 2. Remove trailing commas before `]`, `}`, `)]`, etc.
pub fn strip_json_comments(input: &str) -> String {
    // Pass 1: strip // line comments outside string literals.
    // We process character by character to track string literal boundaries.
    let mut result = String::with_capacity(input.len());
    let mut chars = input.char_indices().peekable();
    let mut in_string = false;

    while let Some((i, ch)) = chars.next() {
        if ch == '"' {
            // Count consecutive preceding backslashes.
            // Even count = quote is unescaped, odd = escaped.
            let mut backslash_count = 0;
            let mut j = i;
            while j > 0 && input.as_bytes()[j - 1] == b'\\' {
                backslash_count += 1;
                j -= 1;
            }
            if backslash_count % 2 == 0 {
                in_string = !in_string;
            }
            result.push(ch);
        } else if !in_string && ch == '/' {
            // Look ahead for '//'
            if let Some(&(_, '/')) = chars.peek() {
                // Skip the second '/'
                chars.next();
                // Skip until newline or EOF
                for (_, c) in chars.by_ref() {
                    if c == '\n' {
                        break;
                    }
                }
            } else {
                result.push(ch);
            }
        } else {
            result.push(ch);
        }
    }

    // Pass 2: strip trailing commas before closing brackets/braces/parens.
    // Again track string literals accurately.
    let mut output = String::with_capacity(result.len());
    let mut in_str = false;
    let result_bytes = result.as_bytes();
    let len = result.len();
    let mut i = 0;

    while i < len {
        let ch = result_bytes[i] as char;
        if ch == '"' {
            let mut backslash_count = 0;
            let mut j = i;
            while j > 0 && result_bytes[j - 1] == b'\\' {
                backslash_count += 1;
                j -= 1;
            }
            if backslash_count % 2 == 0 {
                in_str = !in_str;
            }
            output.push(ch);
            i += 1;
        } else if !in_str && ch == ',' {
            // Look ahead past whitespace for a closing bracket/brace/paren.
            let mut j = i + 1;
            while j < len && (result_bytes[j] as char).is_ascii_whitespace() {
                j += 1;
            }
            if j < len && matches!(result_bytes[j], b']' | b'}' | b')') {
                // Skip the comma entirely.
                i += 1;
                continue;
            }
            output.push(ch);
            i += 1;
        } else {
            output.push(ch);
            i += 1;
        }
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_line_comments() {
        let input = r#"{
            // this is a comment
            "key": "value // not a comment"
        }"#;
        let result = strip_json_comments(input);
        // Line comment "// this is a comment" should be stripped,
        // but "//" inside a string literal must be preserved.
        assert!(!result.contains("// this is a comment"));
        assert!(result.contains(r#""value // not a comment""#));
    }

    #[test]
    fn test_strip_trailing_commas() {
        let input = r#"{
            "a": 1,
            "b": 2,
        }"#;
        let result = strip_json_comments(input);
        assert!(!result.contains(",\n}"));
    }

    #[test]
    fn test_empty_object() {
        assert_eq!(strip_json_comments("{}"), "{}");
    }

    #[test]
    fn test_nested() {
        let input = r#"{
            "arr": [1, 2, 3,],
            "obj": {"x": 1,},
        }"#;
        let result = strip_json_comments(input);
        assert!(!result.contains(",]"));
        assert!(!result.contains(",}"));
    }
}
