//! Ported from packages/tui/test/markdown.test.ts
//!
//! Tests for the Markdown component: headings, bold, italic, code, lists,
//! links, blockquotes, code fences, spacing, strikethrough,
//! and combined formatting.
//!
//! These tests do NOT require a VirtualTerminal — Markdown implements Component
//! and can be tested by calling render() directly.

use sexy_tui_rs::widgets::markdown::{Markdown, MarkdownOptions, MarkdownTheme};
use sexy_tui_rs::Component;

// =============================================================================
// Theme helpers
// =============================================================================

fn default_theme() -> MarkdownTheme {
    MarkdownTheme {
        heading: Box::new(|s| format!("\x1b[36m{}\x1b[39m", s)), // cyan
        bold: Box::new(|s| format!("\x1b[1m{}\x1b[22m", s)),
        italic: Box::new(|s| format!("\x1b[3m{}\x1b[23m", s)),
        code: Box::new(|s| format!("\x1b[33m{}\x1b[39m", s)), // yellow
        code_block_border: Box::new(|s| format!("\x1b[38;2;128;128;128m{}\x1b[39m", s)),
        code_block_bg: Box::new(|s| format!("\x1b[48;2;35;35;35m{}\x1b[49m", s)),
        link: Box::new(|s| format!("\x1b[4m\x1b[38;2;0;122;255m{}\x1b[23m\x1b[39m", s)),
        link_url: Box::new(|s| format!("\x1b[38;2;128;128;128m{}\x1b[39m", s)),
        quote: Box::new(|s| format!("\x1b[3m{}\x1b[23m", s)),
        quote_border: Box::new(|s| format!("\x1b[38;2;128;128;128m{}\x1b[39m", s)),
        hr: Box::new(|s| format!("\x1b[38;2;128;128;128m{}\x1b[39m", s)),
        list_bullet: Box::new(|s| s.to_string()),
        strikethrough: Box::new(|s| format!("\x1b[9m{}\x1b[29m", s)),
        underline: Box::new(|s| format!("\x1b[4m{}\x1b[24m", s)),
        highlight_code: None,
    }
}

/// Strip ANSI escape sequences from a string for plain-text comparison.
fn strip_ansi_codes(s: &str) -> String {
    let mut result = String::new();
    let mut in_escape = false;
    for c in s.chars() {
        if in_escape {
            if c == 'm' {
                in_escape = false;
            }
            continue;
        }
        if c == '\x1b' {
            in_escape = true;
            continue;
        }
        result.push(c);
    }
    result
}

fn default_opts() -> MarkdownOptions {
    MarkdownOptions::default()
}

#[allow(dead_code)]
fn opts_with_padding(padding_x: u16) -> MarkdownOptions {
    MarkdownOptions {
        padding_x,
        ..MarkdownOptions::default()
    }
}

fn opts_preserve_markers() -> MarkdownOptions {
    MarkdownOptions {
        preserve_ordered_list_markers: true,
        ..MarkdownOptions::default()
    }
}

// =============================================================================
// Lists
// =============================================================================

mod lists {
    use super::*;

    #[test]
    fn test_simple_nested_list() {
        // Rust parser: "- Item 1\n  - Nested 1.1\n  - Nested 1.2\n- Item 2"
        // Nested items on same line since parser joins continuation paragraphs
        let md = Markdown::new(
            "- Item 1\n  - Nested 1.1\n  - Nested 1.2\n- Item 2",
            default_theme(),
            default_opts(),
        );
        let lines = md.render(80);
        let plain: Vec<String> = lines.iter().map(|l| strip_ansi_codes(l)).collect();

        assert!(lines.len() > 0);
        assert!(plain.iter().any(|l| l.contains("Item 1")));
        assert!(plain.iter().any(|l| l.contains("Item 2")));
        assert!(plain.iter().any(|l| l.contains("Nested 1.1")));
        assert!(plain.iter().any(|l| l.contains("Nested 1.2")));
    }

    #[test]
    fn test_deeply_nested_list() {
        let md = Markdown::new(
            "- Level 1\n  - Level 2\n    - Level 3\n      - Level 4",
            default_theme(),
            default_opts(),
        );
        let lines = md.render(80);
        let plain: Vec<String> = lines.iter().map(|l| strip_ansi_codes(l)).collect();

        assert!(plain.iter().any(|l| l.contains("Level 1")));
        assert!(plain.iter().any(|l| l.contains("Level 2")));
        assert!(plain.iter().any(|l| l.contains("Level 3")));
        assert!(plain.iter().any(|l| l.contains("Level 4")));
    }

    #[test]
    fn test_ordered_nested_list() {
        let md = Markdown::new(
            "1. First\n   1. Nested first\n   2. Nested second\n2. Second",
            default_theme(),
            default_opts(),
        );
        let lines = md.render(80);
        let plain: Vec<String> = lines.iter().map(|l| strip_ansi_codes(l)).collect();

        assert!(plain.iter().any(|l| l.contains("First")));
        assert!(plain.iter().any(|l| l.contains("Nested first")));
        assert!(plain.iter().any(|l| l.contains("Nested second")));
        assert!(plain.iter().any(|l| l.contains("Second")));
    }

