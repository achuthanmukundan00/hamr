//! FactStore — SQLite-backed structured fact memory with entity resolution,
//! trust scoring, and HRR-based compositional retrieval.
//!
//! Mirrors `packages/coding-agent/src/hamr/memory/FactStore.ts` (v0.7.1).
//!
//! Shares the database with HolographicMemory (transcript FTS5) but uses
//! its own tables for structured knowledge that persists across sessions.
//!
//! # v0.7.1 features
//!
//! - **Entity extraction** — auto-extracts entities from fact content on `add_fact`
//! - **Entity linking** — transactional upsert + link with old-link purge on re-extraction
//! - **probe/related** — entity join search with FTS5 keyword fallback
//! - **listRecentFacts** — recently-updated facts for recall/continuation prefetch
//! - **Bare "*"** — `searchFacts("*")` falls back to `listAllFacts` (trust-sorted)

use std::sync::LazyLock;

use serde::{Deserialize, Serialize};

#[cfg(feature = "hamr-memory")]
use rusqlite::{Connection, params};

use regex::Regex;

// ─── Types ───────────────────────────────────────────────────────────────────

/// A fact entry from the database.
///
/// Strict mirror of TS `FactEntry`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FactEntry {
    pub fact_id: i64,
    pub content: String,
    pub tags: String,
    pub trust_score: f64,
    pub retrieval_count: i64,
    pub helpful_count: i64,
    pub created_at: String,
    pub updated_at: String,
}

/// A fact entry with optional score and entities.
///
/// TS `FactWithScore extends FactEntry` — all FactEntry fields are at the top
/// level, plus optional `score` and `entities`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FactWithScore {
    pub fact_id: i64,
    pub content: String,
    pub tags: String,
    pub trust_score: f64,
    pub retrieval_count: i64,
    pub helpful_count: i64,
    pub created_at: String,
    pub updated_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub score: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entities: Option<Vec<String>>,
}

impl From<FactEntry> for FactWithScore {
    fn from(f: FactEntry) -> Self {
        FactWithScore {
            fact_id: f.fact_id,
            content: f.content,
            tags: f.tags,
            trust_score: f.trust_score,
            retrieval_count: f.retrieval_count,
            helpful_count: f.helpful_count,
            created_at: f.created_at,
            updated_at: f.updated_at,
            score: None,
            entities: None,
        }
    }
}

// ─── Schema ──────────────────────────────────────────────────────────────────

#[cfg(feature = "hamr-memory")]
const FACT_SCHEMA: &str = r#"
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
"#;

// ─── Entity extraction (v0.7.1) ──────────────────────────────────────────────

/// Capitalized multi-word phrases  e.g. "John Doe", "React Router"
static RE_CAPITALIZED: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\b([A-Z][a-z]{1,30}(?:\s+[A-Z][a-z]{1,30}){1,4})\b").unwrap());

/// Double-quoted terms             e.g. "Python"
static RE_DOUBLE_QUOTE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#""([^"\n]{2,120})""#).unwrap());

/// Single-quoted terms             e.g. 'postgres'
static RE_SINGLE_QUOTE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"'([^'\n]{2,120})'").unwrap());

/// Backtick-quoted terms           e.g. `search_memory`
static RE_BACKTICK: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"`([^`\n]{2,60})`").unwrap());

/// AKA patterns                    e.g. "Guido aka BDFL"
static RE_AKA: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\b([\w-]+)\s+(?:aka|also known as)\s+([\w-]+)").unwrap());

