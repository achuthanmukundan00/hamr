//! Ported from packages/tui/test/settings-list.test.ts
//!
//! Tests for the SettingsList widget.

use std::{cell::Cell, rc::Rc};

use sexy_tui_rs::widgets::{SettingItem, SettingsList, SettingsListTheme};
use sexy_tui_rs::Component;

fn make_test_theme() -> SettingsListTheme {
    SettingsListTheme {
        label: Box::new(|text, _selected| text.to_string()),
        value: Box::new(|text, _selected| text.to_string()),
        description: Box::new(|text| text.to_string()),
        section: Box::new(|text| text.to_string()),
        cursor: "> ".into(),
        hint: Box::new(|text| text.to_string()),
    }
}

// =============================================================================
// SettingsList
// =============================================================================

mod settings_list_tests {
    use super::*;

    #[test]
    fn test_calls_on_cancel_when_escape_is_pressed() {
        let cancelled = Rc::new(Cell::new(false));
        let cb_cancelled = cancelled.clone();
        let mut list = SettingsList::new(
            vec![SettingItem {
                id: "a".into(),
                label: "A".into(),
                current_value: "on".into(),
                values: vec![],
                description: None,
                submenu: None,
                section: None,
            }],
            10,
            make_test_theme(),
            Box::new(|_, _| {}),
        );
        list.on_cancel = Some(Box::new(move || {
            cb_cancelled.set(true);
        }));

        list.handle_input("\x1b");
        assert!(cancelled.get());
    }

    #[test]
    fn test_calls_on_cancel_when_ctrl_c_is_pressed() {
        let cancelled = Rc::new(Cell::new(false));
        let cb_cancelled = cancelled.clone();
        let mut list = SettingsList::new(
            vec![SettingItem {
                id: "a".into(),
                label: "A".into(),
                current_value: "on".into(),
                values: vec![],
                description: None,
                submenu: None,
                section: None,
            }],
            10,
            make_test_theme(),
            Box::new(|_, _| {}),
        );
        list.on_cancel = Some(Box::new(move || {
            cb_cancelled.set(true);
        }));

        list.handle_input("\x03");
        assert!(cancelled.get());
    }

    #[test]
    fn test_does_not_crash_with_empty_items() {
        let mut list = SettingsList::new(vec![], 10, make_test_theme(), Box::new(|_, _| {}));

        list.handle_input("\x1b");
        list.handle_input("enter");
        list.handle_input("\x03");
        // No crash = success
    }

    #[test]
    fn test_renders_setting_item() {
        let list = SettingsList::new(
            vec![SettingItem {
                id: "test-id".into(),
                label: "Test Setting".into(),
                current_value: "value".into(),
                values: vec!["value".into(), "other".into()],
                description: Some("A test setting".into()),
                submenu: None,
                section: None,
            }],
            10,
            make_test_theme(),
            Box::new(|_, _| {}),
        );
        let rendered = list.render(80);
        assert!(!rendered.is_empty());
        assert!(rendered[0].contains("Test Setting"));
    }
}
