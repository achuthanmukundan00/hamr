/// Single-line text input widget.
/// Pi-identical port of src/components/input.ts (447 lines).
use std::cell::Cell;

use crate::kill_ring::KillRing;
use crate::tui::{Component, Focusable, CURSOR_MARKER};
use crate::undo_stack::UndoStack;
use crate::utils::visible_width;
use crate::word_navigation::{find_word_backward, find_word_forward};

#[derive(Clone)]
struct InputState {
    text: String,
    cursor: usize,
}

pub struct Input {
    text: String,
    cursor: usize,
    focused: bool,
    scroll_offset: Cell<usize>,
    cached_width: Cell<Option<u16>>,
    cached_lines: Cell<Option<Vec<String>>>,
    kill_ring: KillRing,
    undo_stack: UndoStack<InputState>,
    last_action: Option<String>,
    pub on_submit: Option<Box<dyn FnMut(&str)>>,
}

impl Input {
    pub fn new() -> Self {
        Input {
            text: String::new(),
            cursor: 0,
            focused: false,
            scroll_offset: Cell::new(0),
            cached_width: Cell::new(None),
            cached_lines: Cell::new(None),
            kill_ring: KillRing::new(),
            undo_stack: UndoStack::new(),
            last_action: None,
            on_submit: None,
        }
    }

    pub fn set_value(&mut self, value: &str) {
        self.text = value.to_string();
        self.cursor = self.text.len();
        self.scroll_offset.set(0);
        self.invalidate();
    }

    pub fn get_value(&self) -> &str {
        &self.text
    }

    fn push_undo(&mut self) {
        self.undo_stack.push(&InputState {
            text: self.text.clone(),
            cursor: self.cursor,
        });
    }

    fn undo(&mut self) {
        if let Some(state) = self.undo_stack.pop() {
            self.text = state.text;
            self.cursor = state.cursor;
        }
    }

    fn delete_range(&mut self, start: usize, end: usize, prepend_kill: bool) {
        if start >= end {
            return;
        }
        let deleted = self.text[start..end].to_string();
        let accumulate = self.last_action.as_deref() == Some("kill");
        self.kill_ring
            .push_simple(&deleted, prepend_kill, accumulate);
        self.text = format!("{}{}", &self.text[..start], &self.text[end..]);
        self.cursor = start;
        self.last_action = Some("kill".into());
    }
}

impl Component for Input {
    fn render(&self, width: u16) -> Vec<String> {
        let max_visible = (width as usize).saturating_sub(2); // "> " prefix
        if max_visible == 0 {
            return vec![String::new()];
        }

        // Adjust scroll so cursor is visible (using Cell for interior mutability)
        let mut so = self.scroll_offset.get();
        let cursor_vis = self.cursor;
        if cursor_vis < so {
            so = cursor_vis;
        } else if cursor_vis >= so + max_visible {
            so = cursor_vis.saturating_sub(max_visible - 1);
        }
        self.scroll_offset.set(so);

        let visible_text = &self.text[so..];
        let truncated = crate::utils::truncate_to_width(visible_text, max_visible, None);
        let tw = visible_width(&truncated);

        let marker = if self.focused { CURSOR_MARKER } else { "" };
        let rel_cursor = self.cursor.saturating_sub(so).min(tw);

        let content = if rel_cursor < truncated.len() {
            let before = &truncated[..rel_cursor];
            let at = truncated[rel_cursor..].chars().next().unwrap_or(' ');
            let after = &truncated[rel_cursor + at.len_utf8()..];
            format!("> {}{}\x1b[7m{}\x1b[27m{}", before, marker, at, after)
        } else {
            format!("> {}{}\x1b[7m \x1b[27m", truncated, marker)
        };

        let lw = visible_width(&content);
        vec![format!(
            "{}{}",
            content,
            " ".repeat((width as usize).saturating_sub(lw))
        )]
    }

