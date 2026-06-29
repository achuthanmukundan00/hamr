//! Bash command execution with streaming output support and cancellation.
//!
//! Port of `packages/coding-agent/src/core/bash-executor.ts`.
//!
//! This module provides a unified bash execution implementation used by:
//! - AgentSession.executeBash() for interactive and RPC modes
//! - Direct calls from modes that need bash execution

use crate::core::tools::truncate::{self, DEFAULT_MAX_BYTES, truncate_tail};
use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use tokio::sync::watch;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// Options for bash execution.
pub struct BashExecutorOptions {
    /// Callback for streaming output chunks (already sanitized).
    pub on_chunk: Option<OnChunkFn>,
    /// Abort signal for cancellation.
    pub signal: Option<watch::Receiver<bool>>,
}

/// Type for the streaming chunk callback.
pub type OnChunkFn = Box<dyn Fn(String) -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + Sync>;

/// Result of a bash execution.
#[derive(Debug, Clone)]
pub struct BashResult {
    /// Combined stdout + stderr output (sanitized, possibly truncated).
    pub output: String,
    /// Process exit code (None if killed/cancelled).
    pub exit_code: Option<i32>,
    /// Whether the command was cancelled via signal.
    pub cancelled: bool,
    /// Whether the output was truncated.
    pub truncated: bool,
    /// Path to temp file containing full output (if output exceeded truncation threshold).
    pub full_output_path: Option<PathBuf>,
}

// ---------------------------------------------------------------------------
// Trait: BashOperations (extension point for remote execution)
// ---------------------------------------------------------------------------

/// Options passed to [`BashOperations::exec`].
pub struct BashExecOptions<'a> {
    pub on_data: &'a (dyn Fn(&[u8]) + Send + Sync),
    pub signal: Option<&'a watch::Receiver<bool>>,
}

/// Outcome from [`BashOperations::exec`].
#[derive(Debug, Clone)]
pub struct BashExecOutcome {
    pub exit_code: Option<i32>,
}

/// Abstracts the running of a bash command.
///
/// Used to support both local execution and remote execution
/// (SSH, containers, etc.).
pub trait BashOperations: Send + Sync {
    fn exec(
        &self,
        command: &str,
        cwd: &str,
        options: BashExecOptions<'_>,
    ) -> Pin<Box<dyn Future<Output = std::io::Result<BashExecOutcome>> + Send + '_>>;
}

// ---------------------------------------------------------------------------
// Implementation
// ---------------------------------------------------------------------------

/// Maximum in-memory output buffer (double the truncation threshold).
const MAX_OUTPUT_BUFFER_BYTES: usize = DEFAULT_MAX_BYTES * 2;

