//! In-process typed event bus for decoupled communication between
//! agent subsystems (tools, extensions, TUI, compaction).
//!
//! Port of `packages/coding-agent/src/core/event-bus.ts`.
//!
//! The TypeScript implementation is a thin wrapper around Node's
//! `EventEmitter`.  This Rust port uses spawned tasks with mpsc channels.

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::Mutex;

// ---------------------------------------------------------------------------
// Handler type
// ---------------------------------------------------------------------------

/// An async handler — receives the event payload as a `String`.
pub type Handler = Box<dyn Fn(String) -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + Sync>;

// ---------------------------------------------------------------------------
// Public traits
// ---------------------------------------------------------------------------

pub trait EventBus: Send + Sync {
    fn emit(&self, channel: &str, data: &str) -> Pin<Box<dyn Future<Output = ()> + Send + '_>>;

    fn on(
        &self,
        channel: &str,
        handler: Handler,
    ) -> Pin<Box<dyn Future<Output = Unsubscribe> + Send + '_>>;
}

pub trait EventBusController: EventBus {
    fn clear(&self) -> Pin<Box<dyn Future<Output = ()> + Send + '_>>;
}

// ---------------------------------------------------------------------------
// Unsubscribe handle
// ---------------------------------------------------------------------------

pub struct Unsubscribe {
    cancelled: Arc<AtomicBool>,
    // handler_handle is aborted on unsub to stop the worker task.
    handler_handle: Option<tokio::task::JoinHandle<()>>,
}

impl Unsubscribe {
    /// Synchronously cancel this subscription.
    /// After this returns, no further events will be delivered to the handler.
    pub fn unsubscribe(mut self) {
        self.cancelled.store(true, Ordering::SeqCst);
        if let Some(handle) = self.handler_handle.take() {
            handle.abort();
        }
    }
}

impl Drop for Unsubscribe {
    fn drop(&mut self) {
        self.cancelled.store(true, Ordering::SeqCst);
        if let Some(handle) = self.handler_handle.take() {
            handle.abort();
        }
    }
}

// ---------------------------------------------------------------------------
// Factory
// ---------------------------------------------------------------------------

pub fn create_event_bus() -> impl EventBusController {
    BusController {
        inner: Arc::new(BusInner {
            channels: Mutex::new(HashMap::new()),
        }),
    }
}

// ---------------------------------------------------------------------------
// Implementation details
// ---------------------------------------------------------------------------

struct BusInner {
    channels: Mutex<HashMap<String, Vec<tokio::sync::mpsc::UnboundedSender<String>>>>,
}

struct BusController {
    inner: Arc<BusInner>,
}

impl EventBus for BusController {
    fn emit(&self, channel: &str, data: &str) -> Pin<Box<dyn Future<Output = ()> + Send + '_>> {
        let inner = Arc::clone(&self.inner);
        let channel = channel.to_string();
        let data = data.to_string();
        Box::pin(async move {
            let txs: Vec<tokio::sync::mpsc::UnboundedSender<String>> = {
                let ch = inner.channels.lock().await;
                ch.get(&channel).map(|v| v.clone()).unwrap_or_default()
            };
            for tx in txs {
                let _ = tx.send(data.clone());
            }
        })
    }

    fn on(
        &self,
        channel: &str,
        handler: Handler,
    ) -> Pin<Box<dyn Future<Output = Unsubscribe> + Send + '_>> {
        let inner = Arc::clone(&self.inner);
        let channel = channel.to_string();
        Box::pin(async move {
            let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<String>();
            let cancelled = Arc::new(AtomicBool::new(false));
            let cancelled_clone = Arc::clone(&cancelled);

            // Spawn handler worker — checks cancelled flag before each dispatch.
            let handle = tokio::spawn(async move {
                while let Some(data) = rx.recv().await {
                    if cancelled_clone.load(Ordering::SeqCst) {
                        break;
                    }
                    let fut = handler(data);
                    let _ = fut.await;
                }
            });

            // Register in channel map (for emit to find)
            let mut ch = inner.channels.lock().await;
            ch.entry(channel.clone()).or_default().push(tx);

            Unsubscribe {
                cancelled,
                handler_handle: Some(handle),
            }
        })
    }
}

impl EventBusController for BusController {
    fn clear(&self) -> Pin<Box<dyn Future<Output = ()> + Send + '_>> {
        let inner = Arc::clone(&self.inner);
        Box::pin(async move {
            let mut ch = inner.channels.lock().await;
            ch.clear();
        })
    }
}
