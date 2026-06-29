use crate::tui::Component;

/// Markdown rendering theme — style functions for each element type.
pub struct MarkdownTheme {
    pub heading: Box<dyn Fn(&str) -> String>,
    pub bold: Box<dyn Fn(&str) -> String>,
    pub italic: Box<dyn Fn(&str) -> String>,
    pub code: Box<dyn Fn(&str) -> String>,
    pub code_block_border: Box<dyn Fn(&str) -> String>,
    pub code_block_bg: Box<dyn Fn(&str) -> String>,
    pub link: Box<dyn Fn(&str) -> String>,
    pub link_url: Box<dyn Fn(&str) -> String>,
    pub quote: Box<dyn Fn(&str) -> String>,
    pub quote_border: Box<dyn Fn(&str) -> String>,
    pub hr: Box<dyn Fn(&str) -> String>,
    pub list_bullet: Box<dyn Fn(&str) -> String>,
    pub strikethrough: Box<dyn Fn(&str) -> String>,
    pub underline: Box<dyn Fn(&str) -> String>,
    pub highlight_code: Option<Box<dyn Fn(&str, Option<&str>) -> Vec<String>>>,
}

/// Markdown render options.
pub struct MarkdownOptions {
    pub padding_x: u16,
    pub padding_y: u16,
    pub strip_headings: bool,
    /// If true, preserve source list markers (+/- for unordered, original numbers for ordered).
    pub preserve_ordered_list_markers: bool,
}

impl Default for MarkdownOptions {
    fn default() -> Self {
        MarkdownOptions {
            padding_x: 1,
            padding_y: 1,
            strip_headings: true,
            preserve_ordered_list_markers: false,
        }
    }
}

// ── Inline token types ──────────────────────────────────────────────────────

#[derive(Debug, Clone)]
enum InlineToken {
    Text(String),
    Bold(Vec<InlineToken>),
    Italic(Vec<InlineToken>),
    Code(String),
    Link { text: Vec<InlineToken>, url: String },
    Strikethrough(Vec<InlineToken>),
    Underline(Vec<InlineToken>),
}

// ── Block-level token types ──────────────────────────────────────────────────

#[derive(Debug)]
enum BlockToken {
    Heading {
        level: u8,
        text: Vec<InlineToken>,
    },
    Paragraph(Vec<InlineToken>),
    CodeBlock {
        lang: Option<String>,
        code: String,
    },
    Blockquote(Vec<BlockToken>),
    Hr,
    ListItem {
        ordered: bool,
        checked: Option<bool>,
        tokens: Vec<InlineToken>,
        source_marker: Option<String>,
    },
    Blank,
}

/// Markdown renderer widget with rich styling and syntax highlighting.
pub struct Markdown {
    content: String,
    options: MarkdownOptions,
    theme: MarkdownTheme,
    cached_width: u16,
    cached_lines: Vec<String>,
}

impl Markdown {
    pub fn new(content: &str, theme: MarkdownTheme, options: MarkdownOptions) -> Self {
        Markdown {
            content: content.to_string(),
            options,
            theme,
            cached_width: 0,
            cached_lines: Vec::new(),
        }
    }

    pub fn set_text(&mut self, text: &str) {
        self.content = text.to_string();
        self.cached_width = 0;
    }
}

// ── Inline parsing ───────────────────────────────────────────────────────────

