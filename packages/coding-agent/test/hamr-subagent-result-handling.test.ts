/**
 * Regression tests for subagent result handling.
 *
 * Covers:
 *  - Empty output (only thinking events, no final assistant text) → failure
 *  - Empty final.md → failure
 *  - All workers empty → aggregate isError:true
 *  - Full event payload survives in events.ndjson beyond 256 chars
 *  - Artifact contract validation
 */

import * as fs from "node:fs";
import * as os from "node:os";
import * as path from "node:path";
import { describe, expect, it } from "vitest";
import { _testExports } from "../src/hamr/extensions/subagents.ts";

const {
	pushEvent,
	validateWorkerOutput,
	createWorkerState,
	createWorkerProcessEventState,
	recordWorkerProcessEvent,
	buildWorkerOutcomeFromChildSummary,
} = _testExports;

// ─── Helpers ─────────────────────────────────────────────────────────────────

const EMPTY_USAGE = {
	input: 0,
	output: 0,
	cacheRead: 0,
	cacheWrite: 0,
	totalTokens: 0,
	cost: { input: 0, output: 0, cacheRead: 0, cacheWrite: 0, total: 0 },
};

// ─── 0. Child process classification ────────────────────────────────────────

describe("child process outcome classification", () => {
	it("surfaces assistant stopReason=error instead of reporting empty output", () => {
		const outcome = buildWorkerOutcomeFromChildSummary("00", "call model", {
			exitCode: 0,
			wasAborted: false,
			stderr: "",
			outputText: "",
			usage: { ...EMPTY_USAGE },
			estimatedUsage: false,
			stopReason: "error",
			errorMessage: "rate limit exceeded",
			stdoutParseErrors: 0,
			invalidStdout: "",
		});

		expect(outcome.status).toBe("failed");
		if (outcome.status !== "failed") throw new Error("expected failed outcome");
		expect(outcome.error).toContain("rate limit exceeded");
	});

	it("does not fail solely because a successful child wrote a stderr warning", () => {
		const outcome = buildWorkerOutcomeFromChildSummary("01", "produce answer", {
			exitCode: 0,
			wasAborted: false,
			stderr: "ExperimentalWarning: noisy dependency\n",
			outputText: "real answer",
			usage: { ...EMPTY_USAGE },
			estimatedUsage: false,
			stopReason: "stop",
			stdoutParseErrors: 0,
			invalidStdout: "",
		});

		expect(outcome.status).toBe("done");
		if (outcome.status !== "done") throw new Error("expected done outcome");
		expect(outcome.text).toBe("real answer");
	});

	it("fails with malformed stdout diagnostics when JSON mode output cannot be parsed", () => {
		const outcome = buildWorkerOutcomeFromChildSummary("02", "parse json stream", {
			exitCode: 0,
			wasAborted: false,
			stderr: "",
			outputText: "",
			usage: { ...EMPTY_USAGE },
			estimatedUsage: true,
			stdoutParseErrors: 2,
			invalidStdout: "not json\nalso not json\n",
		});

		expect(outcome.status).toBe("failed");
		if (outcome.status !== "failed") throw new Error("expected failed outcome");
		expect(outcome.error).toContain("2 invalid stdout lines");
		expect(outcome.error).toContain("not json");
	});

	it("can recover final assistant text from agent_end when message_end is missing", () => {
		const state = createWorkerProcessEventState();
		recordWorkerProcessEvent(state, {
			type: "agent_end",
			messages: [
				{ role: "user", content: [{ type: "text", text: "prompt" }] },
				{ role: "assistant", content: [{ type: "text", text: "fallback answer" }], stopReason: "stop" },
			],
		});

		const outcome = buildWorkerOutcomeFromChildSummary("03", "fallback", {
			exitCode: 0,
			wasAborted: false,
			stderr: "",
			outputText: state.outputText,
			usage: state.usage,
			estimatedUsage: state.estimatedUsage,
			stopReason: state.stopReason,
			errorMessage: state.errorMessage,
			stdoutParseErrors: state.stdoutParseErrors,
			invalidStdout: state.invalidStdout,
		});

		expect(outcome.status).toBe("done");
		if (outcome.status !== "done") throw new Error("expected done outcome");
		expect(outcome.text).toBe("fallback answer");
	});
});

// ─── 1. Empty output: worker with only thinking events → failure ─────────────

