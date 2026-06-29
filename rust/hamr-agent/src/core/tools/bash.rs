//! Bash tool — spawns a shell, streams output, truncates, times out, aborts.
//!
//! Port of `packages/coding-agent/src/core/tools/bash.ts` and
//! `packages/coding-agent/src/core/bash-executor.ts`.
#![allow(dead_code)]
#![allow(unused_assignments)]

use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;

use tokio::io::{AsyncReadExt, BufReader};
use tokio::process::Command as TokioCommand;
use tokio::sync::watch;
use tokio::time::Duration;

use super::truncate::{
    DEFAULT_MAX_BYTES, DEFAULT_MAX_LINES, TruncationOptions, TruncationResult, format_size,
    truncate_tail,
};

// ---------------------------------------------------------------------------
// ANSI stripping — port of packages/coding-agent/src/utils/ansi.ts
// ---------------------------------------------------------------------------

/// Strip ANSI escape sequences from a string.
/// Matches CSI (ESC [ …) and OSC (ESC ] … ST) sequences.
pub fn strip_ansi(value: &str) -> String {
    // Fast path: no ESC (U+001B) or CSI 8-bit equivalent (U+009B)
    if !value.contains('\u{001B}') && !value.contains('\u{009B}') {
        return value.to_string();
    }

    // Build the regex lazily so we only pay the compilation cost when needed.
    // Pattern matches:
    //   OSC  : ESC ] … (ST = BEL | ESC \ | 0x9c)
    //   CSI  : ESC/C1, optional intermediates, optional params, final byte
    static RE: std::sync::LazyLock<regex::Regex> = std::sync::LazyLock::new(|| {
        let st = r"(?:\x07|\x1B\\|\x9C)";
        let osc = format!(r"\x1B\][\s\S]*?{st}");
        let csi = r"[\x1B\x9B][\[\]()#;?]*(?:\d{1,4}(?:[;:]\d{0,4})*)?[\dA-PR-TZcf-nq-uy=><~]";
        let pattern = format!("{osc}|{csi}");
        regex::Regex::new(&pattern).expect("valid ansi regex")
    });

    RE.replace_all(value, "").to_string()
}

// ---------------------------------------------------------------------------
// Binary output sanitization — port of packages/coding-agent/src/utils/shell.ts
// ---------------------------------------------------------------------------

