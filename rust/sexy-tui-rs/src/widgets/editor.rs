use std::cell::Cell;

use crate::autocomplete::{AutocompleteItem, AutocompleteProvider};
use crate::editor_component::EditorComponent;
use crate::kill_ring::KillRing;
use crate::theme::Theme;
use crate::tui::{Component, Focusable, CURSOR_MARKER};
use crate::undo_stack::UndoStack;
use crate::utils::{truncate_to_width, visible_width};
use crate::word_navigation::{find_word_backward, find_word_forward};

pub struct EditorTheme {
    pub border_color: Box<dyn Fn(&str) -> String>,
    pub prompt_color: Box<dyn Fn(&str) -> String>,
}

impl EditorTheme {
    pub fn new(theme: &Theme) -> Self {
        let t = theme.clone();
        let t2 = theme.clone();
        EditorTheme {
            border_color: Box::new(move |s| t.fg("accent", s)),
            prompt_color: Box::new(move |s| t2.fg("muted", s)),
        }
    }
}

pub struct EditorOptions {
    pub padding_x: u16,
    pub prompt_prefix: Option<String>,
    pub autocomplete_max_visible: usize,
}

impl Default for EditorOptions {
    fn default() -> Self {
        EditorOptions {
            padding_x: 0,
            prompt_prefix: None,
            autocomplete_max_visible: 5,
        }
    }
}

/// A visual (wrapped) line with its source logical line and column range.
#[derive(Clone)]
struct VisualLine {
    logical_line: usize,
    start_col: usize,
    length: usize,
}

#[derive(Clone)]
struct EditorState {
    lines: Vec<String>,
    cursor_row: usize,
    cursor_col: usize,
}

#[derive(Clone)]
struct PasteEntry {
    #[allow(dead_code)]
    content: String,
}

/// Default trigger characters for autocomplete.
const DEFAULT_AUTOCOMPLETE_TRIGGER_CHARACTERS: &[&str] = &["@", "#"];

pub struct Editor {
    // Core state
    lines: Vec<String>,
    cursor_row: usize,
    cursor_col: usize,
    focused: bool,
    theme: EditorTheme,
    options: EditorOptions,
    pub on_submit: Option<Box<dyn FnMut(&str)>>,
    pub on_change: Option<Box<dyn FnMut(&str)>>,
    pub disable_submit: bool,
    kill_ring: KillRing,
    last_action: Option<String>,
    undo_stack: UndoStack<EditorState>,

    // History
    history: Vec<String>,
    history_index: isize,
    history_draft: Option<EditorState>,

    // Paste
    pastes: std::collections::HashMap<usize, PasteEntry>,
    paste_counter: usize,
    is_in_paste: bool,
    paste_buffer: String,

    // Visual
    scroll_offset: Cell<usize>,
    last_width: Cell<u16>,
    preferred_visual_col: Cell<Option<usize>>,
    snapped_from_cursor_col: Cell<Option<usize>>,

    // Autocomplete
    autocomplete_provider: Option<Box<dyn AutocompleteProvider>>,
    autocomplete_state: Option<String>,  // None = hidden, Some("regular") or Some("force")
    autocomplete_items: Vec<AutocompleteItem>,
    autocomplete_selected: usize,
    autocomplete_prefix: String,
    autocomplete_trigger_characters: Vec<String>,

    // Character jump
    jump_mode: Option<JumpDirection>,
}

#[derive(Clone, Copy)]
enum JumpDirection {
    Forward,
    Backward,
}

impl Editor {
    pub fn new(theme: EditorTheme, options: EditorOptions) -> Self {
        Editor {
            lines: vec![String::new()],
            cursor_row: 0,
            cursor_col: 0,
            focused: false,
            theme,
            options,
            on_submit: None,
            on_change: None,
            disable_submit: false,
            kill_ring: KillRing::new(),
            last_action: None,
            undo_stack: UndoStack::new(),
            history: Vec::new(),
            history_index: -1,
            history_draft: None,
            pastes: std::collections::HashMap::new(),
            paste_counter: 0,
            is_in_paste: false,
            paste_buffer: String::new(),
            scroll_offset: Cell::new(0),
            last_width: Cell::new(80),
            preferred_visual_col: Cell::new(None),
            snapped_from_cursor_col: Cell::new(None),
            autocomplete_provider: None,
            autocomplete_state: None,
            autocomplete_items: Vec::new(),
            autocomplete_selected: 0,
            autocomplete_prefix: String::new(),
            autocomplete_trigger_characters: DEFAULT_AUTOCOMPLETE_TRIGGER_CHARACTERS.iter().map(|s| s.to_string()).collect(),
            jump_mode: None,
        }
    }

