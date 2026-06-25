//! Port of `../../packages/coding-agent/src/hamr/extensions/subagents.ts`.
//!
//! Implements subagent orchestration: spawning child agent sessions that run
//! in parallel, with context inheritance and result aggregation.
//! Registered as a built-in Extension behind `#[cfg(feature = "hamr-subagents")]`.
//!
//! # Architecture
//!
//! The subagents extension registers:
//! - `delegate_subagents` tool — spawns N parallel/sequential subagents
//! - `create_handoff` tool — checkpoints state for context inheritance
//! - Event handlers that manage subagent lifecycle (spawn, monitor, aggregate)
//!
//! Each subagent runs as a separate tokio task with its own `Agent` instance,
//! session, and tool set. Results are streamed back via channels.
//!
//! # Key Types from TS (port these)
//!
//! - `SubagentConfig` — max depth, timeout, concurrency, model override
//! - `SubagentResult` — aggregated output from a subagent run
//! - `SubagentManifest` — handoff context for context inheritance
//!
//! # Dependencies
//!
//! - `hamr-harness` for `Agent`, `AgentLoopConfig`, `AgentContext`
//! - `hamr-agent/src/hamr/handoff/` for handoff manifest types
//! - `tokio::task` for spawning subagent tasks
//! - `tokio::sync::mpsc` for result streaming

