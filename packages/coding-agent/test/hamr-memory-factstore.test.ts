/**
 * FactStore integration tests — verify the full entity extraction → probe → related pipeline.
 *
 * These tests use an in-memory SQLite database (better-sqlite3) so they work
 * without any filesystem setup and run fast (< 10ms each).
 */
import { describe, expect, it } from "vitest";
import { FactStore } from "../src/hamr/memory/FactStore.ts";
import { loadBetterSqlite3 } from "../src/hamr/store/sqlite-loader.ts";

describe("FactStore entity extraction and retrieval", () => {
	const Database = loadBetterSqlite3();

	it("addFact extracts and links entities from content", () => {
		if (!Database) return; // skip if better-sqlite3 unavailable
		const db = new Database(":memory:");
		const store = new FactStore(db);

		const factId = store.addFact('The `search_memory` tool in "hamr" uses FTS5 for full-text search.', "tool,memory");
		expect(factId).toBeGreaterThan(0);

		// Entities should be extracted: "search_memory" (backtick), "hamr" (double-quote)
		const entities = store.getFactEntities(factId!);
		expect(entities.length).toBe(2);
		expect(entities).toContain("search_memory");
		expect(entities).toContain("hamr");

		db.close();
	});

	it("probe finds facts by entity name (case-insensitive)", () => {
		if (!Database) return;
		const db = new Database(":memory:");
		const store = new FactStore(db);

		store.addFact('"hamr" runs on Relay for local inference.', "tool");
		store.addFact('"postgres" stores structured data.', "database");

		const results = store.probe("hamr", 10);
		expect(results.length).toBe(1);
		expect(results[0]?.content).toContain("Relay");

		db.close();
	});

	it("probe falls back to FTS5 keyword search when no structured links exist", () => {
		if (!Database) return;
		const db = new Database(":memory:");
		const store = new FactStore(db);

		// Add a fact without double-quoting the term (so entity isn't extracted)
		store.addFact("hamr runs on Relay for local inference.", "tool");

		// "hamr" isn't double-quoted so not extracted as entity — but FTS5 should find it
		const results = store.probe("hamr", 10);
		expect(results.length).toBeGreaterThanOrEqual(1);
		expect(results.some((r) => r.content.includes("Relay"))).toBe(true);

		db.close();
	});

	it("related finds facts sharing entities with the target", () => {
		if (!Database) return;
		const db = new Database(":memory:");
		const store = new FactStore(db);

		// Two facts both mention "hamr" as double-quoted entity
		store.addFact('"hamr" uses FTS5 for memory search.', "tool,memory");
		store.addFact('"hamr" supports Relay for local inference.', "tool,relay");
		// This one doesn't mention "hamr" as an entity
		store.addFact("postgres is a relational database.", "database");

		const results = store.related("hamr", 10);
		expect(results.length).toBeGreaterThanOrEqual(1);
		// Both hamr facts should appear (direct + related through shared entities)
		const contents = results.map((r) => r.content);
		expect(contents.some((c) => c.includes("FTS5"))).toBe(true);
		expect(contents.some((c) => c.includes("Relay"))).toBe(true);

		db.close();
	});

	it("getFact returns entities in the result", () => {
		if (!Database) return;
		const db = new Database(":memory:");
		const store = new FactStore(db);

		const factId = store.addFact('"typescript" and "vitest" power the test suite.', "testing");
		expect(factId).toBeGreaterThan(0);

		const fact = store.getFact(factId!);
		expect(fact).not.toBeNull();
		expect(fact?.entities).toBeDefined();
		expect(fact?.entities).toContain("typescript");
		expect(fact?.entities).toContain("vitest");

		db.close();
	});

	it("searchFacts uses the improved sanitizer (handles file paths)", () => {
		if (!Database) return;
		const db = new Database(":memory:");
		const store = new FactStore(db);

		store.addFact("The memory code is at src/hamr/memory/HolographicMemory.ts", "code");

		// This query would be destroyed by the old sanitizer
		const results = store.searchFacts("HolographicMemory.ts", 10);
		expect(results.length).toBe(1);
		expect(results[0]?.content).toContain("HolographicMemory");

		db.close();
	});

	it("searchFacts handles hyphenated queries", () => {
		if (!Database) return;
		const db = new Database(":memory:");
		const store = new FactStore(db);

		store.addFact("We use better-sqlite3 for the database layer.", "database");

		const results = store.searchFacts("better-sqlite3", 10);
		expect(results.length).toBe(1);
		expect(results[0]?.content).toContain("better-sqlite3");

		db.close();
	});

	it("addFact deduplicates by content (UNIQUE constraint)", () => {
		if (!Database) return;
		const db = new Database(":memory:");
		const store = new FactStore(db);

		const id1 = store.addFact("hamr is a coding agent.", "tool");
		const id2 = store.addFact("hamr is a coding agent.", "tool");
		expect(id1).toBeGreaterThan(0);
		// Second insert should return the same id (ON CONFLICT DO UPDATE)
		expect(id2).toBe(id1);
		expect(store.getFactCount()).toBe(1);

		db.close();
	});

	it("recordFeedback adjusts trust asymmetrically", () => {
		if (!Database) return;
		const db = new Database(":memory:");
		const store = new FactStore(db);

		const factId = store.addFact("Relay handles model lifecycle.", "relay");
		expect(factId).toBeGreaterThan(0);

		const result = store.recordFeedback(factId!, true); // helpful
		expect(result).not.toBe(false);
		if (result) {
			expect(result.oldTrust).toBe(0.5); // default trust
			expect(result.newTrust).toBe(0.55); // +0.05
		}

		const result2 = store.recordFeedback(factId!, false); // unhelpful
		expect(result2).not.toBe(false);
		if (result2) {
			expect(result2.oldTrust).toBe(0.55);
			expect(result2.newTrust).toBeCloseTo(0.45, 2); // -0.10
		}

		db.close();
	});

	it("getFactCount returns accurate count", () => {
		if (!Database) return;
		const db = new Database(":memory:");
		const store = new FactStore(db);

		expect(store.getFactCount()).toBe(0);
		store.addFact("fact one", "tag");
		store.addFact("fact two", "tag");
		store.addFact("fact three", "tag");
		expect(store.getFactCount()).toBe(3);

		db.close();
	});

	it("listRecentFacts returns newest durable facts first", () => {
		if (!Database) return;
		const db = new Database(":memory:");
		const store = new FactStore(db);

		store.addFact("older memory", "tag");
		store.addFact("newer memory", "tag");

		const results = store.listRecentFacts(2);
		expect(results.map((r) => r.content)).toEqual(["newer memory", "older memory"]);

		db.close();
	});

	it("regression: bare * wildcard lists all facts instead of returning empty", () => {
		if (!Database) return;
		const db = new Database(":memory:");
		const store = new FactStore(db);

		store.addFact("fact alpha", "tag");
		store.addFact("fact beta", "tag");
		store.addFact("fact gamma", "tag");

		// Bare "*" should list all facts, not return empty
		const results = store.searchFacts("*", 10);
		expect(results.length).toBe(3);

		db.close();
	});

	it("regression: entity links are cleaned up on fact upsert", () => {
		if (!Database) return;
		const db = new Database(":memory:");
		const store = new FactStore(db);

		// Add a fact with double-quoted entity "hamr"
		store.addFact('"hamr" uses "postgres" for storage.', "tool");
		const entities1 = store.getFactEntities(1);
		expect(entities1).toContain("hamr");
		expect(entities1).toContain("postgres");

		// Re-add with different content (same fact content = upsert) — but entity links
		// should be cleaned up. Actually the content is the UNIQUE key, so upsert only
		// triggers when content matches. For a true upsert test we need same content.
		// Instead, re-add the SAME content and verify entity count stays correct.
		store.addFact('"hamr" uses "postgres" for storage.', "tool");
		const entities2 = store.getFactEntities(1);
		// Should still have 2 entities (no duplicates from stale links)
		expect(entities2.length).toBe(2);
		expect(entities2).toContain("hamr");
		expect(entities2).toContain("postgres");

		db.close();
	});
});
