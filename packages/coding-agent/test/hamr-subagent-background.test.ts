/**
 * TDD: Background subagent execution.
 *
 * When HAMR_SUBAGENT_BACKGROUND=true, delegate_subagents should:
 *   1. Return immediately with { pending: true } — not block awaiting workers.
 *   2. Call pi.sendMessage() when workers complete with { triggerTurn: true }.
 */

import { EventEmitter } from "node:events";
import * as fs from "node:fs";
import * as os from "node:os";
import * as path from "node:path";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

// ─── Mock child_process.spawn ───────────────────────────────────────────────

const MockEventEmitter = EventEmitter;

vi.mock("node:child_process", () => {
	function createMockChildProcess() {
		const proc = new MockEventEmitter();
		const stdout = new MockEventEmitter();
		const stderr = new MockEventEmitter();
		return {
			pid: Math.floor(Math.random() * 10000) + 1000,
			stdout,
			stderr,
			on: proc.on.bind(proc),
			kill: vi.fn(),
			killed: false,
			_simulateSuccess() {
				stdout.emit(
					"data",
					Buffer.from(
						`${JSON.stringify({
							type: "message_end",
							message: {
								role: "assistant",
								content: [{ type: "text", text: "Worker completed successfully." }],
								usage: { input: 10, output: 5, totalTokens: 15 },
								model: "test-model",
								stopReason: "end",
							},
						})}\n`,
					),
				);
				proc.emit("close", 0, null);
			},
		};
	}

	return {
		spawn: vi.fn(() => {
			const proc = createMockChildProcess();
			setTimeout(() => (proc as any)._simulateSuccess(), 1);
			return proc;
		}),
	};
});

// ─── Imports after mock ─────────────────────────────────────────────────────
import type { ExtensionFactory, ToolDefinition } from "../src/core/extensions/types.ts";
import { createHamrSubagentsExtension } from "../src/hamr/extensions/subagents.ts";

type AnyTool = ToolDefinition<any, any, any>;

// ─── Helpers ─────────────────────────────────────────────────────────────────

let tools: Map<string, AnyTool>;
let sentMessages: Array<{ message: any; options: any }>;
let tmpDir: string;

function buildPi(): Parameters<ExtensionFactory>[0] {
	tools = new Map();
	sentMessages = [];
	return {
		registerTool: (tool: AnyTool) => tools.set(tool.name, tool),
		registerShortcut: () => {},
		registerCommand: () => {},
		registerFlag: () => {},
		on: () => {},
		registerMessageRenderer: () => {},
		registerRoleMessageRenderer: () => {},
		sendMessage: (message: any, options: any) => {
			sentMessages.push({ message, options });
		},
	} as unknown as Parameters<ExtensionFactory>[0];
}

function buildCtx(overrides: Partial<any> = {}): any {
	return {
		cwd: tmpDir,
		mode: "print",
		ui: { setWidget: () => {} },
		model: { provider: "test-provider", id: "test-model" },
		modelRegistry: { getApiKeyAndHeaders: async () => ({ ok: true }), isUsingOAuth: () => false },
		sessionManager: {
			appendSpawnPoint: () => "spawn-1",
			mergeHandoff: () => "merge-1",
			getSessionId: () => "test-session",
			getSessionDir: () => tmpDir,
		},
		...overrides,
	};
}

async function getTool(): Promise<AnyTool> {
	const pi = buildPi();
	await createHamrSubagentsExtension(() => [] as ExtensionFactory[], 0)(pi);
	const tool = tools.get("delegate_subagents");
	if (!tool) throw new Error("delegate_subagents tool not registered");
	return tool;
}

// ─── Lifecycle ───────────────────────────────────────────────────────────────

beforeEach(() => {
	tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), "hamr-subagent-bg-"));
});

afterEach(() => {
	try {
		fs.rmSync(tmpDir, { recursive: true, force: true });
	} catch {
		/* best-effort */
	}
	delete process.env.HAMR_SUBAGENT_BACKGROUND;
});

