import type { AssistantMessage, Usage } from "@hamr/ai";
import { type Component, Text, type TUI } from "@hamr/tui";
import { Type } from "typebox";
import { getAgentDir } from "../../config.ts";
import type { ExtensionContext, ExtensionFactory } from "../../core/extensions/types.ts";
import { defineTool } from "../../core/extensions/types.ts";
import { DefaultResourceLoader } from "../../core/resource-loader.ts";
import { createAgentSession } from "../../core/sdk.ts";
import { SessionManager } from "../../core/session-manager.ts";
import { SettingsManager } from "../../core/settings-manager.ts";
import type { Theme } from "../../modes/interactive/theme/theme.ts";
import { contentText, getAssistantText } from "../helpers.ts";
import { getCurrentTurnId, getMemory } from "../memory.ts";

/**
 * Subagents extension: the `delegate_subagents` tool, a fork-per-worker executor,
 * and a small live status line above the editor.
 *
 * A subagent is a SERIAL pi worker spawned as a fork of the live session
 * (`SessionManager.forkFrom`) — a real node in the session tree, observable the
 * pi-native way: the tool result expands (Ctrl+O) to each worker's handoff, and
 * the worker forks are navigable in the session selector (nested under the parent).
 * Recursion is bounded by depth: at MAX_DEPTH a worker gets no delegate tool.
 */

/** Marks the subagents factory so a parent can re-create it at depth + 1 for workers. */
export const HAMR_SUBAGENTS_FACTORY = Symbol.for("hamr.subagents.factory");

/** Recursion bound. Root = 0; at this depth the worker gets no delegate tool. */
const MAX_DEPTH = 3;

// ─── Live status (above the editor) ───────────────────────────────────────────

type SubagentRun = {
	name: string;
	status: "running" | "done" | "failed";
	task: string;
	startedAt: number;
	endedAt?: number;
	usage?: Usage;
	result?: string;
	error?: string;
};

const subagentRuns = new Map<string, SubagentRun>();
let runCounter = 0;

function nextRunId(): string {
	return `subagent-${Date.now()}-${++runCounter}`;
}

const RUNNING_FRAMES = ["◐", "◓", "◑", "◒"];
function animationFrame(): string {
	return RUNNING_FRAMES[Math.floor(Date.now() / 180) % RUNNING_FRAMES.length]!;
}

function formatTokens(tokens: number): string {
	if (tokens >= 1_000_000) return `${(tokens / 1_000_000).toFixed(1)}m`;
	if (tokens >= 10_000) return `${Math.round(tokens / 1000)}k`;
	if (tokens >= 1000) return `${(tokens / 1000).toFixed(1)}k`;
	return `${tokens}`;
}

function summarizeRuns(): { running: number; done: number; failed: number; totalTokens: number } {
	return Array.from(subagentRuns.values()).reduce(
		(acc, run) => {
			acc[run.status] += 1;
			acc.totalTokens += run.usage?.totalTokens ?? 0;
			return acc;
		},
		{ running: 0, done: 0, failed: 0, totalTokens: 0 },
	);
}

function renderStatusLine(): string | undefined {
	if (subagentRuns.size === 0) return undefined;
	const counts = summarizeRuns();
	const parts: string[] = [];
	if (counts.running > 0) parts.push(`${animationFrame()} ${counts.running} working`);
	if (counts.done > 0) parts.push(`✓ ${counts.done} done`);
	if (counts.failed > 0) parts.push(`✕ ${counts.failed} failed`);
	if (counts.totalTokens > 0) parts.push(`${formatTokens(counts.totalTokens)} tok`);
	return `subagents ${parts.join(" · ")}`;
}

/** Animated one-liner above the editor showing workers working / done. */
class AgentStatusWidget implements Component {
	private interval: ReturnType<typeof setInterval> | undefined;
	private tui: TUI;
	private theme: Theme;

	constructor(tui: TUI, theme: Theme) {
		this.tui = tui;
		this.theme = theme;
		this.interval = setInterval(() => {
			if (summarizeRuns().running > 0) this.tui.requestRender();
		}, 180);
	}

