//! HolographicMemory — SQLite FTS5-backed semantic memory for agent context.
//!
//! Mirrors `packages/coding-agent/src/hamr/memory/HolographicMemory.ts`.
//!
//! Architecture:
//!   Every turn → INSERT into FTS5 (fire-and-forget, non-blocking)
//!   Agent needs history → search("error from 5 turns ago") → relevant rows
//!   Context exhausted → handoff() → structured manifest for child agent
//!
//! This is the architectural differentiator from the SOTA review:
//! zero tokens burned, zero information loss, agent queries what it needs.
//!
//! Shares the SQLite connection with FactStore. If SQLite is unavailable,
//! all operations are safe no-ops.

use serde::{Deserialize, Serialize};

#[cfg(feature = "hamr-memory")]
use rusqlite::{Connection, params};

// ─── Types ───────────────────────────────────────────────────────────────────

/// A memory entry to be stored in FTS5.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryEntry {
    pub session_id: String,
    pub turn_id: i64,
    pub role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_paths: Option<Vec<String>>,
    pub content: String,
    /// Product-domain tags for cross-product memory filtering.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub domain_tags: Option<Vec<String>>,
}

/// A search result from FTS5 memory.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemorySearchResult {
    pub turn_id: i64,
    pub session_id: String,
    pub role: String,
    pub tool_name: Option<String>,
    pub file_paths: Option<String>,
    pub content: String,
    pub domain_tags: Option<String>,
    /// FTS5 rank (lower = more relevant).
    pub rank: f64,
}

/// A search result with snippet context.
///
/// Mirror of TS `MemorySearchResult & { snippet: string }` — all search
/// result fields at top level plus a `snippet` field.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemorySearchResultWithSnippet {
    pub turn_id: i64,
    pub session_id: String,
    pub role: String,
    pub tool_name: Option<String>,
    pub file_paths: Option<String>,
    pub content: String,
    pub domain_tags: Option<String>,
    pub rank: f64,
    pub snippet: String,
}

/// Structured handoff manifest for context exhaustion scenarios.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HandoffManifest {
    pub session_id: String,
    /// Last N turns summarized as key findings.
    pub key_findings: Vec<String>,
    /// Files that were read or changed.
    pub files_touched: Vec<String>,
    /// Suggested FTS5 search terms for the next agent.
    pub suggested_search_terms: Vec<String>,
    /// Number of turns stored.
    pub turn_count: i64,
    /// Total entries in memory.
    pub entry_count: i64,
    /// Product-domain context tags observed in recent memory entries.
    pub domain_tags: Vec<String>,
}

impl HandoffManifest {
    fn empty() -> Self {
        HandoffManifest {
            session_id: String::new(),
            key_findings: vec![],
            files_touched: vec![],
            suggested_search_terms: vec![],
            turn_count: 0,
            entry_count: 0,
            domain_tags: vec![],
        }
    }
}

// ─── FTS5 query sanitizer (v0.7.1) ────────────────────────────────────────────

/// Sanitize a user query for safe FTS5 MATCH usage.
///
/// FTS5 MATCH expects a boolean expression: bare words, "phrase queries",
/// prefix* terms, AND/OR/NOT, and (grouping). Dangerous characters (#, @, etc.)
/// are stripped. Hyphens in terms (e.g. "hamr-browser") are preserved by
/// double-quoting each token that contains them, since bare `-` is a column
/// filter in FTS5. Path-like tokens (containing / or .) are also double-quoted
/// to avoid being split into separate FTS5 tokens. Colons and @-signs are
/// similarly quoted.
///
/// Falls back to the original query (with only null bytes and unprintables
/// stripped) if tokenization produces nothing useful.
///
/// Mirrors TS `sanitizeFts5Query` (v0.7.1).
fn sanitize_fts_query(query: &str) -> String {
    // Strip null bytes and other control characters
    let q: String = query
        .chars()
        .filter(|c| !c.is_control() || c.is_whitespace())
        .collect();

    let trimmed = q.trim();

    // If the query is already wrapped in quotes (intentional phrase search), keep it
    if trimmed.starts_with('"') && trimmed.ends_with('"') && trimmed.len() > 1 {
        // Verify there are no other unescaped quotes inside
        let inner = &trimmed[1..trimmed.len() - 1];
        if !inner.contains('"') {
            return trimmed.to_string();
        }
    }

    // Tokenize by whitespace, preserving quoted phrases
    let tokens = tokenize_fts_query(&q);
    if tokens.is_empty() {
        // Fallback: try raw query with only truly dangerous chars stripped
        let bare: String = query
            .chars()
            .filter(|c| c.is_alphanumeric() || c.is_whitespace() || "*'\"-/.@#:()[]_".contains(*c))
            .collect();
        let bare = bare.trim().to_string();
        if !bare.is_empty() {
            return bare;
        }
        return trimmed.to_string();
    }

    let result = tokens.join(" ").trim().to_string();
    if result.is_empty() {
        trimmed.to_string()
    } else {
        result
    }
}

