import type { ExtensionFactory } from "../../core/extensions/types.ts";
import type { HolographicMemory } from "../memory/HolographicMemory.ts";
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
export declare class HandoffManager {
    private depth;
    constructor(initialDepth?: number);
    get currentDepth(): number;
    canHandoff(): boolean;
    /**
     * Generate a structured handoff manifest combining FTS5 memory data
     * with session-level metadata.
     */
    createHandoff(sessionId: string, memory: HolographicMemory | undefined, options: HandoffOptions): StructuredHandoff;
    /** Increment depth for a child handoff. Returns a new HandoffManager for the child. */
    forChild(): HandoffManager;
}
export declare function registerHandoffTool(pi: Parameters<ExtensionFactory>[0]): void;
//# sourceMappingURL=HandoffManager.d.ts.map