// ─── RED: Tests for background execution (will fail until implemented) ───────

describe("delegate_subagents background execution", () => {
	it("returns pending result immediately (default: background on)", async () => {
		const tool = await getTool();
		const ctx = buildCtx();

		const start = Date.now();
		const result = await tool.execute("id-1", { tasks: [{ task: "Do one thing" }] }, undefined, undefined, ctx);
		const elapsed = Date.now() - start;

		// Must return quickly — real workers would take much longer
		expect(elapsed).toBeLessThan(200);
		// Result detail must indicate background/pending
		expect(result.details).toHaveProperty("pending", true);
	}, 10000);

	it("calls pi.sendMessage with triggerTurn:true after workers complete (default: background on)", async () => {
		const pi = buildPi();
		await createHamrSubagentsExtension(() => [] as ExtensionFactory[], 0)(pi);
		const tool = tools.get("delegate_subagents")!;
		const ctx = buildCtx();

		await tool.execute("id-2", { tasks: [{ task: "Do one thing" }] }, undefined, undefined, ctx);

		// Workers haven't completed yet (returned immediately)
		expect(sentMessages).toHaveLength(0);

		// Wait for async worker completion (mock completes in 1ms)
		await new Promise((r) => setTimeout(r, 50));

		// sendMessage should have been called
		expect(sentMessages.length).toBeGreaterThanOrEqual(1);
		const sent = sentMessages[0]!;
		expect(sent.options).toHaveProperty("triggerTurn", true);
		expect(sent.message.customType).toBe("subagent_handoff");
	}, 10000);

	it("does NOT use pending flag or sendMessage in blocking path (HAMR_SUBAGENT_BACKGROUND=false)", async () => {
		process.env.HAMR_SUBAGENT_BACKGROUND = "false";
		const pi = buildPi();
		await createHamrSubagentsExtension(() => [] as ExtensionFactory[], 0)(pi);
		const tool = tools.get("delegate_subagents")!;
		const ctx = buildCtx();

		const result = await tool.execute("id-3", { tasks: [{ task: "Do one thing" }] }, undefined, undefined, ctx);

		// Blocking path: result should NOT have pending flag
		expect(result.details).not.toHaveProperty("pending", true);
		// sendMessage should NOT be called (blocking path returns results directly)
		expect(sentMessages).toHaveLength(0);
	}, 10000);

	it("background mode works for chain mode (default: background on)", async () => {
		const pi = buildPi();
		await createHamrSubagentsExtension(() => [] as ExtensionFactory[], 0)(pi);
		const tool = tools.get("delegate_subagents")!;
		const ctx = buildCtx();

		const start = Date.now();
		const result = await tool.execute(
			"id-4",
			{ chain: [{ task: "Step 1" }, { task: "Step 2" }] },
			undefined,
			undefined,
			ctx,
		);
		const elapsed = Date.now() - start;

		expect(elapsed).toBeLessThan(200);
		expect(result.details).toHaveProperty("pending", true);
	}, 10000);

	it("background mode works for stages mode (default: background on)", async () => {
		const pi = buildPi();
		await createHamrSubagentsExtension(() => [] as ExtensionFactory[], 0)(pi);
		const tool = tools.get("delegate_subagents")!;
		const ctx = buildCtx();

		const start = Date.now();
		const result = await tool.execute(
			"id-5",
			{
				stages: [
					{ mode: "parallel" as const, tasks: [{ task: "Stage 1-A" }, { task: "Stage 1-B" }] },
					{ mode: "chain" as const, tasks: [{ task: "Stage 2-1" }] },
				],
			},
			undefined,
			undefined,
			ctx,
		);
		const elapsed = Date.now() - start;

		expect(elapsed).toBeLessThan(200);
		expect(result.details).toHaveProperty("pending", true);
	}, 10000);
});
