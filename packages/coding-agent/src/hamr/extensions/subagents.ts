/**
 * Subagents extension: the `delegate_subagents` tool for parallel/chain/stages
 * execution with bounded concurrency, live observability, and memory-safe state.
 *
 * Workers are spawned as isolated child `hamr` processes (`hamr --mode json -p`)
 * and the parent parses JSONL events for live updates. Full logs are persisted
 * to disk; only bounded recent events and output tails are kept in memory.
 *
 * Modes:
 *   - subtasks (serial, backward-compatible legacy)
 *   - tasks (parallel batch with bounded concurrency)
 *   - chain (serial with {previous} placeholder)
 *   - stages (serial stages, each parallel or chain internally)
 */

import { spawn } from "node:child_process";
import * as fs from "node:fs";
import * as path from "node:path";
import type { Usage } from "@hamr/ai";
import { type Component, Container, Spacer, Text, type TUI } from "@hamr/tui";
import { Type } from "typebox";
import type { ExtensionContext, ExtensionFactory } from "../../core/extensions/types.ts";
import { defineTool } from "../../core/extensions/types.ts";
import type { Theme } from "../../modes/interactive/theme/theme.ts";

// ─── Environment configuration ───────────────────────────────────────────────

const ENV_MAX_TASKS = Number.parseInt(process.env.HAMR_SUBAGENT_MAX_TASKS ?? "64", 10) || 64;
const ENV_HARD_MAX_TASKS = Number.parseInt(process.env.HAMR_SUBAGENT_HARD_MAX_TASKS ?? "256", 10) || 256;
const ENV_MAX_CONCURRENCY = Number.parseInt(process.env.HAMR_SUBAGENT_MAX_CONCURRENCY ?? "4", 10) || 4;
const ENV_MAX_LOCAL_CONCURRENCY = Number.parseInt(process.env.HAMR_SUBAGENT_MAX_LOCAL_CONCURRENCY ?? "1", 10) || 1;

const OUTPUT_TAIL_BYTES = Number.parseInt(process.env.HAMR_SUBAGENT_OUTPUT_TAIL_BYTES ?? "32768", 10) || 32768;
const EVENTS_IN_MEMORY = Number.parseInt(process.env.HAMR_SUBAGENT_EVENTS_IN_MEMORY ?? "40", 10) || 40;
const LOG_DIR_BASE = process.env.HAMR_SUBAGENT_LOG_DIR ?? ".hamr/subagents";
/** Max completed runs to retain in memory for the status widget. */
const MAX_ACTIVE_RUNS = 50;

/** Marks the subagents factory so a parent can re-create it at depth + 1 for workers. */
export const HAMR_SUBAGENTS_FACTORY = Symbol.for("hamr.subagents.factory");

/** Recursion bound. Root = 0; at this depth the worker gets no delegate tool. */
const MAX_DEPTH = 3;

const EMPTY_USAGE: Usage = {
	input: 0,
	output: 0,
	cacheRead: 0,
	cacheWrite: 0,
	totalTokens: 0,
	cost: { input: 0, output: 0, cacheRead: 0, cacheWrite: 0, total: 0 },
};

// ─── Bounded memory state ────────────────────────────────────────────────────

interface ActivityEvent {
	ts: number;
	type: string;
	data: string; // compressed summary
}

interface WorkerState {
	workerId: string;
	taskPreview: string;
	cwd: string;
	status: "queued" | "running" | "done" | "failed" | "aborted";
	pid?: number;
	model?: string;
	startedAt?: number;
	endedAt?: number;
	usage: Usage;
	estimatedUsage?: boolean;
	stopReason?: string;
	errorMessage?: string;
	lastActivity?: string;
	lastTool?: string;
	recentEvents: ActivityEvent[]; // ring buffer, max EVENTS_IN_MEMORY entries
	outputTail: string; // capped to OUTPUT_TAIL_BYTES bytes
	finalOutput?: string; // capped
	logPath: string;
	resultPath?: string;
}

interface RunState {
	runId: string;
	mode: "single" | "parallel" | "chain" | "stages";
	total: number;
	queued: number;
	running: number;
	done: number;
	failed: number;
	aborted: number;
	startedAt: number;
	endedAt?: number;
	usage: Usage;
	logDir: string;
	workers: Map<string, WorkerState>;
}

// ─── Global state ────────────────────────────────────────────────────────────

const activeRuns = new Map<string, RunState>();
let runCounter = 0;

// ─── Helpers ─────────────────────────────────────────────────────────────────

function nextRunId(): string {
	return `run-${Date.now().toString(36)}-${(++runCounter).toString(36)}`;
}

function formatTokens(tokens: number): string {
	if (tokens >= 1_000_000) return `${(tokens / 1_000_000).toFixed(1)}M`;
	if (tokens >= 10_000) return `${Math.round(tokens / 1000)}K`;
	if (tokens >= 1000) return `${(tokens / 1000).toFixed(1)}K`;
	return `${tokens}`;
}

function clamp(value: number, min: number, max: number): number {
	return Math.max(min, Math.min(max, value));
}

function padWorkerId(idx: number, total: number): string {
	const width = String(total).length;
	return String(idx).padStart(width, "0");
}

// ─── UI: Live status widget ──────────────────────────────────────────────────

interface AggregatedStats {
	total: number;
	queued: number;
	running: number;
	done: number;
	failed: number;
	aborted: number;
	totalTokens: number;
}

