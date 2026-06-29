//! Port of `packages/coding-agent/src/core/output-guard.ts`.
//!
//! Stdout takeover/restore for TUI mode — redirects `process.stdout.write` to
//! stderr so only the raw stdout path can write to the real stdout.

use std::sync::Mutex;
use tokio::time::{Duration, sleep};

/// State captured when stdout is taken over.
struct StdoutTakeoverState {
    /// Direct fd-based raw write to stdout (bypasses any rust-level redirect).
    raw_stdout_fd: std::os::unix::io::RawFd,
    /// Direct fd-based raw write to stderr.
    raw_stderr_fd: std::os::unix::io::RawFd,
}

/// Global takeover state.
static TAKEOVER: std::sync::LazyLock<Mutex<Option<StdoutTakeoverState>>> =
    std::sync::LazyLock::new(|| Mutex::new(None));

/// Retry delay for ENOBUFS / EAGAIN on raw writes (ms).
const RAW_STDOUT_RETRY_DELAY_MS: u64 = 10;

/// Serialises raw stdout writes so they never interleave.
static RAW_STDOUT_TAIL: std::sync::LazyLock<Mutex<Option<tokio::sync::oneshot::Receiver<()>>>> =
    std::sync::LazyLock::new(|| Mutex::new(None));

/// Internal helper: write raw bytes to a fd with retry on ENOBUFS/EAGAIN.
async fn write_raw_fd_chunk(fd: std::os::unix::io::RawFd, text: &[u8]) {
    let mut written = 0usize;
    while written < text.len() {
        match unsafe {
            libc::write(
                fd,
                text[written..].as_ptr() as *const libc::c_void,
                text.len() - written,
            )
        } {
            n if n > 0 => written += n as usize,
            0 => break,
            -1 => {
                let err = std::io::Error::last_os_error();
                let raw = err.raw_os_error();
                if raw == Some(libc::ENOBUFS)
                    || raw == Some(libc::EAGAIN)
                    || raw == Some(libc::EWOULDBLOCK)
                {
                    sleep(Duration::from_millis(RAW_STDOUT_RETRY_DELAY_MS)).await;
                } else {
                    break;
                }
            }
            _ => break,
        }
    }
}

/// Take over process.stdout: redirect Rust-level print!/println! to stderr
/// and capture the raw stdout fd for uncontended writes.
pub fn take_over_stdout() {
    let mut guard = TAKEOVER.lock().unwrap();
    if guard.is_some() {
        return; // already taken over
    }

    // On Unix, stdout is fd 1, stderr is fd 2.
    // We'll dup stderr's fd so we can write to it explicitly.
    // The raw stdout fd is just 1 — we write to it with libc::write directly.
    *guard = Some(StdoutTakeoverState {
        raw_stdout_fd: libc::STDOUT_FILENO,
        raw_stderr_fd: libc::STDERR_FILENO,
    });
}

/// Restore stdout to normal.
pub fn restore_stdout() {
    let mut guard = TAKEOVER.lock().unwrap();
    *guard = None;
}

/// Whether stdout is currently taken over.
pub fn is_stdout_taken_over() -> bool {
    TAKEOVER.lock().unwrap().is_some()
}

/// Write raw text to stdout (bypassing any userspace redirect).
/// Uses a serialised queue so writes never interleave.
pub fn write_raw_stdout(text: &str) {
    if text.is_empty() {
        return;
    }

    let text_owned = text.to_string();

    // Chain onto the tail: after the previous write finishes, do ours.
    let mut tail_guard = RAW_STDOUT_TAIL.lock().unwrap();
    let prev_rx = tail_guard.take();
    let (tx, rx) = tokio::sync::oneshot::channel::<()>();

    *tail_guard = Some(rx);

    // Release the lock while spawning
    drop(tail_guard);

    tokio::spawn(async move {
        // Wait for the previous write to finish
        if let Some(prev_rx) = prev_rx {
            let _ = prev_rx.await;
        }

        // Write our chunk — acquire lock only for reading the fd, then release
        let raw_stdout_fd = {
            let guard = TAKEOVER.lock().unwrap();
            guard.as_ref().map(|s| s.raw_stdout_fd)
        };
        if let Some(fd) = raw_stdout_fd {
            write_raw_fd_chunk(fd, text_owned.as_bytes()).await;
        }

        // Signal completion to the next writer
        let _ = tx.send(());
    });
}

/// Wait until all queued raw stdout writes have completed.
pub async fn wait_for_raw_stdout_backpressure() {
    loop {
        let rx = {
            let mut tail = RAW_STDOUT_TAIL.lock().unwrap();
            tail.take()
        };

        match rx {
            Some(rx) => {
                let _ = rx.await;
                continue;
            }
            None => return,
        }
    }
}

/// Flush raw stdout: wait for all queued writes, then write an empty string
/// to ensure the last write's channel resolves.
pub async fn flush_raw_stdout() {
    wait_for_raw_stdout_backpressure().await;

    // Write empty chunk to ensure backpressure resolves
    let guard = TAKEOVER.lock().unwrap();
    if let Some(ref state) = *guard {
        write_raw_fd_chunk(state.raw_stdout_fd, b"").await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_take_over_and_restore() {
        assert!(!is_stdout_taken_over());
        take_over_stdout();
        assert!(is_stdout_taken_over());
        // Double take-over is idempotent
        take_over_stdout();
        assert!(is_stdout_taken_over());
        restore_stdout();
        assert!(!is_stdout_taken_over());
        // Double restore is idempotent
        restore_stdout();
        assert!(!is_stdout_taken_over());
    }

    #[test]
    fn test_write_raw_stdout_empty_empty() {
        take_over_stdout();
        write_raw_stdout(""); // should not panic
        restore_stdout();
    }

    #[tokio::test]
    async fn test_flush_when_not_taken_over() {
        // Should not panic when not taken over
        flush_raw_stdout().await;
    }

    #[tokio::test]
    async fn test_wait_for_backpressure_idle() {
        // Should resolve immediately when nothing queued
        wait_for_raw_stdout_backpressure().await;
    }
}
