//! Virtual terminal for headless testing using the `vt100` crate for accurate
//! terminal emulation.
//!
//! Port of `packages/tui/test/virtual-terminal.ts`.
//!
//! Provides a virtual terminal that accepts ANSI-escaped write data, emulates
//! a real terminal, and exposes viewport contents for test assertions.
//!
//! Implements the [`Terminal`] trait using `Arc<Mutex<vt100::Parser>>` so
//! a test can clone the terminal, pass one handle to [`TUI`], and inspect
//! the viewport through the other.

use std::sync::{Arc, Mutex};

use crate::terminal::Terminal;

/// A headless, cloneable terminal emulator for testing TUI rendering.
///
/// Wraps `vt100::Parser` behind an `Arc<Mutex<>>` so clones share the same
/// underlying parser. Tests create one handle for the TUI and keep another
/// to call `get_viewport()` after rendering.
pub struct VirtualTerminal {
    parser: Arc<Mutex<vt100::Parser>>,
    columns: u16,
    rows: u16,
}

impl Clone for VirtualTerminal {
    fn clone(&self) -> Self {
        VirtualTerminal {
            parser: Arc::clone(&self.parser),
            columns: self.columns,
            rows: self.rows,
        }
    }
}

impl VirtualTerminal {
    /// Create a new virtual terminal with the given dimensions.
    pub fn new(columns: u16, rows: u16) -> Self {
        let parser = vt100::Parser::new(rows, columns, 1000);
        VirtualTerminal {
            parser: Arc::new(Mutex::new(parser)),
            columns,
            rows,
        }
    }

    /// Write data (including ANSI escape sequences) to the terminal.
    fn write_internal(&mut self, data: &str) {
        self.parser.lock().unwrap().process(data.as_bytes());
    }

    /// Get the visible viewport as lines of plain text.
    pub fn get_viewport(&self) -> Vec<String> {
        let parser = self.parser.lock().unwrap();
        let screen = parser.screen();
        let contents = screen.contents();
        let lines: Vec<String> = contents.lines().map(|s| s.to_string()).collect();
        let mut result = lines;
        while result.len() < self.rows as usize {
            result.push(String::new());
        }
        result.truncate(self.rows as usize);
        result
    }

    /// Get the viewport with ANSI formatting attributes.
    pub fn get_viewport_formatted(&self) -> Vec<String> {
        let parser = self.parser.lock().unwrap();
        let screen = parser.screen();
        let formatted_bytes = screen.contents_formatted();
        let formatted = String::from_utf8_lossy(&formatted_bytes).to_string();
        formatted.lines().map(|s| s.to_string()).collect()
    }

    /// Get a single cell's character content at (row, col).
    pub fn cell(&self, row: u16, col: u16) -> Option<String> {
        let parser = self.parser.lock().unwrap();
        parser.screen().cell(row, col).map(|c| c.contents())
    }

    /// Check if a cell at (row, col) has a specific attribute.
    pub fn cell_has_attribute(&self, row: u16, col: u16, attr: CellAttribute) -> bool {
        let parser = self.parser.lock().unwrap();
        if let Some(cell) = parser.screen().cell(row, col) {
            match attr {
                CellAttribute::Bold => cell.bold(),
                CellAttribute::Italic => cell.italic(),
                CellAttribute::Underline => cell.underline(),
                CellAttribute::Reverse => cell.inverse(),
            }
        } else {
            false
        }
    }

    /// Resize the terminal.
    pub fn resize(&mut self, columns: u16, rows: u16) {
        self.columns = columns;
        self.rows = rows;
        self.parser.lock().unwrap().set_size(rows, columns);
    }

    /// Get current terminal dimensions.
    pub fn dimensions(&self) -> (u16, u16) {
        (self.columns, self.rows)
    }

    /// Get cursor position as (row, col).
    pub fn cursor_position(&self) -> (u16, u16) {
        self.parser.lock().unwrap().screen().cursor_position()
    }

    /// Clear the terminal screen.
    pub fn clear(&mut self) {
        self.write_internal("\x1b[2J\x1b[H");
    }

    /// Get the entire scrollback buffer.
    pub fn get_scroll_buffer(&self) -> Vec<String> {
        let parser = self.parser.lock().unwrap();
        let screen = parser.screen();
        let formatted_bytes = screen.contents_formatted();
        let formatted = String::from_utf8_lossy(&formatted_bytes).to_string();
        formatted.lines().map(|s| s.to_string()).collect()
    }