    pub fn with_prompt_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.options.prompt_prefix = Some(prefix.into());
        self
    }
    pub fn with_padding_x(mut self, px: u16) -> Self {
        self.options.padding_x = px;
        self
    }

    pub fn set_prompt_prefix(&mut self, prefix: Option<String>) {
        self.options.prompt_prefix = prefix;
    }
    pub fn prompt_prefix(&self) -> Option<&str> {
        self.options.prompt_prefix.as_deref()
    }
    pub fn set_padding_x(&mut self, px: u16) {
        self.options.padding_x = px;
    }
    pub fn padding_x(&self) -> u16 {
        self.options.padding_x
    }

    pub fn set_text(&mut self, text: &str) {
        self.lines = text.lines().map(|l| l.to_string()).collect();
        if self.lines.is_empty() {
            self.lines.push(String::new());
        }
        self.cursor_row = 0;
        self.cursor_col = 0;
        self.scroll_offset.set(0);
        self.exit_history();
        self.cancel_autocomplete();
    }

    pub fn get_text(&self) -> String {
        self.lines.join("\n")
    }
    pub fn get_lines(&self) -> &[String] {
        &self.lines
    }
    pub fn get_cursor(&self) -> (usize, usize) {
        (self.cursor_row, self.cursor_col)
    }

    pub fn add_to_history(&mut self, text: &str) {
        let trimmed = text.trim();
        if trimmed.is_empty() {
            return;
        }
        if self.history.first().is_some_and(|h| h == trimmed) {
            return;
        }
        self.history.insert(0, trimmed.to_string());
        if self.history.len() > 100 {
            self.history.pop();
        }
    }

    // ── Autocomplete ─────────────────────────────────────────────────────

    pub fn set_autocomplete_provider(&mut self, provider: Box<dyn AutocompleteProvider>) {
        let trigger_chars = provider.trigger_characters();
        let mut chars: Vec<String> = DEFAULT_AUTOCOMPLETE_TRIGGER_CHARACTERS.iter().map(|s| s.to_string()).collect();
        for ch in trigger_chars {
            if ch.len() == 1 {
                let c = ch.chars().next().unwrap();
                if c != '/' && !c.is_whitespace() && !chars.contains(&ch) {
                    chars.push(ch.clone());
                }
            }
        }
        self.autocomplete_trigger_characters = chars;
        self.autocomplete_provider = Some(provider);
    }

    pub fn is_showing_autocomplete(&self) -> bool {
        self.autocomplete_state.is_some()
    }

    // Match TS: tryTriggerAutocomplete(explicitTab = false)
    fn try_trigger_autocomplete(&mut self) {
        self.request_autocomplete(false, false);
    }

    // Match TS: handleTabCompletion
    fn handle_tab_completion(&mut self) {
        if self.autocomplete_provider.is_none() {
            return;
        }

        let before = self.text_before_cursor();
        if self.is_in_slash_command_context(&before) && !before.trim_start().contains(' ') {
            self.handle_slash_command_completion();
        } else {
            self.force_file_autocomplete(true);
        }
    }

    fn handle_slash_command_completion(&mut self) {
        self.request_autocomplete(false, true);
    }

    fn force_file_autocomplete(&mut self, explicit_tab: bool) {
        self.request_autocomplete(true, explicit_tab);
    }

    // Match TS: requestAutocomplete({ force, explicitTab })
    fn request_autocomplete(&mut self, force: bool, explicit_tab: bool) {
        let provider = match self.autocomplete_provider {
            Some(ref p) => p,
            None => return,
        };

        if force {
            if !provider.should_trigger_file_completion(&self.lines, self.cursor_row, self.cursor_col) {
                self.cancel_autocomplete();
                return;
            }
        }

        let suggestions = provider.get_suggestions(&self.lines, self.cursor_row, self.cursor_col, force);
        
        let suggestions = match suggestions {
            None => { self.cancel_autocomplete(); return; },
            Some(s) if s.items.is_empty() => { self.cancel_autocomplete(); return; },
            Some(s) => s,
        };

        // Single suggestion + Tab + force → auto-apply without showing menu
        if force && explicit_tab && suggestions.items.len() == 1 {
            let item = suggestions.items.into_iter().next().unwrap();
            let prefix = suggestions.prefix;
            let result = self.autocomplete_provider.as_ref().map(|p| {
                p.apply_completion(&self.lines, self.cursor_row, self.cursor_col, &item, &prefix)
            });
            self.push_undo();
            self.last_action = None;
            if let Some(r) = result {
                self.lines = r.lines;
                self.cursor_row = r.cursor_line;
                self.cursor_col = r.cursor_col;
            }
            self.cancel_autocomplete();
            self.notify_change();
            return;
        }

        // Apply autocomplete suggestions
        self.autocomplete_prefix = suggestions.prefix;
        self.autocomplete_items = suggestions.items;
        self.autocomplete_selected = self.best_autocomplete_index();
        self.autocomplete_state = Some(if force { "force".into() } else { "regular".into() });
    }

    // Match TS: updateAutocomplete
    fn update_autocomplete(&mut self) {
        if self.autocomplete_state.is_none() || self.autocomplete_provider.is_none() {
            return;
        }
        let force = self.autocomplete_state.as_deref() == Some("force");
        self.request_autocomplete(force, false);
    }

    // Match TS: isSlashMenuAllowed
    fn is_slash_menu_allowed(&self) -> bool {
        self.cursor_row == 0
    }

    // Match TS: getBestAutocompleteMatchIndex
    fn best_autocomplete_index(&self) -> usize {
        let prefix = &self.autocomplete_prefix;
        if prefix.is_empty() {
            return 0;
        }
        let mut first_prefix = None;
        for (i, item) in self.autocomplete_items.iter().enumerate() {
            if item.value == *prefix {
                return i;
            }
            if first_prefix.is_none() && item.value.starts_with(prefix) {
                first_prefix = Some(i);
            }
        }
        first_prefix.unwrap_or(0)
    }

    fn cancel_autocomplete(&mut self) {
        self.autocomplete_state = None;
        self.autocomplete_items.clear();
        self.autocomplete_prefix.clear();
        self.autocomplete_selected = 0;
    }

    fn apply_selected_autocomplete(&mut self) {
        if self.autocomplete_items.is_empty() {
            return;
        }
        let idx = self.autocomplete_selected.min(self.autocomplete_items.len().saturating_sub(1));
        let item = self.autocomplete_items[idx].clone();
        let prefix = self.autocomplete_prefix.clone();
        let result = self.autocomplete_provider.as_ref().map(|p| {
            p.apply_completion(&self.lines, self.cursor_row, self.cursor_col, &item, &prefix)
        });
        self.push_undo();
        self.last_action = None;
        if let Some(r) = result {
            self.lines = r.lines;
            self.cursor_row = r.cursor_line;
            self.cursor_col = r.cursor_col;
        }
        self.cancel_autocomplete();
        self.notify_change();
    }

    // ── Helpers ──────────────────────────────────────────────────────────

    fn text_before_cursor(&self) -> String {
        if self.cursor_row < self.lines.len() {
            let line = &self.lines[self.cursor_row];
            line[..self.cursor_col.min(line.len())].to_string()
        } else {
            String::new()
        }
    }

    fn is_editor_empty(&self) -> bool {
        self.lines.len() == 1 && self.lines[0].is_empty()
    }
    fn is_on_first_visual_line(&self) -> bool {
        let vls = self.build_visual_line_map(self.last_width.get());
        self.find_current_visual_line(&vls) == 0 && self.scroll_offset.get() == 0
    }
    fn prompt_visible_width(&self) -> usize {
        self.options
            .prompt_prefix
            .as_ref()
            .map(|p| visible_width(p))
            .unwrap_or(0)
    }
    fn prompt_str(&self) -> &str {
        self.options.prompt_prefix.as_deref().unwrap_or("")
    }

    fn max_visible_lines(term_rows: u16) -> usize {
        (term_rows as f32 * 0.3).max(5.0) as usize
    }

    fn push_undo(&mut self) {
        self.undo_stack.push(&EditorState {
            lines: self.lines.clone(),
            cursor_row: self.cursor_row,
            cursor_col: self.cursor_col,
        });
    }

    fn undo(&mut self) {
        if let Some(state) = self.undo_stack.pop() {
            self.lines = state.lines;
            self.cursor_row = state.cursor_row;
            self.cursor_col = state.cursor_col;
            self.notify_change();
        }
    }

    fn jump_to_char(&mut self, ch: char, direction: JumpDirection) {
        let line = &self.lines[self.cursor_row];
        let search: String = match direction {
            JumpDirection::Forward => line[self.cursor_col + 1..].to_string(),
            JumpDirection::Backward => line[..self.cursor_col].chars().rev().collect(),
        };
        let pos = search.find(ch);
        match direction {
            JumpDirection::Forward => {
                if let Some(p) = pos {
                    self.cursor_col += p + 1;
                }
            }
            JumpDirection::Backward => {
                if let Some(p) = pos {
                    self.cursor_col -= p + ch.len_utf8();
                }
            }
        }
    }

    // ── Visual-line mapping ──────────────────────────────────────────────

    fn build_visual_line_map(&self, width: u16) -> Vec<VisualLine> {
        let w = width.max(1) as usize;
        if w == 0 {
            return vec![VisualLine { logical_line: 0, start_col: 0, length: 0 }];
        }
        let mut vls = Vec::new();
        for (li, line) in self.lines.iter().enumerate() {
            if line.is_empty() {
                vls.push(VisualLine { logical_line: li, start_col: 0, length: 0 });
                continue;
            }
            let mut segment_start = 0usize;
            let mut segment_width = 0usize;
            for (byte_index, grapheme) in
                unicode_segmentation::UnicodeSegmentation::grapheme_indices(line.as_str(), true)
            {
                let grapheme_width = crate::utils::visible_width(grapheme);
                if segment_width > 0 && segment_width + grapheme_width > w {
                    vls.push(VisualLine {
                        logical_line: li,
                        start_col: segment_start,
                        length: byte_index - segment_start,
                    });
                    segment_start = byte_index;
                    segment_width = 0;
                }
                segment_width += grapheme_width;
            }
            vls.push(VisualLine {
                logical_line: li,
                start_col: segment_start,
                length: line.len() - segment_start,
            });
        }
        if vls.is_empty() {
            vls.push(VisualLine { logical_line: 0, start_col: 0, length: 0 });
        }
        vls
    }

    fn find_current_visual_line(&self, vls: &[VisualLine]) -> usize {
        self.find_visual_line_at(vls, self.cursor_row, self.cursor_col)
    }

    fn find_visual_line_at(&self, vls: &[VisualLine], logical_line: usize, col: usize) -> usize {
        for (i, vl) in vls.iter().enumerate() {
            if vl.logical_line != logical_line || col < vl.start_col {
                continue;
            }
            let offset = col - vl.start_col;
            let is_last_segment = i + 1 == vls.len() || vls[i + 1].logical_line != logical_line;
            if offset < vl.length || (is_last_segment && offset == vl.length) {
                return i;
            }
        }
        vls.len().saturating_sub(1)
    }

    fn compute_vertical_move_column(
        &self,
        current_visual_col: usize,
        source_max_visual_col: usize,
        target_max_visual_col: usize,
    ) -> usize {
        let has_preferred = self.preferred_visual_col.get().is_some();
        let cursor_in_middle = current_visual_col < source_max_visual_col;
        let target_too_short = target_max_visual_col < current_visual_col;

        if !has_preferred || cursor_in_middle {
            if target_too_short {
                self.preferred_visual_col.set(Some(current_visual_col));
                return target_max_visual_col;
            }
            self.preferred_visual_col.set(None);
            return current_visual_col;
        }

        let preferred = self.preferred_visual_col.get().unwrap();
        let target_cant_fit = target_max_visual_col < preferred;
        if target_too_short || target_cant_fit {
            return target_max_visual_col;
        }
        self.preferred_visual_col.set(None);
        preferred
    }

    fn move_to_visual_line(&mut self, vls: &[VisualLine], current_vl: usize, target_vl: usize) {
        let Some(current_vl_entry) = vls.get(current_vl) else { return };
        let Some(target_vl_entry) = vls.get(target_vl) else { return };

        let current_visual_col = if let Some(snapped) = self.snapped_from_cursor_col.get() {
            let vl_idx = self.find_visual_line_at(vls, current_vl_entry.logical_line, snapped);
            snapped.saturating_sub(vls.get(vl_idx).map(|v| v.start_col).unwrap_or(0))
        } else {
            self.cursor_col.saturating_sub(current_vl_entry.start_col)
        };

        let is_last_source = current_vl + 1 >= vls.len()
            || vls[current_vl + 1].logical_line != current_vl_entry.logical_line;
        let is_last_target = target_vl + 1 >= vls.len()
            || vls[target_vl + 1].logical_line != target_vl_entry.logical_line;

        let source_max = if is_last_source { usize::MAX } else { 80 };
        let target_max = if is_last_target { usize::MAX } else { 80 };

        let target_visual_col = self.compute_vertical_move_column(
            current_visual_col.min(source_max),
            source_max.min(200),
            target_max.min(200),
        );

        self.cursor_row = target_vl_entry.logical_line;
        let target_col = target_vl_entry.start_col + target_visual_col;
        let logical_line = &self.lines[target_vl_entry.logical_line];
        self.snapped_from_cursor_col.set(None);
        self.cursor_col = target_col.min(logical_line.len());
    }

    fn exit_history(&mut self) {
        self.history_index = -1;
        self.history_draft = None;
    }

    fn notify_change(&mut self) {
        let text = self.get_text();
        if let Some(ref mut cb) = self.on_change {
            cb(&text);
        }
    }
}

