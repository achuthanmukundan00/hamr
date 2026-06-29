//! Port of `packages/coding-agent/src/core/tools/render-utils.ts`.
//!
//! Rendering utilities for tool output display.

/// Shorten a path by replacing the home directory with `~`.
pub fn shorten_path(path: &str) -> String {
    if path.is_empty() {
        return String::new();
    }
    // Try to get home directory
    if let Ok(home) = std::env::var("HOME") {
        if path.starts_with(&home) && path.len() > home.len() {
            return format!("~{}", &path[home.len()..]);
        }
    }
    path.to_string()
}

/// Extract text content from tool result blocks, stripping ANSI and sanitizing binary output.
pub fn get_text_output(content: &[hamr_ai::types::MessageContent]) -> String {
    let mut output = String::new();
    for block in content {
        if let hamr_ai::types::MessageContent::Text(tc) = block {
            if !output.is_empty() {
                output.push('\n');
            }
            // Strip ANSI escape sequences and replace \r
            let text = strip_ansi(&tc.text);
            output.push_str(&sanitize_binary_output(&text.replace('\r', "")));
        }
    }
    output
}

/// Strip ANSI escape sequences from a string.
fn strip_ansi(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '\x1b' && i + 1 < chars.len() && chars[i + 1] == '[' {
            // Skip until we find a command character
            i += 2;
            while i < chars.len() {
                let c = chars[i];
                i += 1;
                if matches!(c, 'A'..='Z' | 'a'..='z') {
                    break;
                }
            }
        } else {
            result.push(chars[i]);
            i += 1;
        }
    }
    result
}

/// Sanitize binary output by replacing non-printable characters (except common whitespace).
fn sanitize_binary_output(text: &str) -> String {
    text.chars()
        .map(|c| {
            if c.is_ascii_control() && c != '\n' && c != '\t' && c != '\r' {
                '�'
            } else {
                c
            }
        })
        .collect()
}

/// Replace tabs with 3 spaces for display.
pub fn replace_tabs(text: &str) -> String {
    text.replace('\t', "   ")
}

/// Normalize display text by removing carriage returns.
pub fn normalize_display_text(text: &str) -> String {
    text.replace('\r', "")
}

/// Convert an unknown value to a string if possible, or empty string if null.
pub fn str_value(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Null => String::new(),
        _ => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hamr_ai::types::{MessageContent, TextContent};

    #[test]
    fn test_shorten_path_home() {
        unsafe {
            std::env::set_var("HOME", "/home/user");
        }
        assert_eq!(shorten_path("/home/user/projects/foo"), "~/projects/foo");
        assert_eq!(shorten_path("/home/user"), "/home/user");
        assert_eq!(shorten_path("/other/path"), "/other/path");
        assert_eq!(shorten_path(""), "");
    }

    #[test]
    fn test_strip_ansi() {
        let text = "\x1b[31mred text\x1b[0m normal";
        let result = strip_ansi(text);
        assert_eq!(result, "red text normal");
    }

    #[test]
    fn test_sanitize_binary_output() {
        let text = "hello\x00world\x01";
        let result = sanitize_binary_output(text);
        assert!(!result.contains('\x00'));
        assert!(!result.contains('\x01'));
        assert!(result.contains("hello"));
        assert!(result.contains("world"));
    }

    #[test]
    fn test_replace_tabs() {
        assert_eq!(replace_tabs("a\tb"), "a   b");
        assert_eq!(replace_tabs("\t\t"), "      ");
    }

    #[test]
    fn test_normalize_display_text() {
        assert_eq!(normalize_display_text("hello\r\nworld\r"), "hello\nworld");
    }

    #[test]
    fn test_get_text_output() {
        let content = vec![
            MessageContent::Text(TextContent {
                text: "hello".to_string(),
                text_signature: None,
            }),
            MessageContent::Text(TextContent {
                text: "world".to_string(),
                text_signature: None,
            }),
        ];
        assert_eq!(get_text_output(&content), "hello\nworld");
    }
}
