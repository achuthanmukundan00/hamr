//! Integration tests for the event bus.
//!
//! These test the public API surface: `EventBus`, `EventBusController`, and
//! `create_event_bus`.  The unit tests in `event_bus.rs` cover finer-grained
//! internals; these integration tests exercise end-to-end behavior from a
//! crate-external vantage point.

use hamr_agent::core::event_bus::{EventBus, EventBusController, create_event_bus};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

/// Handlers should be simpler to write than the full `Handler` type alias —
/// users just pass a closure that takes a String and returns a boxed future.
fn simple_handler(
    counter: Arc<AtomicUsize>,
) -> Box<
    dyn Fn(String) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>> + Send + Sync,
> {
    Box::new(move |_data: String| {
        let c = counter.clone();
        Box::pin(async move {
            c.fetch_add(1, Ordering::SeqCst);
        })
    })
}

#[tokio::test]
async fn emit_on_basic() {
    let bus = create_event_bus();
    let counter = Arc::new(AtomicUsize::new(0));

    let _unsub = bus.on("test", simple_handler(counter.clone())).await;
    bus.emit("test", "hello").await;

    tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    assert_eq!(counter.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn unsubscribe_works() {
    let bus = create_event_bus();
    let counter = Arc::new(AtomicUsize::new(0));

    let unsub = bus.on("test", simple_handler(counter.clone())).await;
    bus.emit("test", "first").await;
    tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    assert_eq!(counter.load(Ordering::SeqCst), 1);

    unsub.unsubscribe();

    bus.emit("test", "second").await;
    tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    assert_eq!(counter.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn clear_removes_all() {
    let bus = create_event_bus();
    let counter = Arc::new(AtomicUsize::new(0));

    let _unsub = bus.on("test", simple_handler(counter.clone())).await;
    bus.emit("test", "before").await;
    tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    assert_eq!(counter.load(Ordering::SeqCst), 1);

    bus.clear().await;

    bus.emit("test", "after").await;
    tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    assert_eq!(counter.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn handler_panics_are_caught() {
    let bus = create_event_bus();
    let counter = Arc::new(AtomicUsize::new(0));
    let c_for_closure = counter.clone();

    let _unsub = bus
        .on(
            "test",
            Box::new(move |_data: String| {
                let c = c_for_closure.clone();
                Box::pin(async move {
                    c.fetch_add(1, Ordering::SeqCst);
                    panic!("boom");
                })
            }),
        )
        .await;

    // This must not panic.
    bus.emit("test", "msg").await;
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    assert_eq!(counter.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn multiple_channels_independent() {
    let bus = create_event_bus();
    let a = Arc::new(AtomicUsize::new(0));
    let b = Arc::new(AtomicUsize::new(0));

    let _u1 = bus.on("a", simple_handler(a.clone())).await;
    let _u2 = bus.on("b", simple_handler(b.clone())).await;

    bus.emit("a", "x").await;
    tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    assert_eq!(a.load(Ordering::SeqCst), 1);
    assert_eq!(b.load(Ordering::SeqCst), 0);

    bus.emit("b", "y").await;
    tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    assert_eq!(a.load(Ordering::SeqCst), 1);
    assert_eq!(b.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn multiple_handlers_same_channel() {
    let bus = create_event_bus();
    let c1 = Arc::new(AtomicUsize::new(0));
    let c2 = Arc::new(AtomicUsize::new(0));

    let _u1 = bus.on("test", simple_handler(c1.clone())).await;
    let _u2 = bus.on("test", simple_handler(c2.clone())).await;

    bus.emit("test", "msg").await;
    tokio::time::sleep(std::time::Duration::from_millis(20)).await;

    assert_eq!(c1.load(Ordering::SeqCst), 1);
    assert_eq!(c2.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn emit_without_handlers_is_noop() {
    let bus = create_event_bus();
    // Should not hang, should not panic.
    bus.emit("no-listeners", "data").await;
}