/// Remove control characters (except tab, newline, carriage return),
/// lone surrogates, Unicode format characters, and undefined code points.
pub fn sanitize_binary_output(s: &str) -> String {
    s.chars()
        .filter(|&c| {
            let code = c as u32;

            // Allow tab, newline, carriage return
            if code == 0x09 || code == 0x0A || code == 0x0D {
                return true;
            }

            // Filter out control characters (0x00-0x1F, except above)
            if code <= 0x1F {
                return false;
            }

            // Filter out Unicode format characters (U+FFF9..U+FFFB)
            if (0xFFF9..=0xFFFB).contains(&code) {
                return false;
            }

            true
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Output accumulator — inline port of the essential logic from
// packages/coding-agent/src/core/tools/output-accumulator.ts
// ---------------------------------------------------------------------------

/// Accumulates streaming output, maintains a temp file spill for large output,
/// and provides truncation snapshots.
struct OutputAccumulator {
    /// Complete output as string (owned).
    full: String,
    /// Rolling buffer of recent output chunks for truncated display.
    recent: VecDeque<String>,
    /// Total byte count in `recent`.
    recent_bytes: usize,
    /// Whether accumulation has finished (no more data expected).
    finished: bool,
    /// Path to a temp file if output exceeds truncation threshold.
    temp_file_path: Option<PathBuf>,
    /// Handle to the temp file (kept open for writing during accumulation).
    /// Writes happen synchronously from `append`, so this is a blocking handle.
    temp_file: Option<std::fs::File>,
    /// Total bytes written (used for truncation decisions).
    total_bytes: usize,
    /// Bytes in the last line (for the partial-line truncation edge case).
    last_line_bytes: usize,
}

impl OutputAccumulator {
    const MAX_RECENT_BYTES: usize = DEFAULT_MAX_BYTES * 2;

    fn new() -> Self {
        Self {
            full: String::new(),
            recent: VecDeque::new(),
            recent_bytes: 0,
            finished: false,
            temp_file_path: None,
            temp_file: None,
            total_bytes: 0,
            last_line_bytes: 0,
        }
    }

    /// Append a chunk of output data.
    fn append(&mut self, data: &str) {
        let text = sanitize_binary_output(&strip_ansi(data)).replace('\r', "");

        self.total_bytes += text.len();
        self.full.push_str(&text);

        // Track last line byte count (reset on newline)
        for ch in text.chars() {
            if ch == '\n' {
                self.last_line_bytes = 0;
            } else {
                self.last_line_bytes += ch.len_utf8();
            }
        }

        // Ensure temp file exists if exceeding threshold
        if self.total_bytes > DEFAULT_MAX_BYTES && self.temp_file_path.is_none() {
            self.ensure_temp_file();
        }

        // Write to temp file if open
        if let Some(ref mut f) = self.temp_file {
            use std::io::Write;
            let _ = f.write_all(text.as_bytes());
        }

        // Maintain rolling buffer for truncation
        let text_len = text.len();
        self.recent.push_back(text);
        self.recent_bytes += text_len;
        while self.recent_bytes > Self::MAX_RECENT_BYTES && self.recent.len() > 1 {
            if let Some(removed) = self.recent.pop_front() {
                self.recent_bytes -= removed.len();
            }
        }
    }

    fn get_last_line_bytes(&self) -> usize {
        self.last_line_bytes
    }

    /// Mark accumulation finished (no more data).
    fn finish(&mut self) {
        self.finished = true;
    }

    /// Take a snapshot of the accumulated output, truncating if needed.
    fn snapshot(&self, persist_if_truncated: bool) -> OutputSnapshot {
        let recent_contiguous: String = self.recent.iter().map(String::as_str).collect();
        let truncation = truncate_tail(
            &recent_contiguous,
            TruncationOptions {
                max_lines: Some(DEFAULT_MAX_LINES),
                max_bytes: Some(DEFAULT_MAX_BYTES),
            },
        );

        OutputSnapshot {
            content: truncation.content.clone(),
            truncation,
            full_output_path: self.temp_file_path.clone(),
            _persist_if_truncated: persist_if_truncated,
        }
    }

    fn ensure_temp_file(&mut self) {
        // Create temp file
        let dir = std::env::temp_dir();
        let path = dir.join(format!("hamr-bash-{}.log", uuid::Uuid::new_v4().simple()));
        // Write accumulated full output to the temp file
        match std::fs::File::create(&path) {
            Ok(mut f) => {
                use std::io::Write;
                let _ = f.write_all(self.full.as_bytes());
                // Set restrictive permissions (0o600) — bash output may contain secrets
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    let _ = f.set_permissions(std::fs::Permissions::from_mode(0o600));
                }
                self.temp_file = Some(f);
                self.temp_file_path = Some(path);
            }
            Err(_) => {
                // Temp file creation failed — silently ignore; output just won't be persisted
            }
        }
    }

    /// Close the temp file if it was opened.
    async fn close_temp_file(&mut self) {
        self.temp_file = None;
    }
}

// ---------------------------------------------------------------------------
// Snapshot types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
struct OutputSnapshot {
    content: String,
    truncation: TruncationResult,
    full_output_path: Option<PathBuf>,
    _persist_if_truncated: bool,
}

// ---------------------------------------------------------------------------
// Bash tool types
// ---------------------------------------------------------------------------

/// Input schema for the bash tool (mirrors TypeBox schema).
#[derive(Debug, Clone, serde::Deserialize, schemars::JsonSchema)]
pub struct BashToolInput {
    /// Bash command to execute.
    pub command: String,
    /// Optional timeout in seconds (no default).
    #[serde(default)]
    pub timeout: Option<u64>,
}

/// Extra details forwarded alongside tool results.
#[derive(Debug, Clone)]
pub struct BashToolDetails {
    pub truncation: Option<TruncationResult>,
    pub full_output_path: Option<String>,
}

/// Asynchronous execution context that the bash tool delegates to.
/// This lets extensions intercept `user_bash` and still reuse the local
/// shell backend while wrapping or rewriting commands.
pub trait BashOperations: Send + Sync {
    /// Execute a command in `cwd` and stream output chunks to `on_data`.
    /// Returns the process exit code, or `None` if the process was killed.
    fn exec<'a>(
        &self,
        command: &str,
        cwd: &Path,
        on_data: &'a mut (dyn FnMut(&str) + Send),
        abort_rx: Option<watch::Receiver<bool>>,
        timeout_secs: Option<u64>,
        env: Option<std::collections::HashMap<String, String>>,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Option<i32>, BashExecError>> + Send + 'a>,
    >;
}

/// The rustc output limit for type names forces us to spell this out.
type BashExecFuture<'a> = std::pin::Pin<
    Box<dyn std::future::Future<Output = Result<Option<i32>, BashExecError>> + Send + 'a>,
>;

/// Errors that can occur during bash execution.
#[derive(Debug, thiserror::Error)]
pub enum BashExecError {
    #[error("Working directory does not exist: {0}\nCannot execute bash commands.")]
    CwdDoesNotExist(PathBuf),
    #[error("aborted")]
    Aborted,
    #[error("timeout:{0}")]
    Timeout(u64),
    #[error("{0}")]
    Other(String),
}

// ---------------------------------------------------------------------------
// Local bash operations
// ---------------------------------------------------------------------------

/// Resolve shell path and arguments (mirrors `getShellConfig` from shell.ts).
fn resolve_shell(shell_path: Option<&str>) -> Result<(String, Vec<String>), BashExecError> {
    if let Some(custom) = shell_path {
        let p = Path::new(custom);
        if p.try_exists().unwrap_or(false) {
            return Ok((custom.to_string(), vec!["-c".to_string()]));
        }
        return Err(BashExecError::Other(format!(
            "Custom shell path not found: {custom}"
        )));
    }

    // Unix: try /bin/bash, then sh
    if Path::new("/bin/bash").try_exists().unwrap_or(false) {
        return Ok(("/bin/bash".to_string(), vec!["-c".to_string()]));
    }

    // Fallback to sh
    Ok(("/bin/sh".to_string(), vec!["-c".to_string()]))
}

/// Create standard local bash operations that spawn an actual shell process.
/// Mirrors `createLocalBashOperations()` from TS.
pub fn create_local_bash_operations(shell_path: Option<String>) -> Arc<dyn BashOperations> {
    struct LocalBashOps {
        shell_path: Option<String>,
    }

    impl BashOperations for LocalBashOps {
        fn exec<'a>(
            &self,
            command: &str,
            cwd: &Path,
            on_data: &'a mut (dyn FnMut(&str) + Send),
            abort_rx: Option<watch::Receiver<bool>>,
            timeout_secs: Option<u64>,
            env: Option<std::collections::HashMap<String, String>>,
        ) -> BashExecFuture<'a> {
            let command = command.to_string();
            let cwd = cwd.to_path_buf();
            let shell_path = self.shell_path.clone();
            Box::pin(async move {
                let (shell, args) = resolve_shell(shell_path.as_deref())?;

                // Verify CWD exists
                if !tokio::fs::try_exists(&cwd).await.unwrap_or(false) {
                    return Err(BashExecError::CwdDoesNotExist(cwd));
                }

                // Check abort signal before spawning
                if let Some(ref rx) = abort_rx {
                    if *rx.borrow() {
                        return Err(BashExecError::Aborted);
                    }
                }

                let mut cmd = TokioCommand::new(&shell);
                cmd.args(&args).arg(&command);
                cmd.current_dir(&cwd);
                cmd.stdin(Stdio::null());
                cmd.stdout(Stdio::piped());
                cmd.stderr(Stdio::piped());

                // Merge env if provided
                if let Some(ref env_map) = env {
                    for (k, v) in env_map {
                        cmd.env(k, v);
                    }
                }

                let mut child = cmd
                    .spawn()
                    .map_err(|e| BashExecError::Other(format!("Failed to spawn shell: {e}")))?;

                let pid = child.id().expect("spawned process should have pid");

                // Collect output from stdout + stderr
                let stdout = child.stdout.take().expect("stdout piped");
                let stderr = child.stderr.take().expect("stderr piped");
                let (mut stdout_reader, mut stderr_reader) =
                    (BufReader::new(stdout), BufReader::new(stderr));

                // Read stdout/stderr and wait for the child concurrently in this
                // task. `on_data` is a borrowed `&mut FnMut`, so it cannot be
                // moved into a spawned task — we drive everything inline with a
                // `select!` loop using independent read buffers.
                let mut abort_rx = abort_rx;
                let mut buf_out = vec![0u8; 8192];
                let mut buf_err = vec![0u8; 8192];
                let mut stdout_done = false;
                let mut stderr_done = false;
                let mut exit_result: Option<std::io::Result<std::process::ExitStatus>> = None;

                // Timeout future: pending forever when no (or zero) timeout is set.
                let timeout_fut = async move {
                    match timeout_secs {
                        Some(0) | None => std::future::pending::<()>().await,
                        Some(secs) => tokio::time::sleep(Duration::from_secs(secs)).await,
                    }
                };
                tokio::pin!(timeout_fut);

                loop {
                    if exit_result.is_some() && stdout_done && stderr_done {
                        break;
                    }

                    tokio::select! {
                        r = stdout_reader.read(&mut buf_out), if !stdout_done => {
                            match r {
                                Ok(0) | Err(_) => stdout_done = true,
                                Ok(n) => {
                                    let chunk = String::from_utf8_lossy(&buf_out[..n]);
                                    on_data(&chunk);
                                }
                            }
                        }
                        r = stderr_reader.read(&mut buf_err), if !stderr_done => {
                            match r {
                                Ok(0) | Err(_) => stderr_done = true,
                                Ok(n) => {
                                    let chunk = String::from_utf8_lossy(&buf_err[..n]);
                                    on_data(&chunk);
                                }
                            }
                        }
                        status = child.wait(), if exit_result.is_none() => {
                            exit_result = Some(status);
                        }
                        _ = &mut timeout_fut, if exit_result.is_none() => {
                            let _ = kill_process(pid);
                            return Err(BashExecError::Timeout(timeout_secs.unwrap_or(0)));
                        }
                        _ = wait_for_abort(&mut abort_rx), if exit_result.is_none() => {
                            let _ = kill_process(pid);
                            return Err(BashExecError::Aborted);
                        }
                    }
                }

                // Final abort check: the signal may have flipped just as the
                // child exited, before the abort branch could fire.
                let aborted = abort_rx.as_ref().map(|rx| *rx.borrow()).unwrap_or(false);
                if aborted {
                    let _ = kill_process(pid);
                    return Err(BashExecError::Aborted);
                }

                let exit_result = exit_result.expect("loop only breaks after child exit");
                Ok(exit_result.map(|s| s.code()).unwrap_or(None))
            })
        }
    }

    Arc::new(LocalBashOps { shell_path })
}