impl Component for Editor {
    fn render(&self, width: u16) -> Vec<String> {
        let max_padding = width.saturating_sub(1) / 2;
        let px = self.options.padding_x.min(max_padding) as usize;
        let prompt_w = self.prompt_visible_width();
        let content_width = (width as usize).saturating_sub(px * 2).max(1);
        let layout_width = content_width.saturating_sub(usize::from(px == 0)).max(1);
        let text_width = layout_width.saturating_sub(prompt_w).max(1);
        self.last_width.set(text_width as u16);

        let max_vis = Self::max_visible_lines(width);
        let vls = self.build_visual_line_map(text_width as u16);
        let current_vl = self.find_current_visual_line(&vls);

        let mut so = self.scroll_offset.get();
        if current_vl < so {
            so = current_vl;
        } else if current_vl >= so + max_vis {
            so = current_vl.saturating_sub(max_vis - 1);
        }
        let max_so = vls.len().saturating_sub(max_vis);
        if so > max_so {
            so = max_so;
        }
        self.scroll_offset.set(so);

        let mut result = Vec::new();

        // Top border with scroll indicator
        if so > 0 {
            let indicator = format!("─── ↑ {} more ", so);
            let ind_w = visible_width(&indicator);
            let border_text = if ind_w <= width as usize {
                format!("{}{}", indicator, "─".repeat(width as usize - ind_w))
            } else {
                truncate_to_width(&indicator, width as usize, None)
            };
            result.push((self.theme.border_color)(&border_text));
        } else {
            result.push((self.theme.border_color)(&"─".repeat(width as usize)));
        }

        let left_padding = " ".repeat(px);
        let right_padding = left_padding.clone();
        let visible_end = (so + max_vis).min(vls.len());
        let visible_vls = &vls[so..visible_end];

        for (visible_index, vl) in visible_vls.iter().enumerate() {
            let line = &self.lines[vl.logical_line];
            let segment_end = (vl.start_col + vl.length).min(line.len());
            let segment = &line[vl.start_col..segment_end];
            let prompt = if vl.start_col == 0 {
                (self.theme.prompt_color)(self.prompt_str())
            } else {
                " ".repeat(prompt_w)
            };
            let is_cursor_line = so + visible_index == current_vl && self.focused;

            let content = if is_cursor_line {
                let marker = if self.focused { CURSOR_MARKER } else { "" };
                let cursor_offset = self
                    .cursor_col
                    .saturating_sub(vl.start_col)
                    .min(segment.len());
                if cursor_offset < segment.len() {
                    let before = &segment[..cursor_offset];
                    let at_cursor = segment[cursor_offset..].chars().next().unwrap_or(' ');
                    let after = &segment[cursor_offset + at_cursor.len_utf8()..];
                    format!(
                        "{}{}{}\x1b[7m{}\x1b[27m{}",
                        prompt, before, marker, at_cursor, after
                    )
                } else {
                    format!("{}{}{}\x1b[7m \x1b[27m", prompt, segment, marker)
                }
            } else {
                format!("{}{}", prompt, segment)
            };

            let lw = visible_width(&content);
            result.push(format!(
                "{}{}{}{}",
                left_padding,
                content,
                " ".repeat(content_width.saturating_sub(lw)),
                right_padding
            ));
        }

        let lines_below = vls.len().saturating_sub(so + visible_vls.len());
        if lines_below > 0 {
            let indicator = format!("─── ↓ {} more ", lines_below);
            let ind_w = visible_width(&indicator);
            let border_text = if ind_w <= width as usize {
                format!("{}{}", indicator, "─".repeat(width as usize - ind_w))
            } else {
                truncate_to_width(&indicator, width as usize, None)
            };
            result.push((self.theme.border_color)(&border_text));
        } else {
            result.push((self.theme.border_color)(&"─".repeat(width as usize)));
        }

        // ── Autocomplete suggestions (match TS: rendered after bottom border) ──
        if self.autocomplete_state.is_some() && !self.autocomplete_items.is_empty() {
            let max_visible = self.options.autocomplete_max_visible.max(3).min(20);
            let total = self.autocomplete_items.len();
            let selected = self.autocomplete_selected.min(total.saturating_sub(1));

            let start = if selected < max_visible / 2 {
                0
            } else if selected + max_visible / 2 >= total {
                total.saturating_sub(max_visible)
            } else {
                selected.saturating_sub(max_visible / 2)
            };
            let end = (start + max_visible).min(total);
            let visible_items = &self.autocomplete_items[start..end];

            for (i, item) in visible_items.iter().enumerate() {
                let idx = start + i;
                let is_selected = idx == selected;
                let prefix_str = if is_selected { "❯ " } else { "  " };
                let display_text = &item.label;

                let line = match item.description {
                    Some(ref desc) => {
                        let desc_styled = format!("\x1b[2m{}\x1b[22m", desc);
                        format!("{}{}  {}", prefix_str, display_text, desc_styled)
                    }
                    None => format!("{}{}", prefix_str, display_text),
                };

                let styled = if is_selected {
                    format!("\x1b[7m{}\x1b[27m", line)
                } else {
                    line
                };

                let lw = visible_width(&styled);
                if lw < content_width {
                    result.push(format!(
                        "{}{}{}{}",
                        left_padding,
                        styled,
                        " ".repeat(content_width - lw),
                        right_padding
                    ));
                } else {
                    result.push(format!("{}{}{}", left_padding, truncate_to_width(&styled, content_width, None), right_padding));
                }
            }

            // Scroll indicator
            if total > max_visible {
                let scroll_info = format!("\x1b[2m── {} of {} ──\x1b[22m", selected + 1, total);
                let sw = visible_width(&scroll_info);
                let padded = if sw <= content_width {
                    format!("{}{}{}", left_padding, scroll_info, " ".repeat(content_width - sw))
                } else {
                    format!("{}{}", left_padding, truncate_to_width(&scroll_info, content_width, None))
                };
                result.push(format!("{}{}", padded, right_padding));
            }
        }

        result
    }