/// Extract entity candidates from text using simple regex rules.
///
/// Rules applied (in order):
/// 1. Capitalized multi-word phrases  e.g. "John Doe"
/// 2. Double-quoted terms             e.g. "Python"
/// 3. Single-quoted terms             e.g. 'pytest'
/// 4. Backtick-quoted terms           e.g. `search_memory`
/// 5. AKA patterns                    e.g. "Guido aka BDFL" → two entities
///
/// Returns a deduplicated list preserving first-seen order.
///
/// Mirrors TS `extractEntities(text)`.
pub fn extract_entities(text: &str) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    let mut candidates: Vec<String> = Vec::new();

    let mut add = |name: &str| {
        let stripped = name.trim();
        if stripped.len() >= 2 && stripped.len() <= 120 {
            let lower = stripped.to_lowercase();
            if !seen.contains(&lower) {
                seen.insert(lower);
                candidates.push(stripped.to_string());
            }
        }
    };

    for cap in RE_CAPITALIZED.captures_iter(text) {
        if let Some(m) = cap.get(1) {
            add(m.as_str());
        }
    }
    for cap in RE_DOUBLE_QUOTE.captures_iter(text) {
        if let Some(m) = cap.get(1) {
            add(m.as_str());
        }
    }
    for cap in RE_SINGLE_QUOTE.captures_iter(text) {
        if let Some(m) = cap.get(1) {
            add(m.as_str());
        }
    }
    for cap in RE_BACKTICK.captures_iter(text) {
        if let Some(m) = cap.get(1) {
            add(m.as_str());
        }
    }
    for cap in RE_AKA.captures_iter(text) {
        let left = cap.get(1).map(|m| m.as_str()).unwrap_or("");
        let right = cap.get(2).map(|m| m.as_str()).unwrap_or("");
        // Only accept AKA matches where the left-hand name starts with a capital
        // letter — prevents false positives like "and AKA patterns"
        if left.starts_with(|c: char| c.is_uppercase()) {
            add(left);
            add(right);
        }
    }

    candidates
}

// ─── FTS5 query sanitizer ────────────────────────────────────────────────────

/// Sanitize a user query for safe FTS5 MATCH usage.
///
/// Uses the same algorithm as `holographic_memory::sanitize_fts_query`.
/// Duplicated here to avoid a circular dependency.
fn sanitize_fts_query(query: &str) -> String {
    crate::hamr::memory::holographic_memory::sanitize_fts5_query(query)
}

// ─── Implementation ──────────────────────────────────────────────────────────

/// SQLite-backed structured fact memory with entity resolution and trust scoring.
///
/// When the `hamr-memory` feature is disabled, all operations are safe no-ops.
pub struct FactStore {
    #[cfg(feature = "hamr-memory")]
    db: Option<Connection>,
    pub is_available: bool,
}

impl FactStore {
    /// Create a new FactStore from an optional rusqlite Connection.
    #[cfg(feature = "hamr-memory")]
    pub fn new(db: Option<Connection>) -> Self {
        let is_available = db
            .as_ref()
            .map_or(false, |conn| conn.execute_batch(FACT_SCHEMA).is_ok());
        FactStore { db, is_available }
    }

    /// Create a new FactStore (no-op fallback when feature is disabled).
    #[cfg(not(feature = "hamr-memory"))]
    pub fn new() -> Self {
        FactStore {
            is_available: false,
        }
    }

    /// Static factory — mirror of `FactStore.create(db)`.
    #[cfg(feature = "hamr-memory")]
    pub fn create(db: Option<Connection>) -> Self {
        Self::new(db)
    }

    #[cfg(not(feature = "hamr-memory"))]
    pub fn create() -> Self {
        Self::new()
    }

    // ── add_fact (v0.7.1: entity extraction + linking) ─────────────────────

    /// Add a fact with the given content and tags.
    ///
    /// On CONFLICT (duplicate content) the existing row's `updated_at` is
    /// touched, old entity links are purged and re-extracted, and the
    /// existing id is returned.
    ///
    /// Returns the new/existing `fact_id` on success, or `None` on failure.
    ///
    /// Mirrors TS `addFact(content, tags)`.
    pub fn add_fact(&self, content: &str, tags: &str) -> Option<i64> {
        #[cfg(feature = "hamr-memory")]
        {
            let db = self.db.as_ref()?;
            // Use a transaction so entity extraction + linking is atomic
            let _ = db.execute_batch("BEGIN");
            let fact_id = {
                let mut stmt = db
                    .prepare_cached(
                        "INSERT INTO facts (content, tags) VALUES (?1, ?2) \
                         ON CONFLICT(content) DO UPDATE SET updated_at = datetime('now')",
                    )
                    .ok()?;
                stmt.execute(params![content, tags]).ok()?;
                db.last_insert_rowid()
            };

            if fact_id > 0 {
                // Purge old entity links on upsert so stale links don't survive re-extraction
                let _ = db.execute(
                    "DELETE FROM fact_entities WHERE fact_id = ?1",
                    params![fact_id],
                );

                // Extract entities from the fact content and link them
                let entities = extract_entities(content);
                for name in &entities {
                    if let Some(entity_id) = self._resolve_entity(name) {
                        self._link_fact_entity(fact_id, entity_id);
                    }
                }
            }

            let _ = db.execute_batch("COMMIT");
            if fact_id > 0 { Some(fact_id) } else { None }
        }
        #[cfg(not(feature = "hamr-memory"))]
        {
            None
        }
    }

