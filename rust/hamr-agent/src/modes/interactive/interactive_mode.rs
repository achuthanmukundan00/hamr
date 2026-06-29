//! Port of `packages/coding-agent/src/modes/interactive/interactive-mode.ts`.
//!
//! Interactive mode for the coding agent — full TUI with chat display and
//! keyboard input, delegating business logic to AgentSession.
//!
//! Architecture:
//! - `HamrChatWidget` — a single sexy_tui_rs::Component that renders the
//!   chat history + input prompt and handles all keyboard input.
//! - sexy_tui_rs::TUI handles differential rendering.
//! - crossterm EventStream (via spawn_blocking) feeds key events to the TUI.
//! - Submitted text flows to AgentSession::prompt() via a tokio channel.

use std::cell::{Cell, RefCell};
#[cfg(feature = "tui")]
use std::collections::HashMap;
#[cfg(feature = "tui")]
use std::io::Write;
use std::rc::Rc;

use crate::core::agent_session_runtime::AgentSessionRuntime;
#[cfg(feature = "tui")]
use crate::core::export_html::export::export_session_to_html;
use crate::core::output_guard::flush_raw_stdout;
#[cfg(feature = "tui")]
use crate::core::session_manager::find_most_recent_session;
#[cfg(feature = "tui")]
use crate::core::slash_commands::BUILTIN_SLASH_COMMANDS;
#[cfg(feature = "tui")]
use crate::modes::interactive::components::assistant_message;
#[cfg(feature = "tui")]
use crate::modes::interactive::components::bash_execution::{
    BashExecutionComponent, TruncationResult,
};
#[cfg(feature = "tui")]
use crate::modes::interactive::components::tool_execution::{
    ToolExecutionComponent, ToolExecutionOptions, ToolResult, ToolResultContentBlock,
};
#[cfg(feature = "tui")]
use crate::modes::interactive::components::tui_shim::Component as ShimComponent;
#[cfg(feature = "tui")]
use crate::modes::interactive::theme::theme::theme;
#[cfg(feature = "tui")]
use std::path::Path;

#[cfg(feature = "tui")]
enum TerminalInput {
    Key(String),
    Paste(String),
    Resize(u16, u16),
}

#[cfg(feature = "tui")]
async fn read_terminal_input() -> Option<TerminalInput> {
    tokio::task::spawn_blocking(|| {
        use crossterm::event::{self, Event, KeyEventKind};

        if !event::poll(std::time::Duration::from_millis(50)).unwrap_or(false) {
            return None;
        }

        match event::read() {
            Ok(Event::Key(key)) if key.kind != KeyEventKind::Release => Some(TerminalInput::Key(
                sexy_tui_rs::terminal::key_to_string(&key),
            )),
            Ok(Event::Paste(text)) => Some(TerminalInput::Paste(text)),
            Ok(Event::Resize(columns, rows)) => Some(TerminalInput::Resize(columns, rows)),
            _ => None,
        }
    })
    .await
    .unwrap_or(None)
}

#[cfg(feature = "tui")]
async fn receive_shutdown_signal(
    signal_rx: &mut Option<tokio::sync::mpsc::UnboundedReceiver<i32>>,
) -> i32 {
    if let Some(rx) = signal_rx {
        if let Some(exit_code) = rx.recv().await {
            return exit_code;
        }
    }
    std::future::pending::<i32>().await
}

#[cfg(feature = "tui")]
fn restore_terminal_state() {
    let mut stdout = std::io::stdout();
    let _ = crossterm::execute!(
        stdout,
        crossterm::cursor::Show,
        crossterm::style::Print("\x1b[?2004l")
    );
    let _ = stdout.flush();
    let _ = crossterm::terminal::disable_raw_mode();
}

#[cfg(feature = "tui")]
struct TerminalRestoreGuard {
    armed: bool,
}

#[cfg(feature = "tui")]
impl TerminalRestoreGuard {
    fn new() -> Self {
        Self { armed: false }
    }

    fn arm(&mut self) {
        self.armed = true;
    }

    fn disarm(&mut self) {
        self.armed = false;
    }
}

#[cfg(feature = "tui")]
impl Drop for TerminalRestoreGuard {
    fn drop(&mut self) {
        if self.armed {
            restore_terminal_state();
        }
    }
}

#[cfg(feature = "tui")]
fn is_exit_key(data: &str) -> bool {
    sexy_tui_rs::keys::matches_key(data, "ctrl+c") || sexy_tui_rs::keys::matches_key(data, "ctrl+d")
}

#[cfg(feature = "tui")]
fn is_interrupt_key(data: &str) -> bool {
    sexy_tui_rs::keys::matches_key(data, "escape") || sexy_tui_rs::keys::matches_key(data, "ctrl+c")
}

// ─── Options ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct InteractiveModeOptions {
    pub migrated_providers: Vec<String>,
    pub model_fallback_message: Option<String>,
    pub auto_trust_on_reload_cwd: Option<String>,
    pub initial_message: Option<String>,
    pub initial_images: Vec<()>,
    pub initial_messages: Vec<String>,
    pub verbose: bool,
}

impl Default for InteractiveModeOptions {
    fn default() -> Self {
        Self {
            migrated_providers: Vec::new(),
            model_fallback_message: None,
            auto_trust_on_reload_cwd: None,
            initial_message: None,
            initial_images: Vec::new(),
            initial_messages: Vec::new(),
            verbose: false,
        }
    }
}

// ─── HamrChatWidget: single component holding chat + input ───────────────────

/// A component managed by the chat widget.
#[cfg(feature = "tui")]
enum InteractiveComponent {
    Tool(RefCell<ToolExecutionComponent>),
    Bash(RefCell<BashExecutionComponent>),
}

#[cfg(feature = "tui")]
impl InteractiveComponent {
    fn render(&self, width: u16) -> Vec<String> {
        match self {
            InteractiveComponent::Tool(c) => c.borrow().render(width),
            InteractiveComponent::Bash(c) => c.borrow().render(width),
        }
    }
}

/// A slot tracking a component and its line range in the chat buffer.
#[cfg(feature = "tui")]
struct ComponentSlot {
    component: InteractiveComponent,
    start: usize,
    end: usize,
}

/// Shared state between the widget and the async event loop.
#[cfg(feature = "tui")]
struct ChatState {
    /// Rendered chat lines (already wrapped to terminal width).
    lines: RefCell<Vec<String>>,
    /// Mirror of the editor text for the async loop and regression tests.
    input: RefCell<String>,
    /// Channel to send submitted text.
    submit_tx: tokio::sync::mpsc::UnboundedSender<String>,
    /// Range occupied by the currently streaming assistant message.
    streaming_range: RefCell<Option<(usize, usize)>>,
    /// Active tool/Bash components with their line ranges.
    active_components: RefCell<HashMap<String, ComponentSlot>>,
    /// Footer data populated once the active model is known.
    footer: RefCell<Option<ChatFooter>>,
    working: Cell<bool>,
    /// Current terminal width (updated on resize).
    terminal_width: Cell<usize>,
    /// Model glyph for card headings.
    model_glyph: RefCell<String>,
    /// Model accent color hex.
    model_accent: RefCell<Option<String>>,
}

#[cfg(feature = "tui")]
struct ChatFooter {
    provider: String,
    model: String,
    model_glyph: String,
    thinking: String,
    context_window: u64,
    context_tokens: u64,
    total_input: u64,
    total_output: u64,
    total_cost: f64,
}

#[cfg(feature = "tui")]
impl ChatState {
    fn add_line(&self, line: &str) {
        self.lines.borrow_mut().push(line.to_string());
    }

    fn remove_working_line(lines: &mut Vec<String>) {
        if lines.last().is_some_and(|line| line.contains("Working...")) {
            lines.pop();
        }
    }

    fn replace_streaming_lines(&self, replacement: Vec<String>) {
        if replacement.is_empty() {
            return;
        }

        let mut lines = self.lines.borrow_mut();
        Self::remove_working_line(&mut lines);
        let start = if let Some((start, end)) = *self.streaming_range.borrow() {
            let end = end.min(lines.len());
            lines.splice(start..end, replacement.iter().cloned());
            start
        } else {
            let start = lines.len();
            lines.extend(replacement.iter().cloned());
            start
        };
        *self.streaming_range.borrow_mut() = Some((start, start + replacement.len()));
    }

    fn finish_streaming_message(&self) {
        self.streaming_range.borrow_mut().take();
    }