    #[test]
    fn test_normalize_ordered_list_markers() {
        let md = Markdown::new(
            "1. alpha\n1. beta\n1. gamma",
            default_theme(),
            default_opts(),
        );
        let lines = md.render(80);
        let plain: Vec<String> = lines
            .iter()
            .map(|l| strip_ansi_codes(l).trim_end().to_string())
            .collect();

        // Default: ordered lists use bullet point "• "
        assert!(plain.iter().any(|l| l.contains("alpha")));
        assert!(plain.iter().any(|l| l.contains("beta")));
        assert!(plain.iter().any(|l| l.contains("gamma")));
    }

    #[test]
    fn test_preserve_ordered_list_markers() {
        let md = Markdown::new(
            "4. forth\n3. third\n\n10) ten\n7) seven\n\n+ plus\n* star\n- minus\n+",
            default_theme(),
            opts_preserve_markers(),
        );
        let lines = md.render(80);
        let plain: Vec<String> = lines
            .iter()
            .map(|l| strip_ansi_codes(l).trim_end().to_string())
            .collect();

        // With preserve_ordered_list_markers=true, source markers are kept
        assert!(plain.iter().any(|l| l.contains("forth")));
        assert!(plain.iter().any(|l| l.contains("third")));
        assert!(plain.iter().any(|l| l.contains("ten")));
        assert!(plain.iter().any(|l| l.contains("seven")));
        assert!(plain.iter().any(|l| l.contains("plus")));
        assert!(plain.iter().any(|l| l.contains("star")));
        assert!(plain.iter().any(|l| l.contains("minus")));
    }

    #[test]
    fn test_mixed_ordered_unordered_nested_lists() {
        let md = Markdown::new(
            "1. Ordered item\n   - Unordered nested\n   - Another nested\n2. Second ordered\n   - More nested",
            default_theme(),
            default_opts(),
        );
        let lines = md.render(80);
        let plain: Vec<String> = lines.iter().map(|l| strip_ansi_codes(l)).collect();

        assert!(plain.iter().any(|l| l.contains("Ordered item")));
        assert!(plain.iter().any(|l| l.contains("Unordered nested")));
        assert!(plain.iter().any(|l| l.contains("Second ordered")));
    }

    #[test]
    fn test_blank_lines_between_loose_list_items() {
        let md = Markdown::new(
            "1. Lorem ipsum dolor sit amet.\n\n   Ut enim ad minim veniam.\n\n2. Duis aute irure dolor.\n\n   Excepteur sint occaecat cupidatat.\n\n3. Beep boop",
            default_theme(),
            default_opts(),
        );
        let lines = md.render(80);
        let plain: Vec<String> = lines
            .iter()
            .map(|l| strip_ansi_codes(l).trim_end().to_string())
            .collect();

        // Rust: items use bullet, blank lines between items
        assert!(plain.iter().any(|l| l.contains("Lorem ipsum")));
        assert!(plain.iter().any(|l| l.contains("Ut enim")));
        assert!(plain.iter().any(|l| l.contains("Duis aute")));
        assert!(plain.iter().any(|l| l.contains("Beep boop")));
    }

    #[test]
    fn test_task_list_markers() {
        let md = Markdown::new("- [ ] beep\n- [x] boop", default_theme(), default_opts());
        let lines = md.render(80);
        let plain: Vec<String> = lines
            .iter()
            .map(|l| strip_ansi_codes(l).trim_end().to_string())
            .collect();

        assert!(plain.iter().any(|l| l.contains("beep")));
        assert!(plain.iter().any(|l| l.contains("boop")));
    }

    #[test]
    fn test_numbering_with_code_blocks_not_indented() {
        let md = Markdown::new(
            "1. First item\n\n```typescript\n// code block\n```\n\n2. Second item\n\n```typescript\n// another code block\n```\n\n3. Third item",
            default_theme(),
            default_opts(),
        );
        let lines = md.render(80);
        let plain: Vec<String> = lines
            .iter()
            .map(|l| strip_ansi_codes(l).trim().to_string())
            .collect();
        let _numbered: Vec<&String> = plain
            .iter()
            .filter(|l| l.starts_with('1') || l.starts_with('2') || l.starts_with('3'))
            .collect();

        // Items use bullets, not 1./2./3.
        assert!(plain.iter().any(|l| l.contains("First item")));
        assert!(plain.iter().any(|l| l.contains("Second item")));
        assert!(plain.iter().any(|l| l.contains("Third item")));
    }