    /// Full-text search over stored facts using FTS5.
    ///
    /// Mirror of `searchFacts(query, limit)` — returns `FactWithScore[]`.
    ///
    /// Bare `"*"` means "list all" — FTS5 can't handle it, so falls back to
    /// a direct SELECT ordered by trust_score (v0.7.1).
    pub fn search_facts(&self, query: &str, limit: usize) -> Vec<FactWithScore> {
        #[cfg(feature = "hamr-memory")]
        {
            let db = match self.db.as_ref() {
                Some(db) => db,
                None => return vec![],
            };

            // Bare "*" means "list all" — FTS5 can't handle it
            if query.trim() == "*" {
                return self._list_all_facts(limit);
            }

            let safe_query = sanitize_fts_query(query);
            if safe_query.is_empty() {
                return vec![];
            }
            let mut stmt = match db.prepare_cached(
                "SELECT f.fact_id, f.content, f.tags, f.trust_score, f.retrieval_count, \
                        f.helpful_count, f.created_at, f.updated_at \
                 FROM facts f \
                 JOIN facts_fts ON f.fact_id = facts_fts.rowid \
                 WHERE facts_fts MATCH ?1 \
                   AND f.trust_score >= ?2 \
                 ORDER BY rank LIMIT ?3",
            ) {
                Ok(s) => s,
                Err(_) => return vec![],
            };
            let rows = match stmt.query_map(
                params![safe_query, 0.0_f64, limit as i64],
                Self::row_to_fact_with_score,
            ) {
                Ok(r) => r,
                Err(_) => return vec![],
            };
            rows.filter_map(|r| r.ok()).collect()
        }
        #[cfg(not(feature = "hamr-memory"))]
        {
            vec![]
        }
    }

    /// List all facts ordered by trust score (fallback for `searchFacts("*")`).
    #[cfg(feature = "hamr-memory")]
    fn _list_all_facts(&self, limit: usize) -> Vec<FactWithScore> {
        let db = match self.db.as_ref() {
            Some(db) => db,
            None => return vec![],
        };
        let mut stmt = match db.prepare_cached(
            "SELECT fact_id, content, tags, trust_score, retrieval_count, \
                    helpful_count, created_at, updated_at \
             FROM facts \
             ORDER BY trust_score DESC \
             LIMIT ?1",
        ) {
            Ok(s) => s,
            Err(_) => return vec![],
        };
        let rows = match stmt.query_map(params![limit as i64], Self::row_to_fact_with_score) {
            Ok(r) => r,
            Err(_) => return vec![],
        };
        rows.filter_map(|r| r.ok()).collect()
    }

    #[cfg(not(feature = "hamr-memory"))]
    fn _list_all_facts(&self, _limit: usize) -> Vec<FactWithScore> {
        vec![]
    }

    /// Get a single fact by its id.
    ///
    /// Mirror of `getFact(factId)` — returns `FactWithScore | null`.
    pub fn get_fact(&self, fact_id: i64) -> Option<FactWithScore> {
        #[cfg(feature = "hamr-memory")]
        {
            let db = self.db.as_ref()?;
            let mut stmt = db
                .prepare_cached(
                    "SELECT fact_id, content, tags, trust_score, retrieval_count, \
                            helpful_count, created_at, updated_at \
                     FROM facts WHERE fact_id = ?1",
                )
                .ok()?;
            let mut fact: FactWithScore = stmt
                .query_row(params![fact_id], Self::row_to_fact_with_score)
                .ok()?;
            fact.entities = Some(self.get_fact_entities(fact_id));
            Some(fact)
        }
        #[cfg(not(feature = "hamr-memory"))]
        {
            None
        }
    }