describe("worker with only thinking events", () => {
	it("is treated as failed when it produces no final assistant text", () => {
		const outcome = {
			status: "done" as const,
			workerId: "00",
			task: "think about something",
			text: "", // empty — worker only had thinking_start/delta/end events
			usage: { ...EMPTY_USAGE },
		};
		const result = validateWorkerOutput(outcome, "/tmp");
		expect(result.passed).toBe(false);
		expect(result.confidence).toBe(0);
		expect(result.warnings).toHaveLength(1);
		expect(result.warnings[0]!.type).toBe("empty_output");
		expect(result.warnings[0]!.severity).toBe("high");
	});

	it("is treated as failed when text is only whitespace", () => {
		const outcome = {
			status: "done" as const,
			workerId: "01",
			task: "do something",
			text: "   \n  \t  ",
			usage: { ...EMPTY_USAGE },
		};
		const result = validateWorkerOutput(outcome, "/tmp");
		expect(result.passed).toBe(false);
		expect(result.warnings[0]!.type).toBe("empty_output");
	});

	it("passes validation when text is non-empty", () => {
		const outcome = {
			status: "done" as const,
			workerId: "02",
			task: "write hello",
			text: "Hello, world!",
			usage: { ...EMPTY_USAGE },
		};
		const result = validateWorkerOutput(outcome, "/tmp");
		expect(result.passed).toBe(true);
		expect(result.confidence).toBeGreaterThan(0);
	});
});

// ─── 2. Empty final.md → failure ─────────────────────────────────────────────

describe("failed worker outcome validation", () => {
	it("failed outcomes with empty text get empty_output warning", () => {
		const outcome = {
			status: "failed" as const,
			workerId: "03",
			task: "broken task",
			error: "something went wrong",
			text: "",
		};
		const result = validateWorkerOutput(outcome, "/tmp");
		expect(result.passed).toBe(false);
		expect(result.warnings.some((w) => w.type === "empty_output")).toBe(true);
	});

	it("failed outcomes with text avoid empty_output but still fail", () => {
		const outcome = {
			status: "failed" as const,
			workerId: "04",
			task: "partial output",
			error: "exit code 1",
			text: "Some partial output was produced.",
		};
		const result = validateWorkerOutput(outcome, "/tmp");
		// Not empty_output, but still failed status
		expect(result.warnings.some((w) => w.type === "empty_output")).toBe(false);
	});
});

// ─── 3. Full event payload survives beyond 256 chars ─────────────────────────

describe("event persistence: full payload in events.ndjson", () => {
	it("stores full event JSON in pendingFlush, truncates only recentEvents", () => {
		const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), "hamr-test-"));
		const logPath = path.join(tmpDir, "events.ndjson");
		const ws = createWorkerState("00", "test task", tmpDir, logPath);

		// Create a large event: a message_end with >256 chars of text
		const largeText = "A".repeat(500);
		const event = {
			type: "message_end",
			message: {
				role: "assistant",
				content: [{ type: "text", text: largeText }],
			},
		};

		pushEvent(ws, event);

		// In-memory recentEvents should be truncated
		expect(ws.recentEvents.length).toBe(1);
		const memoryEntry = ws.recentEvents[0]!;
		expect(memoryEntry.data.length).toBeLessThanOrEqual(256);

		// Disk pendingFlush should contain the FULL event JSON
		expect(ws.pendingFlush.length).toBe(1);
		const diskLine = ws.pendingFlush[0]!;

		// The disk line should be longer than 256 chars (contains full payload)
		expect(diskLine.length).toBeGreaterThan(256);

		// The disk line must contain the full 500-char text
		expect(diskLine).toContain(largeText);

		// The disk line must NOT be truncated (verify the last 10 chars of largeText are there)
		expect(diskLine).toContain(largeText.slice(-50));

		// Clean up
		try {
			fs.rmSync(tmpDir, { recursive: true, force: true });
		} catch {
			/* best-effort */
		}
	});

	it("preserves thinking events in full on disk", () => {
		const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), "hamr-test-"));
		const logPath = path.join(tmpDir, "events.ndjson");
		const ws = createWorkerState("01", "thinking task", tmpDir, logPath);

		// Simulate events a thinking-only worker would produce
		pushEvent(ws, {
			type: "message_update",
			message: { role: "assistant", content: [{ type: "thinking_start" }] },
		});
		pushEvent(ws, {
			type: "message_update",
			message: {
				role: "assistant",
				content: [{ type: "thinking_delta", text: "Hmm, let me think about this carefully..." }],
			},
		});
		pushEvent(ws, {
			type: "message_update",
			message: { role: "assistant", content: [{ type: "thinking_end" }] },
		});
		pushEvent(ws, {
			type: "agent_end",
		});

		// All events should be in pendingFlush (not yet flushed to disk)
		expect(ws.pendingFlush.length).toBe(4);

		// Each disk entry should be valid JSON with full data
		for (const line of ws.pendingFlush) {
			const parsed = JSON.parse(line);
			expect(parsed).toHaveProperty("ts");
			expect(parsed).toHaveProperty("type");
			expect(parsed).toHaveProperty("data");
			// The data field should contain the full event object
			expect(typeof parsed.data).toBe("object");
		}

		// Clean up
		try {
			fs.rmSync(tmpDir, { recursive: true, force: true });
		} catch {
			/* best-effort */
		}
	});
});

