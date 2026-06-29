//! Port of `packages/coding-agent/src/modes/interactive/components/user_message.ts`.
//!
//! Component that renders a user message with a branded PROMPT card header.

use crate::modes::interactive::components::tui_shim::{
    CardBox, Component, Markdown, MarkdownTheme, Text,
};
use crate::modes::interactive::theme::theme::Theme;

const OSC133_ZONE_START: &str = "\x1b]133;A\x07";
const OSC133_ZONE_END: &str = "\x1b]133;B\x07";
const OSC133_ZONE_FINAL: &str = "\x1b]133;C\x07";

/// Convert a hex color string (e.g. "#875fff") to an ANSI foreground color escape.
fn hex_to_ansi_fg(hex: &str) -> String {
    let r = u8::from_str_radix(&hex[1..3], 16).unwrap_or(0);
    let g = u8::from_str_radix(&hex[3..5], 16).unwrap_or(0);
    let b = u8::from_str_radix(&hex[5..7], 16).unwrap_or(0);
    format!("\x1b[38;2;{};{};{}m", r, g, b)
}

/// Component that renders a user message with a branded PROMPT card header.
///
/// Every user message was sent to a specific model — the card heading always
/// shows that model's glyph + "PROMPT" so you can see which model you prompted
/// even after mid-session model switches. The heading color reflects the model's
/// brand accent when modelAdaptive is on, or the theme accent when off.
pub struct UserMessageComponent {
    content_box: CardBox,
    /// Pre-rendered content to add OSC 133 markers around.
    rendered_cache: Vec<String>,
    /// The width this component was last rendered at.
    last_width: Option<u16>,
}

impl UserMessageComponent {
    pub fn new(
        text: &str,
        theme: &Theme,
        markdown_theme: MarkdownTheme,
        model_accent: Option<&str>,
        model_glyph: Option<&str>,
    ) -> Self {
        let cards = &theme.cards;
        let glyph = match cards.prompt_heading_glyph.as_deref() {
            Some("model") => model_glyph.unwrap_or(""),
            Some(g) => g,
            None => "",
        };
        let show_heading = cards.show_headings && !glyph.is_empty();

        // Keep model color as an accent only. Using it as the card background
        // makes orange/red models dominate the entire prompt block.
        let prompt_bg_fn = if cards.shaded_surfaces {
            theme.model_adaptive_bg_fn(model_accent, "userMessageBg")
        } else {
            None
        };

        let mut content_box = CardBox::new(cards.card_pad_x, cards.card_pad_y, prompt_bg_fn);

        // Show the glyph + label heading when configured. Uses model brand color
        // when modelAdaptive, theme accent otherwise.
        if show_heading {
            let heading_color: Option<std::sync::Arc<dyn Fn(&str) -> String + Send + Sync>> =
                if theme.model_adaptive && model_accent.is_some() {
                    let accent = model_accent.unwrap().to_string();
                    let ansi = hex_to_ansi_fg(&accent);
                    Some(std::sync::Arc::new(move |s: &str| -> String {
                        format!("{}{}\x1b[39m", ansi, s)
                    }))
                } else {
                    None
                };

            let label_text = format!("{} {}", glyph, cards.prompt_label);
            let styled_label = theme.bold(
                &heading_color
                    .as_ref()
                    .map(|f| f(&label_text))
                    .unwrap_or_else(|| theme.fg("accent", &label_text)),
            );

            content_box.add_child(Box::new(Text::new(styled_label, cards.heading_indent, 0)));
        }

        // Indent the body so it nests under the label (past the glyph); without a
        // heading, keep the body at the base heading indent.
        let body_indent = if show_heading {
            cards.body_indent
        } else {
            cards.heading_indent
        };

        content_box.add_child(Box::new(Markdown::new(
            text,
            body_indent,
            0,
            markdown_theme,
        )));

        UserMessageComponent {
            content_box,
            rendered_cache: Vec::new(),
            last_width: None,
        }
    }
}

impl Component for UserMessageComponent {
    fn render(&self, width: u16) -> Vec<String> {
        let lines = self.content_box.render(width);
        if lines.is_empty() {
            return lines;
        }

        let mut result = Vec::with_capacity(lines.len());
        for (i, line) in lines.iter().enumerate() {
            if i == 0 {
                result.push(format!("{}{}", OSC133_ZONE_START, line));
            } else if i == lines.len() - 1 {
                result.push(format!("{}{}{}", OSC133_ZONE_END, OSC133_ZONE_FINAL, line));
            } else {
                result.push(line.clone());
            }
        }
        result
    }

    fn invalidate(&mut self) {
        self.content_box.invalidate();
    }
}