/// Resolve once the abort signal is (or becomes) `true`. When there is no
/// receiver, or the sender has been dropped, this stays pending forever so the
/// enclosing `select!` branch never fires spuriously.
async fn wait_for_abort(rx: &mut Option<watch::Receiver<bool>>) {
    match rx {
        Some(rx) => loop {
            if *rx.borrow() {
                return;
            }
            if rx.changed().await.is_err() {
                std::future::pending::<()>().await;
            }
        },
        None => std::future::pending::<()>().await,
    }
}

#[cfg(not(windows))]
fn kill_process(pid: u32) -> std::io::Result<()> {
    // Kill process group with SIGKILL
    unsafe {
        libc::kill(-(pid as i32), libc::SIGKILL);
    }
    Ok(())
}

#[cfg(windows)]
fn kill_process(pid: u32) -> std::io::Result<()> {
    let _ = std::process::Command::new("taskkill")
        .args(["/F", "/T", "/PID", &pid.to_string()])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn();
    Ok(())
}

// ---------------------------------------------------------------------------
// Bash tool
// ---------------------------------------------------------------------------

/// The bash tool itself — holds configuration and the execution backend.
pub struct BashTool {
    cwd: PathBuf,
    operations: Arc<dyn BashOperations>,
    command_prefix: Option<String>,
    shell_path: Option<String>,
}

