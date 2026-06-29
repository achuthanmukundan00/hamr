//! Reusable countdown timer for dialog components.
//!
//! Port of `packages/coding-agent/src/modes/interactive/components/countdown-timer.ts`.

use std::sync::mpsc::{self, Sender};
use std::thread;
use std::time::Duration;

/// A countdown timer that ticks every second and calls `on_tick` and `on_expire` callbacks.
pub struct CountdownTimer {
    /// Signal sender to stop the timer thread.
    stop_tx: Option<Sender<()>>,
}

impl CountdownTimer {
    /// Create a new countdown timer.
    ///
    /// * `timeout_ms` - total duration in milliseconds
    /// * `on_tick` - called every second with the remaining seconds
    /// * `on_expire` - called when the countdown reaches zero
    pub fn new<F, G>(timeout_ms: u64, on_tick: F, on_expire: G) -> Self
    where
        F: Fn(u64) + Send + 'static,
        G: Fn() + Send + 'static,
    {
        let remaining = ((timeout_ms as f64) / 1000.0).ceil() as u64;

        // Initial tick
        on_tick(remaining);

        let (stop_tx, stop_rx) = mpsc::channel();

        let tx = stop_tx.clone();
        thread::spawn(move || {
            let mut rem = remaining;
            loop {
                // Check stop signal every 100ms to be responsive
                for _ in 0..10 {
                    if stop_rx.try_recv().is_ok() {
                        return;
                    }
                    thread::sleep(Duration::from_millis(100));
                }

                rem = rem.saturating_sub(1);
                on_tick(rem);

                if rem == 0 {
                    on_expire();
                    return;
                }
            }
        });

        Self { stop_tx: Some(tx) }
    }

    /// Stop the timer and clean up the thread.
    pub fn dispose(&mut self) {
        if let Some(tx) = self.stop_tx.take() {
            let _ = tx.send(());
        }
    }
}

impl Drop for CountdownTimer {
    fn drop(&mut self) {
        self.dispose();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;
    use std::time::Duration;

    #[test]
    fn test_countdown_completes() {
        let (done_tx, done_rx) = mpsc::channel();

        let _timer = CountdownTimer::new(
            500, // 500ms
            |_| {},
            move || {
                done_tx.send(()).ok();
            },
        );

        // Should complete within 2 seconds
        assert!(done_rx.recv_timeout(Duration::from_secs(2)).is_ok());
    }

    #[test]
    fn test_countdown_dispose_stops_timer() {
        let (tick_tx, tick_rx) = mpsc::channel();

        let mut timer = CountdownTimer::new(
            2000, // 2 seconds
            move |rem| {
                tick_tx.send(rem).ok();
            },
            || {},
        );

        // Get the first tick
        let first = tick_rx.recv_timeout(Duration::from_secs(2));
        assert!(first.is_ok());

        // Dispose immediately
        timer.dispose();

        // Give it a moment, then check no more ticks arrive
        thread::sleep(Duration::from_millis(200));
        assert!(tick_rx.try_recv().is_err());
    }

    #[test]
    fn test_countdown_ticks_decrease() {
        let (tick_tx, tick_rx) = mpsc::channel();
        let ticks = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let ticks_clone = ticks.clone();

        let _timer = CountdownTimer::new(
            1500, // 1.5 seconds → ceil = 2 ticks
            move |rem| {
                ticks.lock().unwrap().push(rem);
                let _ = tick_tx.send(rem);
            },
            || {},
        );

        // Wait for completion
        let _ = tick_rx.recv_timeout(Duration::from_secs(3));
        let _ = tick_rx.recv_timeout(Duration::from_secs(3));
        thread::sleep(Duration::from_millis(200));

        let recorded = ticks_clone.lock().unwrap();
        // Should be monotonically decreasing: 2, 1, 0
        assert!(recorded.len() >= 2);
        assert!(recorded[0] >= recorded[1]);
    }
}