function aggregateAllRuns(): AggregatedStats {
	const stats: AggregatedStats = { total: 0, queued: 0, running: 0, done: 0, failed: 0, aborted: 0, totalTokens: 0 };
	for (const run of activeRuns.values()) {
		stats.total += run.total;
		stats.queued += run.queued;
		stats.running += run.running;
		stats.done += run.done;
		stats.failed += run.failed;
		stats.aborted += run.aborted;
		stats.totalTokens += run.usage.totalTokens ?? 0;
	}
	return stats;
}

const RUNNING_FRAMES = ["◐", "◓", "◑", "◒"];
function animationFrame(): string {
	return RUNNING_FRAMES[Math.floor(Date.now() / 180) % RUNNING_FRAMES.length]!;
}

function renderStatusLine(): string | undefined {
	const stats = aggregateAllRuns();
	if (stats.total === 0) return undefined;
	const parts: string[] = [];
	if (stats.running > 0) parts.push(`${animationFrame()} ${stats.running} running`);
	if (stats.queued > 0) parts.push(`${stats.queued} queued`);
	if (stats.done > 0) parts.push(`✓ ${stats.done} done`);
	if (stats.failed > 0) parts.push(`✕ ${stats.failed} failed`);
	if (stats.totalTokens > 0) parts.push(`↓${formatTokens(stats.totalTokens)} tok`);
	return `subagents ${parts.join(" · ")}`;
}

function evictOldRuns(): void {
	if (activeRuns.size <= MAX_ACTIVE_RUNS) return;
	const runs = [...activeRuns.entries()]
		.filter(([, r]) => r.endedAt != null)
		.sort((a, b) => (a[1].endedAt ?? Infinity) - (b[1].endedAt ?? Infinity));
	for (let i = 0; i < runs.length - MAX_ACTIVE_RUNS; i++) {
		activeRuns.delete(runs[i]![0]);
	}
}

class AgentStatusWidget implements Component {
	private interval: ReturnType<typeof setInterval> | undefined;
	private tui: TUI;
	private theme: Theme;
	private lastLine = "";

	constructor(tui: TUI, theme: Theme) {
		this.tui = tui;
		this.theme = theme;
		this.interval = setInterval(() => {
			const line = renderStatusLine() ?? "";
			if (line !== this.lastLine) {
				this.lastLine = line;
				this.tui.requestRender();
			}
		}, 180);
	}

	render(): string[] {
		const line = renderStatusLine() ?? this.lastLine;
		return line ? [` ${this.theme.fg("muted", line)}`] : [];
	}

	invalidate(): void {}

	dispose(): void {
		if (this.interval) {
			clearInterval(this.interval);
			this.interval = undefined;
		}
	}
}

function updateStatusWidget(ctx: ExtensionContext): void {
	if (ctx.mode !== "tui") return;
	const widget = renderStatusLine() ? (tui: TUI, theme: Theme) => new AgentStatusWidget(tui, theme) : undefined;
	ctx.ui.setWidget("hamr.subagents.status", widget, { placement: "aboveEditor" });
}

// ─── Disk persistence ────────────────────────────────────────────────────────

function ensureLogDir(runId: string, cwd: string): string {
	const base = path.resolve(cwd, LOG_DIR_BASE, "runs", runId);
	fs.mkdirSync(path.join(base, "workers"), { recursive: true });
	return base;
}

function appendNDJSON(filePath: string, event: Record<string, unknown>): void {
	try {
		fs.appendFileSync(filePath, `${JSON.stringify(event)}\n`, "utf-8");
	} catch {
		// best-effort
	}
}

// ─── Worker execution (child hamr process) ───────────────────────────────────

interface TaskResult {
	workerId: string;
	task: string;
	text: string;
	error?: string;
	usage?: Usage;
	model?: string;
	estimatedUsage?: boolean;
	stopReason?: string;
}

function getPiInvocation(args: string[]): { command: string; args: string[] } {
	// Use the current Node.js executable with the CLI script
	const currentScript = process.argv[1];
	const scriptExt = path.extname(currentScript).toLowerCase();

	// If running via bun or the script is a .ts file, use the hamr binary
	const isBun = process.execPath.includes("bun") || process.execPath.includes("$bunfs");
	if (isBun) {
		// Try the locally built cli.js first, fall back to global hamr
		const repoCli = path.resolve(import.meta.dirname ?? __dirname, "../../../dist/cli.js");
		if (fs.existsSync(repoCli)) {
			return { command: process.execPath, args: ["run", repoCli, ...args] };
		}
		return { command: "hamr", args };
	}

	// Node.js: use the current script if it exists and is a .js file
	if (currentScript && !currentScript.startsWith("/$bunfs") && scriptExt === ".js") {
		if (fs.existsSync(currentScript)) {
			return { command: process.execPath, args: [currentScript, ...args] };
		}
	}

	return { command: "hamr", args };
}

