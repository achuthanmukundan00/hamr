import type { AgentMessage } from "@hamr/agent";
import type { ExtensionFactory } from "../../core/extensions/types.ts";
import type { FactWithScore } from "../memory/FactStore.ts";
import type { MemorySearchResult } from "../memory/HolographicMemory.ts";
/**
 * De-duplicate auto-results against existing context messages.
 * Removes result lines whose core content already appears in any existing message.
 */
export declare function deduplicateResults(autoResults: string[], existingMessages: unknown[]): string[];
/**
 * Apply token budget cap to auto-results.
 * Truncates from the end, keeping the most relevant (first) results.
 * Preserves search header lines.
 */
export declare function applyTokenBudget(autoResults: string[], charBudget: number): string[];
/**
 * Builds the user message injected into context from FTS5 auto-retrieval.
 *
 * A survival manifest (from a prior local-model compaction) is surfaced first
 * and prominently — it is the resumed instance's primary orientation, not a
 * generic search hit. Returns null only when there is nothing worth injecting.
 */
export declare function buildMemoryContextMessage(autoResults: string[], index: string, options?: {
    survivalManifest?: string | null;
    timestamp?: number;
}): {
    role: "user";
    content: string;
    timestamp: number;
} | null;
export type MemoryPrefetchReason = "explicit-recall" | "continuation";
export interface MemoryPrefetchPayload {
    reason: MemoryPrefetchReason;
    latestUserText: string;
    queries: string[];
    facts: FactWithScore[];
    transcriptResults: Array<MemorySearchResult & {
        snippet?: string;
    }>;
    timestamp?: number;
}
export declare function classifyMemoryPrefetchPrompt(prompt: string): MemoryPrefetchReason | null;
export declare function buildMemoryPrefetchQueries(prompt: string, reason: MemoryPrefetchReason): string[];
export declare function buildMemoryPrefetchContextMessage(payload: MemoryPrefetchPayload): {
    role: "user";
    content: string;
    timestamp: number;
} | null;
export type LocalCompactionTier = "cloud" | "local-131k" | "local-64k" | "local-32k" | "local-16k";
export interface LocalCompactionPolicy {
    tier: LocalCompactionTier;
    contextWindow: number;
    keyLimit: number;
    searchTermLimit: number;
    resultsPerTerm: number;
    snippetChars: number;
    instructions: string;
}
export declare function selectCompactionPolicy(options: {
    cloud: boolean;
    contextWindow?: number;
}): LocalCompactionPolicy;
/**
 * The survival manifest is NOT a summary — it is a small map back into FTS5 for
 * a local/relay model that cannot afford an LLM compaction call near its limit.
 * Its job is to carry the few things a cold-resumed instance can't reconstruct
 * by searching: the verbatim task, ground-truth status, the planned next action,
 * and a handful of specific FTS5 keys that each recover something important.
 */
export interface SurvivalData {
    /** Tier policy selected for this compaction. */
    tier: LocalCompactionTier;
    /** Context window used to select the tier. */
    contextWindow: number;
    /** Verbatim goal — can't be searched for because the resumed instance doesn't know the words. */
    task: string;
    /** Ground truth right now: files modified, last command + result, branch. */
    status: string[];
    /** The next concrete action that was planned (lives only in the last assistant message). */
    next: string;
    /** 4-8 specific FTS5 search terms that each recover something important. */
    keys: string[];
    /** Files and identifiers that explain where the keys came from. */
    provenance: string[];
    /** Recovery guidance tuned for the selected tier. */
    instructions: string;
}
export declare function extractSurvivalData(messages: AgentMessage[], policy?: LocalCompactionPolicy): SurvivalData;
export declare function formatSurvivalManifest(data: SurvivalData): string;
/** Build the survival manifest string for a set of messages about to be discarded. */
export declare function buildSurvivalManifest(messages: AgentMessage[], policy?: LocalCompactionPolicy): string;
/**
 * Memory extension: FTS5 memory tools (search/save/handoff), message storage,
 * a two-path compaction strategy, and the turn counter. Orthogonal to session
 * topology — purely about persistence.
 */
export declare const hamrMemoryExtension: ExtensionFactory;
//# sourceMappingURL=memory.d.ts.map