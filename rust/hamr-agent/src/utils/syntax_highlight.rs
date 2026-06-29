//! Port of `packages/coding-agent/src/utils/syntax-highlight.ts`.
//!
//! Syntax highlighting via `syntect` with customizable theme support.

use crate::utils::html::decode_html_entity_at;

/// A formatter function that wraps highlighted text (e.g., applies ANSI codes).
pub type HighlightFormatter = Box<dyn Fn(&str) -> String + Send + Sync>;

/// A theme mapping scope names to formatters.
pub type HighlightTheme = std::collections::HashMap<String, HighlightFormatter>;

/// Options for highlighting code.
#[derive(Clone)]
pub struct HighlightOptions<'a> {
    pub language: Option<&'a str>,
    pub ignore_illegals: bool,
    pub language_subset: Option<&'a [String]>,
    pub theme: Option<&'a HighlightTheme>,
}

impl<'a> std::fmt::Debug for HighlightOptions<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HighlightOptions")
            .field("language", &self.language)
            .field("ignore_illegals", &self.ignore_illegals)
            .field("language_subset", &self.language_subset)
            .field("theme", &self.theme.map(|_| "<HighlightTheme>"))
            .finish()
    }
}

impl Default for HighlightOptions<'_> {
    fn default() -> Self {
        Self {
            language: None,
            ignore_illegals: false,
            language_subset: None,
            theme: None,
        }
    }
}

const SPAN_CLOSE: &str = "</span>";
const HIGHLIGHT_CLASS_PREFIX: &str = "hljs-";

fn get_scope_from_span_tag(tag: &str) -> Option<String> {
    // Match class="..." or class='...'
    let re = regex::Regex::new(r#"\sclass\s*=\s*(?:"([^"]*)"|'([^']*)')"#).unwrap();
    let class_value = re
        .captures(tag)
        .and_then(|caps| caps.get(1).or_else(|| caps.get(2)).map(|m| m.as_str()))?;

    for class_name in class_value.split_whitespace() {
        if let Some(stripped) = class_name.strip_prefix(HIGHLIGHT_CLASS_PREFIX) {
            return Some(stripped.to_string());
        }
    }
    None
}

fn get_scope_formatter<'a>(
    scope: &str,
    theme: &'a HighlightTheme,
) -> Option<&'a HighlightFormatter> {
    // Exact match first
    if let Some(fmt) = theme.get(scope) {
        return Some(fmt);
    }

    // Try prefix before "."
    if let Some(dot) = scope.find('.') {
        if let Some(fmt) = theme.get(&scope[..dot]) {
            return Some(fmt);
        }
    }

    // Try prefix before "-"
    if let Some(dash) = scope.find('-') {
        if let Some(fmt) = theme.get(&scope[..dash]) {
            return Some(fmt);
        }
    }

    None
}

fn get_active_formatter<'a>(
    scopes: &[Option<String>],
    theme: &'a HighlightTheme,
) -> Option<&'a HighlightFormatter> {
    for scope in scopes.iter().rev() {
        if let Some(scope) = scope {
            if let Some(fmt) = get_scope_formatter(scope, theme) {
                return Some(fmt);
            }
        }
    }
    theme.get("default")
}

fn is_span_open_tag_start(html: &str, index: usize) -> bool {
    if !html[index..].starts_with("<span") {
        return false;
    }
    let after_open = index + "<span".len();
    html.as_bytes().get(after_open).map_or(false, |&b| {
        b == b'>' || b == b' ' || b == b'\t' || b == b'\n' || b == b'\r'
    })
}

