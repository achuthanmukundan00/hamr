import { mkdirSync } from "node:fs";
import { dirname, join } from "node:path";
import type { AgentMessage } from "@hamr/agent";
import type { AssistantMessage, TextContent, ToolCall, ToolResultMessage } from "@hamr/ai";
import type { AssistantMessageEvent } from "@hamr/ai";
import { Key } from "@hamr/tui";
import { Type } from "typebox";
import { getAgentDir } from "../config.ts";
import { defineTool, type ExtensionContext, type ExtensionFactory } from "../core/extensions/types.ts";
import { DefaultResourceLoader } from "../core/resource-loader.ts";
import { createAgentSession } from "../core/sdk.ts";
import { SessionManager } from "../core/session-manager.ts";
import { SettingsManager } from "../core/settings-manager.ts";
import { HolographicMemory } from "./memory/HolographicMemory.ts";
import { detectParserId } from "./providers/parsers/types.ts";
import { parseModelOutput } from "./providers/tool-calls.ts";
import {
	buildHamrProviderRegistrations,
	getHamrDefaultModel,
	type HamrStartupConfig,
	loadHamrStartupConfig,
} from "./startup-config.ts";
import { loadBetterSqlite3 } from "./store/sqlite-loader.ts";
import { DashboardComponent, type DashboardData } from "./dashboard.ts";

type MemoryHandle = {
	path: string;
	memory: HolographicMemory;
};

const parserByModel = new Map<string, string>();
let memoryHandle: MemoryHandle | undefined;
let currentTurnId = 0;
const recoveryAttempts = new Map<string, number>();
const subagentRuns = new Map<
	string,
	{ name: string; status: "running" | "done" | "failed"; action: string; startedAt: number }
>();

const SHIMMER_FRAMES = ["▁","▂","▃","▄","▅","▆","▇","█","▇","▆","▅","▄","▃","▂","▁"];
const COLD_START_TIMEOUT_MS = 5_000;
let coldStartTimer: ReturnType<typeof setTimeout> | null = null;
let hasReceivedContent = false;

function modelKey(provider: string, model: string): string {
	return `${provider}/${model}`;
}

function contentText(content: unknown): string {
	if (typeof content === "string") return content;
	if (!Array.isArray(content)) return "";
	return content
		.filter(
			(part): part is TextContent =>
				typeof part === "object" && part !== null && "type" in part && part.type === "text",
		)
		.map((part) => part.text)
		.join("");
}

function getAssistantText(message: AssistantMessage): string {
	return message.content
		.filter((part): part is TextContent => part.type === "text")
		.map((part) => part.text)
		.join("");
}

function getThinkingText(message: AssistantMessage): string | undefined {
	const thinking = message.content
		.filter((part) => part.type === "thinking")
		.map((part) => part.thinking)
		.join("\n")
		.trim();
	return thinking || undefined;
}

function hasToolCalls(message: AssistantMessage): boolean {
	return message.content.some((part) => part.type === "toolCall");
}

function fileHints(text: string): string[] {
	const matches = text.match(/(?:^|\s)([./~]?[A-Za-z0-9._@/-]+\.[A-Za-z0-9]{1,8})(?=\s|$|[:),])/g) ?? [];
	return [...new Set(matches.map((match) => match.trim()).filter((match) => match.length < 240))].slice(0, 12);
}

function memoryPath(cwd: string): string {
	return process.env.HAMR_MEMORY_DB || join(cwd, ".hamr", "memory.sqlite");
}

function getMemory(ctx: ExtensionContext): HolographicMemory | undefined {
	const path = memoryPath(ctx.cwd);
	if (memoryHandle?.path === path) return memoryHandle.memory;

	const Database = loadBetterSqlite3();
	if (!Database) return undefined;

	mkdirSync(dirname(path), { recursive: true });
	const db = new Database(path);
	db.pragma("journal_mode = WAL");
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
	memoryHandle = { path, memory: new HolographicMemory(db) };
	return memoryHandle.memory;
}