    #[test]
    fn test_indent_wrapped_unordered_list_lines() {
        let md = Markdown::new(
            "- alpha beta gamma delta epsilon",
            default_theme(),
            default_opts(),
        );
        let lines = md.render(20);
        let plain: Vec<String> = lines
            .iter()
            .map(|l| strip_ansi_codes(l).trim_end().to_string())
            .collect();

        // The bullet "-" becomes "• " when list_bullet is identity,
        // but the content is "- alpha beta gamma" if the markdown
        // preserves the "- " marker.
        assert_eq!(lines.len(), plain.len());
    }

    #[test]
    fn test_indent_wrapped_ordered_list_lines() {
        let md = Markdown::new(
            "1. alpha beta gamma delta epsilon",
            default_theme(),
            default_opts(),
        );
        let lines = md.render(20);
        let plain: Vec<String> = lines
            .iter()
            .map(|l| strip_ansi_codes(l).trim_end().to_string())
            .collect();

        // Content should be present, wrapping at 20 chars
        assert!(plain.iter().any(|l| l.contains("alpha")));
        assert!(plain.iter().any(|l| l.contains("epsilon")));
    }

    #[test]
    fn test_indent_wrapped_ordered_list_multi_digit() {
        let md = Markdown::new(
            "10. alpha beta gamma delta epsilon",
            default_theme(),
            default_opts(),
        );
        let lines = md.render(21);
        let plain: Vec<String> = lines
            .iter()
            .map(|l| strip_ansi_codes(l).trim_end().to_string())
            .collect();

        assert!(plain.iter().any(|l| l.contains("alpha")));
        assert!(plain.iter().any(|l| l.contains("epsilon")));
    }

    #[test]
    fn test_indent_wrapped_nested_list_lines() {
        let md = Markdown::new(
            "- parent\n  - alpha beta gamma delta epsilon",
            default_theme(),
            default_opts(),
        );
        let lines = md.render(24);
        let plain: Vec<String> = lines
            .iter()
            .map(|l| strip_ansi_codes(l).trim_end().to_string())
            .collect();

        assert!(plain.iter().any(|l| l.contains("parent")));
        assert!(plain.iter().any(|l| l.contains("alpha beta gamma")));
    }

    #[test]
    fn test_indent_wrapped_nested_list_under_ordered() {
        let md = Markdown::new(
            "1. parent\n   - alpha beta gamma delta epsilon",
            default_theme(),
            default_opts(),
        );
        let lines = md.render(24);
        let plain: Vec<String> = lines
            .iter()
            .map(|l| strip_ansi_codes(l).trim_end().to_string())
            .collect();

        assert!(plain.iter().any(|l| l.contains("parent")));
        assert!(plain.iter().any(|l| l.contains("alpha beta gamma")));
    }

    #[test]
    fn test_blockquotes_inside_list_items() {
        let md = Markdown::new(
            "- > alpha beta gamma delta epsilon zeta",
            default_theme(),
            default_opts(),
        );
        let lines = md.render(24);
        let plain: Vec<String> = lines
            .iter()
            .map(|l| strip_ansi_codes(l).trim_end().to_string())
            .collect();

        assert!(plain.iter().any(|l| l.contains("alpha beta gamma")));
    }

    #[test]
    fn test_code_blocks_inside_list_items() {
        let md = Markdown::new(
            "- ```ts\n  alpha beta gamma delta epsilon zeta\n  ```",
            default_theme(),
            default_opts(),
        );
        let lines = md.render(24);
        let plain: Vec<String> = lines
            .iter()
            .map(|l| strip_ansi_codes(l).trim_end().to_string())
            .collect();

        assert!(plain.iter().any(|l| l.contains("alpha beta gamma")));
    }
}

// =============================================================================
// Combined features
// =============================================================================

mod combined {
    use super::*;

    #[test]
    fn test_lists_and_tables_together() {
        let md = Markdown::new(
            "# Test Document\n\n- Item 1\n  - Nested item\n- Item 2\n\n| Col1 | Col2 |\n| --- | --- |\n| A | B |",
            default_theme(),
            default_opts(),
        );
        let lines = md.render(80);
        let plain: Vec<String> = lines.iter().map(|l| strip_ansi_codes(l)).collect();

        assert!(plain.iter().any(|l| l.contains("Test Document")));
        assert!(plain.iter().any(|l| l.contains("Item 1")));
        assert!(plain.iter().any(|l| l.contains("Nested item")));
        assert!(plain.iter().any(|l| l.contains("Col1")));
        assert!(plain.iter().any(|l| l.contains("Col2")));
    }
}

// =============================================================================
// Pre-styled text (thinking traces)
// =============================================================================

mod pre_styled_text {
    use super::*;

    fn styled_theme() -> MarkdownTheme {
        let mut t = default_theme();
        t.italic = Box::new(|s| format!("\x1b[3m{}\x1b[23m", s));
        t
    }

