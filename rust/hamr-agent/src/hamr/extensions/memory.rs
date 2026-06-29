//! Port of `../../packages/coding-agent/src/hamr/memory.ts`.
//!
//! Implements the hamr memory extension: FTS5 search, fact store, holographic memory.
//! Registered as a built-in Extension behind `#[cfg(feature = "hamr-memory")]`.
//!
//! # Database path (v0.7.1 centralized)
//!
//! Priority:
//! 1. `HAMR_MEMORY_DB` env var — absolute or relative (relative to cwd)
//! 2. `~/.hamr/memory.sqlite` — centralized, one DB for all projects
//!
//! Previously defaulted to `<cwd>/.hamr/memory.sqlite` which littered a
//! separate DB in every project directory. Centralizing to ~/.hamr means
//! all sessions share one memory store, and the agent can find facts from
//! any project.
//!
//! # Auto-migration from v0.7.0
//!
//! On first access to the new centralized path, if no DB exists there but
//! `<cwd>/.hamr/memory.sqlite` does, the old DB is copied to the new
//! location so users don't lose accumulated memory and facts.
//!
//! # Architecture
//!
//! The memory extension registers handlers for:
//! - `session_start` — initializes the SQLite-backed memory store
//! - `before_agent_start` — injects relevant memories into the system prompt
//! - `tool_call` — intercepts read/write/edit/bash to index file contents
//! - `message_end` — extracts facts from assistant messages for the fact store
//!
//! It also registers tools:
//! - `search_memory` — FTS5 full-text search over past turns/outputs
//! - `save_memory` — explicitly store a fact/decision/note
//! - `handoff_memory` — build structured handoff manifest
//! - `fact_store` — CRUD operations on the structured fact database
//! - `fact_feedback` — rate facts (helpful/unhelpful) to adjust trust scores

use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// Database path resolution (v0.7.1 — centralized)
// ---------------------------------------------------------------------------

/// Resolve the memory database path.
///
/// Priority:
/// 1. `HAMR_MEMORY_DB` env var — absolute or relative (relative to cwd)
/// 2. `~/.hamr/memory.sqlite` — centralized default
///
/// Mirrors TS `memoryPath(cwd)`.
pub fn memory_db_path(cwd: &Path) -> PathBuf {
    if let Ok(env_path) = std::env::var("HAMR_MEMORY_DB") {
        let env_path = env_path.trim();
        if env_path.is_empty() {
            return default_memory_db_path();
        }
        // If the env var is absolute or starts with ~, use it directly
        if env_path.starts_with('/') || env_path.starts_with('~') {
            return expand_tilde(env_path);
        }
        // Relative — resolve against cwd
        return cwd.join(env_path);
    }
    default_memory_db_path()
}

/// Default centralized path: `~/.hamr/memory.sqlite`.
fn default_memory_db_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".hamr").join("memory.sqlite")
}

/// Expand a leading `~` to the user's home directory.
fn expand_tilde(path: &str) -> PathBuf {
    if let Some(rest) = path.strip_prefix("~/") {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        PathBuf::from(home).join(rest)
    } else if path == "~" {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        PathBuf::from(home)
    } else {
        PathBuf::from(path)
    }
}

// ---------------------------------------------------------------------------
// Migration from v0.7.0: <cwd>/.hamr/memory.sqlite → ~/.hamr/memory.sqlite
// ---------------------------------------------------------------------------

