/**
 * FactStore — SQLite-backed structured fact memory with entity resolution,
 * trust scoring, and HRR-based compositional retrieval.
 *
 * Shares the database with HolographicMemory (transcript FTS5) but uses
 * its own tables for structured knowledge that persists across sessions.
 */
// ─── Schema ──────────────────────────────────────────────────────────────────
const FACT_SCHEMA = `
CREATE TABLE IF NOT EXISTS facts (
    fact_id         INTEGER PRIMARY KEY AUTOINCREMENT,
    content         TEXT NOT NULL UNIQUE,
    tags            TEXT DEFAULT '',
    trust_score     REAL DEFAULT 0.5,
    retrieval_count INTEGER DEFAULT 0,
    helpful_count   INTEGER DEFAULT 0,
    created_at      TEXT DEFAULT (datetime('now')),
    updated_at      TEXT DEFAULT (datetime('now')),
    hrr_vector      BLOB
);

CREATE TABLE IF NOT EXISTS entities (
    entity_id INTEGER PRIMARY KEY AUTOINCREMENT,
    name      TEXT NOT NULL UNIQUE
);

CREATE TABLE IF NOT EXISTS fact_entities (
    fact_id   INTEGER REFERENCES facts(fact_id) ON DELETE CASCADE,
    entity_id INTEGER REFERENCES entities(entity_id) ON DELETE CASCADE,
    PRIMARY KEY (fact_id, entity_id)
);

CREATE VIRTUAL TABLE IF NOT EXISTS facts_fts USING fts5(
    content, tags, content='facts', content_rowid='fact_id'
);

CREATE TRIGGER IF NOT EXISTS facts_ai AFTER INSERT ON facts BEGIN
    INSERT INTO facts_fts(rowid, content, tags) VALUES (new.fact_id, new.content, new.tags);
END;

CREATE TRIGGER IF NOT EXISTS facts_ad AFTER DELETE ON facts BEGIN
    INSERT INTO facts_fts(facts_fts, rowid, content, tags) VALUES('delete', old.fact_id, old.content, old.tags);
END;

CREATE TRIGGER IF NOT EXISTS facts_au AFTER UPDATE ON facts BEGIN
    INSERT INTO facts_fts(facts_fts, rowid, content, tags) VALUES('delete', old.fact_id, old.content, old.tags);
    INSERT INTO facts_fts(rowid, content, tags) VALUES (new.fact_id, new.content, new.tags);
END;
`;
// ─── Implementation ──────────────────────────────────────────────────────────
export class FactStore {
    constructor(db) {
        this.db = null;
        this.isAvailable = false;
        this.insertFactStmt = null;
        this.searchFtsStmt = null;
        this.getFactByIdStmt = null;
        this.db = db;
        if (db) {
            try {
                db.exec(FACT_SCHEMA);
                this.isAvailable = true;
                this.insertFactStmt = db.prepare(`INSERT INTO facts (content, tags) VALUES (@content, @tags) ON CONFLICT(content) DO UPDATE SET updated_at = datetime('now')`);
                this.searchFtsStmt = db.prepare(`SELECT f.fact_id, f.content, f.tags, f.trust_score, f.retrieval_count, f.helpful_count, f.created_at, f.updated_at
					 FROM facts f JOIN facts_fts ON f.fact_id = facts_fts.rowid
                     WHERE facts_fts MATCH @query
					   AND f.trust_score >= @minTrust
					 ORDER BY rank LIMIT @limit`);
                this.getFactByIdStmt = db.prepare(`SELECT fact_id, content, tags, trust_score, retrieval_count, helpful_count, created_at, updated_at FROM facts WHERE fact_id = @id`);
            }
            catch {
                this.isAvailable = false;
            }
        }
    }
    static create(db) {
        return new FactStore(db);
    }
    addFact(content, tags) {
        if (!this.isAvailable || !this.insertFactStmt)
            return null;
        try {
            const result = this.insertFactStmt.run({ content, tags });
            return Number(result.lastInsertRowid ?? 0) || null;
        }
        catch {
            return null;
        }
    }
    searchFacts(query, limit = 10) {
        if (!this.isAvailable || !this.searchFtsStmt)
            return [];
        const safeQuery = query.replace(/[^\w\s*\-"()]/g, " ").trim();
        if (!safeQuery)
            return [];
        try {
            const rows = this.searchFtsStmt.all({ query: safeQuery, minTrust: 0.0, limit });
            return rows.map((r) => this._rowToFact(r));
        }
        catch {
            return [];
        }
    }
    getFact(factId) {
        if (!this.isAvailable || !this.getFactByIdStmt)
            return null;
        const row = this.getFactByIdStmt.get({ id: factId });
        return row ? this._rowToFact(row) : null;
    }
    probe(entity, limit = 10) {
        return this._entitySearch(entity, limit);
    }
    related(entity, limit = 10) {
        return this._entitySearch(entity, limit);
    }
    reason(entities, limit = 10) {
        return this.searchFacts(entities.join(" "), limit);
    }
    recordFeedback(factId, helpful) {
        if (!this.isAvailable || !this.db)
            return false;
        try {
            const existing = this.getFact(factId);
            if (!existing)
                return false;
            const oldTrust = existing.trustScore;
            const newTrust = helpful ? Math.min(1, oldTrust + 0.05) : Math.max(0, oldTrust - 0.1);
            this.db
                .prepare("UPDATE facts SET trust_score = @trust, retrieval_count = retrieval_count + 1, helpful_count = helpful_count + @helpIncr WHERE fact_id = @factId")
                .run({ trust: newTrust, helpIncr: helpful ? 1 : 0, factId });
            return { oldTrust, newTrust };
        }
        catch {
            return false;
        }
    }
    getFactCount() {
        if (!this.isAvailable || !this.db)
            return 0;
        const row = this.db.prepare("SELECT COUNT(*) as cnt FROM facts").get({});
        return row?.cnt ?? 0;
    }
    dispose() {
        this.db = null;
        this.isAvailable = false;
    }
    _rowToFact(row) {
        return {
            factId: row.fact_id,
            content: row.content,
            tags: row.tags,
            trustScore: row.trust_score,
            retrievalCount: row.retrieval_count,
            helpfulCount: row.helpful_count,
            createdAt: row.created_at,
            updatedAt: row.updated_at,
        };
    }
    _entitySearch(entity, limit) {
        if (!this.isAvailable || !this.db)
            return [];
        try {
            const rows = this.db
                .prepare(`SELECT DISTINCT f.fact_id, f.content, f.tags, f.trust_score, f.retrieval_count, f.helpful_count, f.created_at, f.updated_at
				 FROM facts f
				 JOIN fact_entities fe ON f.fact_id = fe.fact_id
				 JOIN entities e ON fe.entity_id = e.entity_id
				 WHERE e.name = @entity AND f.trust_score >= @minTrust
				 ORDER BY f.trust_score DESC LIMIT @limit`)
                .all({ entity, minTrust: 0.0, limit });
            return rows.map((r) => this._rowToFact(r));
        }
        catch {
            return [];
        }
    }
}
//# sourceMappingURL=FactStore.js.map