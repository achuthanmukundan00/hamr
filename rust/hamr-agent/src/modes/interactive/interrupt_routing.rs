//! Port of `packages/coding-agent/src/modes/interactive/interrupt-routing.ts`.
//!
//! Pure routing decision for the global (TUI-level) interrupt key.

/// Input state used to decide how the global interrupt key should be routed.
pub struct InterruptKeyState {
    pub is_streaming: bool,
    pub is_bash_running: bool,
    pub in_special_escape_mode: bool,
    pub autocomplete_showing: bool,
}

/// Possible routing outcomes for the global interrupt key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InterruptKeyRoute {
    /// Interrupt the current model stream.
    InterruptStream,
    /// Interrupt a running bash process.
    InterruptBash,
    /// Defer to the focused component (e.g., editor escape handler).
    Defer,
}

/// Determine how the global interrupt key should be routed.
///
/// The interrupt key (esc) is registered as a TUI-level input listener so it
/// fires regardless of which component is focused — that is what makes "esc to
/// interrupt" reliable while a model is streaming, even when an extension widget
/// or overlay holds focus. This function decides whether that global listener
/// should act (and how) or defer to the focused editor's own escape handling.
///
/// It defers in two cases so existing behavior is preserved:
///  - special escape modes (compaction / auto-retry), where the editor's escape
///    handler is temporarily rebound to abort that operation;
///  - while the editor autocomplete popup is open, where escape cancels the popup.
pub fn route_interrupt_key(state: &InterruptKeyState) -> InterruptKeyRoute {
    if state.in_special_escape_mode || state.autocomplete_showing {
        return InterruptKeyRoute::Defer;
    }
    if state.is_streaming {
        return InterruptKeyRoute::InterruptStream;
    }
    if state.is_bash_running {
        return InterruptKeyRoute::InterruptBash;
    }
    InterruptKeyRoute::Defer
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_defers_when_special_escape_mode() {
        let state = InterruptKeyState {
            is_streaming: true,
            is_bash_running: false,
            in_special_escape_mode: true,
            autocomplete_showing: false,
        };
        assert_eq!(route_interrupt_key(&state), InterruptKeyRoute::Defer);
    }

    #[test]
    fn test_defers_when_autocomplete_showing() {
        let state = InterruptKeyState {
            is_streaming: true,
            is_bash_running: false,
            in_special_escape_mode: false,
            autocomplete_showing: true,
        };
        assert_eq!(route_interrupt_key(&state), InterruptKeyRoute::Defer);
    }

    #[test]
    fn test_interrupts_stream() {
        let state = InterruptKeyState {
            is_streaming: true,
            is_bash_running: false,
            in_special_escape_mode: false,
            autocomplete_showing: false,
        };
        assert_eq!(
            route_interrupt_key(&state),
            InterruptKeyRoute::InterruptStream
        );
    }

    #[test]
    fn test_interrupts_bash() {
        let state = InterruptKeyState {
            is_streaming: false,
            is_bash_running: true,
            in_special_escape_mode: false,
            autocomplete_showing: false,
        };
        assert_eq!(
            route_interrupt_key(&state),
            InterruptKeyRoute::InterruptBash
        );
    }

    #[test]
    fn test_stream_priority_over_bash() {
        let state = InterruptKeyState {
            is_streaming: true,
            is_bash_running: true,
            in_special_escape_mode: false,
            autocomplete_showing: false,
        };
        assert_eq!(
            route_interrupt_key(&state),
            InterruptKeyRoute::InterruptStream
        );
    }

    #[test]
    fn test_defers_when_idle() {
        let state = InterruptKeyState {
            is_streaming: false,
            is_bash_running: false,
            in_special_escape_mode: false,
            autocomplete_showing: false,
        };
        assert_eq!(route_interrupt_key(&state), InterruptKeyRoute::Defer);
    }
}