    fn handle_input(&mut self, data: &str) {
        use crate::keys::{matches_key, Key};

        // ── Autocomplete active: intercept navigation keys (TS pattern) ──
        if self.autocomplete_state.is_some() && !self.autocomplete_items.is_empty() {
            // Cancel (Escape)
            if matches_key(data, Key::esc) {
                self.cancel_autocomplete();
                return;
            }

            // Navigate up/down
            if matches_key(data, Key::up) {
                if self.autocomplete_selected > 0 {
                    self.autocomplete_selected -= 1;
                }
                return;
            }
            if matches_key(data, Key::down) {
                if self.autocomplete_selected + 1 < self.autocomplete_items.len() {
                    self.autocomplete_selected += 1;
                }
                return;
            }

            // Tab: apply selected completion
            if matches_key(data, Key::tab) {
                self.apply_selected_autocomplete();
                return;
            }

            // Enter: apply selected completion
            if matches_key(data, Key::enter) {
                let was_slash = self.autocomplete_prefix.starts_with('/');
                self.apply_selected_autocomplete();
                if !was_slash {
                    // For non-slash completions, we consumed the Enter — don't submit
                    return;
                }
                // For slash commands, fall through to submit logic below
            }
        }

        // ── Character jump mode ──
        if self.jump_mode.is_some() {
            let direction = self.jump_mode.unwrap();
            if matches_key(data, &Key::ctrl("]")) || matches_key(data, &Key::ctrl_alt("]")) {
                self.jump_mode = None;
                return;
            }
            if data.len() == 1 {
                let ch = data.chars().next().unwrap();
                if !ch.is_ascii_control() {
                    self.jump_to_char(ch, direction);
                }
            }
            self.jump_mode = None;
            return;
        }

        // ── Bracketed paste ──
        if data.contains("\x1b[200~") {
            self.is_in_paste = true;
            self.paste_buffer = data.replace("\x1b[200~", "");
            if let Some(end) = self.paste_buffer.find("\x1b[201~") {
                let content = self.paste_buffer[..end].to_string();
                self.is_in_paste = false;
                self.paste_buffer.clear();
                if !content.is_empty() {
                    self.handle_paste(&content);
                }
            }
            return;
        }
        if self.is_in_paste {
            self.paste_buffer.push_str(data);
            if let Some(end) = self.paste_buffer.find("\x1b[201~") {
                let content = self.paste_buffer[..end].to_string();
                self.is_in_paste = false;
                self.paste_buffer.clear();
                if !content.is_empty() {
                    self.handle_paste(&content);
                }
            }
            return;
        }

        // ── Submit ──
        if matches_key(data, Key::enter) {
            if self.disable_submit {
                return;
            }
            if self.cursor_col > 0
                && self.lines[self.cursor_row]
                    .as_bytes()
                    .get(self.cursor_col - 1)
                    == Some(&b'\\')
            {
                self.backspace_char();
                self.add_newline();
                return;
            }
            self.submit_value();
            return;
        }

        // ── Newline ──
        if matches_key(data, &Key::shift("enter"))
            || matches_key(data, &Key::alt("enter"))
            || matches_key(data, &Key::ctrl("enter"))
        {
            self.add_newline();
            return;
        }

        // ── Tab (only when autocomplete NOT active) ──
        if matches_key(data, Key::tab) {
            self.handle_tab_completion();
            return;
        }

        // ── Character jump triggers ──
        if matches_key(data, &Key::ctrl("]")) {
            self.jump_mode = Some(JumpDirection::Forward);
            return;
        }
        if matches_key(data, &Key::ctrl_alt("]")) {
            self.jump_mode = Some(JumpDirection::Backward);
            return;
        }

        // ── Undo ──
        if matches_key(data, &Key::ctrl("z")) {
            self.undo();
            return;
        }

        // ── Kill ──
        if matches_key(data, &Key::ctrl("k")) {
            self.kill_to_end_of_line();
            return;
        }
        if matches_key(data, &Key::ctrl("u")) {
            self.kill_to_start_of_line();
            return;
        }
        if matches_key(data, &Key::ctrl("w")) || matches_key(data, &Key::alt("backspace")) {
            self.kill_word_backward();
            return;
        }
        if matches_key(data, &Key::alt("d")) || matches_key(data, &Key::alt("delete")) {
            self.kill_word_forward();
            return;
        }

        // ── Delete ──
        if matches_key(data, Key::backspace) || matches_key(data, "shift+backspace") {
            self.backspace_char();
            return;
        }
        if matches_key(data, Key::delete) || matches_key(data, "shift+delete") {
            self.delete_forward();
            return;
        }

        // ── Yank ──
        if matches_key(data, &Key::ctrl("y")) {
            self.yank_text();
            return;
        }

        // ── Cursor ──
        if matches_key(data, &Key::ctrl("a")) || matches_key(data, Key::home) {
            self.cursor_to_line_start();
            return;
        }
        if matches_key(data, &Key::ctrl("e")) || matches_key(data, Key::end) {
            self.cursor_to_line_end();
            return;
        }
        if matches_key(data, &Key::ctrl("left")) || matches_key(data, &Key::alt("left")) {
            self.cursor_word_backward();
            return;
        }
        if matches_key(data, &Key::ctrl("right")) || matches_key(data, &Key::alt("right")) {
            self.cursor_word_forward();
            return;
        }

        // ── Arrows with history ──
        if matches_key(data, Key::up) {
            let vls = self.build_visual_line_map(self.last_width.get());
            let current_vl = self.find_current_visual_line(&vls);
            if self.is_on_first_visual_line() && current_vl == 0 {
                if self.is_editor_empty() || self.history_index > -1 || self.cursor_col == 0 {
                    self.nav_history(-1);
                } else {
                    self.cursor_to_line_start();
                }
            } else if current_vl > 0 {
                self.last_action = None;
                self.move_to_visual_line(&vls, current_vl, current_vl - 1);
            }
            return;
        }
        if matches_key(data, Key::down) {
            let vls = self.build_visual_line_map(self.last_width.get());
            let current_vl = self.find_current_visual_line(&vls);
            if self.history_index > -1 && current_vl >= vls.len() - 1 {
                self.nav_history(1);
            } else if current_vl + 1 < vls.len() {
                self.last_action = None;
                self.move_to_visual_line(&vls, current_vl, current_vl + 1);
            } else {
                self.cursor_to_line_end();
            }
            return;
        }
        if matches_key(data, Key::left) {
            if self.cursor_col > 0 {
                self.last_action = None;
                self.cursor_col -= self.lines[self.cursor_row][..self.cursor_col]
                    .chars()
                    .last()
                    .unwrap()
                    .len_utf8();
            }
            return;
        }
        if matches_key(data, Key::right) {
            if self.cursor_col < self.lines[self.cursor_row].len() {
                self.last_action = None;
                self.cursor_col += self.lines[self.cursor_row][self.cursor_col..]
                    .chars()
                    .next()
                    .unwrap()
                    .len_utf8();
            }
            return;
        }

        // ── Printable characters ──
        let mut chars = data.chars();
        if !data.starts_with('\x1b')
            && chars.next().is_some_and(|ch| !ch.is_control())
            && chars.next().is_none()
        {
            self.insert_char(data);
        }
    }