/// Options for creating a bash tool.
pub struct BashToolOptions {
    pub operations: Option<Arc<dyn BashOperations>>,
    pub command_prefix: Option<String>,
    pub shell_path: Option<String>,
}

/// Create a bash tool bound to a working directory.
pub fn create_bash_tool(cwd: &Path, options: Option<BashToolOptions>) -> BashTool {
    let opts = options.unwrap_or(BashToolOptions {
        operations: None,
        command_prefix: None,
        shell_path: None,
    });

    let operations = opts
        .operations
        .unwrap_or_else(|| create_local_bash_operations(opts.shell_path.clone()));

    BashTool {
        cwd: cwd.to_path_buf(),
        operations,
        command_prefix: opts.command_prefix,
        shell_path: opts.shell_path,
    }
}

/// Full result of a bash execution.
#[derive(Debug)]
pub struct BashResult {
    pub content: String,
    pub details: Option<BashToolDetails>,
    pub exit_code: Option<i32>,
}

/// Streaming update sent during execution (for live TUI preview).
#[derive(Debug, Clone)]
pub struct BashStreamUpdate {
    pub content: String,
    pub details: Option<BashToolDetails>,
}

impl BashTool {
    /// Execute a bash command. If `on_update` is provided, streaming updates
    /// are throttled (~100ms) and delivered through it.
    ///
    /// `abort_rx` is checked periodically; if it fires, the process is killed
    /// and the method returns an error.
    pub async fn execute(
        &self,
        input: BashToolInput,
        abort_rx: Option<watch::Receiver<bool>>,
        on_update: Option<&(dyn Fn(BashStreamUpdate) + Send + Sync)>,
    ) -> Result<BashResult, BashExecError> {
        let BashToolInput { command, timeout } = input;

        // Prepend command prefix if configured
        let resolved_command = if let Some(ref prefix) = self.command_prefix {
            format!("{prefix}\n{command}")
        } else {
            command
        };

        // Set up output accumulator
        let mut output = OutputAccumulator::new();
        let update_interval_ms = 100;
        let mut last_update = tokio::time::Instant::now();
        let mut update_dirty = false;

        // Closure that emits the latest snapshot to the callback
        let emit_update =
            |acc: &OutputAccumulator, on_upd: Option<&(dyn Fn(BashStreamUpdate) + Send + Sync)>| {
                if let Some(cb) = on_upd {
                    let snap = acc.snapshot(true);
                    cb(BashStreamUpdate {
                        content: snap.content.clone(),
                        details: Some(BashToolDetails {
                            truncation: if snap.truncation.truncated {
                                Some(snap.truncation)
                            } else {
                                None
                            },
                            full_output_path: snap
                                .full_output_path
                                .as_ref()
                                .map(|p| p.to_string_lossy().to_string()),
                        }),
                    });
                }
            };

        // Send the initial empty update before `on_data` borrows the accumulator.
        if on_update.is_some() {
            emit_update(&output, on_update);
        }

        // Drive execution: `on_data` appends to the accumulator and emits
        // throttled (~100ms) streaming updates. It is scoped in its own block so
        // its mutable borrow of `output` ends before we take the final snapshot.
        let exit_code = {
            let mut on_data = |chunk: &str| {
                output.append(chunk);
                update_dirty = true;

                // Throttle updates to ~100ms.
                if on_update.is_some() {
                    let elapsed = last_update.elapsed();
                    if elapsed >= Duration::from_millis(update_interval_ms) {
                        emit_update(&output, on_update);
                        last_update = tokio::time::Instant::now();
                        update_dirty = false;
                    }
                }
            };

            self.operations
                .exec(
                    &resolved_command,
                    &self.cwd,
                    &mut on_data,
                    abort_rx,
                    timeout,
                    None,
                )
                .await?
        };

        // Finish accumulation and flush any remaining dirty update.
        output.finish();
        if update_dirty {
            emit_update(&output, on_update);
            update_dirty = false;
        }
        let snapshot = output.snapshot(true);
        let _ = output.close_temp_file().await;

        // Format the result
        let truncation = &snapshot.truncation;
        let mut text = if truncation.truncated {
            snapshot.content.clone()
        } else {
            snapshot.content.clone()
        };

        let empty_text = "(no output)";
        if text.is_empty() {
            text = empty_text.to_string();
        }

        let mut details: Option<BashToolDetails> = None;
        if truncation.truncated {
            let full_output_path = snapshot
                .full_output_path
                .as_ref()
                .map(|p| p.to_string_lossy().to_string());

            let start_line = truncation
                .total_lines
                .saturating_sub(truncation.output_lines)
                + 1;
            let end_line = truncation.total_lines;

            if truncation.last_line_partial {
                let last_line_size = format_size(output.get_last_line_bytes());
                text = format!(
                    "{text}\n\n[Showing last {} of line {end_line} (line is {last_line_size}). Full output: {full_path}]",
                    format_size(truncation.output_bytes),
                    full_path = full_output_path.as_deref().unwrap_or(""),
                );
            } else if matches!(
                truncation.truncated_by,
                Some(super::truncate::TruncationLimit::Lines)
            ) {
                text = format!(
                    "{text}\n\n[Showing lines {start_line}-{end_line} of {total}. Full output: {full_path}]",
                    total = truncation.total_lines,
                    full_path = full_output_path.as_deref().unwrap_or(""),
                );
            } else {
                text = format!(
                    "{text}\n\n[Showing lines {start_line}-{end_line} of {total} ({limit} limit). Full output: {full_path}]",
                    total = truncation.total_lines,
                    limit = format_size(DEFAULT_MAX_BYTES),
                    full_path = full_output_path.as_deref().unwrap_or(""),
                );
            }

            details = Some(BashToolDetails {
                truncation: Some(truncation.clone()),
                full_output_path,
            });
        }

        // Check exit code
        if let Some(code) = exit_code {
            if code != 0 {
                let append_status = |t: &str, status: &str| -> String {
                    if t.is_empty() || t == empty_text {
                        status.to_string()
                    } else {
                        format!("{t}\n\n{status}")
                    }
                };
                return Err(BashExecError::Other(append_status(
                    &text,
                    &format!("Command exited with code {code}"),
                )));
            }
        }

        Ok(BashResult {
            content: text,
            details,
            exit_code,
        })
    }
}