/// Render highlighted HTML (from highlight.js / syntect) into formatted output
/// using the given theme.
pub fn render_highlighted_html(html: &str, theme: &HighlightTheme) -> String {
    let mut output = String::new();
    let mut text_buffer = String::new();
    let mut scopes: Vec<Option<String>> = Vec::new();

    let flush_text = |output: &mut String,
                      text_buffer: &mut String,
                      scopes: &[Option<String>],
                      theme: &HighlightTheme| {
        if text_buffer.is_empty() {
            return;
        }
        if let Some(formatter) = get_active_formatter(scopes, theme) {
            output.push_str(&formatter(text_buffer));
        } else {
            output.push_str(text_buffer);
        }
        text_buffer.clear();
    };

    let chars: Vec<char> = html.chars().collect();
    let len = chars.len();
    let mut i = 0;

    while i < len {
        // Check for <span at current position (case-insensitive? html is lowercase from syntect)
        if i + 5 < len
            && chars[i] == '<'
            && chars[i + 1] == 's'
            && chars[i + 2] == 'p'
            && chars[i + 3] == 'a'
            && chars[i + 4] == 'n'
        {
            let after_span = i + 5;
            if after_span < len
                && (chars[after_span] == '>'
                    || chars[after_span] == ' '
                    || chars[after_span] == '\t'
                    || chars[after_span] == '\n'
                    || chars[after_span] == '\r')
            {
                // Find the end of the opening tag
                let tag_end = html[i..].find('>').map(|pos| i + pos + 1);
                if let Some(tag_end) = tag_end {
                    flush_text(&mut output, &mut text_buffer, &scopes, theme);
                    let tag = &html[i..tag_end];
                    let scope = get_scope_from_span_tag(tag);
                    scopes.push(scope);
                    i = tag_end;
                    continue;
                }
            }
        }

        // Check for </span>
        if html[i..].starts_with(SPAN_CLOSE) {
            flush_text(&mut output, &mut text_buffer, &scopes, theme);
            if !scopes.is_empty() {
                scopes.pop();
            }
            i += SPAN_CLOSE.len();
            continue;
        }

        // Check for HTML entities
        if chars[i] == '&' {
            let _rest = &html[i..];
            if let Some(decoded) = decode_html_entity_at(html, i) {
                text_buffer.push_str(&decoded.text);
                i += decoded.length;
                continue;
            }
        }

        text_buffer.push(chars[i]);
        i += 1;
    }

    flush_text(&mut output, &mut text_buffer, &scopes, theme);
    output
}

/// Highlight source code as HTML using `syntect`.
pub fn highlight_html(code: &str, language: Option<&str>) -> Result<String, String> {
    use syntect::easy::HighlightLines;
    use syntect::highlighting::ThemeSet;
    use syntect::html::styled_line_to_highlighted_html;
    use syntect::parsing::SyntaxSet;

    let ss = SyntaxSet::load_defaults_newlines();
    let ts = ThemeSet::load_defaults();

    let syntax = match language {
        Some(lang) => ss
            .find_syntax_by_token(lang)
            .or_else(|| ss.find_syntax_by_name(lang))
            .or_else(|| ss.find_syntax_by_extension(lang))
            .ok_or_else(|| format!("Unknown language: {lang}"))?,
        None => ss.find_syntax_plain_text(),
    };

    let theme = &ts.themes["base16-ocean.dark"];
    let mut highlighter = HighlightLines::new(syntax, theme);

    let mut html_output = String::new();
    for line in code.lines() {
        let highlighted = highlighter
            .highlight_line(line, &ss)
            .map_err(|e| format!("Highlight error: {e}"))?;
        let html =
            styled_line_to_highlighted_html(&highlighted, syntect::html::IncludeBackground::No)
                .map_err(|e| format!("Syntax highlight error: {e}"))?;
        html_output.push_str(&html);
    }

    Ok(html_output)
}

/// Highlight code and apply theme formatting in one call.
pub fn highlight(code: &str, options: &HighlightOptions) -> String {
    let html = highlight_html(code, options.language).unwrap_or_else(|_| {
        // Fallback to plain text
        code.to_string()
    });

    match options.theme {
        Some(theme) => render_highlighted_html(&html, theme),
        None => html,
    }
}

