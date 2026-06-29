//! Component that renders an auth provider selector for login/logout.
//!
//! Port of `packages/coding-agent/src/modes/interactive/components/oauth-selector.ts`.

use crate::modes::interactive::components::dynamic_border::DynamicBorder;
use crate::modes::interactive::components::tui_shim::{
    Component, Container, Focusable, Input, Spacer, TruncatedText, fuzzy_filter, get_keybindings,
};
use crate::modes::interactive::theme::theme::THEME;

/// An auth provider that can be selected.
#[derive(Clone)]
pub struct AuthSelectorProvider {
    pub id: String,
    pub name: String,
    pub auth_type: AuthType,
}

#[derive(Clone, PartialEq)]
pub enum AuthType {
    OAuth,
    ApiKey,
}

/// Auth status for a provider.
pub struct AuthStatus {
    pub source: AuthSource,
    pub label: Option<String>,
}

#[derive(Clone, PartialEq)]
pub enum AuthSource {
    Environment,
    Runtime,
    Fallback,
    ModelsJsonKey,
    ModelsJsonCommand,
    Unconfigured,
}

/// Simple auth storage trait (stub).
pub trait AuthStorageTrait: Send + Sync {
    fn get(&self, provider_id: &str) -> Option<Box<dyn std::any::Any>>;
    fn get_auth_status(&self, provider_id: &str) -> AuthStatus;
}

/// A component that renders an auth provider selector with search.
pub struct OAuthSelectorComponent {
    layout: Container,
    all_providers: Vec<AuthSelectorProvider>,
    filtered_providers: Vec<AuthSelectorProvider>,
    selected_index: usize,
    mode: SelectorMode,
    on_select_callback: Box<dyn Fn(String) + Send + Sync>,
    on_cancel_callback: Box<dyn Fn() + Send + Sync>,
    list_pos: usize,
    search_input_ref: Input,
    focused: bool,
}

#[derive(PartialEq)]
enum SelectorMode {
    Login,
    Logout,
}

impl OAuthSelectorComponent {
    /// Create a new OAuth selector component.
    ///
    /// * `mode` - "login" or "logout"
    /// * `providers` - list of available auth providers
    /// * `on_select` - called with the chosen provider ID
    /// * `on_cancel` - called on cancel
    fn new(
        mode: SelectorMode,
        _auth_storage: &dyn AuthStorageTrait,
        providers: Vec<AuthSelectorProvider>,
        on_select: Box<dyn Fn(String) + Send + Sync>,
        on_cancel: Box<dyn Fn() + Send + Sync>,
    ) -> Self {
        let all_providers = providers.clone();
        let _filtered_providers = providers;
        let mut layout = Container::new();

        // Top border
        layout.add_child(Box::new(DynamicBorder::new(None)));
        layout.add_child(Box::new(Spacer::new(1)));

        // Title
        let title = if mode == SelectorMode::Login {
            "Select provider to configure:"
        } else {
            "Select provider to logout:"
        };
        layout.add_child(Box::new(TruncatedText::new(
            THEME.fg("accent", &THEME.bold(title)),
            1,
            0,
        )));
        layout.add_child(Box::new(Spacer::new(1)));

        // Search input
        let _search_input = Input::new();
        layout.add_child(Box::new(Spacer::new(1))); // placeholder for input
        layout.add_child(Box::new(Spacer::new(1)));

        // List container
        let list_pos = layout.children().len();
        layout.add_child(Box::new(Container::new()));

        layout.add_child(Box::new(Spacer::new(1)));
        // Bottom border
        layout.add_child(Box::new(DynamicBorder::new(None)));

        let mut result = Self {
            layout,
            all_providers: all_providers.clone(),
            filtered_providers: all_providers,
            selected_index: 0,
            mode,
            on_select_callback: on_select,
            on_cancel_callback: on_cancel,
            list_pos,
            search_input_ref: Input::new(),
            focused: false,
        };

        result.update_list();
        result
    }

    /// Filter providers by search query.
    fn filter_providers(&mut self, query: &str) {
        self.filtered_providers = if query.is_empty() {
            self.all_providers.clone()
        } else {
            fuzzy_filter(&self.all_providers, query, |p: &AuthSelectorProvider| {
                format!(
                    "{} {} {}",
                    p.name,
                    p.id,
                    if p.auth_type == AuthType::OAuth {
                        "oauth"
                    } else {
                        "api_key"
                    }
                )
            })
        };
        self.selected_index = self
            .selected_index
            .min(self.filtered_providers.len().saturating_sub(1));
        self.update_list();
    }

