//! Port of `../../packages/agent/src/agent-loop.ts` (750+ lines).
//!
//! The core agent loop — pure async functions that drive the conversation.
//!
//! # Architecture
//!
//! `runAgentLoop(prompts, context, config)`:
//! 1. Append prompt messages to context
//! 2. Call `convertToLlm` to transform AgentMessage[] → LLM Message[]
//! 3. Call `transformContext` (optional pre-flight transform, e.g. compaction)
//! 4. Call `streamSimple(model, context, options)` to get an event stream
//! 5. Process each `AssistantMessageEvent` from the stream:
//!    - `start` / `text_*` / `thinking_*` → emit `message_start`/`message_update`
//!    - `toolcall_*` → accumulate tool calls
//!    - `done` → emit `message_end`, `turn_end`
//!    - `error` → emit `message_end`, `turn_end` with error
//! 6. If tool calls exist, execute them (sequential or parallel):
//!    - `beforeToolCall` hook (can block)
//!    - Execute tool
//!    - `afterToolCall` hook (can modify result)
//!    - Emit `tool_execution_start/update/end`
//!    - Collect tool results as ToolResultMessages
//! 7. Append tool results to context, loop back to step 3
//! 8. When model stops without tool calls:
//!    - `shouldStopAfterTurn` check
//!    - Poll `getSteeringMessages()` for queued steering
//!    - Poll `getFollowUpMessages()` for queued follow-ups
//!    - `prepareNextTurn()` for state overrides
//! 9. Exit with `agent_end`
//!
//! `runAgentLoopContinue(context, config)`:
//! Same as above but no initial prompt — continues from existing context.
//!
//! # Abort Signals
//!
//! Two separate abort signals:
//! - **lifecycle signal**: aborted on compaction, auto-retry, user escape, dispose
//! - **tool signal**: aborted ONLY on user escape and dispose (NOT on compaction/retry)
//!   This lets long-running subagents survive lifecycle management.
//!
//! # Key Rust Patterns
//!
//! - Use `tokio::select!` to race LLM streaming against abort signals
//! - Tool execution: spawn tokio tasks for parallel mode, await sequentially for sequential
//! - Event emission: send into `mpsc::UnboundedSender<AgentEvent>`
//! - The loop returns `Vec<AgentMessage>` (new messages added during this run)
//!
//! # Porting Instructions
//!
//! 1. Read `../../packages/agent/src/agent-loop.ts` completely.
//! 2. Translate the TypeScript async generator pattern to Rust async fn + mpsc channels.
//! 3. The TS `for await (const event of stream)` becomes `while let Some(event) = stream.next().await`.
//! 4. Tool execution dispatch: match `config.tool_execution` to spawn sequential or parallel.
//! 5. Hook calls (beforeToolCall, afterToolCall) are `Option<Arc<dyn Fn(...)>>` — check and call.
//! 6. Carefully handle both abort signals — check the tool signal before/after tool execution,
//!    check the lifecycle signal during LLM streaming.