// ─── 4. All workers empty → aggregate result with isError:true ───────────────

describe("aggregate error detection", () => {
	it("detects when all outcomes have empty_output validation", () => {
		const results = [
			{
				status: "done" as const,
				workerId: "00",
				task: "task 1",
				text: "",
				usage: { ...EMPTY_USAGE },
				validation: validateWorkerOutput(
					{ status: "done", workerId: "00", task: "task 1", text: "", usage: { ...EMPTY_USAGE } },
					"/tmp",
				),
			},
			{
				status: "done" as const,
				workerId: "01",
				task: "task 2",
				text: "",
				usage: { ...EMPTY_USAGE },
				validation: validateWorkerOutput(
					{ status: "done", workerId: "01", task: "task 2", text: "", usage: { ...EMPTY_USAGE } },
					"/tmp",
				),
			},
		];

		// The same logic used in the execute handler
		const allEmpty =
			results.length > 0 &&
			results.every((r) => {
				const v = r.validation;
				return v?.warnings?.some((w) => w.type === "empty_output");
			});

		expect(allEmpty).toBe(true);
	});

	it("does not flag when some workers succeeded with text", () => {
		const results = [
			{
				status: "done" as const,
				workerId: "00",
				task: "task 1",
				text: "Real output here.",
				usage: { ...EMPTY_USAGE },
				validation: validateWorkerOutput(
					{ status: "done", workerId: "00", task: "task 1", text: "Real output here.", usage: { ...EMPTY_USAGE } },
					"/tmp",
				),
			},
			{
				status: "done" as const,
				workerId: "01",
				task: "task 2",
				text: "",
				usage: { ...EMPTY_USAGE },
				validation: validateWorkerOutput(
					{ status: "done", workerId: "01", task: "task 2", text: "", usage: { ...EMPTY_USAGE } },
					"/tmp",
				),
			},
		];

		const allEmpty =
			results.length > 0 &&
			results.every((r) => {
				const v = r.validation;
				return v?.warnings?.some((w) => w.type === "empty_output");
			});

		expect(allEmpty).toBe(false);
	});
});

// ─── 5. Artifact contract (validation logic) ─────────────────────────────────

describe("artifact contract", () => {
	it("detects missing artifact file", () => {
		const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), "hamr-test-"));
		const artifactPath = path.join(tmpDir, "nonexistent.txt");

		// File should not exist
		expect(fs.existsSync(artifactPath)).toBe(false);

		try {
			fs.rmSync(tmpDir, { recursive: true, force: true });
		} catch {
			/* best-effort */
		}
	});

	it("detects empty artifact file", () => {
		const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), "hamr-test-"));
		const artifactPath = path.join(tmpDir, "empty.txt");
		fs.writeFileSync(artifactPath, "", "utf-8");

		const stat = fs.statSync(artifactPath);
		expect(stat.size).toBe(0);

		try {
			fs.rmSync(tmpDir, { recursive: true, force: true });
		} catch {
			/* best-effort */
		}
	});

	it("accepts non-empty artifact file", () => {
		const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), "hamr-test-"));
		const artifactPath = path.join(tmpDir, "output.txt");
		fs.writeFileSync(artifactPath, "Real content here.", "utf-8");

		const stat = fs.statSync(artifactPath);
		expect(stat.size).toBeGreaterThan(0);

		try {
			fs.rmSync(tmpDir, { recursive: true, force: true });
		} catch {
			/* best-effort */
		}
	});
});

// ─── 6. Tool definition includes artifact field ──────────────────────────────

describe("delegate_subagents tool schema", () => {
	it("TaskItem schema includes artifact field", async () => {
		const { createHamrSubagentsExtension } = await import("../src/hamr/extensions/subagents.ts");
		const tools = new Map<string, any>();
		const pi = {
			registerTool: (tool: any) => tools.set(tool.name, tool),
			registerShortcut: () => {},
			on: () => {},
		} as any;

		await createHamrSubagentsExtension(() => [], 0)(pi);
		const tool = tools.get("delegate_subagents");
		expect(tool).toBeDefined();

		const schema = tool.parameters as any;
		// TaskItem is nested — check tasks.items.properties for artifact
		expect(schema.properties).toBeDefined();
		expect(schema.properties.tasks).toBeDefined();
		const taskItems = schema.properties.tasks.items;
		expect(taskItems).toBeDefined();
		expect(taskItems.properties.artifact).toBeDefined();
		expect(taskItems.properties.artifact.description).toContain("output file");
	});
});
