//! Component that renders a complete assistant message.
//!
//! Port of `packages/coding-agent/src/modes/interactive/components/assistant-message.ts`.
//!
//! Renders thinking blocks and text blocks as visually distinct cards. Model
//! identity is kept as a heading accent only; card surfaces come from the theme
//! so prompt/response/tool blocks stay visually consistent across models.

use crate::modes::interactive::components::tui_shim::{
    CardBox, Component, Container, Markdown, MarkdownTheme, Spacer, Text,
};
use crate::modes::interactive::theme::theme::{cards, get_markdown_theme, theme};

const OSC133_ZONE_START: &str = "\x1b]133;A\x07";
const OSC133_ZONE_END: &str = "\x1b]133;B\x07";
const OSC133_ZONE_FINAL: &str = "\x1b]133;C\x07";

/// A content block within an assistant message.
#[derive(Debug, Clone)]
pub enum ContentBlock {
    /// Plain text response.
    Text { text: String },
    /// Thinking/reasoning block.
    Thinking { thinking: String },
    /// Tool call request.
    ToolCall {
        id: String,
        name: String,
        input: serde_json::Value,
    },
}

impl ContentBlock {
    pub fn text(text: impl Into<String>) -> Self {
        ContentBlock::Text { text: text.into() }
    }

    pub fn thinking(thinking: impl Into<String>) -> Self {
        ContentBlock::Thinking {
            thinking: thinking.into(),
        }
    }

    pub fn tool_call(
        id: impl Into<String>,
        name: impl Into<String>,
        input: serde_json::Value,
    ) -> Self {
        ContentBlock::ToolCall {
            id: id.into(),
            name: name.into(),
            input,
        }
    }
}

/// Stop reason for an assistant message.
#[derive(Debug, Clone, PartialEq)]
pub enum StopReason {
    EndTurn,
    MaxTokens,
    StopSequence,
    ToolUse,
    Aborted,
    Error,
}

/// A complete assistant message from the LLM.
#[derive(Debug, Clone)]
pub struct AssistantMessage {
    pub content: Vec<ContentBlock>,
    pub model: String,
    pub stop_reason: Option<StopReason>,
    pub error_message: Option<String>,
}

impl AssistantMessage {
    pub fn new(model: impl Into<String>, content: Vec<ContentBlock>) -> Self {
        Self {
            content,
            model: model.into(),
            stop_reason: None,
            error_message: None,
        }
    }

    pub fn with_error(model: impl Into<String>, error_message: impl Into<String>) -> Self {
        Self {
            content: Vec::new(),
            model: model.into(),
            stop_reason: Some(StopReason::Error),
            error_message: Some(error_message.into()),
        }
    }
}

/// Convert a hex color string to an ANSI foreground escape.
pub fn hex_to_ansi_fg(hex: &str) -> String {
    if hex.len() < 7 || !hex.starts_with('#') {
        return String::new();
    }
    let r = u8::from_str_radix(&hex[1..3], 16).unwrap_or(255);
    let g = u8::from_str_radix(&hex[3..5], 16).unwrap_or(255);
    let b = u8::from_str_radix(&hex[5..7], 16).unwrap_or(255);
    format!("\x1b[38;2;{};{};{}m", r, g, b)
}

/// Component that renders a complete assistant message.
pub struct AssistantMessageComponent {
    container: Container,
    hide_thinking_block: bool,
    markdown_theme: MarkdownTheme,
    hidden_thinking_label: String,
    last_message: Option<AssistantMessage>,
    has_tool_calls: bool,
    model_accent: Option<String>,
    model_glyph: Option<String>,
}

impl AssistantMessageComponent {
    pub fn new(
        message: Option<AssistantMessage>,
        hide_thinking_block: bool,
        markdown_theme: Option<MarkdownTheme>,
        hidden_thinking_label: Option<String>,
        model_accent: Option<String>,
        model_glyph: Option<String>,
    ) -> Self {
        let mt = markdown_theme.unwrap_or_else(get_markdown_theme);
        let mut comp = Self {
            container: Container::new(),
            hide_thinking_block,
            markdown_theme: mt,
            hidden_thinking_label: hidden_thinking_label
                .unwrap_or_else(|| "Thinking...".to_string()),
            last_message: None,
            has_tool_calls: false,
            model_accent,
            model_glyph,
        };

        if let Some(msg) = message {
            comp.update_content(&msg);
        }

        comp
    }