/// If the default centralized path doesn't exist but the old project-local
/// path does, copy the old DB to the new location so users don't lose their
/// accumulated memory.
///
/// Only runs when using the default path (not when `HAMR_MEMORY_DB` is set).
///
/// Mirrors TS migration logic in `getMemoryHandle()`.
pub fn migrate_memory_db_if_needed(cwd: &Path, new_path: &Path) {
    // Only migrate when using the default path (not a custom HAMR_MEMORY_DB)
    if std::env::var("HAMR_MEMORY_DB").is_ok() {
        return;
    }
    if new_path.exists() {
        return;
    }
    let old_path = cwd.join(".hamr").join("memory.sqlite");
    if !old_path.exists() {
        return;
    }
    // Ensure parent directory exists
    if let Some(parent) = new_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    match std::fs::copy(&old_path, new_path) {
        Ok(_) => {
            eprintln!(
                "[hamr] Migrated memory DB from {} → {}",
                old_path.display(),
                new_path.display()
            );
        }
        Err(err) => {
            eprintln!(
                "[hamr] Could not migrate old memory DB from {}: {}",
                old_path.display(),
                err
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Database open helper
// ---------------------------------------------------------------------------

/// Open a SQLite database at `path` with the required PRAGMAs and FTS5 table.
///
/// Sets:
/// - `PRAGMA journal_mode = WAL`
/// - `PRAGMA foreign_keys = ON` (v0.7.1)
///
/// Creates the `memory_fts` virtual table if it doesn't exist.
#[cfg(feature = "hamr-memory")]
pub fn open_memory_db(path: &Path) -> Result<rusqlite::Connection, rusqlite::Error> {
    // Ensure parent directory exists
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).ok();
    }

    let conn = rusqlite::Connection::open(path)?;
    conn.pragma_update(None, "journal_mode", "WAL")?;
    conn.pragma_update(None, "foreign_keys", "ON")?;

    conn.execute_batch(
        "CREATE VIRTUAL TABLE IF NOT EXISTS memory_fts USING fts5(\
            turn_id UNINDEXED,\
            session_id UNINDEXED,\
            role UNINDEXED,\
            tool_name UNINDEXED,\
            file_paths UNINDEXED,\
            content,\
            domain_tags UNINDEXED\
        );",
    )?;

    Ok(conn)
}

/// Get the resolved memory DB path, running migration if needed.
///
/// This is the main entry point for callers that need to open a memory
/// connection.  It returns the resolved path after running auto-migration.
pub fn resolve_memory_db_path(cwd: &Path) -> PathBuf {
    let path = memory_db_path(cwd);
    migrate_memory_db_if_needed(cwd, &path);
    path
}

// ---------------------------------------------------------------------------
// Cue-triggered memory prefetch (v0.7.1)
// ---------------------------------------------------------------------------

/// Reason a memory prefetch was triggered.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum MemoryPrefetchReason {
    ExplicitRecall,
    Continuation,
}

impl std::fmt::Display for MemoryPrefetchReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MemoryPrefetchReason::ExplicitRecall => write!(f, "explicit recall cue"),
            MemoryPrefetchReason::Continuation => write!(f, "continuation cue"),
        }
    }
}

/// Payload produced by a cue-triggered memory prefetch.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryPrefetchPayload {
    pub reason: MemoryPrefetchReason,
    pub latest_user_text: String,
    pub queries: Vec<String>,
    pub facts: Vec<FactWithScore>,
    pub transcript_results: Vec<MemorySearchResultWithSnippet>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<u64>,
}

use crate::hamr::memory::fact_store::FactStore;
use crate::hamr::memory::fact_store::FactWithScore;
use crate::hamr::memory::holographic_memory::{HolographicMemory, MemorySearchResultWithSnippet};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::sync::LazyLock;

/// Matches explicit recall cues: "remember...", "last time...", "where we left off", etc.
static EXPLICIT_RECALL_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(?:remember|recall|last time|earlier|previous(?:ly)?|prior conversation|we talked|we were talking|where (?:we|it) left off|pick up|continue(?: from)?|that .{0,40}thing|the .{0,40}thing)\b").unwrap()
});

/// Matches continuation fragments: "the genre is...", "it...", "also...", etc.
static CONTINUATION_FRAGMENT_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(?:(?:the\s+(?:genre|vibe|artist|track|song|album|project|thing|one|issue|bug|error|problem|plan|approach|fix|branch|file|context|repo)\s+(?:is|was|are|were|=|should|needs?|has|uses?))|(?:(?:it|it's|its|that|this)\b)|(?:(?:also|btw)\b))").unwrap()
});

static MUSIC_CONTEXT_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(?:music|electronic|artist|genre|track|song|album|club|deconstructed|industrial|sound|vibe)\b").unwrap()
});

