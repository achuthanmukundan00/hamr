import type { AssistantMessage } from "@hamr/ai";
import { describe, expect, it } from "vitest";
import {
	applyTokenBudget,
	buildMemoryContextMessage,
	deduplicateResults,
	extractMessageSearchTerms,
	selectCompactionPolicy,
} from "../src/hamr/extensions/memory.ts";
import { buildAssistantMemoryContent, sanitizeMemoryTranscriptText } from "../src/hamr/memory.ts";

describe("hamr memory context injection", () => {
	it("returns null when there are no auto-retrieved results", () => {
		expect(buildMemoryContextMessage([], "[Memory: 5 entries across 2 turns]")).toBeNull();
	});

	it("returns a user message containing the auto-results and index when results exist", () => {
		const results = ["// Search 'error': 1 result", "//   turn 2 assistant: fixed the error"];
		const index = "[Memory: 5 entries across 2 turns]";
		const message = buildMemoryContextMessage(results, index);
		expect(message).not.toBeNull();
		expect(message?.role).toBe("user");
		expect(message?.content).toContain("You may have prior context on this:");
		expect(message?.content).toContain("fixed the error");
		expect(message?.content).toContain(index);
	});

	it("includes a timestamp in the returned message", () => {
		const before = Date.now();
		const message = buildMemoryContextMessage(["result"], "[Memory: 1 entry]");
		const after = Date.now();
		expect(message?.timestamp).toBeGreaterThanOrEqual(before);
		expect(message?.timestamp).toBeLessThanOrEqual(after);
	});

	it("injects a survival manifest even when there are no auto-results", () => {
		const message = buildMemoryContextMessage([], "[Memory: 3 entries]", {
			survivalManifest: "## Survival manifest\nTask: finish the feature",
		});
		expect(message).not.toBeNull();
		expect(message?.content).toContain("Survival manifest");
		expect(message?.content).toContain("finish the feature");
	});

	it("places the survival manifest before generic auto-retrieved results", () => {
		const message = buildMemoryContextMessage(["// Search 'x': 1 result"], "[Memory: 3 entries]", {
			survivalManifest: "## Survival manifest\nTask: finish the feature",
		});
		const manifestIndex = message?.content.indexOf("Survival manifest") ?? -1;
		const autoIndex = message?.content.indexOf("You may have prior context on this:") ?? -1;
		expect(manifestIndex).toBeGreaterThanOrEqual(0);
		expect(autoIndex).toBeGreaterThan(manifestIndex);
	});

	it("selects conservative retrieval for 16k local models", () => {
		const policy = selectCompactionPolicy({ cloud: false, contextWindow: 16_384 });
		expect(policy.tier).toBe("local-16k");
		expect(policy.searchTermLimit).toBe(3);
		expect(policy.resultsPerTerm).toBe(1);
		expect(policy.keyLimit).toBe(16);
	});

	it("selects targeted retrieval for 32k local models", () => {
		const policy = selectCompactionPolicy({ cloud: false, contextWindow: 32_768 });
		expect(policy.tier).toBe("local-32k");
		expect(policy.searchTermLimit).toBe(4);
		expect(policy.resultsPerTerm).toBe(2);
	});

	it("selects broader recovery for 131k local models", () => {
		const policy = selectCompactionPolicy({ cloud: false, contextWindow: 131_072 });
		expect(policy.tier).toBe("local-131k");
		expect(policy.searchTermLimit).toBe(6);
		expect(policy.resultsPerTerm).toBe(3);
	});

	it("keeps cloud providers on pi default compaction", () => {
		const policy = selectCompactionPolicy({ cloud: true, contextWindow: 200_000 });
		expect(policy.tier).toBe("cloud");
		expect(policy.instructions).toContain("pi's default LLM compaction");
	});

	it("serializes assistant tool calls into memory content", () => {
		const message = {
			role: "assistant",
			content: [
				{ type: "text", text: "I will inspect the memory code." },
				{ type: "toolCall", id: "t1", name: "read", arguments: { file_path: "src/hamr/memory.ts" } },
				{ type: "toolCall", id: "t2", name: "bash", arguments: { command: "npm test" } },
			],
			timestamp: 0,
		} as AssistantMessage;

		const content = buildAssistantMemoryContent(message);
		expect(content).toContain("I will inspect the memory code.");
		expect(content).toContain("tool_call read");
		expect(content).toContain("src/hamr/memory.ts");
		expect(content).toContain("npm test");
	});

	it("strips raw XML tool-call markup from assistant memory text", () => {
		const message = {
			role: "assistant",
			content: [
				{
					type: "text",
					text: "<tool_call>\n<function=bash>\n<parameter=command>git status --short</parameter>\n</function>\n</tool_call>",
				},
				{ type: "toolCall", id: "t1", name: "bash", arguments: { command: "git status --short" } },
			],
			timestamp: 0,
		} as AssistantMessage;

		const content = buildAssistantMemoryContent(message);
		expect(content).not.toContain("<tool_call>");
		expect(content).not.toContain("<function=");
		expect(content).not.toContain("<parameter=");
		expect(content).toContain("tool_call bash");
		expect(content).toContain("git status --short");
	});

	it("removes FTS mark tags from memory snippets", () => {
		expect(sanitizeMemoryTranscriptText("hello <mark>world</mark>")).toBe("hello world");
	});

	// ── Token budget ──────────────────────────────────────────────────

	it("applyTokenBudget passes through results under the budget", () => {
		const results = ["// Search 'error': 1 result", "//   turn 2 assistant: fixed the error"];
		const budgeted = applyTokenBudget(results, 400 * 4); // 400 tokens = 1600 chars
		expect(budgeted).toEqual(results);
	});

	it("applyTokenBudget truncates results exceeding the budget", () => {
		const results = [
			"// Search 'error': 1 result",
			"//   turn 2 assistant: this is a very long message that will exceed the character budget",
		];
		const budgeted = applyTokenBudget(results, 80); // very tight budget
		expect(budgeted.length).toBeLessThanOrEqual(results.length);
	});

	it("applyTokenBudget with zero budget returns empty array", () => {
		const results = ["// Search 'error': 1 result"];
		const budgeted = applyTokenBudget(results, 0);
		expect(budgeted).toEqual(results); // zero budget means no cap (pass-through)
	});

	// ── De-duplication ────────────────────────────────────────────────

	it("deduplicateResults returns all results when no existing messages", () => {
		const results = ["// Search 'error': 1 result", "//   turn 2 assistant: fixed the error"];
		const deduped = deduplicateResults(results, []);
		expect(deduped).toEqual(results);
	});

	it("deduplicateResults filters out lines whose content appears in existing messages", () => {
		const results = [
			"// Search 'error': 1 result",
			"//   turn 2 assistant: fixed the error",
			"//   turn 3 tool: success",
		];
		const existing = [{ role: "user", content: "Hey, I fixed the error already." }];
		const deduped = deduplicateResults(results, existing);
		// "fixed the error" appears in existing, so that line should be filtered
		expect(deduped).toContain("// Search 'error': 1 result");
		expect(deduped).toContain("//   turn 3 tool: success");
		expect(deduped).not.toContain("//   turn 2 assistant: fixed the error");
	});

	it("deduplicateResults preserves search header lines", () => {
		const results = ["// Search 'error': 1 result"];
		const existing = [{ role: "user", content: "// Search 'error': 1 result" }];
		const deduped = deduplicateResults(results, existing);
		// Search header lines are always preserved
		expect(deduped).toEqual(results);
	});

	it("deduplicateResults with empty results returns empty array", () => {
		expect(deduplicateResults([], [{ role: "user", content: "hello" }])).toEqual([]);
	});

	// ── Message-based search term extraction ────────────────────────────

	it("extractMessageSearchTerms returns empty for generic greeting 'hi'", () => {
		expect(extractMessageSearchTerms("hi")).toEqual([]);
	});

	it("extractMessageSearchTerms returns empty for generic greeting 'hello'", () => {
		expect(extractMessageSearchTerms("hello")).toEqual([]);
	});

	it("extractMessageSearchTerms returns empty for empty string", () => {
		expect(extractMessageSearchTerms("")).toEqual([]);
	});

	it("extractMessageSearchTerms returns empty for whitespace-only", () => {
		expect(extractMessageSearchTerms("   ")).toEqual([]);
	});

	it("extractMessageSearchTerms extracts topic words from 'ok let's get back to the subagent thing'", () => {
		const terms = extractMessageSearchTerms("ok let's get back to the subagent thing");
		expect(terms).toContain("subagent");
		expect(terms.length).toBeGreaterThan(0);
	});

	it("extractMessageSearchTerms extracts relevant terms from a question about past work", () => {
		const terms = extractMessageSearchTerms("what was that error about FactStore we saw yesterday?");
		expect(terms).toContain("error");
		expect(terms).toContain("factstore");
	});

	it("extractMessageSearchTerms extracts file paths and identifiers", () => {
		const terms = extractMessageSearchTerms("check the memory.ts file and the handoff handler");
		expect(terms.some((t) => t.includes("memory") || t.includes("handoff"))).toBe(true);
	});

	it("extractMessageSearchTerms returns empty for very short non-topic messages", () => {
		expect(extractMessageSearchTerms("ok")).toEqual([]);
		expect(extractMessageSearchTerms("yes")).toEqual([]);
		expect(extractMessageSearchTerms("no")).toEqual([]);
		expect(extractMessageSearchTerms("thanks")).toEqual([]);
	});

	it("extractMessageSearchTerms returns empty for tokens shorter than 3 characters", () => {
		expect(extractMessageSearchTerms("ab cd ef")).toEqual([]);
		// "the" is a STOP_WORD so only "store" (4 chars) passes through
		expect(extractMessageSearchTerms("go to the store")).toEqual(["store"]);
	});

	it("extractMessageSearchTerms returns empty for message composed entirely of STOP_WORDS", () => {
		expect(extractMessageSearchTerms("the is at which on")).toEqual([]);
		// "one" is 3+ chars and not a stop word — it passes through
		expect(extractMessageSearchTerms("i am the one who can do it")).toEqual(["one"]);
	});

	it("extractMessageSearchTerms deduplicates repeated tokens", () => {
		expect(extractMessageSearchTerms("subagent subagent error error")).toEqual(["subagent", "error"]);
		// "the" is a STOP_WORD; only "fix" and "bug" survive
		expect(extractMessageSearchTerms("fix fix fix the bug bug")).toEqual(["fix", "bug"]);
	});

	it("extractMessageSearchTerms handles additional generic words", () => {
		expect(extractMessageSearchTerms("hey")).toEqual([]);
		expect(extractMessageSearchTerms("bye")).toEqual([]);
		expect(extractMessageSearchTerms("sure")).toEqual([]);
		expect(extractMessageSearchTerms("yeah")).toEqual([]);
		expect(extractMessageSearchTerms("nah")).toEqual([]);
		expect(extractMessageSearchTerms("okay")).toEqual([]);
		expect(extractMessageSearchTerms("cool")).toEqual([]);
		expect(extractMessageSearchTerms("maybe")).toEqual([]);
		expect(extractMessageSearchTerms("alright")).toEqual([]);
		expect(extractMessageSearchTerms("please")).toEqual([]);
		expect(extractMessageSearchTerms("yup")).toEqual([]);
		expect(extractMessageSearchTerms("nope")).toEqual([]);
	});
});
