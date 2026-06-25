//! Hamr-specific built-in extensions and features.
//!
//! Mirror of `packages/coding-agent/src/hamr/`.
//!
//! These are the features that distinguish hamr from the base pi agent:
//! - FTS5 memory with fact store and holographic retrieval
//! - Subagent orchestration with context inheritance
//! - Tool-call parsing/repair for non-native function-calling models
//! - Context cards, read-loop guard, persistent editor

pub mod extensions;
pub mod handoff;
pub mod helpers;
pub mod memory;
pub mod persistent_editor;
pub mod providers;
pub mod repair;
pub mod shimmer;
pub mod startup_config;
pub mod store;
