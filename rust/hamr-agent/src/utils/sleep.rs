//! Port of `packages/coding-agent/src/utils/sleep.ts`
//!
//! Sleep helper that respects an abort signal via a tokio watch channel.

use tokio::sync::watch;

/// Sleep for `ms` milliseconds, respecting an optional abort signal.
///
/// The signal is a `watch::Receiver<bool>` — when it becomes `true`, the sleep is
/// interrupted and an `"Aborted"` error is returned.
pub async fn sleep(ms: u64, signal: Option<&mut watch::Receiver<bool>>) -> Result<(), String> {
    if signal.as_ref().map_or(false, |s| *s.borrow()) {
        return Err("Aborted".to_string());
    }

    tokio::select! {
        _ = tokio::time::sleep(tokio::time::Duration::from_millis(ms)) => {
            Ok(())
        }
        _ = async {
            if let Some(s) = signal {
                s.changed().await.ok();
            } else {
                std::future::pending::<()>().await;
            }
        } => {
            Err("Aborted".to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use tokio::sync::watch;

    #[tokio::test]
    async fn test_sleep_completes_without_signal() {
        let start = std::time::Instant::now();
        sleep(50, None).await.unwrap();
        let elapsed = start.elapsed();
        assert!(elapsed >= Duration::from_millis(40));
    }

    #[tokio::test]
    async fn test_sleep_aborts_when_signal_fires() {
        let (tx, mut rx) = watch::channel(false);

        // Send abort after 10ms
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(10)).await;
            tx.send(true).ok();
        });

        let result = sleep(5000, Some(&mut rx)).await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Aborted");
    }

    #[tokio::test]
    async fn test_sleep_returns_immediately_when_already_aborted() {
        let (tx, mut rx) = watch::channel(true);
        drop(tx); // prevent hang on changed()

        let result = sleep(5000, Some(&mut rx)).await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Aborted");
    }
}
