//! Component for displaying bash command execution with streaming output.
//!
//! Port of `packages/coding-agent/src/modes/interactive/components/bash-execution.ts`.

use crate::modes::interactive::components::dynamic_border::DynamicBorder;
use crate::modes::interactive::components::keybinding_hints::{key_hint, key_text};
use crate::modes::interactive::components::tui_shim::{Component, Container, Loader, Spacer, Text};
use crate::modes::interactive::theme::theme::{cards, theme};
use std::sync::Arc;

/// Constants matching the bash tool truncation limits.
const DEFAULT_MAX_BYTES: usize = 400_000;
const DEFAULT_MAX_LINES: usize = 500;
/// Preview line limit when not expanded.
const PREVIEW_LINES: usize = 20;

/// Result of truncating output.
#[derive(Debug, Clone)]
pub struct TruncationResult {
    pub truncated: bool,
    pub content: String,
    pub original_byte_count: usize,
    pub original_line_count: usize,
}

impl TruncationResult {
    pub fn empty() -> Self {
        Self {
            truncated: false,
            content: String::new(),
            original_byte_count: 0,
            original_line_count: 0,
        }
    }
}

/// Status of a bash execution.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BashStatus {
    Running,
    Complete,
    Cancelled,
    Error,
}

/// Component for displaying bash command execution with streaming output.
pub struct BashExecutionComponent {
    command: String,
    output_lines: Vec<String>,
    status: BashStatus,
    exit_code: Option<i32>,
    truncation_result: Option<TruncationResult>,
    full_output_path: Option<String>,
    expanded: bool,
    exclude_from_context: bool,
    container: Container,
    content_container: Container,
    leading_spacer: Spacer,
    loader_running: bool,
}

impl BashExecutionComponent {
    pub fn new(command: impl Into<String>, exclude_from_context: bool) -> Self {
        let command = command.into();

        let color_key = if exclude_from_context {
            "dim"
        } else {
            "bashMode"
        };

        let border_color: Arc<dyn Fn(&str) -> String + Send + Sync> =
            Arc::new(move |s: &str| theme().fg(color_key, s));

        let cards_cfg = cards();

        let mut container = Container::new();

        let leading_spacer = Spacer::new(if cards_cfg.gapless_cards { 0 } else { 1 });
        container.add_child(Box::new(Spacer::new(leading_spacer.render(0).len() as u16)));

        // Top border
        container.add_child(Box::new(DynamicBorder::new(Some(border_color.clone()))));

        // Content container
        let _content_container = Container::new();
        container.add_child(Box::new(Container::new())); // placeholder, will be rebuilt

        // Bottom border
        container.add_child(Box::new(DynamicBorder::new(Some(border_color.clone()))));

        let mut comp = Self {
            command: command.clone(),
            output_lines: Vec::new(),
            status: BashStatus::Running,
            exit_code: None,
            truncation_result: None,
            full_output_path: None,
            expanded: false,
            exclude_from_context,
            container: Container::new(), // will be properly set up
            content_container: Container::new(),
            leading_spacer: Spacer::new(if cards().gapless_cards { 0 } else { 1 }),
            loader_running: true,
        };

        comp.rebuild_container(&command);
        comp
    }

    fn rebuild_container(&mut self, command: &str) {
        let color_key = if self.exclude_from_context {
            "dim"
        } else {
            "bashMode"
        };
        let border_color: Arc<dyn Fn(&str) -> String + Send + Sync> =
            Arc::new(move |s: &str| theme().fg(color_key, s));

        self.container.clear();
        self.leading_spacer
            .set_lines(if cards().gapless_cards { 0 } else { 1 });
        self.container.add_child(Box::new(Spacer::new(
            self.leading_spacer.render(0).len() as u16
        )));

        self.container
            .add_child(Box::new(DynamicBorder::new(Some(border_color.clone()))));

        self.content_container = Container::new();
        let header = Text::new(
            theme().fg(color_key, &theme().bold(&format!("$ {}", command))),
            cards().tool_indent,
            0,
        );
        self.content_container.add_child(Box::new(header));

        let loader = if self.loader_running {
            let l = Loader::new(&format!(
                "Running... ({} to cancel)",
                key_text("tui.select.cancel")
            ));
            l
        } else {
            Loader::new("")
        };
        self.content_container.add_child(Box::new(loader));

        self.container.add_child(Box::new(Container::new())); // placeholder

        self.container
            .add_child(Box::new(DynamicBorder::new(Some(border_color.clone()))));
    }