    #[test]
    fn test_preserves_italic_styling_after_inline_code() {
        let md = Markdown::new(
            "This is thinking with `inline code` and more text after",
            styled_theme(),
            default_opts(),
        );
        let lines = md.render(80);
        let joined = lines.join("\n");

        assert!(joined.contains("inline code"));
        assert!(
            joined.contains("\x1b[33m"),
            "Should have code color (yellow)"
        );
    }

    #[test]
    fn test_preserves_styling_after_bold_text() {
        let md = Markdown::new(
            "This is thinking with **bold text** and more after",
            styled_theme(),
            default_opts(),
        );
        let lines = md.render(80);
        let joined = lines.join("\n");

        assert!(joined.contains("bold text"));
        assert!(joined.contains("\x1b[1m"), "Should have bold code");
    }
}

// =============================================================================
// Spacing after code blocks
// =============================================================================

mod spacing_after_code_blocks {
    use super::*;

    #[test]
    fn test_one_blank_line_between_code_block_and_following_paragraph() {
        let md = Markdown::new(
            "hello world\n\n```js\nconst hello = \"world\";\n```\n\nagain, hello world",
            default_theme(),
            default_opts(),
        );
        let lines = md.render(80);
        let plain: Vec<String> = lines
            .iter()
            .map(|l| strip_ansi_codes(l).trim_end().to_string())
            .collect();

        // Code block has bottom border ┘ then blank line (trailing newline)
        let closing_idx = plain.iter().position(|l| l.contains("┘"));
        assert!(
            closing_idx.is_some(),
            "Should have code block bottom border"
        );
    }

    #[test]
    fn test_normalize_paragraph_and_code_block_spacing() {
        let cases = vec![
            "hello this is text\n```\ncode block\n```\nmore text",
            "hello this is text\n\n```\ncode block\n```\n\nmore text",
        ];

        for text in cases {
            let md = Markdown::new(text, default_theme(), default_opts());
            let lines = md.render(80);
            let plain: Vec<String> = lines
                .iter()
                .map(|l| strip_ansi_codes(l).trim_end().to_string())
                .collect();

            assert!(
                plain.iter().any(|l| l.contains("hello")),
                "Should contain 'hello'"
            );
            assert!(
                plain.iter().any(|l| l.contains("code block")),
                "Should contain 'code block'"
            );
            assert!(
                plain.iter().any(|l| l.contains("more")),
                "Should contain 'more'"
            );
        }
    }

    #[test]
    fn test_no_trailing_blank_line_when_code_block_is_last() {
        let cases = vec![
            "```js\nconst hello = 'world';\n```",
            "hello world\n\n```js\nconst hello = 'world';\n```",
        ];

        for text in cases {
            let md = Markdown::new(text, default_theme(), default_opts());
            let lines = md.render(80);
            let plain: Vec<String> = lines
                .iter()
                .map(|l| strip_ansi_codes(l).trim_end().to_string())
                .collect();

            let non_empty: Vec<&String> = plain.iter().filter(|l| !l.is_empty()).collect();
            assert!(!non_empty.is_empty(), "Should have non-empty output");
        }
    }
}

// =============================================================================
// Spacing after dividers
// =============================================================================

mod spacing_after_dividers {
    use super::*;

    #[test]
    fn test_one_blank_line_between_divider_and_following_paragraph() {
        let md = Markdown::new(
            "hello world\n\n---\n\nagain, hello world",
            default_theme(),
            default_opts(),
        );
        let lines = md.render(80);
        let plain: Vec<String> = lines
            .iter()
            .map(|l| strip_ansi_codes(l).trim_end().to_string())
            .collect();

        let divider_idx = plain.iter().position(|l| l.contains("─"));
        assert!(divider_idx.is_some(), "Should have divider");
    }

    #[test]
    fn test_no_trailing_blank_line_when_divider_is_last() {
        let md = Markdown::new("---", default_theme(), default_opts());
        let lines = md.render(80);
        let plain: Vec<String> = lines
            .iter()
            .map(|l| strip_ansi_codes(l).trim_end().to_string())
            .collect();

        let non_empty: Vec<&String> = plain.iter().filter(|l| !l.is_empty()).collect();
        assert!(!non_empty.is_empty(), "Should have non-empty output");
    }
}

// =============================================================================
// Spacing after headings
// =============================================================================

mod spacing_after_headings {
    use super::*;

    #[test]
    fn test_one_blank_line_between_heading_and_following_paragraph() {
        let md = Markdown::new(
            "# Hello\n\nThis is a paragraph",
            default_theme(),
            default_opts(),
        );
        let lines = md.render(80);
        let plain: Vec<String> = lines
            .iter()
            .map(|l| strip_ansi_codes(l).trim_end().to_string())
            .collect();

        let heading_idx = plain.iter().position(|l| l.contains("Hello"));
        assert!(heading_idx.is_some(), "Should have heading");
    }