/// Parse inline tokens from a string.
/// Walks character-by-character, matching **bold**, *italic*, `code`, ~~strike~~, [links](url), <u>underline</u>.
fn parse_inline(text: &str) -> Vec<InlineToken> {
    let mut tokens: Vec<InlineToken> = Vec::new();
    let chars: Vec<char> = text.chars().collect();
    let len = chars.len();
    let char_indices: Vec<(usize, char)> = text.char_indices().collect();

    let mut i = 0;

    while i < len {
        let c = chars[i];
        let byte_pos = char_indices[i].0;

        // Inline code: `code`
        if c == '`' {
            if let Some(end_byte) = text[byte_pos + 1..].find('`') {
                let code = &text[byte_pos + 1..byte_pos + 1 + end_byte];
                tokens.push(InlineToken::Code(code.to_string()));
                let end_char = char_indices
                    .iter()
                    .position(|(bp, _)| *bp == byte_pos + 1 + end_byte);
                if let Some(eci) = end_char {
                    i = eci + 1;
                } else {
                    i += 1 + code.chars().count() + 1;
                }
                continue;
            } else {
                tokens.push(InlineToken::Text("`".to_string()));
                i += 1;
                continue;
            }
        }

        // Strikethrough: ~~text~~
        if c == '~' && i + 1 < len && chars[i + 1] == '~' {
            if let Some(end_byte) = text[byte_pos + 2..].find("~~") {
                let inner = &text[byte_pos + 2..byte_pos + 2 + end_byte];
                tokens.push(InlineToken::Strikethrough(parse_inline(inner)));
                let end_char = char_indices
                    .iter()
                    .position(|(bp, _)| *bp == byte_pos + 2 + end_byte + 2);
                if let Some(eci) = end_char {
                    i = eci;
                } else {
                    i += 2 + inner.chars().count() + 2;
                }
                continue;
            }
        }

        // Bold: **text**
        if c == '*' && i + 1 < len && chars[i + 1] == '*' {
            if let Some(end_byte) = text[byte_pos + 2..].find("**") {
                // skip empty bold
                if end_byte > 0 {
                    let inner = &text[byte_pos + 2..byte_pos + 2 + end_byte];
                    tokens.push(InlineToken::Bold(parse_inline(inner)));
                    let end_char = char_indices
                        .iter()
                        .position(|(bp, _)| *bp == byte_pos + 2 + end_byte + 2);
                    if let Some(eci) = end_char {
                        i = eci;
                    } else {
                        i += 2 + inner.chars().count() + 2;
                    }
                    continue;
                }
            }
        }

        // Italic: *text*
        if c == '*' && (i + 1 >= len || chars[i + 1] != '*') {
            if let Some(end_byte) = text[byte_pos + 1..].find('*') {
                if end_byte > 0 {
                    let inner = &text[byte_pos + 1..byte_pos + 1 + end_byte];
                    tokens.push(InlineToken::Italic(parse_inline(inner)));
                    let end_char = char_indices
                        .iter()
                        .position(|(bp, _)| *bp == byte_pos + 1 + end_byte + 1);
                    if let Some(eci) = end_char {
                        i = eci;
                    } else {
                        i += 1 + inner.chars().count() + 1;
                    }
                    continue;
                }
            }
        }

        // Italic: _text_
        if c == '_' {
            if let Some(end_byte) = text[byte_pos + 1..].find('_') {
                if end_byte > 0 {
                    let inner = &text[byte_pos + 1..byte_pos + 1 + end_byte];
                    tokens.push(InlineToken::Italic(parse_inline(inner)));
                    let end_char = char_indices
                        .iter()
                        .position(|(bp, _)| *bp == byte_pos + 1 + end_byte + 1);
                    if let Some(eci) = end_char {
                        i = eci;
                    } else {
                        i += 1 + inner.chars().count() + 1;
                    }
                    continue;
                }
            }
        }

        // Links: [text](url)
        if c == '[' {
            if let Some(bracket_end_byte) = text[byte_pos + 1..].find("](") {
                let link_text = &text[byte_pos + 1..byte_pos + 1 + bracket_end_byte];
                let start_after = byte_pos + 1 + bracket_end_byte + 2;
                if let Some(paren_end_byte) = text[start_after..].find(')') {
                    let url = &text[start_after..start_after + paren_end_byte];
                    tokens.push(InlineToken::Link {
                        text: parse_inline(link_text),
                        url: url.to_string(),
                    });
                    let end_char = char_indices
                        .iter()
                        .position(|(bp, _)| *bp == start_after + paren_end_byte + 1);
                    if let Some(eci) = end_char {
                        i = eci;
                    } else {
                        i += 1 + bracket_end_byte + 2 + paren_end_byte + 1;
                    }
                    continue;
                }
            }
        }

        // Underline: <u>text</u>
        if c == '<' {
            if text[byte_pos..].starts_with("<u>") {
                let inner_start = byte_pos + 3;
                if let Some(end_byte) = text[inner_start..].find("</u>") {
                    let inner = &text[inner_start..inner_start + end_byte];
                    tokens.push(InlineToken::Underline(parse_inline(inner)));
                    let end_char = char_indices
                        .iter()
                        .position(|(bp, _)| *bp == inner_start + end_byte + 4);
                    if let Some(eci) = end_char {
                        i = eci;
                    } else {
                        i += 3 + inner.chars().count() + 4;
                    }
                    continue;
                }
            }
        }

        // Plain text character
        tokens.push(InlineToken::Text(c.to_string()));
        i += 1;
    }

    // Merge adjacent text tokens
    merge_adjacent_text(&mut tokens);
    tokens
}

