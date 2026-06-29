//! Port of `packages/ai/src/utils/abort-signals.ts`.
//!
//! Combine multiple abort signals into one. The Web `AbortSignal` maps to a
//! [`tokio::sync::watch::Receiver<bool>`] (see [`crate::types::StreamOptions`]),
//! where `true` means "aborted". Combining forwards the first abort from any
//! input signal onto a fresh output signal.

use std::sync::Arc;

use tokio::sync::watch;
use tokio::task::JoinHandle;

/// Result of [`combine_abort_signals`]: an optional combined signal plus a
/// cleanup handle that detaches the forwarding tasks.
pub struct CombinedAbortSignal {
    /// The combined signal, or `None` when no input signals were provided.
    pub signal: Option<watch::Receiver<bool>>,
    tasks: Vec<JoinHandle<()>>,
}

impl CombinedAbortSignal {
    /// Detach the internal forwarding tasks. Mirrors the TS `cleanup()` callback.
    pub fn cleanup(&self) {
        for task in &self.tasks {
            task.abort();
        }
    }
}

impl Drop for CombinedAbortSignal {
    fn drop(&mut self) {
        self.cleanup();
    }
}

/// Combine a set of optional abort signals into a single signal.
///
/// - Zero active signals → `signal: None`.
/// - One active signal → that signal is returned directly (no forwarding task).
/// - Many → a fresh signal that aborts as soon as any input aborts.
pub fn combine_abort_signals(signals: &[Option<watch::Receiver<bool>>]) -> CombinedAbortSignal {
    let active: Vec<watch::Receiver<bool>> = signals.iter().flatten().cloned().collect();

    if active.is_empty() {
        return CombinedAbortSignal {
            signal: None,
            tasks: Vec::new(),
        };
    }
    if active.len() == 1 {
        return CombinedAbortSignal {
            signal: Some(active.into_iter().next().unwrap()),
            tasks: Vec::new(),
        };
    }

    let (tx, rx) = watch::channel(false);
    let tx = Arc::new(tx);
    let mut tasks = Vec::with_capacity(active.len());

    for mut receiver in active {
        let tx = Arc::clone(&tx);
        tasks.push(tokio::spawn(async move {
            loop {
                if *receiver.borrow() {
                    let _ = tx.send(true);
                    return;
                }
                if receiver.changed().await.is_err() {
                    // Source dropped without aborting — stop watching it.
                    return;
                }
            }
        }));
    }

    CombinedAbortSignal {
        signal: Some(rx),
        tasks,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn no_signals_returns_none() {
        let combined = combine_abort_signals(&[]);
        assert!(combined.signal.is_none());
    }

    #[tokio::test]
    async fn single_signal_passthrough() {
        let (_tx, rx) = watch::channel(false);
        let combined = combine_abort_signals(&[Some(rx)]);
        assert!(combined.signal.is_some());
        assert!(combined.tasks.is_empty());
    }

    #[tokio::test]
    async fn combined_aborts_when_any_aborts() {
        let (tx_a, rx_a) = watch::channel(false);
        let (_tx_b, rx_b) = watch::channel(false);
        let combined = combine_abort_signals(&[Some(rx_a), Some(rx_b)]);
        let mut signal = combined.signal.clone().unwrap();
        assert!(!*signal.borrow());
        tx_a.send(true).unwrap();
        signal.changed().await.unwrap();
        assert!(*signal.borrow());
    }

    #[tokio::test]
    async fn combined_reflects_already_aborted() {
        let (tx_a, rx_a) = watch::channel(false);
        tx_a.send(true).unwrap();
        let (_tx_b, rx_b) = watch::channel(false);
        let combined = combine_abort_signals(&[Some(rx_a), Some(rx_b)]);
        let mut signal = combined.signal.clone().unwrap();
        // Forwarding task should observe the already-set value and propagate it.
        signal.changed().await.unwrap();
        assert!(*signal.borrow());
    }
}
