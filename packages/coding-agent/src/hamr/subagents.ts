import type { AssistantMessage, Usage } from "@hamr/ai";
import { type Component, type Focusable, Key, matchesKey, type TUI, visibleWidth } from "@hamr/tui";
import { Type } from "typebox";
import { getAgentDir } from "../config.ts";
import type { ExtensionContext, ExtensionFactory } from "../core/extensions/types.ts";
import { defineTool } from "../core/extensions/types.ts";
import { DefaultResourceLoader } from "../core/resource-loader.ts";
import { createAgentSession } from "../core/sdk.ts";
import { SessionManager } from "../core/session-manager.ts";
import { SettingsManager } from "../core/settings-manager.ts";
import type { Theme } from "../modes/interactive/theme/theme.ts";
import { getAssistantText } from "./helpers.ts";
import { getCurrentTurnId, getMemory } from "./memory.ts";

type SubagentRun = {
	name: string;
	status: "running" | "done" | "failed";
	task: string;
	mode: SubagentParams["mode"];
	action: string;
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

function formatUsage(usage?: Usage): string {
	if (!usage) return "tokens pending";
	const parts = [`${formatTokens(usage.totalTokens)} tok`];
	if (usage.input > 0) parts.push(`in ${formatTokens(usage.input)}`);
	if (usage.output > 0) parts.push(`out ${formatTokens(usage.output)}`);
	return parts.join(" · ");
}

function getSubagentRuns(): Array<[string, SubagentRun]> {
	return Array.from(subagentRuns.entries());
}

function summarizeRuns(): { running: number; done: number; failed: number; totalTokens: number } {
	return getSubagentRuns().reduce(
		(acc, [, run]) => {
			acc[run.status] += 1;
			acc.totalTokens += run.usage?.totalTokens ?? 0;
			return acc;
		},
		{ running: 0, done: 0, failed: 0, totalTokens: 0 },
	);
}

function renderSubagentDashboard(): string[] {
	const runs = Array.from(subagentRuns.values());
	if (runs.length === 0) return [];
	const counts = summarizeRuns();
	const parts: string[] = [];
	if (counts.running > 0) parts.push(`${animationFrame()} ${counts.running} working`);
	if (counts.done > 0) parts.push(`✓ ${counts.done} done`);
	if (counts.failed > 0) parts.push(`✕ ${counts.failed} failed`);
	if (counts.totalTokens > 0) parts.push(`${formatTokens(counts.totalTokens)} tok`);
	const summary = parts.length > 0 ? parts.join(" · ") : `${runs.length} agents`;
	return [`agents ${summary}`];
}

class AgentStatusWidget implements Component {
	private theme: Theme;
	private tui: TUI;
	private interval: ReturnType<typeof setInterval> | undefined;

	constructor(tui: TUI, theme: Theme) {
		this.tui = tui;
		this.theme = theme;
		this.interval = setInterval(() => {
			if (summarizeRuns().running > 0) {
				this.tui.requestRender();
			}
		}, 180);
	}

	render(_availableWidth: number): string[] {
		const lines = renderSubagentDashboard();
		if (lines.length === 0) return [];
		const text = `${lines[0]} · shift+ctrl+o dashboard`;
		return [` ${this.theme.fg("muted", text)}`];
	}

	invalidate(): void {}

	dispose(): void {
		if (this.interval) {
			clearInterval(this.interval);
			this.interval = undefined;
		}
	}
}

function updateSubagentDashboard(ctx: ExtensionContext): void {
	if (ctx.mode !== "tui") return;
	const lines = renderSubagentDashboard();
	ctx.ui.setStatus("hamr.subagents", lines[0]);
	const widget = lines.length > 0 ? (tui: TUI, theme: Theme) => new AgentStatusWidget(tui, theme) : undefined;
	ctx.ui.setWidget("hamr.subagents.above", widget, { placement: "aboveEditor" });
	ctx.ui.setWidget("hamr.subagents.below", undefined, { placement: "belowEditor" });
}

// ─── Agent Dashboard ────────────────────────────────────────────────────────

export interface AgentInfo {
	id: string;
	name: string;
	status: "running" | "waiting" | "done" | "failed";
	action?: string;
	elapsed?: number;
	mode?: SubagentParams["mode"];
	task?: string;
	usage?: Usage;
	result?: string;
	error?: string;
	countAsAgent?: boolean;
}

function formatElapsed(seconds: number): string {
	if (seconds < 60) return `${seconds}s`;
	const m = Math.floor(seconds / 60);
	const s = seconds % 60;
	return `${m}:${s.toString().padStart(2, "0")}`;
}

const STATUS_GLYPH: Record<AgentInfo["status"], { glyph: string; color: string }> = {
	running: { glyph: "◌", color: "accent" },
	waiting: { glyph: "◷", color: "muted" },
	done: { glyph: "✓", color: "success" },
	failed: { glyph: "✕", color: "error" },
};

export type AgentDashboardResult = { action: "close" } | { action: "retry"; index: number };

export function hasSubagentRuns(): boolean {
	return subagentRuns.size > 0;
}

export async function showAgentDashboard(
	_pi: Parameters<ExtensionFactory>[0],
	ctx: ExtensionContext,
): Promise<AgentDashboardResult> {
	return ctx.ui.custom<AgentDashboardResult>(
		(tui, theme, _keybindings, done) => {
			const model = ctx.model;
			const getAgents = (): AgentInfo[] => [
				{
					id: "main",
					name: model ? (model.name ?? model.id) : "Main agent",
					status: "running",
					action: model ? `${model.provider} · ${_pi.getThinkingLevel()}` : "orchestrating",
				},
				...getSubagentRuns().map(([id, run]) => ({
					id,
					name: run.name,
					status: run.status,
					action: run.action,
					elapsed: ((run.endedAt ?? Date.now()) - run.startedAt) / 1000,
					mode: run.mode,
					task: run.task,
					usage: run.usage,
					result: run.result,
					error: run.error,
				})),
			];
			return new AgentDashboardComponent(theme, done, getAgents, tui);
		},
		{
			overlay: true,
			overlayOptions: { width: "80%", maxHeight: "80%", anchor: "center", margin: { top: 2 } },
		},
	);
}

export async function showIdleAgentDashboard(
	pi: Parameters<ExtensionFactory>[0],
	ctx: ExtensionContext,
): Promise<AgentDashboardResult> {
	const model = ctx.model;
	const entries = ctx.sessionManager.getEntries().filter((entry) => entry.type === "message").length;
	const contextUsage = ctx.getContextUsage();
	const context =
		typeof contextUsage?.percent === "number" && contextUsage.contextWindow
			? `${contextUsage.percent.toFixed(1)}% / ${Math.round(contextUsage.contextWindow / 1000)}K ctx`
			: "context pending";
	const commands = pi.getCommands();
	const agents: AgentInfo[] = [
		{
			id: "model",
			name: model ? (model.name ?? model.id) : "No model loaded",
			status: model ? "done" : "waiting",
			action: model ? `${model.provider} · ${pi.getThinkingLevel()}` : "select a model to submit",
		},
		{
			id: "session",
			name: "Session",
			status: "waiting",
			action: `${entries} messages · ${context}`,
			countAsAgent: false,
		},
		{
			id: "commands",
			name: "Commands",
			status: "waiting",
			action: `${commands.length} slash commands available`,
			countAsAgent: false,
		},
	];

	return ctx.ui.custom<AgentDashboardResult>(
		(tui, theme, _keybindings, done) => new AgentDashboardComponent(theme, done, () => agents, tui),
		{
			overlay: true,
			overlayOptions: { width: "80%", maxHeight: "80%", anchor: "center", margin: { top: 2 } },
		},
	);
}

export class AgentDashboardComponent implements Focusable {
	focused = false;
	private theme: Theme;
	private done: (result: AgentDashboardResult) => void;
	private getAgents: () => AgentInfo[];
	private selectedIndex = 0;
	private title: string;
	private mode: "list" | "detail" = "list";
	private interval: ReturnType<typeof setInterval> | undefined;

	constructor(
		theme: Theme,
		done: (result: AgentDashboardResult) => void,
		agents: AgentInfo[] | (() => AgentInfo[]),
		tui?: TUI,
		title = "AGENT DASHBOARD",
	) {
		this.theme = theme;
		this.done = done;
		this.getAgents = Array.isArray(agents) ? () => agents : agents;
		this.title = title;
		if (this.getAgents().length > 0) this.selectedIndex = 0;
		if (tui) {
			this.interval = setInterval(() => tui.requestRender(), 250);
		}
	}

	private moveUp(): void {
		const agents = this.getAgents();
		if (agents.length === 0) return;
		this.selectedIndex = Math.max(0, this.selectedIndex - 1);
	}

	private moveDown(): void {
		const agents = this.getAgents();
		if (agents.length === 0) return;
		this.selectedIndex = Math.min(agents.length - 1, this.selectedIndex + 1);
	}

	handleInput(data: string): void {
		if (this.mode === "detail") {
			if (matchesKey(data, Key.escape) || data === "q") {
				this.mode = "list";
			}
			return;
		}
		if (matchesKey(data, Key.escape)) {
			this.done({ action: "close" });
			return;
		}
		const agents = this.getAgents();
		if (agents.length === 0) return;
		if (matchesKey(data, Key.up) || data === "k") {
			this.moveUp();
		} else if (matchesKey(data, Key.down) || data === "j") {
			this.moveDown();
		} else if (data === "r") {
			this.done({ action: "retry", index: this.selectedIndex });
		} else if (matchesKey(data, Key.enter)) {
			this.mode = "detail";
		}
	}

	render(availableWidth: number): string[] {
		const th = this.theme;
		const agents = this.getAgents();
		if (this.selectedIndex >= agents.length) this.selectedIndex = Math.max(0, agents.length - 1);
		const innerW = Math.max(20, availableWidth - 2);
		const lines: string[] = [];
		const pad = (s: string, len: number): string => s + " ".repeat(Math.max(0, len - visibleWidth(s)));
		const row = (content: string): string => th.fg("border", "│") + pad(content, innerW) + th.fg("border", "│");

		// Top border
		lines.push(th.fg("border", `╭${"─".repeat(innerW)}╮`));

		if (agents.length === 0) {
			// Empty state
			lines.push(row(` ${th.fg("accent", "── subagents ──")}`));
			lines.push(row(""));
			const emptyLine = "  ◇  No active subagents";
			lines.push(row(` ${th.fg("muted", emptyLine)}`));
			lines.push(row(` ${th.fg("dim", "  subagents appear when parallel tasks are dispatched")}`));
		} else if (this.mode === "detail") {
			const agent = agents[this.selectedIndex]!;
			const statusInfo = STATUS_GLYPH[agent.status];
			lines.push(row(` ${th.bold(th.fg("accent", agent.name))}`));
			lines.push(th.fg("border", `├${"─".repeat(innerW)}┤`));
			lines.push(
				row(
					` ${th.fg(statusInfo.color as "accent" | "muted" | "success" | "error", statusInfo.glyph)} ${agent.status}`,
				),
			);
			if (agent.mode) lines.push(row(` ${th.fg("dim", "mode")} ${agent.mode}`));
			if (agent.elapsed !== undefined)
				lines.push(row(` ${th.fg("dim", "elapsed")} ${formatElapsed(Math.floor(agent.elapsed))}`));
			lines.push(row(` ${th.fg("dim", "tokens")} ${formatUsage(agent.usage)}`));
			if (agent.task) {
				lines.push(row(""));
				lines.push(row(` ${th.fg("dim", "task")}`));
				for (const line of agent.task.split("\n").slice(0, 4)) {
					lines.push(row(`   ${line}`));
				}
			}
			const detail = agent.error ?? agent.result;
			if (detail) {
				lines.push(row(""));
				lines.push(row(` ${th.fg("dim", agent.error ? "error" : "latest")}`));
				for (const line of detail.split("\n").slice(0, 8)) {
					lines.push(row(`   ${line}`));
				}
			}
		} else {
			// Header
			const agentCount = agents.filter((agent) => agent.countAsAgent !== false).length;
			const header = `${this.title} · ${agentCount} total`;
			lines.push(row(` ${th.bold(th.fg("accent", header))}`));
			lines.push(th.fg("border", `├${"─".repeat(innerW)}┤`));

			// Agent rows
			for (let i = 0; i < agents.length; i++) {
				const agent = agents[i]!;
				const isSelected = i === this.selectedIndex;
				const prefix = isSelected ? ` ${th.fg("accent", "▸")} ` : "   ";
				const statusInfo = STATUS_GLYPH[agent.status];
				const glyph = th.fg(statusInfo.color as "accent" | "muted" | "success" | "error", statusInfo.glyph);
				const name = th.fg("text", agent.name);
				const line = `${prefix}${glyph} ${name}`;
				const action = agent.action ? `  ${th.fg("dim", agent.action)}` : "";
				const elapsed =
					agent.elapsed !== undefined ? `  ${th.fg("muted", formatElapsed(Math.floor(agent.elapsed)))}` : "";
				const usage = agent.usage ? `  ${th.fg("muted", formatUsage(agent.usage))}` : "";
				lines.push(row(`${line}${action}${elapsed}${usage}`));
			}
		}

		// Footer
		lines.push(th.fg("border", `├${"─".repeat(innerW)}┤`));
		const footer =
			this.mode === "detail" ? "q back  ·  esc back" : "↑↓ navigate  ·  enter details  ·  r retry  ·  esc close";
		lines.push(row(` ${th.fg("dim", footer)}`));

		// Bottom border
		lines.push(th.fg("border", `╰${"─".repeat(innerW)}╯`));

		return lines;
	}

	invalidate(): void {}
	dispose(): void {
		if (this.interval) {
			clearInterval(this.interval);
			this.interval = undefined;
		}
	}
}

interface SubagentParams {
	task: string;
	mode: "read_only" | "full" | "no_tools";
}

async function runOneSubagent(
	pi: Parameters<ExtensionFactory>[0],
	params: SubagentParams,
	signal: AbortSignal | undefined,
	ctx: ExtensionContext,
	getChildExtensions: () => ExtensionFactory[],
): Promise<{ task: string; text: string; error?: string }> {
	const runId = nextRunId();
	subagentRuns.set(runId, {
		name: params.task.slice(0, 48),
		status: "running",
		task: params.task,
		mode: params.mode,
		action: `${params.mode} · starting`,
		startedAt: Date.now(),
	});
	updateSubagentDashboard(ctx);

	const memory = getMemory(ctx);
	const memoryIndex = memory?.buildMemoryIndex();
	const prompt = [
		"You are a focused Hamr subagent. Complete only the delegated task and return a concise handoff.",
		memoryIndex ? `Available memory index:\n${memoryIndex}` : undefined,
		`Task:\n${params.task}`,
	]
		.filter((part): part is string => part !== undefined)
		.join("\n\n");

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
		const child = await createAgentSession({
			cwd: ctx.cwd,
			agentDir,
			model: ctx.model!,
			thinkingLevel: pi.getThinkingLevel(),
			modelRegistry: ctx.modelRegistry,
			settingsManager,
			resourceLoader,
			sessionManager: SessionManager.inMemory(ctx.cwd),
			noTools: params.mode === "no_tools" ? "all" : undefined,
			tools: params.mode === "full" || params.mode === "no_tools" ? undefined : ["read", "grep", "find", "ls"],
		});
		if (signal?.aborted) {
			subagentRuns.set(runId, {
				name: params.task.slice(0, 48),
				status: "failed",
				task: params.task,
				mode: params.mode,
				action: "aborted",
				error: "aborted",
				startedAt: Date.now(),
				endedAt: Date.now(),
			});
			updateSubagentDashboard(ctx);
			return { task: params.task, text: "Subagent aborted before start.", error: "aborted" };
		}
		subagentRuns.set(runId, {
			name: params.task.slice(0, 48),
			status: "running",
			task: params.task,
			mode: params.mode,
			action: `${params.mode} · thinking`,
			startedAt: subagentRuns.get(runId)?.startedAt ?? Date.now(),
		});
		updateSubagentDashboard(ctx);
		await child.session.prompt(prompt, { source: "extension" });
		const assistant = [...child.session.messages].reverse().find((m) => m.role === "assistant") as
			| AssistantMessage
			| undefined;
		const usage = assistant?.usage;
		const text = assistant
			? getAssistantText(assistant) || "(subagent returned no text)"
			: "(subagent returned no message)";
		memory?.store({
			sessionId: ctx.sessionManager.getSessionId(),
			turnId: getCurrentTurnId(),
			role: "tool",
			toolName: "delegate_subagent",
			content: `Task: ${params.task}\n\n${text}`,
			domainTags: ["hamr", "subagent"],
		});
		subagentRuns.set(runId, {
			name: params.task.slice(0, 48),
			status: "done",
			task: params.task,
			mode: params.mode,
			action: "handoff saved",
			startedAt: subagentRuns.get(runId)?.startedAt ?? Date.now(),
			endedAt: Date.now(),
			usage,
			result: text,
		});
		updateSubagentDashboard(ctx);
		return { task: params.task, text };
	} catch (error) {
		const message = error instanceof Error ? error.message : String(error);
		subagentRuns.set(runId, {
			name: params.task.slice(0, 48),
			status: "failed",
			task: params.task,
			mode: params.mode,
			action: message,
			startedAt: subagentRuns.get(runId)?.startedAt ?? Date.now(),
			endedAt: Date.now(),
			error: message,
		});
		updateSubagentDashboard(ctx);
		return { task: params.task, text: `Subagent failed: ${message}`, error: message };
	}
}