/// Execute a bash command using custom [`BashOperations`].
///
/// Used for remote execution (SSH, containers, etc.).
pub async fn execute_bash_with_operations(
    command: &str,
    cwd: &str,
    operations: &dyn BashOperations,
    options: Option<BashExecutorOptions>,
) -> std::io::Result<BashResult> {
    use std::sync::{Arc, Mutex};
    use tokio::sync::mpsc;

    let (on_chunk_arc, signal) = match options {
        Some(o) => {
            let cb = o.on_chunk.map(|f| {
                Arc::new(f)
                    as Arc<dyn Fn(String) -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + Sync>
            });
            (cb, o.signal)
        }
        None => (None, None),
    };

    // Channel to bridge sync on_data callback → async processing
    let (tx, mut rx) = mpsc::unbounded_channel::<Vec<u8>>();

    let on_data: Arc<dyn Fn(&[u8]) + Send + Sync> = Arc::new(move |data: &[u8]| {
        let _ = tx.send(data.to_vec());
    });

    let signal_ref = signal.as_ref();
    let exec_options = BashExecOptions {
        on_data: &*on_data,
        signal: signal_ref,
    };

    // Shared state
    let output_chunks = Arc::new(Mutex::new(Vec::<String>::new()));
    let output_bytes = Arc::new(Mutex::new(0usize));
    let total_bytes = Arc::new(Mutex::new(0usize));
    let temp_file_path = Arc::new(Mutex::new(None::<PathBuf>));

    let chunks_ref = output_chunks.clone();
    let bytes_ref = output_bytes.clone();
    let total_ref = total_bytes.clone();
    let path_ref = temp_file_path.clone();

    // Spawn execution
    let mut exec_handle = Box::pin(operations.exec(command, cwd, exec_options));

    // Process output chunks as they arrive
    loop {
        tokio::select! {
            maybe_data = rx.recv() => {
                match maybe_data {
                    Some(data) => {
                        let text = sanitize_bash_output(
                            &String::from_utf8_lossy(&data),
                        );

                        if let Some(ref cb) = on_chunk_arc {
                            let text_clone = text.clone();
                            let cb = Arc::clone(cb);
                            tokio::spawn(async move {
                                cb(text_clone).await;
                            });
                        }

                        {
                            let mut chunks = chunks_ref.lock().unwrap();
                            let mut ob = bytes_ref.lock().unwrap();
                            let mut tb = total_ref.lock().unwrap();
                            *tb += data.len();
                            chunks.push(text.clone());
                            *ob += text.len();
                            while *ob > MAX_OUTPUT_BUFFER_BYTES && chunks.len() > 1 {
                                let removed = chunks.remove(0);
                                *ob -= removed.len();
                            }

                            // Create temp file if total exceeds threshold
                            if *tb > DEFAULT_MAX_BYTES {
                                let mut path = path_ref.lock().unwrap();
                                if path.is_none() {
                                    if let Ok(p) = create_temp_file(&chunks) {
                                        *path = Some(p);
                                    }
                                }
                            }
                        }
                    }
                    None => break,
                }
            }
            result = &mut exec_handle => {
                return match result {
                    Ok(outcome) => {
                        let full_output: String = {
                            let chunks = output_chunks.lock().unwrap();
                            chunks.join("")
                        };
                        let truncation = truncate_tail(&full_output, truncate::TruncationOptions::default());

                        let cancelled = signal_ref
                            .map(|s| *s.borrow())
                            .unwrap_or(false);

                        Ok(BashResult {
                            output: if truncation.truncated {
                                truncation.content
                            } else {
                                full_output
                            },
                            exit_code: if cancelled { None } else { outcome.exit_code },
                            cancelled,
                            truncated: truncation.truncated,
                            full_output_path: temp_file_path.lock().unwrap().clone(),
                        })
                    }
                    Err(e) => Err(e),
                };
            }
        }
    }

    // If rx channel closed before execution finished, still wait for execution
    match exec_handle.await {
        Ok(outcome) => {
            let full_output: String = {
                let chunks = output_chunks.lock().unwrap();
                chunks.join("")
            };
            let truncation = truncate_tail(&full_output, truncate::TruncationOptions::default());

            let cancelled = signal_ref.map(|s| *s.borrow()).unwrap_or(false);

            Ok(BashResult {
                output: if truncation.truncated {
                    truncation.content
                } else {
                    full_output
                },
                exit_code: if cancelled { None } else { outcome.exit_code },
                cancelled,
                truncated: truncation.truncated,
                full_output_path: temp_file_path.lock().unwrap().clone(),
            })
        }
        Err(e) => {
            // Clean up temp file on error to avoid orphaned files.
            if let Some(path) = temp_file_path.lock().unwrap().take() {
                let _ = tokio::fs::remove_file(&path).await;
            }
            Err(e)
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn create_temp_file(chunks: &[String]) -> std::io::Result<PathBuf> {
    use std::fs::OpenOptions;
    use std::io::Write;

    let id = uuid::Uuid::new_v4().to_string();
    let path = std::env::temp_dir().join(format!("hamr-bash-{id}.log"));
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&path)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = file.metadata()?.permissions();
        perms.set_mode(0o600);
        file.set_permissions(perms)?;
    }

    for chunk in chunks {
        file.write_all(chunk.as_bytes())?;
    }
    file.flush()?;

    Ok(path)
}

/// Sanitize bash output: strip ANSI escape codes, normalize newlines.
fn sanitize_bash_output(raw: &str) -> String {
    let re = regex::Regex::new(r"\x1b\[[0-?]*[ -/]*[@-~]").unwrap();
    let cleaned = re.replace_all(raw, "");
    cleaned.replace('\r', "")
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_ansi_sequences() {
        let input = "\u{1b}[32mgreen\u{1b}[0m text";
        let output = sanitize_bash_output(input);
        assert!(!output.contains("\u{1b}["));
        assert!(output.contains("green"));
        assert!(output.contains("text"));
    }

    #[test]
    fn test_sanitize_carriage_returns() {
        let input = "line1\r\nline2\r\nline3\r";
        let output = sanitize_bash_output(input);
        assert!(!output.contains('\r'));
        assert_eq!(output, "line1\nline2\nline3");
    }

    #[test]
    fn test_sanitize_preserves_normal_text() {
        let input = "hello world";
        let output = sanitize_bash_output(input);
        assert_eq!(output, "hello world");
    }
}
