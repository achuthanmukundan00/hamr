/**
 * Tests for compaction extension events (before_compact / compact).
 */

import { type AssistantMessage, createAssistantMessageEventStream, fauxAssistantMessage } from "@hamr/ai";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import type { AgentSession } from "../src/core/agent-session.ts";
import type { SessionBeforeCompactEvent, SessionCompactEvent, SessionEvent } from "../src/core/extensions/index.ts";
import type { ExtensionFactory } from "../src/index.ts";
import { createHarness, type Harness } from "./suite/harness.ts";

describe("Compaction extensions", () => {
	let harness: Harness | undefined;
	let session: AgentSession;
	let capturedEvents: SessionEvent[];

	beforeEach(() => {
		capturedEvents = [];
	});

	afterEach(() => {
		harness?.cleanup();
		harness = undefined;
	});

	function createUsage(totalTokens: number) {
		return {
			input: totalTokens,
			output: 0,
			cacheRead: 0,
			cacheWrite: 0,
			totalTokens,
			cost: { input: 0, output: 0, cacheRead: 0, cacheWrite: 0, total: 0 },
		};
	}

	function createAssistant(harness: Harness): AssistantMessage {
		const model = harness.getModel();
		return {
			...fauxAssistantMessage("assistant response"),
			api: model.api,
			provider: model.provider,
			model: model.id,
			usage: createUsage(100),
			stopReason: "stop",
			timestamp: Date.now() - 500,
		};
	}

	function seedCompactableSession(harness: Harness): void {
		harness.sessionManager.appendMessage({
			role: "user",
			content: [{ type: "text", text: "message to compact" }],
			timestamp: Date.now() - 1000,
		});
		harness.sessionManager.appendMessage(createAssistant(harness));
		harness.session.agent.state.messages = harness.sessionManager.buildSessionContext().messages;
	}

	function useSummaryStreamFn(harness: Harness, summary: string): void {
		harness.session.agent.streamFn = (model) => {
			const stream = createAssistantMessageEventStream();
			queueMicrotask(() => {
				stream.push({
					type: "done",
					reason: "stop",
					message: {
						...fauxAssistantMessage(summary),
						api: model.api,
						provider: model.provider,
						model: model.id,
						usage: createUsage(10),
					},
				});
			});
			return stream;
		};
	}

	function createExtension(
		onBeforeCompact?: (event: SessionBeforeCompactEvent) => { cancel?: boolean; compaction?: any } | undefined,
		onCompact?: (event: SessionCompactEvent) => void,
	): ExtensionFactory {
		return (pi) => {
			pi.on("session_before_compact", async (event) => {
				capturedEvents.push(event);
				if (onBeforeCompact) {
					return onBeforeCompact(event);
				}
				return undefined;
			});

			pi.on("session_compact", async (event) => {
				capturedEvents.push(event);
				if (onCompact) {
					onCompact(event);
				}
				return undefined;
			});
		};
	}

	async function createSession(extensionFactories: ExtensionFactory[]) {
		harness = await createHarness({ extensionFactories });
		session = harness.session;
		seedCompactableSession(harness);
		useSummaryStreamFn(harness, "Generated test summary");
		return session;
	}

	it("should emit before_compact and compact events", async () => {
		const extension = createExtension();
		await createSession([extension]);

		await session.compact();

		const beforeCompactEvents = capturedEvents.filter(
			(e): e is SessionBeforeCompactEvent => e.type === "session_before_compact",
		);
		const compactEvents = capturedEvents.filter((e): e is SessionCompactEvent => e.type === "session_compact");

		expect(beforeCompactEvents.length).toBe(1);
		expect(compactEvents.length).toBe(1);

		const beforeEvent = beforeCompactEvents[0];
		expect(beforeEvent.preparation).toBeDefined();
		expect(beforeEvent.preparation.messagesToSummarize).toBeDefined();
		expect(beforeEvent.preparation.turnPrefixMessages).toBeDefined();
		expect(beforeEvent.preparation.tokensBefore).toBeGreaterThanOrEqual(0);
		expect(typeof beforeEvent.preparation.isSplitTurn).toBe("boolean");
		expect(beforeEvent.branchEntries).toBeDefined();
		// sessionManager, modelRegistry, and model are now on ctx, not event

		const afterEvent = compactEvents[0];
		expect(afterEvent.compactionEntry).toBeDefined();
		expect(afterEvent.compactionEntry.summary.length).toBeGreaterThan(0);
		expect(afterEvent.compactionEntry.tokensBefore).toBeGreaterThanOrEqual(0);
		expect(afterEvent.fromExtension).toBe(false);
	}, 120000);

	it("should allow extensions to cancel compaction", async () => {
		const extension = createExtension(() => ({ cancel: true }));
		await createSession([extension]);

		await expect(session.compact()).rejects.toThrow("Compaction cancelled");

		const compactEvents = capturedEvents.filter((e) => e.type === "session_compact");
		expect(compactEvents.length).toBe(0);
	}, 120000);

	it("should allow extensions to provide custom compaction", async () => {
		const customSummary = "Custom summary from extension";

		const extension = createExtension((event) => {
			if (event.type === "session_before_compact") {
				return {
					compaction: {
						summary: customSummary,
						firstKeptEntryId: event.preparation.firstKeptEntryId,
						tokensBefore: event.preparation.tokensBefore,
					},
				};
			}
			return undefined;
		});
		await createSession([extension]);

		const result = await session.compact();

		expect(result.summary).toBe(customSummary);

		const compactEvents = capturedEvents.filter((e) => e.type === "session_compact");
		expect(compactEvents.length).toBe(1);

		const afterEvent = compactEvents[0];
		if (afterEvent.type === "session_compact") {
			expect(afterEvent.compactionEntry.summary).toBe(customSummary);
			expect(afterEvent.fromExtension).toBe(true);
		}
	}, 120000);

	it("should include entries in compact event after compaction is saved", async () => {
		const extension = createExtension();
		await createSession([extension]);

		await session.compact();

		const compactEvents = capturedEvents.filter((e) => e.type === "session_compact");
		expect(compactEvents.length).toBe(1);

		const afterEvent = compactEvents[0];
		if (afterEvent.type === "session_compact") {
			// sessionManager is now on ctx, use session.sessionManager directly
			const entries = session.sessionManager.getEntries();
			const hasCompactionEntry = entries.some((e: { type: string }) => e.type === "compaction");
			expect(hasCompactionEntry).toBe(true);
		}
	}, 120000);

	it("should continue with default compaction if extension throws error", async () => {
		const throwingExtension: ExtensionFactory = (pi) => {
			pi.on("session_before_compact", async (event) => {
				capturedEvents.push(event);
				throw new Error("Extension intentionally throws");
			});

			pi.on("session_compact", async (event) => {
				capturedEvents.push(event);
				return undefined;
			});
		};

		await createSession([throwingExtension]);

		const result = await session.compact();

		expect(result.summary).toBeDefined();
		expect(result.summary.length).toBeGreaterThan(0);

		const compactEvents = capturedEvents.filter((e): e is SessionCompactEvent => e.type === "session_compact");
		expect(compactEvents.length).toBe(1);
		expect(compactEvents[0].fromExtension).toBe(false);
	}, 120000);

	it("should call multiple extensions in order", async () => {
		const callOrder: string[] = [];

		const extension1: ExtensionFactory = (pi) => {
			pi.on("session_before_compact", async () => {
				callOrder.push("extension1-before");
				return undefined;
			});

			pi.on("session_compact", async () => {
				callOrder.push("extension1-after");
				return undefined;
			});
		};

		const extension2: ExtensionFactory = (pi) => {
			pi.on("session_before_compact", async () => {
				callOrder.push("extension2-before");
				return undefined;
			});

			pi.on("session_compact", async () => {
				callOrder.push("extension2-after");
				return undefined;
			});
		};

		await createSession([extension1, extension2]);

		await session.compact();

		expect(callOrder).toEqual(["extension1-before", "extension2-before", "extension1-after", "extension2-after"]);
	}, 120000);

	it("should pass correct data in before_compact event", async () => {
		let capturedBeforeEvent: SessionBeforeCompactEvent | null = null;

		const extension = createExtension((event) => {
			capturedBeforeEvent = event;
			return undefined;
		});
		await createSession([extension]);

		await session.compact();

		expect(capturedBeforeEvent).not.toBeNull();
		const event = capturedBeforeEvent!;
		expect(typeof event.preparation.isSplitTurn).toBe("boolean");
		expect(event.preparation.firstKeptEntryId).toBeDefined();

		expect(Array.isArray(event.preparation.messagesToSummarize)).toBe(true);
		expect(Array.isArray(event.preparation.turnPrefixMessages)).toBe(true);

		expect(typeof event.preparation.tokensBefore).toBe("number");

		expect(Array.isArray(event.branchEntries)).toBe(true);

		// sessionManager, modelRegistry, and model are now on ctx, not event
		// Verify they're accessible via session
		expect(typeof session.sessionManager.getEntries).toBe("function");
		expect(typeof session.modelRegistry.getApiKeyAndHeaders).toBe("function");

		const entries = session.sessionManager.getEntries();
		expect(Array.isArray(entries)).toBe(true);
		expect(entries.length).toBeGreaterThan(0);
	}, 120000);

	it("should use extension compaction even with different values", async () => {
		const customSummary = "Custom summary with modified values";

		const extension = createExtension((event) => {
			if (event.type === "session_before_compact") {
				return {
					compaction: {
						summary: customSummary,
						firstKeptEntryId: event.preparation.firstKeptEntryId,
						tokensBefore: 999,
					},
				};
			}
			return undefined;
		});
		await createSession([extension]);

		const result = await session.compact();

		expect(result.summary).toBe(customSummary);
		expect(result.tokensBefore).toBe(999);
	}, 120000);
});
