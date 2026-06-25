import type { AgentMessage } from "@hamr/agent";
import type { ExtensionFactory } from "../../core/extensions/types.ts";
import type { HolographicMemory } from "../memory/HolographicMemory.ts";
/**
 * Extract meaningful search terms from a user message for FTS5 retrieval.
 *
 * Filters out greetings, stop words, and very short tokens so that generic
 * messages like "hi" produce no terms — preventing irrelevant past context
 * from hijacking the current turn. Longer, topical messages naturally yield
 * terms that match stored memory entries.
 *
 * Returns an empty array when the message is too short or entirely generic.
 */
export declare function extractMessageSearchTerms(message: string): string[];
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
/** Pure inputs for computeMemoryInjection — no external state or side effects. */
export interface MemoryInjectionInput {
    memory: HolographicMemory;
    messages: AgentMessage[];
    survivalManifest: string | null;
    /** Policy overrides (derived from selectCompactionPolicy in the handler). */
    searchTermLimit: number;
    resultsPerTerm: number;
    snippetChars: number;
    /** Token budget cap for auto-results (character count). Passed explicitly for testability. */
    charBudget: number;
    /** Optional fact-store status line appended to content and included in hash. */
    factStoreLine?: string;
}
/** Result of a successful memory injection computation. */
export interface MemoryInjectionResult {
    message: {
        role: "user";
        content: string;
        timestamp: number;
    };
    /** Content hash for de-duplication tracking by the caller. */
    contextHash: string;
}
/**
 * Compute whether memory context should be injected for the current turn.
 *
 * Pure function — no side effects. The caller manages session state
 * (hashes, one-shot flags, etc.).
 *
 * Query-triggered recall: search terms are derived from the current user
 * message, not from past-entry word frequency. Generic messages like "hi"
 * produce no terms → null. Topical messages produce terms → FTS5 search
 * → injection. A survival manifest is always surfaced when present.
 *
 * Returns null when there is nothing worth injecting.
 */
export declare function computeMemoryInjection(input: MemoryInjectionInput): MemoryInjectionResult | null;
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