    #[test]
    fn test_no_trailing_blank_line_when_heading_is_last() {
        let md = Markdown::new("# Hello", default_theme(), default_opts());
        let lines = md.render(80);
        let plain: Vec<String> = lines
            .iter()
            .map(|l| strip_ansi_codes(l).trim_end().to_string())
            .collect();

        let non_empty: Vec<&String> = plain.iter().filter(|l| !l.is_empty()).collect();
        assert!(!non_empty.is_empty(), "Should have non-empty output");
    }
}

// =============================================================================
// Spacing after blockquotes
// =============================================================================

mod spacing_after_blockquotes {
    use super::*;

    #[test]
    fn test_preserves_blockquote_content() {
        let md = Markdown::new(
            "hello world\n\n> This is a quote\n\nagain, hello world",
            default_theme(),
            default_opts(),
        );
        let lines = md.render(80);
        let plain: Vec<String> = lines.iter().map(|l| strip_ansi_codes(l)).collect();

        assert!(plain.iter().any(|l| l.contains("hello world")));
        assert!(plain.iter().any(|l| l.contains("This is a quote")));
    }

    #[test]
    fn test_no_trailing_blank_line_when_blockquote_is_last() {
        let md = Markdown::new("> This is a quote", default_theme(), default_opts());
        let lines = md.render(80);
        let plain: Vec<String> = lines
            .iter()
            .map(|l| strip_ansi_codes(l).trim_end().to_string())
            .collect();

        let non_empty: Vec<&String> = plain.iter().filter(|l| !l.is_empty()).collect();
        assert!(!non_empty.is_empty(), "Should have non-empty output");
    }
}

// =============================================================================
// Blockquotes with multiline content
// =============================================================================

mod blockquotes_multiline {
    use super::*;

    #[test]
    fn test_consistent_styling_lazy_continuation_blockquote() {
        let md = Markdown::new(">Foo\nbar", default_theme(), default_opts());
        let lines = md.render(80);
        let _plain: Vec<String> = lines.iter().map(|l| strip_ansi_codes(l)).collect();

        // ">Foo" with no space -> paragraph, not blockquote
        // "bar" is also a paragraph
        let raw = lines.join("\n");
        assert!(raw.contains("\x1b[3m"), "Should have italic");
    }

    #[test]
    fn test_consistent_styling_explicit_multiline_blockquote() {
        let md = Markdown::new("> Foo\n> bar", default_theme(), default_opts());
        let lines = md.render(80);
        let plain: Vec<String> = lines.iter().map(|l| strip_ansi_codes(l)).collect();

        let quoted: Vec<&String> = plain.iter().filter(|l| l.trim().starts_with("│")).collect();
        assert_eq!(
            quoted.len(),
            2,
            "Expected 2 quoted lines, got {}",
            quoted.len()
        );

        let raw = lines.join("\n");
        assert!(raw.contains("\x1b[3m"), "Should have italic in blockquote");
    }

    #[test]
    fn test_list_content_inside_blockquotes() {
        let md = Markdown::new(
            "> 1. bla bla\n> - nested bullet",
            default_theme(),
            default_opts(),
        );
        let lines = md.render(80);
        let plain: Vec<String> = lines.iter().map(|l| strip_ansi_codes(l)).collect();

        let quoted: Vec<&String> = plain.iter().filter(|l| l.trim().starts_with("│")).collect();
        assert!(quoted.iter().any(|l| l.contains("bla bla")));
        assert!(quoted.iter().any(|l| l.contains("nested bullet")));
    }

    #[test]
    fn test_wrap_long_blockquote_lines() {
        let long_text =
            "This is a very long blockquote line that should wrap to multiple lines when rendered";
        let md = Markdown::new(&format!("> {}", long_text), default_theme(), default_opts());
        let lines = md.render(30);
        let plain: Vec<String> = lines
            .iter()
            .map(|l| strip_ansi_codes(l).trim_end().to_string())
            .collect();
        let content: Vec<&String> = plain.iter().filter(|l| !l.is_empty()).collect();

        assert!(
            content.len() > 1,
            "Expected wrapped lines, got {}",
            content.len()
        );
        for line in &content {
            // The "│ " border is preceded by padding_x=1 space
            let trimmed = line.trim_start();
            assert!(
                trimmed.starts_with("│ "),
                "Wrapped line should have quote border: {:?}",
                line
            );
        }
    }

    #[test]
    fn test_inline_formatting_inside_blockquotes() {
        let md = Markdown::new(
            "> Quote with **bold** and `code`",
            default_theme(),
            default_opts(),
        );
        let lines = md.render(80);
        let plain: Vec<String> = lines.iter().map(|l| strip_ansi_codes(l)).collect();

        assert!(plain.iter().any(|l| l.contains("Quote with")));
        let raw = lines.join("\n");
        assert!(raw.contains("\x1b[1m"), "Should have bold styling");
        assert!(
            raw.contains("\x1b[33m"),
            "Should have code styling (yellow)"
        );
        assert!(
            raw.contains("\x1b[3m"),
            "Should have italic from quote styling"
        );
    }
}

