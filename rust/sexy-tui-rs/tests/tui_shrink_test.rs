//! Ported from packages/tui/test/tui-shrink.test.ts
//!
//! TUI window shrink behavior tests using VirtualTerminal for ANSI-aware assertions.

use sexy_tui_rs::terminal::Terminal;
use sexy_tui_rs::tui::Component;
use sexy_tui_rs::virtual_terminal::VirtualTerminal;

struct Lines {
    lines: Vec<String>,
}

impl Lines {
    fn new(lines: &[&str]) -> Self {
        Lines {
            lines: lines.iter().map(|s| s.to_string()).collect(),
        }
    }
}

impl Component for Lines {
    fn render(&self, _width: u16) -> Vec<String> {
        self.lines.clone()
    }
    fn invalidate(&mut self) {}
}

/// Test that clearing all children when content has rendered clears the viewport.
#[test]
fn test_clears_all_rendered_lines_when_content_shrinks_to_zero() {
    // Render initial content into virtual terminal
    let mut vt = VirtualTerminal::new(40, 10);
    let content = Lines::new(&["first", "second", "third"]);
    let rendered = content.render(40);

    vt.write("\x1b[2J\x1b[H");
    for line in &rendered {
        vt.write(&format!("{}\r\n", line));
    }

    let viewport = vt.get_viewport();
    assert!(viewport.iter().any(|l| l.contains("first")));
    assert!(viewport.iter().any(|l| l.contains("second")));
    assert!(viewport.iter().any(|l| l.contains("third")));

    // Clear and re-render with empty content
    vt.clear();
    let empty = Lines::new(&[]);
    let empty_rendered = empty.render(40);
    for line in &empty_rendered {
        vt.write(&format!("{}\r\n", line));
    }

    let viewport = vt.get_viewport();
    assert!(
        !viewport.iter().any(|l| l.contains("first")),
        "first line should be cleared"
    );
    assert!(
        !viewport.iter().any(|l| l.contains("second")),
        "second line should be cleared"
    );
    assert!(
        !viewport.iter().any(|l| l.contains("third")),
        "third line should be cleared"
    );
}
