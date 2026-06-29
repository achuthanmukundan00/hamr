//! Ported from packages/tui/test/input.test.ts
//!
//! Tests for the Input widget: value management, kill ring, undo.

use sexy_tui_rs::keys::set_kitty_protocol_active;
use sexy_tui_rs::widgets::Input;
use sexy_tui_rs::Component;

// =============================================================================
// Input component — basic behavior
// =============================================================================

mod input_basic {
    use super::*;

    #[test]
    fn test_submits_value_including_backslash_on_enter() {
        let mut input = Input::new();
        use std::{cell::RefCell, rc::Rc};
        let submitted: Rc<RefCell<Option<String>>> = Rc::new(RefCell::new(None));
        let cb_submitted = submitted.clone();
        input.on_submit = Some(Box::new(move |value| {
            *cb_submitted.borrow_mut() = Some(value.to_string());
        }));

        input.handle_input("h");
        input.handle_input("e");
        input.handle_input("l");
        input.handle_input("l");
        input.handle_input("o");
        input.handle_input("\\");
        input.handle_input("\r");

        let result = submitted.borrow().clone();
        assert_eq!(result, Some("hello\\".to_string()));
    }

    #[test]
    fn test_inserts_backslash_as_regular_character() {
        let mut input = Input::new();

        input.handle_input("\\");
        input.handle_input("x");

        assert_eq!(input.get_value(), "\\x");
    }
}

// =============================================================================
// Kill ring
// =============================================================================

mod kill_ring {
    use super::*;

    #[test]
    fn test_ctrl_w_saves_deleted_text_and_ctrl_y_yanks_it() {
        let mut input = Input::new();

        input.set_value("foo bar baz");
        input.handle_input("\x05"); // Ctrl+E — go to end

        input.handle_input("\x17"); // Ctrl+W — deletes "baz"
        assert_eq!(input.get_value(), "foo bar ");

        // Move to beginning and yank
        input.handle_input("\x01"); // Ctrl+A
        input.handle_input("\x19"); // Ctrl+Y
        assert_eq!(input.get_value(), "bazfoo bar ");
    }

    #[test]
    fn test_ctrl_w_preserves_ascii_punctuation_boundaries() {
        let mut input = Input::new();

        input.set_value("foo.bar");
        input.handle_input("\x05"); // Ctrl+E
        input.handle_input("\x17"); // Ctrl+W — deletes "bar"
        assert_eq!(input.get_value(), "foo.");

        input.set_value("foo:bar");
        input.handle_input("\x05"); // Ctrl+E
        input.handle_input("\x17"); // Ctrl+W — deletes "bar"
        assert_eq!(input.get_value(), "foo:");
    }

    #[test]
    fn test_ctrl_u_saves_deleted_text_to_kill_ring() {
        let mut input = Input::new();

        input.set_value("hello world");
        // Move cursor to after "hello "
        input.handle_input("\x01"); // Ctrl+A
        for _ in 0..6 {
            input.handle_input("\x1b[C");
        }

        input.handle_input("\x15"); // Ctrl+U — deletes "hello "
        assert_eq!(input.get_value(), "world");

        input.handle_input("\x19"); // Ctrl+Y
        assert_eq!(input.get_value(), "hello world");
    }

    #[test]
    fn test_ctrl_k_saves_deleted_text_to_kill_ring() {
        let mut input = Input::new();

        input.set_value("hello world");
        input.handle_input("\x01"); // Ctrl+A
        input.handle_input("\x0b"); // Ctrl+K — deletes "hello world"

        assert_eq!(input.get_value(), "");

        input.handle_input("\x19"); // Ctrl+Y
        assert_eq!(input.get_value(), "hello world");
    }

    #[test]
    fn test_ctrl_y_does_nothing_when_kill_ring_is_empty() {
        let mut input = Input::new();

        input.set_value("test");
        input.handle_input("\x05"); // Ctrl+E
        input.handle_input("\x19"); // Ctrl+Y
        assert_eq!(input.get_value(), "test");
    }

    #[test]
    fn test_consecutive_ctrl_w_accumulates_into_one_kill_ring_entry() {
        let mut input = Input::new();

        input.set_value("one two three");
        input.handle_input("\x05"); // Ctrl+E
        input.handle_input("\x17"); // Ctrl+W — deletes "three"
        input.handle_input("\x17"); // Ctrl+W — deletes "two "
        input.handle_input("\x17"); // Ctrl+W — deletes "one "

        assert_eq!(input.get_value(), "");

        input.handle_input("\x19"); // Ctrl+Y
        assert_eq!(input.get_value(), "one two three");
    }

    #[test]
    fn test_yank_in_middle_of_text() {
        let mut input = Input::new();

        input.set_value("word");
        input.handle_input("\x05"); // Ctrl+E
        input.handle_input("\x17"); // Ctrl+W — deletes "word"
        input.set_value("hello world");
        // Move to middle (after "hello ")
        input.handle_input("\x01"); // Ctrl+A
        for _ in 0..6 {
            input.handle_input("\x1b[C");
        }

        input.handle_input("\x19"); // Ctrl+Y
        assert_eq!(input.get_value(), "hello wordworld");
    }

    #[test]
    fn test_alt_d_deletes_word_forward() {
        let mut input = Input::new();

        input.set_value("hello world test");
        input.handle_input("\x01"); // Ctrl+A

        // Alt+D via Kitty protocol
        set_kitty_protocol_active(true);
        input.handle_input("\x1b[100;3u"); // Alt+D — deletes "hello"
        set_kitty_protocol_active(false);
        assert_eq!(input.get_value(), " world test");
    }