    /// Return recent durable facts for recall/continuation prefetch.
    ///
    /// This is intentionally separate from `searchFacts("*")`, which sorts by
    /// trust. Recent facts are ordered by `updated_at DESC, fact_id DESC`.
    ///
    /// Mirrors TS `listRecentFacts(limit, minTrust)` (v0.7.1).
    pub fn list_recent_facts(&self, limit: usize, min_trust: f64) -> Vec<FactWithScore> {
        #[cfg(feature = "hamr-memory")]
        {
            let db = match self.db.as_ref() {
                Some(db) => db,
                None => return vec![],
            };
            let mut stmt = match db.prepare_cached(
                "SELECT fact_id, content, tags, trust_score, retrieval_count, \
                        helpful_count, created_at, updated_at \
                 FROM facts \
                 WHERE trust_score >= ?1 \
                 ORDER BY datetime(updated_at) DESC, fact_id DESC \
                 LIMIT ?2",
            ) {
                Ok(s) => s,
                Err(_) => return vec![],
            };
            let rows = match stmt.query_map(
                params![min_trust, limit as i64],
                Self::row_to_fact_with_score,
            ) {
                Ok(r) => r,
                Err(_) => return vec![],
            };
            rows.filter_map(|r| r.ok()).collect()
        }
        #[cfg(not(feature = "hamr-memory"))]
        {
            vec![]
        }
    }

    /// Get entities linked to a fact. Returns entity names.
    ///
    /// Mirrors TS `getFactEntities(factId)`.
    pub fn get_fact_entities(&self, fact_id: i64) -> Vec<String> {
        #[cfg(feature = "hamr-memory")]
        {
            let db = match self.db.as_ref() {
                Some(db) => db,
                None => return vec![],
            };
            let mut stmt = match db.prepare_cached(
                "SELECT e.name FROM entities e \
                 JOIN fact_entities fe ON fe.entity_id = e.entity_id \
                 WHERE fe.fact_id = ?1",
            ) {
                Ok(s) => s,
                Err(_) => return vec![],
            };
            let rows = match stmt.query_map(params![fact_id], |row| row.get::<_, String>(0)) {
                Ok(r) => r,
                Err(_) => return vec![],
            };
            rows.filter_map(|r| r.ok()).collect()
        }
        #[cfg(not(feature = "hamr-memory"))]
        {
            vec![]
        }
    }

    /// Probe for facts about a specific entity (case-insensitive match).
    ///
    /// Uses the fact_entities junction table. Falls back to FTS5 search
    /// if the entity has no linked facts, so probe never returns empty
    /// when there are facts containing the entity name as text.
    ///
    /// Mirrors TS `probe(entity, limit)` (v0.7.1).
    pub fn probe(&self, entity: &str, limit: usize) -> Vec<FactWithScore> {
        let results = self._entity_search(entity, limit);
        if !results.is_empty() {
            return results;
        }
        // Fallback to FTS5 keyword search so probe never returns empty
        // when there are facts containing the entity name as text
        self.search_facts(entity, limit)
    }

