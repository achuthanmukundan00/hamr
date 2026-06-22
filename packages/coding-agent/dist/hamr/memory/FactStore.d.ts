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
export declare class FactStore {
    private db;
    isAvailable: boolean;
    private insertFactStmt;
    private searchFtsStmt;
    private getFactByIdStmt;
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
    probe(entity: string, limit?: number): FactWithScore[];
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
}
export {};
//# sourceMappingURL=FactStore.d.ts.map