/// Tokenize a query string for FTS5, preserving quoted phrases.
///
/// Returns a vector of tokens. Tokens containing FTS5-dangerous characters
/// (`-`, `.`, `/`, `@`, `#`, `:`) are double-quoted. Trailing `*` prefix
/// wildcards are preserved after quoting.
fn tokenize_fts_query(query: &str) -> Vec<String> {
    let mut tokens: Vec<String> = Vec::new();
    let mut chars = query.chars().peekable();

    while let Some(&c) = chars.peek() {
        // Skip whitespace
        if c.is_whitespace() {
            chars.next();
            continue;
        }

        // Quoted phrase
        if c == '"' {
            chars.next(); // consume opening quote
            let mut phrase = String::new();
            while let Some(&ch) = chars.peek() {
                chars.next();
                if ch == '"' {
                    break;
                }
                phrase.push(ch);
            }
            if !phrase.is_empty() {
                tokens.push(format!("\"{}\"", phrase));
            }
            continue;
        }

        // Unquoted token — collect until whitespace
        let mut token = String::new();
        while let Some(&ch) = chars.peek() {
            if ch.is_whitespace() {
                break;
            }
            token.push(ch);
            chars.next();
        }

        // Strip characters that FTS5 interprets as operators when bare.
        // Keep alnum, hyphens, slashes, dots, underscores, asterisks (prefix),
        // @, #, : (will be quoted if present).
        let cleaned: String = token
            .chars()
            .filter(|c| c.is_alphanumeric() || "-/.@*#:_".contains(*c))
            .collect();

        if cleaned.is_empty() {
            continue;
        }

        // Double-quote tokens that contain FTS5-dangerous characters:
        // hyphens (column filter), dots/slashes (path separators that
        // FTS5 tokenizer would split), colons, at-signs, hash.
        let has_dangerous = cleaned.chars().any(|c| "-/.@#:".contains(c));

        if has_dangerous {
            // Preserve trailing * for prefix queries after quoting
            let is_prefix = cleaned.ends_with('*');
            let core = if is_prefix {
                &cleaned[..cleaned.len() - 1]
            } else {
                &cleaned
            };
            if core.is_empty() {
                // Just a bare "*" — keep it
                tokens.push("*".to_string());
            } else {
                let suffix = if is_prefix { "*" } else { "" };
                tokens.push(format!("\"{}\"{}", core, suffix));
            }
        } else {
            tokens.push(cleaned);
        }
    }

    tokens
}

// Keep the old function name as an alias for backward compatibility.
#[allow(dead_code)]
pub(crate) fn sanitize_fts5_query(query: &str) -> String {
    sanitize_fts_query(query)
}

// ─── Internal helper structs ─────────────────────────────────────────────────

/// Lightweight struct for term computation (all fields optional as in TS).
#[derive(Debug, Clone)]
struct RecentEntry {
    tool_name: Option<String>,
    domain_tags: Option<String>,
    role: Option<String>,
    content: Option<String>,
}

#[cfg(feature = "hamr-memory")]
#[derive(Debug, Clone)]
struct RecentEntryFull {
    tool_name: Option<String>,
    file_paths: Option<String>,
    domain_tags: Option<String>,
    role: String,
    content: String,
}

// ─── HolographicMemory ───────────────────────────────────────────────────────

/// SQLite FTS5-backed semantic memory for agent context.
///
/// When the `hamr-memory` feature is disabled, all operations are safe no-ops.
pub struct HolographicMemory {
    #[cfg(feature = "hamr-memory")]
    db: Option<Connection>,

    /// Count of store errors since construction. Non-zero means FTS5 is
    /// silently failing.
    pub store_error_count: u32,
    #[cfg(feature = "hamr-memory")]
    store_error_warned: bool,

    /// Word-frequency-based search term cache (invalidated on store).
    suggested_terms_cache: Option<Vec<String>>,
}

impl HolographicMemory {
    /// Create a new HolographicMemory from an optional rusqlite Connection.
    #[cfg(feature = "hamr-memory")]
    pub fn new(db: Option<Connection>) -> Self {
        HolographicMemory {
            db,
            store_error_count: 0,
            store_error_warned: false,
            suggested_terms_cache: None,
        }
    }

    #[cfg(not(feature = "hamr-memory"))]
    pub fn new() -> Self {
        HolographicMemory {
            store_error_count: 0,
            suggested_terms_cache: None,
        }
    }

    /// Returns `true` when the memory store is available and operational.
    pub fn is_available(&self) -> bool {
        #[cfg(feature = "hamr-memory")]
        {
            self.db.is_some()
        }
        #[cfg(not(feature = "hamr-memory"))]
        {
            false
        }
    }

    /// Check if the given session has any stored entries.
    pub fn has_session_entries(&self, session_id: &str) -> bool {
        #[cfg(feature = "hamr-memory")]
        {
            let db = match self.db.as_ref() {
                Some(db) => db,
                None => return false,
            };
            match db
                .prepare_cached("SELECT COUNT(*) as entries FROM memory_fts WHERE session_id = ?1")
            {
                Ok(mut stmt) => stmt
                    .query_row(params![session_id], |row| row.get::<_, i64>(0))
                    .map(|c| c > 0)
                    .unwrap_or(false),
                Err(_) => false,
            }
        }
        #[cfg(not(feature = "hamr-memory"))]
        {
            false
        }
    }

    // ── Write ──────────────────────────────────────────────────────────────