    /// Discover facts that share entities with the given entity.
    ///
    /// First finds facts directly linked to the entity, then finds
    /// other facts that share at least one entity with those.
    /// Falls back to FTS5 search if no structured links exist.
    ///
    /// Mirrors TS `related(entity, limit)` (v0.7.1).
    pub fn related(&self, entity: &str, limit: usize) -> Vec<FactWithScore> {
        let direct = self._entity_search(entity, limit);
        if direct.is_empty() {
            return self.search_facts(entity, limit);
        }

        #[cfg(feature = "hamr-memory")]
        {
            let db = match self.db.as_ref() {
                Some(db) => db,
                None => return direct,
            };

            let direct_ids: Vec<i64> = direct.iter().map(|f| f.fact_id).collect();

            // Build dynamic IN clause placeholders
            let id_placeholders: Vec<String> = direct_ids
                .iter()
                .enumerate()
                .map(|(i, _)| format!("?{}", i + 1))
                .collect();
            let in_clause = id_placeholders.join(",");
            let in_clause2 = id_placeholders
                .iter()
                .enumerate()
                .map(|(i, _)| format!("?{}", i + 1 + direct_ids.len()))
                .collect::<Vec<_>>()
                .join(",");

            let sql = format!(
                "SELECT DISTINCT f.fact_id, f.content, f.tags, f.trust_score, \
                        f.retrieval_count, f.helpful_count, f.created_at, f.updated_at \
                 FROM facts f \
                 JOIN fact_entities fe ON f.fact_id = fe.fact_id \
                 WHERE fe.entity_id IN ( \
                   SELECT DISTINCT fe2.entity_id FROM fact_entities fe2 \
                   WHERE fe2.fact_id IN ({}) \
                 ) \
                 AND f.fact_id NOT IN ({}) \
                 AND f.trust_score >= ?{} \
                 ORDER BY f.trust_score DESC LIMIT ?{}",
                in_clause,
                in_clause2,
                direct_ids.len() * 2 + 1,
                direct_ids.len() * 2 + 2,
            );

            let mut all_params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
            for &id in &direct_ids {
                all_params.push(Box::new(id));
            }
            for &id in &direct_ids {
                all_params.push(Box::new(id));
            }
            all_params.push(Box::new(0.0_f64));
            all_params.push(Box::new(limit as i64));

            let param_refs: Vec<&dyn rusqlite::types::ToSql> =
                all_params.iter().map(|p| p.as_ref()).collect();

            let mut stmt = match db.prepare(&sql) {
                Ok(s) => s,
                Err(_) => return direct,
            };

            let rows: Vec<FactWithScore> =
                match stmt.query_map(param_refs.as_slice(), Self::row_to_fact_with_score) {
                    Ok(r) => r.filter_map(|r| r.ok()).collect(),
                    Err(_) => return direct,
                };

            if rows.is_empty() {
                return direct;
            }

            // Prepend direct results, deduplicate
            let direct_ids_set: std::collections::HashSet<i64> = direct_ids.into_iter().collect();
            let related: Vec<FactWithScore> = rows
                .into_iter()
                .filter(|r| !direct_ids_set.contains(&r.fact_id))
                .collect();

            let mut combined = direct;
            combined.extend(related);
            combined.truncate(limit);
            combined
        }
        #[cfg(not(feature = "hamr-memory"))]
        {
            direct
        }
    }

    /// Search facts by joining entity names into a FTS5 query (AND semantics).
    pub fn reason(&self, entities: &[String], limit: usize) -> Vec<FactWithScore> {
        self.search_facts(&entities.join(" "), limit)
    }

    /// Record feedback for a fact, updating its trust score.
    ///
    /// Returns `(old_trust, new_trust)` on success, or `None` on failure.
    pub fn record_feedback(&self, fact_id: i64, helpful: bool) -> Option<(f64, f64)> {
        #[cfg(feature = "hamr-memory")]
        {
            let existing = self.get_fact(fact_id)?;
            let old_trust = existing.trust_score;
            let new_trust = if helpful {
                (old_trust + 0.05).min(1.0)
            } else {
                (old_trust - 0.10).max(0.0)
            };
            let db = self.db.as_ref()?;
            let mut stmt = db
                .prepare_cached(
                    "UPDATE facts SET trust_score = ?1, retrieval_count = retrieval_count + 1, \
                     helpful_count = helpful_count + ?2 WHERE fact_id = ?3",
                )
                .ok()?;
            let help_incr: i64 = if helpful { 1 } else { 0 };
            stmt.execute(params![new_trust, help_incr, fact_id]).ok()?;
            Some((old_trust, new_trust))
        }
        #[cfg(not(feature = "hamr-memory"))]
        {
            None
        }
    }

    /// Return the total number of facts stored.
    pub fn get_fact_count(&self) -> i64 {
        #[cfg(feature = "hamr-memory")]
        {
            let db = match self.db.as_ref() {
                Some(db) => db,
                None => return 0,
            };
            match db.prepare_cached("SELECT COUNT(*) as cnt FROM facts") {
                Ok(mut s) => s.query_row([], |row| row.get::<_, i64>(0)).unwrap_or(0),
                Err(_) => 0,
            }
        }
        #[cfg(not(feature = "hamr-memory"))]
        {
            0
        }
    }

    /// Dispose the underlying connection, marking the store as unavailable.
    pub fn dispose(&mut self) {
        #[cfg(feature = "hamr-memory")]
        {
            self.db = None;
        }
        self.is_available = false;
    }

    // ── Private helpers ──────────────────────────────────────────────────

