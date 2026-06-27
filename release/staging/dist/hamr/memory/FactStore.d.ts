/**
 * FactStore — SQLite-backed structured fact memory with entity resolution,
 * trust scoring, and HRR-based compositional retrieval.
 *
 * Shares the database with HolographicMemory (transcript FTS5) but uses
 * its own tables for structured knowledge that persists across sessions.
 */
type Stmt = {
    run(params: Record<string, unknown>): {
        lastInsertRowid: number | bigint;
    };
    all(params: Record<string, unknown>): unknown[];
    get(params: Record<string, unknown>): unknown | undefined;
};
export interface FactEntry {
    factId: number;
    content: string;
    tags: string;
    trustScore: number;
    retrievalCount: number;
    helpfulCount: number;
    createdAt: string;
    updatedAt: string;
}
export interface FactWithScore extends FactEntry {
    score?: number;
    entities?: string[];
}
/**
 * Extract entity candidates from text using simple regex rules.
 *
 * Rules applied (in order):
 * 1. Capitalized multi-word phrases  e.g. "John Doe"
 * 2. Double-quoted terms             e.g. "Python"
 * 3. Single-quoted terms             e.g. 'pytest'
 * 4. Backtick-quoted terms           e.g. `search_memory`
 * 5. AKA patterns                    e.g. "Guido aka BDFL" → two entities
 *
 * Returns a deduplicated list preserving first-seen order.
 */
export declare function extractEntities(text: string): string[];
export declare class FactStore {
    private db;
    isAvailable: boolean;
    private insertFactStmt;
    private searchFtsStmt;
    private getFactByIdStmt;
    private resolveEntityStmt;
    private resolveEntityInsertStmt;
    private linkFactEntityStmt;
    private getEntitiesStmt;
    private deleteFactEntitiesStmt;
    private listAllFactsStmt;
    private listRecentFactsStmt;
    constructor(db: {
        exec(sql: string): void;
        prepare(sql: string): Stmt;
    } | null);
    static create(db: {
        exec(sql: string): void;
        prepare(sql: string): Stmt;
    } | null): FactStore;
    addFact(content: string, tags: string): number | null;
    searchFacts(query: string, limit?: number): FactWithScore[];
    getFact(factId: number): FactWithScore | null;
    /**
     * Return recent durable facts for recall/continuation prefetch.
     * This is intentionally separate from searchFacts("*"), which sorts by trust.
     */
    listRecentFacts(limit?: number, minTrust?: number): FactWithScore[];
    /**
     * Get entities linked to a fact. Returns entity names.
     */
    getFactEntities(factId: number): string[];
    /**
     * Probe for facts about a specific entity (case-insensitive match).
     * Uses the fact_entities junction table. Falls back to FTS5 search
     * if the entity has no linked facts.
     */
    probe(entity: string, limit?: number): FactWithScore[];
    /**
     * Discover facts that share entities with the given entity.
     * First finds facts directly linked to the entity, then finds
     * other facts that share at least one entity with those.
     * Falls back to FTS5 search if no structured links exist.
     */
    related(entity: string, limit?: number): FactWithScore[];
    reason(entities: string[], limit?: number): FactWithScore[];
    recordFeedback(factId: number, helpful: boolean): {
        oldTrust: number;
        newTrust: number;
    } | false;
    getFactCount(): number;
    dispose(): void;
    private _rowToFact;
    private _entitySearch;
    /** Find an existing entity by case-insensitive name match, or create one. */
    private _resolveEntity;
    private _linkFactEntity;
}
export {};
//# sourceMappingURL=FactStore.d.ts.map