    fn start_tool(&self, tool_call_id: &str, tool_name: &str, args: &serde_json::Value) {
        self.finish_streaming_message();
        let args_json = serde_json::to_string(args).unwrap_or_else(|_| "{}".to_string());
        let opts = ToolExecutionOptions {
            model_glyph: Some(self.model_glyph.borrow().clone()),
            model_accent: self.model_accent.borrow().clone(),
            ..Default::default()
        };
        let component = ToolExecutionComponent::new(tool_name, tool_call_id, &args_json, opts);
        let rendered = component.render(self.terminal_width.get() as u16);

        let mut lines = self.lines.borrow_mut();
        Self::remove_working_line(&mut lines);
        let start = lines.len();
        lines.extend(rendered);
        let end = lines.len();
        drop(lines);

        self.active_components.borrow_mut().insert(
            tool_call_id.to_string(),
            ComponentSlot {
                component: InteractiveComponent::Tool(RefCell::new(component)),
                start,
                end,
            },
        );
    }

    fn finish_tool(
        &self,
        tool_call_id: &str,
        _tool_name: &str,
        result: &serde_json::Value,
        is_error: bool,
    ) {
        let Some(slot) = self.active_components.borrow_mut().remove(tool_call_id) else {
            return;
        };

        let InteractiveComponent::Tool(ref component) = slot.component else {
            return;
        };

        let tool_result = result_to_tool_result(result, is_error);
        component.borrow_mut().update_result(tool_result, false);
        component.borrow_mut().set_args_complete();
        component.borrow_mut().mark_execution_started();

        let rendered = component.borrow().render(self.terminal_width.get() as u16);

        let mut lines = self.lines.borrow_mut();
        lines.splice(slot.start..slot.end, rendered);
    }

    fn set_footer(&self, model: &hamr_ai::types::Model, thinking: String, model_glyph: String) {
        *self.footer.borrow_mut() = Some(ChatFooter {
            provider: model.provider.clone(),
            model: model.name.clone(),
            model_glyph,
            thinking,
            context_window: model.context_window,
            context_tokens: 0,
            total_input: 0,
            total_output: 0,
            total_cost: 0.0,
        });
    }

    fn record_usage(&self, message: &hamr_ai::types::AssistantMessage) {
        let Some(ref mut footer) = *self.footer.borrow_mut() else {
            return;
        };
        footer.context_tokens = message
            .usage
            .input
            .saturating_add(message.usage.cache_read)
            .saturating_add(message.usage.cache_write);
        footer.total_input = footer.total_input.saturating_add(message.usage.input);
        footer.total_output = footer.total_output.saturating_add(message.usage.output);
        footer.total_cost += message.usage.cost.total;
    }
}

/// A Component that renders the chat log and an input prompt,
/// and processes keyboard input for the editor.
#[cfg(feature = "tui")]
struct HamrChatWidget {
    state: Rc<ChatState>,
    editor: sexy_tui_rs::Editor,
}

#[cfg(feature = "tui")]
impl HamrChatWidget {
    fn new(submit_tx: tokio::sync::mpsc::UnboundedSender<String>) -> Self {
        use sexy_tui_rs::Focusable;

        let state = Rc::new(ChatState {
            lines: RefCell::new(Vec::new()),
            input: RefCell::new(String::new()),
            submit_tx,
            streaming_range: RefCell::new(None),
            active_components: RefCell::new(HashMap::new()),
            footer: RefCell::new(None),
            working: Cell::new(false),
            terminal_width: Cell::new(80),
            model_glyph: RefCell::new(String::new()),
            model_accent: RefCell::new(None),
        });

        let mut editor = sexy_tui_rs::Editor::new(
            sexy_tui_rs::EditorTheme {
                border_color: Box::new(|text| {
                    crate::modes::interactive::theme::theme::theme().fg("borderMuted", text)
                }),
                prompt_color: Box::new(|text| {
                    crate::modes::interactive::theme::theme::theme().fg("muted", text)
                }),
            },
            sexy_tui_rs::EditorOptions {
                prompt_prefix: Some("> ".to_string()),
                ..Default::default()
            },
        );
        editor.set_focused(true);

        let change_state = Rc::clone(&state);
        editor.on_change = Some(Box::new(move |text| {
            *change_state.input.borrow_mut() = text.to_string();
        }));

        let submit_state = Rc::clone(&state);
        editor.on_submit = Some(Box::new(move |text| {
            if text.is_empty() {
                return;
            }

            for (index, line) in text.lines().enumerate() {
                if index == 0 {
                    submit_state.add_line(&format!("\x1b[1mYou:\x1b[0m {line}"));
                } else {
                    submit_state.add_line(&format!("     {line}"));
                }
            }
            let _ = submit_state.submit_tx.send(text.to_string());
        }));

        // Set up autocomplete provider with built-in slash commands
        {
            use sexy_tui_rs::autocomplete::{CombinedAutocompleteProvider, SlashCommand};
            let commands: Vec<SlashCommand> = BUILTIN_SLASH_COMMANDS
                .iter()
                .map(|cmd| {
                    let mut sc = SlashCommand::new(cmd.name);
                    sc.description = Some(cmd.description.to_string());
                    sc
                })
                .collect();
            let cwd = std::env::current_dir()
                .ok()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|| ".".to_string());
            let provider = CombinedAutocompleteProvider::new(commands, cwd, None);
            editor.set_autocomplete_provider(Box::new(provider));
        }

        Self { state, editor }
    }

    /// Return a clone of the state handle so the async loop can push lines.
    fn state_handle(&self) -> Rc<ChatState> {
        Rc::clone(&self.state)
    }
}

#[cfg(feature = "tui")]
impl sexy_tui_rs::Component for HamrChatWidget {
    fn render(&self, width: u16) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();

        // ── Chat history ──────────────────────────────────────────────
        let lines = self.state.lines.borrow();
        // Show last N lines that fit above the input area.
        {
            if lines.len() > 50 {
                let skip = lines.len() - 50;
                out.push(format!("\x1b[90m  ... {} more lines ...\x1b[0m", skip));
                for line in &lines[skip..] {
                    out.push(line.clone());
                }
            } else {
                out.extend(lines.iter().cloned());
            }
        }

        // ── Spacer ────────────────────────────────────────────────────
        out.push(String::new());

        out.extend(self.editor.render(width));

        if let Some(footer) = self.state.footer.borrow().as_ref() {
            let theme = crate::modes::interactive::theme::theme::theme();
            let left = if self.state.working.get() {
                theme.fg("accent", "Working...")
            } else {
                theme.fg("dim", "Idle")
            };
            let context = if footer.context_window == 0 {
                "?".to_string()
            } else {
                format!(
                    "{:.1}%",
                    footer.context_tokens as f64 / footer.context_window as f64 * 100.0
                )
            };
            let right = theme.fg(
                "dim",
                &format!(
                    "{context}/{}  ${:.3}  ↑ {} ↓ {}  ({}) {} {} {}",
                    format_footer_tokens(footer.context_window),
                    footer.total_cost,
                    format_footer_tokens(footer.total_input),
                    format_footer_tokens(footer.total_output),
                    footer.provider,
                    footer.model_glyph,
                    footer.model,
                    footer.thinking
                ),
            );
            let left_width = sexy_tui_rs::visible_width(&left);
            let right_width = sexy_tui_rs::visible_width(&right);
            let footer_line = if left_width + right_width + 1 <= width as usize {
                format!(
                    "{left}{}{right}",
                    " ".repeat(width as usize - left_width - right_width)
                )
            } else {
                sexy_tui_rs::truncate_to_width(&right, width as usize, Some("…"))
            };
            out.push(footer_line);
        }

        out
    }

    fn handle_input(&mut self, data: &str) {
        use sexy_tui_rs::keys::matches_key;

        // ── Ctrl+C / Ctrl+D — ignored here (handled at top level) ──
        if matches_key(data, "ctrl+c") || matches_key(data, "ctrl+d") {
            return;
        }

        let submitted = if matches_key(data, "enter") {
            Some(self.editor.get_text().trim().to_string())
        } else {
            None
        };
        self.editor.handle_input(data);
        if let Some(text) = submitted.filter(|text| !text.is_empty()) {
            self.editor.add_to_history(&text);
        }
    }

    fn invalidate(&mut self) {
        self.editor.invalidate();
    }

    fn wants_key_release(&self) -> bool {
        self.editor.wants_key_release()
    }
}

#[cfg(feature = "tui")]
fn format_footer_tokens(count: u64) -> String {
    match count {
        0..=999 => count.to_string(),
        1_000..=9_999 => format!("{:.1}K", count as f64 / 1_000.0),
        10_000..=999_999 => format!("{}K", count / 1_000),
        _ => format!("{:.1}M", count as f64 / 1_000_000.0),
    }
}

#[cfg(feature = "tui")]
fn wrap_chat_text(
    text: &str,
    prefix: &str,
    continuation_prefix: &str,
    max_width: usize,
) -> Vec<String> {
    let prefix_visible = sexy_tui_rs::visible_width(prefix);
    let effective_width = max_width.saturating_sub(prefix_visible).max(1);
    let mut output = Vec::new();

    for paragraph in text.split('\n') {
        if paragraph.is_empty() {
            output.push(prefix.to_string());
            continue;
        }

        let mut current = String::new();
        let mut current_width = 0usize;
        let mut first = true;
        for ch in paragraph.chars() {
            let char_width = sexy_tui_rs::visible_width(&ch.to_string()).max(1);
            if current_width > 0 && current_width + char_width > effective_width {
                let line_prefix = if first { prefix } else { continuation_prefix };
                output.push(format!("{line_prefix}{current}"));
                current.clear();
                current_width = 0;
                first = false;
            }
            current.push(ch);
            current_width += char_width;
        }
        let line_prefix = if first { prefix } else { continuation_prefix };
        output.push(format!("{line_prefix}{current}"));
    }

    output
}