    /// Map a rusqlite row to `FactWithScore`.
    fn row_to_fact_with_score(row: &rusqlite::Row<'_>) -> rusqlite::Result<FactWithScore> {
        Ok(FactWithScore {
            fact_id: row.get(0)?,
            content: row.get(1)?,
            tags: row.get(2)?,
            trust_score: row.get(3)?,
            retrieval_count: row.get(4)?,
            helpful_count: row.get(5)?,
            created_at: row.get(6)?,
            updated_at: row.get(7)?,
            score: None,
            entities: None,
        })
    }

    /// Search facts by entity name via the `fact_entities` join table.
    /// Uses case-insensitive LIKE matching (v0.7.1).
    #[cfg(feature = "hamr-memory")]
    fn _entity_search(&self, entity: &str, limit: usize) -> Vec<FactWithScore> {
        let db = match self.db.as_ref() {
            Some(db) => db,
            None => return vec![],
        };
        let mut stmt = match db.prepare_cached(
            "SELECT DISTINCT f.fact_id, f.content, f.tags, f.trust_score, \
                    f.retrieval_count, f.helpful_count, f.created_at, f.updated_at \
             FROM facts f \
             JOIN fact_entities fe ON f.fact_id = fe.fact_id \
             JOIN entities e ON fe.entity_id = e.entity_id \
             WHERE e.name LIKE ?1 AND f.trust_score >= ?2 \
             ORDER BY f.trust_score DESC LIMIT ?3",
        ) {
            Ok(s) => s,
            Err(_) => return vec![],
        };
        let rows = match stmt.query_map(
            params![entity, 0.0_f64, limit as i64],
            Self::row_to_fact_with_score,
        ) {
            Ok(r) => r,
            Err(_) => return vec![],
        };
        rows.filter_map(|r| r.ok()).collect()
    }

    #[cfg(not(feature = "hamr-memory"))]
    fn _entity_search(&self, _entity: &str, _limit: usize) -> Vec<FactWithScore> {
        vec![]
    }

    /// Find an existing entity by case-insensitive name match, or create one.
    ///
    /// Mirrors TS `_resolveEntity(name)`.
    #[cfg(feature = "hamr-memory")]
    fn _resolve_entity(&self, name: &str) -> Option<i64> {
        let db = self.db.as_ref()?;
        // Try to find existing
        let mut stmt = db
            .prepare_cached("SELECT entity_id FROM entities WHERE name LIKE ?1")
            .ok()?;
        if let Ok(id) = stmt.query_row(params![name], |row| row.get::<_, i64>(0)) {
            return Some(id);
        }
        // Create new entity
        let mut insert = db
            .prepare_cached("INSERT OR IGNORE INTO entities (name) VALUES (?1)")
            .ok()?;
        insert.execute(params![name]).ok()?;
        let new_id = db.last_insert_rowid();
        if new_id > 0 {
            return Some(new_id);
        }
        // Race condition: another insert won, fetch its id
        stmt.query_row(params![name], |row| row.get::<_, i64>(0))
            .ok()
    }

    #[cfg(not(feature = "hamr-memory"))]
    fn _resolve_entity(&self, _name: &str) -> Option<i64> {
        None
    }

    /// Link a fact to an entity.
    ///
    /// Mirrors TS `_linkFactEntity(factId, entityId)`.
    #[cfg(feature = "hamr-memory")]
    fn _link_fact_entity(&self, fact_id: i64, entity_id: i64) {
        if let Some(db) = self.db.as_ref() {
            let mut stmt = match db.prepare_cached(
                "INSERT OR IGNORE INTO fact_entities (fact_id, entity_id) VALUES (?1, ?2)",
            ) {
                Ok(s) => s,
                Err(_) => return,
            };
            let _ = stmt.execute(params![fact_id, entity_id]);
        }
    }

    #[cfg(not(feature = "hamr-memory"))]
    fn _link_fact_entity(&self, _fact_id: i64, _entity_id: i64) {}
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
#[cfg(feature = "hamr-memory")]
mod tests {
    use super::*;

    fn make_store() -> FactStore {
        let conn = Connection::open_in_memory().unwrap();
        // Enable foreign keys for the in-memory DB (v0.7.1)
        conn.pragma_update(None, "foreign_keys", "ON").unwrap();
        FactStore::new(Some(conn))
    }

