//! Component that renders a custom message entry from extensions.
//!
//! Port of `packages/coding-agent/src/modes/interactive/components/custom-message.ts`.
//!
//! Uses distinct styling to differentiate from user messages.

use crate::modes::interactive::components::tui_shim::{
    CardBox, Component, Container, Markdown, MarkdownTheme, Text,
};
use crate::modes::interactive::theme::theme::{cards, get_markdown_theme, theme};

/// Content of a custom message — either a plain string or message content blocks.
pub enum CustomMessageContent {
    String(String),
    // For content blocks: we store them as strings keyed by type
    Blocks(Vec<(String, String)>), // (type, text) pairs
}

/// A custom message from an extension.
pub struct CustomMessage {
    pub custom_type: String,
    pub content: CustomMessageContent,
    pub metadata: serde_json::Value,
}

impl CustomMessage {
    pub fn new(custom_type: impl Into<String>, content: CustomMessageContent) -> Self {
        Self {
            custom_type: custom_type.into(),
            content,
            metadata: serde_json::Value::Null,
        }
    }
}

/// Component that renders a custom message entry from extensions.
pub struct CustomMessageComponent {
    container: Container,
    message: CustomMessage,
    expanded: bool,
    markdown_theme: MarkdownTheme,
}

impl CustomMessageComponent {
    pub fn new(message: CustomMessage, markdown_theme: Option<MarkdownTheme>) -> Self {
        let mt = markdown_theme.unwrap_or_else(get_markdown_theme);
        let mut comp = Self {
            container: Container::new(),
            message,
            expanded: false,
            markdown_theme: mt,
        };
        comp.rebuild();
        comp
    }

    pub fn set_expanded(&mut self, expanded: bool) {
        if self.expanded != expanded {
            self.expanded = expanded;
            self.rebuild();
        }
    }

    fn rebuild(&mut self) {
        self.container.clear();

        let cards_cfg = cards();
        let mut card = CardBox::new(
            cards_cfg.card_pad_x,
            cards_cfg.card_pad_y,
            Some(std::sync::Arc::new(|t: &str| {
                theme().bg("customMessageBg", t)
            })),
        );

        let label = theme().fg(
            "customMessageLabel",
            &format!("\x1b[1m[{}]\x1b[22m", self.message.custom_type),
        );
        card.add_child(Box::new(Text::new(label, cards_cfg.heading_indent, 0)));

        let text = match &self.message.content {
            CustomMessageContent::String(s) => s.clone(),
            CustomMessageContent::Blocks(blocks) => blocks
                .iter()
                .filter(|(t, _)| t == "text")
                .map(|(_, text)| text.as_str())
                .collect::<Vec<_>>()
                .join("\n"),
        };

        card.add_child(Box::new(Markdown::new(
            text,
            cards_cfg.body_indent,
            0,
            self.markdown_theme.clone(),
        )));

        self.container.add_child(Box::new(card));
    }
}

impl Component for CustomMessageComponent {
    fn render(&self, width: u16) -> Vec<String> {
        self.container.render(width)
    }

    fn invalidate(&mut self) {
        self.container.invalidate();
        self.rebuild();
    }
}