#[cfg(feature = "tui")]
fn convert_to_component_message(
    msg: &hamr_ai::types::AssistantMessage,
) -> Option<assistant_message::AssistantMessage> {
    let content: Vec<assistant_message::ContentBlock> = msg
        .content
        .iter()
        .filter_map(|block| match block {
            hamr_ai::types::AssistantContentBlock::Text(t) if !t.text.is_empty() => {
                Some(assistant_message::ContentBlock::text(&t.text))
            }
            hamr_ai::types::AssistantContentBlock::Thinking(tc)
                if !tc.thinking.is_empty() && !tc.redacted =>
            {
                Some(assistant_message::ContentBlock::thinking(&tc.thinking))
            }
            hamr_ai::types::AssistantContentBlock::ToolCall(tc) => Some(
                assistant_message::ContentBlock::tool_call(&tc.id, &tc.name, tc.arguments.clone()),
            ),
            _ => None,
        })
        .collect();

    let stop_reason = match msg.stop_reason {
        hamr_ai::types::StopReason::Stop => Some(assistant_message::StopReason::EndTurn),
        hamr_ai::types::StopReason::Length => Some(assistant_message::StopReason::MaxTokens),
        hamr_ai::types::StopReason::ToolUse => Some(assistant_message::StopReason::ToolUse),
        hamr_ai::types::StopReason::Error => Some(assistant_message::StopReason::Error),
        hamr_ai::types::StopReason::Aborted => Some(assistant_message::StopReason::Aborted),
    };

    if content.is_empty()
        && !matches!(
            stop_reason,
            Some(assistant_message::StopReason::Error | assistant_message::StopReason::Aborted)
        )
    {
        return None;
    }

    Some(assistant_message::AssistantMessage {
        content,
        model: msg.model.clone(),
        stop_reason,
        error_message: msg.error_message.clone(),
    })
}

#[cfg(feature = "tui")]
fn format_assistant_lines(
    message: &hamr_ai::types::AssistantMessage,
    max_width: usize,
    model_glyph: &str,
    model_accent: Option<&str>,
) -> Vec<String> {
    let mut lines = Vec::new();

    // Try component rendering first
    if let Some(component_msg) = convert_to_component_message(message) {
        let hide_thinking = true;
        let comp = assistant_message::AssistantMessageComponent::new(
            Some(component_msg),
            hide_thinking,
            None,
            None,
            model_accent.map(|s| s.to_string()),
            (!model_glyph.is_empty()).then(|| model_glyph.to_string()),
        );
        let rendered = comp.render(max_width as u16);
        if !rendered.is_empty() {
            // Prepend card heading line: model glyph + name in accent color
            if !model_glyph.is_empty() {
                let heading_color = match model_accent {
                    Some(hex) => assistant_message::hex_to_ansi_fg(hex),
                    None => theme().model_color(&message.provider, Some(&message.model)),
                };
                lines.push(format!(
                    "{}  {}{} {}",
                    heading_color,
                    theme().bold(""),
                    model_glyph,
                    message.model
                ));
            }
            lines.extend(rendered);
            return lines;
        }
    }

    // Fallback to existing text formatting
    let t = theme();
    for block in &message.content {
        match block {
            hamr_ai::types::AssistantContentBlock::Text(text) if !text.text.is_empty() => {
                lines.extend(wrap_chat_text(&text.text, "  ", "  ", max_width));
            }
            hamr_ai::types::AssistantContentBlock::Thinking(thinking)
                if !thinking.thinking.is_empty() && !thinking.redacted =>
            {
                let thinking_lines = wrap_chat_text(&thinking.thinking, "  ", "  ", max_width);
                lines.extend(thinking_lines.into_iter().map(|line| t.fg("dim", &line)));
            }
            hamr_ai::types::AssistantContentBlock::ToolCall(tc) => {
                let args =
                    serde_json::to_string(&tc.arguments).unwrap_or_else(|_| "{}".to_string());
                lines.push(t.fg("dim", &format!("  {} {}", tc.name, args)));
            }
            _ => {}
        }
    }

    // Prepend heading for fallback text output too
    if !lines.is_empty() && !model_glyph.is_empty() {
        let heading_color = match model_accent {
            Some(hex) => assistant_message::hex_to_ansi_fg(hex),
            None => theme().model_color(&message.provider, Some(&message.model)),
        };
        lines.insert(
            0,
            format!(
                "{}  {}{} {}",
                heading_color,
                theme().bold(""),
                model_glyph,
                message.model
            ),
        );
    }

    lines
}

#[cfg(feature = "tui")]
fn format_restored_history(
    messages: &[hamr_harness::types::AgentMessage],
    max_width: usize,
) -> (Vec<String>, Vec<hamr_ai::types::AssistantMessage>) {
    use hamr_harness::types::AgentMessage;

    let mut lines = Vec::new();
    let mut assistant_messages = Vec::new();
    for message in messages {
        match message {
            AgentMessage::User(message) => {
                for block in &message.content {
                    if let hamr_ai::types::MessageContent::Text(text) = block {
                        lines.extend(wrap_chat_text(
                            &text.text,
                            "\x1b[1mYou:\x1b[0m ",
                            "     ",
                            max_width,
                        ));
                    }
                }
            }
            AgentMessage::Assistant(message) => {
                let glyph = theme().model_glyph(&message.provider, Some(&message.model));
                let accent = theme().model_hex_color(&message.provider, Some(&message.model));
                lines.extend(format_assistant_lines(
                    message,
                    max_width,
                    &glyph,
                    accent.as_deref(),
                ));
                assistant_messages.push(message.clone());
            }
            AgentMessage::ToolResult(message) => {
                let text = message
                    .content
                    .iter()
                    .filter_map(|block| match block {
                        hamr_ai::types::MessageContent::Text(text) => Some(text.text.as_str()),
                        _ => None,
                    })
                    .collect::<Vec<_>>()
                    .join("\n");
                if !text.is_empty() {
                    let color = if message.is_error { 31 } else { 90 };
                    lines.extend(
                        wrap_chat_text(
                            &text,
                            &format!("\x1b[{color}m[{}] ", message.tool_name),
                            "  ",
                            max_width,
                        )
                        .into_iter()
                        .map(|line| format!("{line}\x1b[0m")),
                    );
                }
            }
            AgentMessage::BranchSummary(message) => {
                lines.extend(
                    wrap_chat_text(
                        &message.summary,
                        "\x1b[90m[branch summary] ",
                        "                 ",
                        max_width,
                    )
                    .into_iter()
                    .map(|line| format!("{line}\x1b[0m")),
                );
            }
            AgentMessage::CompactionSummary(message) => {
                lines.extend(
                    wrap_chat_text(
                        &message.summary,
                        "\x1b[90m[compaction] ",
                        "             ",
                        max_width,
                    )
                    .into_iter()
                    .map(|line| format!("{line}\x1b[0m")),
                );
            }
            AgentMessage::BashExecution(bash) => {
                let mut component =
                    BashExecutionComponent::new(&bash.command, bash.exclude_from_context);
                // Feed all output at once
                component.append_output(&bash.output);

                let truncation = if bash.truncated {
                    Some(TruncationResult {
                        truncated: true,
                        content: bash.output.clone(),
                        original_byte_count: bash.output.len(),
                        original_line_count: bash.output.lines().count(),
                    })
                } else {
                    None
                };

                component.set_complete(
                    bash.exit_code,
                    bash.cancelled,
                    truncation,
                    bash.full_output_path.clone(),
                );

                let rendered = component.render(max_width as u16);
                lines.extend(rendered);
            }
            AgentMessage::Custom(_) => {}
        }
    }
    (lines, assistant_messages)
}

#[cfg(feature = "tui")]
fn result_to_tool_result(result: &serde_json::Value, is_error: bool) -> ToolResult {
    let content = result
        .get("content")
        .and_then(|c| c.as_array())
        .map(|blocks| {
            blocks
                .iter()
                .map(|block| ToolResultContentBlock {
                    block_type: block
                        .get("type")
                        .and_then(|t| t.as_str())
                        .unwrap_or("text")
                        .to_string(),
                    text: block.get("text").and_then(|t| t.as_str()).map(String::from),
                    data: block.get("data").and_then(|t| t.as_str()).map(String::from),
                    mime_type: block
                        .get("mime_type")
                        .and_then(|t| t.as_str())
                        .map(String::from),
                })
                .collect()
        })
        .unwrap_or_default();
    ToolResult {
        content,
        is_error,
        details: None,
    }
}