function storeMessage(ctx: ExtensionContext, message: AgentMessage): void {
	const memory = getMemory(ctx);
	if (!memory) return;

	if (message.role === "user") {
		const text = contentText(message.content);
		if (!text.trim()) return;
		memory.store({
			sessionId: ctx.sessionManager.getSessionId(),
			turnId: currentTurnId,
			role: "user",
			content: text,
			filePaths: fileHints(text),
			domainTags: ["hamr"],
		});
		return;
	}

	if (message.role === "assistant") {
		const text = getAssistantText(message as AssistantMessage);
		if (!text.trim()) return;
		memory.store({
			sessionId: ctx.sessionManager.getSessionId(),
			turnId: currentTurnId,
			role: "assistant",
			content: text,
			filePaths: fileHints(text),
			domainTags: ["hamr"],
		});
		return;
	}

	if (message.role === "toolResult") {
		const result = message as ToolResultMessage;
		const text = contentText(result.content);
		if (!text.trim()) return;
		memory.store({
			sessionId: ctx.sessionManager.getSessionId(),
			turnId: currentTurnId,
			role: "tool",
			toolName: result.toolName,
			content: text,
			filePaths: fileHints(text),
			domainTags: ["hamr"],
		});
	}
}

function parserFor(message: AssistantMessage, ctx: ExtensionContext): string {
	return (
		parserByModel.get(modelKey(message.provider, message.model)) ??
		(ctx.model ? parserByModel.get(modelKey(ctx.model.provider, ctx.model.id)) : undefined) ??
		detectParserId(message.model) ??
		(ctx.model ? detectParserId(ctx.model.id) : undefined) ??
		"generic"
	);
}

function repairLocalToolCalls(message: AssistantMessage, ctx: ExtensionContext): AssistantMessage | undefined {
	if (hasToolCalls(message)) return undefined;
	const text = getAssistantText(message);
	if (!text.trim()) return undefined;

	const parsed = parseModelOutput(text, parserFor(message, ctx), getThinkingText(message));
	if (parsed.toolCalls.length === 0) return undefined;

	const content: AssistantMessage["content"] = [];
	if (parsed.reasoning) {
		content.push({ type: "thinking", thinking: parsed.reasoning });
	}
	if (parsed.assistantText.trim()) {
		content.push({ type: "text", text: parsed.assistantText.trim() });
	}
	for (const call of parsed.toolCalls) {
		content.push({
			type: "toolCall",
			id: call.id,
			name: call.name,
			arguments: call.arguments,
		} satisfies ToolCall);
	}

	return {
		...message,
		content,
		stopReason: "toolUse",
		diagnostics: [
			...(message.diagnostics ?? []),
			...parsed.warnings.map((warning) => ({
				type: "hamr.tool_call_repair",
				timestamp: Date.now(),
				details: {
					source: warning.source,
					message: warning.message,
				},
			})),
		],
	};
}

function renderSubagentDashboard(): string[] {
	const runs = Array.from(subagentRuns.values());
	if (runs.length === 0) return [];
	const counts = runs.reduce(
		(acc, run) => {
			acc[run.status] += 1;
			return acc;
		},
		{ running: 0, done: 0, failed: 0 },
	);
	const lines = [
		`AGENTS ${runs.length} total · ${counts.running} running · ${counts.done} done · ${counts.failed} failed`,
	];
	for (const run of runs.slice(-6)) {
		const glyph = run.status === "running" ? "◌" : run.status === "done" ? "◆" : "x";
		const elapsed = Math.max(0, Math.round((Date.now() - run.startedAt) / 1000));
		lines.push(`${glyph} ${run.name} · ${run.status} · ${elapsed}s · ${run.action}`);
	}
	return lines;
}

function updateSubagentDashboard(ctx: ExtensionContext): void {
	if (ctx.mode !== "tui") return;
	const lines = renderSubagentDashboard();
	ctx.ui.setStatus("hamr.subagents", lines[0]);
	ctx.ui.setWidget("hamr.subagents", lines.length > 0 ? lines : undefined, { placement: "aboveEditor" });
}