fn merge_adjacent_text(tokens: &mut Vec<InlineToken>) {
    let mut merged: Vec<InlineToken> = Vec::new();
    let mut current_text = String::new();

    for t in tokens.drain(..) {
        match t {
            InlineToken::Text(s) => current_text.push_str(&s),
            other => {
                if !current_text.is_empty() {
                    merged.push(InlineToken::Text(std::mem::take(&mut current_text)));
                }
                merged.push(other);
            }
        }
    }
    if !current_text.is_empty() {
        merged.push(InlineToken::Text(current_text));
    }
    *tokens = merged;
}

// ── Block-level parsing ──────────────────────────────────────────────────────

fn parse_blocks(text: &str) -> Vec<BlockToken> {
    let lines: Vec<&str> = text.lines().collect();
    let mut tokens: Vec<BlockToken> = Vec::new();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i];

        // Blank line
        if line.trim().is_empty() {
            tokens.push(BlockToken::Blank);
            i += 1;
            continue;
        }

        // Heading: # Text
        if let Some(heading_level) = heading_level(line) {
            let content = line[heading_level as usize..].trim().to_string();
            tokens.push(BlockToken::Heading {
                level: heading_level,
                text: parse_inline(&content),
            });
            i += 1;
            continue;
        }

        // Horizontal rule: ---, ***, ___
        if is_hr(line) {
            tokens.push(BlockToken::Hr);
            i += 1;
            continue;
        }

        // Fenced code block: ```lang ... ```
        if line.trim().starts_with("```") {
            let lang = line.trim().strip_prefix("```").and_then(|s| {
                let s = s.trim();
                if s.is_empty() {
                    None
                } else {
                    Some(s.to_string())
                }
            });
            let mut code_lines: Vec<&str> = Vec::new();
            i += 1;
            while i < lines.len() && !lines[i].starts_with("```") {
                code_lines.push(lines[i]);
                i += 1;
            }
            i += 1; // skip closing ```
            tokens.push(BlockToken::CodeBlock {
                lang,
                code: code_lines.join("\n"),
            });
            continue;
        }

        // Blockquote: > text
        if line.starts_with("> ") || line.starts_with('>') {
            let quote_text = line.trim_start_matches('>').trim().to_string();
            tokens.push(BlockToken::Blockquote(parse_blocks(&quote_text)));
            i += 1;
            continue;
        }

        // Unordered list: - item, * item, or + item
        if let Some(rest) = line
            .strip_prefix("- ")
            .or_else(|| line.strip_prefix("* "))
            .or_else(|| line.strip_prefix("+ "))
        {
            let marker = if line.starts_with("- ") {
                "- "
            } else if line.starts_with("+ ") {
                "+ "
            } else {
                "* "
            };
            tokens.push(BlockToken::ListItem {
                ordered: false,
                checked: None,
                tokens: parse_inline(rest),
                source_marker: Some(marker.to_string()),
            });
            i += 1;
            continue;
        }

        // Ordered list: 1. item
        if let Some((rest, marker)) = ordered_list_prefix_with_marker(line) {
            tokens.push(BlockToken::ListItem {
                ordered: true,
                checked: None,
                tokens: parse_inline(&rest),
                source_marker: Some(marker),
            });
            i += 1;
            continue;
        }

        // Task list: - [x] item or - [ ] item
        if let Some(rest) = line
            .strip_prefix("- [x] ")
            .or_else(|| line.strip_prefix("- [X] "))
        {
            tokens.push(BlockToken::ListItem {
                ordered: false,
                checked: Some(true),
                tokens: parse_inline(rest),
                source_marker: None,
            });
            i += 1;
            continue;
        }
        if let Some(rest) = line.strip_prefix("- [ ] ") {
            tokens.push(BlockToken::ListItem {
                ordered: false,
                checked: Some(false),
                tokens: parse_inline(rest),
                source_marker: None,
            });
            i += 1;
            continue;
        }

        // Paragraph
        let mut para_lines: Vec<&str> = vec![line];
        i += 1;
        while i < lines.len() && !lines[i].trim().is_empty() && !is_block_start(lines[i]) {
            para_lines.push(lines[i]);
            i += 1;
        }
        let para_text = para_lines.join(" ");
        tokens.push(BlockToken::Paragraph(parse_inline(&para_text)));
    }

    tokens
}