#[cfg(feature = "tui")]
fn apply_agent_event(state: &ChatState, event: hamr_harness::types::AgentEvent) -> bool {
    use hamr_harness::types::{AgentEvent, AgentMessage};

    match event {
        AgentEvent::MessageUpdate {
            message: AgentMessage::Assistant(message),
            ..
        } => {
            let glyph = state.model_glyph.borrow();
            let accent = state.model_accent.borrow();
            let lines = format_assistant_lines(
                &message,
                state.terminal_width.get(),
                &glyph,
                accent.as_deref(),
            );
            let rendered = !lines.is_empty();
            state.replace_streaming_lines(lines);
            rendered
        }
        AgentEvent::MessageEnd {
            message: AgentMessage::Assistant(message),
        } => {
            state.record_usage(&message);
            let glyph = state.model_glyph.borrow();
            let accent = state.model_accent.borrow();
            let lines = format_assistant_lines(
                &message,
                state.terminal_width.get(),
                &glyph,
                accent.as_deref(),
            );
            let rendered = !lines.is_empty();
            state.replace_streaming_lines(lines);
            state.finish_streaming_message();
            rendered
        }
        AgentEvent::ToolExecutionStart {
            tool_call_id,
            tool_name,
            args,
        } => {
            state.start_tool(&tool_call_id, &tool_name, &args);
            false
        }
        AgentEvent::ToolExecutionUpdate {
            tool_call_id,
            tool_name: _,
            args,
            partial_result,
        } => {
            // Capture what we need from active_components first
            let (start, end, rendered) = {
                let components = state.active_components.borrow();
                let Some(slot) = components.get(&tool_call_id) else {
                    return false;
                };
                if let InteractiveComponent::Tool(component) = &slot.component {
                    let args_json =
                        serde_json::to_string(&args).unwrap_or_else(|_| "{}".to_string());
                    component.borrow_mut().update_args(&args_json);

                    if !partial_result.is_null() {
                        let pr = result_to_tool_result(&partial_result, false);
                        component.borrow_mut().update_result(pr, true);
                    }

                    (
                        slot.start,
                        slot.end,
                        component.borrow().render(state.terminal_width.get() as u16),
                    )
                } else {
                    return false;
                }
            };

            let new_end = start + rendered.len();
            let mut lines = state.lines.borrow_mut();
            lines.splice(start..end, rendered);
            drop(lines);

            if let Some(slot) = state.active_components.borrow_mut().get_mut(&tool_call_id) {
                slot.end = new_end;
            }
            false
        }
        AgentEvent::ToolExecutionEnd {
            tool_call_id,
            tool_name,
            result,
            is_error,
        } => {
            state.finish_tool(&tool_call_id, &tool_name, &result, is_error);
            false
        }
        AgentEvent::ModelLoading { model, elapsed_ms } => {
            let mut lines = state.lines.borrow_mut();
            ChatState::remove_working_line(&mut lines);
            lines.push(format!(
                "\x1b[90mLoading {model} ({:.1}s)...\x1b[0m",
                elapsed_ms as f64 / 1000.0
            ));
            false
        }
        AgentEvent::AgentEnd { .. } => {
            state.finish_streaming_message();
            false
        }
        AgentEvent::CompactionStart { reason } => {
            let mut lines = state.lines.borrow_mut();
            ChatState::remove_working_line(&mut lines);
            lines.push(format!("\x1b[90mCompacting context ({reason})...\x1b[0m"));
            false
        }
        AgentEvent::CompactionEnd {
            aborted,
            reason,
            result,
        } => {
            let mut lines = state.lines.borrow_mut();
            ChatState::remove_working_line(&mut lines);
            if aborted {
                lines.push(format!("\x1b[91mCompaction failed: {reason}\x1b[0m"));
            } else if let Some(r) = result {
                lines.push(format!(
                    "\x1b[90mCompacted context: {:.0}k → {:.0}k tokens\x1b[0m",
                    r.tokens_before as f64 / 1000.0,
                    r.tokens_after as f64 / 1000.0
                ));
            }
            false
        }
        AgentEvent::CompactionSummary { summary, .. } => {
            state.add_line(&format!("\x1b[2m── Compaction Summary ──\x1b[0m"));
            for line in summary.lines() {
                state.add_line(&format!("\x1b[2m{line}\x1b[0m"));
            }
            false
        }
        _ => {
            tracing::debug!("unhandled agent event: {:?}", event);
            false
        }
    }
}

// ─── InteractiveMode ──────────────────────────────────────────────────────────

pub struct InteractiveMode {
    runtime_host: AgentSessionRuntime,
    options: InteractiveModeOptions,
    version: String,
    is_initialized: bool,

    // ── TUI (feature-gated) ──────────────────────────────────────────────
    #[cfg(feature = "tui")]
    tui: sexy_tui_rs::TUI,
    #[cfg(feature = "tui")]
    chat_state: Option<Rc<ChatState>>,
    #[cfg(feature = "tui")]
    submit_rx: Option<tokio::sync::mpsc::UnboundedReceiver<String>>,
    #[cfg(feature = "tui")]
    agent_event_tx: Option<tokio::sync::mpsc::UnboundedSender<hamr_harness::types::AgentEvent>>,
    #[cfg(feature = "tui")]
    agent_event_rx: Option<tokio::sync::mpsc::UnboundedReceiver<hamr_harness::types::AgentEvent>>,
    #[cfg(feature = "tui")]
    signal_rx: Option<tokio::sync::mpsc::UnboundedReceiver<i32>>,
    #[cfg(feature = "tui")]
    terminal_restore_guard: TerminalRestoreGuard,

    // ── State ──
    shutdown_requested: bool,
    exit_code: i32,
    signal_cleanup_handlers: Vec<Box<dyn FnOnce() + Send>>,
}

impl InteractiveMode {
    // ── Constructor ────────────────────────────────────────────────────────

    #[cfg(feature = "tui")]
    pub fn new(runtime_host: AgentSessionRuntime, options: InteractiveModeOptions) -> Self {
        let (submit_tx, submit_rx) = tokio::sync::mpsc::unbounded_channel::<String>();
        let (agent_event_tx, agent_event_rx) =
            tokio::sync::mpsc::unbounded_channel::<hamr_harness::types::AgentEvent>();

        let terminal: Box<dyn sexy_tui_rs::Terminal> = Box::new(
            sexy_tui_rs::ProcessTerminal::new().expect("Failed to create ProcessTerminal"),
        );
        let mut tui = sexy_tui_rs::TUI::new(terminal);

        let widget = HamrChatWidget::new(submit_tx);
        let chat_state = Some(widget.state_handle());
        tui.add_child(Box::new(widget));
        // TUI routes input only to the focused child. The editor is child 0.
        tui.set_focus(Some(0));

        Self {
            runtime_host,
            options,
            version: env!("CARGO_PKG_VERSION").to_string(),
            is_initialized: false,
            tui,
            chat_state,
            submit_rx: Some(submit_rx),
            agent_event_tx: Some(agent_event_tx),
            agent_event_rx: Some(agent_event_rx),
            signal_rx: None,
            terminal_restore_guard: TerminalRestoreGuard::new(),
            shutdown_requested: false,
            exit_code: 0,
            signal_cleanup_handlers: Vec::new(),
        }
    }

    #[cfg(not(feature = "tui"))]
    pub fn new(runtime_host: AgentSessionRuntime, options: InteractiveModeOptions) -> Self {
        Self {
            runtime_host,
            options,
            version: env!("CARGO_PKG_VERSION").to_string(),
            is_initialized: false,
            shutdown_requested: false,
            exit_code: 0,
            signal_cleanup_handlers: Vec::new(),
        }
    }

    // ── init() ──────────────────────────────────────────────────────────────

    #[cfg(feature = "tui")]
    pub async fn init(&mut self) -> Result<(), String> {
        if self.is_initialized {
            return Ok(());
        }

        crossterm::terminal::enable_raw_mode().map_err(|e| {
            format!(
                "Terminal setup failed: {}. Is stdin a TTY? Use --print for non-interactive mode.",
                e
            )
        })?;
        self.terminal_restore_guard.arm();
        let _ = crossterm::execute!(std::io::stdout(), crossterm::style::Print("\x1b[?2004h"));

        self.register_signal_handlers();
        if let Some(agent_event_tx) = self.agent_event_tx.take() {
            self.runtime_host
                .session()
                .subscribe(move |event| {
                    let agent_event_tx = agent_event_tx.clone();
                    Box::pin(async move {
                        let _ = agent_event_tx.send(event);
                    })
                })
                .await;
        }
        self.render_restored_history().await;
        self.render_splash().await;
        self.tui.start();
        self.is_initialized = true;

        if !self.options.migrated_providers.is_empty() {
            eprintln!(
                "Migrated credentials to auth.json: {}",
                self.options.migrated_providers.join(", ")
            );
        }
        if let Some(ref msg) = self.options.model_fallback_message {
            eprintln!("Model fallback: {}", msg);
        }

        Ok(())
    }