    fn invalidate(&mut self) {}
}

impl Editor {
    fn submit_value(&mut self) {
        let result = self.lines.join("\n").trim().to_string();
        self.lines = vec![String::new()];
        self.cursor_row = 0;
        self.cursor_col = 0;
        self.pastes.clear();
        self.paste_counter = 0;
        self.exit_history();
        self.scroll_offset.set(0);
        self.undo_stack.clear();
        self.last_action = None;
        self.cancel_autocomplete();
        if let Some(ref mut cb) = self.on_change {
            cb("");
        }
        if let Some(ref mut cb) = self.on_submit {
            cb(&result);
        }
    }

    // Match TS: insertCharacter — handles undo coalescing, auto-trigger/update autocomplete
    fn insert_char(&mut self, ch: &str) {
        self.exit_history();

        // Undo coalescing (fish-style):
        // whitespace or non-consecutive word chars → push snapshot
        if ch.chars().any(|c| c.is_whitespace()) || self.last_action.as_deref() != Some("type") {
            self.push_undo();
        }
        self.last_action = Some("type".into());

        // Insert the character
        let line = &mut self.lines[self.cursor_row];
        line.insert_str(self.cursor_col, ch);
        self.cursor_col += ch.len();

        self.notify_change();

        // ── Autocomplete trigger/update logic (matching TS) ──
        if self.autocomplete_state.is_none() {
            // Auto-trigger for "/" at start of message (slash commands)
            if ch == "/" && self.is_at_start_of_message() {
                self.try_trigger_autocomplete();
            }
            // Auto-trigger for symbol-based completion like @, #, or provider triggers
            else if self.autocomplete_trigger_characters.iter().any(|t| t == ch) {
                let before = self.text_before_cursor();
                let char_before_symbol = before.as_bytes().get(before.len().wrapping_sub(2)).copied().unwrap_or(0) as char;
                if before.len() == 1 || char_before_symbol == ' ' || char_before_symbol == '\t' {
                    self.try_trigger_autocomplete();
                }
            }
            // Auto-trigger when typing letters in a slash command or symbol completion context
            else if ch.chars().all(|c| c.is_alphanumeric() || c == '.' || c == '-' || c == '_') {
                let before = self.text_before_cursor();
                // Check if we're in a slash command (with or without space for arguments)
                if self.is_in_slash_command_context(&before) {
                    self.try_trigger_autocomplete();
                }
                // Check if we're in a symbol-based completion context like @, #, or provider triggers
                else if self.is_in_trigger_context(&before) {
                    self.try_trigger_autocomplete();
                }
            }
        } else {
            // Autocomplete is already active — update it
            self.update_autocomplete();
        }
    }

