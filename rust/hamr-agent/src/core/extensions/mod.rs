//! Extensions infrastructure — traits, runner, loader, types.
//!
//! Mirror of `packages/coding-agent/src/core/extensions/`.
//!
//! This is the core extension system that the agent loop dispatches events through.
//! Built-in extensions (memory, subagents, etc.) implement the [`Extension`] trait.
//! Agent-authored Rhai scripts are loaded by the runner at startup.

pub mod loader;
pub mod runner;
pub mod types;
pub mod wrapper;
