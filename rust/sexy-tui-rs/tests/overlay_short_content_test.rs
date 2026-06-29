//! Ported from packages/tui/test/overlay-short-content.test.ts
//!
//! Tests for short content in overlays using VirtualTerminal with TUI integration.

use sexy_tui_rs::tui::{Component, TUI};
use sexy_tui_rs::virtual_terminal::VirtualTerminal;

struct SimpleContent {
    lines: Vec<String>,
}

impl SimpleContent {
    fn new(lines: &[&str]) -> Self {
        SimpleContent {
            lines: lines.iter().map(|s| s.to_string()).collect(),
        }
    }
}

impl Component for SimpleContent {
    fn render(&self, _width: u16) -> Vec<String> {
        self.lines.clone()
    }
    fn invalidate(&mut self) {}
}

struct SimpleOverlay {
    lines: Vec<String>,
}

impl SimpleOverlay {
    fn new(lines: &[&str]) -> Self {
        SimpleOverlay {
            lines: lines.iter().map(|s| s.to_string()).collect(),
        }
    }
}

impl Component for SimpleOverlay {
    fn render(&self, _width: u16) -> Vec<String> {
        self.lines.clone()
    }
    fn invalidate(&mut self) {}
}

#[test]
fn test_overlay_renders_when_content_is_shorter_than_terminal() {
    // Terminal has 24 rows, content only has 3 lines
    let vt = VirtualTerminal::new(80, 24);
    let vt_inspect = vt.clone();
    let mut tui = TUI::new(Box::new(vt));

    tui.add_child(Box::new(SimpleContent::new(&[
        "Line 1", "Line 2", "Line 3",
    ])));

    // Show overlay
    let overlay = SimpleOverlay::new(&["OVERLAY_TOP", "OVERLAY_MID", "OVERLAY_BOT"]);
    tui.show_overlay(Box::new(overlay), None);

    tui.start();
    tui.request_render();

    let viewport = vt_inspect.get_viewport();
    let has_overlay = viewport.iter().any(|line| line.contains("OVERLAY"));
    assert!(
        has_overlay,
        "Overlay should be visible. Viewport: {:?}",
        &viewport[0..5]
    );
}

#[test]
fn test_overlay_not_clipped_when_short() {
    let vt = VirtualTerminal::new(80, 24);
    let vt_inspect = vt.clone();
    let mut tui = TUI::new(Box::new(vt));

    tui.add_child(Box::new(SimpleContent::new(&["Line 1", "Line 2"])));

    let overlay = SimpleOverlay::new(&["TOP", "MID", "BOT"]);
    tui.show_overlay(Box::new(overlay), None);

    tui.start();
    tui.request_render();

    let viewport = vt_inspect.get_viewport();
    assert!(viewport.iter().any(|l| l.contains("TOP")));
    assert!(viewport.iter().any(|l| l.contains("MID")));
    assert!(viewport.iter().any(|l| l.contains("BOT")));
}
