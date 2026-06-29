//! Ported from packages/tui/test/tui-overlay-style-leak.test.ts
//!
//! Tests for style isolation between overlay and background using VirtualTerminal.

use sexy_tui_rs::terminal::Terminal;
use sexy_tui_rs::virtual_terminal::{CellAttribute, VirtualTerminal};

/// Test that trailing SGR resets beyond the last visible column don't leak italics
/// to subsequent lines (no overlay case).
#[test]
fn test_no_style_leak_without_overlay() {
    let width: u16 = 20;
    // Italic SGR: \x1b[3m, reset: \x1b[23m
    let base_line = format!("\x1b[3m{}\x1b[23m", "X".repeat(width as usize));

    let mut vt = VirtualTerminal::new(width, 6);
    vt.write(&format!("{}\r\n", base_line));
    vt.write("INPUT\r\n");

    // Line 1 (row index 1) should NOT be italic
    assert!(
        !vt.cell_has_attribute(1, 0, CellAttribute::Italic),
        "Italic style should not leak to next line after SGR reset at column boundary"
    );
}

/// Test that overlay slicing that drops trailing SGR resets doesn't leak styles.
#[test]
fn test_no_style_leak_with_overlay() {
    let width: u16 = 20;
    let base_line = format!("\x1b[3m{}\x1b[23m", "X".repeat(width as usize));

    let mut vt = VirtualTerminal::new(width, 6);
    // Render base content
    vt.write(&format!("{}\r\n", base_line));
    vt.write("INPUT\r\n");

    // Simulate overlay rendering: move to row 0, col 5, write overlay with width 3
    vt.write("\x1b[1;6H"); // row 1 (0-indexed -> 1), col 6 (0-indexed -> 6) = 5+1
    vt.write("OVR");

    // Line 1 (row index 1) should NOT be italic
    assert!(
        !vt.cell_has_attribute(1, 0, CellAttribute::Italic),
        "Italic style should not leak to next line when overlay slicing drops trailing SGR resets"
    );
}
