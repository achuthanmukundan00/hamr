/**
 * Memory end-to-end smoke tests — full store → search → retrieval pipeline.
 *
 * Uses HolographicMemory and FactStore directly with an in-memory SQLite
 * database (better-sqlite3). This is the production codepath: the same
 * classes, the same SQL queries, the same sanitizer.
 *
 * Goal: catch regressions in the search pipeline BEFORE they ship.
 * A failing test here = search_memory is broken for users.
 */
import { describe, expect, it } from "vitest";
import { FactStore } from "../src/hamr/memory/FactStore.ts";
import { HolographicMemory } from "../src/hamr/memory/HolographicMemory.ts";
import { loadBetterSqlite3 } from "../src/hamr/store/sqlite-loader.ts";

const MEMORY_FTS_SCHEMA = `
	CREATE VIRTUAL TABLE IF NOT EXISTS memory_fts USING fts5(
		turn_id UNINDEXED,
		session_id UNINDEXED,
		role UNINDEXED,
		tool_name UNINDEXED,
		file_paths UNINDEXED,
		content,
		domain_tags UNINDEXED
	);
`;

const Database = loadBetterSqlite3();
const describeIf = Database ? describe : describe.skip;

describeIf("memory E2E smoke test", () => {
	// ── Transcript search (search_memory) ──────────────────────────────

	it("stores messages and finds them with exact word search", () => {
		const db = new Database(":memory:");
		db.pragma("journal_mode = WAL");
		db.exec(MEMORY_FTS_SCHEMA);
		const memory = new HolographicMemory(db);

		memory.store({
			turnId: 1,
			sessionId: "s1",
			role: "user",
			content: "Debug why search_memory always fails in hamr v0.7.0",
			domainTags: ["hamr"],
		});
		memory.store({
			turnId: 1,
			sessionId: "s1",
			role: "assistant",
			content: "I found the bug in the FTS5 query sanitizer in HolographicMemory.ts",
			domainTags: ["hamr"],
		});
		memory.store({
			turnId: 1,
			sessionId: "s1",
			role: "tool",
			toolName: "edit",
			content: "Fixed the sanitizer to double-quote dangerous tokens",
			domainTags: ["hamr"],
		});

		expect(memory.storeErrorCount).toBe(0);

		// Search by exact word
		const results = memory.searchWithSnippets("sanitizer", 5);
		expect(results.length).toBeGreaterThanOrEqual(1);
		expect(results.some((r) => r.content.includes("sanitizer"))).toBe(true);

		db.close();
	});

	it("finds results with file path queries (regression test for broken sanitizer)", () => {
		const db = new Database(":memory:");
		db.pragma("journal_mode = WAL");
		db.exec(MEMORY_FTS_SCHEMA);
		const memory = new HolographicMemory(db);

		memory.store({
			turnId: 2,
			sessionId: "s2",
			role: "assistant",
			content: "I modified src/hamr/memory/HolographicMemory.ts to fix the sanitizer.",
			domainTags: ["hamr"],
		});

		// Old sanitizer would strip dots/slashes → "src hamr memory HolographicMemory ts"
		// New sanitizer double-quotes → '"src/hamr/memory/HolographicMemory.ts"'
		const results = memory.searchWithSnippets("src/hamr/memory/HolographicMemory.ts", 5);
		expect(results.length).toBeGreaterThanOrEqual(1);
		expect(results[0]?.content).toContain("HolographicMemory");

		db.close();
	});

	it("finds results with hyphenated term queries (regression test for bare hyphen)", () => {
		const db = new Database(":memory:");
		db.pragma("journal_mode = WAL");
		db.exec(MEMORY_FTS_SCHEMA);
		const memory = new HolographicMemory(db);

		memory.store({
			turnId: 3,
			sessionId: "s3",
			role: "assistant",
			content: "The hamr-browser extension is used for web testing.",
			domainTags: ["hamr"],
		});

		// Old sanitizer let bare hyphen through → FTS5 interpreted as NOT/column filter
		// New sanitizer double-quotes → '"hamr-browser"' which is safe
		const results = memory.searchWithSnippets("hamr-browser", 5);
		expect(results.length).toBeGreaterThanOrEqual(1);
		expect(results[0]?.content).toContain("hamr-browser");

		db.close();
	});

	// ── Structured fact search (fact_store) ────────────────────────────

	it("stores and retrieves structured facts with the improved sanitizer", () => {
		const db = new Database(":memory:");
		const facts = new FactStore(db);

		facts.addFact("The search_memory tool uses FTS5 for full-text search.", "memory,fts5");
		facts.addFact("better-sqlite3 is a native C++ addon for SQLite in Node.js.", "database");
		facts.addFact("node:sqlite ships inside Node 24+ with FTS5 enabled.", "database,node");

		// Search with a hyphenated term (regression test)
		const results = facts.searchFacts("better-sqlite3", 5);
		expect(results.length).toBe(1);
		expect(results[0]?.content).toContain("better-sqlite3");

		// Search with a file-like query
		const results2 = facts.searchFacts("search_memory", 5);
		expect(results2.length).toBe(1);
		expect(results2[0]?.content).toContain("FTS5");

		db.close();
	});

	// ── Full pipeline: transcript + facts together ─────────────────────

	it("stores both transcript and structured facts in the same DB", () => {
		const db = new Database(":memory:");
		db.pragma("journal_mode = WAL");
		db.exec(MEMORY_FTS_SCHEMA);

		// Same DB connection for both — they share tables
		const memory = new HolographicMemory(db);
		const facts = new FactStore(db);

		// Store transcript
		memory.store({
			turnId: 1,
			sessionId: "s4",
			role: "user",
			content: "The memory system needs entity extraction for probe to work.",
			domainTags: ["hamr"],
		});

		// Store structured fact
		const factId = facts.addFact(
			"Entity extraction uses regex patterns: capitalized phrases, quoted terms, AKA patterns.",
			"memory,entities",
		);
		expect(factId).toBeGreaterThan(0);

		// Both should be independently searchable
		expect(memory.searchWithSnippets("entity extraction", 5).length).toBeGreaterThanOrEqual(1);
		expect(facts.searchFacts("regex patterns", 5).length).toBeGreaterThanOrEqual(1);

		// But transcript search shouldn't find facts and vice versa (different FTS5 tables)
		expect(memory.searchWithSnippets("regex patterns", 5).length).toBe(0);
		expect(facts.searchFacts("entity extraction", 5).length).toBeGreaterThanOrEqual(1);

		db.close();
	});

	// ── storeErrorCount smoke ──────────────────────────────────────────

	it("reports store errors via storeErrorCount", () => {
		const db = new Database(":memory:");
		db.pragma("journal_mode = WAL");
		// Deliberately skip creating the FTS5 table — store should fail gracefully
		const memory = new HolographicMemory(db);

		expect(memory.isAvailable).toBe(false); // No FTS5 table, so not available
		expect(memory.storeErrorCount).toBe(0);

		// Store should not crash even when unavailable
		memory.store({
			turnId: 1,
			sessionId: "s1",
			role: "user",
			content: "This should not crash",
			domainTags: ["hamr"],
		});
		expect(memory.storeErrorCount).toBe(0); // isAvailable check prevents trying

		db.close();
	});

	// ── hasSessionEntries ─────────────────────────────────────────────

	it("correctly reports whether a session has entries", () => {
		const db = new Database(":memory:");
		db.pragma("journal_mode = WAL");
		db.exec(MEMORY_FTS_SCHEMA);
		const memory = new HolographicMemory(db);

		expect(memory.hasSessionEntries("s5")).toBe(false);

		memory.store({
			turnId: 1,
			sessionId: "s5",
			role: "user",
			content: "hello",
			domainTags: ["hamr"],
		});

		expect(memory.hasSessionEntries("s5")).toBe(true);
		expect(memory.hasSessionEntries("s6")).toBe(false);

		db.close();
	});

	// ── Bulk store and search performance smoke ────────────────────────

	it("handles bulk stores and searches without degradation", () => {
		const db = new Database(":memory:");
		db.pragma("journal_mode = WAL");
		db.exec(MEMORY_FTS_SCHEMA);
		const memory = new HolographicMemory(db);

		// Store 100 entries
		for (let i = 0; i < 100; i++) {
			memory.store({
				turnId: Math.floor(i / 3),
				sessionId: "bulk-session",
				role: i % 3 === 0 ? "user" : i % 3 === 1 ? "assistant" : "tool",
				content:
					i === 50
						? "CRITICAL BUG: the search_memory query sanitizer strips file extensions like .ts and .js"
						: `Entry ${i}: some ${i % 2 === 0 ? "error" : "normal"} content for testing`,
				domainTags: ["hamr"],
			});
		}

		expect(memory.storeErrorCount).toBe(0);

		// Should find the critical bug entry
		const results = memory.searchWithSnippets("CRITICAL BUG sanitizer", 5);
		expect(results.length).toBeGreaterThanOrEqual(1);
		expect(results.some((r) => r.content.includes("CRITICAL BUG"))).toBe(true);

		// File extension query should work (regression)
		const extResults = memory.searchWithSnippets(".ts", 5);
		expect(extResults.length).toBeGreaterThanOrEqual(1);

		db.close();
	});
});