fn heading_level(line: &str) -> Option<u8> {
    let trimmed = line.trim();
    if trimmed.starts_with("###### ") {
        return Some(6);
    }
    if trimmed.starts_with("##### ") {
        return Some(5);
    }
    if trimmed.starts_with("#### ") {
        return Some(4);
    }
    if trimmed.starts_with("### ") {
        return Some(3);
    }
    if trimmed.starts_with("## ") {
        return Some(2);
    }
    if trimmed.starts_with("# ") {
        return Some(1);
    }
    None
}

fn is_hr(line: &str) -> bool {
    let trimmed = line.trim();
    (trimmed.len() >= 3 && trimmed.chars().all(|c| c == '-' || c == ' '))
        || (trimmed.len() >= 3 && trimmed.chars().all(|c| c == '*' || c == ' '))
        || (trimmed.len() >= 3 && trimmed.chars().all(|c| c == '_' || c == ' '))
}

fn is_block_start(line: &str) -> bool {
    heading_level(line).is_some()
        || line.trim().starts_with("```")
        || is_hr(line)
        || line.starts_with("> ")
        || line.starts_with("- ")
        || line.starts_with("* ")
        || ordered_list_prefix(line).is_some()
}

fn ordered_list_prefix(line: &str) -> Option<String> {
    ordered_list_prefix_with_marker(line).map(|(rest, _)| rest)
}

fn ordered_list_prefix_with_marker(line: &str) -> Option<(String, String)> {
    let trimmed = line.trim();
    let dot_pos = trimmed.find(". ")?;
    let num_part = &trimmed[..dot_pos];
    if num_part.chars().all(|c| c.is_ascii_digit()) {
        let marker = format!("{}. ", num_part);
        Some((trimmed[dot_pos + 2..].to_string(), marker))
    } else {
        None
    }
}

// ── Rendering ────────────────────────────────────────────────────────────────

impl Markdown {
    fn render_inline(&self, tokens: &[InlineToken]) -> String {
        let mut result = String::new();
        for token in tokens {
            match token {
                InlineToken::Text(s) => result.push_str(s),
                InlineToken::Bold(inner) => {
                    let inner_text = self.render_inline(inner);
                    result.push_str(&(self.theme.bold)(&inner_text));
                }
                InlineToken::Italic(inner) => {
                    let inner_text = self.render_inline(inner);
                    result.push_str(&(self.theme.italic)(&inner_text));
                }
                InlineToken::Code(s) => {
                    result.push_str(&(self.theme.code)(s));
                }
                InlineToken::Link { text, url } => {
                    let link_text = self.render_inline(text);
                    let styled = format!(
                        "{}{}",
                        (self.theme.link)(&link_text),
                        (self.theme.link_url)(&format!(" ({})", url))
                    );
                    result.push_str(&styled);
                }
                InlineToken::Strikethrough(inner) => {
                    let inner_text = self.render_inline(inner);
                    result.push_str(&(self.theme.strikethrough)(&inner_text));
                }
                InlineToken::Underline(inner) => {
                    let inner_text = self.render_inline(inner);
                    result.push_str(&(self.theme.underline)(&inner_text));
                }
            }
        }
        result
    }

