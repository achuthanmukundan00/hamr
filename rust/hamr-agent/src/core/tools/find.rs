//! Port of `packages/coding-agent/src/core/tools/find.ts` — the find tool.
//!
//! Searches for files by glob pattern using `fd`, returning paths relative to
//! the search directory. Respects `.gitignore`. Output is capped at a result
//! limit (default 1000) or a byte limit, whichever hits first.
//!
//! As with the other ported tools, this covers the execute logic and types;
//! TUI rendering is handled elsewhere.
//!
//! NOTE: the TypeScript original auto-downloads `fd` via `ensureTool`. Until
//! `tools-manager` is ported, this resolves `fd` (or `fdfind`) from `PATH` and
//! errors if it is unavailable.

use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::process::Stdio;
use std::sync::Arc;

use tokio::process::Command as TokioCommand;
use tokio::sync::watch;

use hamr_ai::types::{MessageContent, TextContent};

use super::path_utils::resolve_to_cwd;
use super::truncate::{
    DEFAULT_MAX_BYTES, TruncationOptions, TruncationResult, format_size, truncate_head,
};

const DEFAULT_LIMIT: usize = 1000;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// Input parameters for the find tool.
#[derive(Debug, Clone, serde::Deserialize, schemars::JsonSchema)]
pub struct FindToolInput {
    /// Glob pattern to match files, e.g. `*.ts`, `**/*.json`.
    pub pattern: String,
    /// Directory to search in (default: current directory).
    #[serde(default)]
    pub path: Option<String>,
    /// Maximum number of results (default: 1000).
    #[serde(default)]
    pub limit: Option<usize>,
}

/// Details attached to a find result.
#[derive(Debug, Clone, Default)]
pub struct FindToolDetails {
    pub truncation: Option<TruncationResult>,
    pub result_limit_reached: Option<usize>,
}

impl FindToolDetails {
    fn is_empty(&self) -> bool {
        self.truncation.is_none() && self.result_limit_reached.is_none()
    }
}

/// Result of executing the find tool.
#[derive(Debug, Clone)]
pub struct FindToolResult {
    pub content: Vec<MessageContent>,
    pub details: Option<FindToolDetails>,
}

/// Errors that can occur while finding files.
#[derive(Debug, thiserror::Error)]
pub enum FindToolError {
    #[error("Path not found: {0}")]
    PathNotFound(String),
    #[error("fd is not available on PATH (auto-download not yet ported)")]
    FdUnavailable,
    #[error("Failed to run fd: {0}")]
    SpawnFailed(String),
    #[error("{0}")]
    Fd(String),
    #[error("Operation aborted")]
    Aborted,
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

// ---------------------------------------------------------------------------
// Pluggable operations
// ---------------------------------------------------------------------------

type FindFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// Pluggable file-search operations. Override to delegate to remote systems
/// (for example SSH). When supplied, [`glob`](FindOperations::glob) is used in
/// place of the local `fd` backend.
pub trait FindOperations: Send + Sync {
    /// Whether `path` exists.
    fn exists<'a>(&'a self, path: &'a Path) -> FindFuture<'a, bool>;
    /// Find files matching `pattern` under `cwd`, honoring `ignore` globs and
    /// the result `limit`.
    fn glob<'a>(
        &'a self,
        pattern: &'a str,
        cwd: &'a Path,
        ignore: &'a [String],
        limit: usize,
    ) -> FindFuture<'a, std::io::Result<Vec<String>>>;
}

// ---------------------------------------------------------------------------
// FindTool
// ---------------------------------------------------------------------------

/// The find tool — searches for files by glob pattern.
pub struct FindTool {
    cwd: PathBuf,
    operations: Option<Arc<dyn FindOperations>>,
}

/// Options for constructing a [`FindTool`].
#[derive(Default)]
pub struct FindToolOptions {
    /// Custom operations. When `None`, the local `fd` backend is used.
    pub operations: Option<Arc<dyn FindOperations>>,
}

impl FindTool {
    /// Create a new find tool rooted at `cwd`.
    pub fn new(cwd: &Path, options: FindToolOptions) -> Self {
        Self {
            cwd: cwd.to_path_buf(),
            operations: options.operations,
        }
    }

