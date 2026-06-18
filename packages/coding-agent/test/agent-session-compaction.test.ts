/**
 * E2E tests for AgentSession compaction behavior.
 *
 * These tests use the faux provider to verify:
 * - Manual compaction works correctly
 * - Session persistence during compaction
 * - Compaction entry is saved to session file
 */

import { fauxAssistantMessage } from "@hamr/ai";
import { afterEach, describe, expect, it } from "vitest";
import type { AgentSession, AgentSessionEvent } from "../src/core/agent-session.ts";
import type { SessionManager } from "../src/core/session-manager.ts";
import { createHarness, type Harness } from "./suite/harness.ts";

describe("AgentSession compaction e2e", () => {
	let harness: Harness | undefined;
	let session: AgentSession;
	let sessionManager: SessionManager;
	let events: AgentSessionEvent[];

	afterEach(() => {
		harness?.cleanup();
		harness = undefined;
	});

	async function createSession() {
		harness = await createHarness({
			settings: { compaction: { keepRecentTokens: 1 } },
		});
		session = harness.session;
		sessionManager = harness.sessionManager;
		events = harness.events;

		return session;
	}

	it("should trigger manual compaction via compact()", async () => {
		await createSession();
		harness!.setResponses([
			fauxAssistantMessage("4"),
			fauxAssistantMessage("6"),
			fauxAssistantMessage("Generated turn prefix summary"),
			fauxAssistantMessage("Generated compaction summary"),
		]);

		// Send a few prompts to build up history
		await session.prompt("What is 2+2? Reply with just the number.");
		await session.agent.waitForIdle();

		await session.prompt("What is 3+3? Reply with just the number.");
		await session.agent.waitForIdle();

		// Manually compact
		const result = await session.compact();

		expect(result.summary).toBeDefined();
		expect(result.summary.length).toBeGreaterThan(0);
		expect(result.tokensBefore).toBeGreaterThan(0);

		// Verify messages were compacted (should have summary + recent)
		const messages = session.messages;
		expect(messages.length).toBeGreaterThan(0);

		// First message should be the summary (a user message with summary content)
		const firstMsg = messages[0];
		expect(firstMsg.role).toBe("compactionSummary");
	}, 120000);

	it("should maintain valid session state after compaction", async () => {
		await createSession();
		harness!.setResponses([
			fauxAssistantMessage("Paris"),
			fauxAssistantMessage("Berlin"),
			fauxAssistantMessage("Generated turn prefix summary"),
			fauxAssistantMessage("Generated compaction summary"),
			fauxAssistantMessage("Rome"),
		]);

		// Build up history
		await session.prompt("What is the capital of France? One word answer.");
		await session.agent.waitForIdle();

		await session.prompt("What is the capital of Germany? One word answer.");
		await session.agent.waitForIdle();

		// Compact
		await session.compact();

		// Session should still be usable
		await session.prompt("What is the capital of Italy? One word answer.");
		await session.agent.waitForIdle();

		// Should have messages after compaction
		expect(session.messages.length).toBeGreaterThan(0);

		// The agent should have responded
		const assistantMessages = session.messages.filter((m) => m.role === "assistant");
		expect(assistantMessages.length).toBeGreaterThan(0);
	}, 180000);

	it("should persist compaction to session file", async () => {
		await createSession();
		harness!.setResponses([
			fauxAssistantMessage("hello"),
			fauxAssistantMessage("goodbye"),
			fauxAssistantMessage("Generated turn prefix summary"),
			fauxAssistantMessage("Generated compaction summary"),
		]);

		await session.prompt("Say hello");
		await session.agent.waitForIdle();

		await session.prompt("Say goodbye");
		await session.agent.waitForIdle();

		// Compact
		await session.compact();

		// Load entries from session manager
		const entries = sessionManager.getEntries();

		// Should have a compaction entry
		const compactionEntries = entries.filter((e) => e.type === "compaction");
		expect(compactionEntries.length).toBe(1);

		const compaction = compactionEntries[0];
		expect(compaction.type).toBe("compaction");
		if (compaction.type === "compaction") {
			expect(compaction.summary.length).toBeGreaterThan(0);
			expect(typeof compaction.firstKeptEntryId).toBe("string");
			expect(compaction.tokensBefore).toBeGreaterThan(0);
		}
	}, 120000);

	it("should work with --no-session mode (in-memory only)", async () => {
		await createSession(); // harness uses in-memory session storage
		harness!.setResponses([
			fauxAssistantMessage("4"),
			fauxAssistantMessage("6"),
			fauxAssistantMessage("Generated turn prefix summary"),
			fauxAssistantMessage("Generated compaction summary"),
		]);

		// Send prompts
		await session.prompt("What is 2+2? Reply with just the number.");
		await session.agent.waitForIdle();

		await session.prompt("What is 3+3? Reply with just the number.");
		await session.agent.waitForIdle();

		// Compact should work even without file persistence
		const result = await session.compact();

		expect(result.summary).toBeDefined();
		expect(result.summary.length).toBeGreaterThan(0);

		// In-memory entries should have the compaction
		const entries = sessionManager.getEntries();
		const compactionEntries = entries.filter((e) => e.type === "compaction");
		expect(compactionEntries.length).toBe(1);
	}, 120000);

	it("should emit compaction events during manual compaction", async () => {
		await createSession();
		harness!.setResponses([fauxAssistantMessage("hello"), fauxAssistantMessage("Generated compaction summary")]);

		// Build some history
		await session.prompt("Say hello");
		await session.agent.waitForIdle();

		// Manually trigger compaction and check events
		await session.compact();

		const compactionEvents = events.filter((e) => e.type === "compaction_start" || e.type === "compaction_end");
		expect(compactionEvents).toHaveLength(2);
		expect(compactionEvents[0]).toEqual({ type: "compaction_start", reason: "manual" });
		expect(compactionEvents[1]).toMatchObject({
			type: "compaction_end",
			reason: "manual",
			aborted: false,
			willRetry: false,
		});

		// Regular events should have been emitted
		const messageEndEvents = events.filter((e) => e.type === "message_end");
		expect(messageEndEvents.length).toBeGreaterThan(0);
	}, 120000);
});
