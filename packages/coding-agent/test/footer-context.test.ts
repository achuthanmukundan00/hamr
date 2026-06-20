import { describe, expect, it } from "vitest";
import { formatContextPart } from "../src/modes/interactive/components/footer.ts";

describe("formatContextPart", () => {
	it("returns undefined when contextWindow is 0", () => {
		expect(formatContextPart(1000, 0, 50, false)).toBeUndefined();
	});

	it("returns undefined when contextWindow is negative", () => {
		expect(formatContextPart(1000, -1, 50, false)).toBeUndefined();
	});

	it("shows ? when tokens is null (unknown)", () => {
		expect(formatContextPart(null, 128000, null, false)).toBe("? used of 128K tokens");
	});

	it("shows ? when tokens is undefined (getContextUsage returned undefined)", () => {
		// Fix: == null catches both null and undefined
		expect(formatContextPart(undefined, 128000, undefined, false)).toBe("? used of 128K tokens");
	});

	it("shows ? when percent is null even if tokens is a number", () => {
		expect(formatContextPart(5000, 128000, null, false)).toBe("? used of 128K tokens");
	});

	it("shows compact format when compact is true", () => {
		expect(formatContextPart(5000, 128000, 3.9, true)).toBe("4% / 128K");
	});

	it("shows non-compact format when compact is false", () => {
		expect(formatContextPart(5000, 128000, 3.9, false)).toBe("4% used of 128K tokens");
	});

	it("shows ? in compact mode for unknown tokens", () => {
		expect(formatContextPart(null, 128000, null, true)).toBe("? / 128K");
	});

	it("shows 0% when tokens is 0", () => {
		expect(formatContextPart(0, 128000, 0, false)).toBe("0% used of 128K tokens");
	});

	it("formats large context windows", () => {
		expect(formatContextPart(50000, 1000000, 5, true)).toBe("5% / 1.0M");
	});

	it("shows error color threshold above 90% does not affect format", () => {
		// Just verify the function returns a string (coloring is done by caller)
		const result = formatContextPart(120000, 128000, 93.75, false);
		expect(result).toBe("94% used of 128K tokens");
	});
});
