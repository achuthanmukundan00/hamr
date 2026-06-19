import { mkdirSync } from "node:fs";
import { dirname, join } from "node:path";
import type { AgentMessage } from "@hamr/agent";
import type { AssistantMessage, ToolCall, ToolResultMessage } from "@hamr/ai";
import { Text } from "@hamr/tui";
import { Type } from "typebox";
import type { ExtensionContext, ExtensionFactory } from "../core/extensions/types.ts";
import { defineTool } from "../core/extensions/types.ts";
import { contentText, fileHints, getAssistantText } from "./helpers.ts";
import { stripFtsMarks } from "./memory/fts-marks.ts";
import { HolographicMemory } from "./memory/HolographicMemory.ts";
import { loadBetterSqlite3 } from "./store/sqlite-loader.ts";

export type MemoryHandle = {
	path: string;
	memory: HolographicMemory;
};

let memoryHandle: MemoryHandle | undefined;
let currentTurnId = 0;

export function setCurrentTurnId(id: number): void {
	currentTurnId = id;
}

export function getCurrentTurnId(): number {
	return currentTurnId;
}

function memoryPath(cwd: string): string {
	return process.env.HAMR_MEMORY_DB || join(cwd, ".hamr", "memory.sqlite");
}

export function getMemory(ctx: ExtensionContext): HolographicMemory | undefined {
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

export function buildAssistantMemoryContent(message: AssistantMessage): string {
	const parts: string[] = [];
	const text = sanitizeMemoryTranscriptText(getAssistantText(message)).trim();
	if (text) parts.push(text);

	for (const block of message.content) {
		if (block.type !== "toolCall") continue;
		const call = block as ToolCall;
		parts.push(`tool_call ${call.name} ${JSON.stringify(call.arguments)}`);
	}

	return parts.join("\n");
}

export function sanitizeMemoryTranscriptText(text: string): string {
	return stripFtsMarks(text)
		.replace(/<think\b[^>]*>[\s\S]*?<\/think>/gi, "")
		.replace(/<thinking\b[^>]*>[\s\S]*?<\/thinking>/gi, "")
		.replace(/<function=([^>\s]+)>/gi, "tool_call $1 ")
		.replace(/<parameter=([^>\s]+)>/gi, "$1=")
		.replace(/<\/parameter>/gi, " ")
		.replace(/<\/function>/gi, " ")
		.replace(/<\/?tool_call>/gi, " ")
		.replace(/<\/?function_calls>/gi, " ")
		.replace(/<\/?function_call>/gi, " ")
		.replace(/[ \t]+\n/g, "\n")
		.replace(/\n{3,}/g, "\n\n")
		.trim();
}

export function storeMessage(ctx: ExtensionContext, message: AgentMessage): void {
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
		const text = buildAssistantMemoryContent(message as AssistantMessage);
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

export function registerMemoryTools(pi: Parameters<ExtensionFactory>[0]): void {
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
			renderResult: (result, options, theme) => {
				const details = result.details as any;
				const count = details?.count ?? 0;
				if (!options.expanded) {
					return new Text(theme.fg("dim", `${count} result${count !== 1 ? "s" : ""}`), 0, 0);
				}
				const output = contentText(result.content);
				return new Text(theme.fg("toolOutput", output), 0, 0);
			},
			execute: async (_toolCallId, params, _signal, _onUpdate, ctx) => {
				const memory = getMemory(ctx);
				if (!memory) return { content: [{ type: "text", text: "FTS5 memory is unavailable." }], details: {} };
				const results = memory.searchWithSnippets(params.query, Math.min(params.limit ?? 5, 20));
				const text =
					results.length === 0
						? "No memory results."
						: results
								.map((result, index) => {
									const excerpt = sanitizeMemoryTranscriptText(result.snippet || result.content.slice(0, 500));
									return `${index + 1}. turn ${result.turnId} ${result.role}${result.toolName ? `/${result.toolName}` : ""}\n${excerpt}`;
								})
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
				const tags = params.tags ?? [];
				const tagLine = tags.length > 0 ? `\nTags: ${tags.join(", ")}` : "";
				const preview = params.content.length > 300 ? `${params.content.slice(0, 300)}…` : params.content;
				return {
					content: [{ type: "text", text: `📝 Saved to Hamr memory:\n${preview}${tagLine}` }],
					details: { tags, storedLength: params.content.length },
				};
			},
		}),
	);

	pi.registerTool(
		defineTool({
			name: "handoff_memory",
			label: "Memory handoff",
			description: "Build a structured handoff manifest from Hamr's FTS5 memory for another agent or future turn.",
			parameters: Type.Object({}),
			renderResult: (result, options, theme) => {
				const handoff = result.details as any;
				if (!options.expanded) {
					const count = handoff?.entryCount ?? 0;
					return new Text(
						theme.fg("dim", `${count} entr${count !== 1 ? "ies" : "y"} · ${handoff?.turnCount ?? 0} turns`),
						0,
						0,
					);
				}
				const output = contentText(result.content);
				return new Text(theme.fg("toolOutput", output), 0, 0);
			},
			execute: async (_toolCallId, _params, _signal, _onUpdate, ctx) => {
				const memory = getMemory(ctx);
				if (!memory) return { content: [{ type: "text", text: "FTS5 memory is unavailable." }], details: {} };
				const handoff = memory.handoff();
				const lines = [
					`📋 Hamr handoff manifest`,
					`Session: ${handoff.sessionId || ctx.sessionManager.getSessionId()}`,
					`Turns: ${handoff.turnCount}, entries: ${handoff.entryCount}`,
				];
				if (handoff.filesTouched.length > 0) lines.push(`Files touched: ${handoff.filesTouched.join(", ")}`);
				if (handoff.domainTags.length > 0) lines.push(`Tags: ${handoff.domainTags.join(", ")}`);
				if (handoff.suggestedSearchTerms.length > 0)
					lines.push(`Suggested searches: ${handoff.suggestedSearchTerms.join(", ")}`);
				if (handoff.keyFindings.length > 0) {
					lines.push(`Key findings:\n${handoff.keyFindings.map((f) => `  - ${f}`).join("\n")}`);
				}
				return { content: [{ type: "text", text: lines.join("\n") }], details: handoff };
			},
		}),
	);
}