    #[cfg(feature = "tui")]
    async fn render_splash(&mut self) {
        let Some(ref chat_state) = self.chat_state else {
            return;
        };

        // Always seed model glyph + accent so live streaming can use them.
        if let Some(model) = self.runtime_host.session().model().cloned() {
            *chat_state.model_glyph.borrow_mut() =
                theme().model_glyph(&model.provider, Some(&model.name));
            *chat_state.model_accent.borrow_mut() =
                theme().model_hex_color(&model.provider, Some(&model.name));
        }

        if !chat_state.lines.borrow().is_empty() {
            return;
        }

        let Some(model) = self.runtime_host.session().model().cloned() else {
            return;
        };
        let context_paths: Vec<String> = self
            .runtime_host
            .session()
            .context_files()
            .iter()
            .map(|file| file.path.clone())
            .collect();
        let skill_names: Vec<String> = self
            .runtime_host
            .session()
            .skills()
            .iter()
            .map(|skill| skill.name.clone())
            .collect();
        let agent_state = self.runtime_host.session().state().await;
        let thinking = format!("{:?}", agent_state.thinking_level).to_lowercase();
        let theme = crate::modes::interactive::theme::theme::theme();
        let model_color = theme.model_color(&model.provider, Some(&model.name));
        let model_glyph = theme.model_glyph(&model.provider, Some(&model.name));
        chat_state.set_footer(&model, thinking.clone(), model_glyph.clone());
        let brand = |text: &str| format!("{model_color}{text}\x1b[39m");
        let dim = |text: &str| theme.fg("dim", text);
        let muted = |text: &str| theme.fg("muted", text);
        let accent = |text: &str| theme.fg("accent", text);
        let kv =
            |label: &str, value: String| format!(" {} {}", dim(&format!("{label:<12}")), value);

        chat_state.add_line(&format!(
            " {}",
            theme.bold(&format!(
                "{}  {}",
                brand("⚒ hamr"),
                dim(&format!("v{}", self.version))
            ))
        ));
        chat_state.add_line("");
        chat_state.add_line(&format!(" {}", theme.bold(&brand("── SESSION ──"))));
        chat_state.add_line("");
        chat_state.add_line(&kv(
            "Model",
            brand(&format!("{model_glyph} {}", model.name)),
        ));
        chat_state.add_line(&kv("Provider", accent(&model.provider)));
        chat_state.add_line(&kv("Thinking", muted(&thinking)));
        chat_state.add_line(&kv("Version", dim(&format!("v{}", self.version))));
        if !model.base_url.is_empty() {
            chat_state.add_line(&kv("Endpoint", muted(&model.base_url)));
        }

        if !context_paths.is_empty() {
            chat_state.add_line("");
            chat_state.add_line(&format!(" {}", theme.bold(&brand("── CONTEXT ──"))));
            for path in context_paths {
                chat_state.add_line(&format!("  {}", muted(&path)));
            }
        }

        if !skill_names.is_empty() {
            chat_state.add_line("");
            chat_state.add_line(&format!(" {}", theme.bold(&brand("── SKILLS ──"))));
            if skill_names.len() <= 5 {
                for name in skill_names {
                    chat_state.add_line(&format!("  {}", muted(&name)));
                }
            } else {
                chat_state.add_line(&format!(
                    "  {}",
                    muted(&format!("{} skills loaded", skill_names.len()))
                ));
            }
        }
        chat_state.add_line("");
    }

    #[cfg(feature = "tui")]
    async fn render_restored_history(&mut self) {
        let Some(ref chat_state) = self.chat_state else {
            return;
        };
        let agent_state = self.runtime_host.session().state().await;
        let (lines, assistant_messages) =
            format_restored_history(&agent_state.messages, chat_state.terminal_width.get());
        for line in lines {
            chat_state.add_line(&line);
        }
        for message in assistant_messages {
            chat_state.record_usage(&message);
        }
    }

    #[cfg(not(feature = "tui"))]
    pub async fn init(&mut self) -> Result<(), String> {
        if self.is_initialized {
            return Ok(());
        }
        self.is_initialized = true;
        Ok(())
    }

    // ── run() ───────────────────────────────────────────────────────────────

    #[cfg(feature = "tui")]
    pub async fn run(&mut self) -> Result<(), String> {
        self.init().await?;

        // Process initial messages
        if let Some(ref msg) = self.options.initial_message.clone() {
            self.process_prompt(msg).await;
        }
        for msg in std::mem::take(&mut self.options.initial_messages) {
            if self.shutdown_requested {
                break;
            }
            self.process_prompt(&msg).await;
        }

        let mut submit_rx = self.submit_rx.take().expect("submit_rx missing");

        while !self.shutdown_requested {
            // ── 1. Read keyboard input ──────────────────────────────────
            let terminal_input = tokio::select! {
                input = read_terminal_input() => input,
                exit_code = receive_shutdown_signal(&mut self.signal_rx) => {
                    self.exit_code = exit_code;
                    self.shutdown_requested = true;
                    None
                }
            };

            if !self.shutdown_requested {
                if let Some(input) = terminal_input {
                    match input {
                        TerminalInput::Resize(columns, rows) => {
                            self.tui.resize(columns, rows);
                            if let Some(ref state) = self.chat_state {
                                state.terminal_width.set(columns as usize);
                            }
                        }
                        TerminalInput::Key(data) => {
                            if is_exit_key(&data) {
                                self.shutdown_requested = true;
                                break;
                            }
                            self.tui.handle_input(&data);
                        }
                        TerminalInput::Paste(text) => {
                            self.tui.handle_input(&format!("\x1b[200~{text}\x1b[201~"));
                        }
                    }
                }
            }

            // ── 2. Check for submitted text ────────────────────────────
            match submit_rx.try_recv() {
                Ok(text) => {
                    let trimmed = text.trim().to_string();
                    if self.dispatch_slash_command(&trimmed).await {
                        continue;
                    }
                    self.process_prompt(&text).await;
                }
                Err(tokio::sync::mpsc::error::TryRecvError::Disconnected) => {
                    break;
                }
                Err(tokio::sync::mpsc::error::TryRecvError::Empty) => {}
            }
        }

        self.shutdown().await;
        Ok(())
    }

    #[cfg(not(feature = "tui"))]
    pub async fn run(&mut self) -> Result<(), String> {
        self.init().await?;
        if let Some(ref msg) = self.options.initial_message.clone() {
            self.process_prompt(msg).await;
        }
        for msg in std::mem::take(&mut self.options.initial_messages) {
            if self.shutdown_requested {
                break;
            }
            self.process_prompt(&msg).await;
        }
        eprintln!("hamr: interactive TUI mode requires the `tui` feature.");
        eprintln!("Recompile with `--features tui` or use `--print`.");
        Ok(())
    }

    // ── Slash command dispatch ──────────────────────────────────────────────