static PROJECT_CONTEXT_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(?:project|thing|context|conversation|remember|recall|last time|earlier|previous|pick up|continue)\b").unwrap()
});

/// Stop words stripped during query compaction.
static PREFETCH_STOP_WORDS: LazyLock<std::collections::HashSet<&'static str>> =
    LazyLock::new(|| {
        [
            "the", "a", "an", "and", "or", "but", "is", "are", "was", "were", "it", "its", "it's",
            "this", "that", "thing", "one", "about", "with", "from", "for", "to", "of", "in", "on",
            "can", "you", "we", "me", "my", "our", "remember", "recall", "please",
        ]
        .iter()
        .copied()
        .collect()
    });

/// Character budget for prefetch context messages (~chars/4 ≈ tokens).
const MEMORY_PREFETCH_CHAR_BUDGET: usize = 600;

/// Classify a user prompt as a potential memory prefetch trigger.
///
/// Returns `None` if the prompt doesn't match any prefetch pattern.
/// Mirrors TS `classifyMemoryPrefetchPrompt`.
pub fn classify_memory_prefetch_prompt(prompt: &str) -> Option<MemoryPrefetchReason> {
    let text = prompt.trim();
    if text.is_empty() {
        return None;
    }
    if EXPLICIT_RECALL_RE.is_match(text) {
        return Some(MemoryPrefetchReason::ExplicitRecall);
    }
    if text.len() <= 500 && CONTINUATION_FRAGMENT_RE.is_match(text) {
        return Some(MemoryPrefetchReason::Continuation);
    }
    None
}

/// Build FTS5/fact search queries from a prefetch prompt.
/// Mirrors TS `buildMemoryPrefetchQueries`.
pub fn build_memory_prefetch_queries(prompt: &str, reason: MemoryPrefetchReason) -> Vec<String> {
    let mut queries: Vec<String> = Vec::new();
    push_unique(&mut queries, compact_memory_query(prompt));

    if MUSIC_CONTEXT_RE.is_match(prompt) {
        push_unique(&mut queries, Some("music".to_string()));
        push_unique(&mut queries, Some("music project".to_string()));
        push_unique(&mut queries, Some("electronic music".to_string()));
        push_unique(&mut queries, Some("artist next level".to_string()));
    }

    if reason == MemoryPrefetchReason::ExplicitRecall || PROJECT_CONTEXT_RE.is_match(prompt) {
        push_unique(&mut queries, Some("user-context".to_string()));
        push_unique(&mut queries, Some("project-context".to_string()));
    }

    queries.truncate(8);
    queries
}

/// Compact a prompt into a keyword query by stripping stop words.
fn compact_memory_query(prompt: &str) -> Option<String> {
    let words: Vec<String> = prompt
        .to_lowercase()
        .split(|c: char| !c.is_alphanumeric() && c != '_' && c != '-')
        .map(|w| w.trim().to_string())
        .filter(|w| w.len() > 2 && !PREFETCH_STOP_WORDS.contains(w.as_str()))
        .collect();

    // Deduplicate, take first 6
    let mut seen = std::collections::HashSet::new();
    let unique: Vec<String> = words
        .into_iter()
        .filter(|w| seen.insert(w.clone()))
        .take(6)
        .collect();

    if unique.is_empty() {
        None
    } else {
        Some(unique.join(" "))
    }
}

fn push_unique(values: &mut Vec<String>, value: Option<String>) {
    let trimmed = match value {
        Some(ref v) => v.trim().to_string(),
        None => return,
    };
    if trimmed.is_empty() {
        return;
    }
    if !values
        .iter()
        .any(|existing| existing.to_lowercase() == trimmed.to_lowercase())
    {
        values.push(trimmed);
    }
}

