//! Port of `../../packages/ai/src/utils/event-stream.ts`.
//!
//! A generic event stream that producers push to and consumers iterate over,
//! plus an awaitable final result that resolves when a terminal event is pushed.
//! This is the **fundamental communication primitive** between providers and the
//! agent loop.
//!
//! ## TS → Rust shape
//!
//! The TS `EventStream<T, R>` is a single object that is both pushed-to and
//! async-iterated. In Rust we split it into two halves that share the same
//! channel + result slot:
//!
//! - [`EventStreamSender<T, R>`] — the producer half: `push(event)` / `end(result?)`.
//! - [`EventStream<T, R>`] — the consumer half: implements [`futures::Stream`] and
//!   exposes an async [`EventStream::result`] that resolves once an `is_complete`
//!   event is pushed (or `end(Some(result))` is called).
//!
//! Events flow over a `tokio::sync::mpsc::unbounded_channel`. The final result is
//! held in an `Arc<Mutex<Option<R>>>` paired with a `tokio::sync::Notify`, so
//! `result()` can be awaited independently of iteration (matching the TS
//! `finalResultPromise`).

use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context as TaskContext, Poll};

use futures::Stream;
use tokio::sync::{Notify, mpsc};

use crate::types::{AssistantMessage, AssistantMessageEvent};

/// Shared final-result slot, resolved exactly once.
struct ResultSlot<R> {
    value: Mutex<Option<R>>,
    notify: Notify,
}

impl<R> ResultSlot<R> {
    fn new() -> Self {
        Self {
            value: Mutex::new(None),
            notify: Notify::new(),
        }
    }

    /// Store the result if not already set, and wake any waiters.
    fn resolve(&self, result: R) {
        let mut guard = self.value.lock().unwrap_or_else(|e| e.into_inner());
        if guard.is_none() {
            *guard = Some(result);
            self.notify.notify_waiters();
        }
    }
}

/// Producer half of an [`EventStream`].
///
/// Mirrors the `push`/`end` methods of the TS `EventStream`.
pub struct EventStreamSender<T, R> {
    tx: mpsc::UnboundedSender<T>,
    result: Arc<ResultSlot<R>>,
    is_complete: Arc<dyn Fn(&T) -> bool + Send + Sync>,
    extract_result: Arc<dyn Fn(T) -> R + Send + Sync>,
    done: bool,
}

impl<T, R> EventStreamSender<T, R>
where
    T: Clone,
{
    /// Push an event to the consumer.
    ///
    /// If the event satisfies `is_complete`, the final result is resolved from it
    /// (the event is still delivered to the consumer, matching the TS behavior).
    /// After completion further pushes are ignored.
    pub fn push(&mut self, event: T) {
        if self.done {
            return;
        }
        if (self.is_complete)(&event) {
            self.done = true;
            let result = (self.extract_result)(event.clone());
            self.result.resolve(result);
        }
        // Ignore send errors: a dropped consumer simply means nobody is listening.
        let _ = self.tx.send(event);
    }

    /// End the stream, optionally carrying a final result.
    ///
    /// Marks the sender done (further `push`es are no-ops) and resolves the final
    /// result if provided. End-of-iteration is signalled to the consumer when this
    /// sender is dropped, which closes the underlying channel.
    pub fn end(&mut self, result: Option<R>) {
        self.done = true;
        if let Some(result) = result {
            self.result.resolve(result);
        }
    }
}

/// Consumer half of an event stream.
///
/// Implements [`futures::Stream`] for `for await`-style iteration and exposes an
/// awaitable [`EventStream::result`].
pub struct EventStream<T, R> {
    rx: mpsc::UnboundedReceiver<T>,
    result: Arc<ResultSlot<R>>,
}

impl<T, R> EventStream<T, R>
where
    R: Clone,
{
    /// Create a connected (sender, consumer) pair.
    ///
    /// - `is_complete` decides whether a pushed event terminates the result.
    /// - `extract_result` derives the final `R` from a completing event.
    pub fn new(
        is_complete: impl Fn(&T) -> bool + Send + Sync + 'static,
        extract_result: impl Fn(T) -> R + Send + Sync + 'static,
    ) -> (EventStreamSender<T, R>, EventStream<T, R>) {
        let (tx, rx) = mpsc::unbounded_channel();
        let result = Arc::new(ResultSlot::new());
        let sender = EventStreamSender {
            tx,
            result: Arc::clone(&result),
            is_complete: Arc::new(is_complete),
            extract_result: Arc::new(extract_result),
            done: false,
        };
        let stream = EventStream { rx, result };
        (sender, stream)
    }

    /// Await the final result, resolving when a completing event is pushed (or
    /// `end(Some(result))` is called).
    ///
    /// This can be awaited independently of iteration. If the stream ends without
    /// a result ever being resolved, this future never completes — callers that
    /// might hit that case should also drive iteration / use a timeout.
    pub async fn result(&self) -> R {
        loop {
            {
                let guard = self.result.value.lock().unwrap_or_else(|e| e.into_inner());
                if let Some(value) = guard.as_ref() {
                    return value.clone();
                }
            }
            self.result.notify.notified().await;
        }
    }

    /// Receive the next event, or `None` when the stream has ended.
    pub async fn next_event(&mut self) -> Option<T> {
        self.rx.recv().await
    }
}

