/**
 * HandoffManager — structured context handoff over FTS5 memory.
 *
 * Tracks handoff depth (max 3), generates structured manifests consumable
 * by subagents and future turns. Lean — uses HolographicMemory for all storage.
 */
import { Type } from "typebox";
import type { ExtensionFactory } from "../../core/extensions/types.ts";
import { defineTool } from "../../core/extensions/types.ts";
import type { HandoffManifest, HolographicMemory } from "../memory/HolographicMemory.ts";
import { getMemory } from "../memory.ts";

export interface HandoffOptions {
	/** Why the handoff was triggered. */
	reason: "context_exhaustion" | "task_delegation" | "explicit";
	/** The task being handed off. */
	task: string;
	/** Files modified in this session. */
	filesChanged?: string[];
	/** Files read/inspected in this session. */
	filesRead?: string[];
	/** Work still pending. */
	pendingWork?: string[];
	/** Orchestration context (subtask id, plan id, sibling summaries). */
	orchestrationContext?: string;
}

export interface StructuredHandoff {
	handoffId: string;
	parentSessionId: string;
	reason: HandoffOptions["reason"];
	task: string;
	status: string;
	keyFindings: string[];
	filesChanged: string[];
	filesRead: string[];
	pendingWork: string[];
	suggestedSearchTerms: string[];
	depth: number;
	createdAt: string;
	orchestrationContext?: string;
}

const MAX_DEPTH = 3;

export class HandoffManager {
	private depth = 0;

	constructor(initialDepth = 0) {
		this.depth = Math.min(initialDepth, MAX_DEPTH);
	}

	get currentDepth(): number {
		return this.depth;
	}

	canHandoff(): boolean {
		return this.depth < MAX_DEPTH;
	}

	/**
	 * Generate a structured handoff manifest combining FTS5 memory data
	 * with session-level metadata.
	 */
	createHandoff(sessionId: string, memory: HolographicMemory | undefined, options: HandoffOptions): StructuredHandoff {
		const memoryManifest: HandoffManifest | undefined = memory?.handoff(sessionId);
		const handoffId = `handoff-${Date.now()}-${this.depth}`;

		// Merge memory-derived findings with caller-supplied context
		const keyFindings = [
			...(memoryManifest?.keyFindings ?? []),
			...(options.pendingWork ?? []).map((w) => `pending: ${w}`),
		].slice(0, 15);

		const filesChanged = [...new Set([...(options.filesChanged ?? []), ...(memoryManifest?.filesTouched ?? [])])].slice(
			0,
			30,
		);

		const filesRead = [...new Set(options.filesRead ?? [])].slice(0, 30);

		const statusParts: string[] = [];
		if (memoryManifest) {
			statusParts.push(`${memoryManifest.entryCount} memory entries across ${memoryManifest.turnCount} turns`);
		}
		if (filesChanged.length > 0) {
			statusParts.push(`${filesChanged.length} files changed`);
		}
		if (filesRead.length > 0) {
			statusParts.push(`${filesRead.length} files read`);
		}

		return {
			handoffId,
			parentSessionId: sessionId,
			reason: options.reason,
			task: options.task,
			status: statusParts.length > 0 ? statusParts.join("; ") : "no prior state",
			keyFindings,
			filesChanged,
			filesRead,
			pendingWork: options.pendingWork ?? [],
			suggestedSearchTerms: memoryManifest?.suggestedSearchTerms ?? [],
			depth: this.depth,
			createdAt: new Date().toISOString(),
			orchestrationContext: options.orchestrationContext,
		};
	}

	/** Increment depth for a child handoff. Returns a new HandoffManager for the child. */
	forChild(): HandoffManager {
		return new HandoffManager(this.depth + 1);
	}
}

// ── Per-session HandoffManager instances ──────────────────────────────────────

const managersBySession = new Map<string, HandoffManager>();

function getManager(sessionId: string): HandoffManager {
	let manager = managersBySession.get(sessionId);
	if (!manager) {
		manager = new HandoffManager();
		managersBySession.set(sessionId, manager);
	}
	return manager;
}

// ── Tool registration ────────────────────────────────────────────────────────

export function registerHandoffTool(pi: Parameters<ExtensionFactory>[0]): void {
	pi.registerTool(
		defineTool({
			name: "create_handoff",
			label: "Create handoff",
			description:
				"Create a structured handoff manifest from FTS5 memory. Consumable by subagents for context inheritance. Tracks depth (max 3).",
			promptSnippet: "Use create_handoff to checkpoint state before spawning subagents or when context is exhausted.",
			parameters: Type.Object({
				task: Type.String({ description: "The task being handed off." }),
				reason: Type.Optional(
					Type.Union([Type.Literal("context_exhaustion"), Type.Literal("task_delegation"), Type.Literal("explicit")], {
						description: "Why the handoff was triggered. Defaults to explicit.",
					}),
				),
				filesChanged: Type.Optional(Type.Array(Type.String(), { description: "Files modified in this session." })),
				filesRead: Type.Optional(Type.Array(Type.String(), { description: "Files read/inspected in this session." })),
				pendingWork: Type.Optional(Type.Array(Type.String(), { description: "Work still pending." })),
			}),
			execute: async (_toolCallId, params, _signal, _onUpdate, ctx) => {
				const sessionId = ctx.sessionManager.getSessionId();
				const manager = getManager(sessionId);
				if (!manager.canHandoff()) {
					return {
						content: [{ type: "text", text: `Handoff depth limit reached (max ${3}). Cannot create another handoff.` }],
						details: { depth: manager.currentDepth },
						isError: true,
					};
				}

				const memory = getMemory(ctx);
				const handoff = manager.createHandoff(sessionId, memory, {
					reason: params.reason ?? "explicit",
					task: params.task,
					filesChanged: params.filesChanged,
					filesRead: params.filesRead,
					pendingWork: params.pendingWork,
				});

				// Store the handoff in FTS5 memory for future search
				memory?.store({
					sessionId,
					turnId: 0,
					role: "tool",
					toolName: "create_handoff",
					content: JSON.stringify(handoff, null, 2),
					domainTags: ["hamr", "handoff"],
				});

				const lines = [
					`📋 Handoff manifest (depth ${handoff.depth}/3)`,
					`ID: ${handoff.handoffId}`,
					`Task: ${handoff.task}`,
					`Status: ${handoff.status}`,
				];
				if (handoff.keyFindings.length > 0) {
					lines.push(`\nKey findings:\n${handoff.keyFindings.map((f) => `  - ${f}`).join("\n")}`);
				}
				if (handoff.filesChanged.length > 0) {
					lines.push(`\nFiles changed: ${handoff.filesChanged.join(", ")}`);
				}
				if (handoff.pendingWork.length > 0) {
					lines.push(`\nPending:\n${handoff.pendingWork.map((w) => `  - ${w}`).join("\n")}`);
				}
				if (handoff.suggestedSearchTerms.length > 0) {
					lines.push(`\nSearch terms: ${handoff.suggestedSearchTerms.join(", ")}`);
				}

				// Advance depth for next handoff
				managersBySession.set(sessionId, manager.forChild());

				return {
					content: [{ type: "text", text: lines.join("\n") }],
					details: handoff,
				};
			},
		}),
	);
}