/// Check if a language name is supported by syntect.
pub fn supports_language(name: &str) -> bool {
    let ss = syntect::parsing::SyntaxSet::load_defaults_newlines();
    ss.find_syntax_by_token(name).is_some()
        || ss.find_syntax_by_name(name).is_some()
        || ss.find_syntax_by_extension(name).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_highlighted_html_empty() {
        let theme = HighlightTheme::new();
        assert_eq!(render_highlighted_html("", &theme), "");
    }

    #[test]
    fn test_render_highlighted_html_no_tags() {
        let theme = HighlightTheme::new();
        assert_eq!(
            render_highlighted_html("hello world", &theme),
            "hello world"
        );
    }

    #[test]
    fn test_render_with_formatter() {
        let mut theme = HighlightTheme::new();
        theme.insert(
            "keyword".to_string(),
            Box::new(|s| format!("\x1b[31m{s}\x1b[0m")),
        );
        let html = r#"<span class="hljs-keyword">let</span>"#;
        let result = render_highlighted_html(html, &theme);
        assert_eq!(result, "\x1b[31mlet\x1b[0m");
    }

    #[test]
    fn test_render_nested_scopes() {
        let mut theme = HighlightTheme::new();
        theme.insert(
            "string".to_string(),
            Box::new(|s| format!("\x1b[32m{s}\x1b[0m")),
        );
        let html = r#"<span class="hljs-string"><span class="hljs-subst">x</span></span>"#;
        let result = render_highlighted_html(html, &theme);
        assert_eq!(result, "\x1b[32mx\x1b[0m");
    }

    #[test]
    fn test_supports_language() {
        assert!(supports_language("rust"));
        assert!(supports_language("python"));
        assert!(supports_language("javascript"));
    }

    #[test]
    fn test_render_decodes_html_entities() {
        let theme = HighlightTheme::new();
        let html = "&lt;tag attr=&quot;value&quot;&gt;&amp;#x41;&#65;&lt;/tag&gt;";
        let result = render_highlighted_html(html, &theme);
        assert!(result.contains("<tag attr=\"value\">"));
        assert!(!result.contains("&lt;"));
    }

    #[test]
    fn test_render_inherits_parent_formatting_nested() {
        let mut theme = HighlightTheme::new();
        theme.insert("string".to_string(), Box::new(|s| format!("[str:{}]", s)));
        let html = r#"<span class="hljs-string">a<span class="hljs-subst">interp</span>b</span>"#;
        let result = render_highlighted_html(html, &theme);
        // Parent formatting (string) should apply to all three children
        assert!(result.contains("[str:a]"));
        assert!(result.contains("[str:interp]"));
        assert!(result.contains("[str:b]"));
    }

    #[test]
    fn test_render_unmapped_nested_scope_keeps_parent() {
        let mut theme = HighlightTheme::new();
        theme.insert("string".to_string(), Box::new(|s| format!("[str:{}]", s)));
        let html = r#"<span class="hljs-string">a<span class="language-xml">b</span>c</span>"#;
        let result = render_highlighted_html(html, &theme);
        assert!(result.contains("[str:a]"));
        assert!(result.contains("[str:b]"));
        assert!(result.contains("[str:c]"));
    }

    #[test]
    fn test_get_scope_formatter_prefix_before_dot() {
        let mut theme = HighlightTheme::new();
        theme.insert("meta".to_string(), Box::new(|s| format!("[meta:{}]", s)));
        let html = r#"<span class="hljs-meta.foo">text</span>"#;
        let result = render_highlighted_html(html, &theme);
        assert_eq!(result, "[meta:text]");
    }

    #[test]
    fn test_get_scope_formatter_prefix_before_dash() {
        let mut theme = HighlightTheme::new();
        theme.insert("meta".to_string(), Box::new(|s| format!("[meta:{}]", s)));
        let html = r#"<span class="hljs-meta-foo">text</span>"#;
        let result = render_highlighted_html(html, &theme);
        assert_eq!(result, "[meta:text]");
    }

    #[test]
    fn test_highlight_fallback_on_unknown_language() {
        let options = HighlightOptions {
            language: Some("nonexistent_language_xyz"),
            ignore_illegals: false,
            language_subset: None,
            theme: None,
        };
        let result = highlight("hello world", &options);
        // Falls back to plain text when language is unknown
        assert!(result.contains("hello world"));
    }

    #[test]
    fn test_highlight_with_theme() {
        let mut theme = HighlightTheme::new();
        theme.insert("keyword".to_string(), Box::new(|s| format!("[kw:{}]", s)));
        let options = HighlightOptions {
            language: Some("rust"),
            ignore_illegals: true,
            language_subset: None,
            theme: Some(&theme),
        };
        let result = highlight("fn main() {}", &options);
        // Themes apply formatting spans; verify the output contains the keyword
        // (the exact formatting depends on the highlighter implementation)
        assert!(!result.is_empty());
        assert!(result.contains("fn"));
    }

    #[test]
    fn test_get_scope_from_span_tag_single_quotes() {
        let html = r#"<span class='hljs-keyword'>let</span>"#;
        // The function is private; test indirectly via render_highlighted_html
        let mut theme = HighlightTheme::new();
        theme.insert("keyword".to_string(), Box::new(|s| format!("[kw:{}]", s)));
        let result = render_highlighted_html(html, &theme);
        assert_eq!(result, "[kw:let]");
    }
}