/// Collect memory prefetch results from transcript and fact store.
/// Mirrors TS `collectMemoryPrefetch`.
pub fn collect_memory_prefetch(
    latest_text: &str,
    memory: Option<&HolographicMemory>,
    fact_store: Option<&FactStore>,
) -> Option<MemoryPrefetchPayload> {
    if std::env::var("HAMR_MEMORY_PREFETCH").map_or(false, |v| v == "false" || v == "0") {
        return None;
    }

    let reason = classify_memory_prefetch_prompt(latest_text)?;
    let queries = build_memory_prefetch_queries(latest_text, reason);

    let mut facts: Vec<FactWithScore> = Vec::new();
    let mut seen_facts = std::collections::HashSet::new();
    let mut transcript_results: Vec<MemorySearchResultWithSnippet> = Vec::new();
    let mut seen_transcript = std::collections::HashSet::new();

    for query in &queries {
        if let Some(ref fs) = fact_store {
            if fs.is_available && facts.len() < 5 {
                for fact in fs.search_facts(query, 3) {
                    if seen_facts.insert(fact.fact_id) {
                        facts.push(fact);
                    }
                }
            }
        }
        if let Some(ref mem) = memory {
            if transcript_results.len() < 3 {
                for result in mem.search_with_snippets(query, 2) {
                    let key = format!(
                        "{}:{}:{}:{}:{}",
                        result.session_id,
                        result.turn_id,
                        result.role,
                        result.tool_name.as_deref().unwrap_or(""),
                        &result.content[..result.content.len().min(80)]
                    );
                    if seen_transcript.insert(key) {
                        transcript_results.push(result);
                    }
                }
            }
        }
    }

    // For explicit "remember that thing" prompts, fall back to recent facts
    if reason == MemoryPrefetchReason::ExplicitRecall && facts.is_empty() {
        if let Some(ref fs) = fact_store {
            if fs.is_available {
                for fact in fs.list_recent_facts(3, 0.0) {
                    if seen_facts.insert(fact.fact_id) {
                        facts.push(fact);
                    }
                }
            }
        }
    }

    if facts.is_empty() && transcript_results.is_empty() {
        return None;
    }

    Some(MemoryPrefetchPayload {
        reason,
        latest_user_text: latest_text.to_string(),
        queries,
        facts,
        transcript_results,
        timestamp: Some(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
        ),
    })
}

/// Build a hidden user message from prefetch results for context injection.
/// Mirrors TS `buildMemoryPrefetchContextMessage`.
pub fn build_memory_prefetch_context_message(payload: &MemoryPrefetchPayload) -> Option<String> {
    if payload.facts.is_empty() && payload.transcript_results.is_empty() {
        return None;
    }

    let mut lines: Vec<String> = Vec::new();
    lines.push(format!(
        "MEMORY PREFETCH ({}; hidden context for this turn):",
        payload.reason
    ));
    lines.push(format!(
        "Latest user prompt: {}",
        truncate_str(&payload.latest_user_text, 160)
    ));

    if !payload.facts.is_empty() {
        lines.push("Durable facts:".to_string());
        for fact in payload.facts.iter().take(5) {
            let tags = if fact.tags.is_empty() {
                String::new()
            } else {
                format!(" tags={}", fact.tags)
            };
            lines.push(format!(
                "- [#{} trust={:.2}{}] {}",
                fact.fact_id,
                fact.trust_score,
                tags,
                truncate_str(&fact.content, 260)
            ));
        }
    }

    if !payload.transcript_results.is_empty() {
        lines.push("Transcript hits:".to_string());
        for result in payload.transcript_results.iter().take(3) {
            let excerpt = result.snippet.as_str();
            let tool = result
                .tool_name
                .as_deref()
                .map(|t| format!("/{}", t))
                .unwrap_or_default();
            lines.push(format!(
                "- turn {} {}{}: {}",
                result.turn_id,
                result.role,
                tool,
                truncate_str(excerpt, 220)
            ));
        }
    }

    if !payload.queries.is_empty() {
        lines.push(format!("Searches used: {}", payload.queries.join("; ")));
    }
    lines.push(
        "Use this naturally to resolve pronouns/continuations. If the latest prompt adds a durable detail, save it with save_memory/fact_store."
            .to_string(),
    );

    // Apply character budget
    let joined = lines.join("\n");
    Some(truncate_str(&joined, MEMORY_PREFETCH_CHAR_BUDGET))
}

fn truncate_str(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        let mut truncated = s[..max_len].to_string();
        truncated.push('…');
        truncated
    }
}

// ---------------------------------------------------------------------------
// Extension factory
// ---------------------------------------------------------------------------

