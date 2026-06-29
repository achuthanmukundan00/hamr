//! Ported from packages/tui/test/keys.test.ts
//!
//! Tests for keyboard input handling: key matching, parsing, Kitty protocol,
//! legacy sequences, and modifiers.

use sexy_tui_rs::keys::{
    decode_kitty_printable, is_key_release, is_key_repeat, is_kitty_protocol_active, matches_key,
    parse_key, set_kitty_protocol_active, Key, KeyEventType,
};

// =============================================================================
// matchesKey — Kitty protocol alternate keys (non-Latin layouts)
// =============================================================================

#[test]
fn test_kitty_ctrl_c_cyrillic_with_base() {
    set_kitty_protocol_active(true);
    assert!(matches_key("\x1b[1089::99;5u", "ctrl+c"));
    set_kitty_protocol_active(false);
}

#[test]
fn test_kitty_ctrl_d_cyrillic_with_base() {
    set_kitty_protocol_active(true);
    assert!(matches_key("\x1b[1074::100;5u", "ctrl+d"));
    set_kitty_protocol_active(false);
}

#[test]
fn test_kitty_ctrl_z_cyrillic_with_base() {
    set_kitty_protocol_active(true);
    assert!(matches_key("\x1b[1103::122;5u", "ctrl+z"));
    set_kitty_protocol_active(false);
}

#[test]
fn test_kitty_ctrl_shift_p_cyrillic_with_base() {
    set_kitty_protocol_active(true);
    assert!(matches_key("\x1b[1079::112;6u", "ctrl+shift+p"));
    set_kitty_protocol_active(false);
}

#[test]
fn test_kitty_direct_codepoint_no_base() {
    set_kitty_protocol_active(true);
    assert!(matches_key("\x1b[99;5u", "ctrl+c"));
    set_kitty_protocol_active(false);
}

#[test]
fn test_kitty_super_modifier_bindings() {
    set_kitty_protocol_active(true);
    assert!(matches_key("\x1b[107;9u", "super+k"));
    assert!(matches_key("\x1b[13;9u", "super+enter"));
    assert!(matches_key("\x1b[107;13u", "ctrl+super+k"));
    assert!(matches_key("\x1b[107;14u", "ctrl+shift+super+k"));
    assert!(!matches_key("\x1b[107;13u", "super+k")); // wrong modifier
    set_kitty_protocol_active(false);
}

#[test]
fn test_kitty_digit_bindings() {
    set_kitty_protocol_active(true);
    assert!(matches_key("\x1b[49u", "1"));
    assert!(matches_key("\x1b[49;5u", "ctrl+1"));
    assert!(!matches_key("\x1b[49;5u", "ctrl+2"));
    set_kitty_protocol_active(false);
}

#[test]
fn test_kitty_shifted_key_format() {
    set_kitty_protocol_active(true);
    // CSI codepoint:shifted:base;modifier u — shift modifier=1, +1 = 2
    assert!(matches_key("\x1b[99:67:99;2u", "shift+c"));
    set_kitty_protocol_active(false);
}

#[test]
fn test_kitty_dvorak_prefers_codepoint() {
    set_kitty_protocol_active(true);
    // Dvorak Ctrl+K: codepoint 'k' (107), base layout 'v' (118)
    assert!(matches_key("\x1b[107::118;5u", "ctrl+k"));
    assert!(!matches_key("\x1b[107::118;5u", "ctrl+v"));
    set_kitty_protocol_active(false);
}

#[test]
fn test_kitty_dvorak_symbol_prefers_codepoint() {
    set_kitty_protocol_active(true);
    // Dvorak Ctrl+/: codepoint '/' (47), base layout '[' (91)
    assert!(matches_key("\x1b[47::91;5u", "ctrl+/"));
    assert!(!matches_key("\x1b[47::91;5u", "ctrl+["));
    set_kitty_protocol_active(false);
}

#[test]
fn test_kitty_wrong_key_with_base() {
    set_kitty_protocol_active(true);
    assert!(!matches_key("\x1b[1089::99;5u", "ctrl+d"));
    set_kitty_protocol_active(false);
}

#[test]
fn test_kitty_wrong_modifiers_with_base() {
    set_kitty_protocol_active(true);
    assert!(!matches_key("\x1b[1089::99;5u", "ctrl+shift+c"));
    set_kitty_protocol_active(false);
}

// =============================================================================
// matchesKey — modifyOtherKeys (xterm)
// =============================================================================

#[test]
fn test_modify_other_keys_ctrl_c() {
    set_kitty_protocol_active(false);
    assert!(matches_key("\x1b[27;5;99~", "ctrl+c"));
}

