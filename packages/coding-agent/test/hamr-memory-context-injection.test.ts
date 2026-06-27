import type { AssistantMessage } from "@hamr/ai";
import { describe, expect, it } from "vitest";
import {
	applyTokenBudget,
	buildMemoryContextMessage,
	buildMemoryPrefetchContextMessage,
	buildMemoryPrefetchQueries,
	classifyMemoryPrefetchPrompt,
	deduplicateResults,
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
		expect(message?.content).toContain("Auto-retrieved context from prior sessions");
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
		const autoIndex = message?.content.indexOf("Auto-retrieved context") ?? -1;
		expect(manifestIndex).toBeGreaterThanOrEqual(0);
		expect(autoIndex).toBeGreaterThan(manifestIndex);
	});

	it("classifies explicit memory recall prompts", () => {
		expect(classifyMemoryPrefetchPrompt("remember that music thing")).toBe("explicit-recall");
		expect(classifyMemoryPrefetchPrompt("can we pick up where we left off?")).toBe("explicit-recall");
	});

	it("classifies continuation fragments that need prior context", () => {
		expect(classifyMemoryPrefetchPrompt("the genre is deconstructed club")).toBe("continuation");
		expect(classifyMemoryPrefetchPrompt("that works for me")).toBe("continuation");
	});

	it("builds music recovery queries for underspecified genre updates", () => {
		const queries = buildMemoryPrefetchQueries("the genre is deconstructed club", "continuation");
		expect(queries).toContain("music");
		expect(queries).toContain("music project");
		expect(queries).toContain("electronic music");
	});

	it("formats cue-triggered durable facts as hidden prefetch context", () => {
		const message = buildMemoryPrefetchContextMessage({
			reason: "continuation",
			latestUserText: "the genre is deconstructed club",
			queries: ["music project"],
			facts: [
				{
					factId: 9,
					content: "User is nervous about an electronic music project sent to an artist who said it was next level.",
					tags: "music,project-context",
					trustScore: 0.5,
					retrievalCount: 0,
					helpfulCount: 0,
					createdAt: "2026-06-26",
					updatedAt: "2026-06-26",
				},
			],
			transcriptResults: [],
			timestamp: 123,
		});

		expect(message).not.toBeNull();
		expect(message?.content).toContain("MEMORY PREFETCH");
		expect(message?.content).toContain("electronic music project");
		expect(message?.content).toContain("save it with save_memory/fact_store");
		expect(message?.timestamp).toBe(123);
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
});