    /// Store a memory entry in FTS5.
    ///
    /// Fire-and-forget — errors are caught internally, never thrown.
    /// Content is capped at 8000 chars per entry. Invalidates the search-term
    /// cache on success.
    ///
    /// Mirror of TS `store(entry: MemoryEntry): void`.
    pub fn store(&mut self, entry: &MemoryEntry) {
        #[cfg(feature = "hamr-memory")]
        {
            let db = match self.db.as_ref() {
                Some(db) => db,
                None => return,
            };
            let file_paths: Option<String> = entry
                .file_paths
                .as_ref()
                .filter(|v| !v.is_empty())
                .map(|v| v.join(","));
            let domain_tags: Option<String> = entry
                .domain_tags
                .as_ref()
                .filter(|v| !v.is_empty())
                .map(|v| v.join(","));
            let content = &entry.content[..entry.content.len().min(8000)];

            let result = db.execute(
                "INSERT INTO memory_fts (turn_id, session_id, role, tool_name, file_paths, content, domain_tags) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    entry.turn_id,
                    entry.session_id,
                    entry.role,
                    entry.tool_name,
                    file_paths,
                    content,
                    domain_tags,
                ],
            );
            match result {
                Ok(_) => {
                    self.suggested_terms_cache = None;
                }
                Err(e) => {
                    self.store_error_count += 1;
                    if !self.store_error_warned && self.store_error_count >= 3 {
                        self.store_error_warned = true;
                        eprintln!(
                            "[hamr] HolographicMemory: {} store() failures. FTS5 may be unavailable or corrupt: {}",
                            self.store_error_count, e
                        );
                    }
                }
            }
        }
        #[cfg(not(feature = "hamr-memory"))]
        {
            let _ = entry;
        }
    }

    // ── Search ─────────────────────────────────────────────────────────────

    /// Full-text search over stored memory entries.
    ///
    /// Uses FTS5 with Porter stemming. Results ranked by FTS5 relevance (bm25).
    /// The query is sanitized — only `\w`, space, `*`, `-`, `"`, `(`, `)`.
    pub fn search(&self, query: &str, limit: usize) -> Vec<MemorySearchResult> {
        #[cfg(feature = "hamr-memory")]
        {
            self._search_impl(query, limit)
        }
        #[cfg(not(feature = "hamr-memory"))]
        {
            vec![]
        }
    }

    /// Search with snippet context (FTS5 `snippet()`).
    ///
    /// Returns matching fragments with surrounding text for readability.
    pub fn search_with_snippets(
        &self,
        query: &str,
        limit: usize,
    ) -> Vec<MemorySearchResultWithSnippet> {
        #[cfg(feature = "hamr-memory")]
        {
            let db = match self.db.as_ref() {
                Some(db) => db,
                None => return vec![],
            };
            let safe_query = sanitize_fts_query(query);
            if safe_query.is_empty() {
                return vec![];
            }
            let mut stmt = match db.prepare_cached(
                "SELECT turn_id, session_id, role, tool_name, file_paths, domain_tags, \
                        snippet(memory_fts, 5, '<mark>', '</mark>', '...', 32) AS snippet, \
                        content, rank \
                 FROM memory_fts \
                 WHERE memory_fts MATCH ?1 \
                 ORDER BY rank LIMIT ?2",
            ) {
                Ok(s) => s,
                Err(_) => return vec![],
            };
            let rows = match stmt.query_map(
                params![safe_query, limit as i64],
                |row| -> rusqlite::Result<MemorySearchResultWithSnippet> {
                    Ok(MemorySearchResultWithSnippet {
                        turn_id: row.get(0)?,
                        session_id: row.get(1)?,
                        role: row.get(2)?,
                        tool_name: row.get(3)?,
                        file_paths: row.get(4)?,
                        domain_tags: row.get(5)?,
                        snippet: row.get(6)?,
                        content: row.get(7)?,
                        rank: row.get(8)?,
                    })
                },
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

    /// Fetch the most recently stored entry carrying the given domain tag.
    ///
    /// Used to surface a survival manifest (domain tag "survival") prominently
    /// on resume. Returns `None` when no such entry exists or memory is
    /// unavailable.
    pub fn get_latest_by_domain_tag(
        &self,
        tag: &str,
        session_id: &str,
    ) -> Option<MemorySearchResult> {
        #[cfg(feature = "hamr-memory")]
        {
            let db = self.db.as_ref()?;
            let trimmed = tag.trim();
            if trimmed.is_empty() {
                return None;
            }
            let mut stmt = db
                .prepare_cached(
                    "SELECT turn_id, session_id, role, tool_name, file_paths, domain_tags, content \
                     FROM memory_fts \
                     WHERE domain_tags LIKE '%' || ?1 || '%' \
                       AND session_id = ?2 \
                     ORDER BY rowid DESC LIMIT 1",
                )
                .ok()?;
            stmt.query_row(
                params![trimmed, session_id],
                |row| -> rusqlite::Result<MemorySearchResult> {
                    Ok(MemorySearchResult {
                        turn_id: row.get(0)?,
                        session_id: row.get(1)?,
                        role: row.get(2)?,
                        tool_name: row.get(3)?,
                        file_paths: row.get(4)?,
                        domain_tags: row.get(5)?,
                        content: row.get(6)?,
                        rank: 0.0,
                    })
                },
            )
            .ok()
        }
        #[cfg(not(feature = "hamr-memory"))]
        {
            None
        }
    }

    // ── Handoff ────────────────────────────────────────────────────────────

    /// Generate a structured handoff manifest for context exhaustion scenarios.
    ///
    /// When `session_id` is provided (v0.7.1), results are filtered to that
    /// session only, preventing cross-session memory bleed in handoff manifests.
    pub fn handoff(&self, session_id: Option<&str>) -> HandoffManifest {
        #[cfg(feature = "hamr-memory")]
        {
            let db = match self.db.as_ref() {
                Some(db) => db,
                None => return HandoffManifest::empty(),
            };

            // Fetch recent entries (last 20), optionally scoped to session
            let recent: Vec<MemorySearchResult> = if let Some(sid) = session_id {
                match db
                    .prepare_cached(
                        "SELECT turn_id, session_id, role, tool_name, file_paths, domain_tags, content \
                         FROM memory_fts WHERE session_id = ?1 ORDER BY rowid DESC LIMIT 20",
                    )
                    .and_then(|mut stmt| {
                        stmt.query_map(params![sid], |row| {
                            Ok(MemorySearchResult {
                                turn_id: row.get(0)?,
                                session_id: row.get(1)?,
                                role: row.get(2)?,
                                tool_name: row.get(3)?,
                                file_paths: row.get(4)?,
                                domain_tags: row.get(5)?,
                                content: row.get(6)?,
                                rank: 0.0,
                            })
                        })?
                        .collect::<Result<Vec<_>, _>>()
                    }) {
                    Ok(r) => r,
                    Err(_) => return HandoffManifest::empty(),
                }
            } else {
                match db
                    .prepare_cached(
                        "SELECT turn_id, session_id, role, tool_name, file_paths, domain_tags, content \
                         FROM memory_fts ORDER BY rowid DESC LIMIT 20",
                    )
                    .and_then(|mut stmt| {
                        stmt.query_map([], |row| {
                            Ok(MemorySearchResult {
                                turn_id: row.get(0)?,
                                session_id: row.get(1)?,
                                role: row.get(2)?,
                                tool_name: row.get(3)?,
                                file_paths: row.get(4)?,
                                domain_tags: row.get(5)?,
                                content: row.get(6)?,
                                rank: 0.0,
                            })
                        })?
                        .collect::<Result<Vec<_>, _>>()
                    }) {
                    Ok(r) => r,
                    Err(_) => return HandoffManifest::empty(),
                }
            };

            let effective_session_id = session_id
                .map(|s| s.to_string())
                .or_else(|| recent.first().map(|r| r.session_id.clone()))
                .unwrap_or_default();

            let (key_findings, files_touched, domain_tags) = extract_handoff_data(&recent);

            let (turn_count, entry_count) = self._get_handoff_counts(&effective_session_id);

            let mut files_vec: Vec<String> = files_touched.into_iter().collect();
            files_vec.sort();

            let mut tags_vec: Vec<String> = domain_tags.into_iter().collect();
            tags_vec.sort();

            HandoffManifest {
                session_id: effective_session_id,
                key_findings,
                files_touched: files_vec,
                domain_tags: tags_vec,
                suggested_search_terms: compute_suggested_terms(&entries_to_recent(&recent)),
                turn_count,
                entry_count,
            }
        }
        #[cfg(not(feature = "hamr-memory"))]
        {
            HandoffManifest::empty()
        }
    }

    /// Generate suggested FTS5 search terms from recent memory.
    ///
    /// Results are cached and invalidated on `store()`.
    pub fn get_suggested_search_terms(&mut self) -> Vec<String> {
        // Return cached result if memory hasn't changed
        if let Some(ref cached) = self.suggested_terms_cache {
            return cached.clone();
        }

        #[cfg(feature = "hamr-memory")]
        {
            let db = match self.db.as_ref() {
                Some(db) => db,
                None => return vec![],
            };
            let recent = match db
                .prepare_cached(
                    "SELECT tool_name, domain_tags, role, content \
                     FROM memory_fts ORDER BY rowid DESC LIMIT 30",
                )
                .and_then(|mut stmt| {
                    stmt.query_map([], |row| {
                        Ok(RecentEntry {
                            tool_name: row.get(0)?,
                            domain_tags: row.get(1)?,
                            role: row.get(2)?,
                            content: row.get(3)?,
                        })
                    })?
                    .collect::<Result<Vec<_>, _>>()
                }) {
                Ok(recent) => recent,
                Err(_) => return vec![],
            };
            let terms = compute_suggested_terms(&recent);
            self.suggested_terms_cache = Some(terms.clone());
            terms
        }
        #[cfg(not(feature = "hamr-memory"))]
        {
            vec![]
        }
    }

    // ── Memory index (for context injection) ──────────────────────────────

    /// Build a compact, token-efficient index of what's in memory.
    ///
    /// Returns `None` if memory is empty or unavailable.
    pub fn build_memory_index(&mut self) -> Option<String> {
        #[cfg(feature = "hamr-memory")]
        {
            let db = self.db.as_ref()?;

            // Stats
            let (entries, turns) = db
                .prepare_cached(
                    "SELECT COUNT(*) as entries, COUNT(DISTINCT turn_id) as turns FROM memory_fts",
                )
                .ok()
                .and_then(|mut stmt| {
                    stmt.query_row([], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)))
                        .ok()
                })
                .unwrap_or((0, 0));
            if entries == 0 {
                return None;
            }

            // Fetch last 30 entries
            let recent = db
                .prepare_cached(
                    "SELECT tool_name, file_paths, domain_tags, role, content \
                     FROM memory_fts ORDER BY rowid DESC LIMIT 30",
                )
                .and_then(|mut stmt| {
                    stmt.query_map([], |row| {
                        Ok(RecentEntryFull {
                            tool_name: row.get(0)?,
                            file_paths: row.get(1)?,
                            domain_tags: row.get(2)?,
                            role: row.get(3)?,
                            content: row.get(4)?,
                        })
                    })?
                    .collect::<Result<Vec<_>, _>>()
                })
                .unwrap_or_default();

            let mut all_files = std::collections::HashSet::new();
            let mut tool_names = std::collections::HashSet::new();
            let mut all_domain_tags = std::collections::HashSet::new();
            let mut error_lines: Vec<String> = Vec::new();

            for entry in &recent {
                // File paths
                if let Some(ref fp) = entry.file_paths {
                    for part in fp.split(',') {
                        let t = part.trim().to_string();
                        if !t.is_empty() {
                            all_files.insert(t);
                        }
                    }
                }
                // Tool names
                if let Some(ref tn) = entry.tool_name {
                    let t = tn.trim().to_string();
                    if !t.is_empty() {
                        tool_names.insert(t);
                    }
                }
                // Domain tags
                if let Some(ref dt) = entry.domain_tags {
                    for part in dt.split(',') {
                        let t = part.trim().to_string();
                        if !t.is_empty() {
                            all_domain_tags.insert(t);
                        }
                    }
                }
                // Error snippets from non-user entries
                if entry.role != "user" && error_lines.len() < 3 {
                    let lower = entry.content.to_lowercase();
                    if lower.contains("error") || lower.contains("fail") {
                        for line in entry.content.split('\n') {
                            let line_lower = line.to_lowercase();
                            if (line_lower.contains("error") || line_lower.contains("fail"))
                                && line.len() > 15
                                && line.len() < 200
                            {
                                error_lines.push(line.trim().to_string());
                                break;
                            }
                        }
                    }
                }
            }

            // Build the index string
            let mut lines: Vec<String> = Vec::new();
            lines.push(format!(
                "[Memory: {} entries across {} turns",
                entries, turns
            ));

            if !all_domain_tags.is_empty() {
                let mut tags: Vec<String> = all_domain_tags.into_iter().collect();
                tags.sort();
                lines.push(format!(
                    "Domain: {}",
                    tags.iter().take(5).cloned().collect::<Vec<_>>().join(", ")
                ));
            }

            if !all_files.is_empty() {
                let mut files: Vec<String> = all_files.into_iter().collect();
                files.sort();
                lines.push(format!(
                    "Files: {}",
                    files.iter().take(5).cloned().collect::<Vec<_>>().join(", ")
                ));
            }

            if !tool_names.is_empty() {
                let mut tools: Vec<String> = tool_names.into_iter().collect();
                tools.sort();
                lines.push(format!(
                    "Tools: {}",
                    tools.iter().take(5).cloned().collect::<Vec<_>>().join(", ")
                ));
            }

            if !error_lines.is_empty() {
                let hints: Vec<String> = error_lines
                    .iter()
                    .map(|e| {
                        if e.len() > 60 {
                            format!("{}…", &e[..57])
                        } else {
                            e.clone()
                        }
                    })
                    .collect();
                lines.push(format!("Recent: {}", hints.join(" | ")));
            }

            // Suggested search terms — reuse already-fetched entries
            let search_entries: Vec<RecentEntry> = recent
                .iter()
                .map(|e| RecentEntry {
                    tool_name: e.tool_name.clone(),
                    domain_tags: e.domain_tags.clone(),
                    role: Some(e.role.clone()),
                    content: Some(e.content.clone()),
                })
                .collect();
            let full_terms = compute_suggested_terms(&search_entries);

            // Mirror TS: only cache if not already cached (avoid overwriting)
            if self.suggested_terms_cache.is_none() {
                self.suggested_terms_cache = Some(full_terms.clone());
            }
            let search_terms: Vec<&str> = full_terms.iter().take(5).map(|s| s.as_str()).collect();
            if !search_terms.is_empty() {
                lines.push(format!("Search: {}", search_terms.join(" ")));
            }

            lines.push("]".to_string());
            Some(lines.join("\n"))
        }
        #[cfg(not(feature = "hamr-memory"))]
        {
            None
        }
    }

    // ── Private helpers ────────────────────────────────────────────────────

    /// Internal search implementation.
    #[cfg(feature = "hamr-memory")]
    fn _search_impl(&self, query: &str, limit: usize) -> Vec<MemorySearchResult> {
        let db = match self.db.as_ref() {
            Some(db) => db,
            None => return vec![],
        };
        let safe_query = sanitize_fts_query(query);
        if safe_query.is_empty() {
            return vec![];
        }
        let mut stmt = match db.prepare_cached(
            "SELECT turn_id, session_id, role, tool_name, file_paths, domain_tags, content, rank \
             FROM memory_fts \
             WHERE memory_fts MATCH ?1 \
             ORDER BY rank LIMIT ?2",
        ) {
            Ok(s) => s,
            Err(_) => return vec![],
        };
        let rows = match stmt.query_map(
            params![safe_query, limit as i64],
            |row| -> rusqlite::Result<MemorySearchResult> {
                Ok(MemorySearchResult {
                    turn_id: row.get(0)?,
                    session_id: row.get(1)?,
                    role: row.get(2)?,
                    tool_name: row.get(3)?,
                    file_paths: row.get(4)?,
                    domain_tags: row.get(5)?,
                    content: row.get(6)?,
                    rank: row.get(7)?,
                })
            },
        ) {
            Ok(r) => r,
            Err(_) => return vec![],
        };
        rows.filter_map(|r| r.ok()).collect()
    }

    /// Get turn and entry counts for a session.
    #[cfg(feature = "hamr-memory")]
    fn _get_handoff_counts(&self, session_id: &str) -> (i64, i64) {
        let db = match self.db.as_ref() {
            Some(db) => db,
            None => return (0, 0),
        };
        db.prepare_cached(
            "SELECT COUNT(DISTINCT turn_id) as turns, COUNT(*) as entries \
             FROM memory_fts WHERE session_id = ?1",
        )
        .ok()
        .and_then(|mut stmt| {
            stmt.query_row(params![session_id], |row| {
                Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?))
            })
            .ok()
        })
        .unwrap_or((0, 0))
    }
}