    fn render_block(&self, token: &BlockToken, width: usize) -> Vec<String> {
        let mut lines: Vec<String> = Vec::new();
        match token {
            BlockToken::Heading { level, text } => {
                let content = self.render_inline(text);
                // Always strip the heading marker — only show styled text
                let styled = match level {
                    1 => {
                        (self.theme.heading)(&(self.theme.bold)(&(self.theme.underline)(&content)))
                    }
                    _ => (self.theme.heading)(&(self.theme.bold)(&content)),
                };
                for wrapped in crate::utils::wrap_text_with_ansi(&styled, width) {
                    lines.push(wrapped);
                }
                lines.push(String::new()); // spacing after heading
            }
            BlockToken::Paragraph(tokens) => {
                let content = self.render_inline(tokens);
                for wrapped in crate::utils::wrap_text_with_ansi(&content, width) {
                    lines.push(wrapped);
                }
            }
            BlockToken::CodeBlock { lang, code } => {
                // Top border with optional language tag
                let border = (self.theme.code_block_border)("─");
                let _lang_label = lang
                    .as_ref()
                    .map(|l| format!(" {} ", l))
                    .unwrap_or_default();
                lines.push(format!(
                    "{}{}{}",
                    (self.theme.code_block_border)("┌"),
                    border.repeat(width.saturating_sub(2).min(60)),
                    (self.theme.code_block_border)("┐"),
                ));

                // Code content — apply syntax highlighting if available
                let highlighted_lines = if let Some(ref hl) = self.theme.highlight_code {
                    hl(code, lang.as_deref())
                } else {
                    code.lines().map(|l| l.to_string()).collect()
                };

                for code_line in &highlighted_lines {
                    let visible = crate::utils::visible_width(code_line);
                    let padding = width.saturating_sub(visible + 4);
                    let bg_line = (self.theme.code_block_bg)(&format!(
                        "│ {}{}│",
                        code_line,
                        " ".repeat(padding)
                    ));
                    lines.push(bg_line);
                }

                // Bottom border
                lines.push(format!(
                    "{}{}{}",
                    (self.theme.code_block_border)("└"),
                    border.repeat(width.saturating_sub(2).min(60)),
                    (self.theme.code_block_border)("┘"),
                ));
                lines.push(String::new());
            }
            BlockToken::Blockquote(inner) => {
                for inner_token in inner {
                    let inner_lines = self.render_block(inner_token, width.saturating_sub(2));
                    for l in inner_lines {
                        if l.is_empty() {
                            lines.push((self.theme.quote_border)("│ "));
                        } else {
                            lines.push(format!(
                                "{}{}",
                                (self.theme.quote_border)("│ "),
                                (self.theme.quote)(&l)
                            ));
                        }
                    }
                }
            }
            BlockToken::Hr => {
                let rule = (self.theme.hr)(&"─".repeat(width.min(60)));
                lines.push(rule);
                lines.push(String::new());
            }
            BlockToken::ListItem {
                ordered: _ordered,
                checked,
                tokens,
                source_marker,
            } => {
                let content = self.render_inline(tokens);
                let bullet = match checked {
                    Some(true) => (self.theme.list_bullet)("✔ "),
                    Some(false) => (self.theme.list_bullet)("○ "),
                    None => {
                        if self.options.preserve_ordered_list_markers {
                            if let Some(ref marker) = source_marker {
                                (self.theme.list_bullet)(marker)
                            } else {
                                (self.theme.list_bullet)("• ")
                            }
                        } else {
                            (self.theme.list_bullet)("• ")
                        }
                    }
                };
                let line = format!("{}{}", bullet, content);
                for wrapped in crate::utils::wrap_text_with_ansi(&line, width) {
                    lines.push(wrapped);
                }
            }
            BlockToken::Blank => {
                lines.push(String::new());
            }
        }
        lines
    }
}

impl Component for Markdown {
    fn render(&self, width: u16) -> Vec<String> {
        let inner = (width as usize).saturating_sub(self.options.padding_x as usize * 2);
        let spacer = " ".repeat(self.options.padding_x as usize);

        let mut lines: Vec<String> = vec![String::new(); self.options.padding_y as usize];

        let blocks = parse_blocks(&self.content);
        for block in &blocks {
            for l in self.render_block(block, inner) {
                lines.push(format!("{}{}", spacer, l));
            }
        }

        lines.extend(vec![String::new(); self.options.padding_y as usize]);
        lines
    }

    fn invalidate(&mut self) {
        self.cached_width = 0;
        self.cached_lines.clear();
    }
}
