/**
 * Tests for sanitizeFts5Query — the FTS5 query sanitizer in HolographicMemory.
 *
 * Regression suite for the bug where the old sanitizer (`/[^\w\s*\-"()]/g`)
 * stripped meaningful query characters (`.` `/` `:` `@`) while letting
 * dangerous FTS5 operators (`-`) through. This test suite codifies the
 * exact behavior every query must satisfy.
 */
import { describe, expect, it } from "vitest";
import { sanitizeFts5Query } from "../src/hamr/memory/HolographicMemory.ts";

describe("sanitizeFts5Query", () => {
	// ── Simple word queries (must pass through unchanged) ──────────

	it("passes through simple words", () => {
		expect(sanitizeFts5Query("error")).toBe("error");
		expect(sanitizeFts5Query("login")).toBe("login");
	});

	it("passes through multi-word queries", () => {
		expect(sanitizeFts5Query("login form validation")).toBe("login form validation");
	});

	it("passes through underscore-separated identifiers", () => {
		expect(sanitizeFts5Query("search_memory")).toBe("search_memory");
		expect(sanitizeFts5Query("fact_store")).toBe("fact_store");
	});

	// ── File paths (must be double-quoted to preserve as single FTS5 token) ──

	it("double-quotes file paths containing slashes", () => {
		expect(sanitizeFts5Query("src/hamr/memory.ts")).toBe('"src/hamr/memory.ts"');
	});

	it("double-quotes absolute paths", () => {
		expect(sanitizeFts5Query("/tmp/devour-hermes-agent-memory")).toBe('"/tmp/devour-hermes-agent-memory"');
	});

	it("double-quotes paths with dots", () => {
		expect(sanitizeFts5Query("packages/coding-agent/src/hamr/memory/HolographicMemory.ts")).toBe(
			'"packages/coding-agent/src/hamr/memory/HolographicMemory.ts"',
		);
	});

	// ── Hyphenated terms (regression: old sanitizer let bare `-` through → FTS5 error) ──

	it("double-quotes terms containing hyphens", () => {
		expect(sanitizeFts5Query("hamr-browser")).toBe('"hamr-browser"');
		expect(sanitizeFts5Query("better-sqlite3")).toBe('"better-sqlite3"');
		expect(sanitizeFts5Query("hamr-memory-context-injection")).toBe('"hamr-memory-context-injection"');
	});

	it("double-quotes terms with hyphens in a multi-word query", () => {
		expect(sanitizeFts5Query("fix the hamr-browser extension")).toBe('fix the "hamr-browser" extension');
	});

	// ── Colons and @-signs (must be double-quoted) ──

	it("double-quotes terms containing colons", () => {
		expect(sanitizeFts5Query("@skaft/hamr")).toBe('"@skaft/hamr"');
	});

	it("handles error message patterns with colons", () => {
		// "Error: something broke" → token "Error:" gets double-quoted, rest stays bare
		const result = sanitizeFts5Query("Error: something broke");
		// Must contain the quoted Error: token
		expect(result).toContain('"Error:"');
		// Must still contain the bare continuation tokens
		expect(result).toContain("something");
		expect(result).toContain("broke");
	});

	// ── Prefix queries (asterisk preserved after quoting) ──

	it("preserves bare prefix queries", () => {
		expect(sanitizeFts5Query("error*")).toBe("error*");
	});

	it("preserves prefix on quoted path queries", () => {
		expect(sanitizeFts5Query("src/hamr/*")).toBe('"src/hamr/"*');
	});

	it("preserves prefix on hyphenated terms", () => {
		expect(sanitizeFts5Query("hamr-*")).toBe('"hamr-"*');
	});

	// ── Intentional phrase queries (already quoted — must be preserved) ──

	it("preserves existing double-quoted phrase queries", () => {
		expect(sanitizeFts5Query('"search_memory always fails"')).toBe('"search_memory always fails"');
		expect(sanitizeFts5Query('"login form"')).toBe('"login form"');
	});

	it("preserves quoted phrases with internal punctuation", () => {
		expect(sanitizeFts5Query('"src/hamr/memory.ts"')).toBe('"src/hamr/memory.ts"');
	});

	// ── Edge cases ──

	it("handles empty input", () => {
		const result = sanitizeFts5Query("");
		// Returns "*" (match-all) as fallback, or empty
		expect(typeof result).toBe("string");
	});

	it("handles whitespace-only input", () => {
		const result = sanitizeFts5Query("   ");
		expect(typeof result).toBe("string");
	});

	it("handles only-punctuation input gracefully", () => {
		const result = sanitizeFts5Query("... --- ???");
		expect(typeof result).toBe("string");
		// Should not contain raw triple-dots (FTS5 may not handle them)
	});

	it("strips truly dangerous control characters", () => {
		// Null bytes kill SQLite — must be stripped
		expect(sanitizeFts5Query("hello\x00world")).toBe("helloworld");
	});

	it("preserves numeric identifiers", () => {
		expect(sanitizeFts5Query("error 404")).toBe("error 404");
		expect(sanitizeFts5Query("turn 42")).toBe("turn 42");
	});

	it("handles mixed safe and unsafe tokens", () => {
		const result = sanitizeFts5Query("fix the login form in src/auth.ts");
		expect(result).toContain('"src/auth.ts"');
		expect(result).toContain("fix");
		expect(result).toContain("login");
	});

	// ── Regression: the exact bug pattern ──

	it("regression: does NOT strip dots from file paths", () => {
		// Old sanitizer: /[^\w\s*\-"()]/g → stripped dots → "memory ts" instead of "memory.ts"
		expect(sanitizeFts5Query("memory.ts")).toBe('"memory.ts"');
	});

	it("regression: does NOT let bare hyphen through in hyphenated terms", () => {
		// Old sanitizer preserved `-` but bare `-` is an FTS5 column filter / NOT operator
		// The result should not contain a bare hyphen (\"hamr-browser\" is safe)
		const result = sanitizeFts5Query("hamr-browser");
		expect(result).not.toMatch(/^\s*hamr-browser\s*$/);
		expect(result).toBe('"hamr-browser"');
	});

	it("regression: preserves meaningful path context", () => {
		// Old sanitizer: "src/hamr/memory.ts" → "src hamr memory ts" (four unrelated tokens)
		// New sanitizer: "src/hamr/memory.ts" → '"src/hamr/memory.ts"' (one phrase token)
		expect(sanitizeFts5Query("src/hamr/memory.ts")).toBe('"src/hamr/memory.ts"');
	});
});