    pub fn set_hide_thinking_block(&mut self, hide: bool) {
        self.hide_thinking_block = hide;
        if let Some(ref msg) = self.last_message.clone() {
            self.update_content(msg);
        }
    }

    pub fn set_hidden_thinking_label(&mut self, label: String) {
        self.hidden_thinking_label = label;
        if let Some(ref msg) = self.last_message.clone() {
            self.update_content(msg);
        }
    }

    pub fn set_model_accent(&mut self, hex: Option<String>) {
        self.model_accent = hex;
        if let Some(ref msg) = self.last_message.clone() {
            self.update_content(msg);
        }
    }

    pub fn update_content(&mut self, message: &AssistantMessage) {
        self.last_message = Some(message.clone());
        self.container.clear();

        let has_visible_content = message.content.iter().any(|c| match c {
            ContentBlock::Text { text } => !text.trim().is_empty(),
            ContentBlock::Thinking { thinking } => !thinking.trim().is_empty(),
            ContentBlock::ToolCall { .. } => false,
        });

        if !has_visible_content {
            let has_tool_calls = message
                .content
                .iter()
                .any(|c| matches!(c, ContentBlock::ToolCall { .. }));
            self.has_tool_calls = has_tool_calls;

            if !has_tool_calls {
                if message.stop_reason == Some(StopReason::Aborted) {
                    let msg = message
                        .error_message
                        .as_ref()
                        .filter(|e| *e != "Request was aborted")
                        .cloned()
                        .unwrap_or_else(|| "Operation aborted".to_string());
                    self.add_status_card(&theme().fg("error", &msg));
                } else if message.stop_reason == Some(StopReason::Error) {
                    let error_msg = message.error_message.as_deref().unwrap_or("Unknown error");
                    self.add_status_card(&theme().fg("error", &format!("Error: {}", error_msg)));
                }
            }
            return;
        }

        let accent_fg = self.model_accent.as_ref().map(|h| hex_to_ansi_fg(h));

        let accent = |s: &str| -> String {
            if let Some(ref fg) = accent_fg {
                format!("{}\x1b[39m{}", fg, s)
            } else {
                theme().fg("accent", s)
            }
        };

        let cards_cfg = cards();
        let glyph: Option<String> = match cards_cfg.heading_glyph.as_str() {
            "model" => self.model_glyph.clone(),
            "" => None,
            g => Some(g.to_string()),
        };
        let show_headings = cards_cfg.show_headings && glyph.is_some();

        // Extract model_accent before building closures to avoid borrow conflicts
        let model_accent = self.model_accent.clone();

        let response_bg: Option<std::sync::Arc<dyn Fn(&str) -> String + Send + Sync>> =
            if cards_cfg.shaded_surfaces {
                Some(std::sync::Arc::new(move |t: &str| {
                    if let Some(ref hex) = model_accent {
                        let fg = hex_to_ansi_fg(hex);
                        format!("{}\x1b[49m{}", fg, theme().bg("cardBg", t))
                    } else {
                        theme().bg("cardBg", t)
                    }
                }))
            } else {
                None
            };

        let thinking_bg: Option<std::sync::Arc<dyn Fn(&str) -> String + Send + Sync>> =
            if cards_cfg.shaded_surfaces {
                Some(std::sync::Arc::new(move |t: &str| {
                    if cards_cfg.thinking_shaded {
                        theme().bg("thinkingBg", t)
                    } else {
                        theme().bg("cardBg", t)
                    }
                }))
            } else {
                None
            };

        let body_indent = if show_headings {
            cards_cfg.body_indent
        } else {
            cards_cfg.heading_indent
        };

        let mut response_heading_rendered = false;
        let mut thought_heading_rendered = false;

        let add_heading = |card: &mut CardBox, label: &str| {
            if !show_headings {
                return;
            }
            if let Some(ref g) = glyph {
                let heading = accent(&format!("{}{} {}", theme().bold(""), g, label));
                card.add_child(Box::new(Text::new(heading, cards_cfg.heading_indent, 0)));
            }
        };

        let mut blocks_added = 0;
        for content in &message.content {
            if !cards_cfg.gapless_cards && blocks_added > 0 {
                self.container.add_child(Box::new(Spacer::new(1)));
            }

            match content {
                ContentBlock::Text { text } if !text.trim().is_empty() => {
                    let mut text_card = CardBox::new(
                        cards_cfg.card_pad_x,
                        cards_cfg.card_pad_y,
                        response_bg.clone(),
                    );
                    if !response_heading_rendered {
                        add_heading(&mut text_card, &cards_cfg.response_label);
                        response_heading_rendered = true;
                    }
                    text_card.add_child(Box::new(Markdown::new(
                        text.trim(),
                        body_indent,
                        0,
                        self.markdown_theme.clone(),
                    )));
                    self.container.add_child(Box::new(text_card));
                    blocks_added += 1;
                }

                ContentBlock::Thinking { thinking } if !thinking.trim().is_empty() => {
                    if self.hide_thinking_block {
                        let label = theme()
                            .italic(&theme().fg("thinkingText", &self.hidden_thinking_label));
                        self.container
                            .add_child(Box::new(Text::new(label, body_indent, 0)));
                    } else {
                        let mut thinking_card = CardBox::new(
                            cards_cfg.card_pad_x,
                            cards_cfg.card_pad_y,
                            thinking_bg.clone(),
                        );
                        if !thought_heading_rendered && cards_cfg.show_thought_heading {
                            add_heading(&mut thinking_card, &cards_cfg.thought_label);
                            thought_heading_rendered = true;
                        }
                        thinking_card.add_child(Box::new(Markdown::new(
                            thinking.trim(),
                            body_indent,
                            0,
                            self.markdown_theme.clone(),
                        )));
                        self.container.add_child(Box::new(thinking_card));
                        blocks_added += 1;
                    }
                }

                _ => {}
            }
        }

        let has_tool_calls = message
            .content
            .iter()
            .any(|c| matches!(c, ContentBlock::ToolCall { .. }));
        self.has_tool_calls = has_tool_calls;

        if !has_tool_calls {
            match &message.stop_reason {
                Some(StopReason::Aborted) => {
                    let msg = message
                        .error_message
                        .as_ref()
                        .filter(|e| *e != "Request was aborted")
                        .cloned()
                        .unwrap_or_else(|| "Operation aborted".to_string());
                    self.add_status_card(&theme().fg("error", &msg));
                }
                Some(StopReason::Error) => {
                    let error_msg = message.error_message.as_deref().unwrap_or("Unknown error");
                    self.add_status_card(&theme().fg("error", &format!("Error: {}", error_msg)));
                }
                _ => {}
            }
        }
    }