    #[test]
    fn test_alt_d_preserves_ascii_punctuation_boundaries() {
        let mut input = Input::new();

        input.set_value("foo.bar baz");
        input.handle_input("\x01"); // Ctrl+A
        set_kitty_protocol_active(true);
        input.handle_input("\x1b[100;3u"); // Alt+D — deletes "foo" (forward word)
        set_kitty_protocol_active(false);
        assert_eq!(input.get_value(), ".bar baz");
        set_kitty_protocol_active(true);
        input.handle_input("\x1b[100;3u"); // Alt+D — deletes "."
        set_kitty_protocol_active(false);
        assert_eq!(input.get_value(), "bar baz");
        set_kitty_protocol_active(true);
        input.handle_input("\x1b[100;3u"); // Alt+D — deletes "bar"
        set_kitty_protocol_active(false);
        assert_eq!(input.get_value(), " baz");
    }
}

// =============================================================================
// Undo
// =============================================================================

mod undo {
    use super::*;

    #[test]
    fn test_undo_does_nothing_when_stack_is_empty() {
        let mut input = Input::new();

        input.handle_input("\x01"); // Ctrl+A as undo (same binding)
        assert_eq!(input.get_value(), "");
    }

    #[test]
    fn test_undoes_backspace() {
        let mut input = Input::new();

        input.handle_input("h");
        input.handle_input("e");
        input.handle_input("l");
        input.handle_input("l");
        input.handle_input("o");
        input.handle_input("\x7f"); // Backspace
        assert_eq!(input.get_value(), "hell");

        // The Input widget maps Ctrl+Z to undo
        input.handle_input("\x1a"); // Ctrl+Z (undo)
        assert_eq!(input.get_value(), "hello");
    }

    #[test]
    fn test_undoes_forward_delete() {
        let mut input = Input::new();

        input.handle_input("h");
        input.handle_input("e");
        input.handle_input("l");
        input.handle_input("l");
        input.handle_input("o");
        input.handle_input("\x01"); // Ctrl+A — go to start
        input.handle_input("\x1b[C"); // Right arrow
        input.handle_input("\x1b[3~"); // Delete key
        assert_eq!(input.get_value(), "hllo");

        input.handle_input("\x1a"); // Ctrl+Z (undo)
        assert_eq!(input.get_value(), "hello");
    }

    #[test]
    fn test_undoes_ctrl_w_delete_word_backward() {
        let mut input = Input::new();

        input.handle_input("h");
        input.handle_input("e");
        input.handle_input("l");
        input.handle_input("l");
        input.handle_input("o");
        input.handle_input(" ");
        input.handle_input("w");
        input.handle_input("o");
        input.handle_input("r");
        input.handle_input("l");
        input.handle_input("d");
        assert_eq!(input.get_value(), "hello world");

        input.handle_input("\x17"); // Ctrl+W
        assert_eq!(input.get_value(), "hello ");

        input.handle_input("\x1a"); // Ctrl+Z (undo)
        assert_eq!(input.get_value(), "hello world");
    }

    #[test]
    fn test_undoes_ctrl_k_delete_to_line_end() {
        let mut input = Input::new();

        input.handle_input("h");
        input.handle_input("e");
        input.handle_input("l");
        input.handle_input("l");
        input.handle_input("o");
        input.handle_input(" ");
        input.handle_input("w");
        input.handle_input("o");
        input.handle_input("r");
        input.handle_input("l");
        input.handle_input("d");
        input.handle_input("\x01"); // Ctrl+A
        for _ in 0..6 {
            input.handle_input("\x1b[C");
        }

        input.handle_input("\x0b"); // Ctrl+K
        assert_eq!(input.get_value(), "hello ");

        input.handle_input("\x1a"); // Ctrl+Z (undo)
        assert_eq!(input.get_value(), "hello world");
    }

    #[test]
    fn test_undoes_ctrl_u_delete_to_line_start() {
        let mut input = Input::new();

        input.handle_input("h");
        input.handle_input("e");
        input.handle_input("l");
        input.handle_input("l");
        input.handle_input("o");
        input.handle_input(" ");
        input.handle_input("w");
        input.handle_input("o");
        input.handle_input("r");
        input.handle_input("l");
        input.handle_input("d");
        input.handle_input("\x01"); // Ctrl+A
        for _ in 0..6 {
            input.handle_input("\x1b[C");
        }

        input.handle_input("\x15"); // Ctrl+U
        assert_eq!(input.get_value(), "world");

        input.handle_input("\x1a"); // Ctrl+Z (undo)
        assert_eq!(input.get_value(), "hello world");
    }

    #[test]
    fn test_undoes_yank() {
        let mut input = Input::new();

        input.handle_input("h");
        input.handle_input("e");
        input.handle_input("l");
        input.handle_input("l");
        input.handle_input("o");
        input.handle_input(" ");
        input.handle_input("\x17"); // Ctrl+W — delete "hello "
        input.handle_input("\x19"); // Ctrl+Y — yank
        assert_eq!(input.get_value(), "hello ");

        input.handle_input("\x1a"); // Ctrl+Z (undo)
        assert_eq!(input.get_value(), "");
    }

    #[test]
    fn test_undoes_alt_d_delete_word_forward() {
        let mut input = Input::new();

        input.set_value("hello world");
        input.handle_input("\x01"); // Ctrl+A

        set_kitty_protocol_active(true);
        input.handle_input("\x1b[100;3u"); // Alt+D — deletes "hello"
        set_kitty_protocol_active(false);
        assert_eq!(input.get_value(), " world");

        input.handle_input("\x1a"); // Ctrl+Z (undo)
        assert_eq!(input.get_value(), "hello world");
    }
}