	render(): string[] {
		const line = renderStatusLine();
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

// ─── Worker (fork of the live session) ────────────────────────────────────────

type SubagentResult = { task: string; text: string; error?: string };

async function runOneSubagent(
	pi: Parameters<ExtensionFactory>[0],
	task: string,
	signal: AbortSignal | undefined,
	ctx: ExtensionContext,
	getChildExtensions: () => ExtensionFactory[],
): Promise<SubagentResult> {
	const runId = nextRunId();
	const name = task.slice(0, 48);
	subagentRuns.set(runId, { name, status: "running", task, startedAt: Date.now() });
	updateStatusWidget(ctx);

	const prompt = [
		"You are a focused Hamr subagent. Complete only the delegated task and return a concise handoff.",
		`Task:\n${task}`,
	].join("\n\n");

	try {
		const agentDir = getAgentDir();
		const settingsManager = SettingsManager.create(ctx.cwd, agentDir);
		const resourceLoader = new DefaultResourceLoader({
			cwd: ctx.cwd,
			agentDir,
			settingsManager,
			extensionFactories: getChildExtensions(),
		});
		await resourceLoader.reload();
		// Fork the live session into a real node in the session tree: the worker
		// inherits the parent context and is navigable via the session selector
		// (nested under the parent). Falls back to in-memory if not persisted.
		const parentFile = ctx.sessionManager.getSessionFile();
		const workerSessionManager = parentFile
			? SessionManager.forkFrom(parentFile, ctx.cwd, ctx.sessionManager.getSessionDir())
			: SessionManager.inMemory(ctx.cwd);
		const child = await createAgentSession({
			cwd: ctx.cwd,
			agentDir,
			model: ctx.model!,
			thinkingLevel: pi.getThinkingLevel(),
			modelRegistry: ctx.modelRegistry,
			settingsManager,
			resourceLoader,
			sessionManager: workerSessionManager,
		});
		if (signal?.aborted) {
			subagentRuns.set(runId, { name, status: "failed", task, startedAt: Date.now(), endedAt: Date.now(), error: "aborted" });
			updateStatusWidget(ctx);
			return { task, text: "Subagent aborted before start.", error: "aborted" };
		}
		await child.session.prompt(prompt, { source: "extension" });
		const assistant = [...child.session.messages].reverse().find((m) => m.role === "assistant") as
			| AssistantMessage
			| undefined;
		const text = assistant
			? getAssistantText(assistant) || "(subagent returned no text)"
			: "(subagent returned no message)";
		getMemory(ctx)?.store({
			sessionId: ctx.sessionManager.getSessionId(),
			turnId: getCurrentTurnId(),
			role: "tool",
			toolName: "delegate_subagent",
			content: `Task: ${task}\n\n${text}`,
			domainTags: ["hamr", "subagent"],
		});
		subagentRuns.set(runId, {
			name,
			status: "done",
			task,
			startedAt: subagentRuns.get(runId)?.startedAt ?? Date.now(),
			endedAt: Date.now(),
			usage: assistant?.usage,
			result: text,
		});
		updateStatusWidget(ctx);
		return { task, text };
	} catch (error) {
		const message = error instanceof Error ? error.message : String(error);
		subagentRuns.set(runId, {
			name,
			status: "failed",
			task,
			startedAt: subagentRuns.get(runId)?.startedAt ?? Date.now(),
			endedAt: Date.now(),
			error: message,
		});
		updateStatusWidget(ctx);
		return { task, text: `Subagent failed: ${message}`, error: message };
	}
}

// ─── Tool ─────────────────────────────────────────────────────────────────────

function registerSubagentTool(
	pi: Parameters<ExtensionFactory>[0],
	getChildExtensions: () => ExtensionFactory[],
): void {
	pi.registerTool(
		defineTool({
			name: "delegate_subagents",
			label: "Subagents",
			description:
				"Delegate one or more focused subtasks to worker subagents. Each worker is a fork of the current session that runs its narrow task and returns a handoff. Workers run one at a time (serial). Use to divide-and-conquer large efforts.",
			promptSnippet: "Use delegate_subagents to hand focused subtasks to serial worker subagents.",
			promptGuidelines: [
				"Each subtask should be a clearly scoped, self-contained piece of work.",
				"Workers run serially in order and cannot see each other's results; for dependent steps, call again after reading the handoffs.",
				"Delegate only as many subtasks as the work genuinely warrants.",
			],
			parameters: Type.Object({
				subtasks: Type.Array(
					Type.Object({
						task: Type.String({ description: "Focused, self-contained task for one worker subagent." }),
					}),
					{ description: "One or more subtasks, run in order.", minItems: 1 },
				),
			}),
			renderCall: (_args, theme) => new Text(theme.fg("toolTitle", theme.bold("delegate_subagents")), 0, 0),
			renderResult: (result, options, theme) => {
				const results = (result.details as { results?: SubagentResult[] } | undefined)?.results ?? [];
				const failed = results.filter((r) => r.error && r.error !== "aborted").length;
				const done = results.length - failed;
				if (!options.expanded) {
					const status = failed > 0 ? `${done}/${results.length} done, ${failed} failed` : `${results.length} subagent${results.length !== 1 ? "s" : ""} done`;
					return new Text(theme.fg("dim", status), 0, 0);
				}
				// Expanded: per-worker handoff — the pi-native way to observe each subagent.
				const lines = results
					.map((r, i) => {
						const glyph = r.error ? theme.fg("error", "✕") : theme.fg("success", "✓");
						const head = `${glyph} ${theme.fg("toolTitle", `${i + 1}. ${r.task.slice(0, 64)}`)}`;
						return `${head}\n${theme.fg("toolOutput", r.text.slice(0, 1200))}`;
					})
					.join("\n\n");
				return new Text(lines || theme.fg("dim", "(no subagents)"), 0, 0);
			},
			execute: async (_toolCallId, params, signal, onUpdate, ctx) => {
				if (!ctx.model) {
					return { content: [{ type: "text", text: "No current model is available for subagents." }], details: {} };
				}
				if (params.subtasks.length === 0) {
					return { content: [{ type: "text", text: "No subtasks provided." }], details: {} };
				}

				// Fresh status for this call; serial — one forked worker at a time.
				subagentRuns.clear();
				const results: SubagentResult[] = [];
				const total = params.subtasks.length;
				for (let i = 0; i < total; i++) {
					onUpdate?.({
						content: [{ type: "text", text: `subagent ${i + 1}/${total}: ${params.subtasks[i]!.task.slice(0, 60)}` }],
						details: { results: [...results] },
					});
					results.push(await runOneSubagent(pi, params.subtasks[i]!.task, signal, ctx, getChildExtensions));
					if (signal?.aborted) break;
				}

				const errors = results.filter((r) => r.error && r.error !== "aborted");
				const summary = results
					.map((r, i) => `${i + 1}. ${r.error ? "✗" : "✓"} ${r.task.slice(0, 60)}\n${r.text.slice(0, 300)}`)
					.join("\n\n");
				return {
					content: [{ type: "text", text: summary }],
					details: { results },
					...(errors.length > 0 ? { isError: true } : {}),
				};
			},
		}),
	);
}

// ─── Extension factory ────────────────────────────────────────────────────────

export function createHamrSubagentsExtension(
	getChildExtensions: () => ExtensionFactory[],
	depth = 0,
): ExtensionFactory {
	const factory: ExtensionFactory = async (pi) => {
		// Leaf: no delegate tool, so recursion stops here.
		if (depth >= MAX_DEPTH) return;
		// Workers load the identical set, with THIS factory bumped to depth + 1.
		registerSubagentTool(pi, () =>
			getChildExtensions().map((f) =>
				(f as { [HAMR_SUBAGENTS_FACTORY]?: boolean })[HAMR_SUBAGENTS_FACTORY]
					? createHamrSubagentsExtension(getChildExtensions, depth + 1)
					: f,
			),
		);
	};
	(factory as { [HAMR_SUBAGENTS_FACTORY]?: boolean })[HAMR_SUBAGENTS_FACTORY] = true;
	return factory;
}