// ---------------------------------------------------------------------------
// execute_bash_with_operations — standalone execution for non-tool callers
// Port of packages/coding-agent/src/core/bash-executor.ts
// ---------------------------------------------------------------------------

/// Result of executing a bash command outside the tool framework.
pub struct ExecuteBashResult {
    pub output: String,
    pub exit_code: Option<i32>,
    pub cancelled: bool,
    pub truncated: bool,
    pub full_output_path: Option<String>,
}

/// Execute a bash command using custom `BashOperations`, with support for
/// abort signals and streaming chunks.
pub async fn execute_bash_with_operations(
    command: &str,
    cwd: &Path,
    operations: Arc<dyn BashOperations>,
    abort_rx: Option<watch::Receiver<bool>>,
    on_chunk: Option<&(dyn Fn(&str) + Send + Sync)>,
) -> Result<ExecuteBashResult, BashExecError> {
    let mut output = OutputAccumulator::new();

    let acc_ref = &mut output;
    let mut on_data = |chunk: &str| {
        acc_ref.append(chunk);
        if let Some(ref cb) = on_chunk {
            cb(chunk);
        }
    };

    let result = operations
        .exec(command, cwd, &mut on_data, abort_rx, None, None)
        .await;

    let cancelled = if let Err(BashExecError::Aborted) = &result {
        true
    } else {
        false
    };

    match result {
        Ok(exit_code) => {
            output.finish();
            let snapshot = output.snapshot(true);
            let _ = output.close_temp_file().await;

            Ok(ExecuteBashResult {
                output: if snapshot.truncation.truncated {
                    snapshot.truncation.content.clone()
                } else {
                    snapshot.content.clone()
                },
                exit_code: if cancelled { None } else { exit_code },
                cancelled,
                truncated: snapshot.truncation.truncated,
                full_output_path: snapshot
                    .full_output_path
                    .map(|p| p.to_string_lossy().to_string()),
            })
        }
        Err(BashExecError::Aborted) => {
            output.finish();
            let snapshot = output.snapshot(true);
            let _ = output.close_temp_file().await;

            Ok(ExecuteBashResult {
                output: if snapshot.truncation.truncated {
                    snapshot.truncation.content.clone()
                } else {
                    snapshot.content.clone()
                },
                exit_code: None,
                cancelled: true,
                truncated: snapshot.truncation.truncated,
                full_output_path: snapshot
                    .full_output_path
                    .map(|p| p.to_string_lossy().to_string()),
            })
        }
        Err(e) => {
            let _ = output.close_temp_file().await;
            Err(e)
        }
    }
}
