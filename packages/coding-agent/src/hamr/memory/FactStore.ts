/**
 * FactStore — SQLite-backed structured fact memory with entity resolution,
 * trust scoring, and HRR-based compositional retrieval.
 *
 * Shares the database with HolographicMemory (transcript FTS5) but uses
 * its own tables for structured knowledge that persists across sessions.
 */

import { sanitizeFts5Query } from "./HolographicMemory.ts";

type Stmt = {
	run(params: Record<string, unknown>): { lastInsertRowid: number | bigint };
	all(params: Record<string, unknown>): unknown[];
	get(params: Record<string, unknown>): unknown | undefined;
};

// ─── Types ───────────────────────────────────────────────────────────────────

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

// ─── Entity extraction patterns ──────────────────────────────────────────────

// Capitalized multi-word phrases  e.g. "John Doe", "React Router"
const RE_CAPITALIZED = /\b([A-Z][a-z]+(?:\s+[A-Z][a-z]+)+)\b/g;
// Double-quoted terms             e.g. "hamr"
const RE_DOUBLE_QUOTE = /"([^"]+)"/g;
// Single-quoted terms             e.g. 'postgres'
const RE_SINGLE_QUOTE = /'([^']+)'/g;
// Backtick-quoted terms           e.g. `search_memory`
const RE_BACKTICK = /`([^`\n]{2,60})`/g;
// AKA patterns                    e.g. "Guido aka BDFL"
const RE_AKA = /\b([\w-]+)\s+(?:aka|also known as)\s+([\w-]+)/gi;

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
export function extractEntities(text: string): string[] {
	const seen = new Set<string>();
	const candidates: string[] = [];

	const add = (name: string): void => {
		const stripped = name.trim();
		if (stripped && stripped.length >= 2 && stripped.length <= 120 && !seen.has(stripped.toLowerCase())) {
			seen.add(stripped.toLowerCase());
			candidates.push(stripped);
		}
	};

	for (const m of text.matchAll(RE_CAPITALIZED)) add(m[1]);
	for (const m of text.matchAll(RE_DOUBLE_QUOTE)) add(m[1]);
	for (const m of text.matchAll(RE_SINGLE_QUOTE)) add(m[1]);
	for (const m of text.matchAll(RE_BACKTICK)) add(m[1]);
	for (const m of text.matchAll(RE_AKA)) {
		// Only accept AKA matches where the left-hand name starts with a capital
		// letter — prevents false positives like "and AKA patterns"
		if (/^[A-Z]/.test(m[1])) {
			add(m[1]);
			add(m[2]);
		}
	}

	return candidates;
}

// ─── Implementation ──────────────────────────────────────────────────────────

export class FactStore {
	private db: { exec(sql: string): void; prepare(sql: string): Stmt } | null = null;
	public isAvailable = false;
	private insertFactStmt: Stmt | null = null;
	private searchFtsStmt: Stmt | null = null;
	private getFactByIdStmt: Stmt | null = null;
	private resolveEntityStmt: Stmt | null = null;
	private resolveEntityInsertStmt: Stmt | null = null;
	private linkFactEntityStmt: Stmt | null = null;
	private getEntitiesStmt: Stmt | null = null;
	private deleteFactEntitiesStmt: Stmt | null = null;
	private listAllFactsStmt: Stmt | null = null;
	private listRecentFactsStmt: Stmt | null = null;

	constructor(db: { exec(sql: string): void; prepare(sql: string): Stmt } | null) {
		this.db = db;
		if (db) {
			try {
				db.exec(FACT_SCHEMA);
				this.isAvailable = true;
				this.insertFactStmt = db.prepare(
					`INSERT INTO facts (content, tags) VALUES (@content, @tags) ON CONFLICT(content) DO UPDATE SET tags = excluded.tags, updated_at = datetime('now')`,
				);
				this.searchFtsStmt = db.prepare(
					`SELECT f.fact_id, f.content, f.tags, f.trust_score, f.retrieval_count, f.helpful_count, f.created_at, f.updated_at
					 FROM facts f JOIN facts_fts ON f.fact_id = facts_fts.rowid
                     WHERE facts_fts MATCH @query
					   AND f.trust_score >= @minTrust
					 ORDER BY rank LIMIT @limit`,
				);
				this.getFactByIdStmt = db.prepare(
					`SELECT fact_id, content, tags, trust_score, retrieval_count, helpful_count, created_at, updated_at FROM facts WHERE fact_id = @id`,
				);
				this.resolveEntityStmt = db.prepare(`SELECT entity_id FROM entities WHERE name = @name COLLATE NOCASE`);
				this.resolveEntityInsertStmt = db.prepare(`INSERT OR IGNORE INTO entities (name) VALUES (@name)`);
				this.linkFactEntityStmt = db.prepare(
					`INSERT OR IGNORE INTO fact_entities (fact_id, entity_id) VALUES (@factId, @entityId)`,
				);
				this.getEntitiesStmt = db.prepare(
					`SELECT e.name FROM entities e JOIN fact_entities fe ON fe.entity_id = e.entity_id WHERE fe.fact_id = @factId`,
				);
				this.deleteFactEntitiesStmt = db.prepare(`DELETE FROM fact_entities WHERE fact_id = @factId`);
				this.listAllFactsStmt = db.prepare(
					`SELECT fact_id, content, tags, trust_score, retrieval_count, helpful_count, created_at, updated_at
					 FROM facts
					 ORDER BY trust_score DESC
					 LIMIT @limit`,
				);
				this.listRecentFactsStmt = db.prepare(
					`SELECT fact_id, content, tags, trust_score, retrieval_count, helpful_count, created_at, updated_at
					 FROM facts
					 WHERE trust_score >= @minTrust
					 ORDER BY datetime(updated_at) DESC, fact_id DESC
					 LIMIT @limit`,
				);
			} catch {
				this.isAvailable = false;
			}
		}
	}

	static create(db: { exec(sql: string): void; prepare(sql: string): Stmt } | null): FactStore {
		return new FactStore(db);
	}

	addFact(content: string, tags: string): number | null {
		if (!this.isAvailable || !this.insertFactStmt || !this.db) return null;
		try {
			this.db.exec("BEGIN");
			const result = this.insertFactStmt.run({ content, tags });
			let factId = Number(result.lastInsertRowid ?? 0) || null;
			if (!factId || factId <= 0) {
				const row = this.db.prepare("SELECT fact_id FROM facts WHERE content = @content").get({ content }) as
					| { fact_id: number }
					| undefined;
				factId = row?.fact_id ?? null;
			}
			if (factId && factId > 0) {
				// Purge old entity links on upsert so stale links don't survive re-extraction
				if (this.deleteFactEntitiesStmt) {
					this.deleteFactEntitiesStmt.run({ factId });
				}
				// Extract entities from the fact content and link them
				const entities = extractEntities(content);
				for (const name of entities) {
					const entityId = this._resolveEntity(name);
					if (entityId) this._linkFactEntity(factId, entityId);
				}
			}
			this.db.exec("COMMIT");
			return factId;
		} catch {
			this.db?.exec("ROLLBACK");
			return null;
		}
	}

	searchFacts(query: string, limit: number = 10): FactWithScore[] {
		if (!this.isAvailable || !this.searchFtsStmt) return [];
		// Bare "*" means "list all" — FTS5 can't handle it, so fall back to a direct SELECT
		if (query.trim() === "*" && this.listAllFactsStmt) {
			try {
				const rows = this.listAllFactsStmt.all({ limit }) as unknown as Array<{
					fact_id: number;
					content: string;
					tags: string;
					trust_score: number;
					retrieval_count: number;
					helpful_count: number;
					created_at: string;
					updated_at: string;
				}>;
				return rows.map((r) => this._rowToFact(r));
			} catch {
				return [];
			}
		}
		const safeQuery = sanitizeFts5Query(query);
		if (!safeQuery) return [];
		try {
			const rows = this.searchFtsStmt.all({ query: safeQuery, minTrust: 0.0, limit }) as unknown as Array<{
				fact_id: number;
				content: string;
				tags: string;
				trust_score: number;
				retrieval_count: number;
				helpful_count: number;
				created_at: string;
				updated_at: string;
			}>;
			return rows.map((r) => this._rowToFact(r));
		} catch {
			return [];
		}
	}

	getFact(factId: number): FactWithScore | null {
		if (!this.isAvailable || !this.getFactByIdStmt) return null;
		const row = this.getFactByIdStmt.get({ id: factId }) as Record<string, unknown> | undefined;
		if (!row) return null;
		const fact: FactWithScore = this._rowToFact(row as any);
		fact.entities = this.getFactEntities(factId);
		return fact;
	}

	/**
	 * Return recent durable facts for recall/continuation prefetch.
	 * This is intentionally separate from searchFacts("*"), which sorts by trust.
	 */
	listRecentFacts(limit: number = 5, minTrust: number = 0.0): FactWithScore[] {
		if (!this.isAvailable || !this.listRecentFactsStmt) return [];
		try {
			const rows = this.listRecentFactsStmt.all({ minTrust, limit }) as unknown as Array<{
				fact_id: number;
				content: string;
				tags: string;
				trust_score: number;
				retrieval_count: number;
				helpful_count: number;
				created_at: string;
				updated_at: string;
			}>;
			return rows.map((r) => this._rowToFact(r));
		} catch {
			return [];
		}
	}

	/**
	 * Get entities linked to a fact. Returns entity names.
	 */
	getFactEntities(factId: number): string[] {
		if (!this.isAvailable || !this.getEntitiesStmt) return [];
		try {
			const rows = this.getEntitiesStmt.all({ factId }) as unknown as Array<{ name: string }>;
			return rows.map((r) => r.name);
		} catch {
			return [];
		}
	}

	/**
	 * Probe for facts about a specific entity (case-insensitive match).
	 * Uses the fact_entities junction table. Falls back to FTS5 search
	 * if the entity has no linked facts.
	 */
	probe(entity: string, limit: number = 10): FactWithScore[] {
		const results = this._entitySearch(entity, limit);
		if (results.length > 0) return results;
		// Fallback to FTS5 keyword search so probe never returns empty
		// when there are facts containing the entity name as text
		return this.searchFacts(entity, limit);
	}

	/**
	 * Discover facts that share entities with the given entity.
	 * First finds facts directly linked to the entity, then finds
	 * other facts that share at least one entity with those.
	 * Falls back to FTS5 search if no structured links exist.
	 */
	related(entity: string, limit: number = 10): FactWithScore[] {
		const direct = this._entitySearch(entity, limit);
		if (direct.length === 0) return this.searchFacts(entity, limit);

		if (!this.isAvailable || !this.db || direct.length === 0) return direct;
		try {
			// Find entities linked to the direct facts, then find other facts sharing those
			const directIds = direct.map((f) => f.factId);
			// Use named params for the IN clause in better-sqlite3 style
			const idParams: Record<string, number> = {};
			const idPlaceholders = directIds
				.map((id, i) => {
					idParams[`id${i}`] = id;
					return `@id${i}`;
				})
				.join(",");
			const rows = this.db
				.prepare(
					`SELECT DISTINCT f.fact_id, f.content, f.tags, f.trust_score, f.retrieval_count, f.helpful_count, f.created_at, f.updated_at
					 FROM facts f
					 JOIN fact_entities fe ON f.fact_id = fe.fact_id
					 WHERE fe.entity_id IN (
					   SELECT DISTINCT fe2.entity_id FROM fact_entities fe2
					   WHERE fe2.fact_id IN (${idPlaceholders})
					 )
					 AND f.fact_id NOT IN (${idPlaceholders})
					 AND f.trust_score >= @minTrust
					 ORDER BY f.trust_score DESC LIMIT @limit`,
				)
				.all({ ...idParams, minTrust: 0.0, limit }) as unknown as Array<{
				fact_id: number;
				content: string;
				tags: string;
				trust_score: number;
				retrieval_count: number;
				helpful_count: number;
				created_at: string;
				updated_at: string;
			}>;
			if (rows.length === 0) return direct;
			// Prepend direct results, deduplicate
			const directIdsSet = new Set(directIds);
			const related = rows.filter((r) => !directIdsSet.has(r.fact_id)).map((r) => this._rowToFact(r));
			return [...direct, ...related].slice(0, limit);
		} catch {
			return direct;
		}
	}

	reason(entities: string[], limit: number = 10): FactWithScore[] {
		return this.searchFacts(entities.join(" "), limit);
	}

	recordFeedback(factId: number, helpful: boolean): { oldTrust: number; newTrust: number } | false {
		if (!this.isAvailable || !this.db) return false;
		try {
			const existing = this.getFact(factId);
			if (!existing) return false;
			const oldTrust = existing.trustScore;
			const newTrust = helpful ? Math.min(1, oldTrust + 0.05) : Math.max(0, oldTrust - 0.1);
			this.db
				.prepare(
					"UPDATE facts SET trust_score = @trust, retrieval_count = retrieval_count + 1, helpful_count = helpful_count + @helpIncr WHERE fact_id = @factId",
				)
				.run({ trust: newTrust, helpIncr: helpful ? 1 : 0, factId });
			return { oldTrust, newTrust };
		} catch {
			return false;
		}
	}

	getFactCount(): number {
		if (!this.isAvailable || !this.db) return 0;
		const row = this.db.prepare("SELECT COUNT(*) as cnt FROM facts").get({}) as { cnt: number } | undefined;
		return row?.cnt ?? 0;
	}

	dispose(): void {
		this.db = null;
		this.isAvailable = false;
	}

	private _rowToFact(row: {
		fact_id: number;
		content: string;
		tags: string;
		trust_score: number;
		retrieval_count: number;
		helpful_count: number;
		created_at: string;
		updated_at: string;
	}): FactEntry {
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

	private _entitySearch(entity: string, limit: number): FactWithScore[] {
		if (!this.isAvailable || !this.db) return [];
		// Try case-insensitive match first
		let rows: Array<{
			fact_id: number;
			content: string;
			tags: string;
			trust_score: number;
			retrieval_count: number;
			helpful_count: number;
			created_at: string;
			updated_at: string;
		}> = [];
		try {
			rows = this.db
				.prepare(
					`SELECT DISTINCT f.fact_id, f.content, f.tags, f.trust_score, f.retrieval_count, f.helpful_count, f.created_at, f.updated_at
				 FROM facts f
				 JOIN fact_entities fe ON f.fact_id = fe.fact_id
				 JOIN entities e ON fe.entity_id = e.entity_id
				 WHERE e.name = @entity COLLATE NOCASE AND f.trust_score >= @minTrust
				 ORDER BY f.trust_score DESC LIMIT @limit`,
				)
				.all({ entity, minTrust: 0.0, limit }) as unknown as typeof rows;
		} catch {
			return [];
		}
		return rows.map((r) => this._rowToFact(r));
	}

	/** Find an existing entity by case-insensitive name match, or create one. */
	private _resolveEntity(name: string): number | null {
		if (!this.isAvailable || !this.db || !this.resolveEntityStmt) return null;
		try {
			const existing = this.resolveEntityStmt.get({ name }) as { entity_id: number } | undefined;
			if (existing) return existing.entity_id;
			// Create new entity
			const result = this.resolveEntityInsertStmt!.run({ name });
			const id = Number(result.lastInsertRowid ?? 0);
			if (id > 0) return id;
			// Race condition: another insert won, fetch its id
			const retry = this.resolveEntityStmt.get({ name }) as { entity_id: number } | undefined;
			return retry?.entity_id ?? null;
		} catch {
			return null;
		}
	}

	private _linkFactEntity(factId: number, entityId: number): void {
		if (!this.linkFactEntityStmt) return;
		try {
			this.linkFactEntityStmt.run({ factId, entityId });
		} catch {
			// ignore duplicate links
		}
	}
}