#[test]
fn test_modify_other_keys_ctrl_d() {
    set_kitty_protocol_active(false);
    assert!(matches_key("\x1b[27;5;100~", "ctrl+d"));
}

#[test]
fn test_modify_other_keys_ctrl_z() {
    set_kitty_protocol_active(false);
    assert!(matches_key("\x1b[27;5;122~", "ctrl+z"));
}

#[test]
fn test_modify_other_keys_enter_variants() {
    set_kitty_protocol_active(false);
    assert!(matches_key("\x1b[27;5;13~", "ctrl+enter"));
    assert!(matches_key("\x1b[27;2;13~", "shift+enter"));
    assert!(matches_key("\x1b[27;3;13~", "alt+enter"));
}

#[test]
fn test_modify_other_keys_tab_variants() {
    set_kitty_protocol_active(false);
    assert!(matches_key("\x1b[27;2;9~", "shift+tab"));
    assert!(matches_key("\x1b[27;5;9~", "ctrl+tab"));
    assert!(matches_key("\x1b[27;3;9~", "alt+tab"));
}

#[test]
fn test_modify_other_keys_backspace_variants() {
    set_kitty_protocol_active(false);
    assert!(matches_key("\x1b[27;1;127~", "backspace"));
    assert!(matches_key("\x1b[27;5;127~", "ctrl+backspace"));
    assert!(matches_key("\x1b[27;3;127~", "alt+backspace"));
}

#[test]
fn test_modify_other_keys_escape() {
    set_kitty_protocol_active(false);
    assert!(matches_key("\x1b[27;1;27~", "escape"));
}

#[test]
fn test_modify_other_keys_space() {
    set_kitty_protocol_active(false);
    assert!(matches_key("\x1b[27;1;32~", "space"));
    assert!(matches_key("\x1b[27;5;32~", "ctrl+space"));
}

#[test]
fn test_modify_other_keys_symbols() {
    set_kitty_protocol_active(false);
    assert!(matches_key("\x1b[27;5;47~", "ctrl+/"));
}

#[test]
fn test_modify_other_keys_digits() {
    set_kitty_protocol_active(false);
    assert!(matches_key("\x1b[27;5;49~", "ctrl+1"));
    assert!(matches_key("\x1b[27;2;49~", "shift+1"));
}

#[test]
fn test_modify_other_keys_shifted_uppercase() {
    set_kitty_protocol_active(false);
    assert!(matches_key("\x1b[27;2;69~", "shift+e"));
    assert!(matches_key("\x1b[27;6;69~", "ctrl+shift+e"));
}

#[test]
fn test_ctrl_alt_letter_via_csi_u_kitty_inactive() {
    set_kitty_protocol_active(false);
    assert!(matches_key("\x1b[104;7u", "ctrl+alt+h"));
}

#[test]
fn test_ctrl_alt_letter_via_modify_other_keys() {
    set_kitty_protocol_active(false);
    assert!(matches_key("\x1b[27;7;104~", "ctrl+alt+h"));
}

// =============================================================================
// matchesKey — Legacy key matching
// =============================================================================

#[test]
fn test_legacy_ctrl_c() {
    set_kitty_protocol_active(false);
    assert!(matches_key("\x03", "ctrl+c"));
}

#[test]
fn test_legacy_ctrl_d() {
    set_kitty_protocol_active(false);
    assert!(matches_key("\x04", "ctrl+d"));
}

#[test]
fn test_escape_key() {
    assert!(matches_key("\x1b", "escape"));
    assert!(matches_key("\x1b", "esc"));
}

#[test]
fn test_legacy_linefeed_as_enter() {
    set_kitty_protocol_active(false);
    assert!(matches_key("\n", "enter"));
}

#[test]
fn test_ctrl_space() {
    set_kitty_protocol_active(false);
    assert!(matches_key("\x00", "ctrl+space"));
}

#[test]
fn test_legacy_ctrl_symbols() {
    set_kitty_protocol_active(false);
    // Ctrl+\ sends ASCII 28
    assert!(matches_key("\x1c", "ctrl+\\"));
    // Ctrl+] sends ASCII 29
    assert!(matches_key("\x1d", "ctrl+]"));
    // Ctrl+_ / Ctrl+- sends ASCII 31
    assert!(matches_key("\x1f", "ctrl+_"));
}

#[test]
fn test_raw_backspace() {
    set_kitty_protocol_active(false);
    assert!(matches_key("\x7f", "backspace"));
    assert!(!matches_key("\x7f", "ctrl+backspace"));
    assert!(matches_key("\x08", "backspace"));
    assert!(matches_key("\x08", "ctrl+h"));
}

#[test]
fn test_enter_variants() {
    set_kitty_protocol_active(false);
    assert!(matches_key("\r", "enter"));
    assert!(matches_key("\n", "enter"));
}