    /// Dispatch a slash command. Returns `true` if the text was handled as a
    /// slash command (don't send to LLM), `false` if it should be sent normally.
    #[cfg(feature = "tui")]
    async fn dispatch_slash_command(&mut self, text: &str) -> bool {
        let command = text.trim();
        if !command.starts_with('/') {
            return false;
        }

        // Split into command name and arguments
        let (cmd_name, _args) = match command.split_once(' ') {
            Some((name, args)) => (name, args),
            None => (command, ""),
        };

        let cmd_name = cmd_name.to_lowercase();

        // ── Commands that don't need the LLM ──
        match cmd_name.as_str() {
            "/exit" | "/quit" => {
                self.shutdown_requested = true;
                return true;
            }
            "/help" | "/?" | "/" => {
                if let Some(ref state) = self.chat_state {
                    state.add_line("\x1b[1m── Available Slash Commands ──\x1b[0m");
                    for cmd in BUILTIN_SLASH_COMMANDS.iter() {
                        state.add_line(&format!(
                            "  \x1b[36m/{}\x1b[0m  \x1b[90m{}\x1b[0m",
                            cmd.name, cmd.description
                        ));
                    }
                    state.add_line("");
                    state.add_line(
                        "\x1b[90m  Type /<command> to use. Use /help <command> for details.\x1b[0m",
                    );
                }
                self.tui.request_render();
                return true;
            }
            "/model" => {
                if let Some(ref state) = self.chat_state {
                    state.add_line(&format!("\x1b[1m── Model ──\x1b[0m"));
                    if let Some(model) = self.runtime_host.session().model() {
                        state.add_line(&format!("  Current: \x1b[1m{}\x1b[0m", model.name));
                        state.add_line(&format!("  Provider: {}", model.provider));
                        state.add_line(&format!(
                            "  Context window: {} tokens",
                            model.context_window
                        ));
                    } else {
                        state.add_line("  \x1b[90mNo model loaded yet.\x1b[0m");
                    }
                    state.add_line("");
                    state.add_line(
                        "\x1b[90m  Full model selector with search/arrow keys requires\x1b[0m",
                    );
                    state.add_line("\x1b[90m  TUI overlay mode (planned feature).\x1b[0m");
                }
                self.tui.request_render();
                return true;
            }
            "/session" => {
                if let Some(ref state) = self.chat_state {
                    let agent_state = self.runtime_host.session().state().await;
                    state.add_line(&format!("\x1b[1m── Session ──\x1b[0m"));
                    state.add_line(&format!("  Version: hamr v{}", self.version));
                    state.add_line(&format!("  CWD: {}", self.runtime_host.services().cwd));
                    state.add_line(&format!("  Messages: {}", agent_state.messages.len()));
                    if let Some(ref path) = self
                        .runtime_host
                        .session()
                        .session_manager()
                        .get_session_file()
                    {
                        state.add_line(&format!("  File: {}", path));
                    }
                    let session_id = self
                        .runtime_host
                        .session()
                        .session_manager()
                        .get_session_id();
                    state.add_line(&format!("  ID: {}", session_id));
                    state.add_line("");
                    state.add_line(
                        "\x1b[90m  Full session selector with search/arrow keys requires\x1b[0m",
                    );
                    state.add_line("\x1b[90m  TUI overlay mode (planned feature).\x1b[0m");
                }
                self.tui.request_render();
                return true;
            }
            "/settings" => {
                if let Some(ref state) = self.chat_state {
                    state.add_line(&format!("\x1b[1m── Settings ──\x1b[0m"));
                    if let Some(model) = self.runtime_host.session().model() {
                        state.add_line(&format!("  Model: {} ({})", model.name, model.provider));
                        state.add_line(&format!(
                            "  Context window: {} tokens",
                            model.context_window
                        ));
                    }
                    state.add_line("");
                    state.add_line(
                        "\x1b[90m  Full settings selector with arrow keys requires\x1b[0m",
                    );
                    state.add_line("\x1b[90m  TUI overlay mode (planned feature).\x1b[0m");
                }
                self.tui.request_render();
                return true;
            }
            "/tree" => {
                if let Some(ref state) = self.chat_state {
                    state.add_line(&format!("\x1b[1m── Session Tree ──\x1b[0m"));
                    let sm = self.runtime_host.session().session_manager();
                    let leaf_id = sm.get_leaf_id();
                    state.add_line(&format!(
                        "  Leaf ID: {}",
                        leaf_id.as_deref().unwrap_or("none")
                    ));
                    let entries = sm.get_entries();
                    state.add_line(&format!("  Total entries: {}", entries.len()));
                    state.add_line("");
                    state.add_line("\x1b[90m  Full tree navigator with arrow keys, folding, and filters\x1b[0m");
                    state.add_line("\x1b[90m  requires TUI overlay mode (planned feature).\x1b[0m");
                }
                self.tui.request_render();
                return true;
            }
            "/compact" => {
                if let Some(ref state) = self.chat_state {
                    state.add_line("\x1b[36m/compact\x1b[0m  Triggering session compaction...");
                }
                if let Some(ref runner) = self.runtime_host.session().extension_runner() {
                    let ctx = runner.create_context();
                    ctx.compact(None);
                    if let Some(ref state) = self.chat_state {
                        state.add_line(
                            "\x1b[90m  Compaction triggered. Runs before next prompt.\x1b[0m",
                        );
                    }
                } else {
                    if let Some(ref state) = self.chat_state {
                        state.add_line("\x1b[90m  No extension runner available. Compaction will run automatically when needed.\x1b[0m");
                    }
                }
                self.tui.request_render();
                return true;
            }
            "/export" => {
                if let Some(ref state) = self.chat_state {
                    state.add_line("\x1b[36m/export\x1b[0m  Exporting session to HTML...");
                }
                let sm = self.runtime_host.session().session_manager();
                let header = sm.get_header();
                let entries = sm.get_entries();
                let leaf_id = sm.get_leaf_id();
                let html = export_session_to_html(header, &entries, leaf_id.as_deref());
                let output_path = std::env::current_dir()
                    .unwrap_or_else(|_| Path::new(".").to_path_buf())
                    .join(format!("hamr-export-{}.html", sm.get_session_id()));
                match std::fs::write(&output_path, &html) {
                    Ok(_) => {
                        if let Some(ref state) = self.chat_state {
                            state.add_line(&format!(
                                "\x1b[32m  Exported to: {}\x1b[0m",
                                output_path.display()
                            ));
                        }
                    }
                    Err(e) => {
                        if let Some(ref state) = self.chat_state {
                            state.add_line(&format!("\x1b[31m  Export failed: {}\x1b[0m", e));
                        }
                    }
                }
                self.tui.request_render();
                return true;
            }
            "/new" => {
                if let Some(ref state) = self.chat_state {
                    state.add_line("\x1b[36m/new\x1b[0m  Creating new session...");
                }
                match self.runtime_host.new_session() {
                    Ok(_) => {
                        if let Some(ref state) = self.chat_state {
                            state.add_line("\x1b[32m  New session created.\x1b[0m");
                            state.add_line(&format!("  CWD: {}", self.runtime_host.services().cwd));
                        }
                    }
                    Err(e) => {
                        if let Some(ref state) = self.chat_state {
                            state.add_line(&format!("\x1b[31m  Failed: {}\x1b[0m", e));
                        }
                    }
                }
                self.tui.request_render();
                return true;
            }
            "/trust" => {
                if let Some(ref state) = self.chat_state {
                    state.add_line(&format!("\x1b[1m── Project Trust ──\x1b[0m"));
                    let cwd = &self.runtime_host.services().cwd;
                    state.add_line(&format!("  Directory: {}", cwd));
                    let is_trusted = self
                        .runtime_host
                        .session()
                        .extension_runner()
                        .map(|r| {
                            let ctx = r.create_context();
                            ctx.is_project_trusted()
                        })
                        .unwrap_or(true);
                    state.add_line(&format!(
                        "  Current session: {}",
                        if is_trusted {
                            "\x1b[32mtrusted\x1b[0m"
                        } else {
                            "\x1b[31muntrusted\x1b[0m"
                        }
                    ));
                    state.add_line("");
                    state.add_line("\x1b[90m  Full trust selector with arrow keys requires\x1b[0m");
                    state.add_line("\x1b[90m  TUI overlay mode (planned feature).\x1b[0m");
                }
                self.tui.request_render();
                return true;
            }
            "/reload" => {
                if let Some(ref state) = self.chat_state {
                    state.add_line("\x1b[36m/reload\x1b[0m  Reloading keybindings, extensions, skills, prompts, and themes...");
                }
                if let Some(ref runner) = self.runtime_host.session().extension_runner() {
                    let cmd_ctx = runner.create_command_context();
                    cmd_ctx.reload().await;
                    if let Some(ref state) = self.chat_state {
                        state.add_line("\x1b[32m  Reload complete.\x1b[0m");
                    }
                } else {
                    if let Some(ref state) = self.chat_state {
                        state.add_line("\x1b[90m  No extension runner available.\x1b[0m");
                    }
                }
                self.tui.request_render();
                return true;
            }
            "/resume" => {
                if let Some(ref state) = self.chat_state {
                    state.add_line("\x1b[36m/resume\x1b[0m  Finding most recent session...");
                }
                let agent_dir = self.runtime_host.services().agent_dir.clone();
                let session_dir = Path::new(&agent_dir).join("sessions");
                let most_recent = find_most_recent_session(
                    &session_dir,
                    Some(self.runtime_host.services().cwd.as_str()),
                );
                match most_recent {
                    Some(path) => {
                        let session_path = Path::new(&path).to_path_buf();
                        match self.runtime_host.switch_session(&session_path, None) {
                            Ok(_) => {
                                if let Some(ref state) = self.chat_state {
                                    state.add_line(&format!(
                                        "\x1b[32m  Resumed session: {}\x1b[0m",
                                        session_path.display()
                                    ));
                                }
                            }
                            Err(e) => {
                                if let Some(ref state) = self.chat_state {
                                    state.add_line(&format!("\x1b[31m  Failed: {}\x1b[0m", e));
                                }
                            }
                        }
                    }
                    None => {
                        if let Some(ref state) = self.chat_state {
                            state.add_line("\x1b[90m  No previous sessions found.\x1b[0m");
                        }
                    }
                }
                self.tui.request_render();
                return true;
            }
            "/scoped-models" | "/import" | "/share" | "/copy" | "/name" | "/changelog"
            | "/hotkeys" | "/fork" | "/clone" | "/login" | "/logout" => {
                // Find the description from BUILTIN_SLASH_COMMANDS
                let desc = BUILTIN_SLASH_COMMANDS
                    .iter()
                    .find(|c| c.name == cmd_name.trim_start_matches('/'))
                    .map(|c| c.description)
                    .unwrap_or("coming soon");
                if let Some(ref state) = self.chat_state {
                    state.add_line(&format!(
                        "\x1b[90m[hamr] /{} — {} (coming soon)\x1b[0m",
                        cmd_name.trim_start_matches('/'),
                        desc
                    ));
                }
                self.tui.request_render();
                return true;
            }
            _ => {
                // Unknown slash command — let it pass through to the LLM
                // (extensions may register custom slash commands)
                return false;
            }
        }
    }