    /// Execute the find with the given input.
    pub async fn execute(
        &self,
        input: &FindToolInput,
        abort_rx: Option<watch::Receiver<bool>>,
    ) -> Result<FindToolResult, FindToolError> {
        let mut abort_rx = abort_rx;
        if is_aborted(abort_rx.as_ref()) {
            return Err(FindToolError::Aborted);
        }

        let search_path = resolve_to_cwd(input.path.as_deref().unwrap_or("."), &self.cwd);
        let search_path_str = search_path.to_string_lossy().into_owned();
        let effective_limit = input.limit.unwrap_or(DEFAULT_LIMIT);

        // Custom operations path: use the provided glob() backend.
        if let Some(ops) = &self.operations {
            if !ops.exists(&search_path).await {
                return Err(FindToolError::PathNotFound(search_path_str));
            }
            if is_aborted(abort_rx.as_ref()) {
                return Err(FindToolError::Aborted);
            }
            let ignore = vec!["**/node_modules/**".to_string(), "**/.git/**".to_string()];
            let results = ops
                .glob(&input.pattern, &search_path, &ignore, effective_limit)
                .await?;
            if is_aborted(abort_rx.as_ref()) {
                return Err(FindToolError::Aborted);
            }
            if results.is_empty() {
                return Ok(empty_result());
            }
            let (output, details) = build_find_output(results, &search_path_str, effective_limit);
            return Ok(FindToolResult {
                content: vec![text_content(&output)],
                details,
            });
        }

        // Default backend: fd.
        let fd_path = resolve_fd().ok_or(FindToolError::FdUnavailable)?;

        // Build fd arguments. --no-require-git applies hierarchical .gitignore
        // semantics regardless of whether the search path is inside a git repo.
        let mut args: Vec<String> = vec![
            "--glob".into(),
            "--color=never".into(),
            "--hidden".into(),
            "--no-require-git".into(),
            "--max-results".into(),
            effective_limit.to_string(),
        ];

        // fd --glob matches the basename unless --full-path is set; in full-path
        // mode a path-containing pattern needs a leading '**/' to match.
        let mut effective_pattern = input.pattern.clone();
        if input.pattern.contains('/') {
            args.push("--full-path".into());
            if !input.pattern.starts_with('/')
                && !input.pattern.starts_with("**/")
                && input.pattern != "**"
            {
                effective_pattern = format!("**/{}", input.pattern);
            }
        }
        args.push("--".into());
        args.push(effective_pattern);
        args.push(search_path_str.clone());

        let child = TokioCommand::new(&fd_path)
            .args(&args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| FindToolError::SpawnFailed(e.to_string()))?;

        let pid = child.id();

        // fd output is bounded by --max-results, so reading to completion is
        // cheap. Race it against the abort signal, killing fd if aborted.
        let output = tokio::select! {
            res = child.wait_with_output() => res?,
            _ = wait_for_abort(&mut abort_rx) => {
                if let Some(pid) = pid {
                    let _ = kill_pid(pid);
                }
                return Err(FindToolError::Aborted);
            }
        };

        if is_aborted(abort_rx.as_ref()) {
            return Err(FindToolError::Aborted);
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let lines: Vec<String> = stdout
            .split('\n')
            .filter(|l| !l.is_empty())
            .map(|l| l.to_string())
            .collect();

        let code = output.status.code();
        if code != Some(0) && lines.is_empty() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let msg = stderr.trim();
            let msg = if msg.is_empty() {
                format!(
                    "fd exited with code {}",
                    code.map(|c| c.to_string())
                        .unwrap_or_else(|| "unknown".into())
                )
            } else {
                msg.to_string()
            };
            return Err(FindToolError::Fd(msg));
        }

        if lines.is_empty() {
            return Ok(empty_result());
        }

        let (output_text, details) = build_find_output(lines, &search_path_str, effective_limit);
        Ok(FindToolResult {
            content: vec![text_content(&output_text)],
            details,
        })
    }
}

