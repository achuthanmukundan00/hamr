import { describe, expect, it } from "vitest";
import { FactStore } from "../src/hamr/memory/FactStore.ts";
import { HolographicMemory } from "../src/hamr/memory/HolographicMemory.ts";
import { loadNodeSqliteDatabase } from "../src/hamr/store/node-sqlite-adapter.ts";

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

describe("node:sqlite adapter (better-sqlite3 fallback)", () => {
	const Database = loadNodeSqliteDatabase();

	it("loads the node:sqlite backend on this Node version", () => {
		// Node 22.5+ ships node:sqlite; the whole fallback depends on this.
		expect(Database).not.toBeNull();
	});

	it("is a drop-in for HolographicMemory (store + FTS5 search + snippets)", () => {
		if (!Database) return;
		const db = new Database(":memory:");
		db.pragma("journal_mode = WAL");
		db.exec(MEMORY_FTS_SCHEMA);

		const memory = new HolographicMemory(db);
		memory.store({
			turnId: "t1",
			sessionId: "s1",
			role: "assistant",
			content: "fixed the login form validation bug",
			domainTags: ["auth"],
		});
		memory.store({
			turnId: "t2",
			sessionId: "s1",
			role: "user",
			content: "unrelated note about deployment",
		});

		expect(memory.storeErrorCount).toBe(0);

		const results = memory.search("login");
		expect(results.length).toBe(1);
		expect(results[0]?.content).toContain("login form");

		const snippets = memory.searchWithSnippets("validation");
		expect(snippets.length).toBe(1);
		expect(snippets[0]?.snippet).toContain("<mark>");

		expect(memory.hasSessionEntries("s1")).toBe(true);
		db.close();
	});

	it("is a drop-in for FactStore (addFact returns rowid, searchFacts works)", () => {
		if (!Database) return;
		const db = new Database(":memory:");
		const facts = FactStore.create(db);

		const id = facts.addFact("the relay runs qwen on a single gpu", "relay,gpu");
		expect(id).toBeGreaterThan(0);
		expect(facts.getFactCount()).toBe(1);

		const found = facts.searchFacts("relay");
		expect(found.length).toBe(1);
		expect(found[0]?.content).toContain("qwen");
		db.close();
	});
});
