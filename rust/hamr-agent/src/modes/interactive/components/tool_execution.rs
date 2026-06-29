//! Port of `packages/coding-agent/src/modes/interactive/components/tool-execution.ts`.
//!
//! Component that renders tool execution with call/result rendering.

use std::collections::HashMap;
use std::sync::Arc;

use crate::modes::interactive::components::tui_shim::{CardBox, Component, Spacer, Text};
use crate::modes::interactive::theme::theme::theme;

/// Options for configuring a ToolExecutionComponent.
pub struct ToolExecutionOptions {
    pub show_images: bool,
    pub image_width_cells: u16,
    pub model_glyph: Option<String>,
    pub model_accent: Option<String>,
}

impl Default for ToolExecutionOptions {
    fn default() -> Self {
        Self {
            show_images: true,
            image_width_cells: 60,
            model_glyph: None,
            model_accent: None,
        }
    }
}

/// Result content from a tool execution.
#[derive(Debug, Clone)]
pub struct ToolResult {
    pub content: Vec<ToolResultContentBlock>,
    pub is_error: bool,
    pub details: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ToolResultContentBlock {
    pub block_type: String,
    pub text: Option<String>,
    pub data: Option<String>,
    pub mime_type: Option<String>,
}

/// Component that renders tool execution with call/result rendering.
///
/// This is a stub that mirrors the TS structure. In the full implementation,
/// tool-specific renderers (renderCall, renderResult) from `ToolDefinition`
/// are used for custom rendering. This stub handles the generic fallback path.
pub struct ToolExecutionComponent {
    tool_name: String,
    tool_call_id: String,
    args_json: String,
    show_images: bool,
    image_width_cells: u16,
    is_partial: bool,
    execution_started: bool,
    args_complete: bool,
    expanded: bool,
    result: Option<ToolResult>,
    converted_images: HashMap<usize, (String, String)>, // index → (data, mime_type)
    hide_component: bool,
}

impl ToolExecutionComponent {
    pub fn new(
        tool_name: &str,
        tool_call_id: &str,
        args_json: &str,
        options: ToolExecutionOptions,
    ) -> Self {
        ToolExecutionComponent {
            tool_name: tool_name.to_string(),
            tool_call_id: tool_call_id.to_string(),
            args_json: args_json.to_string(),
            show_images: options.show_images,
            image_width_cells: options.image_width_cells,
            is_partial: true,
            execution_started: false,
            args_complete: false,
            expanded: false,
            result: None,
            converted_images: HashMap::new(),
            hide_component: false,
        }
    }

    pub fn update_args(&mut self, args_json: &str) {
        self.args_json = args_json.to_string();
    }

    pub fn mark_execution_started(&mut self) {
        self.execution_started = true;
    }

    pub fn set_args_complete(&mut self) {
        self.args_complete = true;
    }

    pub fn update_result(&mut self, result: ToolResult, is_partial: bool) {
        self.result = Some(result);
        self.is_partial = is_partial;
    }

    pub fn set_expanded(&mut self, expanded: bool) {
        self.expanded = expanded;
    }

    pub fn set_show_images(&mut self, show: bool) {
        self.show_images = show;
    }

    pub fn set_image_width_cells(&mut self, width: u16) {
        self.image_width_cells = width.max(1);
    }

    fn format_tool_execution(&self) -> String {
        let t = theme();
        let mut text = t.fg("toolTitle", &t.bold(&self.tool_name));
        if !self.args_json.is_empty() && self.args_json != "{}" {
            text.push_str(&format!("\n\n{}", self.args_json));
        }
        if let Some(output) = self.get_text_output() {
            text.push_str(&format!("\n{}", output));
        }
        text
    }

    fn get_text_output(&self) -> Option<String> {
        let result = self.result.as_ref()?;
        let mut text = String::new();
        for block in &result.content {
            if block.block_type == "text" {
                if let Some(ref t) = block.text {
                    text.push_str(t);
                }
            }
        }
        if text.is_empty() { None } else { Some(text) }
    }
}

impl Component for ToolExecutionComponent {
    fn render(&self, width: u16) -> Vec<String> {
        if self.hide_component {
            return Vec::new();
        }

        let t = theme();
        let cards = &t.cards;

        let bg_fn: Option<Arc<dyn Fn(&str) -> String + Send + Sync>> = if !cards.shaded_surfaces {
            None
        } else if self.is_partial {
            t.model_adaptive_bg_fn(None, "toolPendingBg")
        } else if self.result.as_ref().map(|r| r.is_error).unwrap_or(false) {
            t.model_adaptive_bg_fn(None, "toolErrorBg")
        } else {
            t.model_adaptive_bg_fn(None, "toolSuccessBg")
        };

        let card_padding_y = cards.card_pad_y;

        let mut content_box = CardBox::new(cards.card_pad_x, card_padding_y, bg_fn);

        // Tool name heading
        content_box.add_child(Box::new(Text::new(
            t.fg("toolTitle", &t.bold(&self.tool_name)),
            cards.tool_indent,
            0,
        )));

        let formatted = self.format_tool_execution();

        // Add the formatted content (args + output)
        for line in formatted.lines().skip(1) {
            // skip the first line which is the tool name
            if !line.is_empty() {
                content_box.add_child(Box::new(Text::new(
                    t.fg("toolOutput", line),
                    cards.tool_result_indent,
                    0,
                )));
            }
        }

        let mut lines = Vec::new();

        // Leading gap (consistent with BashExecutionComponent)
        lines.extend(Spacer::new(if cards.gapless_cards { 0 } else { 1 }).render(width));

        lines.extend(content_box.render(width));

        lines
    }

    fn invalidate(&mut self) {}
}
