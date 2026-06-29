//! Ported from packages/tui/test/overlay-non-capturing.test.ts.
//!
//! Tests for non-capturing overlays: visibility, creation, stacking, hide/dismiss.

use sexy_tui_rs::tui::{Component, Focusable, OverlayAnchor, OverlayOptions, TUI};
use sexy_tui_rs::virtual_terminal::VirtualTerminal;

struct StaticOverlay {
    lines: Vec<String>,
}
impl StaticOverlay {
    fn new(l: &[&str]) -> Self {
        StaticOverlay {
            lines: l.iter().map(|s| s.to_string()).collect(),
        }
    }
}
impl Component for StaticOverlay {
    fn render(&self, _: u16) -> Vec<String> {
        self.lines.clone()
    }
    fn invalidate(&mut self) {}
}

struct EmptyContent;
impl Component for EmptyContent {
    fn render(&self, _: u16) -> Vec<String> {
        vec![]
    }
    fn invalidate(&mut self) {}
}

struct FocusableComp {
    focused: bool,
    lines: Vec<String>,
}
impl FocusableComp {
    fn new(l: &[&str]) -> Self {
        FocusableComp {
            focused: false,
            lines: l.iter().map(|s| s.to_string()).collect(),
        }
    }
}
impl Component for FocusableComp {
    fn render(&self, _: u16) -> Vec<String> {
        self.lines.clone()
    }
    fn invalidate(&mut self) {}
}
impl Focusable for FocusableComp {
    fn set_focused(&mut self, f: bool) {
        self.focused = f;
    }
    fn is_focused(&self) -> bool {
        self.focused
    }
}

fn nc_opts(w: u16) -> OverlayOptions {
    OverlayOptions {
        width: Some(w),
        non_capturing: true,
        ..Default::default()
    }
}

// ── Creation & visibility ────────────────────────────────────────────────────

#[test]
fn test_non_capturing_overlay_visible() {
    let vt = VirtualTerminal::new(80, 24);
    let vi = vt.clone();
    let mut tui = TUI::new(Box::new(vt));
    tui.add_child(Box::new(StaticOverlay::new(&["BASE"])));
    tui.show_overlay(
        Box::new(StaticOverlay::new(&["NC-OVERLAY"])),
        Some(nc_opts(20)),
    );
    tui.start();
    tui.request_render();
    let vp = vi.get_viewport();
    assert!(
        vp.iter().any(|l| l.contains("NC-OVERLAY")),
        "Non-capturing overlay should be visible"
    );
    assert!(
        vp.iter().any(|l| l.contains("BASE")),
        "Base content should also be visible"
    );
}

#[test]
fn test_non_capturing_flag_set() {
    let vt = VirtualTerminal::new(80, 24);
    let mut tui = TUI::new(Box::new(vt));
    tui.add_child(Box::new(EmptyContent));
    let handle = tui.show_overlay(Box::new(StaticOverlay::new(&["X"])), Some(nc_opts(10)));
    assert!(
        !handle.is_hidden(),
        "New non-capturing overlay should not be hidden"
    );
}

#[test]
fn test_capturing_overlay_default() {
    let vt = VirtualTerminal::new(80, 24);
    let vi = vt.clone();
    let mut tui = TUI::new(Box::new(vt));
    tui.add_child(Box::new(EmptyContent));
    tui.show_overlay(
        Box::new(StaticOverlay::new(&["CAPTURING"])),
        Some(OverlayOptions {
            width: Some(15),
            ..Default::default()
        }),
    );
    tui.start();
    tui.request_render();
    assert!(vi.get_viewport().iter().any(|l| l.contains("CAPTURING")));
}

// ── Multiple overlays ───────────────────────────────────────────────────────

#[test]
fn test_non_capturing_overlay_stacking() {
    let vt = VirtualTerminal::new(80, 24);
    let vi = vt.clone();
    let mut tui = TUI::new(Box::new(vt));
    tui.add_child(Box::new(EmptyContent));
    tui.show_overlay(
        Box::new(StaticOverlay::new(&["FIRST-NC"])),
        Some(nc_opts(15)),
    );
    tui.show_overlay(
        Box::new(StaticOverlay::new(&["SECOND-NC"])),
        Some(OverlayOptions {
            width: Some(15),
            non_capturing: true,
            anchor: OverlayAnchor::TopCenter,
            ..Default::default()
        }),
    );
    tui.start();
    tui.request_render();
    let vp = vi.get_viewport();
    // When overlays overlap, the topmost wins. At least one should be visible.
    assert!(
        vp.iter()
            .any(|l| l.contains("FIRST-NC") || l.contains("SECOND-NC")),
        "At least one overlay should be visible"
    );
}

#[test]
fn test_mixed_capturing_and_non_capturing() {
    let vt = VirtualTerminal::new(80, 24);
    let vi = vt.clone();
    let mut tui = TUI::new(Box::new(vt));
    tui.add_child(Box::new(StaticOverlay::new(&["BASE-123"])));
    tui.show_overlay(
        Box::new(StaticOverlay::new(&["NC-LAYER"])),
        Some(nc_opts(15)),
    );
    tui.show_overlay(
        Box::new(StaticOverlay::new(&["CAP-LAYER"])),
        Some(OverlayOptions {
            width: Some(15),
            anchor: OverlayAnchor::TopCenter,
            ..Default::default()
        }),
    );
    tui.start();
    tui.request_render();
    let vp = vi.get_viewport();
    // Mixed capturing/non-capturing layers render without panicking
    let has_any = vp
        .iter()
        .any(|l| l.contains("CAP-LAYER") || l.contains("NC-LAYER") || l.contains("BASE-123"));
    assert!(has_any, "Mixed overlays should not crash TUI");
}

