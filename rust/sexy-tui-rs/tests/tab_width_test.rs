//! Ported from packages/tui/test/tab-width.test.ts
//!
//! Tests for tab width calculations in sliceByColumn/sliceWithWidth and extractSegments.

use sexy_tui_rs::utils::{extract_segments, slice_with_width, visible_width};

// =============================================================================
// Tab width accounting
// =============================================================================

mod tab_width_accounting {
    use super::*;

    #[test]
    fn test_keeps_slice_helper_widths_consistent_with_visible_width() {
        let text = "out 192M\t.pi/skill-tests/results-ha";
        let (slice_text, slice_width) = slice_with_width(text, 0, 10, true);

        assert_eq!(slice_text, "out 192M");
        assert_eq!(slice_width, 8);
        assert_eq!(visible_width(&slice_text), slice_width);
    }

    #[test]
    fn test_keeps_overlay_segment_widths_consistent_with_visible_width_tab_excluded() {
        let text = "out 192M\t.pi/skill-tests/results-ha";
        let segments = extract_segments(text, 10, 13, 10, true);

        assert_eq!(segments.before, "out 192M");
        assert_eq!(segments.before_width, 8);
        assert_eq!(visible_width(&segments.before), segments.before_width);
    }

    #[test]
    fn test_keeps_overlay_segment_widths_consistent_with_visible_width_tab_included() {
        let text = "out 192M\t.pi/skill-tests/results-ha";
        let segments = extract_segments(text, 11, 13, 10, true);

        assert_eq!(segments.before_width, 11);
        assert_eq!(visible_width(&segments.before), segments.before_width);
    }
}