    /// Check if text before cursor matches a trigger pattern like " @file" or " #tag"
    fn is_in_trigger_context(&self, text_before_cursor: &str) -> bool {
        for ch in &self.autocomplete_trigger_characters {
            // Match (start of text or whitespace) + trigger_char + non-whitespace chars + end
            let mut found = false;
            for (i, c) in text_before_cursor.char_indices() {
                let ch_str: &str = ch.as_str();
                let ch_char = ch_str.chars().next().unwrap();
                if c == ch_char {
                    let is_start = i == 0;
                    let after_delim = i > 0 && {
                        let prev = text_before_cursor.as_bytes().get(i - 1).copied().unwrap_or(0) as char;
                        prev == ' ' || prev == '\t'
                    };
                    if is_start || after_delim {
                        found = true;
                        break;
                    }
                }
            }
            if found {
                return true;
            }
        }
        false
    }

    fn is_at_start_of_message(&self) -> bool {
        if !self.is_slash_menu_allowed() {
            return false;
        }
        let before = self.text_before_cursor();
        before.trim().is_empty() || before.trim() == "/"
    }

    fn is_in_slash_command_context(&self, text_before_cursor: &str) -> bool {
        self.is_slash_menu_allowed() && text_before_cursor.trim_start().starts_with('/')
    }