    // ── extract_entities tests ───────────────────────────────────────────

    #[test]
    fn test_extract_entities_capitalized() {
        let entities = extract_entities("John Doe and Jane Smith worked on React Router");
        assert!(entities.contains(&"John Doe".to_string()));
        assert!(entities.contains(&"Jane Smith".to_string()));
        assert!(entities.contains(&"React Router".to_string()));
    }

    #[test]
    fn test_extract_entities_double_quoted() {
        let entities = extract_entities(r#"The "Python" language and "TypeScript""#);
        assert!(entities.contains(&"Python".to_string()));
        assert!(entities.contains(&"TypeScript".to_string()));
    }

    #[test]
    fn test_extract_entities_single_quoted() {
        let entities = extract_entities("Use 'pytest' for testing");
        assert!(entities.contains(&"pytest".to_string()));
    }

    #[test]
    fn test_extract_entities_backtick() {
        let entities = extract_entities("Call `search_memory` tool");
        assert!(entities.contains(&"search_memory".to_string()));
    }

    #[test]
    fn test_extract_entities_aka() {
        let entities = extract_entities("Guido aka BDFL created Python");
        assert!(entities.contains(&"Guido".to_string()));
        assert!(entities.contains(&"BDFL".to_string()));
    }

    #[test]
    fn test_extract_entities_deduplicates() {
        let entities = extract_entities("John Doe and john doe are the same");
        // Case-insensitive dedup: only first occurrence preserved
        assert_eq!(
            entities
                .iter()
                .filter(|e| e.to_lowercase() == "john doe")
                .count(),
            1
        );
    }

    #[test]
    fn test_extract_entities_min_length() {
        let entities = extract_entities("A B C");
        // "A", "B", "C" are too short
        assert!(entities.is_empty());
    }

    // ── FactStore tests ──────────────────────────────────────────────────

    #[test]
    fn test_add_and_get_fact() {
        let store = make_store();
        let id = store.add_fact("The sky is blue", "science").unwrap();
        assert!(id > 0);

        let fact = store.get_fact(id).unwrap();
        assert_eq!(fact.content, "The sky is blue");
        assert_eq!(fact.tags, "science");
        assert!((fact.trust_score - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn test_add_fact_with_entity_extraction() {
        let store = make_store();
        let id = store
            .add_fact("John Doe created React Router for navigation", "tech")
            .unwrap();
        assert!(id > 0);

        let fact = store.get_fact(id).unwrap();
        assert!(fact.entities.is_some());
        let entities = fact.entities.unwrap();
        // Should have extracted "John Doe" and "React Router"
        assert!(entities.iter().any(|e| e == "John Doe"));
        assert!(entities.iter().any(|e| e == "React Router"));
    }

    #[test]
    fn test_add_duplicate_content() {
        let store = make_store();
        let id1 = store.add_fact("duplicate test", "tag1").unwrap();
        let id2 = store.add_fact("duplicate test", "tag2").unwrap();
        assert!(id2 > 0);
    }

    #[test]
    fn test_search_facts() {
        let store = make_store();
        store.add_fact("Rust is a systems programming language", "programming");
        store.add_fact("Python is great for data science", "programming");
        store.add_fact("The sky is blue", "science");

        let results = store.search_facts("programming", 10);
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_search_all_facts_bare_star() {
        let store = make_store();
        store.add_fact("fact one", "tag1");
        store.add_fact("fact two", "tag2");
        store.add_fact("fact three", "tag3");

        let results = store.search_facts("*", 10);
        assert_eq!(results.len(), 3);
    }

    #[test]
    fn test_search_sanitizes_query() {
        let store = make_store();
        store.add_fact("alpha beta gamma", "test");
        // Query with special chars that are stripped
        let _results = store.search_facts("alpha; DROP TABLE;", 10);
        // Should not crash — only "alpha" remains after sanitization
    }

    #[test]
    fn test_search_preserves_underscore() {
        let store = make_store();
        store.add_fact("hello_world test", "test");
        let results = store.search_facts("hello_world", 10);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].content, "hello_world test");
    }

    #[test]
    fn test_probe_with_entity() {
        let store = make_store();
        let id = store
            .add_fact("React Router v7 introduces new features", "tech")
            .unwrap();

        // Probe should find via entity link
        let results = store.probe("React Router", 10);
        assert!(!results.is_empty());
        assert!(results.iter().any(|r| r.fact_id == id));
    }

    #[test]
    fn test_probe_fallback_to_fts() {
        let store = make_store();
        store.add_fact(
            "This is about React Router but with no entity recognition here",
            "tech",
        );

        // Entity search may not find it (no capitalized phrase extracted since
        // the whole sentence starts with "This"), but FTS5 fallback should
        let results = store.probe("React Router", 10);
        // FTS5 search on the content should find it
        assert!(!results.is_empty());
    }

    #[test]
    fn test_related() {
        let store = make_store();
        // Two facts sharing an entity
        store.add_fact("React Router handles client-side routing", "frontend");
        store.add_fact("React Router v7 is the latest version", "frontend");
        store.add_fact("Express is a Node.js server framework", "backend");

        let results = store.related("React Router", 10);
        assert!(!results.is_empty());
        // Should find both React Router facts
        assert!(results.len() >= 2);
    }

    #[test]
    fn test_list_recent_facts() {
        let store = make_store();
        store.add_fact("old fact", "tag1");
        store.add_fact("newer fact", "tag2");
        store.add_fact("newest fact", "tag3");

        let recent = store.list_recent_facts(2, 0.0);
        assert_eq!(recent.len(), 2);
        // Most recent first
        assert_eq!(recent[0].content, "newest fact");
    }

    #[test]
    fn test_get_fact_entities() {
        let store = make_store();
        let id = store
            .add_fact(
                "\"Alice Smith\" and \"Bob Jones\" collaborated on \"Project X\"",
                "team",
            )
            .unwrap();

        let entities = store.get_fact_entities(id);
        assert!(!entities.is_empty());
        assert!(entities.iter().any(|e| e == "Alice Smith"));
        assert!(entities.iter().any(|e| e == "Bob Jones"));
        assert!(entities.iter().any(|e| e == "Project X"));
    }

    #[test]
    fn test_record_feedback() {
        let store = make_store();
        let id = store.add_fact("test fact", "test").unwrap();

        let (old, new) = store.record_feedback(id, true).unwrap();
        assert!((old - 0.5).abs() < f64::EPSILON);
        assert!((new - 0.55).abs() < 0.001);

        let (old2, new2) = store.record_feedback(id, false).unwrap();
        assert!((old2 - 0.55).abs() < 0.001);
        assert!((new2 - 0.45).abs() < 0.001);
    }

    #[test]
    fn test_get_fact_count() {
        let store = make_store();
        assert_eq!(store.get_fact_count(), 0);
        store.add_fact("fact 1", "tag");
        store.add_fact("fact 2", "tag");
        assert_eq!(store.get_fact_count(), 2);
    }

    #[test]
    fn test_entity_upsert_purges_stale_links() {
        let store = make_store();
        // First add — double-quoted names to trigger entity extraction
        let id = store
            .add_fact(
                "\"Alice Smith\" wrote a paper about \"Quantum Computing\"",
                "research",
            )
            .unwrap();
        let entities1 = store.get_fact_entities(id);
        assert!(entities1.iter().any(|e| e == "Alice Smith"));
        assert!(entities1.iter().any(|e| e == "Quantum Computing"));
        assert_eq!(entities1.len(), 2);

        // Update (same content triggers upsert)
        store.add_fact(
            "\"Alice Smith\" wrote a paper about \"Quantum Computing\"",
            "research",
        );
        let entities2 = store.get_fact_entities(id);
        // Should not have duplicate links after re-extraction
        let smith_count = entities2.iter().filter(|e| *e == "Alice Smith").count();
        assert_eq!(
            smith_count, 1,
            "Entity links should not be duplicated on re-extraction"
        );
        let qc_count = entities2
            .iter()
            .filter(|e| *e == "Quantum Computing")
            .count();
        assert_eq!(
            qc_count, 1,
            "Entity links should not be duplicated on re-extraction"
        );
    }

    #[test]
    fn test_dispose() {
        let mut store = make_store();
        assert!(store.is_available);
        store.dispose();
        assert!(!store.is_available);
    }
}
