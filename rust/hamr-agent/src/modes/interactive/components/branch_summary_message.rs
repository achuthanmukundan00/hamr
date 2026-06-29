//! Component that renders a branch summary message with collapsed/expanded state.
//!
//! Port of `packages/coding-agent/src/modes/interactive/components/branch-summary-message.ts`.
//!
//! Uses same background color as custom messages for visual consistency.

use crate::modes::interactive::components::keybinding_hints::key_text;
use crate::modes::interactive::components::tui_shim::{
    CardBox, Component, Markdown, MarkdownTheme, Text,
};
use crate::modes::interactive::theme::theme::{cards, get_markdown_theme, theme};

/// The data for a branch summary message.
pub struct BranchSummaryMessage {
    pub summary: String,
}

impl BranchSummaryMessage {
    pub fn new(summary: impl Into<String>) -> Self {
        Self {
            summary: summary.into(),
        }
    }
}

/// Component that renders a branch summary message with collapsed/expanded state.
pub struct BranchSummaryMessageComponent {
    expanded: bool,
    message: BranchSummaryMessage,
    markdown_theme: MarkdownTheme,
    card: CardBox,
}

impl BranchSummaryMessageComponent {
    pub fn new(message: BranchSummaryMessage, markdown_theme: Option<MarkdownTheme>) -> Self {
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
        let label = theme().fg("customMessageLabel", "\x1b[1m[branch]\x1b[22m");
        self.card
            .add_child(Box::new(Text::new(label, cards_cfg.heading_indent, 0)));

        if self.expanded {
            let header = "**Branch Summary**\n\n";
            let md_text = format!("{}{}", header, self.message.summary);
            self.card.add_child(Box::new(Markdown::new(
                md_text,
                cards_cfg.body_indent,
                0,
                self.markdown_theme.clone(),
            )));
        } else {
            let msg = format!(
                "{}Branch summary ({}{}{} to expand)",
                theme().fg("customMessageText", ""),
                theme().fg("dim", &key_text("app.tools.expand")),
                theme().fg("customMessageText", ""),
                theme().fg("customMessageText", ""),
            );
            // Simplified: render as text
            self.card
                .add_child(Box::new(Text::new(msg, cards_cfg.body_indent, 0)));
        }
    }
}

impl Component for BranchSummaryMessageComponent {
    fn render(&self, width: u16) -> Vec<String> {
        self.card.render(width)
    }

    fn invalidate(&mut self) {
        self.card.invalidate();
        self.update_display();
    }
}
