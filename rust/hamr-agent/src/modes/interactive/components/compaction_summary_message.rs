//! Component that renders a compaction summary message with collapsed/expanded state.
//!
//! Port of `packages/coding-agent/src/modes/interactive/components/compaction-summary-message.ts`.
//!
//! Uses same background color as custom messages for visual consistency.

use crate::modes::interactive::components::keybinding_hints::key_text;
use crate::modes::interactive::components::tui_shim::{
    CardBox, Component, Markdown, MarkdownTheme, Text,
};
use crate::modes::interactive::theme::theme::{cards, get_markdown_theme, theme};

/// The data for a compaction summary message.
pub struct CompactionSummaryMessage {
    pub summary: String,
    pub tokens_before: u64,
}

impl CompactionSummaryMessage {
    pub fn new(summary: impl Into<String>, tokens_before: u64) -> Self {
        Self {
            summary: summary.into(),
            tokens_before,
        }
    }
}

/// Component that renders a compaction summary message with collapsed/expanded state.
pub struct CompactionSummaryMessageComponent {
    expanded: bool,
    message: CompactionSummaryMessage,
    markdown_theme: MarkdownTheme,
    card: CardBox,
}

impl CompactionSummaryMessageComponent {
    pub fn new(message: CompactionSummaryMessage, markdown_theme: Option<MarkdownTheme>) -> Self {
        let cards_cfg = cards();
        let card = CardBox::new(
            cards_cfg.card_pad_x,
            cards_cfg.card_pad_y,
            Some(std::sync::Arc::new(|t: &str| {
                theme().bg("customMessageBg", t)
            })),
        );
        let mt = markdown_theme.unwrap_or_else(get_markdown_theme);

        let mut comp = Self {
            expanded: false,
            message,
            markdown_theme: mt,
            card,
        };
        comp.update_display();
        comp
    }

    pub fn set_expanded(&mut self, expanded: bool) {
        self.expanded = expanded;
        self.update_display();
    }

    fn update_display(&mut self) {
        self.card.clear();

        let cards_cfg = cards();
        let token_str = format_number(self.message.tokens_before);
        let label = theme().fg("customMessageLabel", "\x1b[1m[compaction]\x1b[22m");
        self.card
            .add_child(Box::new(Text::new(label, cards_cfg.heading_indent, 0)));

        if self.expanded {
            let header = format!("**Compacted from {} tokens**\n\n", token_str);
            let md_text = format!("{}{}", header, self.message.summary);
            self.card.add_child(Box::new(Markdown::new(
                md_text,
                cards_cfg.body_indent,
                0,
                self.markdown_theme.clone(),
            )));
        } else {
            let msg = format!(
                "{}Compacted from {} tokens ({}{}{} to expand)",
                theme().fg("customMessageText", ""),
                token_str,
                theme().fg("dim", &key_text("app.tools.expand")),
                theme().fg("customMessageText", ""),
                theme().fg("customMessageText", ""),
            );
            self.card
                .add_child(Box::new(Text::new(msg, cards_cfg.body_indent, 0)));
        }
    }
}

impl Component for CompactionSummaryMessageComponent {
    fn render(&self, width: u16) -> Vec<String> {
        self.card.render(width)
    }

    fn invalidate(&mut self) {
        self.card.invalidate();
        self.update_display();
    }
}

/// Format a number with locale-aware separators (simplified: just commas for thousands).
fn format_number(n: u64) -> String {
    let s = n.to_string();
    let len = s.len();
    if len <= 3 {
        return s;
    }
    let mut result = String::new();
    for (i, ch) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            result.push(',');
        }
        result.push(ch);
    }
    result.chars().rev().collect()
}
