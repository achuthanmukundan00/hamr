//! Port of `../../packages/coding-agent/src/hamr/extensions/memory.ts`.
//!
//! Implements the hamr memory extension: FTS5 search, fact store, holographic memory.
//! Registered as a built-in Extension behind `#[cfg(feature = "hamr-memory")]`.
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
//! - `fact_store` — CRUD operations on the structured fact database
//!
//! # Dependencies
//!
//! - `rusqlite` with `bundled` + `vtab` features (FTS5)
//! - `hamr-agent/src/hamr/memory/fact_store.rs` — structured fact storage
//! - `hamr-agent/src/hamr/memory/fts_marks.rs` — FTS5 mark tracking
//! - `hamr-agent/src/hamr/memory/holographic_memory.rs` — holographic retrieval
//!
//! # Porting Order
//!
//! 1. Port the memory submodules first (fact_store, fts_marks, holographic_memory)
//! 2. Then port this file — it composes them into an Extension impl