async function runWorkerChildProcess(
	workerId: string,
	task: string,
	cwd: string,
	signal: AbortSignal | undefined,
	onEvent: (event: Record<string, unknown>) => void,
): Promise<TaskResult> {
	const args: string[] = ["--mode", "json", "-p", "--no-session", task];

	let wasAborted = false;
	let stderr = "";

	const result: TaskResult = {
		workerId,
		task,
		text: "",
		usage: { ...EMPTY_USAGE },
	};

	const exitCode = await new Promise<number>((resolve) => {
		const invocation = getPiInvocation(args);
		const proc = spawn(invocation.command, invocation.args, {
			cwd,
			shell: false,
			stdio: ["ignore", "pipe", "pipe"],
			env: { ...process.env },
		});

		let buffer = "";

		const processLine = (line: string) => {
			if (!line.trim()) return;
			let event: Record<string, unknown>;
			try {
				event = JSON.parse(line);
			} catch {
				return;
			}
			onEvent(event);

			if (event.type === "message_end" && event.message) {
				const msg = event.message as Record<string, unknown>;
				if (msg.role === "assistant") {
					const usage = msg.usage as Usage | undefined;
					if (usage) {
						result.usage = { ...EMPTY_USAGE, ...usage };
						result.estimatedUsage = false;
					}
					if (msg.model && typeof msg.model === "string") result.model = msg.model;
					if (msg.stopReason && typeof msg.stopReason === "string") result.stopReason = msg.stopReason;

					// Extract text
					const content = (msg as { content?: Array<{ type: string; text?: string }> }).content;
					if (content) {
						for (const part of content) {
							if (part.type === "text" && part.text) result.text += part.text;
						}
					}
				}
			}

			if (event.type === "tool_result_end" && event.message) {
				onEvent({ ...event, _recorded: true });
			}
		};

		proc.stdout!.on("data", (data: Buffer) => {
			buffer += data.toString();
			const lines = buffer.split("\n");
			buffer = lines.pop() || "";
			for (const line of lines) processLine(line);
		});

		proc.stderr!.on("data", (data: Buffer) => {
			stderr += data.toString();
		});

		proc.on("close", (code) => {
			if (buffer.trim()) processLine(buffer);
			resolve(code ?? 0);
		});

		proc.on("error", () => resolve(1));

		if (signal) {
			const killProc = () => {
				wasAborted = true;
				proc.kill("SIGTERM");
				setTimeout(() => {
					if (!proc.killed) proc.kill("SIGKILL");
				}, 5000);
			};
			if (signal.aborted) killProc();
			else signal.addEventListener("abort", killProc, { once: true });
		}
	});

	if (wasAborted) {
		result.error = "aborted";
		result.text = result.text || `[aborted] ${task}`;
	} else if (exitCode !== 0 || stderr) {
		if (!result.error) result.error = stderr || `exit code ${exitCode}`;
	}

	return result;
}

// ─── Concurrency limiter ─────────────────────────────────────────────────────

async function mapWithConcurrencyLimit<TIn, TOut>(
	items: TIn[],
	concurrency: number,
	fn: (item: TIn, index: number) => Promise<TOut>,
	onProgress: (done: number) => void,
): Promise<TOut[]> {
	if (items.length === 0) return [];
	const limit = Math.max(1, Math.min(concurrency, items.length));
	const results: TOut[] = new Array(items.length);
	let nextIndex = 0;
	let completed = 0;

	const worker = async () => {
		while (true) {
			const idx = nextIndex++;
			if (idx >= items.length) return;
			results[idx] = await fn(items[idx], idx);
			completed++;
			onProgress(completed);
		}
	};

	const workers = Array.from({ length: limit }, () => worker());
	await Promise.all(workers);
	return results;
}

// ─── Worker state management ─────────────────────────────────────────────────

function createWorkerState(workerId: string, task: string, cwd: string, logPath: string): WorkerState {
	return {
		workerId,
		taskPreview: task.length > 80 ? `${task.slice(0, 80)}…` : task,
		cwd,
		status: "queued",
		usage: { ...EMPTY_USAGE },
		recentEvents: [],
		outputTail: "",
		logPath,
	};
}

function pushEvent(ws: WorkerState, event: Record<string, unknown>): void {
	const entry: ActivityEvent = {
		ts: Date.now(),
		type: (event.type as string) ?? "unknown",
		data: JSON.stringify(event).slice(0, 256),
	};
	ws.recentEvents.push(entry);
	if (ws.recentEvents.length > EVENTS_IN_MEMORY) {
		ws.recentEvents.splice(0, ws.recentEvents.length - EVENTS_IN_MEMORY);
	}

	const type = event.type as string;
	ws.lastActivity = type;

	if (type === "tool_execution_start" || type === "tool_execution_end") {
		ws.lastTool = (event as { toolName?: string }).toolName ?? type;
	}

	// Update output tail from streamed text
	if (type === "message_update" || type === "message_end") {
		const msg = event.message as { content?: Array<{ type: string; text?: string }> } | undefined;
		if (msg?.content) {
			let text = "";
			for (const part of msg.content) {
				if (part.type === "text" && part.text) text += part.text;
			}
			if (text) {
				ws.outputTail = (ws.outputTail + text).slice(-OUTPUT_TAIL_BYTES);
			}
		}
	}

	appendNDJSON(ws.logPath, event);
}

function updateRunCounts(run: RunState): void {
	let queued = 0;
	let running = 0;
	let done = 0;
	let failed = 0;
	let aborted = 0;
	let totalTokens = 0;

	for (const ws of run.workers.values()) {
		switch (ws.status) {
			case "queued":
				queued++;
				break;
			case "running":
				running++;
				break;
			case "done":
				done++;
				totalTokens += ws.usage.totalTokens ?? 0;
				break;
			case "failed":
				failed++;
				totalTokens += ws.usage.totalTokens ?? 0;
				break;
			case "aborted":
				aborted++;
				break;
		}
	}

	run.queued = queued;
	run.running = running;
	run.done = done;
	run.failed = failed;
	run.aborted = aborted;
	run.usage = { ...run.usage, totalTokens };
}

// ─── Core execution ──────────────────────────────────────────────────────────