    fn add_newline(&mut self) {
        self.exit_history();
        self.push_undo();
        self.last_action = None;
        self.cancel_autocomplete();
        let line = self.lines[self.cursor_row].clone();
        let before: String = line[..self.cursor_col].into();
        let after: String = line[self.cursor_col..].into();
        self.lines[self.cursor_row] = before;
        self.lines.insert(self.cursor_row + 1, after);
        self.cursor_row += 1;
        self.cursor_col = 0;
        self.notify_change();
    }

    fn backspace_char(&mut self) {
        self.exit_history();
        self.push_undo();
        self.last_action = None;
        if self.cursor_col > 0 {
            let line = &mut self.lines[self.cursor_row];
            let prev_char = line[..self.cursor_col].chars().last().unwrap();
            line.remove(self.cursor_col - prev_char.len_utf8());
            self.cursor_col -= prev_char.len_utf8();
        } else if self.cursor_row > 0 {
            let rest = self.lines.remove(self.cursor_row);
            self.cursor_row -= 1;
            self.cursor_col = self.lines[self.cursor_row].len();
            self.lines[self.cursor_row].push_str(&rest);
        }

        // Update or re-trigger autocomplete after backspace
        let before = self.text_before_cursor();
        if self.is_in_slash_command_context(&before) || self.is_in_trigger_context(&before) {
            if self.autocomplete_state.is_some() {
                self.update_autocomplete();
            } else {
                self.try_trigger_autocomplete();
            }
        } else {
            self.cancel_autocomplete();
        }

        self.notify_change();
    }

    fn delete_forward(&mut self) {
        self.exit_history();
        self.push_undo();
        self.last_action = None;
        if self.cursor_col < self.lines[self.cursor_row].len() {
            let col = self.cursor_col;
            let ch = self.lines[self.cursor_row][col..].chars().next().unwrap();
            self.lines[self.cursor_row].remove(col);
            let _ = ch;
        } else if self.cursor_row + 1 < self.lines.len() {
            let next = self.lines.remove(self.cursor_row + 1);
            self.lines[self.cursor_row].push_str(&next);
        }
        if self.autocomplete_state.is_some() {
            self.update_autocomplete();
        }
        self.notify_change();
    }

    fn kill_to_end_of_line(&mut self) {
        self.exit_history();
        self.push_undo();
        self.cancel_autocomplete();
        let mut did_kill = false;
        if self.cursor_col < self.lines[self.cursor_row].len() {
            let deleted = self.lines[self.cursor_row][self.cursor_col..].to_string();
            let accumulate = self.last_action.as_deref() == Some("kill");
            self.kill_ring.push_simple(&deleted, false, accumulate);
            self.lines[self.cursor_row].truncate(self.cursor_col);
            did_kill = true;
        } else if self.cursor_row + 1 < self.lines.len() {
            let next = self.lines.remove(self.cursor_row + 1);
            let accumulate = self.last_action.as_deref() == Some("kill");
            self.kill_ring.push_simple("\n", false, accumulate);
            self.lines[self.cursor_row].push_str(&next);
            did_kill = true;
        }
        self.last_action = if did_kill { Some("kill".into()) } else { self.last_action.take() };
        self.notify_change();
    }

    fn kill_to_start_of_line(&mut self) {
        self.exit_history();
        self.push_undo();
        self.cancel_autocomplete();
        let mut did_kill = false;
        if self.cursor_col > 0 {
            let deleted = self.lines[self.cursor_row][..self.cursor_col].to_string();
            let accumulate = self.last_action.as_deref() == Some("kill");
            self.kill_ring.push_simple(&deleted, true, accumulate);
            self.lines[self.cursor_row] = self.lines[self.cursor_row][self.cursor_col..].to_string();
            self.cursor_col = 0;
            did_kill = true;
        } else if self.cursor_row > 0 {
            let accumulate = self.last_action.as_deref() == Some("kill");
            self.kill_ring.push_simple("\n", true, accumulate);
            let rest = self.lines.remove(self.cursor_row);
            self.cursor_row -= 1;
            self.cursor_col = self.lines[self.cursor_row].len();
            self.lines[self.cursor_row].push_str(&rest);
            did_kill = true;
        }
        self.last_action = if did_kill { Some("kill".into()) } else { self.last_action.take() };
        self.notify_change();
    }

