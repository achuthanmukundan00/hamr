//! Port of `packages/coding-agent/src/utils/frontmatter.ts`.
//!
//! Parse YAML frontmatter from Markdown content.

use serde_yaml;

/// Result of parsing frontmatter from a document.
#[derive(Debug, Clone)]
pub struct ParsedFrontmatter<T: serde::de::DeserializeOwned = serde_json::Value> {
    pub frontmatter: T,
    pub body: String,
}

fn normalize_newlines(value: &str) -> String {
    value.replace("\r\n", "\n").replace('\r', "\n")
}

fn extract_frontmatter(content: &str) -> (Option<String>, String) {
    let normalized = normalize_newlines(content);

    if !normalized.starts_with("---") {
        return (None, normalized);
    }

    // TS: normalized.indexOf("\n---", 3)
    let end_index = normalized[3..].find("\n---").map(|i| i + 3);

    match end_index {
        Some(end) => {
            // TS: normalized.slice(4, endIndex) — yaml content after "---\n"
            let yaml_str = &normalized[4..end];
            // TS: normalized.slice(endIndex + 4).trim()
            let body = normalized[end + 4..].trim().to_string();
            let yaml = if yaml_str.is_empty() {
                None
            } else {
                Some(yaml_str.to_string())
            };
            (yaml, body)
        }
        None => (None, normalized),
    }
}

/// Parse YAML frontmatter from a string of content.
///
/// Returns the parsed frontmatter (empty object if none found) and the body.
pub fn parse_frontmatter<T: serde::de::DeserializeOwned + Default>(
    content: &str,
) -> ParsedFrontmatter<T> {
    let (yaml_string, body) = extract_frontmatter(content);
    let frontmatter = match yaml_string {
        Some(yaml) => serde_yaml::from_str(&yaml).unwrap_or_default(),
        None => serde_json::from_str("{}").unwrap_or_default(),
    };
    ParsedFrontmatter { frontmatter, body }
}

/// Strip frontmatter from content, returning just the body.
pub fn strip_frontmatter(content: &str) -> String {
    parse_frontmatter::<serde_json::Value>(content).body
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_frontmatter() {
        let result = parse_frontmatter::<serde_json::Value>("Hello world");
        assert!(result.frontmatter.is_object());
        assert_eq!(result.body, "Hello world");
    }

    #[test]
    fn test_with_frontmatter() {
        let content = "---\ntitle: Test\nversion: 1\n---\n\nBody text";
        let result = parse_frontmatter::<serde_json::Value>(content);
        assert_eq!(result.frontmatter["title"], "Test");
        assert_eq!(result.frontmatter["version"], 1);
        assert!(result.body.contains("Body text"));
    }

    #[test]
    fn test_strip_frontmatter() {
        let content = "---\ntitle: Test\n---\n\nBody";
        assert_eq!(strip_frontmatter(content), "Body");
    }

    #[test]
    fn test_no_frontmatter_dashes() {
        let content = "Some text with --- in it\nand more";
        let result = parse_frontmatter::<serde_json::Value>(content);
        assert!(result.frontmatter.is_object());
        assert!(result.body.contains("---"));
    }

    #[test]
    fn test_windows_newlines() {
        let content = "---\r\ntitle: Test\r\n---\r\n\r\nBody";
        let result = parse_frontmatter::<serde_json::Value>(content);
        assert_eq!(result.frontmatter["title"], "Test");
    }

    #[test]
    fn test_multiline_yaml() {
        let content = "---\ndescription: |\n  Line one\n  Line two\n---\n\nBody";
        let result = parse_frontmatter::<serde_json::Value>(content);
        assert_eq!(result.frontmatter["description"], "Line one\nLine two");
        assert_eq!(result.body, "Body");
    }

    #[test]
    fn test_empty_frontmatter() {
        let content = "---\n# just a comment\n---\nBody";
        let result = parse_frontmatter::<serde_json::Value>(content);
        // YAML with only a comment parses as null, not an object
        assert!(result.frontmatter.is_null());
        assert_eq!(result.body, "Body");
    }

    #[test]
    fn test_unterminated_frontmatter_returns_full_content() {
        let content = "---\nname: test\nBody without terminator";
        let result = parse_frontmatter::<serde_json::Value>(content);
        assert!(result.frontmatter.is_object());
        assert!(result.body.contains("---"));
        assert!(result.body.contains("Body without terminator"));
    }
}
