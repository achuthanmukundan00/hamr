import { describe, expect, it } from "vitest";
import { hamrDefaultExtensions } from "../src/hamr/extensions/index.ts";

describe("hamr default extensions composition", () => {
	it("composes the focused extension factories (no import cycle at load)", () => {
		// Importing the composition resolves all factory modules; an import cycle
		// or a broken factory wiring would throw here.
		expect(Array.isArray(hamrDefaultExtensions)).toBe(true);
		expect(hamrDefaultExtensions.length).toBe(6);
		for (const factory of hamrDefaultExtensions) {
			expect(typeof factory).toBe("function");
		}
	});
});
