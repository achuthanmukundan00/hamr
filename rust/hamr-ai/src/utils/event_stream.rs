//! Port of `../../packages/ai/src/utils/event-stream.ts`.
//!
//! An async stream that can be pushed to from producers and consumed by listeners.
//! This is the **fundamental communication primitive** between providers and the agent loop.
//!
//! # What to Port
//!
//! The TS implementation is a custom `EventStream<T, TResult>` class that:
//! 1. Accepts `push(event)` calls from producers (provider backends)
//! 2. Allows consumers to iterate via `for await (const event of stream)`
//! 3. Terminates with `end(result)` carrying a final value
//! 4. Supports multiple listeners via `tee()`
//! 5. Handles backpressure — producers wait if no consumer is ready
//!
//! In Rust, implement this as a wrapper around `tokio::sync::mpsc`:
//!
//! ```rust
//! pub struct EventStream<T, R> {
//!     rx: tokio::sync::mpsc::UnboundedReceiver<StreamEvent<T, R>>,
//! }
//!
//! enum StreamEvent<T, R> {
//!     Item(T),
//!     End(R),
//! }
//!
//! impl<T, R> EventStream<T, R> {
//!     pub fn new() -> (EventStreamSender<T, R>, Self) { ... }
//!     pub async fn next(&mut self) -> Option<T> { ... }
//!     pub async fn into_result(self) -> R { ... }
//! }
//! ```
//!
//! # Key Behaviors
//!
//! - **Guaranteed delivery**: every pushed event is received by the consumer
//! - **Single consumer**: the stream is consumed once (no Clone)
//! - **Termination**: after `end(result)`, consumers get only the final result
//! - **Error propagation**: errors are regular events, not stream failures
//!
//! # Dependencies
//!
//! - `tokio::sync::mpsc`
//! - `std::pin::Pin`
//! - `futures_core::Stream` for trait impl

