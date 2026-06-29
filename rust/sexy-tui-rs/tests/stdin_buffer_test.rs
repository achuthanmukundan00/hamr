//! Ported from packages/tui/test/stdin-buffer.test.ts
//!
//! Tests for stdin buffer handling: bracketed paste, escape sequences,
//! multi-byte UTF-8, and buffer edge cases.

use sexy_tui_rs::stdin_buffer::{StdinBuffer, StdinBufferOptions};

const PASTE_START: &str = "\x1b[200~";
const PASTE_END: &str = "\x1b[201~";

fn new_buffer() -> StdinBuffer {
    StdinBuffer::new(StdinBufferOptions::default())
}

// =============================================================================
// Basic input
// =============================================================================

#[test]
fn test_plain_text_returns_on_newline() {
    let mut buf = new_buffer();
    let results = buf.feed("hello\nworld\n");
    assert_eq!(results, vec!["hello\n", "world\n"]);
}

#[test]
fn test_no_newline_stays_in_buffer() {
    let mut buf = new_buffer();
    let results = buf.feed("hello");
    assert!(results.is_empty());
    let flushed = buf.flush();
    assert_eq!(flushed, vec!["hello"]);
}

#[test]
fn test_multiple_lines() {
    let mut buf = new_buffer();
    let results = buf.feed("line1\nline2\nline3\n");
    assert_eq!(results, vec!["line1\n", "line2\n", "line3\n"]);
}

#[test]
fn test_single_char_input() {
    let mut buf = new_buffer();
    let results = buf.feed("a");
    assert!(results.is_empty());
    let flushed = buf.flush();
    assert_eq!(flushed, vec!["a"]);
}

// =============================================================================
// Bracketed paste — small content
// =============================================================================

#[test]
fn test_small_paste_is_returned_as_is() {
    let mut buf = new_buffer();
    let data = format!("{PASTE_START}small paste{PASTE_END}");
    let results = buf.feed(&data);
    assert_eq!(results, vec!["small paste"]);
}

#[test]
fn test_small_paste_with_newlines() {
    let mut buf = new_buffer();
    let data = format!("{PASTE_START}line1\nline2\nline3{PASTE_END}");
    let results = buf.feed(&data);
    // Small paste (≤10 lines) is returned as-is
    assert_eq!(results, vec!["line1\nline2\nline3"]);
}

#[test]
fn test_paste_marker_removed() {
    let mut buf = new_buffer();
    let data = format!("{PASTE_START}content{PASTE_END}");
    let results = buf.feed(&data);
    assert!(results.iter().all(|r| !r.contains("\x1b[200~")));
    assert!(results.iter().all(|r| !r.contains("\x1b[201~")));
}

// =============================================================================
// Bracketed paste — large content
// =============================================================================

#[test]
fn test_large_paste_returns_marker() {
    let mut buf = new_buffer();
    let lines: Vec<String> = (1..=15).map(|i| format!("line {}", i)).collect();
    let content = lines.join("\n");
    let data = format!("{PASTE_START}{content}{PASTE_END}");
    let results = buf.feed(&data);
    assert_eq!(results.len(), 1);
    assert!(results[0].starts_with("[paste #"));
    assert!(results[0].contains("+15 lines"));
}

#[test]
fn test_large_paste_marker_includes_line_count() {
    let mut buf = new_buffer();
    let lines: Vec<String> = (1..=25).map(|i| format!("line {}", i)).collect();
    let content = lines.join("\n");
    let data = format!("{PASTE_START}{content}{PASTE_END}");
    let results = buf.feed(&data);
    assert_eq!(results.len(), 1);
    assert!(results[0].contains("+25 lines"));
}

#[test]
fn test_exactly_eleven_lines_returns_marker() {
    let mut buf = new_buffer();
    let lines: Vec<String> = (1..=11).map(|i| format!("line {}", i)).collect();
    let content = lines.join("\n");
    let data = format!("{PASTE_START}{content}{PASTE_END}");
    let results = buf.feed(&data);
    assert_eq!(results.len(), 1);
    assert!(results[0].starts_with("[paste #"));
    assert!(results[0].contains("+11 lines"));
}

#[test]
fn test_exactly_ten_lines_returns_as_is() {
    let mut buf = new_buffer();
    let lines: Vec<String> = (1..=10).map(|i| format!("line {}", i)).collect();
    let content = lines.join("\n");
    let data = format!("{PASTE_START}{content}{PASTE_END}");
    let results = buf.feed(&data);
    assert_eq!(results.len(), 1);
    assert_eq!(results[0], content);
}

// =============================================================================
// Bracketed paste — with surrounding text
// =============================================================================