    /// Set whether the output is expanded (shows full output) or collapsed (preview only).
    pub fn set_expanded(&mut self, expanded: bool) {
        self.expanded = expanded;
        self.update_display();
    }

    /// Append streaming output chunk.
    pub fn append_output(&mut self, chunk: &str) {
        let clean = strip_ansi(chunk).replace("\r\n", "\n").replace('\r', "\n");

        let new_lines: Vec<&str> = clean.split('\n').collect();

        if !self.output_lines.is_empty() && !new_lines.is_empty() {
            // Append first chunk to last line (incomplete line continuation)
            if let Some(last) = self.output_lines.last_mut() {
                last.push_str(new_lines[0]);
            }
            for line in &new_lines[1..] {
                self.output_lines.push(line.to_string());
            }
        } else {
            for line in &new_lines {
                self.output_lines.push(line.to_string());
            }
        }

        self.update_display();
    }

    /// Mark execution as complete, cancelled, or error.
    pub fn set_complete(
        &mut self,
        exit_code: Option<i32>,
        cancelled: bool,
        truncation_result: Option<TruncationResult>,
        full_output_path: Option<String>,
    ) {
        self.exit_code = exit_code;
        self.status = if cancelled {
            BashStatus::Cancelled
        } else if exit_code.is_some() && exit_code != Some(0) {
            BashStatus::Error
        } else {
            BashStatus::Complete
        };
        self.truncation_result = truncation_result;
        self.full_output_path = full_output_path;
        self.loader_running = false;

        self.update_display();
    }

    /// Get the raw output.
    pub fn get_output(&self) -> String {
        self.output_lines.join("\n")
    }

    /// Get the command that was executed.
    pub fn get_command(&self) -> &str {
        &self.command
    }

