use crate::tui::Component;

/// Card widget — rich content block with title, glyph, markdown body,
/// background shading, and configurable padding/spacing.
///
/// Replaces renderPrompt, renderNote, renderAssistantText,
/// renderToolResult, renderCommand functions in Hamr.
pub struct Card {
    /// Optional title line (rendered above body).
    title: Option<String>,
    /// Optional Nerd Font glyph displayed before the title.
    glyph: Option<String>,
    /// Main body content (markdown source text).
    body: String,
    /// Background color applied to every line.
    bg_color: Option<String>,
    /// Border character set (none, thin, rounded, double).
    border_style: BorderStyle,
    /// Border color hex.
    border_color: Option<String>,
    /// Padding: left/right columns.
    padding_x: u16,
    /// Padding: top/bottom rows.
    padding_y: u16,
    /// If true, pad each line to full width.
    full_width: bool,
    /// If true, the title and glyph are rendered as an accent-colored banner bar.
    accent_banner: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BorderStyle {
    None,
    Thin,
    Rounded,
    Double,
}

impl Card {
    pub fn new() -> Self {
        Card {
            title: None,
            glyph: None,
            body: String::new(),
            bg_color: None,
            border_style: BorderStyle::None,
            border_color: None,
            padding_x: 1,
            padding_y: 0,
            full_width: false,
            accent_banner: false,
        }
    }

    /// Set the card title.
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Set the title glyph (Nerd Font icon).
    pub fn with_glyph(mut self, glyph: impl Into<String>) -> Self {
        self.glyph = Some(glyph.into());
        self
    }

    /// Set the body content (markdown).
    pub fn with_body(mut self, body: impl Into<String>) -> Self {
        self.body = body.into();
        self
    }

    /// Set the background color hex.
    pub fn with_bg_color(mut self, color: impl Into<String>) -> Self {
        self.bg_color = Some(color.into());
        self
    }

    /// Set the border style.
    pub fn with_border(mut self, style: BorderStyle) -> Self {
        self.border_style = style;
        self
    }

    /// Set the border color hex.
    pub fn with_border_color(mut self, color: impl Into<String>) -> Self {
        self.border_color = Some(color.into());
        self
    }

    /// Set horizontal padding.
    pub fn with_padding_x(mut self, px: u16) -> Self {
        self.padding_x = px;
        self
    }

    /// Set vertical padding.
    pub fn with_padding_y(mut self, py: u16) -> Self {
        self.padding_y = py;
        self
    }

    /// Enable full-width background on every line.
    pub fn with_full_width(mut self, full: bool) -> Self {
        self.full_width = full;
        self
    }

    /// Enable accent-colored banner bar for title.
    pub fn with_accent_banner(mut self, banner: bool) -> Self {
        self.accent_banner = banner;
        self
    }

    // Mutable setters for runtime updates
    pub fn set_title(&mut self, title: impl Into<String>) {
        self.title = Some(title.into());
    }

    pub fn set_glyph(&mut self, glyph: impl Into<String>) {
        self.glyph = Some(glyph.into());
    }

    pub fn set_body(&mut self, body: impl Into<String>) {
        self.body = body.into();
    }

    pub fn set_bg_color(&mut self, color: impl Into<String>) {
        self.bg_color = Some(color.into());
    }

    /// Apply background color to a line, optionally padding to full width.
    fn style_line(&self, line: &str, width: usize) -> String {
        let w = crate::utils::visible_width(line);

        // If full_width and bg_color is set, pad to full width
        if self.full_width {
            let padded = if w < width {
                format!("{}{}", line, " ".repeat(width.saturating_sub(w)))
            } else {
                line.to_string()
            };
            if let Some(ref bg) = self.bg_color {
                crate::theme::palette::apply_bg(bg, &padded)
            } else {
                padded
            }
        } else if let Some(ref bg) = self.bg_color {
            let padded = format!("{}{}", line, " ".repeat(width.saturating_sub(w)));
            crate::theme::palette::apply_bg(bg, &padded)
        } else {
            line.to_string()
        }
    }