async function executeSingleWorker(
	run: RunState,
	workerId: string,
	task: string,
	cwd: string,
	signal: AbortSignal | undefined,
	_onUpdate: ((update: { text: string; details: Record<string, unknown> }) => void) | undefined,
	ctx: ExtensionContext,
): Promise<TaskResult> {
	const logPath = path.join(run.logDir, "workers", `${workerId}.events.ndjson`);
	const resultPath = path.join(run.logDir, "workers", `${workerId}.final.md`);

	let ws = run.workers.get(workerId) ?? createWorkerState(workerId, task, cwd, logPath);
	ws.status = "running";
	ws.startedAt = Date.now();
	run.workers.set(workerId, ws);
	updateRunCounts(run);
	updateStatusWidget(ctx);

	try {
		const result = await runWorkerChildProcess(workerId, task, cwd, signal, (event) => {
			ws = run.workers.get(workerId) ?? ws;
			pushEvent(ws, event);
		});

		ws = run.workers.get(workerId) ?? ws;
		ws.endedAt = Date.now();
		ws.model = result.model ?? ws.model;
		ws.stopReason = result.stopReason;

		if (result.error === "aborted") {
			ws.status = "aborted";
			run.workers.set(workerId, ws);
			updateRunCounts(run);
			updateStatusWidget(ctx);
			return result;
		}

		if (result.error) {
			ws.status = "failed";
			ws.errorMessage = result.error;
			ws.finalOutput = result.text.slice(0, OUTPUT_TAIL_BYTES);
			ws.resultPath = resultPath;
			try {
				fs.writeFileSync(resultPath, result.text.slice(0, OUTPUT_TAIL_BYTES), "utf-8");
			} catch {
				/* best-effort */
			}
			run.workers.set(workerId, ws);
			updateRunCounts(run);
			updateStatusWidget(ctx);
			return result;
		}

		// Success
		ws.status = "done";
		if (result.usage) {
			ws.usage = result.usage;
			ws.estimatedUsage = false;
		} else {
			// Estimate tokens from output length
			ws.usage = {
				...ws.usage,
				output: Math.ceil(result.text.length / 3),
				totalTokens: (ws.usage.input ?? 0) + Math.ceil(result.text.length / 3),
			};
			ws.estimatedUsage = true;
		}
		ws.finalOutput = result.text.slice(0, OUTPUT_TAIL_BYTES);
		ws.resultPath = resultPath;
		try {
			fs.writeFileSync(resultPath, result.text.slice(0, OUTPUT_TAIL_BYTES), "utf-8");
		} catch {
			/* best-effort */
		}

		run.workers.set(workerId, ws);
		updateRunCounts(run);
		updateStatusWidget(ctx);

		return { ...result, usage: ws.usage, model: ws.model };
	} catch (error) {
		ws = run.workers.get(workerId) ?? ws;
		ws.status = "failed";
		ws.errorMessage = error instanceof Error ? error.message : String(error);
		ws.endedAt = Date.now();
		run.workers.set(workerId, ws);
		updateRunCounts(run);
		updateStatusWidget(ctx);
		return { workerId, task, text: "", error: ws.errorMessage };
	}
}

// ─── Mode: Tasks (parallel batch) ────────────────────────────────────────────

async function executeTasks(
	run: RunState,
	taskItems: Array<{ task: string; cwd?: string }>,
	concurrency: number,
	failFast: boolean,
	signal: AbortSignal | undefined,
	onUpdate: ((update: { text: string; details: Record<string, unknown> }) => void) | undefined,
	ctx: ExtensionContext,
): Promise<TaskResult[]> {
	// Initialize all workers as queued
	for (let i = 0; i < taskItems.length; i++) {
		const item = taskItems[i]!;
		const workerId = padWorkerId(i, taskItems.length);
		const ws = createWorkerState(
			workerId,
			item.task,
			item.cwd ?? ctx.cwd,
			path.join(run.logDir, "workers", `${workerId}.events.ndjson`),
		);
		run.workers.set(workerId, ws);
	}
	run.total = taskItems.length;
	updateRunCounts(run);
	updateStatusWidget(ctx);

	const results: TaskResult[] = await mapWithConcurrencyLimit(
		taskItems,
		concurrency,
		async (item, idx) => {
			if (signal?.aborted) {
				return { workerId: padWorkerId(idx, taskItems.length), task: item.task, text: "", error: "aborted" };
			}
			return executeSingleWorker(
				run,
				padWorkerId(idx, taskItems.length),
				item.task,
				item.cwd ?? ctx.cwd,
				signal,
				onUpdate,
				ctx,
			);
		},
		(done) => {
			if (onUpdate) {
				onUpdate({
					text: `${done}/${taskItems.length} tasks complete`,
					details: { mode: "tasks", runId: run.runId, done, total: taskItems.length },
				});
			}
		},
	);

	// If failFast, check for failures
	if (failFast) {
		const firstFailure = results.find((r) => r.error && r.error !== "aborted");
		if (firstFailure) {
			// Mark remaining queued/running workers as aborted
			for (const ws of run.workers.values()) {
				if (ws.status === "queued" || ws.status === "running") {
					ws.status = "aborted";
					ws.endedAt = Date.now();
				}
			}
			updateRunCounts(run);
			updateStatusWidget(ctx);
		}
	}

	run.endedAt = Date.now();
	updateRunCounts(run);
	updateStatusWidget(ctx);
	return results;
}

// ─── Mode: Chain (serial) ────────────────────────────────────────────────────

