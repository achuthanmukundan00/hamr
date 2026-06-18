import { describe, expect, it } from "vitest";
import { routeInterruptKey } from "../src/modes/interactive/interrupt-routing.ts";

describe("global interrupt routing", () => {
	it("interrupts an active stream regardless of focus", () => {
		expect(
			routeInterruptKey({
				isStreaming: true,
				isBashRunning: false,
				inSpecialEscapeMode: false,
				autocompleteShowing: false,
			}),
		).toBe("interrupt-stream");
	});

	it("aborts a running bash command when not streaming", () => {
		expect(
			routeInterruptKey({
				isStreaming: false,
				isBashRunning: true,
				inSpecialEscapeMode: false,
				autocompleteShowing: false,
			}),
		).toBe("interrupt-bash");
	});

	it("defers to the focused editor when nothing is running", () => {
		expect(
			routeInterruptKey({
				isStreaming: false,
				isBashRunning: false,
				inSpecialEscapeMode: false,
				autocompleteShowing: false,
			}),
		).toBe("defer");
	});

	it("defers during compaction/retry special escape modes even while streaming", () => {
		expect(
			routeInterruptKey({
				isStreaming: true,
				isBashRunning: false,
				inSpecialEscapeMode: true,
				autocompleteShowing: false,
			}),
		).toBe("defer");
	});

	it("defers while the autocomplete popup is open so escape cancels it", () => {
		expect(
			routeInterruptKey({
				isStreaming: true,
				isBashRunning: false,
				inSpecialEscapeMode: false,
				autocompleteShowing: true,
			}),
		).toBe("defer");
	});
});
