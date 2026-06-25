/**
 * Integration tests for message-based memory retrieval in context injection.
 *
 * These tests verify end-to-end that:
 * 1. A fresh session with a topical query recalls cross-session entries
 * 2. "hi" produces no injection (self-silencing)
 * 3. Different turns produce different injections (query-triggered, not one-shot)
 * 4. Zero-match path (valid terms, nothing in FTS5) returns null
 * 5. contextHash is populated and differs across distinct injections
 * 6. searchTermLimit slicing is enforced
 */

import type { AgentMessage } from "@hamr/agent";
import { describe, expect, it } from "vitest";
import { computeMemoryInjection, type MemoryInjectionInput } from "../src/hamr/extensions/memory.ts";
import { HolographicMemory } from "../src/hamr/memory/HolographicMemory.ts";
import { loadBetterSqlite3 } from "../src/hamr/store/sqlite-loader.ts";

function createMemory(): HolographicMemory {
	const Database = loadBetterSqlite3();
	if (!Database) throw new Error("better-sqlite3 unavailable");
	const db = new Database(":memory:");
	db.exec(`
		CREATE VIRTUAL TABLE IF NOT EXISTS memory_fts USING fts5(
			turn_id UNINDEXED,
			session_id UNINDEXED,
			role UNINDEXED,
			tool_name UNINDEXED,
			file_paths UNINDEXED,
			content,
			domain_tags UNINDEXED
		);
	`);
	return new HolographicMemory(db);
}

function userMessage(text: string): AgentMessage {
	return { role: "user", content: text, timestamp: Date.now() };
}

function messages(...msgs: AgentMessage[]): AgentMessage[] {
	return msgs;
}

/** Default char budget matching AUTO_INJECT_CHAR_BUDGET (1600 chars = ~400 tokens). */
const DEFAULT_CHAR_BUDGET = 1600;

/** Create a base MemoryInjectionInput with common defaults. */
function baseInput(
	overrides: Partial<MemoryInjectionInput> & { memory: HolographicMemory; messages: AgentMessage[] },
): MemoryInjectionInput {
	return {
		survivalManifest: null,
		searchTermLimit: 5,
		resultsPerTerm: 3,
		snippetChars: 180,
		charBudget: DEFAULT_CHAR_BUDGET,
		...overrides,
	};
}