#[test]
fn test_tab_variants() {
    set_kitty_protocol_active(false);
    assert!(matches_key("\t", "tab"));
    assert!(matches_key("\x1b[Z", "shift+tab"));
}

#[test]
fn test_alt_enter() {
    set_kitty_protocol_active(false);
    assert!(matches_key("\x1b\r", "alt+enter"));
    assert!(matches_key("\x1b\n", "alt+enter"));
}

#[test]
fn test_alt_space() {
    set_kitty_protocol_active(false);
    assert!(matches_key("\x1b ", "alt+space"));
}

// =============================================================================
// Special keys
// =============================================================================

#[test]
fn test_arrow_keys() {
    set_kitty_protocol_active(false);
    assert!(matches_key("\x1b[A", "up"));
    assert!(matches_key("\x1b[B", "down"));
    assert!(matches_key("\x1b[C", "right"));
    assert!(matches_key("\x1b[D", "left"));
}

#[test]
fn test_arrow_keys_modifiers() {
    set_kitty_protocol_active(false);
    assert!(matches_key("\x1b[1;2A", "shift+up"));
    assert!(matches_key("\x1b[1;3B", "alt+down"));
    assert!(matches_key("\x1b[1;5C", "ctrl+right"));
}

#[test]
fn test_home_end() {
    set_kitty_protocol_active(false);
    assert!(matches_key("\x1b[H", "home"));
    assert!(matches_key("\x1b[F", "end"));
}

#[test]
fn test_page_up_down() {
    set_kitty_protocol_active(false);
    assert!(matches_key("\x1b[5~", "pageup"));
    assert!(matches_key("\x1b[6~", "pagedown"));
}

#[test]
fn test_insert_delete() {
    set_kitty_protocol_active(false);
    assert!(matches_key("\x1b[2~", "insert"));
    assert!(matches_key("\x1b[3~", "delete"));
}

#[test]
fn test_function_keys() {
    set_kitty_protocol_active(true);
    let f_keys = [
        ("f1", "\x1b[11u"),
        ("f2", "\x1b[12u"),
        ("f3", "\x1b[13u"),
        ("f4", "\x1b[14u"),
        ("f5", "\x1b[15u"),
        ("f6", "\x1b[17u"),
        ("f7", "\x1b[18u"),
        ("f8", "\x1b[19u"),
        ("f9", "\x1b[20u"),
        ("f10", "\x1b[21u"),
        ("f11", "\x1b[23u"),
        ("f12", "\x1b[24u"),
    ];
    for (key, seq) in &f_keys {
        assert!(matches_key(seq, key), "failed for key: {}", key);
    }
    set_kitty_protocol_active(false);
}

// =============================================================================
// parseKey
// =============================================================================

#[test]
fn test_parse_key_single_chars() {
    assert_eq!(parse_key("a"), KeyEventType::Char('a'));
    assert_eq!(parse_key("A"), KeyEventType::Char('A'));
    assert_eq!(parse_key("1"), KeyEventType::Char('1'));
    assert_eq!(parse_key("!"), KeyEventType::Char('!'));
}

#[test]
fn test_parse_key_control_chars() {
    set_kitty_protocol_active(false);
    assert_eq!(parse_key("\x1b"), KeyEventType::Key(Key::escape));
    assert_eq!(parse_key("\r"), KeyEventType::Key(Key::enter));
    assert_eq!(parse_key("\t"), KeyEventType::Key(Key::tab));
    assert_eq!(parse_key("\x7f"), KeyEventType::Key(Key::backspace));
    // Ctrl+@ (ASCII NUL)
    assert_eq!(parse_key("\x00"), KeyEventType::Key("ctrl+ "));
    set_kitty_protocol_active(false);
}

#[test]
fn test_parse_key_escape_sequences() {
    set_kitty_protocol_active(false);
    assert_eq!(parse_key("\x1b[A"), KeyEventType::Key(Key::up));
    assert_eq!(parse_key("\x1b[B"), KeyEventType::Key(Key::down));
    assert_eq!(parse_key("\x1b[C"), KeyEventType::Key(Key::right));
    assert_eq!(parse_key("\x1b[D"), KeyEventType::Key(Key::left));
    assert_eq!(parse_key("\x1b[H"), KeyEventType::Key(Key::home));
    assert_eq!(parse_key("\x1b[F"), KeyEventType::Key(Key::end));
    assert_eq!(parse_key("\x1b[2~"), KeyEventType::Key(Key::insert));
    assert_eq!(parse_key("\x1b[3~"), KeyEventType::Key(Key::delete));
    assert_eq!(parse_key("\x1b[5~"), KeyEventType::Key(Key::page_up));
    assert_eq!(parse_key("\x1b[6~"), KeyEventType::Key(Key::page_down));
    assert_eq!(parse_key("\x1b[Z"), KeyEventType::Key("shift+tab"));
    // Alt combos
    assert_eq!(parse_key("\x1b\r"), KeyEventType::Key("alt+enter"));
    assert_eq!(parse_key("\x1b "), KeyEventType::Key("alt+ "));
    set_kitty_protocol_active(false);
}

