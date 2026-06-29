//! Port of `packages/ai/src/utils/oauth/device_code.ts`
//!
//! RFC 8628 Device Authorization Grant polling loop.
//! Generic helper used by multiple OAuth providers.

use crate::utils::oauth::types::OAuthError;
use tokio::time::{Duration, sleep};

/// Result of a single poll attempt.
pub enum DeviceCodePollResult<T> {
    /// Authorization is still pending – keep polling.
    Pending,
    /// The authorization server requested a slower polling interval.
    SlowDown,
    /// The flow completed successfully.
    Complete(T),
    /// The flow failed with a message.
    Failed(String),
}

/// Options for the device code polling loop.
pub struct DeviceCodePollOptions<T, F, Fut>
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = DeviceCodePollResult<T>>,
{
    /// Initial polling interval in seconds (default: 5 per RFC 8628 §3.2).
    pub interval_seconds: Option<u64>,
    /// Maximum time to poll in seconds.
    pub expires_in_seconds: Option<u64>,
    /// The poll function to call on each interval.
    pub poll: F,
    /// Optional abort signal.
    pub aborted: Option<tokio::sync::watch::Receiver<bool>>,
}

const DEFAULT_POLL_INTERVAL_SECONDS: u64 = 5;
const SLOW_DOWN_INTERVAL_INCREMENT_MS: u64 = 5000;
const MINIMUM_INTERVAL_MS: u64 = 1000;

/// Poll the device code endpoint until complete, failed, timed out, or cancelled.
///
/// Returns the completed value on success.
pub async fn poll_oauth_device_code_flow<T, F, Fut>(
    options: DeviceCodePollOptions<T, F, Fut>,
) -> Result<T, OAuthError>
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = DeviceCodePollResult<T>>,
{
    let deadline = options
        .expires_in_seconds
        .map(|s| tokio::time::Instant::now() + Duration::from_secs(s));

    let mut interval_ms = std::cmp::max(
        MINIMUM_INTERVAL_MS,
        options
            .interval_seconds
            .unwrap_or(DEFAULT_POLL_INTERVAL_SECONDS)
            * 1000,
    );

    let mut slow_down_count = 0u32;

    loop {
        // Check for cancellation
        if let Some(ref aborted_rx) = options.aborted {
            if *aborted_rx.borrow() {
                return Err(OAuthError::Cancelled);
            }
        }
        // Ensure we don't start a poll past the deadline
        if let Some(dl) = deadline {
            if tokio::time::Instant::now() >= dl {
                break;
            }
        }

        let result = (options.poll)().await;

        match result {
            DeviceCodePollResult::Complete(value) => return Ok(value),
            DeviceCodePollResult::Failed(msg) => return Err(OAuthError::Failed(msg)),
            DeviceCodePollResult::SlowDown => {
                slow_down_count += 1;
                interval_ms = std::cmp::max(
                    MINIMUM_INTERVAL_MS,
                    interval_ms + SLOW_DOWN_INTERVAL_INCREMENT_MS,
                );
            }
            DeviceCodePollResult::Pending => {
                // keep polling at current rate
            }
        }

        // Check deadline again before sleeping
        if let Some(dl) = deadline {
            let remaining = dl
                .checked_duration_since(tokio::time::Instant::now())
                .unwrap_or(Duration::ZERO);
            if remaining.is_zero() {
                break;
            }
            sleep(std::cmp::min(Duration::from_millis(interval_ms), remaining)).await;
        } else {
            sleep(Duration::from_millis(interval_ms)).await;
        }
    }

    Err(if slow_down_count > 0 {
        OAuthError::SlowDownTimeout
    } else {
        OAuthError::Timeout
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_complete_on_first_poll() {
        let options = DeviceCodePollOptions {
            interval_seconds: Some(1),
            expires_in_seconds: None,
            poll: || async { DeviceCodePollResult::Complete(42) },
            aborted: None,
        };
        let result = poll_oauth_device_code_flow(options).await.unwrap();
        assert_eq!(result, 42);
    }

    #[tokio::test]
    async fn test_failed_on_first_poll() {
        let options = DeviceCodePollOptions {
            interval_seconds: Some(1),
            expires_in_seconds: None,
            poll: || async { DeviceCodePollResult::Failed("boom".to_string()) },
            aborted: None,
        };
        let err = poll_oauth_device_code_flow::<(), _, _>(options)
            .await
            .unwrap_err();
        assert!(matches!(err, OAuthError::Failed(msg) if msg == "boom"));
    }

    #[tokio::test]
    async fn test_cancelled_before_poll() {
        let (tx, rx) = tokio::sync::watch::channel(true);
        drop(tx);
        let options = DeviceCodePollOptions {
            interval_seconds: Some(1),
            expires_in_seconds: None,
            poll: || async { panic!("should not be called") },
            aborted: Some(rx),
        };
        let err = poll_oauth_device_code_flow::<(), _, _>(options)
            .await
            .unwrap_err();
        assert!(matches!(err, OAuthError::Cancelled));
    }

    #[tokio::test]
    async fn test_timeout() {
        let options = DeviceCodePollOptions {
            interval_seconds: Some(1),
            expires_in_seconds: Some(0),
            poll: || async { DeviceCodePollResult::<()>::Pending },
            aborted: None,
        };
        let err = poll_oauth_device_code_flow(options).await.unwrap_err();
        assert!(matches!(err, OAuthError::Timeout));
    }
}
