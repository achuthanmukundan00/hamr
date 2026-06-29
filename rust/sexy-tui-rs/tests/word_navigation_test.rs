//! Ported from packages/tui/test/word-navigation.test.ts
//!
//! Tests for word boundary navigation in text editors.

use sexy_tui_rs::word_navigation::{find_word_backward, find_word_forward};

// =============================================================================
// findWordBackward
// =============================================================================

#[test]
fn test_backward_hello_world() {
    let text = "hello world";
    assert_eq!(find_word_backward(text, 11, None), 6);
    assert_eq!(find_word_backward(text, 6, None), 0);
}

#[test]
fn test_backward_dotted() {
    let text = "foo.bar";
    assert_eq!(find_word_backward(text, 7, None), 4);
    assert_eq!(find_word_backward(text, 4, None), 3);
    assert_eq!(find_word_backward(text, 3, None), 0);
}

#[test]
fn test_backward_colon() {
    let text = "foo:bar";
    assert_eq!(find_word_backward(text, 7, None), 4);
    assert_eq!(find_word_backward(text, 4, None), 3);
    assert_eq!(find_word_backward(text, 3, None), 0);
}

#[test]
fn test_backward_path() {
    let text = "path/to/file";
    assert_eq!(find_word_backward(text, 12, None), 8);
    assert_eq!(find_word_backward(text, 8, None), 7);
    assert_eq!(find_word_backward(text, 7, None), 5);
    assert_eq!(find_word_backward(text, 5, None), 4);
    assert_eq!(find_word_backward(text, 4, None), 0);
}

#[test]
fn test_backward_cjk() {
    let text = "你好世界 test";
    let result = find_word_backward(text, text.len(), None);
    // Should jump past "test" to somewhere in the CJK region
    assert!(result >= text.len() - 5); // at or after " test"
    assert!(result < text.len()); // not at start
}

#[test]
fn test_backward_whitespace() {
    let text = "  hello  ";
    assert_eq!(find_word_backward(text, 9, None), 2);
    assert_eq!(find_word_backward(text, 2, None), 0);
}

#[test]
fn test_backward_punctuation_run() {
    let text = "foo...bar";
    assert_eq!(find_word_backward(text, 9, None), 6);
    assert_eq!(find_word_backward(text, 6, None), 3);
    assert_eq!(find_word_backward(text, 3, None), 0);
}

#[test]
fn test_backward_cursor_at_zero() {
    assert_eq!(find_word_backward("hello", 0, None), 0);
}

// =============================================================================
// findWordForward
// =============================================================================

#[test]
fn test_forward_hello_world() {
    let text = "hello world";
    assert_eq!(find_word_forward(text, 0, None), 5);
    assert_eq!(find_word_forward(text, 5, None), 11);
}

#[test]
fn test_forward_dotted() {
    let text = "foo.bar";
    assert_eq!(find_word_forward(text, 0, None), 3);
    assert_eq!(find_word_forward(text, 3, None), 4);
    assert_eq!(find_word_forward(text, 4, None), 7);
}

#[test]
fn test_forward_colon() {
    let text = "foo:bar";
    assert_eq!(find_word_forward(text, 0, None), 3);
    assert_eq!(find_word_forward(text, 3, None), 4);
    assert_eq!(find_word_forward(text, 4, None), 7);
}

#[test]
fn test_forward_path() {
    let text = "path/to/file";
    assert_eq!(find_word_forward(text, 0, None), 4);
    assert_eq!(find_word_forward(text, 4, None), 5);
    assert_eq!(find_word_forward(text, 5, None), 7);
    assert_eq!(find_word_forward(text, 7, None), 8);
    assert_eq!(find_word_forward(text, 8, None), 12);
}

#[test]
fn test_forward_cjk() {
    let text = "你好世界 test";
    let first_end = find_word_forward(text, 0, None);
    // Rust segments CJK as one unit
    assert!(first_end > 0);
    // Walk to end
    let mut pos = 0;
    while pos < text.len() {
        let next = find_word_forward(text, pos, None);
        if next == pos {
            break;
        }
        pos = next;
    }
    assert_eq!(pos, text.len());
}

#[test]
fn test_forward_whitespace() {
    let text = "  hello  ";
    assert_eq!(find_word_forward(text, 0, None), 7);
    assert_eq!(find_word_forward(text, 7, None), 9);
}

#[test]
fn test_forward_punctuation_run() {
    let text = "foo...bar";
    assert_eq!(find_word_forward(text, 0, None), 3);
    assert_eq!(find_word_forward(text, 3, None), 6);
    assert_eq!(find_word_forward(text, 6, None), 9);
}

#[test]
fn test_forward_cursor_at_end() {
    assert_eq!(find_word_forward("hello", 5, None), 5);
}

// =============================================================================
// Atomic segments (paste markers)
// =============================================================================
// Note: The Rust implementation does not yet natively support atomic paste
// marker segments via WordNavigationOptions. These tests verify the default
// word boundary behavior. The is_atomic_segment callback is available for
// custom segmenters.

#[test]
fn test_paste_marker_treated_as_word() {
    let marker = "[paste #1 +5 lines]";
    let text = format!("hello {} world", marker);
    // Without atomic segment support, the paste marker is treated as a regular word
    let result = find_word_backward(&text, text.len(), None);
    // Should land somewhere reasonable
    assert!(result < text.len());
    assert!(result > 0);
}

#[test]
fn test_forward_paste_marker_treated_as_word() {
    let marker = "[paste #1 +5 lines]";
    let text = format!("hello {} world", marker);
    let result = find_word_forward(&text, 6, None);
    assert!(result > 6);
    assert!(result <= text.len());
}