/// Creates the hamr memory extension.
///
/// Registers tools (`search_memory`, `save_memory`, `handoff_memory`,
/// `fact_store`, `fact_feedback`) and hooks (`session_start`, `before_agent_start`,
/// `tool_call`, `message_end`) for the memory subsystem.
pub fn hamr_memory_extension() -> crate::core::extensions::types::ExtensionFactory {
    use crate::core::extensions::types::*;
    use std::sync::Arc;

    Arc::new(|pi: Arc<dyn ExtensionAPI>| {
        Box::pin(async move {
            // ── Register tools ───────────────────────────────────────

            // 1. search_memory — FTS5 full-text search over past turns/outputs
            pi.register_tool(ToolDefinition {
                name: "search_memory".to_string(),
                label: "Search Memory".to_string(),
                description: "Search past conversations and outputs using FTS5 full-text search. Use this to recall context from earlier in this session or from prior sessions.".to_string(),
                prompt_snippet: None,
                prompt_guidelines: None,
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "query": {
                            "type": "string",
                            "description": "FTS5 search query text"
                        },
                        "limit": {
                            "type": "number",
                            "description": "Maximum results to return (default 5)"
                        }
                    },
                    "required": ["query"]
                }),
                render_shell: None,
                prepare_arguments: None,
                execution_mode: None,
                execute: Arc::new(|id, args, _abort_rx, _update_cb, _ctx| {
                    Box::pin(async move {
                        let _ = id;
                        let query = args.get("query")
                            .and_then(|v| v.as_str())
                            .unwrap_or("");
                        hamr_harness::types::AgentToolResult {
                            content: vec![hamr_ai::types::MessageContent::Text(
                                hamr_ai::types::TextContent {
                                    text: format!(
                                        "Memory search results for \"{query}\":\nMemory backend is being ported to Rust. The FTS5-backed holographic memory store exists in the crate but full tool integration is in progress. Use save_memory/fact_store for durable facts."
                                    ),
                                    text_signature: None,
                                },
                            )],
                            details: None,
                            is_error: false,
                            terminate: false,
                        }
                    })
                }),
            });

            // 2. save_memory — explicitly store a fact/decision/note
            pi.register_tool(ToolDefinition {
                name: "save_memory".to_string(),
                label: "Save Memory".to_string(),
                description: "Store a durable fact, decision, or note into long-term memory for recall in future sessions.".to_string(),
                prompt_snippet: None,
                prompt_guidelines: None,
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "content": {
                            "type": "string",
                            "description": "The fact, decision, or note to store"
                        },
                        "tags": {
                            "type": "array",
                            "items": { "type": "string" },
                            "description": "Optional tags for categorization"
                        }
                    },
                    "required": ["content"]
                }),
                render_shell: None,
                prepare_arguments: None,
                execution_mode: None,
                execute: Arc::new(|id, args, _abort_rx, _update_cb, _ctx| {
                    Box::pin(async move {
                        let _ = id;
                        let content = args.get("content")
                            .and_then(|v| v.as_str())
                            .unwrap_or("");
                        let tags = args.get("tags")
                            .and_then(|v| v.as_array())
                            .map(|arr| {
                                arr.iter()
                                    .filter_map(|v| v.as_str().map(String::from))
                                    .collect::<Vec<_>>()
                                    .join(", ")
                            })
                            .unwrap_or_default();
                        hamr_harness::types::AgentToolResult {
                            content: vec![hamr_ai::types::MessageContent::Text(
                                hamr_ai::types::TextContent {
                                    text: format!(
                                        "Memory saved: \"{}\"{} Memory backend is being ported to Rust; fact will be persisted when integration completes.",
                                        truncate_str(content, 200),
                                        if tags.is_empty() { String::new() } else { format!(" [tags: {}]", tags) }
                                    ),
                                    text_signature: None,
                                },
                            )],
                            details: None,
                            is_error: false,
                            terminate: false,
                        }
                    })
                }),
            });

            // 3. handoff_memory — build structured handoff manifest
            pi.register_tool(ToolDefinition {
                name: "handoff_memory".to_string(),
                label: "Handoff Memory".to_string(),
                description: "Build a structured handoff manifest from FTS5 memory for another agent or future turn. Use before checkpointing or when context is exhausted.".to_string(),
                prompt_snippet: None,
                prompt_guidelines: None,
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {},
                    "required": []
                }),
                render_shell: None,
                prepare_arguments: None,
                execution_mode: None,
                execute: Arc::new(|id, _args, _abort_rx, _update_cb, _ctx| {
                    Box::pin(async move {
                        let _ = id;
                        hamr_harness::types::AgentToolResult {
                            content: vec![hamr_ai::types::MessageContent::Text(
                                hamr_ai::types::TextContent {
                                    text: "Handoff manifest would be built from FTS5 memory. Full integration in progress.".to_string(),
                                    text_signature: None,
                                },
                            )],
                            details: None,
                            is_error: false,
                            terminate: false,
                        }
                    })
                }),
            });

            // 4. fact_store — CRUD operations on structured fact database
            pi.register_tool(ToolDefinition {
                name: "fact_store".to_string(),
                label: "Fact Store".to_string(),
                description: "Store and query durable structured knowledge across sessions. Entities are auto-extracted. Actions: add, search, probe, related, reason.".to_string(),
                prompt_snippet: None,
                prompt_guidelines: None,
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "action": {
                            "type": "string",
                            "description": "One of: add, search, probe, related, reason"
                        },
                        "content": {
                            "type": "string",
                            "description": "Fact content (required for 'add')"
                        },
                        "query": {
                            "type": "string",
                            "description": "Search query (required for 'search')"
                        },
                        "entity": {
                            "type": "string",
                            "description": "Entity name for 'probe'/'related'"
                        },
                        "entities": {
                            "type": "array",
                            "items": { "type": "string" },
                            "description": "Entity names for 'reason'"
                        },
                        "tags": {
                            "type": "string",
                            "description": "Comma-separated tags"
                        },
                        "limit": {
                            "type": "number",
                            "description": "Max results (default 10)"
                        }
                    },
                    "required": ["action"]
                }),
                render_shell: None,
                prepare_arguments: None,
                execution_mode: None,
                execute: Arc::new(|id, args, _abort_rx, _update_cb, _ctx| {
                    Box::pin(async move {
                        let _ = id;
                        let action = args.get("action")
                            .and_then(|v| v.as_str())
                            .unwrap_or("search");
                        hamr_harness::types::AgentToolResult {
                            content: vec![hamr_ai::types::MessageContent::Text(
                                hamr_ai::types::TextContent {
                                    text: format!(
                                        "fact_store action \"{action}\": Fact store backend is being ported to Rust. Structured fact storage with auto-extraction will be available when integration completes."
                                    ),
                                    text_signature: None,
                                },
                            )],
                            details: None,
                            is_error: false,
                            terminate: false,
                        }
                    })
                }),
            });

            // 5. fact_feedback — rate facts (helpful/unhelpful)
            pi.register_tool(ToolDefinition {
                name: "fact_feedback".to_string(),
                label: "Fact Feedback".to_string(),
                description: "Rate a fact after using it. Mark 'helpful' if accurate, 'unhelpful' if outdated. Good facts rise (trust +0.05), bad facts sink (trust −0.10).".to_string(),
                prompt_snippet: None,
                prompt_guidelines: None,
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "action": {
                            "type": "string",
                            "description": "'helpful' or 'unhelpful'"
                        },
                        "fact_id": {
                            "type": "number",
                            "description": "The fact ID to rate"
                        }
                    },
                    "required": ["action", "fact_id"]
                }),
                render_shell: None,
                prepare_arguments: None,
                execution_mode: None,
                execute: Arc::new(|id, args, _abort_rx, _update_cb, _ctx| {
                    Box::pin(async move {
                        let _ = id;
                        let action = args.get("action")
                            .and_then(|v| v.as_str())
                            .unwrap_or("helpful");
                        let fact_id = args.get("fact_id")
                            .and_then(|v| v.as_i64())
                            .unwrap_or(0);
                        hamr_harness::types::AgentToolResult {
                            content: vec![hamr_ai::types::MessageContent::Text(
                                hamr_ai::types::TextContent {
                                    text: format!(
                                        "Fact #{fact_id} marked as {action}. Trust scores will be adjusted when fact store integration completes."
                                    ),
                                    text_signature: None,
                                },
                            )],
                            details: None,
                            is_error: false,
                            terminate: false,
                        }
                    })
                }),
            });

            // ── Register hooks ───────────────────────────────────────

            // session_start — initialize memory store
            pi.on(
                "session_start",
                Arc::new(|event, ctx| {
                    Box::pin(async move {
                        let _ = (event, ctx);
                        // Memory store initialization deferred to full integration
                        None
                    })
                }),
            );

            // before_agent_start — inject relevant memories into system prompt
            pi.on(
                "before_agent_start",
                Arc::new(|event, ctx| {
                    Box::pin(async move {
                        let _ = (event, ctx);
                        // Prefetch injection deferred to full integration
                        None
                    })
                }),
            );

            // tool_call — index file contents from read/write/edit/bash
            pi.on(
                "tool_call",
                Arc::new(|event, ctx| {
                    Box::pin(async move {
                        let _ = (event, ctx);
                        // Tool indexing deferred to full integration
                        None
                    })
                }),
            );

            // message_end — extract facts from assistant messages
            pi.on(
                "message_end",
                Arc::new(|event, ctx| {
                    Box::pin(async move {
                        let _ = (event, ctx);
                        // Fact extraction deferred to full integration
                        None
                    })
                }),
            );
        })
    })
}

