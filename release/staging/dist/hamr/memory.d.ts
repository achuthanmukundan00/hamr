import type { AgentMessage } from "@hamr/agent";
import type { AssistantMessage } from "@hamr/ai";
import type { ExtensionContext, ExtensionFactory } from "../core/extensions/types.ts";
import { FactStore } from "./memory/FactStore.ts";
import { HolographicMemory } from "./memory/HolographicMemory.ts";
export type MemoryHandle = {
    path: string;
    memory: HolographicMemory;
    factStore: FactStore;
};
export declare function setCurrentTurnId(id: number): void;
export declare function getCurrentTurnId(): number;
export declare function getMemory(ctx: ExtensionContext): HolographicMemory | undefined;
/** Get the cross-session structured FactStore (entity resolution, trust scoring, HRR). */
export declare function getFactStore(ctx: ExtensionContext): FactStore | undefined;
export declare function buildAssistantMemoryContent(message: AssistantMessage): string;
export declare function sanitizeMemoryTranscriptText(text: string): string;
export declare function storeMessage(ctx: ExtensionContext, message: AgentMessage): void;
export declare function registerMemoryTools(pi: Parameters<ExtensionFactory>[0]): void;
/**
 * Register structured fact store tools (fact_store and fact_feedback).
 * The fact store provides cross-session durable knowledge with entity
 * resolution, trust scoring, and HRR-based compositional queries.
 */
export declare function registerFactStoreTools(pi: Parameters<ExtensionFactory>[0]): void;
//# sourceMappingURL=memory.d.ts.map