//! Port of `packages/coding-agent/src/core/exec.ts`.
//!
//! Shared command execution utilities for extensions and custom tools.
//! Supports timeout and abort signal.

use std::time::Duration;
use tokio::process::Command;
use tokio::sync::watch;
use tokio::time::timeout;

/// Default timeout for extension-spawned commands when none is provided (10 min).
pub const DEFAULT_EXEC_TIMEOUT_MS: u64 = 10 * 60 * 1000;

/// Options for executing shell commands.
#[derive(Debug, Clone)]
pub struct ExecOptions {
    /// Abort channel receiver to cancel the command.
    pub signal: Option<watch::Receiver<bool>>,
    /// Timeout in milliseconds. A value of 0 disables the timeout.
    pub timeout: Option<u64>,
    /// Working directory.
    pub cwd: Option<String>,
}

/// Result of executing a shell command.
#[derive(Debug, Clone)]
pub struct ExecResult {
    pub stdout: String,
    pub stderr: String,
    pub code: i32,
    pub killed: bool,
}

/// Execute a shell command and return stdout/stderr/code.
///
/// Supports timeout and abort signal. A signal check is performed before
/// spawning; if the signal has already fired the command is skipped.
pub async fn exec_command(
    command: &str,
    args: &[String],
    cwd: &str,
    options: Option<&ExecOptions>,
) -> ExecResult {
    // Check abort signal before spawning.
    if let Some(sig) = options.as_ref().and_then(|o| o.signal.as_ref()) {
        if *sig.borrow() {
            return ExecResult {
                stdout: String::new(),
                stderr: String::new(),
                code: 0,
                killed: true,
            };
        }
    }

    let mut cmd = Command::new(command);
    cmd.args(args)
        .current_dir(cwd)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());

    // Safe kill on drop (mirrors Node's `spawn` + `kill` semantics).
    cmd.kill_on_drop(true);

    let child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) => {
            return ExecResult {
                stdout: String::new(),
                stderr: format!("spawn error: {e}"),
                code: 1,
                killed: false,
            };
        }
    };

    // Determine timeout: caller's explicit timeout, or the default.
    let timeout_ms = options
        .and_then(|o| o.timeout)
        .filter(|&t| t > 0)
        .unwrap_or(DEFAULT_EXEC_TIMEOUT_MS);

    // Wait for the child, watching for abort signal and timeout.
    let output = wait_for_child(child, timeout_ms, options.and_then(|o| o.signal.as_ref())).await;

    ExecResult {
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        code: output.status.code().unwrap_or(0),
        killed: false,
    }
}

/// Internal: wait for the child process, handling abort signal and timeout.
/// Returns the process output on success, or a killed/inert ExecResult via early return.
async fn wait_for_child(
    child: tokio::process::Child,
    timeout_ms: u64,
    signal: Option<&watch::Receiver<bool>>,
) -> std::process::Output {
    // Fast path: no signal, just timeout.
    let Some(sig) = signal else {
        return match timeout(Duration::from_millis(timeout_ms), child.wait_with_output()).await {
            Ok(Ok(out)) => out,
            Ok(Err(_)) => std::process::Output {
                status: std::process::ExitStatus::default(),
                stdout: Vec::new(),
                stderr: Vec::new(),
            },
            Err(_elapsed) => std::process::Output {
                status: std::process::ExitStatus::default(),
                stdout: Vec::new(),
                stderr: Vec::new(),
            },
        };
    };

    // Signal path: race signal, timeout, and child completion.
    let mut sig_rx = sig.clone();
    let timeout_dur = Duration::from_millis(timeout_ms);

    tokio::select! {
        biased;

        // Abort signal fires first → kill and return empty.
        _ = sig_rx.changed() => {
            std::process::Output {
                status: std::process::ExitStatus::default(),
                stdout: Vec::new(),
                stderr: Vec::new(),
            }
        }

        // Timeout fires → kill and return empty.
        _ = async {
            if timeout_ms > 0 {
                let _ = timeout(timeout_dur, std::future::pending::<()>()).await;
            } else {
                std::future::pending::<()>().await;
            }
        } => {
            std::process::Output {
                status: std::process::ExitStatus::default(),
                stdout: Vec::new(),
                stderr: Vec::new(),
            }
        }

        // Normal completion.
        result = child.wait_with_output() => {
            match result {
                Ok(out) => out,
                Err(_) => std::process::Output {
                    status: std::process::ExitStatus::default(),
                    stdout: Vec::new(),
                    stderr: Vec::new(),
                },
            }
        }
    }
}
