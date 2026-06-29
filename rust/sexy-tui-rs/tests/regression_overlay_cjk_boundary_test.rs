//! Ported from packages/tui/test/regression-overlay-cjk-boundary.test.ts
//!
//! Regression tests for CJK character boundary behavior in overlay compositing.

use sexy_tui_rs::utils::{extract_segments, slice_by_column, visible_width};

// =============================================================================
// Overlay CJK boundary regression
// =============================================================================

mod overlay_cjk_boundary_regression {
    use super::*;

    #[test]
    fn test_excludes_wide_grapheme_from_before_when_overlay_starts_inside_it() {
        let segments = extract_segments("abcd让EFGH", 5, 9, 11, true);

        assert_eq!(segments.before, "abcd");
        assert_eq!(segments.before_width, 4);
        assert_eq!(visible_width(&segments.before), segments.before_width);
        assert_eq!(segments.after, "H");
        assert_eq!(segments.after_width, 1);
        assert_eq!(visible_width(&segments.after), segments.after_width);
    }

    #[test]
    fn test_keeps_ascii_before_segment_behavior_at_same_boundary() {
        let segments = extract_segments("abcdG EFGH", 5, 9, 11, true);

        assert_eq!(segments.before, "abcdG");
        assert_eq!(segments.before_width, 5);
        assert_eq!(visible_width(&segments.before), segments.before_width);
    }
}
