//! hamr-agent: Coding agent CLI with tools, extensions, TUI integration, and session management.
//!
//! Architectural mirror of [`@hamr/coding-agent`] (`packages/coding-agent/`).
//!
//! This crate composes the hamr harness with provider backends, connects the TUI,
//! and bundles built-in extensions (memory, subagents, context management, etc.).
//!
//! # Crate structure
//!
//! | Module | TS source | Purpose |
//! |--------|-----------|---------|
//! | [`cli`] | `cli/` | CLI entrypoint, args parsing, startup |
//! | [`core`] | `core/` | Agent session, tools, extensions, compaction, session manager |
//! | [`hamr`] | `hamr/` | Hamr-specific built-in extensions, memory, providers |
//! | [`modes`] | `modes/` | Run modes: interactive (TUI), print, RPC |
//! | [`utils`] | `utils/` | Shared utilities (clipboard, git, images, paths) |

// ---------------------------------------------------------------------------
// CLI entrypoint
// ---------------------------------------------------------------------------
pub mod cli;

// ---------------------------------------------------------------------------
// Core modules
// ---------------------------------------------------------------------------
pub mod core;

// ---------------------------------------------------------------------------
// Hamr-specific built-in extensions
// ---------------------------------------------------------------------------
pub mod hamr;

// ---------------------------------------------------------------------------
// Run modes
// ---------------------------------------------------------------------------
pub mod modes;

// ---------------------------------------------------------------------------
// Utilities
// ---------------------------------------------------------------------------
pub mod utils;