    #[cfg(not(feature = "tui"))]
    async fn dispatch_slash_command(&mut self, text: &str) -> bool {
        // Non-TUI mode: only handle /exit and /quit
        let trimmed = text.trim();
        if trimmed == "/exit" || trimmed == "/quit" {
            self.shutdown_requested = true;
            return true;
        }
        false
    }

    // ── Prompt dispatch ─────────────────────────────────────────────────────

    async fn process_prompt(&mut self, text: &str) {
        #[cfg(feature = "tui")]
        {
            if let Some(ref state) = self.chat_state {
                state.working.set(true);
                state.add_line("\x1b[90mWorking...\x1b[0m");
            }
            self.tui.request_render();
        }

        let abort_agent = self.runtime_host.session().agent().clone();
        let mut shutdown_after_prompt = false;
        let mut signal_exit_code = None;
        let mut rendered_live_response = false;
        #[cfg(feature = "tui")]
        let mut agent_event_rx = self
            .agent_event_rx
            .take()
            .expect("agent event receiver missing");
        #[cfg(feature = "tui")]
        let chat_state = self.chat_state.as_ref().map(Rc::clone);
        let prompt_result = {
            let session = self.runtime_host.session_mut();
            let mut prompt = Box::pin(session.prompt(text, None));

            loop {
                tokio::select! {
                    result = &mut prompt => break Some(result),
                    event = agent_event_rx.recv() => {
                        if let (Some(state), Some(event)) = (chat_state.as_deref(), event) {
                            rendered_live_response |= apply_agent_event(state, event);
                            self.tui.request_render();
                        }
                    }
                    exit_code = receive_shutdown_signal(&mut self.signal_rx) => {
                        abort_agent.abort().await;
                        shutdown_after_prompt = true;
                        signal_exit_code = Some(exit_code);
                        break None;
                    }
                    input = read_terminal_input() => {
                        match input {
                            Some(TerminalInput::Resize(columns, rows)) => {
                                self.tui.resize(columns, rows);
                                if let Some(ref state) = self.chat_state {
                                    state.terminal_width.set(columns as usize);
                                }
                            }
                            Some(TerminalInput::Paste(text)) => {
                                self.tui
                                    .handle_input(&format!("\x1b[200~{text}\x1b[201~"));
                            }
                            Some(TerminalInput::Key(data)) => {
                                if sexy_tui_rs::keys::matches_key(&data, "ctrl+d") {
                                    abort_agent.abort().await;
                                    shutdown_after_prompt = true;
                                    break None;
                                } else if is_interrupt_key(&data) {
                                    abort_agent.abort().await;
                                } else {
                                    self.tui.handle_input(&data);
                                }
                            }
                            None => {}
                        }
                    }
                }
            }
        };

        #[cfg(feature = "tui")]
        {
            while let Ok(event) = agent_event_rx.try_recv() {
                if let Some(state) = chat_state.as_deref() {
                    rendered_live_response |= apply_agent_event(state, event);
                }
            }
            self.agent_event_rx = Some(agent_event_rx);
        }

        self.shutdown_requested |= shutdown_after_prompt;
        if let Some(exit_code) = signal_exit_code {
            self.exit_code = exit_code;
        }
        #[cfg(feature = "tui")]
        if let Some(ref state) = self.chat_state {
            state.working.set(false);
        }

        let Some(prompt_result) = prompt_result else {
            #[cfg(feature = "tui")]
            if let Some(ref state) = self.chat_state {
                let mut lines = state.lines.borrow_mut();
                if lines.last().is_some_and(|line| line.contains("Working...")) {
                    lines.pop();
                }
            }
            #[cfg(feature = "tui")]
            self.tui.request_render();
            return;
        };

        match prompt_result {
            Ok(()) => {
                let response = if rendered_live_response {
                    Vec::new()
                } else {
                    self.format_last_response().await
                };
                #[cfg(feature = "tui")]
                if let Some(ref state) = self.chat_state {
                    // Remove the "Working..." line
                    {
                        let mut lines = state.lines.borrow_mut();
                        if lines.last().map_or(false, |l| l.contains("Working...")) {
                            lines.pop();
                        }
                    }
                    for line in &response {
                        state.add_line(line);
                    }
                }
                #[cfg(not(feature = "tui"))]
                for line in &response {
                    println!("{}", line);
                }
            }
            Err(e) => {
                let msg = format!("\x1b[31mError: {}\x1b[0m", e);
                #[cfg(feature = "tui")]
                if let Some(ref state) = self.chat_state {
                    let mut lines = state.lines.borrow_mut();
                    if lines.last().map_or(false, |l| l.contains("Working...")) {
                        lines.pop();
                    }
                    state.add_line(&msg);
                }
                #[cfg(not(feature = "tui"))]
                eprintln!("{}", msg);
            }
        }

        #[cfg(feature = "tui")]
        self.tui.request_render();
    }

    /// Format the last assistant message for display.
    async fn format_last_response(&self) -> Vec<String> {
        let mut lines = Vec::new();

        #[cfg(feature = "tui")]
        let max_width = self
            .chat_state
            .as_ref()
            .map(|s| s.terminal_width.get())
            .unwrap_or(80);
        #[cfg(not(feature = "tui"))]
        let max_width: usize = 80;

        if let Some(assistant) = self
            .runtime_host
            .session()
            .last_assistant_message_pub()
            .await
        {
            for block in &assistant.content {
                match block {
                    hamr_ai::types::AssistantContentBlock::Text(t) => {
                        lines.extend(wrap_chat_text(&t.text, "  ", "  ", max_width));
                    }
                    hamr_ai::types::AssistantContentBlock::Thinking(tc) => {
                        if !tc.redacted {
                            lines.push(format!(
                                "\x1b[90m  [thinking] {}\x1b[0m",
                                &tc.thinking.chars().take(200).collect::<String>()
                            ));
                        } else {
                            lines.push("\x1b[90m  [reasoning redacted]\x1b[0m".to_string());
                        }
                    }
                    hamr_ai::types::AssistantContentBlock::ToolCall(tc) => {
                        lines.push(format!(
                            "\x1b[33m  [tool:{}] {}\x1b[0m",
                            tc.name,
                            serde_json::to_string(&tc.arguments)
                                .unwrap_or_else(|_| "{}".to_string())
                        ));
                    }
                }
            }
        }

        if lines.is_empty() {
            lines.push("\x1b[90m  (no response)\x1b[0m".to_string());
        }

        lines
    }

    // ── Signal handlers ─────────────────────────────────────────────────────

    fn register_signal_handlers(&mut self) {
        #[cfg(unix)]
        {
            use tokio::signal::unix::{SignalKind, signal};

            let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
            self.signal_rx = Some(rx);

            if let Ok(mut sigint) = signal(SignalKind::interrupt()) {
                let tx = tx.clone();
                let h = tokio::spawn(async move {
                    sigint.recv().await;
                    let _ = tx.send(130);
                });
                self.signal_cleanup_handlers.push(Box::new(move || {
                    h.abort();
                }));
            }

            if let Ok(mut sigterm) = signal(SignalKind::terminate()) {
                let tx = tx.clone();
                let h = tokio::spawn(async move {
                    sigterm.recv().await;
                    let _ = tx.send(143);
                });
                self.signal_cleanup_handlers.push(Box::new(move || {
                    h.abort();
                }));
            }

            if let Ok(mut sighup) = signal(SignalKind::hangup()) {
                let h = tokio::spawn(async move {
                    sighup.recv().await;
                    let _ = tx.send(129);
                });
                self.signal_cleanup_handlers.push(Box::new(move || {
                    h.abort();
                }));
            }
        }
    }

    fn cleanup_signal_handlers(&mut self) {
        for f in self.signal_cleanup_handlers.drain(..) {
            f();
        }
    }

    // ── Shutdown ────────────────────────────────────────────────────────────

    pub async fn shutdown(&mut self) {
        self.shutdown_requested = true;

        let from_signal = self.exit_code != 0;
        if from_signal {
            self.runtime_host.session().dispose().await;
        }

        #[cfg(feature = "tui")]
        {
            self.tui.stop();
            let _ = crossterm::terminal::disable_raw_mode();
            self.terminal_restore_guard.disarm();
        }

        if !from_signal {
            self.runtime_host.session().dispose().await;
        }

        self.cleanup_signal_handlers();
        flush_raw_stdout().await;
    }

    pub fn exit_code(&self) -> i32 {
        self.exit_code
    }
}

