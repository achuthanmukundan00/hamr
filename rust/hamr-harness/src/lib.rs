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
pub mod agent_loop;

// ---------------------------------------------------------------------------
// Agent harness (higher-level abstractions)
// ---------------------------------------------------------------------------
pub mod harness;
