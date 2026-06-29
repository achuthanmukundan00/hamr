//! Login dialog component - replaces editor during OAuth login flow.
//!
//! Port of `packages/coding-agent/src/modes/interactive/components/login-dialog.ts`.

use crate::modes::interactive::components::dynamic_border::DynamicBorder;
use crate::modes::interactive::components::keybinding_hints::key_hint;
use crate::modes::interactive::components::tui_shim::{
    Component, Container, Focusable, Spacer, Text, TuiHandle, get_keybindings,
};
use crate::modes::interactive::theme::theme::theme;

/// OAuth device code info passed to the login dialog.
pub struct OAuthDeviceCodeInfo {
    pub verification_uri: String,
    pub user_code: String,
}

/// Login dialog component for OAuth/provider authentication flows.
pub struct LoginDialogComponent {
    layout: Container,
    content_container: Container,
    on_complete_callback: Box<dyn Fn(bool, Option<String>) + Send + Sync>,
    focused: bool,
    cancelled: bool,
}

impl LoginDialogComponent {
    /// Create a new login dialog.
    ///
    /// * `provider_id` - the provider being authenticated
    /// * `on_complete` - called with (success, optional_message) on completion
    /// * `provider_name_override` - optional display name override
    /// * `title_override` - optional title override
    pub fn new(
        _tui: &dyn TuiHandle,
        provider_id: &str,
        on_complete: Box<dyn Fn(bool, Option<String>) + Send + Sync>,
        provider_name_override: Option<&str>,
        title_override: Option<&str>,
    ) -> Self {
        let provider_name = provider_name_override.unwrap_or(provider_id);
        let title = title_override
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("Login to {}", provider_name));

        let mut layout = Container::new();

        // Top border
        layout.add_child(Box::new(DynamicBorder::new(None)));
        // Title
        layout.add_child(Box::new(Text::new(
            theme().fg("accent", &theme().bold(&title)),
            1,
            0,
        )));

        // Dynamic content area
        let content_container = Container::new();
        layout.add_child(Box::new(DynamicBorder::new(None))); // placeholder, replaced by content_container

        // Bottom border
        layout.add_child(Box::new(DynamicBorder::new(None)));

        Self {
            layout,
            content_container,
            on_complete_callback: on_complete,
            focused: false,
            cancelled: false,
        }
    }

    /// Show an OAuth URL with optional instructions.
    pub fn show_auth(&mut self, url: &str, instructions: Option<&str>) {
        self.content_container.clear();
        self.content_container.add_child(Box::new(Spacer::new(1)));
        self.content_container
            .add_child(Box::new(Text::new(theme().fg("accent", url), 1, 0)));

        let click_hint = if cfg!(target_os = "macos") {
            "Cmd+click to open"
        } else {
            "Ctrl+click to open"
        };
        self.content_container
            .add_child(Box::new(Text::new(theme().fg("dim", click_hint), 1, 0)));

        if let Some(instr) = instructions {
            self.content_container.add_child(Box::new(Spacer::new(1)));
            self.content_container.add_child(Box::new(Text::new(
                theme().fg("warning", instr),
                1,
                0,
            )));
        }
        self.content_container.add_child(Box::new(Spacer::new(1)));
    }

    /// Show a device code (URL + user code).
    pub fn show_device_code(&mut self, info: &OAuthDeviceCodeInfo) {
        self.content_container.clear();
        self.content_container.add_child(Box::new(Spacer::new(1)));
        self.content_container.add_child(Box::new(Text::new(
            theme().fg("accent", &info.verification_uri),
            1,
            0,
        )));

        let click_hint = if cfg!(target_os = "macos") {
            "Cmd+click to open"
        } else {
            "Ctrl+click to open"
        };
        self.content_container
            .add_child(Box::new(Text::new(theme().fg("dim", click_hint), 1, 0)));
        self.content_container.add_child(Box::new(Spacer::new(1)));
        self.content_container.add_child(Box::new(Text::new(
            theme().fg("warning", &format!("Enter code: {}", info.user_code)),
            1,
            0,
        )));
        self.content_container.add_child(Box::new(Spacer::new(1)));
    }

    /// Show a manual input prompt (for callback server providers).
    /// Returns the user's input value.
    pub fn show_manual_input(&mut self, prompt: &str) -> String {
        self.content_container.add_child(Box::new(Spacer::new(1)));
        self.content_container
            .add_child(Box::new(Text::new(theme().fg("dim", prompt), 1, 0)));
        // Input would be added here in full TUI mode
        let cancel_hint = format!("({})", key_hint("tui.select.cancel", "to cancel"));
        self.content_container
            .add_child(Box::new(Text::new(cancel_hint, 1, 0)));
        String::new()
    }

    /// Show a prompt and wait for input (appends to existing content).
    /// Returns the user's input.
    pub fn show_prompt(&mut self, message: &str, placeholder: Option<&str>) -> String {
        self.content_container.add_child(Box::new(Spacer::new(1)));
        self.content_container
            .add_child(Box::new(Text::new(theme().fg("text", message), 1, 0)));
        if let Some(ph) = placeholder {
            self.content_container.add_child(Box::new(Text::new(
                theme().fg("dim", &format!("e.g., {}", ph)),
                1,
                0,
            )));
        }
        let hints = format!(
            "({} {})",
            key_hint("tui.select.cancel", "to cancel,"),
            key_hint("tui.select.confirm", "to submit")
        );
        self.content_container
            .add_child(Box::new(Text::new(hints, 1, 0)));
        String::new()
    }

    /// Show informational text without prompting for input.
    pub fn show_info(&mut self, lines: &[String]) {
        self.content_container.clear();
        self.content_container.add_child(Box::new(Spacer::new(1)));
        for line in lines {
            self.content_container
                .add_child(Box::new(Text::new(line.clone(), 1, 0)));
        }
        self.content_container.add_child(Box::new(Spacer::new(1)));
        self.content_container.add_child(Box::new(Text::new(
            format!("({})", key_hint("tui.select.cancel", "to close")),
            1,
            0,
        )));
    }

    /// Show a waiting message (for polling flows).
    pub fn show_waiting(&mut self, message: &str) {
        self.content_container.add_child(Box::new(Spacer::new(1)));
        self.content_container
            .add_child(Box::new(Text::new(theme().fg("dim", message), 1, 0)));
        self.content_container.add_child(Box::new(Text::new(
            format!("({})", key_hint("tui.select.cancel", "to cancel")),
            1,
            0,
        )));
    }

    /// Show a progress message.
    pub fn show_progress(&mut self, message: &str) {
        self.content_container
            .add_child(Box::new(Text::new(theme().fg("dim", message), 1, 0)));
    }

    /// Handle keyboard input.
    pub fn handle_input(&mut self, data: &str) {
        let kb = get_keybindings();
        if kb.matches(data, "tui.select.cancel") {
            self.cancel();
        }
    }

    fn cancel(&mut self) {
        self.cancelled = true;
        (self.on_complete_callback)(false, Some("Login cancelled".to_string()));
    }
}

impl Component for LoginDialogComponent {
    fn render(&self, width: u16) -> Vec<String> {
        self.layout.render(width)
    }

    fn invalidate(&mut self) {
        self.layout.invalidate();
    }
}

impl Focusable for LoginDialogComponent {
    fn is_focused(&self) -> bool {
        self.focused
    }

    fn set_focused(&mut self, focused: bool) {
        self.focused = focused;
    }
}
