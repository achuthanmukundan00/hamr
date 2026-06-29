//! hamr-harness: Core agent loop, compaction, session management, and extension infrastructure.
//!
//! Architectural mirror of [`@hamr/agent`] (`packages/agent/`).
//!
//! # Crate structure
//!
//! | Module | TS source | Purpose |
//! |--------|-----------|---------|
//! | [`types`] | `types.ts` | Agent types: AgentMessage, AgentEvent, AgentTool, AgentContext, etc. |
//! | [`agent`] | `agent.ts` | Agent class — stateful wrapper owning transcript, tools, streaming |
//! | [`agent_loop`] | `agent-loop.ts` | `runAgentLoop()` / `runAgentLoopContinue()` — pure functions, event streams |
//! | [`harness`] | `harness/` | Higher-level agent harness with compaction, sessions, skills, prompts |

// ---------------------------------------------------------------------------
// Core agent types
// ---------------------------------------------------------------------------
pub mod types;
pub use types::*;

// ---------------------------------------------------------------------------
// Agent runtime
// ---------------------------------------------------------------------------
pub mod agent;
pub use agent::{Agent, AgentOptions, AgentStateSnapshot};
pub mod agent_loop;

// ---------------------------------------------------------------------------
// Agent harness (higher-level abstractions)
// ---------------------------------------------------------------------------
pub mod harness;

// ---------------------------------------------------------------------------
// Test support
// ---------------------------------------------------------------------------
/// Serializes tests that register a `faux` provider into the PROCESS-GLOBAL
/// provider registry and then dispatch through it (`stream_simple` /
/// `complete_simple`). Without this, two parallel tests can register under the
/// same `Api` key and clobber each other's scripted responses. Tests that
/// bypass the registry (calling `reg.stream_simple` directly) don't need it.
///
/// Hold the guard for the whole test body:
/// `let _g = crate::faux_registry_guard();`
#[cfg(test)]
pub(crate) static FAUX_REGISTRY_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[cfg(test)]
pub(crate) fn faux_registry_guard() -> std::sync::MutexGuard<'static, ()> {
    FAUX_REGISTRY_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}