function hasSubstantialContent(event: AssistantMessageEvent): boolean {
  switch (event.type) {
    case "start": case "done": case "error": return false;
    case "text_delta": case "thinking_delta": case "toolcall_delta":
      return event.delta.trim().length > 0;
    case "text_start": case "thinking_start": case "toolcall_start":
    case "text_end": case "thinking_end": case "toolcall_end": return true;
    default: return false;
  }
}

function registerMemoryTools(pi: Parameters<ExtensionFactory>[0]): void {
	pi.registerTool(
		defineTool({
			name: "search_memory",
			label: "Search memory",
			description: "Search Hamr's local FTS5 memory for prior turns, tool outputs, files, and handoff facts.",
			promptSnippet: "Use search_memory to recover relevant prior context without rereading the whole conversation.",
			parameters: Type.Object({
				query: Type.String({ description: "FTS5 query text, for example an error, file path, or feature name." }),
				limit: Type.Optional(Type.Number({ description: "Maximum results to return. Default 5." })),
			}),
			execute: async (_toolCallId, params, _signal, _onUpdate, ctx) => {
				const memory = getMemory(ctx);
				if (!memory) return { content: [{ type: "text", text: "FTS5 memory is unavailable." }], details: {} };
				const results = memory.searchWithSnippets(params.query, Math.min(params.limit ?? 5, 20));
				const text =
					results.length === 0
						? "No memory results."
						: results
								.map(
									(result, index) =>
										`${index + 1}. turn ${result.turnId} ${result.role}${result.toolName ? `/${result.toolName}` : ""}\n${result.snippet || result.content.slice(0, 500)}`,
								)
								.join("\n\n");
				return { content: [{ type: "text", text }], details: { count: results.length } };
			},
		}),
	);

	pi.registerTool(
		defineTool({
			name: "save_memory",
			label: "Save memory",
			description: "Save a durable fact, decision, or handoff note into Hamr's local FTS5 memory.",
			parameters: Type.Object({
				content: Type.String({ description: "The fact, decision, or handoff note to store." }),
				tags: Type.Optional(Type.Array(Type.String({ description: "Optional tags." }))),
			}),
			execute: async (_toolCallId, params, _signal, _onUpdate, ctx) => {
				const memory = getMemory(ctx);
				if (!memory) return { content: [{ type: "text", text: "FTS5 memory is unavailable." }], details: {} };
				memory.store({
					sessionId: ctx.sessionManager.getSessionId(),
					turnId: currentTurnId,
					role: "tool",
					toolName: "save_memory",
					content: params.content,
					filePaths: fileHints(params.content),
					domainTags: ["hamr", ...(params.tags ?? [])],
				});
				return { content: [{ type: "text", text: "Saved to Hamr memory." }], details: { tags: params.tags ?? [] } };
			},
		}),
	);

	pi.registerTool(
		defineTool({
			name: "handoff_memory",
			label: "Memory handoff",
			description: "Build a structured handoff manifest from Hamr's FTS5 memory for another agent or future turn.",
			parameters: Type.Object({}),
			execute: async (_toolCallId, _params, _signal, _onUpdate, ctx) => {
				const memory = getMemory(ctx);
				if (!memory) return { content: [{ type: "text", text: "FTS5 memory is unavailable." }], details: {} };
				const handoff = memory.handoff();
				const text = [
					`Session: ${handoff.sessionId || ctx.sessionManager.getSessionId()}`,
					`Turns: ${handoff.turnCount}, entries: ${handoff.entryCount}`,
					handoff.filesTouched.length > 0 ? `Files: ${handoff.filesTouched.join(", ")}` : undefined,
					handoff.domainTags.length > 0 ? `Tags: ${handoff.domainTags.join(", ")}` : undefined,
					handoff.suggestedSearchTerms.length > 0
						? `Search terms: ${handoff.suggestedSearchTerms.join(", ")}`
						: undefined,
					handoff.keyFindings.length > 0 ? `Findings:\n- ${handoff.keyFindings.join("\n- ")}` : undefined,
				]
					.filter((line): line is string => line !== undefined)
					.join("\n");
				return { content: [{ type: "text", text }], details: handoff };
			},
		}),
	);
}

