import { describe, expect, it } from "vitest";
import { stripFtsMarks } from "../src/hamr/memory/fts-marks.ts";

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