    /// Render the title bar with optional glyph.
    fn render_title(&self, width: usize, accent_color: Option<&str>) -> Vec<String> {
        let mut lines = Vec::new();

        if let Some(ref title) = self.title {
            let glyph_str = self.glyph.as_deref().unwrap_or("");
            let full_title = if glyph_str.is_empty() {
                title.clone()
            } else {
                format!("{}  {}", glyph_str, title)
            };

            let styled_title = if let Some(accent) = accent_color {
                if self.accent_banner {
                    // Banner: full-width accent background with bold
                    let padded = format!(
                        "  {}{}",
                        full_title,
                        " ".repeat(
                            width.saturating_sub(crate::utils::visible_width(&full_title) + 2)
                        )
                    );
                    crate::theme::palette::apply_bg(accent, &format!("\x1b[1m{}\x1b[22m", padded))
                } else {
                    format!(
                        "\x1b[1m{}\x1b[22m",
                        crate::theme::palette::apply_fg(accent, &full_title)
                    )
                }
            } else {
                format!("\x1b[1m{}\x1b[22m", full_title)
            };

            lines.push(styled_title);
            lines.push(String::new()); // spacing after title
        }

        lines
    }

    /// Render body content with basic wrapping.
    fn render_body(&self, inner_width: usize) -> Vec<String> {
        if self.body.is_empty() {
            return Vec::new();
        }

        let mut lines = Vec::new();
        for line in self.body.lines() {
            if line.is_empty() {
                lines.push(String::new());
                continue;
            }

            // Basic inline formatting
            let formatted = self.format_body_line(line);
            let wrapped = crate::utils::wrap_text_with_ansi(&formatted, inner_width);
            for w in wrapped {
                lines.push(w);
            }
        }
        lines
    }

    /// Apply basic inline formatting: **bold**, *italic*, `code`.
    fn format_body_line(&self, line: &str) -> String {
        let mut result = line.to_string();

        // Bold: **text**
        while let Some(start) = result.find("**") {
            if let Some(end) = result[start + 2..].find("**") {
                let bold_text = &result[start + 2..start + 2 + end];
                let replacement = format!("\x1b[1m{}\x1b[22m", bold_text);
                result.replace_range(start..start + 4 + end, &replacement);
            } else {
                break;
            }
        }

        // Inline code: `text`
        while let Some(start) = result.find('`') {
            let rest = &result[start + 1..];
            if let Some(end) = rest.find('`') {
                let code = &rest[..end];
                let replacement = format!("\x1b[2m\x1b[3m{}\x1b[23m\x1b[22m", code);
                result.replace_range(start..start + 2 + end, &replacement);
            } else {
                break;
            }
        }

        result
    }

    /// Render border lines.
    fn render_border_top(&self, width: usize) -> Option<String> {
        let (tl, h, tr) = self.border_chars();
        if self.border_style == BorderStyle::None {
            return None;
        }
        let colored = |s: &str| {
            if let Some(ref c) = self.border_color {
                crate::theme::palette::apply_fg(c, s)
            } else {
                s.to_string()
            }
        };
        Some(format!(
            "{}{}{}",
            colored(tl),
            colored(&h.repeat(width.saturating_sub(2))),
            colored(tr)
        ))
    }

    fn render_border_bottom(&self, width: usize) -> Option<String> {
        let (_tl, h, _tr) = self.border_chars();
        let (bl, _h, br) = self.border_bottom_chars();
        if self.border_style == BorderStyle::None {
            return None;
        }
        let colored = |s: &str| {
            if let Some(ref c) = self.border_color {
                crate::theme::palette::apply_fg(c, s)
            } else {
                s.to_string()
            }
        };
        Some(format!(
            "{}{}{}",
            colored(bl),
            colored(&h.repeat(width.saturating_sub(2))),
            colored(br)
        ))
    }