function registerSubagentTool(pi: Parameters<ExtensionFactory>[0]): void {
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
			],
			executionMode: "sequential",
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
				const runId = `subagent-${Date.now()}-${subagentRuns.size + 1}`;
				subagentRuns.set(runId, {
					name: params.task.slice(0, 48),
					status: "running",
					action: params.mode ?? "read_only",
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
						extensionFactories: [hamrBuiltinExtension],
					});
					await resourceLoader.reload();
					const child = await createAgentSession({
						cwd: ctx.cwd,
						agentDir,
						model: ctx.model,
						thinkingLevel: pi.getThinkingLevel(),
						modelRegistry: ctx.modelRegistry,
						settingsManager,
						resourceLoader,
						sessionManager: SessionManager.inMemory(ctx.cwd),
						noTools: params.mode === "no_tools" ? "all" : undefined,
						tools: params.mode === "full" || params.mode === "no_tools" ? undefined : ["read", "grep", "find", "ls"],
					});
					if (signal?.aborted) {
						subagentRuns.set(runId, { ...subagentRuns.get(runId)!, status: "failed", action: "aborted" });
						updateSubagentDashboard(ctx);
						return { content: [{ type: "text", text: "Subagent aborted before start." }], details: { aborted: true } };
					}
					await child.session.prompt(prompt, { source: "extension" });
					const assistant = [...child.session.messages].reverse().find((message) => message.role === "assistant") as
						| AssistantMessage
						| undefined;
					const text = assistant
						? getAssistantText(assistant) || "(subagent returned no text)"
						: "(subagent returned no message)";
					memory?.store({
						sessionId: ctx.sessionManager.getSessionId(),
						turnId: currentTurnId,
						role: "tool",
						toolName: "delegate_subagent",
						content: `Task: ${params.task}\n\n${text}`,
						domainTags: ["hamr", "subagent"],
					});
					subagentRuns.set(runId, { ...subagentRuns.get(runId)!, status: "done", action: "handoff saved" });
					updateSubagentDashboard(ctx);
					pi.sendMessage(
						{
							customType: "hamr.subagent",
							content: `Subagent completed: ${params.task}`,
							display: true,
							details: { task: params.task, mode: params.mode ?? "read_only", result: text },
						},
						{ triggerTurn: false },
					);
					return {
						content: [{ type: "text", text }],
						details: { task: params.task, mode: params.mode ?? "read_only" },
					};
				} catch (error) {
					const message = error instanceof Error ? error.message : String(error);
					subagentRuns.set(runId, { ...subagentRuns.get(runId)!, status: "failed", action: message });
					updateSubagentDashboard(ctx);
					return {
						content: [{ type: "text", text: `Subagent failed: ${message}` }],
						details: { task: params.task, mode: params.mode ?? "read_only", error: message },
						isError: true,
					};
				}
			},
		}),
	);
}

async function registerHamrProviders(pi: Parameters<ExtensionFactory>[0], config: HamrStartupConfig): Promise<void> {
	const registrations = await buildHamrProviderRegistrations(config);
	for (const registration of registrations) {
		for (const [modelId, parserId] of registration.parserByModel) {
			parserByModel.set(modelKey(registration.name, modelId), parserId);
		}
		pi.registerProvider(registration.name, registration.config);
	}
}

async function showDashboard(pi: Parameters<ExtensionFactory>[0], ctx: ExtensionContext): Promise<void> {
  if (ctx.mode !== "tui") {
    ctx.ui.notify("Dashboard requires TUI mode", "error");
    return;
  }
  const allCommands = pi.getCommands();
  const commands = allCommands.map((c) => ({ name: c.name, description: c.description }));
  const model = ctx.model;
  const contextUsage = ctx.getContextUsage();
  const data: DashboardData = {
    commands,
    provider: model?.provider ?? "N/A",
    modelName: model?.name ?? model?.id ?? "N/A",
    thinkingLevel: pi.getThinkingLevel(),
    sessionFile: ctx.sessionManager.getSessionFile() ?? "N/A",
    messageCount: ctx.sessionManager.getEntries().filter((e) => e.type === "message").length,
    contextPercent: contextUsage?.percent ?? null,
    contextWindow: contextUsage?.contextWindow ?? 0,
  };
  await ctx.ui.custom<undefined>(
    (_tui, theme, _kb, done) => new DashboardComponent(theme, done, data),
    { overlay: true, overlayOptions: { width: "60%", maxHeight: "90%", anchor: "center", margin: { top: 1 } } },
  );
}

