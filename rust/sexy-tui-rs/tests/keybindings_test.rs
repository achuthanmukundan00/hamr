//! Ported from packages/tui/test/keybindings.test.ts
//!
//! Tests for KeybindingsManager: register, resolve conflicts, user overrides.

use std::collections::HashMap;

use sexy_tui_rs::keybindings::{
    KeybindingConflict, KeybindingsConfig, KeybindingsManager, TUI_KEYBINDINGS,
};

/// Helper to create a simple KeybindingsManager with custom user bindings.
fn make_manager(user: KeybindingsConfig) -> KeybindingsManager {
    KeybindingsManager::with_user_bindings(TUI_KEYBINDINGS.clone(), user)
}

// =============================================================================
// KeybindingsManager
// =============================================================================

#[test]
fn test_does_not_evict_selector_confirm_when_input_submit_is_rebound() {
    let mut user = HashMap::new();
    user.insert("tui.input.submit".into(), vec!["enter", "ctrl+enter"]);
    let km = make_manager(user);

    assert_eq!(km.get_keys("tui.input.submit"), vec!["enter", "ctrl+enter"]);
    assert_eq!(km.get_keys("tui.select.confirm"), vec!["enter"]);
}

#[test]
fn test_does_not_evict_cursor_bindings_when_another_action_reuses_same_key() {
    let mut user = HashMap::new();
    user.insert("tui.select.up".into(), vec!["up", "ctrl+p"]);
    let km = make_manager(user);

    assert_eq!(km.get_keys("tui.select.up"), vec!["up", "ctrl+p"]);
    assert_eq!(km.get_keys("tui.editor.cursorUp"), vec!["up"]);
}

#[test]
fn test_still_reports_direct_user_binding_conflicts_without_evicting_defaults() {
    let mut user = HashMap::new();
    user.insert("tui.input.submit".into(), vec!["ctrl+x"]);
    user.insert("tui.select.confirm".into(), vec!["ctrl+x"]);
    let km = make_manager(user);

    assert_eq!(
        km.get_conflicts(),
        vec![KeybindingConflict {
            key: "ctrl+x",
            keybindings: vec!["tui.input.submit".into(), "tui.select.confirm".into()],
        }]
    );
    assert_eq!(km.get_keys("tui.editor.cursorLeft"), vec!["left", "ctrl+b"]);
}

#[test]
fn test_get_keys_for_unknown_action_returns_empty() {
    let km = KeybindingsManager::new(TUI_KEYBINDINGS.clone());
    assert!(km.get_keys("tui.editor.nonexistent").is_empty());
}

#[test]
fn test_get_definition_returns_none_for_unknown_action() {
    let km = KeybindingsManager::new(TUI_KEYBINDINGS.clone());
    assert!(km.get_definition("tui.editor.nonexistent").is_none());
}