async function executeChain(
	run: RunState,
	chainItems: Array<{ task: string; cwd?: string }>,
	failFast: boolean,
	signal: AbortSignal | undefined,
	onUpdate: ((update: { text: string; details: Record<string, unknown> }) => void) | undefined,
	ctx: ExtensionContext,
): Promise<TaskResult[]> {
	// Initialize all workers as queued
	for (let i = 0; i < chainItems.length; i++) {
		const item = chainItems[i]!;
		const workerId = padWorkerId(i, chainItems.length);
		const ws = createWorkerState(
			workerId,
			item.task,
			item.cwd ?? ctx.cwd,
			path.join(run.logDir, "workers", `${workerId}.events.ndjson`),
		);
		run.workers.set(workerId, ws);
	}
	run.total = chainItems.length;
	updateRunCounts(run);
	updateStatusWidget(ctx);

	const results: TaskResult[] = [];
	let previousOutput = "";

	for (let i = 0; i < chainItems.length; i++) {
		if (signal?.aborted) {
			const ws = run.workers.get(padWorkerId(i, chainItems.length));
			if (ws) {
				ws.status = "aborted";
				ws.endedAt = Date.now();
			}
			// Mark remaining as aborted
			for (let j = i + 1; j < chainItems.length; j++) {
				const w = run.workers.get(padWorkerId(j, chainItems.length));
				if (w) {
					w.status = "aborted";
					w.endedAt = Date.now();
				}
			}
			updateRunCounts(run);
			updateStatusWidget(ctx);
			results.push({
				workerId: padWorkerId(i, chainItems.length),
				task: chainItems[i]!.task,
				text: "",
				error: "aborted",
			});
			break;
		}

		const item = chainItems[i]!;
		const workerId = padWorkerId(i, chainItems.length);
		const taskWithContext = item.task.replace(/\{previous\}/g, previousOutput);

		const result = await executeSingleWorker(
			run,
			workerId,
			taskWithContext,
			item.cwd ?? ctx.cwd,
			signal,
			onUpdate,
			ctx,
		);
		results.push(result);

		if (onUpdate) {
			onUpdate({
				text: `${i + 1}/${chainItems.length} chain steps complete`,
				details: { mode: "chain", runId: run.runId, step: i + 1, total: chainItems.length },
			});
		}

		if (failFast && result.error && result.error !== "aborted") {
			// Mark remaining as aborted
			for (let j = i + 1; j < chainItems.length; j++) {
				const w = run.workers.get(padWorkerId(j, chainItems.length));
				if (w) {
					w.status = "aborted";
					w.endedAt = Date.now();
				}
			}
			updateRunCounts(run);
			updateStatusWidget(ctx);
			break;
		}

		previousOutput = result.text;
	}

	run.endedAt = Date.now();
	updateRunCounts(run);
	updateStatusWidget(ctx);
	return results;
}

// ─── Mode: Stages (mixed parallel/chain) ─────────────────────────────────────

interface StageSpec {
	tasks: Array<{ task: string; cwd?: string }>;
	mode: "parallel" | "chain";
}

async function executeStages(
	run: RunState,
	stages: StageSpec[],
	concurrency: number,
	failFast: boolean,
	signal: AbortSignal | undefined,
	onUpdate: ((update: { text: string; details: Record<string, unknown> }) => void) | undefined,
	ctx: ExtensionContext,
): Promise<TaskResult[]> {
	// Initialize all workers from all stages
	let globalIdx = 0;
	const totalTasks = stages.reduce((sum, s) => sum + s.tasks.length, 0);
	for (const stage of stages) {
		for (const item of stage.tasks) {
			const workerId = padWorkerId(globalIdx, totalTasks);
			const ws = createWorkerState(
				workerId,
				item.task,
				item.cwd ?? ctx.cwd,
				path.join(run.logDir, "workers", `${workerId}.events.ndjson`),
			);
			run.workers.set(workerId, ws);
			globalIdx++;
		}
	}
	run.total = totalTasks;
	run.mode = "stages";
	updateRunCounts(run);
	updateStatusWidget(ctx);

	const allResults: TaskResult[] = [];
	let stageOffset = 0;
	let previousOutput = "";

	for (let si = 0; si < stages.length; si++) {
		if (signal?.aborted) break;

		const stage = stages[si]!;

		if (stage.mode === "parallel") {
			const stageResults = await executeTasks(run, stage.tasks, concurrency, failFast, signal, undefined, ctx);
			allResults.push(...stageResults);

			if (failFast && stageResults.some((r) => r.error && r.error !== "aborted")) {
				// Abort any queued workers in remaining stages
				for (const ws of run.workers.values()) {
					if (ws.status === "queued") {
						ws.status = "aborted";
						ws.endedAt = Date.now();
					}
				}
				updateRunCounts(run);
				updateStatusWidget(ctx);
				break;
			}

			// Use last result's text as previousOutput for next stage
			const lastResult = stageResults[stageResults.length - 1];
			if (lastResult) previousOutput = lastResult.text;
		} else {
			// chain within stage
			for (let i = 0; i < stage.tasks.length; i++) {
				if (signal?.aborted) break;

				const item = stage.tasks[i]!;
				const taskWithContext = item.task.replace(/\{previous\}/g, previousOutput);
				const workerId = padWorkerId(stageOffset + i, totalTasks);
				const result = await executeSingleWorker(
					run,
					workerId,
					taskWithContext,
					item.cwd ?? ctx.cwd,
					signal,
					undefined,
					ctx,
				);
				allResults.push(result);

				if (failFast && result.error && result.error !== "aborted") {
					// Abort any queued workers in remaining stages
					for (const ws of run.workers.values()) {
						if (ws.status === "queued") {
							ws.status = "aborted";
							ws.endedAt = Date.now();
						}
					}
					break;
				}

				previousOutput = result.text;
			}
		}

		stageOffset += stage.tasks.length;

		if (onUpdate) {
			onUpdate({
				text: `Stage ${si + 1}/${stages.length} complete`,
				details: { mode: "stages", runId: run.runId, stage: si + 1, totalStages: stages.length },
			});
		}
	}

	run.endedAt = Date.now();
	updateRunCounts(run);
	updateStatusWidget(ctx);
	return allResults;
}