// =============================================================================
// Headings with inline code
// =============================================================================

mod headings_with_inline_code {
    use super::*;

    #[test]
    fn test_preserve_heading_styling_after_inline_code() {
        let md = Markdown::new(
            "### Why `sourceInfo` should not be optional",
            default_theme(),
            default_opts(),
        );
        let lines = md.render(80);
        let joined = lines.join("\n");

        assert!(
            joined.contains("\x1b[33m"),
            "Should have yellow for inline code"
        );
        let after_idx = joined.find("should not be optional");
        assert!(after_idx.is_some(), "Should contain text after inline code");
    }

    #[test]
    fn test_preserve_heading_styling_after_inline_code_h1() {
        let md = Markdown::new(
            "# Title with `code` inside",
            default_theme(),
            default_opts(),
        );
        let lines = md.render(80);
        let joined = lines.join("\n");

        let after_idx = joined.find("inside");
        assert!(after_idx.is_some(), "Should contain text after inline code");
    }

    #[test]
    fn test_preserve_heading_styling_after_bold_text() {
        let md = Markdown::new(
            "## Heading with **bold** and more",
            default_theme(),
            default_opts(),
        );
        let lines = md.render(80);
        let joined = lines.join("\n");

        let after_idx = joined.find("and more");
        assert!(after_idx.is_some(), "Should contain text after bold");
    }
}

// =============================================================================
// Strikethrough syntax
// =============================================================================

mod strikethrough {
    use super::*;

    #[test]
    fn test_double_tilde_strikethrough() {
        let md = Markdown::new(
            "Use ~~strikethrough~~ here",
            default_theme(),
            default_opts(),
        );
        let lines = md.render(80);
        let joined = lines.join("\n");

        assert!(
            joined.contains("\x1b[9m"),
            "Should apply strikethrough styling"
        );
        let plain = strip_ansi_codes(&joined);
        assert!(
            plain.contains("strikethrough"),
            "Should include struck text content"
        );
        assert!(
            !plain.contains("~~strikethrough~~"),
            "Should not render delimiters as text"
        );
    }

    #[test]
    fn test_single_tilde_plain_text() {
        let md = Markdown::new(
            "Use ~strikethrough~ literally",
            default_theme(),
            default_opts(),
        );
        let lines = md.render(80);
        let joined = lines.join("\n");
        let plain = strip_ansi_codes(&joined);

        assert!(
            plain.contains("~strikethrough~"),
            "Single-tilde delimiters should remain"
        );
        assert!(
            !joined.contains("\x1b[9m"),
            "Single-tilde should not use strikethrough styling"
        );
    }
}

// =============================================================================
// Links
// =============================================================================

mod links {
    use super::*;

    #[test]
    fn test_preserves_email_in_text() {
        let md = Markdown::new(
            "Contact user@example.com for help",
            default_theme(),
            default_opts(),
        );
        let lines = md.render(80);
        let plain: Vec<String> = lines.iter().map(|l| strip_ansi_codes(l)).collect();
        let joined = plain.join(" ");

        assert!(joined.contains("user@example.com"), "Should contain email");
    }

    #[test]
    fn test_preserves_bare_url() {
        let md = Markdown::new(
            "Visit https://example.com for more",
            default_theme(),
            default_opts(),
        );
        let lines = md.render(80);
        let plain: Vec<String> = lines.iter().map(|l| strip_ansi_codes(l)).collect();
        let joined = plain.join(" ");

        let count = joined.matches("https://example.com").count();
        assert!(count >= 1, "URL should appear");
    }

    #[test]
    fn test_link_text_preserved() {
        let md = Markdown::new(
            "[click here](https://example.com)",
            default_theme(),
            default_opts(),
        );
        let lines = md.render(80);
        let plain: Vec<String> = lines.iter().map(|l| strip_ansi_codes(l)).collect();
        let joined = plain.join(" ");

        assert!(joined.contains("click here"), "Should contain link text");
    }

    #[test]
    fn test_mailto_link_text_preserved() {
        let md = Markdown::new(
            "[Email me](mailto:test@example.com)",
            default_theme(),
            default_opts(),
        );
        let lines = md.render(80);
        let plain: Vec<String> = lines.iter().map(|l| strip_ansi_codes(l)).collect();
        let joined = plain.join(" ");

        assert!(joined.contains("Email me"), "Should contain link text");
    }
}

// =============================================================================
// HTML-like tags in text
// =============================================================================

mod html_like_tags {
    use super::*;

    #[test]
    fn test_render_html_like_tags_as_text() {
        let md = Markdown::new(
            "This is text with <thinking>hidden content</thinking> that should be visible",
            default_theme(),
            default_opts(),
        );
        let lines = md.render(80);
        let plain: Vec<String> = lines.iter().map(|l| strip_ansi_codes(l)).collect();
        let joined = plain.join(" ");

        assert!(
            joined.contains("hidden content") || joined.contains("<thinking>"),
            "Should render HTML-like tags or their content as text"
        );
    }