    fn kill_word_backward(&mut self) {
        self.exit_history();
        self.push_undo();
        self.cancel_autocomplete();
        let line = self.lines[self.cursor_row].clone();
        let col = find_word_backward(&line, self.cursor_col, None);
        if col < self.cursor_col {
            let deleted = line[col..self.cursor_col].to_string();
            let accumulate = self.last_action.as_deref() == Some("kill");
            self.kill_ring.push_simple(&deleted, true, accumulate);
            self.lines[self.cursor_row] = format!("{}{}", &line[..col], &line[self.cursor_col..]);
            self.cursor_col = col;
        }
        self.last_action = Some("kill".into());
        self.notify_change();
    }

    fn kill_word_forward(&mut self) {
        self.exit_history();
        self.push_undo();
        self.cancel_autocomplete();
        let line = self.lines[self.cursor_row].clone();
        let col = find_word_forward(&line, self.cursor_col, None);
        if col > self.cursor_col {
            let deleted = line[self.cursor_col..col].to_string();
            let accumulate = self.last_action.as_deref() == Some("kill");
            self.kill_ring.push_simple(&deleted, false, accumulate);
            self.lines[self.cursor_row] = format!("{}{}", &line[..self.cursor_col], &line[col..]);
        }
        self.last_action = Some("kill".into());
        self.notify_change();
    }

    fn yank_text(&mut self) {
        self.exit_history();
        self.push_undo();
        self.cancel_autocomplete();
        if let Some(text) = self.kill_ring.yank() {
            self.lines[self.cursor_row].insert_str(self.cursor_col, &text);
            self.cursor_col += text.len();
            self.last_action = Some("yank".into());
            self.notify_change();
        }
    }

    fn cursor_to_line_start(&mut self) {
        self.last_action = None;
        self.cursor_col = 0;
        self.preferred_visual_col.set(None);
        self.snapped_from_cursor_col.set(None);
    }
    fn cursor_to_line_end(&mut self) {
        self.last_action = None;
        self.cursor_col = self.lines[self.cursor_row].len();
        self.preferred_visual_col.set(None);
        self.snapped_from_cursor_col.set(None);
    }

    fn cursor_word_backward(&mut self) {
        self.last_action = None;
        let col = find_word_backward(&self.lines[self.cursor_row], self.cursor_col, None);
        self.cursor_col = col;
    }

    fn cursor_word_forward(&mut self) {
        self.last_action = None;
        let col = find_word_forward(&self.lines[self.cursor_row], self.cursor_col, None);
        self.cursor_col = col;
    }

    fn nav_history(&mut self, direction: i8) {
        if self.history.is_empty() {
            return;
        }
        let new = self.history_index - direction as isize;
        if new < -1 || new >= self.history.len() as isize {
            return;
        }
        if self.history_index == -1 && new >= 0 {
            self.push_undo();
            self.history_draft = Some(EditorState {
                lines: self.lines.clone(),
                cursor_row: self.cursor_row,
                cursor_col: self.cursor_col,
            });
        }
        self.history_index = new;
        if self.history_index == -1 {
            if let Some(draft) = self.history_draft.take() {
                self.lines = draft.lines;
                self.cursor_row = draft.cursor_row;
                self.cursor_col = draft.cursor_col;
                self.scroll_offset.set(0);
            } else {
                self.set_text("");
            }
        } else {
            self.set_text(&self.history[self.history_index as usize].clone());
        }
        self.notify_change();
    }

    fn handle_paste(&mut self, content: &str) {
        self.exit_history();
        self.push_undo();
        self.last_action = None;
        self.cancel_autocomplete();
        let clean: String = content
            .replace("\r\n", "\n")
            .replace('\r', "\n")
            .chars()
            .filter(|c| *c == '\n' || !c.is_ascii_control() || *c == '\t')
            .collect();
        let lines: Vec<&str> = clean.split('\n').collect();
        if lines.len() > 10 || clean.len() > 1000 {
            self.paste_counter += 1;
            let id = self.paste_counter;
            let marker = if lines.len() > 10 {
                format!("[paste #{} +{} lines]", id, lines.len())
            } else {
                format!("[paste #{} {} chars]", id, clean.len())
            };
            self.pastes.insert(id, PasteEntry { content: clean });
            self.lines[self.cursor_row].insert_str(self.cursor_col, &marker);
            self.cursor_col += marker.len();
        } else if lines.len() == 1 {
            self.lines[self.cursor_row].insert_str(self.cursor_col, &clean);
            self.cursor_col += clean.len();
        } else {
            let line = self.lines[self.cursor_row].clone();
            let before: String = line[..self.cursor_col].into();
            let after: String = line[self.cursor_col..].into();
            self.lines[self.cursor_row] = format!("{}{}", before, lines[0]);
            for (i, l) in lines.iter().enumerate().skip(1) {
                if i < lines.len() - 1 {
                    self.lines.insert(self.cursor_row + i, l.to_string());
                } else {
                    self.lines.insert(self.cursor_row + i, format!("{}{}", l, after));
                }
            }
            self.cursor_row += lines.len() - 1;
            self.cursor_col = lines.last().map(|l| l.len()).unwrap_or(0);
        }
        self.notify_change();
    }
}

impl Focusable for Editor {
    fn set_focused(&mut self, focused: bool) {
        self.focused = focused;
    }
    fn is_focused(&self) -> bool {
        self.focused
    }
}

impl EditorComponent for Editor {
    fn get_text(&self) -> String {
        self.get_text()
    }
    fn set_text(&mut self, text: &str) {
        self.set_text(text);
    }
    fn on_submit(&mut self, text: &str) {
        if let Some(ref mut cb) = self.on_submit {
            cb(text);
        }
    }
    fn on_change(&mut self, text: &str) {
        if let Some(ref mut cb) = self.on_change {
            cb(text);
        }
    }
}