/** Message returned when a local/relay model attempts to dispatch subagents. */
const DISPATCH_DISABLED_MESSAGE =
	"Subagent dispatch is disabled for local/relay models. Switch to a cloud model to delegate work, or complete the task in this session.";

export function registerSubagentTool(
	pi: Parameters<ExtensionFactory>[0],
	/**
	 * The extension set a spawned child session should load. A thunk so the
	 * default-extension array can be resolved lazily at spawn time (avoids
	 * import cycles between this module and the extension composition).
	 */
	getChildExtensions: () => ExtensionFactory[],
	/**
	 * Whether the current model may dispatch subagents. Local/relay models are
	 * gated out so they cannot fan out parallel work onto a single inference
	 * backend. Defaults to always-allowed when not provided.
	 */
	allowDispatch: (ctx: ExtensionContext) => boolean = () => true,
): void {
	// Singular subagent — parallel execution mode so multiple calls fan out
	pi.registerTool(
		defineTool({
			name: "delegate_subagent",
			label: "Subagent",
			description:
				"Delegate a focused investigation or implementation step to an isolated Hamr subagent using the current model.",
			promptSnippet:
				"Use delegate_subagent for parallelizable research, scoped code inspection, or independent verification.",
			promptGuidelines: [
				"Use delegate_subagent only for clearly scoped work with an explicit expected output.",
				"Prefer read_only mode unless the delegated task must modify files.",
				"Call delegate_subagent multiple times in one message to fan out independent work in parallel.",
			],
			executionMode: "parallel",
			parameters: Type.Object({
				task: Type.String({ description: "Focused task for the child agent." }),
				mode: Type.Optional(
					Type.Union([Type.Literal("read_only"), Type.Literal("full"), Type.Literal("no_tools")], {
						description: "Child tool access. Defaults to read_only.",
					}),
				),
			}),
			execute: async (_toolCallId, params, signal, _onUpdate, ctx) => {
				if (!ctx.model) {
					return { content: [{ type: "text", text: "No current model is available for a subagent." }], details: {} };
				}
				if (!allowDispatch(ctx)) {
					return { content: [{ type: "text", text: DISPATCH_DISABLED_MESSAGE }], details: {}, isError: true };
				}
				const result = await runOneSubagent(
					pi,
					{ task: params.task, mode: params.mode ?? "read_only" },
					signal,
					ctx,
					getChildExtensions,
				);
				if (result.error && result.error !== "aborted") {
					return {
						content: [{ type: "text", text: result.text }],
						details: { task: params.task, mode: params.mode ?? "read_only", error: result.error },
						isError: true,
					};
				}
				pi.sendMessage(
					{
						customType: "hamr.subagent",
						content: `Subagent completed: ${params.task}`,
						display: true,
						details: { task: params.task, mode: params.mode ?? "read_only", result: result.text },
					},
					{ triggerTurn: false },
				);
				return {
					content: [{ type: "text", text: result.text }],
					details: { task: params.task, mode: params.mode ?? "read_only" },
				};
			},
		}),
	);

	// Plural: explicit parallel fan-out from a single tool call
	pi.registerTool(
		defineTool({
			name: "delegate_subagents",
			label: "Parallel subagents",
			description:
				"Dispatch multiple subagents in parallel. Each runs independently and returns a handoff. Use for fan-out investigation or concurrent implementation steps.",
			promptSnippet: "Use delegate_subagents to run multiple independent investigations or tasks concurrently.",
			promptGuidelines: [
				"Each subtask should be clearly scoped and independent of the others.",
				"Prefer read_only mode unless a subtask must modify files.",
				"All subtasks run in parallel — they cannot depend on each other's results.",
			],
			executionMode: "sequential",
			parameters: Type.Object({
				subtasks: Type.Array(
					Type.Object({
						task: Type.String({ description: "Focused task for this child agent." }),
						mode: Type.Optional(
							Type.Union([Type.Literal("read_only"), Type.Literal("full"), Type.Literal("no_tools")], {
								description: "Child tool access. Defaults to read_only.",
							}),
						),
					}),
					{ description: "Array of subtask specs. All run in parallel.", minItems: 1, maxItems: 8 },
				),
			}),
			execute: async (_toolCallId, params, signal, _onUpdate, ctx) => {
				if (!ctx.model) {
					return { content: [{ type: "text", text: "No current model is available for subagents." }], details: {} };
				}
				if (!allowDispatch(ctx)) {
					return { content: [{ type: "text", text: DISPATCH_DISABLED_MESSAGE }], details: {}, isError: true };
				}
				if (params.subtasks.length === 0) {
					return { content: [{ type: "text", text: "No subtasks provided." }], details: {} };
				}

				const results = await Promise.all(
					params.subtasks.map((st) =>
						runOneSubagent(pi, { task: st.task, mode: st.mode ?? "read_only" }, signal, ctx, getChildExtensions),
					),
				);

				const summary = results
					.map((r, i) => `${i + 1}. ${r.error ? "✗" : "✓"} ${r.task.slice(0, 60)}\n${r.text.slice(0, 300)}`)
					.join("\n\n");

				const errors = results.filter((r) => r.error && r.error !== "aborted");
				if (errors.length > 0) {
					pi.sendMessage(
						{
							customType: "hamr.subagents",
							content: `${results.length - errors.length}/${results.length} subagents completed, ${errors.length} failed`,
							display: true,
							details: { results },
						},
						{ triggerTurn: false },
					);
				}

				return {
					content: [{ type: "text", text: `${results.length} subagents completed.\n\n${summary}` }],
					details: { results },
					...(errors.length > 0 ? { isError: true } : {}),
				};
			},
		}),
	);
}
