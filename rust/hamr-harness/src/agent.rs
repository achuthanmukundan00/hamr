//! Port of `../../packages/agent/src/agent.ts` (600+ lines).
//!
//! The `Agent` struct — a stateful wrapper around `run_agent_loop`.
//!
//! # Architecture
//!
//! The Agent:
//! - Owns the conversation transcript (`AgentState` with messages, tools, model, etc.)
//! - Provides `prompt(input)` and `continue_run()` entry points
//! - Manages steering and follow-up message queues (`PendingMessageQueue`)
//! - Manages two abort signals (lifecycle + tool) — see `agent_loop.rs` for details
//! - Subscribes listeners for `AgentEvent` notifications
//! - Ensures only one run is active at a time (`activeRun` tracking)
//! - Handles retry/error message generation on run failure
//!
//! # Key Struct Fields
//!
//! ```rust
//! pub struct Agent {
//!     // Owned state
//!     state: Mutex<AgentState>,
//!     
//!     // Event listeners — called for every AgentEvent
//!     listeners: Vec<Arc<dyn Fn(AgentEvent, AbortSignal) -> Pin<Box<dyn Future<Output = ()>>>>,
//!     
//!     // Message queues
//!     steering_queue: PendingMessageQueue,
//!     follow_up_queue: PendingMessageQueue,
//!     
//!     // Active run tracking
//!     active_run: Option<ActiveRun>,
//!     
//!     // Hooks (optional, set at construction)
//!     convert_to_llm: Arc<dyn Fn(Vec<AgentMessage>) -> Vec<Message>>,
//!     stream_fn: Arc<dyn Fn(...) -> EventStream<...>>,
//!     before_tool_call: Option<Arc<dyn Fn(...) -> ...>>,
//!     after_tool_call: Option<Arc<dyn Fn(...) -> ...>>,
//!     // ... etc
//!     
//!     // Config
//!     session_id: Option<String>,
//!     tool_execution: ToolExecutionMode,
//!     max_retry_delay_ms: Option<u64>,
//! }
//! ```
//!
//! # Porting Instructions
//!
//! 1. Read `../../packages/agent/src/agent.ts` completely.
//! 2. Translate the `Agent` class to a Rust struct with an async API.
//! 3. Use `tokio::sync::Mutex` for `state` (async-aware, unlike `std::sync::Mutex`).
//! 4. The `prompt()` method creates an `ActiveRun` with two `AbortController` equivalents,
//!    calls `runAgentLoop`, and resolves when it completes.
//! 5. Listeners are called in order; their futures are awaited.
//! 6. Steering/follow-up queues use `QueueMode` (All / OneAtATime) for drain strategy.
//! 7. `waitForIdle()` returns a future that resolves when `activeRun` settles.
//!
//! # Key Difference from TS
//!
//! In TS, the Agent class has mutable public properties (streamFn, beforeToolCall, etc.)
//! that can be reassigned at runtime. In Rust, make these `Arc<dyn Fn>` so they can be
//! swapped atomically if needed, or just make them immutable after construction.

