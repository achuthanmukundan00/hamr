import { describe, expect, it } from "vitest";
import { stripFtsMarks, renderFtsMarks } from "../src/hamr/memory/fts-marks.ts";

describe("stripFtsMarks", () => {
	it("removes mark tags", () => {
		expect(stripFtsMarks("hello <mark>world</mark>")).toBe("hello world");
	});
	it("handles multiple marks", () => {
		expect(stripFtsMarks("<mark>foo</mark> bar <mark>baz</mark>")).toBe("foo bar baz");
	});
	it("passes through strings without marks", () => {
		expect(stripFtsMarks("no marks here")).toBe("no marks here");
	});
	it("handles empty string", () => {
		expect(stripFtsMarks("")).toBe("");
	});
});

describe("renderFtsMarks", () => {
	const bracket = (s: string) => `[${s}]`;
	it("applies highlight to marked text", () => {
		expect(renderFtsMarks("hello <mark>world</mark>", bracket)).toBe("hello [world]");
	});
	it("handles multiple marks", () => {
		expect(renderFtsMarks("<mark>foo</mark> <mark>bar</mark>", bracket)).toBe("[foo] [bar]");
	});
	it("passes through unmarked text", () => {
		expect(renderFtsMarks("no marks", bracket)).toBe("no marks");
	});
});