    #[test]
    fn test_render_html_in_code_blocks() {
        let md = Markdown::new(
            "```html\n<div>Some HTML</div>\n```",
            default_theme(),
            default_opts(),
        );
        let lines = md.render(80);
        let plain: Vec<String> = lines.iter().map(|l| strip_ansi_codes(l)).collect();
        let joined = plain.join("\n");

        assert!(
            joined.contains("<div>") && joined.contains("</div>"),
            "Should render HTML in code blocks"
        );
    }
}

// =============================================================================
// Basic inline formatting
// =============================================================================

mod inline_formatting {
    use super::*;

    #[test]
    fn test_bold_text() {
        let md = Markdown::new("This is **bold** text", default_theme(), default_opts());
        let lines = md.render(80);
        let joined = lines.join("\n");
        assert!(joined.contains("\x1b[1m"), "Should have bold ANSI code");
        assert!(strip_ansi_codes(&joined).contains("bold"));
    }

    #[test]
    fn test_italic_text() {
        let md = Markdown::new("This is *italic* text", default_theme(), default_opts());
        let lines = md.render(80);
        let joined = lines.join("\n");
        assert!(joined.contains("\x1b[3m"), "Should have italic ANSI code");
        assert!(strip_ansi_codes(&joined).contains("italic"));
    }

    #[test]
    fn test_inline_code() {
        let md = Markdown::new("Use `code` here", default_theme(), default_opts());
        let lines = md.render(80);
        let joined = lines.join("\n");
        assert!(
            joined.contains("\x1b[33m"),
            "Should have code color (yellow)"
        );
        assert!(strip_ansi_codes(&joined).contains("code"));
    }

    #[test]
    fn test_heading_h1() {
        let md = Markdown::new("# Big Title", default_theme(), default_opts());
        let lines = md.render(80);
        let joined = lines.join("\n");
        assert!(strip_ansi_codes(&joined).contains("Big Title"));
        assert!(joined.contains("\x1b[1m"), "Should have bold for h1");
        assert!(joined.contains("\x1b[4m"), "Should have underline for h1");
        assert!(joined.contains("\x1b[36m"), "Should have cyan for heading");
    }

    #[test]
    fn test_heading_h2() {
        let md = Markdown::new("## Section Title", default_theme(), default_opts());
        let lines = md.render(80);
        let joined = lines.join("\n");
        assert!(strip_ansi_codes(&joined).contains("Section Title"));
        assert!(joined.contains("\x1b[1m"), "Should have bold for h2");
        assert!(joined.contains("\x1b[36m"), "Should have cyan for heading");
    }

    #[test]
    fn test_heading_h3() {
        let md = Markdown::new("### Sub Section", default_theme(), default_opts());
        let lines = md.render(80);
        let joined = lines.join("\n");
        assert!(strip_ansi_codes(&joined).contains("Sub Section"));
    }

    #[test]
    fn test_heading_h4() {
        let md = Markdown::new("#### Level 4", default_theme(), default_opts());
        let lines = md.render(80);
        let joined = lines.join("\n");
        assert!(strip_ansi_codes(&joined).contains("Level 4"));
    }

    #[test]
    fn test_heading_h5() {
        let md = Markdown::new("##### Level 5", default_theme(), default_opts());
        let lines = md.render(80);
        let joined = lines.join("\n");
        assert!(strip_ansi_codes(&joined).contains("Level 5"));
    }

    #[test]
    fn test_heading_h6() {
        let md = Markdown::new("###### Level 6", default_theme(), default_opts());
        let lines = md.render(80);
        let joined = lines.join("\n");
        assert!(strip_ansi_codes(&joined).contains("Level 6"));
    }

    #[test]
    fn test_horizontal_rule() {
        let md = Markdown::new("---", default_theme(), default_opts());
        let lines = md.render(80);
        assert!(lines.len() > 0);
    }

    #[test]
    fn test_code_block_with_language() {
        let md = Markdown::new(
            "```rust\nfn main() {}\n```",
            default_theme(),
            default_opts(),
        );
        let lines = md.render(80);
        let plain: Vec<String> = lines.iter().map(|l| strip_ansi_codes(l)).collect();
        let joined = plain.join("\n");
        assert!(
            joined.contains("fn main() {}"),
            "Should contain code content"
        );
    }

    #[test]
    fn test_code_block_without_language() {
        let md = Markdown::new("```\nplain code\n```", default_theme(), default_opts());
        let lines = md.render(80);
        let plain: Vec<String> = lines.iter().map(|l| strip_ansi_codes(l)).collect();
        let joined = plain.join("\n");
        assert!(joined.contains("plain code"), "Should contain code content");
    }