    fn update_display(&mut self) {
        self.leading_spacer
            .set_lines(if cards().gapless_cards { 0 } else { 1 });

        let full_output = self.output_lines.join("\n");
        let context_truncation = truncate_tail(&full_output, DEFAULT_MAX_LINES, DEFAULT_MAX_BYTES);

        let available_lines: Vec<&str> = if context_truncation.content.is_empty() {
            Vec::new()
        } else {
            context_truncation.content.split('\n').collect()
        };

        let preview_start = available_lines.len().saturating_sub(PREVIEW_LINES);
        let preview_logical_lines: Vec<&str> = available_lines[preview_start..].to_vec();
        let hidden_line_count = available_lines
            .len()
            .saturating_sub(preview_logical_lines.len());

        // Rebuild content container
        self.content_container.clear();

        let color_key = if self.exclude_from_context {
            "dim"
        } else {
            "bashMode"
        };

        // Command header
        let header = Text::new(
            theme().fg(color_key, &theme().bold(&format!("$ {}", self.command))),
            cards().tool_indent,
            0,
        );
        self.content_container.add_child(Box::new(header));

        // Output
        if !available_lines.is_empty() {
            if self.expanded {
                let display_text: Vec<String> = available_lines
                    .iter()
                    .map(|line| theme().fg("muted", line))
                    .collect();
                let display = format!("\n{}", display_text.join("\n"));
                self.content_container.add_child(Box::new(Text::new(
                    display,
                    cards().tool_result_indent,
                    0,
                )));
            } else {
                let styled_output: Vec<String> = preview_logical_lines
                    .iter()
                    .map(|line| theme().fg("muted", line))
                    .collect();
                let styled_input = format!("\n{}", styled_output.join("\n"));

                // Visual truncation: just show the preview lines
                let truncated = clip_lines(&styled_input, PREVIEW_LINES);
                self.content_container.add_child(Box::new(Text::new(
                    truncated,
                    cards().tool_result_indent,
                    0,
                )));
            }
        }

        // Loader or status
        if self.status == BashStatus::Running {
            let loader = Loader::new(&format!(
                "Running... ({} to cancel)",
                key_text("tui.select.cancel")
            ));
            self.content_container.add_child(Box::new(loader));
        } else {
            let mut status_parts: Vec<String> = Vec::new();

            // Show how many lines are hidden
            if hidden_line_count > 0 {
                if self.expanded {
                    status_parts.push(format!(
                        "{}{}{}",
                        theme().fg("muted", "("),
                        key_hint("app.tools.expand", "to collapse"),
                        theme().fg("muted", ")")
                    ));
                } else {
                    status_parts.push(format!(
                        "{}... {} more lines ({}{}{}",
                        theme().fg("muted", ""),
                        hidden_line_count,
                        theme().fg("muted", "("),
                        key_hint("app.tools.expand", "to expand"),
                        theme().fg("muted", ")")
                    ));
                }
            }

            match self.status {
                BashStatus::Cancelled => {
                    status_parts.push(theme().fg("warning", "(cancelled)"));
                }
                BashStatus::Error => {
                    if let Some(code) = self.exit_code {
                        status_parts.push(theme().fg("error", &format!("(exit {})", code)));
                    }
                }
                BashStatus::Complete => {}
                BashStatus::Running => {}
            }

            // Truncation warning
            let was_truncated = self
                .truncation_result
                .as_ref()
                .map(|t| t.truncated)
                .unwrap_or(false)
                || context_truncation.truncated;

            if was_truncated {
                if let Some(ref path) = self.full_output_path {
                    status_parts.push(theme().fg(
                        "warning",
                        &format!("Output truncated. Full output: {}", path),
                    ));
                }
            }

            if !status_parts.is_empty() {
                self.content_container.add_child(Box::new(Text::new(
                    format!("\n{}", status_parts.join("\n")),
                    cards().tool_result_indent,
                    0,
                )));
            }
        }
    }
}

impl Component for BashExecutionComponent {
    fn render(&self, width: u16) -> Vec<String> {
        // Recompute fresh each time due to &self constraint
        let lines = self.container.render(width);
        lines
    }

    fn invalidate(&mut self) {
        self.container.invalidate();
        self.update_display();
    }
}

/// Truncate output from the tail (keep most recent lines).
fn truncate_tail(content: &str, max_lines: usize, max_bytes: usize) -> TruncationResult {
    let lines: Vec<&str> = content.split('\n').collect();
    let total_bytes = content.len();

    let mut result = content.to_string();
    let mut truncated = false;

    // Truncate by lines
    if lines.len() > max_lines {
        let start = lines.len() - max_lines;
        result = lines[start..].join("\n");
        truncated = true;
    }

    // Truncate by bytes
    if result.len() > max_bytes {
        // Find a valid UTF-8 boundary near max_bytes
        let mut byte_end = max_bytes;
        while byte_end > 0 && !result.is_char_boundary(byte_end) {
            byte_end -= 1;
        }
        result = result[..byte_end].to_string();
        truncated = true;
    }

    TruncationResult {
        truncated,
        content: result,
        original_byte_count: total_bytes,
        original_line_count: lines.len(),
    }
}

/// Strip ANSI escape sequences from a string.
fn strip_ansi(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut in_escape = false;
    for ch in s.chars() {
        if in_escape {
            if ch.is_ascii_alphabetic() && ch != '[' && ch != ';' {
                in_escape = false;
            }
        } else if ch == '\x1b' {
            in_escape = true;
        } else {
            result.push(ch);
        }
    }
    result
}

/// Clip a multi-line string to at most `max_lines` lines.
fn clip_lines(s: &str, max_lines: usize) -> String {
    let lines: Vec<&str> = s.lines().collect();
    if lines.len() <= max_lines {
        s.to_string()
    } else {
        lines[..max_lines].join("\n")
    }
}