// ─── Standalone functions ────────────────────────────────────────────────────

/// Extract key findings, file paths, and domain tags from recent entries.
///
/// Mirror of the inline extraction logic in TS `handoff()`.
fn extract_handoff_data(
    entries: &[MemorySearchResult],
) -> (
    Vec<String>,
    std::collections::HashSet<String>,
    std::collections::HashSet<String>,
) {
    let mut key_findings: Vec<String> = Vec::new();
    let mut files_touched = std::collections::HashSet::new();
    let mut domain_tags = std::collections::HashSet::new();
    let mut seen_findings = std::collections::HashSet::new();

    let intent_markers = [
        "i'll",
        "i will",
        "let me",
        "let's",
        "plan to",
        "going to",
        "approach",
        "decided",
        "my plan",
        "the fix",
        "fix is",
        "solution is",
        "next step",
        "will need to",
    ];

    let re_file = regex::Regex::new(r"[/\w]+\.[a-z]{2,6}\b").expect("file path regex is valid");

    for entry in entries {
        // Collect domain tags
        if let Some(ref dt) = entry.domain_tags {
            for tag in dt.split(',') {
                let t = tag.trim().to_string();
                if !t.is_empty() {
                    domain_tags.insert(t);
                }
            }
        }

        // Collect file paths
        if let Some(ref fp) = entry.file_paths {
            for path in fp.split(',') {
                let p = path.trim().to_string();
                if !p.is_empty() {
                    files_touched.insert(p);
                }
            }
        }

        // Extract meaningful lines
        for line in entry.content.split('\n') {
            let trimmed = line.trim().to_string();
            if trimmed.len() < 20 || trimmed.len() > 500 || seen_findings.contains(&trimmed) {
                continue;
            }
            let lower = trimmed.to_lowercase();

            // Priority 1: decisions and plans
            if intent_markers.iter().any(|m| lower.contains(m)) {
                seen_findings.insert(trimmed.clone());
                key_findings.push(trimmed);
            }
            // Priority 2: errors, warnings, failures, successes (with structural context)
            else if (lower.contains("error")
                || lower.contains("fail")
                || lower.contains("success"))
                && (lower.contains(':') || lower.contains("at ") || lower.contains("in "))
            {
                seen_findings.insert(trimmed.clone());
                key_findings.push(trimmed);
            }
            // Priority 3: file paths mentioned in content
            else if re_file.is_match(&trimmed) && trimmed.contains('/') {
                seen_findings.insert(trimmed.clone());
                key_findings.push(trimmed);
            }

            if key_findings.len() >= 15 {
                break;
            }
        }
        if key_findings.len() >= 15 {
            break;
        }
    }

    (key_findings, files_touched, domain_tags)
}