#[test]
fn test_parse_key_kitty_digit() {
    set_kitty_protocol_active(true);
    assert_eq!(parse_key("\x1b[49u"), KeyEventType::Char('1'));
    set_kitty_protocol_active(false);
}

#[test]
fn test_parse_key_unknown_returns_unknown() {
    set_kitty_protocol_active(true);
    // Something that isn't a known key
    let result = parse_key("\x1b[999u");
    match result {
        KeyEventType::Key(_) | KeyEventType::Char(_) | KeyEventType::Unknown(_) => {}
    }
    set_kitty_protocol_active(false);
}

#[test]
fn test_parse_key_empty() {
    assert_eq!(parse_key(""), KeyEventType::Unknown(String::new()));
}

// =============================================================================
// decodeKittyPrintable
// =============================================================================

#[test]
fn test_decode_kitty_printable_letters() {
    assert_eq!(decode_kitty_printable("\x1b[97u"), Some('a'));
    assert_eq!(decode_kitty_printable("\x1b[98u"), Some('b'));
    assert_eq!(decode_kitty_printable("\x1b[65u"), Some('A'));
}

#[test]
fn test_decode_kitty_printable_symbols() {
    assert_eq!(decode_kitty_printable("\x1b[33u"), Some('!'));
    assert_eq!(decode_kitty_printable("\x1b[64u"), Some('@'));
}

#[test]
fn test_decode_kitty_printable_not_kitty() {
    assert_eq!(decode_kitty_printable("a"), None);
    assert_eq!(decode_kitty_printable("\x1b"), None);
    assert_eq!(decode_kitty_printable(""), None);
}

// =============================================================================
// isKeyRelease / isKeyRepeat
// =============================================================================

#[test]
fn test_is_key_release_kitty() {
    // Kitty release: modifier has bit 3 set (value 8). 1-indexed: 9.
    assert!(is_key_release("\x1b[97;9u"));
}

#[test]
fn test_is_key_release_legacy() {
    // Legacy release: event type 3 in the sequence
    assert!(is_key_release("\x1b[27:3~"));
}

#[test]
fn test_is_key_release_false() {
    assert!(!is_key_release("\x1b[97;5u")); // press, not release
    assert!(!is_key_release("a"));
    assert!(!is_key_release(""));
}

#[test]
fn test_is_key_repeat() {
    // Kitty repeat: event type 2
    assert!(is_key_repeat("\x1b[97:2u"));
}

#[test]
fn test_is_key_repeat_false() {
    assert!(!is_key_repeat("\x1b[97;5u"));
    assert!(!is_key_repeat("a"));
}

// =============================================================================
// Key constants
// =============================================================================

#[test]
fn test_key_constant_values() {
    assert_eq!(Key::escape, "escape");
    assert_eq!(Key::enter, "enter");
    assert_eq!(Key::tab, "tab");
    assert_eq!(Key::space, "space");
    assert_eq!(Key::backspace, "backspace");
    assert_eq!(Key::delete, "delete");
    assert_eq!(Key::up, "up");
    assert_eq!(Key::down, "down");
    assert_eq!(Key::left, "left");
    assert_eq!(Key::right, "right");
    assert_eq!(Key::home, "home");
    assert_eq!(Key::end, "end");
    assert_eq!(Key::page_up, "pageUp");
    assert_eq!(Key::page_down, "pageDown");
    assert_eq!(Key::f1, "f1");
    assert_eq!(Key::f12, "f12");
}

#[test]
fn test_key_helpers() {
    assert_eq!(Key::ctrl("c"), "ctrl+c");
    assert_eq!(Key::shift("c"), "shift+c");
    assert_eq!(Key::alt("c"), "alt+c");
    assert_eq!(Key::super_key("k"), "super+k");
    assert_eq!(Key::ctrl_shift("p"), "ctrl+shift+p");
    assert_eq!(Key::ctrl_alt("h"), "ctrl+alt+h");
    assert_eq!(Key::ctrl_super("k"), "ctrl+super+k");
}

// =============================================================================
// Kitty protocol active state
// =============================================================================

#[test]
fn test_kitty_protocol_active_toggle() {
    set_kitty_protocol_active(true);
    assert!(is_kitty_protocol_active());
    set_kitty_protocol_active(false);
    assert!(!is_kitty_protocol_active());
}