// ─── Tool registration ───────────────────────────────────────────────────────

const TaskItem = Type.Object({
	task: Type.String({ description: "Focused, self-contained task for one worker subagent." }),
	cwd: Type.Optional(Type.String({ description: "Working directory for this worker." })),
});

const SubagentParams = Type.Object({
	// Legacy: serial subtasks (kept for backward compatibility)
	subtasks: Type.Optional(
		Type.Array(TaskItem, {
			description:
				"DEPRECATED. Use 'tasks' (parallel) or 'chain' (sequential) instead. One or more subtasks run in order.",
			minItems: 1,
		}),
	),
	// Parallel batch
	tasks: Type.Optional(
		Type.Array(TaskItem, {
			description:
				"Batch of tasks run in parallel with bounded concurrency. Each worker gets an isolated child process.",
			minItems: 1,
		}),
	),
	// Sequential chain
	chain: Type.Optional(
		Type.Array(TaskItem, {
			description:
				"Tasks run sequentially in order. Use {previous} in a task to reference the output of the prior step.",
			minItems: 1,
		}),
	),
	// Mixed stages
	stages: Type.Optional(
		Type.Array(
			Type.Object({
				mode: Type.Union([Type.Literal("parallel"), Type.Literal("chain")], {
					description: 'Stage execution mode: "parallel" for concurrent workers, "chain" for sequential.',
				}),
				tasks: Type.Array(TaskItem, { description: "Tasks for this stage.", minItems: 1 }),
			}),
			{
				description: "Array of stages executed sequentially. Each stage runs its tasks in the specified mode.",
				minItems: 1,
			},
		),
	),
	concurrency: Type.Optional(
		Type.Number({ description: `Max concurrent workers (default: ${ENV_MAX_CONCURRENCY}). Clamped to safe limits.` }),
	),
	failFast: Type.Optional(
		Type.Boolean({
			description: "If true, abort remaining workers on first failure (default: false).",
			default: false,
		}),
	),
	observe: Type.Optional(
		Type.Union([Type.Literal("silent"), Type.Literal("compact"), Type.Literal("verbose")], {
			description:
				"Observation verbosity: silent (no per-worker output), compact (summary only), verbose (per-worker details).",
			default: "compact",
		}),
	),
});

function modeDescription(): string {
	return [
		"Delegate focused subtasks to parallel or sequential worker subagents.",
		"Each worker runs as an isolated child hamr process.",
		"",
		"Modes (exactly one required):",
		`• tasks: parallel batch with bounded concurrency (default max ${ENV_MAX_CONCURRENCY}).`,
		"• chain: sequential execution in order. Use {previous} in a task to reference prior output.",
		"• stages: sequential stages; each stage can be 'parallel' or 'chain'.",
		"• subtasks: DEPRECATED serial alias — use 'chain' instead.",
		"",
		"Concurrency is capped for memory/GPU safety. Thousands of planned workers are allowed;",
		"hundreds of simultaneous model calls are not. Default concurrency is conservative.",
		"",
		"Workers that fail do not kill the swarm unless failFast=true.",
		"Full logs persisted to disk: .hamr/subagents/runs/<runId>/",
	].join("\n");
}