#[test]
fn test_text_before_and_after_paste() {
    let mut buf = new_buffer();
    let data = format!("prefix\n{PASTE_START}pasted{PASTE_END}\nsuffix\n");
    let results = buf.feed(&data);
    // After paste end, the trailing \n+suffix\n is processed character by character
    assert_eq!(results.len(), 4);
    assert_eq!(results[0], "prefix\n");
    assert_eq!(results[1], "pasted");
    assert_eq!(results[2], "\n");
    assert_eq!(results[3], "suffix\n");
}

#[test]
fn test_multiple_small_pastes() {
    let mut buf = new_buffer();
    let data = format!("{PASTE_START}first{PASTE_END}\n{PASTE_START}second{PASTE_END}\n");
    let results = buf.feed(&data);
    assert_eq!(results, vec!["first", "\n", "second", "\n"]);
}

#[test]
fn test_multiple_large_pastes() {
    let mut buf = new_buffer();
    let lines: Vec<String> = (1..=15).map(|i| format!("line {}", i)).collect();
    let content = lines.join("\n");
    let data = format!("{PASTE_START}{content}{PASTE_END}\n{PASTE_START}{content}{PASTE_END}\n");
    let results = buf.feed(&data);
    assert_eq!(results.len(), 4);
    assert!(results[0].starts_with("[paste #1"));
    assert_eq!(results[1], "\n");
    assert!(results[2].starts_with("[paste #2"));
    assert_eq!(results[3], "\n");
}

// =============================================================================
// Escape sequences
// =============================================================================

#[test]
fn test_escape_sequence_passthrough() {
    let mut buf = new_buffer();
    // Arrow up sequence: ESC [ A
    let results = buf.feed("\x1b[A\n");
    assert_eq!(results, vec!["\x1b[A\n"]);
}

#[test]
fn test_csi_sequence_mid_line() {
    let mut buf = new_buffer();
    let results = buf.feed("text \x1b[31mred\x1b[0m text\n");
    assert_eq!(results, vec!["text \x1b[31mred\x1b[0m text\n"]);
}

#[test]
fn test_partial_escape_sequence_across_feeds() {
    let mut buf = new_buffer();
    // Feed ESC, then the rest later
    let results1 = buf.feed("\x1b");
    assert!(results1.is_empty());
    let results2 = buf.feed("[A\n");
    assert_eq!(results2, vec!["\x1b[A\n"]);
}

// =============================================================================
// UTF-8 handling
// =============================================================================

#[test]
fn test_multi_byte_utf8_characters() {
    let mut buf = new_buffer();
    let results = buf.feed("café résumé\n");
    assert_eq!(results, vec!["café résumé\n"]);
}

#[test]
fn test_cjk_characters() {
    let mut buf = new_buffer();
    let results = buf.feed("こんにちは世界\n");
    assert_eq!(results, vec!["こんにちは世界\n"]);
}

#[test]
fn test_emoji_characters() {
    let mut buf = new_buffer();
    let results = buf.feed("hello 👋 world 🌍\n");
    assert_eq!(results, vec!["hello 👋 world 🌍\n"]);
}

// =============================================================================
// Max buffer size
// =============================================================================

#[test]
fn test_max_buffer_flush() {
    let mut buf = StdinBuffer::new(StdinBufferOptions {
        flush_timeout_ms: 10,
        max_buffer_size: 5,
    });
    // Feed more characters than max_buffer_size
    let results = buf.feed("abcdefghij\n");
    // Should have flushed at least once due to max buffer
    assert!(!results.is_empty());
    // All characters should be present across all flushed chunks
    let all_text: String = results.join("");
    assert!(all_text.contains("abcdefghij"));
}

#[test]
fn test_empty_feed() {
    let mut buf = new_buffer();
    let results = buf.feed("");
    assert!(results.is_empty());
}

#[test]
fn test_flush_empty() {
    let mut buf = new_buffer();
    let results = buf.flush();
    assert!(results.is_empty());
}

// =============================================================================
// Edge cases
// =============================================================================

#[test]
fn test_unclosed_paste_is_flushed() {
    let mut buf = new_buffer();
    let results = buf.feed(&format!("{PASTE_START}content without end marker"));
    // Content stays in buffer since paste isn't closed
    assert!(results.is_empty());
    let flushed = buf.flush();
    assert!(!flushed.is_empty());
    assert!(flushed[0].contains("content without end marker"));
}

#[test]
fn test_nested_paste_markers() {
    let mut buf = new_buffer();
    // Start paste, then another start (edge case)
    let data = format!("{PASTE_START}content{PASTE_START}more{PASTE_END}");
    let results = buf.feed(&data);
    assert!(!results.is_empty());
}