    /// Wait for render — synchronous in this implementation.
    pub async fn wait_for_render(&self) {
        // vt100 processes synchronously
    }
}

/// Attributes that can be checked on a terminal cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CellAttribute {
    Bold,
    Italic,
    Underline,
    Reverse,
}

impl Default for VirtualTerminal {
    fn default() -> Self {
        VirtualTerminal::new(80, 24)
    }
}

// ---------------------------------------------------------------------------
// Terminal trait impl — enables TUI to use VirtualTerminal as a backend
// ---------------------------------------------------------------------------

impl Terminal for VirtualTerminal {
    fn start(&mut self, _on_input: Box<dyn FnMut(&str)>, _on_resize: Box<dyn FnMut()>) {}

    fn stop(&mut self) {}

    fn write(&mut self, data: &str) {
        self.write_internal(data);
    }

    fn columns(&self) -> u16 {
        self.columns
    }

    fn rows(&self) -> u16 {
        self.rows
    }

    fn set_size(&mut self, columns: u16, rows: u16) {
        self.resize(columns, rows);
    }

    fn move_by(&mut self, lines: i16) {
        if lines > 0 {
            self.write_internal(&format!("\x1b[{}B", lines));
        } else if lines < 0 {
            self.write_internal(&format!("\x1b[{}A", -lines));
        }
    }

    fn hide_cursor(&mut self) {
        self.write_internal("\x1b[?25l");
    }

    fn show_cursor(&mut self) {
        self.write_internal("\x1b[?25h");
    }

    fn clear_line(&mut self) {
        self.write_internal("\x1b[K");
    }

    fn clear_from_cursor(&mut self) {
        self.write_internal("\x1b[J");
    }

    fn clear_screen(&mut self) {
        self.clear();
    }

    fn enable_mouse_capture(&mut self) {}
    fn disable_mouse_capture(&mut self) {}
    fn enter_alternate_screen(&mut self) {
        self.write_internal("\x1b[?1049h");
    }
    fn leave_alternate_screen(&mut self) {
        self.write_internal("\x1b[?1049l");
    }
    fn set_title(&mut self, _title: &str) {}
    fn set_progress(&mut self, _active: bool) {}
    fn drain_input(&mut self, _max_ms: u64, _idle_ms: u64) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_terminal() {
        let vt = VirtualTerminal::new(80, 24);
        assert_eq!(vt.dimensions(), (80, 24));
    }

    #[test]
    fn test_write_text() {
        let mut vt = VirtualTerminal::new(40, 5);
        vt.write_internal("Hello, World!");
        let viewport = vt.get_viewport();
        assert_eq!(viewport[0], "Hello, World!");
    }

    #[test]
    fn test_clear_screen() {
        let mut vt = VirtualTerminal::new(40, 5);
        vt.write_internal("Hello");
        vt.clear();
        assert!(vt.get_viewport()[0].is_empty());
    }

    #[test]
    fn test_newline() {
        let mut vt = VirtualTerminal::new(40, 5);
        vt.write_internal("Line 1\r\nLine 2");
        let viewport = vt.get_viewport();
        assert_eq!(viewport[0], "Line 1");
        assert_eq!(viewport[1], "Line 2");
    }

    #[test]
    fn test_cell_attribute() {
        let mut vt = VirtualTerminal::new(40, 5);
        vt.write_internal("\x1b[1mBold text\x1b[0m normal");
        assert!(vt.cell_has_attribute(0, 0, CellAttribute::Bold));
        assert!(!vt.cell_has_attribute(0, 12, CellAttribute::Bold));
    }

    #[test]
    fn test_resize() {
        let mut vt = VirtualTerminal::new(80, 24);
        vt.resize(120, 40);
        assert_eq!(vt.dimensions(), (120, 40));
    }

    #[test]
    fn test_cursor_position_after_write() {
        let mut vt = VirtualTerminal::new(40, 5);
        vt.write_internal("Hi");
        let (row, col) = vt.cursor_position();
        assert_eq!(col, 2);
        assert_eq!(row, 0);
    }

    #[test]
    fn test_clone_shares_parser() {
        let mut vt1 = VirtualTerminal::new(40, 5);
        let vt2 = vt1.clone();

        vt1.write_internal("Hello from vt1");
        let viewport = vt2.get_viewport();
        assert_eq!(viewport[0], "Hello from vt1");
    }
}