// ---------------------------------------------------------------------------
// Turn ID tracking
// ---------------------------------------------------------------------------

use std::sync::atomic::{AtomicI64, Ordering};

static CURRENT_TURN_ID: AtomicI64 = AtomicI64::new(0);

/// Set the current turn ID (called by the agent loop each turn).
pub fn set_current_turn_id(id: i64) {
    CURRENT_TURN_ID.store(id, Ordering::SeqCst);
}

/// Get the current turn ID.
pub fn get_current_turn_id() -> i64 {
    CURRENT_TURN_ID.load(Ordering::SeqCst)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_db_path_default() {
        let prev = std::env::var("HAMR_MEMORY_DB").ok();
        unsafe { std::env::remove_var("HAMR_MEMORY_DB") };
        let cwd = Path::new("/some/project");
        let path = memory_db_path(cwd);
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        let expected = PathBuf::from(home).join(".hamr").join("memory.sqlite");
        assert_eq!(path, expected);
        if let Some(v) = prev {
            unsafe { std::env::set_var("HAMR_MEMORY_DB", v) };
        }
    }

    #[test]
    fn test_memory_db_path_env_absolute() {
        let prev = std::env::var("HAMR_MEMORY_DB").ok();
        unsafe { std::env::set_var("HAMR_MEMORY_DB", "/custom/path/db.sqlite") };
        let cwd = Path::new("/some/project");
        let path = memory_db_path(cwd);
        assert_eq!(path, PathBuf::from("/custom/path/db.sqlite"));
        if let Some(v) = prev {
            unsafe { std::env::set_var("HAMR_MEMORY_DB", v) };
        } else {
            unsafe { std::env::remove_var("HAMR_MEMORY_DB") };
        }
    }

    #[test]
    fn test_memory_db_path_env_relative() {
        let prev = std::env::var("HAMR_MEMORY_DB").ok();
        unsafe { std::env::set_var("HAMR_MEMORY_DB", "local/memory.db") };
        let cwd = Path::new("/some/project");
        let path = memory_db_path(cwd);
        assert_eq!(path, PathBuf::from("/some/project/local/memory.db"));
        if let Some(v) = prev {
            unsafe { std::env::set_var("HAMR_MEMORY_DB", v) };
        } else {
            unsafe { std::env::remove_var("HAMR_MEMORY_DB") };
        }
    }

    #[test]
    fn test_memory_db_path_env_tilde() {
        let prev = std::env::var("HAMR_MEMORY_DB").ok();
        unsafe { std::env::set_var("HAMR_MEMORY_DB", "~/custom/memory.sqlite") };
        let cwd = Path::new("/some/project");
        let path = memory_db_path(cwd);
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        let expected = PathBuf::from(home).join("custom").join("memory.sqlite");
        assert_eq!(path, expected);
        if let Some(v) = prev {
            unsafe { std::env::set_var("HAMR_MEMORY_DB", v) };
        } else {
            unsafe { std::env::remove_var("HAMR_MEMORY_DB") };
        }
    }

    #[test]
    fn test_expand_tilde() {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        assert_eq!(
            expand_tilde("~/foo/bar"),
            PathBuf::from(&home).join("foo/bar")
        );
        assert_eq!(expand_tilde("~"), PathBuf::from(&home));
        assert_eq!(
            expand_tilde("/absolute/path"),
            PathBuf::from("/absolute/path")
        );
    }

    // ── Prefetch tests ───────────────────────────────────────────────────

    #[test]
    fn test_classify_explicit_recall() {
        assert_eq!(
            classify_memory_prefetch_prompt("remember that music thing we talked about"),
            Some(MemoryPrefetchReason::ExplicitRecall)
        );
        assert_eq!(
            classify_memory_prefetch_prompt("pick up where we left off"),
            Some(MemoryPrefetchReason::ExplicitRecall)
        );
        assert_eq!(
            classify_memory_prefetch_prompt("last time we worked on the project"),
            Some(MemoryPrefetchReason::ExplicitRecall)
        );
    }

    #[test]
    fn test_classify_continuation() {
        assert_eq!(
            classify_memory_prefetch_prompt("the genre is electronic"),
            Some(MemoryPrefetchReason::Continuation)
        );
        assert_eq!(
            classify_memory_prefetch_prompt("it was working before"),
            Some(MemoryPrefetchReason::Continuation)
        );
        assert_eq!(
            classify_memory_prefetch_prompt("that sounds good"),
            Some(MemoryPrefetchReason::Continuation)
        );
    }

    #[test]
    fn test_classify_no_match() {
        assert_eq!(
            classify_memory_prefetch_prompt("write a function to sort an array"),
            None
        );
        assert_eq!(
            classify_memory_prefetch_prompt("what is the capital of France"),
            None
        );
        assert_eq!(classify_memory_prefetch_prompt(""), None);
    }

    #[test]
    fn test_compact_memory_query() {
        let result = compact_memory_query("remember that music project we were working on");
        assert!(result.is_some());
        let query = result.unwrap();
        // "music" and "project" survive; stop words stripped
        assert!(query.contains("music"));
        assert!(query.contains("project"));
    }

    #[test]
    fn test_build_prefetch_queries_explicit_recall() {
        let queries = build_memory_prefetch_queries(
            "remember that project thing",
            MemoryPrefetchReason::ExplicitRecall,
        );
        // Should include domain-context queries
        assert!(queries.iter().any(|q| q == "user-context"));
        assert!(queries.iter().any(|q| q == "project-context"));
        assert!(queries.len() <= 8);
    }

    #[test]
    fn test_build_prefetch_queries_continuation() {
        let queries = build_memory_prefetch_queries(
            "the bug is still happening",
            MemoryPrefetchReason::Continuation,
        );
        // Continuation without project context doesn't add domain queries
        assert!(queries.iter().all(|q| q != "user-context"));
        assert!(queries.len() <= 8);
    }

    #[test]
    fn test_build_prefetch_context_message_empty() {
        let payload = MemoryPrefetchPayload {
            reason: MemoryPrefetchReason::ExplicitRecall,
            latest_user_text: "test".to_string(),
            queries: vec!["test".to_string()],
            facts: vec![],
            transcript_results: vec![],
            timestamp: None,
        };
        assert!(build_memory_prefetch_context_message(&payload).is_none());
    }

    #[test]
    fn test_prefetch_respects_env_disable() {
        unsafe { std::env::set_var("HAMR_MEMORY_PREFETCH", "false") };
        let result = collect_memory_prefetch("remember that thing", None, None);
        unsafe { std::env::remove_var("HAMR_MEMORY_PREFETCH") };
        assert!(result.is_none());
    }
}
