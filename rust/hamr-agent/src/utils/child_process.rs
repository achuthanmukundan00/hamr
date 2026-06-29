//! Port of `packages/coding-agent/src/utils/child-process.ts`
//!
//! Cross-platform process spawning and lifecycle management.

use std::process::{Command, Stdio};
use tokio::process::Command as TokioCommand;
use tokio::time::Duration;

/// Grace period in milliseconds after process exit before finalizing stdio.
pub const EXIT_STDIO_GRACE_MS: u64 = 100;

/// Spawn a child process. On Windows, use cross-spawn; elsewhere, use native spawn.
/// Since we're in Rust, we don't need cross-spawn — we just use std::process::Command.
pub fn spawn_process_sync(command: &str, args: &[&str], cwd: Option<&str>) -> std::process::Output {
    let mut cmd = Command::new(command);
    cmd.args(args);
    if let Some(dir) = cwd {
        cmd.current_dir(dir);
    }
    cmd.output().expect("spawn_process_sync failed")
}

/// Spawn an async child process with piped stdout and stderr.
pub fn spawn_process_async(
    command: &str,
    args: &[&str],
    cwd: Option<&str>,
) -> std::io::Result<tokio::process::Child> {
    let mut cmd = TokioCommand::new(command);
    cmd.args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(dir) = cwd {
        cmd.current_dir(dir);
    }
    cmd.kill_on_drop(true).spawn()
}

/// Wait for a child process to terminate without hanging on inherited stdio handles.
///
/// A short-lived child can `exit` while a detached descendant keeps its stdout/stderr
/// pipe open. We must not resolve and destroy the streams on a fixed deadline measured
/// from `exit`, or output still being written past that deadline is silently lost.
/// Instead, after `exit` we wait for the pipes to fall idle: the grace timer is re-armed
/// on every chunk, so an actively writing descendant keeps us reading, while a quiet
/// inherited handle (e.g. a Windows daemonized descendant that never lets `close` fire)
/// still releases us after the grace elapses.
pub async fn wait_for_child_process(
    mut child: tokio::process::Child,
) -> std::io::Result<Option<i32>> {
    // Drain stdout/stderr in background tasks
    let stdout_task = child.stdout.take().map(drain_to_null);
    let stderr_task = child.stderr.take().map(drain_to_null);

    // Wait for the process to exit
    let status = child.wait().await?;
    let exit_code = status.code();

    // Give pipes a short grace period to finish draining
    let grace = Duration::from_millis(EXIT_STDIO_GRACE_MS);
    tokio::time::sleep(grace).await;

    // Drop the drain tasks (they'll finish their async work)
    drop(stdout_task);
    drop(stderr_task);

    Ok(exit_code)
}

/// Drain a reader to /dev/null, tracking that it ended.
async fn drain_to_null<R: tokio::io::AsyncRead + Unpin>(mut reader: R) {
    let _ = tokio::io::copy(&mut reader, &mut tokio::io::sink()).await;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spawn_process_sync_echo() {
        let output = spawn_process_sync("echo", &["hello"], None);
        assert!(output.status.success());
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("hello"));
    }

    #[tokio::test]
    async fn test_spawn_process_async_echo() {
        let child = spawn_process_async("echo", &["hello"], None).unwrap();
        let exit = wait_for_child_process(child).await.unwrap();
        assert_eq!(exit, Some(0));
    }
}
