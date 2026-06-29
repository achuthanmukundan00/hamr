//! Dynamic border component that adjusts to viewport width.
//!
//! Port of `packages/coding-agent/src/modes/interactive/components/dynamic-border.ts`.
//!
//! Note: When used from extensions loaded via jiti, the global `theme` may be undefined
//! because jiti creates a separate module cache. Always pass an explicit color
//! function when using DynamicBorder in components exported for extension use.

use crate::modes::interactive::components::tui_shim::Component;
use crate::modes::interactive::theme::theme::theme;
use std::sync::Arc;

/// Dynamic border component that adjusts to viewport width.
pub struct DynamicBorder {
    /// Color function — defaults to theme.fg("border", ...)
    color_fn: Arc<dyn Fn(&str) -> String + Send + Sync>,
}

impl DynamicBorder {
    /// Create a new DynamicBorder with an optional color function.
    /// If no color function is provided, uses `theme.fg("border", str)`.
    pub fn new(color_fn: Option<Arc<dyn Fn(&str) -> String + Send + Sync>>) -> Self {
        Self {
            color_fn: color_fn.unwrap_or_else(|| Arc::new(|s: &str| theme().fg("border", s))),
        }
    }
}

impl Component for DynamicBorder {
    fn render(&self, width: u16) -> Vec<String> {
        let line = "─".repeat((width as usize).max(1));
        vec![(self.color_fn)(&line)]
    }

    fn invalidate(&mut self) {
        // No cached state to invalidate
    }
}
