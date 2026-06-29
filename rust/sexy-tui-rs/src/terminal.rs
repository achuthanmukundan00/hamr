/// Terminal abstraction layer. Port of src/terminal.ts (531 lines).
use std::io::{self, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use crossterm::cursor;
use crossterm::event::{self, Event, KeyEventKind, MouseEventKind};
use crossterm::execute;
use crossterm::terminal::{self, ClearType};

use crate::keys::set_kitty_protocol_active;
use crate::terminal_colors::is_osc11_background_color_response;
use crate::terminal_image::get_capabilities;

const KITTY_KEYBOARD_PROTOCOL_QUERY: &str = "\x1b[>7u\x1b[?u\x1b[c";
/// Poll interval for the input loop shutdown check.
const POLL_TIMEOUT_MS: u64 = 50;

/// Mouse scroll event direction.
#[derive(Debug, Clone, Copy)]
pub enum MouseScroll {
    Up,
    Down,
}

/// Trait for terminal I/O implementations.
pub trait Terminal {
    /// Start the terminal with input and resize handlers.
    fn start(&mut self, on_input: Box<dyn FnMut(&str)>, on_resize: Box<dyn FnMut()>);

    /// Stop the terminal and restore state.
    fn stop(&mut self);

    /// Write data to the terminal.
    fn write(&mut self, data: &str);

    /// Get current terminal width in columns.
    fn columns(&self) -> u16;

    /// Get current terminal height in rows.
    fn rows(&self) -> u16;

    /// Update cached terminal dimensions after a resize event.
    fn set_size(&mut self, columns: u16, rows: u16);

    /// Move cursor by N lines (negative = up).
    fn move_by(&mut self, lines: i16);

    /// Hide the terminal cursor.
    fn hide_cursor(&mut self);

    /// Show the terminal cursor.
    fn show_cursor(&mut self);

    /// Clear the current line.
    fn clear_line(&mut self);

    /// Clear from cursor position to end of screen.
    fn clear_from_cursor(&mut self);

    /// Clear the entire screen.
    fn clear_screen(&mut self);

    /// Enable mouse capture with SGR extended mode (1006).
    fn enable_mouse_capture(&mut self);

    /// Disable mouse capture.
    fn disable_mouse_capture(&mut self);

    /// Enter alternate screen (1049h).
    fn enter_alternate_screen(&mut self);

    /// Leave alternate screen (1049l).
    fn leave_alternate_screen(&mut self);

    /// Set terminal window title (OSC 0).
    fn set_title(&mut self, title: &str);

    /// Set terminal progress indicator (OSC 9;4).
    fn set_progress(&mut self, active: bool);

    /// Drain stdin before exit to prevent key events leaking to parent shell.
    fn drain_input(&mut self, max_ms: u64, idle_ms: u64);
}

/// Production terminal implementation using crossterm.
pub struct ProcessTerminal {
    stdout: io::Stdout,
    columns: u16,
    rows: u16,
    raw_mode: bool,
    shutdown: Arc<AtomicBool>,
    input_thread: Option<JoinHandle<()>>,
    kitty_negotiation_buffer: String,
}

impl ProcessTerminal {
    pub fn new() -> io::Result<Self> {
        let (cols, rows) = terminal::size()?;
        Ok(ProcessTerminal {
            stdout: io::stdout(),
            columns: cols,
            rows,
            raw_mode: false,
            shutdown: Arc::new(AtomicBool::new(false)),
            input_thread: None,
            kitty_negotiation_buffer: String::new(),
        })
    }

    /// Negotiate Kitty keyboard protocol support.
    /// Sends the query and returns true if terminal is likely to support it.
    /// Actual response parsing happens in the input loop.
    fn negotiate_kitty_keyboard(&self) -> bool {
        let caps = get_capabilities();
        if caps.kitty_keyboard {
            let mut stdout = io::stdout();
            let _ = execute!(
                stdout,
                crossterm::style::Print(KITTY_KEYBOARD_PROTOCOL_QUERY)
            );
            let _ = stdout.flush();
            return true;
        }
        false
    }
}

/// Parse a Kitty keyboard protocol negotiation response.
/// Returns Some(flags) for `ESC[?Nu`, or None for unrecognised.
pub fn parse_keyboard_protocol_negotiation_sequence(sequence: &str) -> Option<u32> {
    // Kitty flags: ESC [ ? N u
    if let Some(rest) = sequence.strip_prefix("\x1b[?") {
        if let Some(flags_str) = rest.strip_suffix('u') {
            if let Ok(flags) = flags_str.parse::<u32>() {
                return Some(flags);
            }
        }
    }
    // Device attributes: ESC [ ? N ; M c — indicates terminal doesn't support Kitty
    // We return None; caller falls back to modifyOtherKeys
    None
}

/// Check if a partial sequence might be the start of a Kitty negotiation response.
pub fn is_keyboard_protocol_negotiation_sequence_prefix(sequence: &str) -> bool {
    sequence == "\x1b["
        || sequence.starts_with("\x1b[?") && !sequence.ends_with('u') && !sequence.ends_with('c')
}

impl Terminal for ProcessTerminal {
    fn start(&mut self, mut on_input: Box<dyn FnMut(&str)>, mut on_resize: Box<dyn FnMut()>) {
        // Enable raw mode
        terminal::enable_raw_mode().expect("Failed to enable raw mode");
        self.raw_mode = true;

        // Hide cursor
        execute!(self.stdout, cursor::Hide).ok();

        // Enable bracketed paste
        execute!(self.stdout, crossterm::style::Print("\x1b[?2004h")).ok();

        // Negotiate Kitty keyboard protocol
        let kitty = self.negotiate_kitty_keyboard();
        set_kitty_protocol_active(kitty);

        // Spawn input reader thread with shutdown signalling
        let (tx, rx) = mpsc::channel();
        let tx_for_thread = tx.clone(); // clone for the thread; keep tx for drop signalling
        let shutdown_flag = Arc::clone(&self.shutdown);

        let handle = thread::spawn(move || {
            loop {
                // Check shutdown flag before blocking on event::read
                if shutdown_flag.load(Ordering::Relaxed) {
                    break;
                }
                // Poll with timeout so we can check the shutdown flag periodically
                if let Ok(true) = event::poll(Duration::from_millis(POLL_TIMEOUT_MS)) {
                    if let Ok(event) = event::read() {
                        match event {
                            Event::Key(key_event) => {
                                if key_event.kind == KeyEventKind::Repeat {
                                    continue;
                                }
                                let data = key_to_string(&key_event);
                                if tx_for_thread.send(data).is_err() {
                                    break;
                                }
                            }
                            Event::Resize(cols, rows)
                                if tx_for_thread
                                    .send(format!("RESIZE:{}:{}", cols, rows))
                                    .is_err() =>
                            {
                                break;
                            }
                            Event::Mouse(mouse_event) => match mouse_event.kind {
                                MouseEventKind::ScrollUp => {
                                    if tx_for_thread.send("MOUSE_SCROLL_UP".to_string()).is_err() {
                                        break;
                                    }
                                }
                                MouseEventKind::ScrollDown => {
                                    if tx_for_thread.send("MOUSE_SCROLL_DOWN".to_string()).is_err()
                                    {
                                        break;
                                    }
                                }
                                _ => {}
                            },
                            _ => {}
                        }
                    }
                }
            }
        });
        self.input_thread = Some(handle);

        // Drop our sender clone so the receiver loop can detect
        // when the input thread has stopped.
        drop(tx);

        // Process input events in a loop that checks the shutdown flag.
        // The receiver will yield None when all senders are dropped
        // (i.e. the input thread exited).
        loop {
            // Check shutdown flag so we don't block forever if the
            // input thread is still running but we've been told to stop.
            if self.shutdown.load(Ordering::Relaxed) {
                break;
            }
            match rx.recv_timeout(Duration::from_millis(POLL_TIMEOUT_MS)) {
                Ok(data) => {
                    if data.starts_with("RESIZE:") {
                        let parts: Vec<&str> = data.split(':').collect();
                        if parts.len() == 3 {
                            if let (Ok(cols), Ok(rows)) = (parts[1].parse(), parts[2].parse()) {
                                self.columns = cols;
                                self.rows = rows;
                                on_resize();
                            }
                        }
                    } else {
                        if is_osc11_background_color_response(&data) {
                            continue;
                        }

                        // Check for Kitty keyboard protocol negotiation response
                        let combined = if !self.kitty_negotiation_buffer.is_empty() {
                            let c = format!("{}{}", self.kitty_negotiation_buffer, data);
                            self.kitty_negotiation_buffer.clear();
                            c
                        } else {
                            data.clone()
                        };

                        if let Some(flags) = parse_keyboard_protocol_negotiation_sequence(&combined)
                        {
                            if flags != 0 {
                                set_kitty_protocol_active(true);
                            }
                            continue;
                        }

                        if is_keyboard_protocol_negotiation_sequence_prefix(&combined) {
                            self.kitty_negotiation_buffer = combined;
                            continue;
                        }

                        on_input(&combined);
                    }
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    // No event yet — loop back to check shutdown flag
                    continue;
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    // Input thread exited — clean shutdown
                    break;
                }
            }
        }
    }

    fn stop(&mut self) {
        // Signal the input thread to exit
        self.shutdown.store(true, Ordering::Release);
        // Restore terminal state
        execute!(self.stdout, cursor::Show).ok();
        execute!(self.stdout, crossterm::style::Print("\x1b[?2004l")).ok();
        terminal::disable_raw_mode().ok();
        self.raw_mode = false;
        // Wait for the input thread to finish
        if let Some(handle) = self.input_thread.take() {
            let _ = handle.join();
        }
    }

    fn write(&mut self, data: &str) {
        let _ = self.stdout.write_all(data.as_bytes());
        let _ = self.stdout.flush();
    }

    fn columns(&self) -> u16 {
        self.columns
    }
    fn rows(&self) -> u16 {
        self.rows
    }

    fn set_size(&mut self, columns: u16, rows: u16) {
        self.columns = columns;
        self.rows = rows;
    }

    fn move_by(&mut self, lines: i16) {
        if lines < 0 {
            execute!(self.stdout, cursor::MoveUp((-lines) as u16)).ok();
        } else {
            execute!(self.stdout, cursor::MoveDown(lines as u16)).ok();
        }
    }

    fn hide_cursor(&mut self) {
        execute!(self.stdout, cursor::Hide).ok();
    }

    fn show_cursor(&mut self) {
        execute!(self.stdout, cursor::Show).ok();
    }

    fn clear_line(&mut self) {
        execute!(
            self.stdout,
            crossterm::terminal::Clear(ClearType::CurrentLine)
        )
        .ok();
    }

    fn clear_from_cursor(&mut self) {
        execute!(
            self.stdout,
            crossterm::terminal::Clear(ClearType::FromCursorDown)
        )
        .ok();
    }

    fn clear_screen(&mut self) {
        execute!(self.stdout, crossterm::terminal::Clear(ClearType::All)).ok();
    }

    fn enable_mouse_capture(&mut self) {
        // SGR extended mouse mode: \x1b[?1006h
        // This gives scroll events as buttons 64 (up) and 65 (down)
        execute!(self.stdout, crossterm::event::EnableMouseCapture).ok();
        execute!(self.stdout, crossterm::style::Print("\x1b[?1006h")).ok();
    }

    fn disable_mouse_capture(&mut self) {
        execute!(self.stdout, crossterm::event::DisableMouseCapture).ok();
    }

    fn enter_alternate_screen(&mut self) {
        execute!(self.stdout, crossterm::style::Print("\x1b[?1049h")).ok();
        // Also enable SGR mouse in alternate screen
        self.enable_mouse_capture();
    }

    fn leave_alternate_screen(&mut self) {
        self.disable_mouse_capture();
        execute!(self.stdout, crossterm::style::Print("\x1b[?1049l")).ok();
    }

    fn set_title(&mut self, title: &str) {
        execute!(
            self.stdout,
            crossterm::style::Print(format!("\x1b]0;{}\x07", title))
        )
        .ok();
    }

    fn set_progress(&mut self, active: bool) {
        let sequence = if active {
            "\x1b]9;4;3\x07"
        } else {
            "\x1b]9;4;0;\x07"
        };
        execute!(self.stdout, crossterm::style::Print(sequence)).ok();
    }

    fn drain_input(&mut self, max_ms: u64, idle_ms: u64) {
        self.shutdown.store(true, Ordering::Release);
        // Wait briefly for the input thread to finish
        let start = std::time::Instant::now();
        while start.elapsed().as_millis() < max_ms as u128 {
            if self.input_thread.is_none() {
                break;
            }
            std::thread::sleep(Duration::from_millis(idle_ms.min(10)));
        }
        if let Some(handle) = self.input_thread.take() {
            let _ = handle.join();
        }
    }
}

/// Convert a crossterm KeyEvent to a string representation.
pub fn key_to_string(event: &event::KeyEvent) -> String {
    use crossterm::event::{KeyCode, KeyModifiers};

    let mut result = String::new();

    // Kitty protocol format: ESC [ codepoint ; modifier u
    // Crossterm has already applied Shift to printable characters. Returning
    // those characters directly keeps normal text entry (`A`, `?`, etc.)
    // working while reserving CSI-u encoding for actual modified shortcuts.
    let shortcut_modifiers =
        event.modifiers & (KeyModifiers::CONTROL | KeyModifiers::ALT | KeyModifiers::SUPER);
    if !shortcut_modifiers.is_empty()
        || (event.modifiers.contains(KeyModifiers::SHIFT)
            && !matches!(event.code, KeyCode::Char(_)))
    {
        let mut mod_val = 1u8; // 1-indexed
        if event.modifiers.contains(KeyModifiers::SHIFT) {
            mod_val += 1;
        }
        if event.modifiers.contains(KeyModifiers::ALT) {
            mod_val += 2;
        }
        if event.modifiers.contains(KeyModifiers::CONTROL) {
            mod_val += 4;
        }
        if event.modifiers.contains(KeyModifiers::SUPER) {
            mod_val += 8;
        }

        match event.code {
            KeyCode::Char(c) => {
                return format!("\x1b[{};{}u", c as u32, mod_val);
            }
            KeyCode::Enter => return format!("\x1b[13;{}u", mod_val),
            KeyCode::Tab => return format!("\x1b[9;{}u", mod_val),
            KeyCode::Backspace => return format!("\x1b[127;{}u", mod_val),
            KeyCode::Esc => return format!("\x1b[27;{}u", mod_val),
            _ => {}
        }
    }

    // Plain key events
    match event.code {
        KeyCode::Char(c) => result.push(c),
        KeyCode::Enter => result.push('\r'),
        KeyCode::Tab => result.push('\t'),
        KeyCode::Backspace => result.push('\x7f'),
        KeyCode::Esc => result.push('\x1b'),
        KeyCode::Up => result.push_str("\x1b[A"),
        KeyCode::Down => result.push_str("\x1b[B"),
        KeyCode::Left => result.push_str("\x1b[D"),
        KeyCode::Right => result.push_str("\x1b[C"),
        KeyCode::Home => result.push_str("\x1b[H"),
        KeyCode::End => result.push_str("\x1b[F"),
        KeyCode::Delete => result.push_str("\x1b[3~"),
        KeyCode::Insert => result.push_str("\x1b[2~"),
        KeyCode::PageUp => result.push_str("\x1b[5~"),
        KeyCode::PageDown => result.push_str("\x1b[6~"),
        KeyCode::F(n) if n <= 12 => {
            result.push_str(&format!(
                "\x1b[{}~",
                match n {
                    1 => "11",
                    2 => "12",
                    3 => "13",
                    4 => "14",
                    5 => "15",
                    6 => "17",
                    7 => "18",
                    8 => "19",
                    9 => "20",
                    10 => "21",
                    11 => "23",
                    12 => "24",
                    _ => unreachable!(),
                }
            ));
        }
        _ => {}
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    #[test]
    fn shifted_printable_characters_are_returned_directly() {
        let event = KeyEvent::new(KeyCode::Char('A'), KeyModifiers::SHIFT);
        assert_eq!(key_to_string(&event), "A");

        let event = KeyEvent::new(KeyCode::Char('?'), KeyModifiers::SHIFT);
        assert_eq!(key_to_string(&event), "?");
    }

    #[test]
    fn ctrl_characters_are_encoded_as_csi_u_shortcuts() {
        let event = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
        assert_eq!(key_to_string(&event), "\x1b[99;5u");
        assert!(crate::keys::matches_key(&key_to_string(&event), "ctrl+c"));

        let event = KeyEvent::new(KeyCode::Char('d'), KeyModifiers::CONTROL);
        assert_eq!(key_to_string(&event), "\x1b[100;5u");
        assert!(crate::keys::matches_key(&key_to_string(&event), "ctrl+d"));
    }
}
