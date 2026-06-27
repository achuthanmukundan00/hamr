/**
 * HolographicMemory — SQLite FTS5-backed semantic memory for agent context.
 *
 * Architecture:
 *   Every turn → INSERT into FTS5 (fire-and-forget, non-blocking)
 *   Agent needs history → search("error from 5 turns ago") → relevant rows
 *   Context exhausted → handoff() → structured manifest for child agent
 *
 * This is the architectural differentiator from the SOTA review:
 * zero tokens burned, zero information loss, agent queries what it needs.
 *
 * Shares the SQLite connection with EventStore. If SQLite is unavailable,
 * all operations are safe no-ops.
 */
import type { Database } from "../store/sqlite-loader.ts";
export interface MemoryEntry {
    sessionId: string;
    turnId: number;
    role: "user" | "assistant" | "tool";
    toolName?: string;
    filePaths?: string[];
    content: string;
    /** Product-domain tags for cross-product memory filtering (e.g. 'autocareer', 'wytos'). */
    domainTags?: string[];
}
export interface MemorySearchResult {
    turnId: number;
    sessionId: string;
    role: string;
    toolName: string | null;
    filePaths: string | null;
    content: string;
    /** Product-domain tags stored with the entry (e.g. 'autocareer', 'wytos'). */
    domainTags: string | null;
    /** FTS5 rank (lower = more relevant). */
    rank: number;
}
export interface HandoffManifest {
    sessionId: string;
    /** Last N turns summarized as key findings. */
    keyFindings: string[];
    /** Files that were read or changed. */
    filesTouched: string[];
    /** Suggested FTS5 search terms for the next agent. */
    suggestedSearchTerms: string[];
    /** Number of turns stored. */
    turnCount: number;
    /** Total entries in memory. */
    entryCount: number;
    /** Product-domain context tags observed in recent memory entries. */
    domainTags: string[];
}
/**
 * Sanitize a user query for safe FTS5 MATCH usage.
 *
 * FTS5 MATCH expects a boolean expression: bare words, "phrase queries",
 * prefix* terms, AND/OR/NOT, and (grouping). Dangerous characters (#, @, etc.)
 * are stripped. Hyphens in terms (e.g. "hamr-browser") are preserved by
 * double-quoting each token that contains them, since bare `-` is a column
 * filter in FTS5. Path-like tokens (containing / or .) are also double-quoted
 * to avoid being split into separate FTS5 tokens.
 *
 * Falls back to the original query (with only null bytes and unprintables
 * stripped) if tokenization produces nothing useful.
 */
export declare function sanitizeFts5Query(query: string): string;
/**
 * @public
 */
export declare class HolographicMemory {
    private db;
    private insertStmt;
    private statsStmt;
    private recentEntriesStmt;
    private searchStmt;
    private searchSnippetStmt;
    private handoffCountsStmt;
    private handoffRecentEntriesStmt;
    private latestByTagStmt;
    private _suggestedTermsCache;
    /** Count of store errors since construction. Non-zero means FTS5 is silently failing. */
    storeErrorCount: number;
    private _storeErrorWarned;
    constructor(db: Database.Database | null);
    get isAvailable(): boolean;
    /** Check if the given session has any stored entries. */
    hasSessionEntries(sessionId: string): boolean;
    /**
     * Store a memory entry in FTS5.
     * Fire-and-forget — errors are caught, never thrown.
     * Target: <5ms per store.
     */
    store(entry: MemoryEntry): void;
    /**
     * Full-text search over stored memory entries.
     *
     * Uses FTS5 with Porter stemming — "login form" matches "login forms".
     * Results ranked by FTS5 relevance (bm25).
     *
     * @param query - FTS5 search query (supports AND, OR, NOT, prefix*).
     * @param limit - Maximum results to return (default 10).
     */
    search(query: string, limit?: number): MemorySearchResult[];
    /**
     * Search with snippet context (FTS5 snippet()).
     * Returns matching fragments with surrounding text for readability.
     */
    searchWithSnippets(query: string, limit?: number): Array<MemorySearchResult & {
        snippet: string;
    }>;
    /**
     * Fetch the most recently stored entry carrying the given domain tag.
     *
     * Used to surface a survival manifest (domainTag "survival") prominently on
     * resume. Returns null when no such entry exists or memory is unavailable.
     */
    getLatestByDomainTag(tag: string, sessionId: string): MemorySearchResult | null;
    /**
     * Generate a structured handoff manifest for context exhaustion scenarios.
     *
     * Contains:
     *   - Key findings from recent turns
     *   - Files touched (read or changed)
     *   - Suggested search terms for the next agent
     *   - Turn/entry counts
     */
    handoff(sessionId: string): HandoffManifest;
    /**
     * Generate suggested FTS5 search terms from recent memory.
     *
     * Extracts:
     *   - File paths mentioned in content
     *   - Error/diagnostic keywords
     *   - Tool names from recent tool calls
     */
    getSuggestedSearchTerms(): string[];
    /**
     * Compute suggested search terms from already-fetched entries.
     * Extracted so buildMemoryIndex() and handoff() can reuse their
     * single recent-entries fetch without issuing duplicate queries.
     */
    private _computeSuggestedTerms;
    /**
     * Build a compact, token-efficient index of what's in memory.
     *
     * Injected into every model request so the agent knows what's searchable
     * without burning context on the full content. Target: ~30-50 tokens.
     *
     * Returns null if memory is empty or unavailable.
     */
    buildMemoryIndex(): string | null;
}
//# sourceMappingURL=HolographicMemory.d.ts.map