// ─── Static helpers ───────────────────────────────────────────────────────

pub fn is_anthropic_subscription_auth_key(api_key: Option<&str>) -> bool {
    matches!(api_key, Some(key) if key.starts_with("sk-ant-oat"))
}

pub fn is_unknown_model(provider: &str, id: &str) -> bool {
    provider == "unknown" && id == "unknown"
}

pub fn quote_if_needed(value: &str) -> String {
    if value.is_empty() {
        return String::from("''");
    }
    let needs_quoting = value.chars().any(|c| {
        !c.is_ascii_alphanumeric()
            && c != '_'
            && c != '-'
            && c != '.'
            && c != '/'
            && c != '~'
            && c != ':'
            && c != '@'
    });
    if needs_quoting {
        let escaped = value.replace('\'', "'\\''");
        format!("'{}'", escaped)
    } else {
        value.to_string()
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
#[cfg(feature = "tui")]
mod tests {
    use super::*;
    use crate::core::agent_session_runtime::CreateAgentSessionRuntimeOptions;
    use crate::core::agent_session_services::AgentSessionServices;
    use sexy_tui_rs::Component;

    fn stub_runtime() -> AgentSessionRuntime {
        let session = crate::core::agent_session::stub_agent_session();
        let services = AgentSessionServices::for_test();
        AgentSessionRuntime::new(
            session,
            services,
            Box::new(|_opts: CreateAgentSessionRuntimeOptions| {
                Ok(
                    crate::core::agent_session_runtime::CreateAgentSessionRuntimeResult {
                        session: crate::core::agent_session::stub_agent_session(),
                        services: AgentSessionServices::for_test(),
                        diagnostics: Vec::new(),
                        model_fallback_message: None,
                    },
                )
            }),
            Vec::new(),
            None,
        )
    }

    fn chat_widget() -> (HamrChatWidget, Rc<ChatState>) {
        let (submit_tx, _submit_rx) = tokio::sync::mpsc::unbounded_channel::<String>();
        let widget = HamrChatWidget::new(submit_tx);
        let state = widget.state_handle();
        (widget, state)
    }

    fn assistant_message(text: &str) -> hamr_ai::types::AssistantMessage {
        hamr_ai::types::AssistantMessage {
            role: hamr_ai::types::MessageRole::Assistant,
            content: vec![hamr_ai::types::AssistantContentBlock::Text(
                hamr_ai::types::TextContent {
                    text: text.to_string(),
                    text_signature: None,
                },
            )],
            api: "openai-completions".to_string(),
            provider: "test".to_string(),
            model: "test".to_string(),
            response_model: None,
            response_id: None,
            usage: hamr_ai::types::Usage {
                input: 0,
                output: 0,
                cache_read: 0,
                cache_write: 0,
                cache_write_1h: None,
                total_tokens: 0,
                cost: hamr_ai::types::UsageCost {
                    input: 0.0,
                    output: 0.0,
                    cache_read: 0.0,
                    cache_write: 0.0,
                    total: 0.0,
                },
            },
            stop_reason: hamr_ai::types::StopReason::Stop,
            error_message: None,
            diagnostics: None,
            timestamp: chrono::Utc::now(),
        }
    }

    #[test]
    fn test_restored_history_formats_user_and_assistant_messages() {
        use hamr_harness::types::AgentMessage;

        let user = AgentMessage::User(hamr_ai::types::UserMessage {
            role: hamr_ai::types::MessageRole::User,
            content: vec![hamr_ai::types::MessageContent::Text(
                hamr_ai::types::TextContent {
                    text: "previous question".to_string(),
                    text_signature: None,
                },
            )],
            timestamp: chrono::Utc::now(),
        });
        let assistant = AgentMessage::Assistant(assistant_message("previous answer"));

        let (lines, assistant_messages) = format_restored_history(&[user, assistant], 80);

        assert!(lines.iter().any(|line| line.contains("previous question")));
        assert!(lines.iter().any(|line| line.contains("previous answer")));
        assert_eq!(assistant_messages.len(), 1);
    }

    #[tokio::test]
    async fn test_interactive_mode_new() {
        let runtime = stub_runtime();
        let mut mode = InteractiveMode::new(runtime, InteractiveModeOptions::default());
        assert!(!mode.is_initialized);
        assert!(!mode.shutdown_requested);
        assert_eq!(mode.version, env!("CARGO_PKG_VERSION"));

        mode.tui.handle_input("A");
        assert_eq!(
            mode.chat_state.as_ref().unwrap().input.borrow().as_str(),
            "A"
        );
    }

    #[test]
    fn test_csi_u_ctrl_exit_keys_are_recognized() {
        assert!(is_exit_key("\x1b[99;5u"));
        assert!(is_exit_key("\x1b[100;5u"));
    }

    #[test]
    fn test_shifted_printable_input_is_inserted() {
        let (mut widget, state) = chat_widget();
        widget.handle_input("A");
        widget.handle_input("?");
        assert_eq!(state.input.borrow().as_str(), "A?");
    }

    #[test]
    fn test_unicode_insert_backspace_and_delete() {
        let (mut widget, state) = chat_widget();
        widget.handle_input("é");
        widget.handle_input("中");
        widget.handle_input("🙂");
        assert_eq!(state.input.borrow().as_str(), "é中🙂");

        widget.handle_input("\x1b[D");
        widget.handle_input("\x1b[3~");
        assert_eq!(state.input.borrow().as_str(), "é中");

        widget.handle_input("\x7f");
        assert_eq!(state.input.borrow().as_str(), "é");
    }

    #[test]
    fn test_multiline_paste_is_inserted() {
        let (mut widget, state) = chat_widget();
        widget.handle_input("\x1b[200~first\nsecond\x1b[201~");
        assert_eq!(state.input.borrow().as_str(), "first\nsecond");
    }

    #[test]
    fn test_chat_wrapping_preserves_unicode_boundaries() {
        let text = "🙂".repeat(80);
        let lines = wrap_chat_text(&text, "  ", "  ", 80);

        assert_eq!(lines.len(), 3);
        assert_eq!(lines.concat().matches('🙂').count(), 80);
        assert!(!lines.concat().contains('\u{fffd}'));
    }

    #[test]
    fn test_stream_updates_replace_in_place_without_final_duplication() {
        use hamr_harness::types::{AgentEvent, AgentMessage};

        let (_widget, state) = chat_widget();
        state.add_line("\x1b[90mWorking...\x1b[0m");
        let partial = assistant_message("Hello");
        assert!(apply_agent_event(
            &state,
            AgentEvent::MessageUpdate {
                message: AgentMessage::Assistant(partial.clone()),
                assistant_message_event: hamr_ai::types::AssistantMessageEvent::TextDelta {
                    content_index: 0,
                    delta: "Hello".to_string(),
                    partial,
                },
            },
        ));

        assert!(apply_agent_event(
            &state,
            AgentEvent::MessageEnd {
                message: AgentMessage::Assistant(assistant_message("Hello world")),
            },
        ));

        let lines = state.lines.borrow();
        // Component rendering may add card padding; verify content not duplicated and "Working..." removed
        assert!(
            !lines.iter().any(|l| l.contains("Working...")),
            "Working... should be removed"
        );
        assert!(
            lines.iter().any(|l| l.contains("Hello world")),
            "should contain final text"
        );
        let hello_count = lines.iter().filter(|l| l.contains("Hello world")).count();
        assert_eq!(hello_count, 1, "'Hello world' should appear exactly once");
    }

    #[test]
    fn test_tool_events_replace_activity_and_append_text_result() {
        use hamr_harness::types::AgentEvent;

        let (_widget, state) = chat_widget();
        state.add_line("\x1b[90mWorking...\x1b[0m");
        apply_agent_event(
            &state,
            AgentEvent::ToolExecutionStart {
                tool_call_id: "call-1".to_string(),
                tool_name: "bash".to_string(),
                args: serde_json::json!({"command": "pwd"}),
            },
        );
        apply_agent_event(
            &state,
            AgentEvent::ToolExecutionEnd {
                tool_call_id: "call-1".to_string(),
                tool_name: "bash".to_string(),
                result: serde_json::json!({
                    "content": [{"type": "text", "text": "/tmp/project"}]
                }),
                is_error: false,
            },
        );

        let lines = state.lines.borrow();
        let joined = lines.join("\n");
        // Working... should be removed by tool start
        assert!(!joined.contains("Working..."));
        // Tool name and result should appear in the rendered component
        assert!(joined.contains("bash"));
        assert!(joined.contains("/tmp/project"));
    }

    #[test]
    fn test_quote_if_needed_safe() {
        assert_eq!(quote_if_needed("hello"), "hello");
    }

    #[test]
    fn test_quote_if_needed_unsafe() {
        assert_eq!(quote_if_needed("hello world"), "'hello world'");
        assert_eq!(quote_if_needed(""), "''");
    }
}
