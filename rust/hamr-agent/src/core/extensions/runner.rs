//! Port of `packages/coding-agent/src/core/extensions/runner.ts` (900+ lines).
//!
//! The `ExtensionRunner` manages the lifecycle of all loaded extensions:
//! - Holds a list of `Extension` trait objects
//! - Dispatches events to extensions in registration order
//! - Collects and chains results (context transforms, tool call blocks, etc.)
//! - Handles error isolation — one extension crashing doesn't kill others
//! - Manages the shared `ExtensionRuntime` (actions available to extensions)
//! - Creates `ExtensionContext` and `ExtensionCommandContext` instances
//! - Handles `before_*` event cancellation (session_before_switch, etc.)
//! - Emits `ExtensionError` events to registered error listeners
//! - Manages stale context invalidation after session replacement
//!
//! # Porting Instructions
//!
//! 1. Read `../../packages/coding-agent/src/core/extensions/runner.ts` in full.
//! 2. Translate the `ExtensionRunner` class to a Rust struct.
//! 3. Each `emit*` method (emitMessageEnd, emitToolResult, emitToolCall, etc.)
//!    becomes an `async fn` on the struct that iterates extensions and dispatches.
//! 4. The `bindCore` method connects the runtime to concrete implementations
//!    (sendMessage, appendEntry, setModel, etc.) — these become callback fields.
//! 5. The `createContext()` method returns an `ExtensionContext` with lazy accessors
//!    that assert the context isn't stale.
//! 6. Error handling: every handler call is wrapped in a try-catch equivalent.
//!    In Rust, use per-handler match on Result and collect errors.
//!
//! # Key Patterns
//!
//! - Event dispatch: iterate extensions, call handler, collect/chain results
//! - Cancellation: `before_*` events short-circuit on first handler that returns `cancel: true`
//! - Transformation: `context` event chains messages through all handlers
//! - Provider registration: queued during load, flushed when `bindCore` is called
//! - Stale context: `invalidate()` sets a flag; `assertActive()` checks it before every access
//!
//! # Dependencies
//!
//! - `super::types` for all extension types
//! - `hamr-ai` for provider types
//! - `hamr-harness` for agent types
//! - `tokio::sync` for async coordination
//! - `std::sync::Arc` for shared ownership