    fn border_chars(&self) -> (&str, &str, &str) {
        match self.border_style {
            BorderStyle::None => ("", "", ""),
            BorderStyle::Thin => ("┌", "─", "┐"),
            BorderStyle::Rounded => ("╭", "─", "╮"),
            BorderStyle::Double => ("╔", "═", "╗"),
        }
    }

    fn border_bottom_chars(&self) -> (&str, &str, &str) {
        match self.border_style {
            BorderStyle::None => ("", "", ""),
            BorderStyle::Thin => ("└", "─", "┘"),
            BorderStyle::Rounded => ("╰", "─", "╯"),
            BorderStyle::Double => ("╚", "═", "╝"),
        }
    }

    fn border_side(&self) -> &str {
        match self.border_style {
            BorderStyle::None => "",
            BorderStyle::Thin => "│",
            BorderStyle::Rounded => "│",
            BorderStyle::Double => "║",
        }
    }
}

impl Component for Card {
    fn render(&self, width: u16) -> Vec<String> {
        let w = width as usize;
        let has_border = self.border_style != BorderStyle::None;
        let border_w = if has_border { 2 } else { 0 }; // two chars: │ on each side
        let inner_w = w
            .saturating_sub(self.padding_x as usize * 2)
            .saturating_sub(border_w);
        let px = self.padding_x as usize;
        let py = self.padding_y as usize;

        let mut lines: Vec<String> = Vec::new();

        // Top padding
        for _ in 0..py {
            lines.push(self.style_line("", w));
        }

        // Top border
        if let Some(border_line) = self.render_border_top(w) {
            lines.push(self.style_line(&border_line, w));
        }

        // Title
        let accent = self.bg_color.as_deref().or(Some("accent"));
        for title_line in self.render_title(inner_w, accent) {
            let padded = if has_border {
                let side = self.border_side();
                let colored_side = if let Some(ref c) = self.border_color {
                    crate::theme::palette::apply_fg(c, side)
                } else {
                    side.to_string()
                };
                format!(
                    "{}{}{}{}{}",
                    colored_side,
                    " ".repeat(px),
                    title_line,
                    " ".repeat(
                        inner_w.saturating_sub(crate::utils::visible_width(&title_line) + px)
                    ),
                    colored_side
                )
            } else {
                format!("{}{}", " ".repeat(px), title_line)
            };
            lines.push(self.style_line(&padded, w));
        }

        // Body
        for body_line in self.render_body(inner_w) {
            if body_line.is_empty() {
                let padded = if has_border {
                    let side = self.border_side();
                    let colored_side = if let Some(ref c) = self.border_color {
                        crate::theme::palette::apply_fg(c, side)
                    } else {
                        side.to_string()
                    };
                    format!(
                        "{}{}{}",
                        colored_side,
                        " ".repeat(w.saturating_sub(2)),
                        colored_side
                    )
                } else {
                    String::new()
                };
                lines.push(self.style_line(&padded, w));
                continue;
            }

            let padded = if has_border {
                let side = self.border_side();
                let colored_side = if let Some(ref c) = self.border_color {
                    crate::theme::palette::apply_fg(c, side)
                } else {
                    side.to_string()
                };
                let body_visible = crate::utils::visible_width(&body_line);
                format!(
                    "{}{}{}{}{}",
                    colored_side,
                    " ".repeat(px),
                    body_line,
                    " ".repeat(inner_w.saturating_sub(body_visible + px)),
                    colored_side
                )
            } else {
                format!("{}{}", " ".repeat(px), body_line)
            };
            lines.push(self.style_line(&padded, w));
        }

        // Bottom border
        if let Some(border_line) = self.render_border_bottom(w) {
            lines.push(self.style_line(&border_line, w));
        }

        // Bottom padding
        for _ in 0..py {
            lines.push(self.style_line("", w));
        }

        lines
    }

    fn invalidate(&mut self) {}
}

impl Default for Card {
    fn default() -> Self {
        Self::new()
    }
}