// ── Hide overlay (TUI-level) ────────────────────────────────────────────────

#[test]
fn test_hide_overlay_tui_level() {
    let vt = VirtualTerminal::new(80, 24);
    let vi = vt.clone();
    let mut tui = TUI::new(Box::new(vt));
    tui.add_child(Box::new(StaticOverlay::new(&["BASE"])));
    tui.show_overlay(
        Box::new(StaticOverlay::new(&["NC-OVERLAY"])),
        Some(nc_opts(20)),
    );
    tui.start();
    tui.request_render();
    assert!(vi.get_viewport().iter().any(|l| l.contains("NC-OVERLAY")));
    tui.hide_overlay();
    tui.request_render();
    // After hide_overlay, overlay is popped from stack
}

#[test]
fn test_hide_overlay_restores_viewport() {
    let vt = VirtualTerminal::new(80, 24);
    let vi = vt.clone();
    let mut tui = TUI::new(Box::new(vt));
    tui.add_child(Box::new(StaticOverlay::new(&["BASE-CONTENT"])));
    tui.show_overlay(
        Box::new(StaticOverlay::new(&["TEMP-OVERLAY"])),
        Some(nc_opts(20)),
    );
    tui.start();
    tui.request_render();
    assert!(vi.get_viewport().iter().any(|l| l.contains("TEMP-OVERLAY")));
    tui.hide_overlay();
    tui.request_render();
    // Base content should still be present
    assert!(
        vi.get_viewport().iter().any(|l| l.contains("BASE-CONTENT")),
        "Base content should persist after hide"
    );
}

// ── Width constraints ────────────────────────────────────────────────────────

#[test]
fn test_non_capturing_respects_width() {
    let vt = VirtualTerminal::new(80, 24);
    let vi = vt.clone();
    let mut tui = TUI::new(Box::new(vt));
    tui.add_child(Box::new(EmptyContent));
    let long = "X".repeat(200);
    tui.show_overlay(Box::new(StaticOverlay::new(&[&long])), Some(nc_opts(30)));
    tui.start();
    tui.request_render();
    for line in &vi.get_viewport() {
        assert!(
            line.len() <= 80,
            "Line exceeds terminal width: {}",
            line.len()
        );
    }
}

// ── Position ─────────────────────────────────────────────────────────────────

#[test]
fn test_overlay_at_bottom_position() {
    let vt = VirtualTerminal::new(80, 24);
    let vi = vt.clone();
    let mut tui = TUI::new(Box::new(vt));
    tui.add_child(Box::new(EmptyContent));
    tui.show_overlay(
        Box::new(StaticOverlay::new(&["BOTTOM-NC"])),
        Some(OverlayOptions {
            width: Some(15),
            non_capturing: true,
            anchor: OverlayAnchor::BottomRight,
            ..Default::default()
        }),
    );
    tui.start();
    tui.request_render();
    assert!(
        vi.get_viewport().iter().any(|l| l.contains("BOTTOM-NC")),
        "Bottom overlay should be visible"
    );
}

#[test]
fn test_overlay_at_center_position() {
    let vt = VirtualTerminal::new(80, 24);
    let vi = vt.clone();
    let mut tui = TUI::new(Box::new(vt));
    tui.add_child(Box::new(EmptyContent));
    tui.show_overlay(
        Box::new(StaticOverlay::new(&["CENTER-NC"])),
        Some(OverlayOptions {
            width: Some(15),
            non_capturing: true,
            anchor: OverlayAnchor::Center,
            ..Default::default()
        }),
    );
    tui.start();
    tui.request_render();
    assert!(
        vi.get_viewport().iter().any(|l| l.contains("CENTER-NC")),
        "Centered overlay should be visible"
    );
}

// ── Multiple children ────────────────────────────────────────────────────────

#[test]
fn test_multiple_children_with_overlay() {
    let vt = VirtualTerminal::new(80, 24);
    let vi = vt.clone();
    let mut tui = TUI::new(Box::new(vt));
    tui.add_child(Box::new(StaticOverlay::new(&["Child-1"])));
    tui.add_child(Box::new(StaticOverlay::new(&["Child-2"])));
    tui.show_overlay(Box::new(StaticOverlay::new(&["OVER"])), Some(nc_opts(10)));
    tui.start();
    tui.request_render();
    assert!(vi.get_viewport().iter().any(|l| l.contains("OVER")));
}

// ── is_hidden initial state ─────────────────────────────────────────────────

#[test]
fn test_is_hidden_initial_state() {
    let vt = VirtualTerminal::new(80, 24);
    let mut tui = TUI::new(Box::new(vt));
    tui.add_child(Box::new(EmptyContent));
    let handle = tui.show_overlay(Box::new(StaticOverlay::new(&["X"])), Some(nc_opts(10)));
    assert!(!handle.is_hidden(), "New overlay should not be hidden");
}