/// Relativize raw fd/glob output lines against the search root, apply byte
/// truncation, and append actionable notices. Factored out for testing.
fn build_find_output(
    lines: Vec<String>,
    search_path: &str,
    effective_limit: usize,
) -> (String, Option<FindToolDetails>) {
    let mut relativized: Vec<String> = Vec::with_capacity(lines.len());
    for raw_line in &lines {
        let line = raw_line.trim_end_matches('\r').trim();
        if line.is_empty() {
            continue;
        }
        let had_trailing_slash = line.ends_with('/') || line.ends_with('\\');
        let mut relative_path = if let Some(rest) = line.strip_prefix(search_path) {
            // Drop the leading separator after the search root.
            rest.strip_prefix(['/', '\\']).unwrap_or(rest).to_string()
        } else {
            relative_path(search_path, line)
        };
        if had_trailing_slash && !relative_path.ends_with('/') {
            relative_path.push('/');
        }
        relativized.push(to_posix_path(&relative_path));
    }

    let result_limit_reached = relativized.len() >= effective_limit;
    let raw_output = relativized.join("\n");
    let truncation = truncate_head(
        &raw_output,
        TruncationOptions {
            max_lines: Some(usize::MAX),
            max_bytes: None,
        },
    );
    let mut result_output = truncation.content.clone();
    let mut details = FindToolDetails::default();
    let mut notices: Vec<String> = Vec::new();

    if result_limit_reached {
        notices.push(format!(
            "{effective_limit} results limit reached. Use limit={} for more, or refine pattern",
            effective_limit * 2
        ));
        details.result_limit_reached = Some(effective_limit);
    }
    if truncation.truncated {
        notices.push(format!("{} limit reached", format_size(DEFAULT_MAX_BYTES)));
        details.truncation = Some(truncation);
    }
    if !notices.is_empty() {
        result_output += &format!("\n\n[{}]", notices.join(". "));
    }

    let details = if details.is_empty() {
        None
    } else {
        Some(details)
    };
    (result_output, details)
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn empty_result() -> FindToolResult {
    FindToolResult {
        content: vec![text_content("No files found matching pattern")],
        details: None,
    }
}

fn text_content(text: &str) -> MessageContent {
    MessageContent::Text(TextContent {
        text: text.to_string(),
        text_signature: None,
    })
}

fn to_posix_path(value: &str) -> String {
    value.replace(std::path::MAIN_SEPARATOR, "/")
}

/// Lexical `path.relative(base, target)` for the find fallback case.
fn relative_path(base: &str, target: &str) -> String {
    let base_comps: Vec<&str> = base.split(['/', '\\']).filter(|s| !s.is_empty()).collect();
    let target_comps: Vec<&str> = target
        .split(['/', '\\'])
        .filter(|s| !s.is_empty())
        .collect();
    let mut i = 0;
    while i < base_comps.len() && i < target_comps.len() && base_comps[i] == target_comps[i] {
        i += 1;
    }
    let mut parts: Vec<String> = Vec::new();
    for _ in i..base_comps.len() {
        parts.push("..".to_string());
    }
    for c in &target_comps[i..] {
        parts.push((*c).to_string());
    }
    parts.join("/")
}

fn is_aborted(abort_rx: Option<&watch::Receiver<bool>>) -> bool {
    abort_rx.map(|rx| *rx.borrow()).unwrap_or(false)
}

/// Resolve once the abort signal becomes `true`; stays pending forever when
/// there is no receiver or the sender was dropped.
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

/// Locate `fd` (or the Debian `fdfind` alias) on `PATH`.
fn resolve_fd() -> Option<PathBuf> {
    for name in ["fd", "fdfind"] {
        if let Some(p) = which(name) {
            return Some(p);
        }
    }
    None
}

/// Minimal `which`: scan `PATH` for an executable named `name`.
fn which(name: &str) -> Option<PathBuf> {
    let path_var = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path_var) {
        let candidate = dir.join(name);
        if is_executable(&candidate) {
            return Some(candidate);
        }
    }
    None
}

#[cfg(unix)]
fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    std::fs::metadata(path)
        .map(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

#[cfg(not(unix))]
fn is_executable(path: &Path) -> bool {
    path.is_file()
}

#[cfg(unix)]
fn kill_pid(pid: u32) -> std::io::Result<()> {
    unsafe {
        libc::kill(pid as i32, libc::SIGKILL);
    }
    Ok(())
}

#[cfg(not(unix))]
fn kill_pid(pid: u32) -> std::io::Result<()> {
    let _ = std::process::Command::new("taskkill")
        .args(["/F", "/PID", &pid.to_string()])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn();
    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relativizes_against_search_root() {
        let lines = vec![
            "/repo/src/a.ts".to_string(),
            "/repo/src/sub/b.ts".to_string(),
        ];
        let (output, details) = build_find_output(lines, "/repo", 1000);
        assert_eq!(output, "src/a.ts\nsrc/sub/b.ts");
        assert!(details.is_none());
    }

    #[test]
    fn preserves_directory_trailing_slash() {
        let lines = vec!["/repo/src/".to_string()];
        let (output, _) = build_find_output(lines, "/repo", 1000);
        assert_eq!(output, "src/");
    }

    #[test]
    fn result_limit_notice() {
        let lines: Vec<String> = (0..3).map(|i| format!("/repo/f{i}.ts")).collect();
        let (output, details) = build_find_output(lines, "/repo", 3);
        assert!(output.contains("3 results limit reached. Use limit=6 for more"));
        assert_eq!(details.unwrap().result_limit_reached, Some(3));
    }

    #[test]
    fn relative_path_fallback_outside_root() {
        assert_eq!(
            relative_path("/repo/sub", "/repo/other/x.ts"),
            "../other/x.ts"
        );
    }
}
