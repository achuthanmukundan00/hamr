/**
 * Tests for extractEntities — the entity extraction function in FactStore.
 *
 * Entity extraction uses regex patterns to find named entities in fact content:
 * - Capitalized multi-word phrases (e.g., "John Doe")
 * - Double-quoted terms (e.g., "hamr")
 * - Single-quoted terms (e.g., 'postgres')
 * - Backtick-quoted terms (e.g., `search_memory`)
 * - AKA patterns (e.g., "Guido aka BDFL")
 */
import { describe, expect, it } from "vitest";
import { extractEntities } from "../src/hamr/memory/FactStore.ts";

describe("extractEntities", () => {
	// ── Capitalized multi-word phrases ──

	it("extracts capitalized multi-word phrases", () => {
		const entities = extractEntities("John Doe wrote the React Router library.");
		expect(entities).toContain("John Doe");
		expect(entities).toContain("React Router");
	});

	it("does not extract single capitalized words", () => {
		const entities = extractEntities("Python is great.");
		// "Python" alone is a single word — the regex requires 2+ capitalized words
		expect(entities).not.toContain("Python");
	});

	// ── Double-quoted terms ──

	it("extracts double-quoted terms", () => {
		const entities = extractEntities('The tool "search_memory" is useful in "hamr".');
		expect(entities).toContain("search_memory");
		expect(entities).toContain("hamr");
	});

	// ── Single-quoted terms ──

	it("extracts single-quoted terms", () => {
		const entities = extractEntities("He uses 'postgres' and 'redis' for storage.");
		expect(entities).toContain("postgres");
		expect(entities).toContain("redis");
	});

	// ── Backtick-quoted terms ──

	it("extracts backtick-quoted terms", () => {
		const entities = extractEntities("Call `search_memory` and then `fact_store` to query.");
		expect(entities).toContain("search_memory");
		expect(entities).toContain("fact_store");
	});

	it("skips backtick spans that are too long", () => {
		// The regex limits backtick content to {2,60} characters.
		// Use a string that is 65 characters inside the backtick.
		const longContent = "a".repeat(65);
		const entities = extractEntities(`\`${longContent}\``);
		expect(entities.length).toBe(0);
	});

	it("skips multi-line backtick spans", () => {
		const entities = extractEntities("`multi\nline`");
		expect(entities.length).toBe(0);
	});

	// ── AKA patterns ──

	it("extracts both names from AKA patterns", () => {
		const entities = extractEntities("Guido aka BDFL wrote Python.");
		expect(entities).toContain("Guido");
		expect(entities).toContain("BDFL");
	});

	it("extracts from 'also known as' patterns", () => {
		const entities = extractEntities("Bruce also known as coding-agent created hamr.");
		expect(entities).toContain("Bruce");
		expect(entities).toContain("coding-agent");
	});

	it("regression: does NOT match lowercase-starting AKA false positives", () => {
		// Bug: old regex matched "and AKA patterns" → extracted "and", "patterns"
		const entities = extractEntities("The system uses and AKA patterns for matching.");
		expect(entities).not.toContain("and");
		expect(entities).not.toContain("patterns");
	});

	// ── Deduplication ──

	it("deduplicates entities preserving first-seen order", () => {
		const entities = extractEntities('"hamr" is "hamr" and also "hamr"');
		expect(entities.filter((e) => e === "hamr").length).toBe(1);
	});

	it("case-insensitive deduplication", () => {
		const entities = extractEntities("John Doe and john doe are the same.");
		const lower = entities.map((e) => e.toLowerCase());
		expect(lower.filter((e) => e === "john doe").length).toBe(1);
	});

	// ── Minimum length guard ──

	it("skips entities shorter than 2 characters", () => {
		// "A" and "B" are too short
		const entities = extractEntities('"A" and "B"');
		expect(entities.length).toBe(0);
	});

	// ── Maximum length guard ──

	it("skips entities longer than 120 characters", () => {
		const long = "a".repeat(130);
		const entities = extractEntities(`"${long}"`);
		expect(entities.length).toBe(0);
	});

	// ── Real-world content ──

	it("extracts entities from a typical fact about a tool", () => {
		const content = "The `search_memory` tool in 'hamr' uses FTS5 for full-text search. John Smith implemented it.";
		const entities = extractEntities(content);
		expect(entities).toContain("search_memory");
		expect(entities).toContain("hamr");
		expect(entities).toContain("John Smith");
	});

	it("extracts entities from a project decision", () => {
		const content =
			'"hamr" uses "better-sqlite3" for the memory backend. The relay server (Relayd aka my-relay) handles inference.';
		const entities = extractEntities(content);
		expect(entities).toContain("hamr");
		expect(entities).toContain("better-sqlite3");
		expect(entities).toContain("Relayd");
		expect(entities).toContain("my-relay");
	});

	// ── Empty / whitespace input ──

	it("returns empty array for empty string", () => {
		expect(extractEntities("")).toEqual([]);
	});

	it("returns empty array for whitespace-only string", () => {
		expect(extractEntities("   \n  \t  ")).toEqual([]);
	});
});