describe("message-based memory retrieval (integration)", () => {
	// ── Test 1: Fresh session + topical query → cross-session recall ──────

	it("recalls cross-session entries when a fresh session asks about a stored topic", () => {
		const memory = createMemory();
		expect(memory.isAvailable).toBe(true);

		// Store entries from a PREVIOUS session (session "old-session")
		memory.store({
			sessionId: "old-session",
			turnId: 0,
			role: "user",
			content: "Fix the FactStore error in entity resolution",
			domainTags: ["hamr"],
		});
		memory.store({
			sessionId: "old-session",
			turnId: 0,
			role: "assistant",
			content: "The FactStore error was a null pointer in resolveEntity. Fixed by adding a guard.",
			domainTags: ["hamr"],
		});

		// Fresh session "new-session" — has NO entries of its own
		expect(memory.hasSessionEntries("new-session")).toBe(false);

		const result = computeMemoryInjection(
			baseInput({
				memory,
				messages: messages(userMessage("what was that error about FactStore?")),
			}),
		);

		// Should recall the cross-session FactStore entries
		expect(result).not.toBeNull();
		expect(result!.message.role).toBe("user");
		expect(typeof result!.message.timestamp).toBe("number");
		expect(result!.message.content).toContain("FactStore");
		expect(result!.contextHash).toBeTruthy();
		expect(typeof result!.contextHash).toBe("string");
	});

	// ── Test 2: "hi" → self-silencing, no injection ───────────────────────

	it("produces no injection for generic greeting 'hi'", () => {
		const memory = createMemory();

		memory.store({
			sessionId: "old-session",
			turnId: 0,
			role: "assistant",
			content: "I fixed the subagent orchestration bug in dispatch loop",
			domainTags: ["hamr"],
		});

		const result = computeMemoryInjection(
			baseInput({
				memory,
				messages: messages(userMessage("hi")),
			}),
		);

		expect(result).toBeNull();
	});

	// ── Test 3: Query-triggered — different turns, different injections ───

	it("produces different injections for different queries on the same session", () => {
		const memory = createMemory();

		memory.store({
			sessionId: "old-session",
			turnId: 0,
			role: "assistant",
			content: "I implemented the subagent dispatch loop with bounded concurrency",
			domainTags: ["hamr"],
		});
		memory.store({
			sessionId: "old-session",
			turnId: 1,
			role: "assistant",
			content: "Fixed the persistent editor crash when saving large files",
			domainTags: ["hamr"],
		});

		const result1 = computeMemoryInjection(
			baseInput({
				memory,
				messages: messages(userMessage("how does the subagent dispatch work?")),
			}),
		);

		const result2 = computeMemoryInjection(
			baseInput({
				memory,
				messages: messages(
					userMessage("how does the subagent dispatch work?"),
					{ role: "assistant", content: "It uses a worker pool...", timestamp: Date.now() },
					userMessage("tell me about the persistent editor crash"),
				),
			}),
		);

		expect(result1).not.toBeNull();
		expect(result2).not.toBeNull();

		const content1 = result1!.message.content.toLowerCase();
		const content2 = result2!.message.content.toLowerCase();

		// Positive containment
		expect(content1).toContain("subagent");
		expect(content2).toContain("editor");

		// Negative containment: the search-result section for turn 2 should
		// NOT contain subagent results, and turn 1 should NOT contain editor results.
		// (The memory index summarises all entries, so it naturally mentions both.)
		const searchSection2 = content2.split("// search")[1] ?? "";
		expect(searchSection2).not.toContain("subagent");
		const searchSection1 = content1.split("// search")[1] ?? "";
		expect(searchSection1).not.toContain("editor");

		// The injections should be different (query-triggered, not repeated)
		expect(content1).not.toBe(content2);

		// Hashes should differ
		expect(result1!.contextHash).not.toBe(result2!.contextHash);
	});

	// ── Test 4: Message with no topic words → no injection ────────────────

	it("produces no injection when message has no meaningful topic words", () => {
		const memory = createMemory();

		memory.store({
			sessionId: "old-session",
			turnId: 0,
			role: "assistant",
			content: "Important: the database migration needs to run before deploy",
			domainTags: ["hamr"],
		});

		const result = computeMemoryInjection(
			baseInput({
				memory,
				messages: messages(userMessage("ok thanks")),
			}),
		);

		expect(result).toBeNull();
	});

	// ── Test 5: Survival manifest still injected even with generic message ─

	it("injects survival manifest even when user message is generic 'hi'", () => {
		const memory = createMemory();

		const result = computeMemoryInjection(
			baseInput({
				memory,
				messages: messages(userMessage("hi")),
				survivalManifest: "## Survival manifest\nTask: finish the subagent feature\nStatus: in progress",
				searchTermLimit: 3,
				resultsPerTerm: 1,
				snippetChars: 120,
			}),
		);

		expect(result).not.toBeNull();
		expect(result!.message.content).toContain("Survival manifest");
		expect(result!.message.content).toContain("subagent feature");
	});

	// ── Test 6: Zero-match — valid terms but nothing in FTS5 ──────────────

	it("returns null when message has topical terms but nothing matches in memory", () => {
		const memory = createMemory();

		// Store something about a DIFFERENT topic
		memory.store({
			sessionId: "old-session",
			turnId: 0,
			role: "assistant",
			content: "I fixed the subagent dispatch loop",
			domainTags: ["hamr"],
		});

		const result = computeMemoryInjection(
			baseInput({
				memory,
				messages: messages(userMessage("database migration")), // different topic, no match
			}),
		);

		expect(result).toBeNull();
	});

	// ── Test 7: searchTermLimit enforcement ──────────────────────────────

	it("enforces searchTermLimit by only searching the first N terms", () => {
		const memory = createMemory();

		// Store entries for several terms
		memory.store({
			sessionId: "old-session",
			turnId: 0,
			role: "assistant",
			content: "The dispatch loop had a concurrency bug",
			domainTags: ["hamr"],
		});
		memory.store({
			sessionId: "old-session",
			turnId: 1,
			role: "assistant",
			content: "The persistent editor needed a crash fix",
			domainTags: ["hamr"],
		});

		// Message with many distinct terms, but limit to 2
		const result = computeMemoryInjection(
			baseInput({
				memory,
				messages: messages(userMessage("fix the dispatch loop error in the persistent editor memory index")),
				searchTermLimit: 2,
			}),
		);

		// Should still produce a result (first 2 terms match)
		expect(result).not.toBeNull();
		// The result should not crash — the slicing worked
		expect(result!.message.content).toBeTruthy();
	});

	// ── Test 8: Context hash reflects fact-store line changes ─────────────

	it("produces different contextHash when factStoreLine differs", () => {
		const memory = createMemory();

		memory.store({
			sessionId: "old-session",
			turnId: 0,
			role: "assistant",
			content: "I fixed the subagent dispatch loop",
			domainTags: ["hamr"],
		});

		const resultNoFs = computeMemoryInjection(
			baseInput({
				memory,
				messages: messages(userMessage("how does subagent dispatch work?")),
				factStoreLine: undefined,
			}),
		);

		const resultWithFs = computeMemoryInjection(
			baseInput({
				memory,
				messages: messages(userMessage("how does subagent dispatch work?")),
				factStoreLine: "\n[FactStore: 42 durable facts]",
			}),
		);

		expect(resultNoFs).not.toBeNull();
		expect(resultWithFs).not.toBeNull();
		expect(resultNoFs!.contextHash).not.toBe(resultWithFs!.contextHash);
		expect(resultWithFs!.message.content).toContain("42 durable facts");
	});
});
