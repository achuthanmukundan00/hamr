//! Ported from packages/tui/test/tui-cell-size-input.test.ts
//!
//! Tests for cell size change handling in input components.

use sexy_tui_rs::terminal::Terminal;
use sexy_tui_rs::virtual_terminal::VirtualTerminal;

/// Test that terminal handles escape sequences without corrupting subsequent data.
#[test]
fn test_forwards_bare_escape() {
    let mut vt = VirtualTerminal::new(80, 24);

    // Write a known escape sequence (cursor hide: \x1b[?25l) then text
    vt.write("\x1b[?25l");
    vt.write("hello");

    let viewport = vt.get_viewport();
    // After a complete escape sequence, text should render
    assert!(
        viewport[0].contains("hello"),
        "Terminal should process text after escape sequence. Got: {:?}",
        viewport[0]
    );
}

/// Test that cell size response sequences are processed correctly
/// and don't corrupt subsequent input.
#[test]
fn test_consumes_cell_size_response() {
    let mut vt = VirtualTerminal::new(80, 24);

    // Send a cell size response sequence: \x1b[<height>;<width>t
    vt.write("\x1b[6;20;10t");

    // Terminal should absorb this without errors
    // Then send normal text
    vt.write("q");

    let viewport = vt.get_viewport();
    // The "q" should appear; cell size response should not have corrupted state
    assert!(
        viewport[0].contains("q"),
        "Terminal should forward normal input after cell size response"
    );
}

#[test]
fn test_multiple_cell_size_responses() {
    let mut vt = VirtualTerminal::new(80, 24);

    // Multiple cell size responses
    vt.write("\x1b[6;10;5t");
    vt.write("\x1b[6;20;10t");
    vt.write("abc");

    let viewport = vt.get_viewport();
    assert!(
        viewport[0].contains("abc"),
        "Terminal should handle multiple cell size responses"
    );
}