function registerSubagentTool(pi: Parameters<ExtensionFactory>[0]): void {
	pi.registerTool(
		defineTool({
			name: "delegate_subagents",
			label: "Subagents",
			description: modeDescription(),
			promptSnippet: "Use delegate_subagents to dispatch focused subtasks to parallel/sequential worker subagents.",
			promptGuidelines: [
				"Each task should be a clearly scoped, self-contained piece of work.",
				"For independent subtasks, use 'tasks' (parallel batch). For dependent steps, use 'chain' or 'stages'.",
				"Use {previous} in chain/stages tasks to reference the prior worker's output.",
				"Parallel concurrency is bounded — do not worry about overloading, the system caps it safely.",
				"Delegate only as many tasks as the work genuinely warrants.",
			],
			parameters: SubagentParams,
			renderCall: (args, theme) => {
				const hasSubtasks = (args.subtasks?.length ?? 0) > 0;
				const hasTasks = (args.tasks?.length ?? 0) > 0;
				const hasChain = (args.chain?.length ?? 0) > 0;
				const hasStages = (args.stages?.length ?? 0) > 0;

				let modeLabel: string;
				let count: number;
				let items: Array<{ task: string; cwd?: string }>;

				if (hasStages) {
					const stageList = args.stages as StageSpec[];
					count = stageList.reduce((s, st) => s + st.tasks.length, 0);
					modeLabel = `stages (${stageList.length} stages, ${count} tasks)`;
					items = stageList.flatMap((s) => s.tasks);
				} else if (hasTasks) {
					modeLabel = `parallel (${args.tasks!.length} tasks)`;
					count = args.tasks!.length;
					items = args.tasks!;
				} else if (hasChain) {
					modeLabel = `chain (${args.chain!.length} steps)`;
					count = args.chain!.length;
					items = args.chain!;
				} else {
					modeLabel = `serial (${(args.subtasks as Array<{ task: string }>)?.length ?? 0} tasks)`;
					items = (args.subtasks as Array<{ task: string; cwd?: string }>) ?? [];
					count = items.length;
				}

				let text = theme.fg("toolTitle", theme.bold("delegate_subagents ")) + theme.fg("accent", modeLabel);
				for (let i = 0; i < Math.min(items.length, 3); i++) {
					const preview = items[i]!.task.length > 50 ? `${items[i]!.task.slice(0, 50)}…` : items[i]!.task;
					text += `\n  ${theme.fg("muted", `${i + 1}.`)} ${theme.fg("dim", preview)}`;
				}
				if (items.length > 3) text += `\n  ${theme.fg("muted", `… +${items.length - 3} more`)}`;
				return new Text(text, 0, 0);
			},
			renderResult: (result, options, theme) => {
				const details = result.details as
					| {
							mode: string;
							runId: string;
							total: number;
							done: number;
							failed: number;
							aborted: number;
							logDir: string;
							results?: TaskResult[];
					  }
					| undefined;

				if (!details?.results) {
					const text = result.content?.[0];
					return new Text(text?.type === "text" ? text.text : "(no output)", 0, 0);
				}

				const { results, mode: dMode, runId, logDir, done, failed, aborted } = details;
				const successCount = results.filter((r) => !r.error || r.error === "").length;
				const failCount = results.filter((r) => r.error && r.error !== "aborted").length;
				const abortedCount = aborted ?? results.filter((r) => r.error === "aborted").length;

				if (!options.expanded) {
					// Collapsed: summary line + log path
					const statusParts: string[] = [];
					if (done) statusParts.push(`${done ?? results.length} done`);
					if (failCount > 0) statusParts.push(`${failCount} failed`);
					if (abortedCount > 0) statusParts.push(`${abortedCount} aborted`);

					let text = theme.fg("toolTitle", `${dMode} `) + theme.fg("accent", statusParts.join(", "));
					text += `\n${theme.fg("muted", `logs: ${logDir}`)}`;
					text += `\n${theme.fg("muted", "(Ctrl+O to expand)")}`;
					return new Text(text, 0, 0);
				}

				// Expanded: per-worker details (bounded)
				const container = new Container();
				const headerIcon = failCount > 0 ? theme.fg("warning", "◐") : theme.fg("success", "✓");
				container.addChild(
					new Text(
						`${headerIcon} ${theme.fg("toolTitle", theme.bold(dMode))} ${theme.fg("accent", `${successCount}/${results.length} succeeded`)}`,
						0,
						0,
					),
				);

				// Show top failures first (max 5)
				const failures = results
					.map((r, i) => ({ result: r, idx: i }))
					.filter(({ result }) => result.error && result.error !== "aborted")
					.slice(0, 5);
				if (failures.length > 0) {
					container.addChild(new Spacer(1));
					container.addChild(new Text(theme.fg("error", "Failures:"), 0, 0));
					for (const { result: r, idx } of failures) {
						container.addChild(
							new Text(
								`  ${theme.fg("error", "✕")} [${padWorkerId(idx, results.length)}] ${theme.fg("dim", r.task.slice(0, 60))}`,
								0,
								0,
							),
						);
						if (r.error) container.addChild(new Text(`    ${theme.fg("error", r.error.slice(0, 120))}`, 0, 0));
					}
				}

				// Show recent successful workers (max 10)
				const successWorkers = results
					.map((r, i) => ({ result: r, idx: i }))
					.filter(({ result }) => !result.error || result.error === "")
					.slice(-10);

				if (successWorkers.length > 0) {
					container.addChild(new Spacer(1));
					for (const { result: r, idx } of successWorkers) {
						const usageStr = r.usage?.totalTokens ? ` ↓${formatTokens(r.usage.totalTokens)} tok` : "";
						container.addChild(
							new Text(
								`  ${theme.fg("success", "✓")} [${padWorkerId(idx, results.length)}] ${theme.fg("dim", r.task.slice(0, 50))}${theme.fg("muted", usageStr)}`,
								0,
								0,
							),
						);
						if (r.text) {
							const preview = r.text.slice(0, 120).replace(/\n/g, " ");
							container.addChild(new Text(`    ${theme.fg("toolOutput", preview)}`, 0, 0));
						}
					}
				}

				// Aborted workers
				if (abortedCount > 0) {
					container.addChild(new Spacer(1));
					container.addChild(new Text(theme.fg("muted", `${abortedCount} aborted`), 0, 0));
				}

				container.addChild(new Spacer(1));
				container.addChild(new Text(theme.fg("muted", `Full logs: ${logDir}`), 0, 0));

				return container;
			},
			execute: async (_toolCallId, params, signal, onUpdate, ctx) => {
				// Validate exactly one mode
				const hasSubtasks = (params.subtasks?.length ?? 0) > 0;
				const hasTasks = (params.tasks?.length ?? 0) > 0;
				const hasChain = (params.chain?.length ?? 0) > 0;
				const hasStages = (params.stages?.length ?? 0) > 0;
				const modeCount = Number(hasSubtasks) + Number(hasTasks) + Number(hasChain) + Number(hasStages);

				if (modeCount === 0) {
					return {
						content: [
							{
								type: "text",
								text: "No mode specified. Provide exactly one of: tasks, chain, stages, or subtasks (deprecated).",
							},
						],
						details: {},
					};
				}
				if (modeCount > 1) {
					return {
						content: [
							{
								type: "text",
								text: "Multiple modes specified. Provide exactly one of: tasks, chain, stages, or subtasks (deprecated).",
							},
						],
						details: {},
					};
				}

				// Determine concurrency
				const concurrency = clamp(
					params.concurrency ?? ENV_MAX_CONCURRENCY,
					1,
					ENV_MAX_LOCAL_CONCURRENCY > 0 ? ENV_MAX_LOCAL_CONCURRENCY : ENV_MAX_CONCURRENCY,
				);
				const failFast = params.failFast ?? false;
				const observe = (params.observe as string | undefined) ?? "compact";

				// Validate task counts against ENV soft limit (warn but don't block)
				let taskCount = 0;
				if (hasSubtasks) taskCount = (params.subtasks as Array<unknown>).length;
				else if (hasTasks) taskCount = (params.tasks as Array<unknown>).length;
				else if (hasChain) taskCount = (params.chain as Array<unknown>).length;
				else if (hasStages) {
					taskCount = (params.stages as Array<{ tasks: Array<unknown> }>).reduce((s, st) => s + st.tasks.length, 0);
				}
				if (taskCount > ENV_MAX_TASKS) {
					return {
						content: [
							{
								type: "text",
								text: `Too many tasks (${taskCount}). Soft limit is ${ENV_MAX_TASKS}. Set HAMR_SUBAGENT_MAX_TASKS to increase (hard max: ${ENV_HARD_MAX_TASKS}).`,
							},
						],
						details: {},
					};
				}

				// Create run state
				const runId = nextRunId();
				const logDir = ensureLogDir(runId, ctx.cwd);
				const run: RunState = {
					runId,
					mode: hasTasks ? "parallel" : hasChain ? "chain" : hasStages ? "stages" : "single",
					total: 0,
					queued: 0,
					running: 0,
					done: 0,
					failed: 0,
					aborted: 0,
					startedAt: Date.now(),
					usage: { ...EMPTY_USAGE },
					logDir,
					workers: new Map(),
				};
				activeRuns.set(runId, run);

				// Save run metadata
				try {
					fs.writeFileSync(
						path.join(logDir, "run.json"),
						JSON.stringify(
							{
								runId,
								mode: run.mode,
								startedAt: new Date(run.startedAt).toISOString(),
								cwd: ctx.cwd,
							},
							null,
							2,
						),
						"utf-8",
					);
				} catch {
					/* best-effort */
				}

				const onUpdateWrapper =
					observe !== "silent" && onUpdate
						? (update: { text: string; details: Record<string, unknown> }) => {
								onUpdate({
									content: [{ type: "text", text: update.text }],
									details: { ...update.details },
								});
							}
						: undefined;

				let results: TaskResult[];

				try {
					if (hasStages) {
						const stageSpecs: StageSpec[] = (
							params.stages as Array<{ mode: string; tasks: Array<{ task: string; cwd?: string }> }>
						).map((s) => ({
							mode: s.mode as "parallel" | "chain",
							tasks: s.tasks,
						}));
						results = await executeStages(run, stageSpecs, concurrency, failFast, signal, onUpdateWrapper, ctx);
					} else if (hasTasks) {
						const tasks = params.tasks as Array<{ task: string; cwd?: string }>;
						if (tasks.length > ENV_HARD_MAX_TASKS) {
							return {
								content: [
									{ type: "text", text: `Too many tasks (${tasks.length}). Hard limit is ${ENV_HARD_MAX_TASKS}.` },
								],
								details: { logDir },
							};
						}
						results = await executeTasks(run, tasks, concurrency, failFast, signal, onUpdateWrapper, ctx);
					} else if (hasChain) {
						const chain = params.chain as Array<{ task: string; cwd?: string }>;
						results = await executeChain(run, chain, failFast, signal, onUpdateWrapper, ctx);
					} else {
						// Legacy subtasks — run as chain
						const subtasks = params.subtasks as Array<{ task: string; cwd?: string }>;
						results = await executeChain(run, subtasks, failFast, signal, onUpdateWrapper, ctx);
					}
				} finally {
					// Save final run state
					try {
						fs.writeFileSync(
							path.join(logDir, "run.json"),
							JSON.stringify(
								{
									runId,
									mode: run.mode,
									total: run.total,
									done: run.done,
									failed: run.failed,
									aborted: run.aborted,
									startedAt: new Date(run.startedAt).toISOString(),
									endedAt: new Date().toISOString(),
									usage: run.usage,
									cwd: ctx.cwd,
								},
								null,
								2,
							),
							"utf-8",
						);
					} catch {
						/* best-effort */
					}
					evictOldRuns();
				}

				const errors = results.filter((r) => r.error && r.error !== "aborted");
				const successCount = results.length - errors.length;

				// Build summary
				const summaryParts: string[] = [];
				summaryParts.push(
					`Swarm ${runId} complete: ${successCount}/${results.length} succeeded${errors.length > 0 ? `, ${errors.length} failed` : ""}.`,
				);

				if (errors.length > 0) {
					summaryParts.push("Top failures:");
					for (const r of errors.slice(0, 5)) {
						summaryParts.push(`- [${r.workerId}] ${r.task.slice(0, 60)}: ${(r.error ?? "unknown").slice(0, 100)}`);
					}
				}

				summaryParts.push(`\nFull logs: ${logDir}`);
				summaryParts.push(`(Use /subagents open ${runId} for interactive details when available)`);

				return {
					content: [{ type: "text", text: summaryParts.join("\n") }],
					details: {
						mode: run.mode,
						runId,
						total: run.total,
						done: run.done,
						failed: run.failed,
						aborted: run.aborted,
						logDir,
						results,
					},
					...(errors.length > 0 ? { isError: true } : {}),
				};
			},
		}),
	);
}

// ─── Extension factory ───────────────────────────────────────────────────────

export function createHamrSubagentsExtension(
	_getChildExtensions: () => ExtensionFactory[],
	depth = 0,
): ExtensionFactory {
	const factory: ExtensionFactory = async (pi) => {
		// Leaf: no delegate tool, so recursion stops here.
		if (depth >= MAX_DEPTH) return;
		registerSubagentTool(pi);
	};
	(factory as { [HAMR_SUBAGENTS_FACTORY]?: boolean })[HAMR_SUBAGENTS_FACTORY] = true;
	return factory;
}