    /// Rebuild the visible list based on current state.
    fn update_list(&mut self) {
        let mut new_list = Container::new();

        let max_visible = 8usize;
        let start_index = self
            .selected_index
            .saturating_sub(max_visible / 2)
            .min(self.filtered_providers.len().saturating_sub(max_visible));
        let end_index = (start_index + max_visible).min(self.filtered_providers.len());

        for i in start_index..end_index {
            let provider = &self.filtered_providers[i];
            let is_selected = i == self.selected_index;

            let line = if is_selected {
                format!(
                    "{} {}",
                    THEME.fg("accent", "→"),
                    THEME.fg("accent", &provider.name),
                )
            } else {
                format!("  {}", THEME.fg("text", &provider.name))
            };

            new_list.add_child(Box::new(TruncatedText::new(line, 1, 0)));
        }

        // Scroll indicator
        if start_index > 0 || end_index < self.filtered_providers.len() {
            let scroll_info = THEME.fg(
                "muted",
                &format!(
                    "  ({}/{})",
                    self.selected_index + 1,
                    self.filtered_providers.len()
                ),
            );
            new_list.add_child(Box::new(TruncatedText::new(scroll_info, 1, 0)));
        }

        // Empty state
        if self.filtered_providers.is_empty() {
            let message = if self.all_providers.is_empty() {
                match self.mode {
                    SelectorMode::Login => "No providers available",
                    SelectorMode::Logout => "No providers logged in. Use /login first.",
                }
            } else {
                "No matching providers"
            };
            new_list.add_child(Box::new(TruncatedText::new(
                THEME.fg("muted", &format!("  {}", message)),
                1,
                0,
            )));
        }

        // Replace the list container
        if self.list_pos < self.layout.children().len() {
            self.layout.children_mut()[self.list_pos] = Box::new(new_list);
        }
    }

    /// Handle keyboard input.
    pub fn handle_input(&mut self, key_data: &str) {
        let kb = get_keybindings();

        if kb.matches(key_data, "tui.select.up") {
            if self.filtered_providers.is_empty() {
                return;
            }
            self.selected_index = self.selected_index.saturating_sub(1);
            self.update_list();
        } else if kb.matches(key_data, "tui.select.down") {
            if self.filtered_providers.is_empty() {
                return;
            }
            if self.selected_index + 1 < self.filtered_providers.len() {
                self.selected_index += 1;
            }
            self.update_list();
        } else if kb.matches(key_data, "tui.select.confirm") {
            if let Some(provider) = self.filtered_providers.get(self.selected_index) {
                (self.on_select_callback)(provider.id.clone());
            }
        } else if kb.matches(key_data, "tui.select.cancel") {
            (self.on_cancel_callback)();
        } else {
            self.search_input_ref.handle_input(key_data);
            self.filter_providers("");
        }
    }
}

impl Component for OAuthSelectorComponent {
    fn render(&self, width: u16) -> Vec<String> {
        self.layout.render(width)
    }

    fn invalidate(&mut self) {
        self.layout.invalidate();
    }
}

impl Focusable for OAuthSelectorComponent {
    fn is_focused(&self) -> bool {
        self.focused
    }

    fn set_focused(&mut self, focused: bool) {
        self.focused = focused;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct StubAuthStorage;
    impl AuthStorageTrait for StubAuthStorage {
        fn get(&self, _provider_id: &str) -> Option<Box<dyn std::any::Any>> {
            None
        }

        fn get_auth_status(&self, _provider_id: &str) -> AuthStatus {
            AuthStatus {
                source: AuthSource::Unconfigured,
                label: None,
            }
        }
    }

    #[test]
    fn test_oauth_selector_creation() {
        let storage = StubAuthStorage;
        let providers = vec![
            AuthSelectorProvider {
                id: "anthropic".to_string(),
                name: "Anthropic".to_string(),
                auth_type: AuthType::ApiKey,
            },
            AuthSelectorProvider {
                id: "openai".to_string(),
                name: "OpenAI".to_string(),
                auth_type: AuthType::ApiKey,
            },
        ];

        let _selector = OAuthSelectorComponent::new(
            SelectorMode::Login,
            &storage,
            providers,
            Box::new(|_id| {}),
            Box::new(|| {}),
        );
    }
}