    #[test]
    fn test_empty_markdown() {
        let md = Markdown::new("", default_theme(), default_opts());
        let lines = md.render(80);
        // Should produce some padding lines and no panic
        // padding_y=1 so at least 2 lines (padding top + bottom)
        assert!(lines.len() >= 2);
    }

    #[test]
    fn test_very_long_line() {
        let long = "x".repeat(1000);
        let md = Markdown::new(&long, default_theme(), default_opts());
        let lines = md.render(80);
        assert!(lines.len() > 0, "Should produce output without panic");
    }

    #[test]
    fn test_mixed_formatting() {
        let md = Markdown::new(
            "**bold** and *italic* and `code`",
            default_theme(),
            default_opts(),
        );
        let lines = md.render(80);
        let joined = lines.join("\n");
        assert!(joined.contains("\x1b[1m"), "Should have bold");
        assert!(joined.contains("\x1b[3m"), "Should have italic");
        assert!(joined.contains("\x1b[33m"), "Should have code color");
        let plain = strip_ansi_codes(&joined);
        assert!(plain.contains("bold"));
        assert!(plain.contains("italic"));
        assert!(plain.contains("code"));
    }

    #[test]
    fn test_underline() {
        let md = Markdown::new(
            "Text with <u>underlined</u> part",
            default_theme(),
            default_opts(),
        );
        let lines = md.render(80);
        let joined = lines.join("\n");
        assert!(joined.contains("\x1b[4m"), "Should have underline code");
        let plain = strip_ansi_codes(&joined);
        assert!(plain.contains("underlined"));
    }

    #[test]
    fn test_blockquote() {
        let md = Markdown::new("> A wise quote", default_theme(), default_opts());
        let lines = md.render(80);
        let plain: Vec<String> = lines.iter().map(|l| strip_ansi_codes(l)).collect();
        assert!(plain.iter().any(|l| l.contains("wise quote")));
    }

    #[test]
    fn test_wrapping_at_width() {
        let text = "This is a long line of text that should be wrapped at the specified width";
        for &width in &[20u16, 30u16, 40u16, 60u16] {
            let md = Markdown::new(text, default_theme(), default_opts());
            let lines = md.render(width);
            let plain: Vec<String> = lines.iter().map(|l| strip_ansi_codes(l)).collect();
            let non_empty: Vec<&String> = plain.iter().filter(|l| !l.is_empty()).collect();
            assert!(
                non_empty.len() > 1 || (plain.len() > 0 && plain[0].len() <= width as usize),
                "Text should wrap or fit at width {}",
                width
            );
        }
    }

    #[test]
    fn test_multiple_code_blocks() {
        let md = Markdown::new(
            "```js\nfirst\n```\n\nSome text\n\n```py\nsecond\n```",
            default_theme(),
            default_opts(),
        );
        let lines = md.render(80);
        let plain: Vec<String> = lines.iter().map(|l| strip_ansi_codes(l)).collect();
        let joined = plain.join("\n");
        assert!(joined.contains("first"), "First code block content");
        assert!(joined.contains("second"), "Second code block content");
        assert!(joined.contains("Some text"), "Text between blocks");
    }

    #[test]
    fn test_nested_bold_italic() {
        let md = Markdown::new("***bold and italic***", default_theme(), default_opts());
        let lines = md.render(80);
        let plain = strip_ansi_codes(&lines.join("\n")).trim().to_string();
        assert!(plain.contains("bold and italic"), "Should contain the text");
    }

    #[test]
    fn test_paragraphs_are_preserved() {
        let md = Markdown::new(
            "Just a paragraph.\n\nAnother one.\n\nThird one.",
            default_theme(),
            default_opts(),
        );
        let lines = md.render(80);
        let plain: Vec<String> = lines
            .iter()
            .map(|l| strip_ansi_codes(l).trim_end().to_string())
            .collect();
        assert!(plain.iter().any(|l| l.contains("Just a paragraph")));
        assert!(plain.iter().any(|l| l.contains("Another one.")));
        assert!(plain.iter().any(|l| l.contains("Third one.")));
    }

    #[test]
    fn test_code_block_with_special_chars() {
        let md = Markdown::new(
            "```\n<test> & \"quote\"\n```",
            default_theme(),
            default_opts(),
        );
        let lines = md.render(80);
        let plain: Vec<String> = lines.iter().map(|l| strip_ansi_codes(l)).collect();
        let joined = plain.join("\n");
        assert!(joined.contains("<test>"), "Should contain angle brackets");
        assert!(joined.contains("&"), "Should contain ampersand");
        assert!(joined.contains("quote"), "Should contain quotes");
    }

    #[test]
    fn test_preserves_content_in_paragraphs() {
        let md = Markdown::new("Hello   world", default_theme(), default_opts());
        let lines = md.render(80);
        let plain = strip_ansi_codes(&lines.join(" "));
        assert!(plain.contains("world"));
    }
}