    fn handle_input(&mut self, data: &str) {
        use crate::keys::{matches_key, Key};

        // Submit
        if matches_key(data, Key::enter) {
            let val = self.text.clone();
            if let Some(ref mut cb) = self.on_submit {
                cb(&val);
            }
            return;
        }

        // Undo
        if matches_key(data, &Key::ctrl("z")) {
            self.undo();
            return;
        }

        // Kill ring deletions
        if matches_key(data, &Key::ctrl("k")) {
            self.push_undo();
            let end = self.text.len();
            self.delete_range(self.cursor, end, false);
            return;
        }
        if matches_key(data, &Key::ctrl("u")) {
            self.push_undo();
            self.delete_range(0, self.cursor, true);
            return;
        }
        if matches_key(data, &Key::ctrl("w")) || matches_key(data, &Key::alt("backspace")) {
            self.push_undo();
            let col = find_word_backward(&self.text, self.cursor, None);
            self.delete_range(col, self.cursor, true);
            return;
        }
        if matches_key(data, &Key::alt("d")) || matches_key(data, &Key::alt("delete")) {
            self.push_undo();
            let col = find_word_forward(&self.text, self.cursor, None);
            self.delete_range(self.cursor, col, false);
            return;
        }

        // Yank
        if matches_key(data, &Key::ctrl("y")) {
            self.push_undo();
            if let Some(yanked) = self.kill_ring.yank() {
                self.text.insert_str(self.cursor, &yanked);
                self.cursor += yanked.len();
                self.last_action = Some("yank".into());
            }
            return;
        }

        // Delete
        if matches_key(data, Key::backspace) || matches_key(data, "shift+backspace") {
            if self.cursor > 0 {
                self.push_undo();
                self.last_action = None;
                let prev = self.text[..self.cursor].chars().last().unwrap();
                self.text.remove(self.cursor - prev.len_utf8());
                self.cursor -= prev.len_utf8();
            }
            return;
        }
        if matches_key(data, Key::delete) || matches_key(data, "shift+delete") {
            if self.cursor < self.text.len() {
                self.push_undo();
                self.last_action = None;
                let ch = self.text[self.cursor..].chars().next().unwrap();
                self.text.remove(self.cursor);
                let _ = ch;
            }
            return;
        }

        // Cursor movement
        if matches_key(data, &Key::ctrl("a")) || matches_key(data, Key::home) {
            self.last_action = None;
            self.cursor = 0;
            return;
        }
        if matches_key(data, &Key::ctrl("e")) || matches_key(data, Key::end) {
            self.last_action = None;
            self.cursor = self.text.len();
            return;
        }
        if matches_key(data, &Key::ctrl("left")) || matches_key(data, &Key::alt("left")) {
            self.last_action = None;
            self.cursor = find_word_backward(&self.text, self.cursor, None);
            return;
        }
        if matches_key(data, &Key::ctrl("right")) || matches_key(data, &Key::alt("right")) {
            self.last_action = None;
            self.cursor = find_word_forward(&self.text, self.cursor, None);
            return;
        }
        if matches_key(data, Key::left) && self.cursor > 0 {
            self.last_action = None;
            self.cursor -= self.text[..self.cursor].chars().last().unwrap().len_utf8();
            return;
        }
        if matches_key(data, Key::right) && self.cursor < self.text.len() {
            self.last_action = None;
            self.cursor += self.text[self.cursor..].chars().next().unwrap().len_utf8();
            return;
        }

        // Printable
        if !data.starts_with('\x1b') && data.len() == 1 && !data.as_bytes()[0].is_ascii_control() {
            if data.chars().any(|c| c.is_whitespace())
                || self.last_action.as_deref() != Some("type")
            {
                self.push_undo();
            }
            self.last_action = Some("type".into());
            self.text.insert_str(self.cursor, data);
            self.cursor += data.len();
        }
        self.invalidate();
    }

    fn invalidate(&mut self) {
        self.cached_width.set(None);
        self.cached_lines.set(None);
        // Track approximate scroll position based on cursor
        self.scroll_offset.set(self.cursor.saturating_sub(20));
    }
}

impl Focusable for Input {
    fn set_focused(&mut self, focused: bool) {
        self.focused = focused;
    }
    fn is_focused(&self) -> bool {
        self.focused
    }
}

impl Default for Input {
    fn default() -> Self {
        Self::new()
    }
}
