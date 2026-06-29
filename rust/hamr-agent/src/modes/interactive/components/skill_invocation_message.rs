//! Component that renders a skill invocation message with collapsed/expanded state.
//!
//! Port of `packages/coding-agent/src/modes/interactive/components/skill-invocation-message.ts`.
//!
//! Uses same background color as custom messages for visual consistency.
//! Only renders the skill block itself — user message is rendered separately.

use crate::modes::interactive::components::keybinding_hints::key_text;
use crate::modes::interactive::components::tui_shim::{
    CardBox, Component, Markdown, MarkdownTheme, Text,
};
use crate::modes::interactive::theme::theme::{cards, get_markdown_theme, theme};
use std::sync::Arc;

/// Represents a parsed skill block extracted from a user message.
pub struct ParsedSkillBlock {
    pub name: String,
    pub content: String,
}

/// A component that renders a skill invocation message with collapsed/expanded state.
/// Collapsed: single line `[skill] name (key to expand)`
/// Expanded: `[skill]` label + bold header + full markdown content.
pub struct SkillInvocationMessageComponent {
    card: CardBox,
    expanded: bool,
    skill_block: ParsedSkillBlock,
    markdown_theme: MarkdownTheme,
}

impl SkillInvocationMessageComponent {
    /// Create a new skill invocation message component.
    ///
    /// * `skill_block` - the parsed skill block (name + content)
    /// * `markdown_theme` - optional custom markdown theme
    pub fn new(skill_block: ParsedSkillBlock, markdown_theme: Option<MarkdownTheme>) -> Self {
        let c = cards();
        let bg_fn: Arc<dyn Fn(&str) -> String + Send + Sync> =
            Arc::new(|s: &str| theme().bg("customMessageBg", s));
        let card = CardBox::new(c.card_pad_x, c.card_pad_y, Some(bg_fn));

        let theme = markdown_theme.unwrap_or_else(get_markdown_theme);
        let mut result = Self {
            card,
            expanded: false,
            skill_block,
            markdown_theme: theme,
        };

        result.update_display();
        result
    }

    /// Expand or collapse the skill content.
    pub fn set_expanded(&mut self, expanded: bool) {
        self.expanded = expanded;
        self.update_display();
    }

    /// Rebuild the card contents based on expanded/collapsed state.
    fn update_display(&mut self) {
        self.card.clear();
        let c = cards();

        if self.expanded {
            // Expanded: label + skill name header + full content
            let label = theme().fg("customMessageLabel", "\x1b[1m[skill]\x1b[22m");
            self.card
                .add_child(Box::new(Text::new(label, c.heading_indent, 0)));

            let header = format!("**{}**\n\n", self.skill_block.name);
            let content = format!("{}{}", header, self.skill_block.content);
            self.card.add_child(Box::new(Markdown::new(
                content,
                c.body_indent,
                0,
                self.markdown_theme.clone(),
            )));
        } else {
            // Collapsed: single line — [skill] name (hint to expand)
            let line = format!(
                "{}{}{}",
                theme().fg("customMessageLabel", "\x1b[1m[skill]\x1b[22m "),
                theme().fg("customMessageText", &self.skill_block.name),
                theme().fg(
                    "dim",
                    &format!(" ({} to expand)", key_text("app.tools.expand"))
                )
            );
            self.card
                .add_child(Box::new(Text::new(line, c.heading_indent, 0)));
        }
    }
}

impl Component for SkillInvocationMessageComponent {
    fn render(&self, width: u16) -> Vec<String> {
        self.card.render(width)
    }

    fn invalidate(&mut self) {
        self.card.invalidate();
        self.update_display();
    }
}
