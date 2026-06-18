/**
 * Pure routing decision for the global (TUI-level) interrupt key.
 *
 * The interrupt key (esc) is registered as a TUI-level input listener so it
 * fires regardless of which component is focused — that is what makes "esc to
 * interrupt" reliable while a model is streaming, even when an extension widget
 * or overlay holds focus. This function decides whether that global listener
 * should act (and how) or defer to the focused editor's own escape handling.
 *
 * It defers in two cases so existing behavior is preserved:
 *  - special escape modes (compaction / auto-retry), where the editor's escape
 *    handler is temporarily rebound to abort that operation;
 *  - while the editor autocomplete popup is open, where escape cancels the popup.
 */
export interface InterruptKeyState {
	isStreaming: boolean;
	isBashRunning: boolean;
	inSpecialEscapeMode: boolean;
	autocompleteShowing: boolean;
}

export type InterruptKeyRoute = "interrupt-stream" | "interrupt-bash" | "defer";

export function routeInterruptKey(state: InterruptKeyState): InterruptKeyRoute {
	if (state.inSpecialEscapeMode || state.autocompleteShowing) return "defer";
	if (state.isStreaming) return "interrupt-stream";
	if (state.isBashRunning) return "interrupt-bash";
	return "defer";
}