export const hamrBuiltinExtension: ExtensionFactory = async (pi) => {
	const config = loadHamrStartupConfig(process.cwd());
	await registerHamrProviders(pi, config);
	registerMemoryTools(pi);
	registerSubagentTool(pi);

	// Cold-start detection
	function clearColdStartTimer(): void {
		if (coldStartTimer !== null) { clearTimeout(coldStartTimer); coldStartTimer = null; }
	}

	pi.on("message_start", (_event, ctx) => {
		if (_event.message.role === "assistant") {
			hasReceivedContent = false;
			clearColdStartTimer();
			coldStartTimer = setTimeout(() => {
				coldStartTimer = null;
				if (!hasReceivedContent) {
					ctx.ui.setStatus("hamr-cold-start", ctx.ui.theme.fg("warning", "Cold starting..."));
				}
			}, COLD_START_TIMEOUT_MS);
		}
	});

	pi.on("message_update", (_event, ctx) => {
		if (_event.message.role !== "assistant") return;
		if (!hasReceivedContent && hasSubstantialContent(_event.assistantMessageEvent)) {
			hasReceivedContent = true;
			clearColdStartTimer();
			ctx.ui.setStatus("hamr-cold-start", undefined);
			ctx.ui.setWorkingIndicator({ frames: SHIMMER_FRAMES, intervalMs: 120 });
		}
	});

	pi.on("session_shutdown", () => {
		clearColdStartTimer();
		hasReceivedContent = false;
	});

	// Dashboard
	pi.registerCommand("dashboard", {
		description: "Open the Hamr dashboard overlay",
		handler: async (_args, ctx) => { await showDashboard(pi, ctx); },
	});

	pi.registerShortcut(Key.ctrl("o"), {
		description: "Open Hamr dashboard",
		handler: async (ctx) => { await showDashboard(pi, ctx); },
	});

	pi.on("message_end", (event, ctx) => {
		clearColdStartTimer();
		hasReceivedContent = false;
		storeMessage(ctx, event.message);
		if (event.message.role !== "assistant") return;
		const replacement = repairLocalToolCalls(event.message as AssistantMessage, ctx);
		return replacement ? { message: replacement } : undefined;
	});

	pi.on("context", (event, ctx) => {
		const index = getMemory(ctx)?.buildMemoryIndex();
		if (!index) return;
		return {
			messages: [
				{
					role: "user",
					content: `Hamr FTS5 memory index is available. Use search_memory for details before rereading broad history.\n${index}`,
					timestamp: Date.now(),
				},
				...event.messages,
			],
		};
	});

	pi.on("turn_end", (event, ctx) => {
		currentTurnId = event.turnIndex + 1;
		const message = event.message;
		if (message.role !== "assistant") return;
		const assistant = message as AssistantMessage;
		const sessionId = ctx.sessionManager.getSessionId();
		const attempts = recoveryAttempts.get(sessionId) ?? 0;
		if (attempts >= 2) return;
		const text = getAssistantText(assistant).trim();
		if (assistant.stopReason === "error" || (!text && !hasToolCalls(assistant))) {
			recoveryAttempts.set(sessionId, attempts + 1);
			pi.sendUserMessage(
				assistant.stopReason === "error"
					? `Hamr recovery: the last model response failed (${assistant.errorMessage ?? "unknown error"}). Try again with a simpler, valid tool call or explain the blocker.`
					: "Hamr recovery: your last response was empty. Continue with a valid answer or a valid tool call.",
				{ deliverAs: "followUp" },
			);
		} else {
			recoveryAttempts.delete(sessionId);
		}
	});
};

export { getHamrDefaultModel, loadHamrStartupConfig };
