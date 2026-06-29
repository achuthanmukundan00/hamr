//! Loader wrapped with borders for extension UI.
//!
//! Port of `packages/coding-agent/src/modes/interactive/components/bordered-loader.ts`.

use crate::modes::interactive::components::dynamic_border::DynamicBorder;
use crate::modes::interactive::components::keybinding_hints::key_hint;
use crate::modes::interactive::components::tui_shim::{
    CancellableLoader, Component, Container, Loader, Spacer, Text,
};
use crate::modes::interactive::theme::theme::theme;
use std::sync::Arc;

/// A loader component wrapped with top and bottom borders, with optional cancel support.
pub struct BorderedLoader {
    container: Container,
    cancellable: bool,
    aborted: bool,
    on_abort_fn: Option<Box<dyn Fn() + Send + Sync>>,
}

impl BorderedLoader {
    /// Create a new bordered loader.
    ///
    /// * `cancellable` - if true, shows a cancel hint and supports cancellation.
    pub fn new(message: &str, cancellable: bool) -> Self {
        let mut container = Container::new();

        let border_color: Arc<dyn Fn(&str) -> String + Send + Sync> =
            Arc::new(|s: &str| theme().fg("border", s));
        container.add_child(Box::new(DynamicBorder::new(Some(border_color.clone()))));

        if cancellable {
            let loader = CancellableLoader::new(message);
            container.add_child(Box::new(loader));
        } else {
            let loader = Loader::new(message);
            container.add_child(Box::new(loader));
        };

        if cancellable {
            container.add_child(Box::new(Spacer::new(1)));
            container.add_child(Box::new(Text::new(
                key_hint("tui.select.cancel", "cancel"),
                1,
                0,
            )));
        }

        container.add_child(Box::new(Spacer::new(1)));
        container.add_child(Box::new(DynamicBorder::new(Some(border_color.clone()))));

        Self {
            container,
            cancellable,
            aborted: false,
            on_abort_fn: None,
        }
    }

    /// Set a callback invoked when the loader is aborted.
    pub fn set_on_abort(&mut self, f: Option<Box<dyn Fn() + Send + Sync>>) {
        if self.cancellable {
            self.on_abort_fn = f;
        }
    }

    /// Handle input key data. If cancellable and the cancel key is pressed,
    /// triggers the abort callback.
    pub fn handle_input(&mut self, _data: &str) {
        if self.cancellable {
            // In the real TUI, this would check keybindings.matches(data, "tui.select.cancel")
            // For now, stub — actual matching happens in the full TUI integration.
        }
    }

    /// Whether the loader was aborted.
    pub fn is_aborted(&self) -> bool {
        self.aborted
    }

    /// Trigger abort manually.
    pub fn abort(&mut self) {
        self.aborted = true;
        if let Some(ref f) = self.on_abort_fn {
            f();
        }
    }

    /// Stop the loader.
    pub fn stop(&mut self) {
        // In real impl we'd call self.loader.stop() or self.loader.dispose()
    }

    /// Clean up the loader.
    pub fn dispose(&mut self) {
        self.stop();
    }
}

impl Component for BorderedLoader {
    fn render(&self, width: u16) -> Vec<String> {
        self.container.render(width)
    }

    fn invalidate(&mut self) {
        self.container.invalidate();
    }
}