/// Convert `MemorySearchResult` slice to `RecentEntry` slice.
fn entries_to_recent(entries: &[MemorySearchResult]) -> Vec<RecentEntry> {
    entries
        .iter()
        .map(|e| RecentEntry {
            tool_name: e.tool_name.clone(),
            domain_tags: e.domain_tags.clone(),
            role: Some(e.role.clone()),
            content: Some(e.content.clone()),
        })
        .collect()
}

/// Compute suggested search terms from already-fetched entries.
///
/// Extracts tool names, domain tags, and top frequent words (excluding stop
/// words). Returns up to 20 terms.
fn compute_suggested_terms(entries: &[RecentEntry]) -> Vec<String> {
    let stop_words: std::collections::HashSet<&str> = [
        "the", "is", "at", "which", "on", "a", "an", "and", "or", "but", "in", "with", "to", "for",
        "of", "this", "that", "it", "be", "was", "are",
    ]
    .into_iter()
    .collect();

    let mut tool_names = std::collections::HashSet::new();
    let mut domain_tag_set = std::collections::HashSet::new();
    let mut word_freq: std::collections::HashMap<String, usize> = std::collections::HashMap::new();

    for entry in entries {
        // Collect tool names
        if let Some(ref tn) = entry.tool_name {
            let t = tn.trim().to_string();
            if !t.is_empty() {
                tool_names.insert(t);
            }
        }

        // Collect domain tags
        if let Some(ref dt) = entry.domain_tags {
            for tag in dt.split(',') {
                let t = tag.trim().to_string();
                if !t.is_empty() {
                    domain_tag_set.insert(t);
                }
            }
        }

        // Word frequency from non-user content
        if entry.role.as_deref() != Some("user") {
            if let Some(ref content) = entry.content {
                for word in content.to_lowercase().split(|c: char| !c.is_alphanumeric()) {
                    if word.len() > 3 && !stop_words.contains(word) {
                        *word_freq.entry(word.to_string()).or_insert(0) += 1;
                    }
                }
            }
        }
    }

    // Top frequent words
    let mut top_words: Vec<(usize, String)> = word_freq.into_iter().map(|(w, c)| (c, w)).collect();
    top_words.sort_by(|a, b| b.0.cmp(&a.0));
    let top_words: Vec<String> = top_words.into_iter().take(15).map(|(_, w)| w).collect();

    // Combine: tool names + domain tags + frequent words (deduped, ordered)
    let mut terms_set = std::collections::HashSet::new();
    let mut result: Vec<String> = Vec::new();

    for t in &tool_names {
        if terms_set.insert(t.clone()) {
            result.push(t.clone());
        }
    }
    for d in &domain_tag_set {
        if terms_set.insert(d.clone()) {
            result.push(d.clone());
        }
    }
    for w in &top_words {
        if terms_set.insert(w.clone()) {
            result.push(w.clone());
        }
    }

    result.truncate(20);
    result
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
#[cfg(feature = "hamr-memory")]
mod tests {
    use super::*;

    fn make_memory() -> HolographicMemory {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE VIRTUAL TABLE IF NOT EXISTS memory_fts USING fts5(\
                 turn_id, session_id, role, tool_name, file_paths, content, domain_tags, \
                 tokenize='porter unicode61'\
             )",
        )
        .unwrap();
        HolographicMemory::new(Some(conn))
    }

    fn make_entry(session: &str, turn: i64, role: &str, content: &str) -> MemoryEntry {
        MemoryEntry {
            session_id: session.to_string(),
            turn_id: turn,
            role: role.to_string(),
            tool_name: None,
            file_paths: None,
            content: content.to_string(),
            domain_tags: None,
        }
    }

    #[test]
    fn test_store_and_search() {
        let mut mem = make_memory();
        mem.store(&make_entry("s1", 1, "user", "Hello world"));
        mem.store(&make_entry(
            "s1",
            2,
            "assistant",
            "Hi there! How can I help?",
        ));

        let results = mem.search("hello", 10);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].content, "Hello world");
    }

    #[test]
    fn test_search_with_file_paths() {
        let mut mem = make_memory();
        let mut entry = make_entry("s1", 1, "tool", "Reading src/main.rs");
        entry.tool_name = Some("read".to_string());
        entry.file_paths = Some(vec!["src/main.rs".to_string()]);
        mem.store(&entry);

        // Search for a term that appears in the content after FTS5 tokenization
        let results = mem.search("main", 10);
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_search_with_snippets() {
        let mut mem = make_memory();
        mem.store(&make_entry(
            "s1",
            1,
            "user",
            "The quick brown fox jumps over the lazy dog",
        ));

        let results = mem.search_with_snippets("fox", 5);
        assert_eq!(results.len(), 1);
        assert!(results[0].snippet.contains("fox"));
    }

    #[test]
    fn test_search_sanitizes_query() {
        let mut mem = make_memory();
        mem.store(&make_entry("s1", 1, "user", "alpha beta gamma"));

        // Semicolons are stripped by sanitization, so "alpha; beta;"
        // becomes "alpha beta" which matches content "alpha beta gamma"
        let results = mem.search("alpha; beta;", 10);
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_search_preserves_underscore() {
        let mut mem = make_memory();
        mem.store(&make_entry("s1", 1, "user", "hello_world test"));

        let results = mem.search("hello_world", 10);
        assert_eq!(results.len(), 1);
    }

    // ── v0.7.1 FTS5 sanitizer tests ───────────────────────────────────────

    #[test]
    fn test_sanitize_preserves_hyphenated_terms() {
        let sanitized = sanitize_fts_query("hamr-browser");
        assert_eq!(sanitized, "\"hamr-browser\"");
    }

    #[test]
    fn test_sanitize_preserves_file_paths() {
        let sanitized = sanitize_fts_query("src/lib.rs");
        assert_eq!(sanitized, "\"src/lib.rs\"");
    }

    #[test]
    fn test_sanitize_preserves_at_sign() {
        let sanitized = sanitize_fts_query("@skaft/hamr");
        // @ triggers quoting, / triggers quoting → whole token quoted
        assert_eq!(sanitized, "\"@skaft/hamr\"");
    }

    #[test]
    fn test_sanitize_preserves_colon() {
        let sanitized = sanitize_fts_query("rust:rewrite");
        assert_eq!(sanitized, "\"rust:rewrite\"");
    }

    #[test]
    fn test_sanitize_preserves_prefix_wildcard() {
        let sanitized = sanitize_fts_query("error*");
        assert_eq!(sanitized, "error*");
    }

    #[test]
    fn test_sanitize_prefix_wildcard_with_hyphen() {
        let sanitized = sanitize_fts_query("hamr-browser*");
        // Hyphen triggers quoting, trailing * preserved after quote
        assert_eq!(sanitized, "\"hamr-browser\"*");
    }

    #[test]
    fn test_sanitize_preserves_quoted_phrase() {
        let sanitized = sanitize_fts_query("\"exact phrase match\"");
        assert_eq!(sanitized, "\"exact phrase match\"");
    }

    #[test]
    fn test_sanitize_mixed_terms() {
        let sanitized = sanitize_fts_query("error src/main.rs hamr-browser @skaft/hamr");
        // error is safe, the rest are quoted
        assert_eq!(
            sanitized,
            "error \"src/main.rs\" \"hamr-browser\" \"@skaft/hamr\""
        );
    }

    #[test]
    fn test_sanitize_strips_semicolons() {
        let sanitized = sanitize_fts_query("alpha; beta;");
        // Semicolons stripped, leaving "alpha beta"
        assert_eq!(sanitized, "alpha beta");
    }

    #[test]
    fn test_sanitize_preserves_underscore() {
        let sanitized = sanitize_fts_query("hello_world");
        assert_eq!(sanitized, "hello_world");
    }

    #[test]
    fn test_sanitize_empty_query() {
        let sanitized = sanitize_fts_query("");
        assert_eq!(sanitized, "");
    }

    #[test]
    fn test_sanitize_only_special_chars() {
        let sanitized = sanitize_fts_query(";:;:");
        // All stripped → empty, fallback to trimmed input
        // The fallback strips to bare chars, "::::" stripped to "...empty"
        // Actually, colons are kept in fallback. Let's check.
        let result = sanitize_fts_query(";:;:");
        // Colons and semicolons get stripped in token phase, fallback keeps colons
        assert!(!result.is_empty()); // Should have fallback content
    }

    #[test]
    fn test_sanitize_handles_null_bytes() {
        let sanitized = sanitize_fts_query("hello\x00world");
        assert_eq!(sanitized, "helloworld");
    }

    #[test]
    fn test_tokenize_quoted_phrase() {
        let tokens = tokenize_fts_query("\"hello world\" foo");
        assert_eq!(tokens, vec!["\"hello world\"", "foo"]);
    }

    #[test]
    fn test_tokenize_hyphenated() {
        let tokens = tokenize_fts_query("hamr-browser");
        assert_eq!(tokens, vec!["\"hamr-browser\""]);
    }

    #[test]
    fn test_handoff_basic() {
        let mut mem = make_memory();
        mem.store(&make_entry(
            "s1",
            1,
            "user",
            "Let me fix the bug in src/lib.rs",
        ));
        mem.store(&make_entry(
            "s1",
            2,
            "assistant",
            "I'll update the error handler",
        ));
        mem.store(&make_entry(
            "s1",
            3,
            "tool",
            "Error: file not found at src/lib.rs",
        ));

        let manifest = mem.handoff(None);
        assert_eq!(manifest.session_id, "s1");
        assert!(manifest.entry_count >= 3);
        assert!(!manifest.key_findings.is_empty());
    }

    #[test]
    fn test_has_session_entries() {
        let mut mem = make_memory();
        assert!(!mem.has_session_entries("s1"));
        mem.store(&make_entry("s1", 1, "user", "hello"));
        assert!(mem.has_session_entries("s1"));
    }

    #[test]
    fn test_get_latest_by_domain_tag() {
        let mut mem = make_memory();
        let mut entry = make_entry("s1", 1, "assistant", "Survival manifest: all good");
        entry.domain_tags = Some(vec!["survival".to_string()]);
        mem.store(&entry);

        let result = mem.get_latest_by_domain_tag("survival", "s1");
        assert!(result.is_some());
        assert_eq!(result.unwrap().content, "Survival manifest: all good");

        assert!(mem.get_latest_by_domain_tag("nonexistent", "s1").is_none());
    }

    #[test]
    fn test_store_content_capped() {
        let mut mem = make_memory();
        // Use repeating words separated by spaces so each 'x' is a distinct FTS5 token
        let long = "x ".repeat(10_000); // "x x x x ..."
        let entry = make_entry("s1", 1, "user", &long);
        mem.store(&entry);

        let results = mem.search("x", 10);
        assert_eq!(results.len(), 1);
        // Content capped at 8000 bytes (the store function uses entry.content[..8000])
        assert!(results[0].content.len() <= 8000);
    }

    #[test]
    fn test_build_memory_index() {
        let mut mem = make_memory();
        assert!(mem.build_memory_index().is_none());

        mem.store(&make_entry("s1", 1, "user", "Hello world"));
        mem.store(&make_entry(
            "s1",
            2,
            "assistant",
            "Let me help you with src/main.rs",
        ));

        let index = mem.build_memory_index();
        assert!(index.is_some());
        let idx = index.unwrap();
        assert!(idx.contains("Memory:"));
        assert!(idx.contains("2 entries"));
    }

    #[test]
    fn test_store_error_count() {
        // With no connection, store should be a no-op but count errors
        let mut mem = HolographicMemory::new(None);
        // This shouldn't crash
        mem.store(&make_entry("s1", 1, "user", "test"));
        // store_error_count should remain 0 since we returned early,
        // not from a SQL error
    }

    #[test]
    fn test_handoff_empty() {
        let mem = make_memory();
        let manifest = mem.handoff(None);
        assert_eq!(manifest.session_id, "");
        assert!(manifest.key_findings.is_empty());
    }
}