impl<T, R> Stream for EventStream<T, R> {
    type Item = T;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut TaskContext<'_>) -> Poll<Option<T>> {
        self.rx.poll_recv(cx)
    }
}

// ---------------------------------------------------------------------------
// AssistantMessageEventStream
// ---------------------------------------------------------------------------

/// The concrete event stream used across the AI layer:
/// `EventStream<AssistantMessageEvent, AssistantMessage>`.
///
/// Completes on `done`/`error` events; `done` → message, `error` → error message.
pub type AssistantMessageEventStream = EventStream<AssistantMessageEvent, AssistantMessage>;

/// The producer half paired with [`AssistantMessageEventStream`].
pub type AssistantMessageEventStreamSender =
    EventStreamSender<AssistantMessageEvent, AssistantMessage>;

/// Factory for an [`AssistantMessageEventStream`] (for use in providers/extensions).
///
/// Mirrors the TS `createAssistantMessageEventStream()`, returning both halves.
pub fn create_assistant_message_event_stream() -> (
    AssistantMessageEventStreamSender,
    AssistantMessageEventStream,
) {
    AssistantMessageEventStream::new(
        |event: &AssistantMessageEvent| {
            matches!(
                event,
                AssistantMessageEvent::Done { .. } | AssistantMessageEvent::Error { .. }
            )
        },
        |event: AssistantMessageEvent| match event {
            AssistantMessageEvent::Done { message, .. } => message,
            AssistantMessageEvent::Error { error, .. } => error,
            // Unreachable: `is_complete` only matches Done/Error.
            _ => unreachable!("extract_result called on non-terminal event"),
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{DoneReason, MessageRole, StopReason, Usage, UsageCost};
    use chrono::Utc;
    use futures::StreamExt;

    fn dummy_message(text: &str) -> AssistantMessage {
        AssistantMessage {
            role: MessageRole::Assistant,
            content: Vec::new(),
            api: "anthropic-messages".to_string(),
            provider: "anthropic".to_string(),
            model: text.to_string(),
            response_model: None,
            response_id: None,
            usage: Usage {
                input: 0,
                output: 0,
                cache_read: 0,
                cache_write: 0,
                cache_write_1h: None,
                total_tokens: 0,
                cost: UsageCost {
                    input: 0.0,
                    output: 0.0,
                    cache_read: 0.0,
                    cache_write: 0.0,
                    total: 0.0,
                },
            },
            stop_reason: StopReason::Stop,
            error_message: None,
            diagnostics: None,
            timestamp: Utc::now(),
        }
    }

    #[tokio::test]
    async fn push_iterate_and_result() {
        let (mut tx, mut stream) = create_assistant_message_event_stream();
        tx.push(AssistantMessageEvent::Start {
            partial: dummy_message("partial"),
        });
        tx.push(AssistantMessageEvent::Done {
            reason: DoneReason::Stop,
            message: dummy_message("final"),
        });

        // First event is the start.
        let first = stream.next().await.expect("start event");
        assert!(matches!(first, AssistantMessageEvent::Start { .. }));

        // Second event is the done.
        let second = stream.next().await.expect("done event");
        assert!(matches!(second, AssistantMessageEvent::Done { .. }));

        // Result resolved from the done event.
        let result = stream.result().await;
        assert_eq!(result.model, "final");
    }

    #[tokio::test]
    async fn result_resolves_independently_of_iteration() {
        let (mut tx, stream) = create_assistant_message_event_stream();
        tx.push(AssistantMessageEvent::Done {
            reason: DoneReason::Stop,
            message: dummy_message("done"),
        });
        // Never iterate; result still resolves.
        let result = stream.result().await;
        assert_eq!(result.model, "done");
    }

    #[tokio::test]
    async fn error_event_resolves_result_to_error_message() {
        use crate::types::ErrorReason;
        let (mut tx, stream) = create_assistant_message_event_stream();
        tx.push(AssistantMessageEvent::Error {
            reason: ErrorReason::Error,
            error: dummy_message("boom"),
        });
        let result = stream.result().await;
        assert_eq!(result.model, "boom");
    }

    #[tokio::test]
    async fn end_without_result_terminates_iteration() {
        let (mut tx, mut stream) = create_assistant_message_event_stream();
        tx.push(AssistantMessageEvent::Start {
            partial: dummy_message("only"),
        });
        tx.end(None);
        // Drop the sender so the channel closes.
        drop(tx);

        let first = stream.next().await.expect("start event");
        assert!(matches!(first, AssistantMessageEvent::Start { .. }));
        // Channel closed, no more events.
        assert!(stream.next().await.is_none());
    }

    #[tokio::test]
    async fn push_after_complete_is_ignored() {
        let (mut tx, mut stream) = create_assistant_message_event_stream();
        tx.push(AssistantMessageEvent::Done {
            reason: DoneReason::Stop,
            message: dummy_message("first"),
        });
        // This push should be dropped because the stream is already done.
        tx.push(AssistantMessageEvent::Done {
            reason: DoneReason::Stop,
            message: dummy_message("second"),
        });
        drop(tx);

        let first = stream.next().await.expect("done event");
        assert!(matches!(first, AssistantMessageEvent::Done { .. }));
        assert!(stream.next().await.is_none());

        let result = stream.result().await;
        assert_eq!(result.model, "first");
    }
}
