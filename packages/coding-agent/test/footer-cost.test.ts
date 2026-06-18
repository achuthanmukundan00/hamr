import { describe, expect, it } from "vitest";
import { formatCostPart } from "../src/modes/interactive/components/footer.ts";

describe("footer cost formatting", () => {
	it("shows sub-cent costs at 3-decimal precision", () => {
		expect(formatCostPart(0.003, 3, false)).toBe("$0.003");
	});

	it("shows $0.000 for a priced model with zero accumulated cost", () => {
		expect(formatCostPart(0, 5, false)).toBe("$0.000");
	});

	it("appends the subscription marker", () => {
		expect(formatCostPart(1.2, 5, true)).toBe("$1.200 (sub)");
	});

	it("omits the cost entirely for zero-priced (relay/local) models", () => {
		expect(formatCostPart(0.5, 0, false)).toBeUndefined();
	});
});
