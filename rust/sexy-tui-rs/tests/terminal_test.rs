//! Ported from packages/tui/test/terminal.test.ts
//!
//! Tests for ProcessTerminal keyboard protocol negotiation and Apple Terminal input normalization.

use sexy_tui_rs::terminal::{
    is_keyboard_protocol_negotiation_sequence_prefix, parse_keyboard_protocol_negotiation_sequence,
};

// =============================================================================
// parseKeyboardProtocolNegotiationSequence
// =============================================================================

mod parse_keyboard_protocol_negotiation {
    use super::*;

    #[test]
    fn test_parses_kitty_flags_response() {
        assert_eq!(
            parse_keyboard_protocol_negotiation_sequence("\x1b[?7u"),
            Some(7)
        );
    }

    #[test]
    fn test_parses_zero_kitty_flags() {
        assert_eq!(
            parse_keyboard_protocol_negotiation_sequence("\x1b[?0u"),
            Some(0)
        );
    }

    #[test]
    fn test_returns_none_for_device_attributes() {
        // Device attributes (DA) response — not a Kitty response
        assert_eq!(
            parse_keyboard_protocol_negotiation_sequence("\x1b[?62;4;52c"),
            None
        );
    }

    #[test]
    fn test_returns_none_for_unrecognized_sequences() {
        assert_eq!(parse_keyboard_protocol_negotiation_sequence("a"), None);
        assert_eq!(parse_keyboard_protocol_negotiation_sequence(""), None);
    }
}

// =============================================================================
// isKeyboardProtocolNegotiationSequencePrefix
// =============================================================================

mod keyboard_protocol_negotiation_prefix {
    use super::*;

    #[test]
    fn test_detects_csi_prefix() {
        assert!(is_keyboard_protocol_negotiation_sequence_prefix("\x1b["));
    }

    #[test]
    fn test_detects_partial_kitty_response_prefix() {
        assert!(
            is_keyboard_protocol_negotiation_sequence_prefix("\x1b[?7"),
            "should detect partial CSI ?... without trailing u"
        );
    }

    #[test]
    fn test_rejects_complete_kitty_response() {
        assert!(
            !is_keyboard_protocol_negotiation_sequence_prefix("\x1b[?7u"),
            "complete response should not be a prefix"
        );
    }

    #[test]
    fn test_rejects_non_prefix_sequences() {
        assert!(!is_keyboard_protocol_negotiation_sequence_prefix("a"));
        assert!(!is_keyboard_protocol_negotiation_sequence_prefix(""));
    }
}