    fn add_status_card(&mut self, message: &str) {
        let cards_cfg = cards();
        let bg_fn: Option<std::sync::Arc<dyn Fn(&str) -> String + Send + Sync>> =
            if cards_cfg.shaded_surfaces {
                Some(std::sync::Arc::new(|t: &str| theme().bg("cardBg", t)))
            } else {
                None
            };
        let mut status_card = CardBox::new(cards_cfg.card_pad_x, cards_cfg.card_pad_y, bg_fn);
        status_card.add_child(Box::new(Text::new(message, cards_cfg.heading_indent, 0)));
        self.container.add_child(Box::new(status_card));
    }
}

impl Component for AssistantMessageComponent {
    fn render(&self, width: u16) -> Vec<String> {
        let mut lines = self.container.render(width);
        if self.has_tool_calls || lines.is_empty() {
            return lines;
        }

        if let Some(first) = lines.first_mut() {
            *first = format!("{}{}", OSC133_ZONE_START, first);
        }
        if let Some(last) = lines.last_mut() {
            *last = format!("{}{}{}", OSC133_ZONE_END, OSC133_ZONE_FINAL, last);
        }

        lines
    }

    fn invalidate(&mut self) {
        self.container.invalidate();
        if let Some(ref msg) = self.last_message.clone() {
            self.update_content(msg);
        }
    }
}
