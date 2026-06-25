//! Port of `packages/coding-agent/src/core/extensions/types.ts` (1650 lines).
//!
//! This is the **most architecturally important file** in the entire codebase.
//! It defines the extension system contract: 30+ event types, the `ExtensionAPI`,
//! `ExtensionContext`, `ExtensionUIContext`, `ToolDefinition`, command registration,
//! provider registration, message renderers, and all event result types.
//!
//! # Porting Instructions
//!
//! 1. Read `../../packages/coding-agent/src/core/extensions/types.ts` in full.
//! 2. Translate every TypeScript interface/type to a Rust struct/enum.
//! 3. Translate every discriminated union (`type FooEvent = A | B | C`) to a
//!    `#[serde(tag = "type")]` enum.
//! 4. Translate every callback type (`(x: T) => Promise<R>`) to:
//!    `Arc<dyn Fn(T) -> Pin<Box<dyn Future<Output = R> + Send>> + Send + Sync>`
//! 5. Define the `Extension` trait with default empty implementations for every
//!    event handler (30+ methods). This mirrors the TS `ExtensionAPI.on()` pattern.
//! 6. Define `ToolDefinition<TParams, TDetails, TState>` as a struct with an
//!    `execute` field of type `Arc<dyn Fn(...) -> Pin<Box<dyn Future<...>>>>`.
//! 7. Use `serde(rename_all = "camelCase")` on all serializable types.
//! 8. For TypeBox schemas, use `schemars::JsonSchema` derive + `#[schemars(...)]` attrs.
//!
//! # Key Types to Define
//!
//! - `ExtensionEvent` — the 30+ event discriminated union
//! - `ExtensionAPI` — trait with on(), registerTool(), registerCommand(), etc.
//! - `ExtensionContext` — struct with UI, session, model, abort, shutdown access
//! - `ExtensionUIContext` — trait for dialogs, widgets, status, editor control
//! - `ExtensionCommandContext` — extends ExtensionContext with session control
//! - `ToolDefinition<TParams, TDetails, TState>` — tool registration contract
//! - `ToolCallEvent` — discriminated union of bash/read/edit/write/grep/find/ls/custom
//! - `ToolResultEvent` — discriminated union with details per tool
//! - All event result types (ContextEventResult, ToolCallEventResult, etc.)
//! - `RegisteredCommand`, `RegisteredTool`, `ExtensionFlag`, `ExtensionShortcut`
//! - `ProviderConfig`, `ProviderModelConfig`
//! - `CompactionPreparation`, `CompactionResult`, `TreePreparation`
//! - `MessageRenderer`, `RoleMessageRenderer`, `RoleMessageRenderContext`
//!
//! # Dependencies
//!
//! - `hamr-ai` types: AgentMessage, ImageContent, Model, ToolResultMessage, etc.
//! - `hamr-harness` types: AgentTool, AgentEvent, ThinkingLevel, ToolExecutionMode
//! - `schemars::JsonSchema` for TypeBox equivalent
//! - `serde` for all serialization
//! - `std::collections::HashMap` for maps
//! - `tokio::sync` for signals
//!
//! # Rust Patterns
//!
//! For the `Extension` trait (mirrors TS `ExtensionAPI`):
//! ```rust
//! pub trait Extension: Send + Sync {
//!     fn name(&self) -> &'static str;
//!     
//!     // Event handlers — default empty impls
//!     fn on_session_start(&self, ctx: &mut ExtensionContext, event: SessionStartEvent) -> Result<()> { Ok(()) }
//!     fn on_tool_call(&self, ctx: &mut ExtensionContext, event: ToolCallEvent) -> Result<ToolCallEventResult> { Ok(ToolCallEventResult::default()) }
//!     // ... 28 more
//!
//!     // Registration
//!     fn register_tools(&self, registry: &mut ToolRegistry) {}
//!     fn register_commands(&self, registry: &mut CommandRegistry) {}
//!     fn register_providers(&self, registry: &mut ProviderRegistry) {}
//! }
//! ```
//!
//! For the discriminated union events:
//! ```rust
//! #[derive(Serialize, Deserialize)]
//! #[serde(tag = "type")]
//! pub enum ExtensionEvent {
//!     #[serde(rename = "session_start")]
//!     SessionStart(SessionStartEvent),
//!     #[serde(rename = "tool_call")]
//!     ToolCall(ToolCallEvent),
//!     // ... etc
//! }
//! ```

