//! Port of `packages/coding-agent/src/core/tools/grep.ts` — the grep tool.
//!
//! Searches file contents for a pattern using ripgrep's `--json` output,
//! returning matching lines with file paths and line numbers. Respects
//! `.gitignore`. Output is capped at a match limit (default 100) or a byte
//! limit, whichever hits first; long match lines are truncated.
//!
//! As with the other ported tools, this covers the execute logic and types;
//! TUI rendering is handled elsewhere.
//!
//! NOTE: the TypeScript original auto-downloads `rg` via `ensureTool`. Until
//! `tools-manager` is ported, this resolves `rg` from `PATH` and errors if it
//! is unavailable.

use std::collections::HashMap;
use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::process::Stdio;
use std::sync::Arc;

use tokio::io::{AsyncBufReadExt, AsyncReadExt, BufReader};
use tokio::process::Command as TokioCommand;
use tokio::sync::watch;

use hamr_ai::types::{MessageContent, TextContent};

use super::path_utils::resolve_to_cwd;
use super::truncate::{
    DEFAULT_MAX_BYTES, GREP_MAX_LINE_LENGTH, TruncationOptions, TruncationResult, format_size,
    truncate_head, truncate_line,
};

const DEFAULT_LIMIT: usize = 100;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// Input parameters for the grep tool.
#[derive(Debug, Clone, serde::Deserialize, schemars::JsonSchema)]
pub struct GrepToolInput {
    /// Search pattern (regex or literal string).
    pub pattern: String,
    /// Directory or file to search (default: current directory).
    #[serde(default)]
    pub path: Option<String>,
    /// Filter files by glob pattern, e.g. `*.ts`.
    #[serde(default)]
    pub glob: Option<String>,
    /// Case-insensitive search (default: false).
    #[serde(default)]
    pub ignore_case: Option<bool>,
    /// Treat pattern as a literal string instead of regex (default: false).
    #[serde(default)]
    pub literal: Option<bool>,
    /// Lines of context to show before and after each match (default: 0).
    #[serde(default)]
    pub context: Option<usize>,
    /// Maximum number of matches to return (default: 100).
    #[serde(default)]
    pub limit: Option<usize>,
}

/// Details attached to a grep result.
#[derive(Debug, Clone, Default)]
pub struct GrepToolDetails {
    pub truncation: Option<TruncationResult>,
    pub match_limit_reached: Option<usize>,
    pub lines_truncated: bool,
}

impl GrepToolDetails {
    fn is_empty(&self) -> bool {
        self.truncation.is_none() && self.match_limit_reached.is_none() && !self.lines_truncated
    }
}

/// Result of executing the grep tool.
#[derive(Debug, Clone)]
pub struct GrepToolResult {
    pub content: Vec<MessageContent>,
    pub details: Option<GrepToolDetails>,
}

/// Errors that can occur while grepping.
#[derive(Debug, thiserror::Error)]
pub enum GrepToolError {
    #[error("Path not found: {0}")]
    PathNotFound(String),
    #[error("ripgrep (rg) is not available on PATH")]
    RgUnavailable,
    #[error("Failed to run ripgrep: {0}")]
    SpawnFailed(String),
    #[error("{0}")]
    Rg(String),
    #[error("Operation aborted")]
    Aborted,
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

// ---------------------------------------------------------------------------
// Pluggable operations
// ---------------------------------------------------------------------------

type GrepFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// Pluggable operations for the grep tool. Override to delegate to remote
/// systems (for example SSH).
pub trait GrepOperations: Send + Sync {
    /// Whether `path` is a directory. Errors if the path does not exist.
    fn is_directory<'a>(&'a self, path: &'a Path) -> GrepFuture<'a, std::io::Result<bool>>;
    /// Read file contents (for context lines).
    fn read_file<'a>(&'a self, path: &'a Path) -> GrepFuture<'a, std::io::Result<String>>;
}

/// Default local-filesystem operations backed by `tokio::fs`.
pub struct LocalGrepOperations;

impl GrepOperations for LocalGrepOperations {
    fn is_directory<'a>(&'a self, path: &'a Path) -> GrepFuture<'a, std::io::Result<bool>> {
        Box::pin(async move { Ok(tokio::fs::metadata(path).await?.is_dir()) })
    }

    fn read_file<'a>(&'a self, path: &'a Path) -> GrepFuture<'a, std::io::Result<String>> {
        Box::pin(async move { tokio::fs::read_to_string(path).await })
    }
}

// ---------------------------------------------------------------------------
// GrepTool
// ---------------------------------------------------------------------------

/// The grep tool — searches file contents for a pattern.
pub struct GrepTool {
    cwd: PathBuf,
    operations: Arc<dyn GrepOperations>,
}

/// Options for constructing a [`GrepTool`].
#[derive(Default)]
pub struct GrepToolOptions {
    pub operations: Option<Arc<dyn GrepOperations>>,
}

impl GrepTool {
    /// Create a new grep tool rooted at `cwd`.
    pub fn new(cwd: &Path, options: GrepToolOptions) -> Self {
        let operations = options
            .operations
            .unwrap_or_else(|| Arc::new(LocalGrepOperations));
        Self {
            cwd: cwd.to_path_buf(),
            operations,
        }
    }

    /// Execute the grep with the given input.
    pub async fn execute(
        &self,
        input: &GrepToolInput,
        abort_rx: Option<watch::Receiver<bool>>,
    ) -> Result<GrepToolResult, GrepToolError> {
        let mut abort_rx = abort_rx;
        if is_aborted(abort_rx.as_ref()) {
            return Err(GrepToolError::Aborted);
        }

        let rg_path = resolve_rg().ok_or(GrepToolError::RgUnavailable)?;

        let search_path = resolve_to_cwd(input.path.as_deref().unwrap_or("."), &self.cwd);
        let search_path_str = search_path.to_string_lossy().into_owned();

        // The search root must exist (and tells us whether to relativize paths).
        let is_directory = match self.operations.is_directory(&search_path).await {
            Ok(is_dir) => is_dir,
            Err(_) => return Err(GrepToolError::PathNotFound(search_path_str)),
        };

        let context_value = input.context.filter(|c| *c > 0).unwrap_or(0);
        let effective_limit = input.limit.unwrap_or(DEFAULT_LIMIT).max(1);

        // Build rg arguments.
        let mut args: Vec<String> = vec![
            "--json".into(),
            "--line-number".into(),
            "--color=never".into(),
            "--hidden".into(),
        ];
        if input.ignore_case.unwrap_or(false) {
            args.push("--ignore-case".into());
        }
        if input.literal.unwrap_or(false) {
            args.push("--fixed-strings".into());
        }
        if let Some(glob) = &input.glob {
            args.push("--glob".into());
            args.push(glob.clone());
        }
        args.push("--".into());
        args.push(input.pattern.clone());
        args.push(search_path_str.clone());

        let mut child = TokioCommand::new(&rg_path)
            .args(&args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| GrepToolError::SpawnFailed(e.to_string()))?;

        let pid = child.id();
        let stdout = child.stdout.take().expect("stdout piped");
        let mut reader = BufReader::new(stdout);
        let mut line = String::new();

        // Collect matches during streaming; format them afterwards so custom
        // read_file() backends can be async.
        let mut matches: Vec<RgMatch> = Vec::new();
        let mut match_count = 0usize;
        let mut match_limit_reached = false;
        let mut killed_due_to_limit = false;
        let mut aborted = false;

        loop {
            line.clear();
            let n = tokio::select! {
                r = reader.read_line(&mut line) => r?,
                _ = wait_for_abort(&mut abort_rx) => {
                    aborted = true;
                    if let Some(pid) = pid { let _ = kill_pid(pid); }
                    break;
                }
            };
            if n == 0 {
                break; // EOF
            }
            let trimmed = line.trim();
            if trimmed.is_empty() || match_count >= effective_limit {
                continue;
            }
            let Ok(event) = serde_json::from_str::<serde_json::Value>(trimmed) else {
                continue;
            };
            if event.get("type").and_then(|t| t.as_str()) != Some("match") {
                continue;
            }
            let data = &event["data"];
            let file_path = data
                .get("path")
                .and_then(|p| p.get("text"))
                .and_then(|t| t.as_str());
            let line_number = data.get("line_number").and_then(|n| n.as_u64());
            let line_text = data
                .get("lines")
                .and_then(|l| l.get("text"))
                .and_then(|t| t.as_str())
                .map(|s| s.to_string());
            if let (Some(file_path), Some(line_number)) = (file_path, line_number) {
                match_count += 1;
                matches.push(RgMatch {
                    file_path: file_path.to_string(),
                    line_number: line_number as usize,
                    line_text,
                });
                if match_count >= effective_limit {
                    match_limit_reached = true;
                    killed_due_to_limit = true;
                    if let Some(pid) = pid {
                        let _ = kill_pid(pid);
                    }
                    break;
                }
            }
        }

        // Drain stderr (small — errors only) and reap the child.
        let mut stderr_buf = String::new();
        if let Some(mut err) = child.stderr.take() {
            let _ = err.read_to_string(&mut stderr_buf).await;
        }
        let status = child.wait().await?;

        if aborted {
            return Err(GrepToolError::Aborted);
        }

        let code = status.code();
        // rg exits 0 (matches) or 1 (no matches); other codes are real errors.
        if !killed_due_to_limit && code != Some(0) && code != Some(1) {
            let msg = stderr_buf.trim();
            let msg = if msg.is_empty() {
                format!(
                    "ripgrep exited with code {}",
                    code.map(|c| c.to_string())
                        .unwrap_or_else(|| "unknown".into())
                )
            } else {
                msg.to_string()
            };
            return Err(GrepToolError::Rg(msg));
        }

        if match_count == 0 {
            return Ok(GrepToolResult {
                content: vec![text_content("No matches found")],
                details: None,
            });
        }

        // Format matches into output lines.
        let mut output_lines: Vec<String> = Vec::new();
        let mut lines_truncated = false;
        let mut file_cache: HashMap<String, Vec<String>> = HashMap::new();

        for m in &matches {
            if context_value == 0 {
                if let Some(line_text) = &m.line_text {
                    let relative_path = format_path(&m.file_path, is_directory, &search_path_str);
                    let sanitized = line_text.replace("\r\n", "\n").replace('\r', "");
                    let sanitized = sanitized
                        .strip_suffix('\n')
                        .unwrap_or(&sanitized)
                        .to_string();
                    let lt = truncate_line(&sanitized, GREP_MAX_LINE_LENGTH);
                    if lt.was_truncated {
                        lines_truncated = true;
                    }
                    output_lines.push(format!("{relative_path}:{}: {}", m.line_number, lt.text));
                    continue;
                }
            }
            let block = self
                .format_block(
                    &m.file_path,
                    m.line_number,
                    context_value,
                    is_directory,
                    &search_path_str,
                    &mut file_cache,
                    &mut lines_truncated,
                )
                .await;
            output_lines.extend(block);
        }

        let (output, details) = assemble_grep_output(
            output_lines,
            match_limit_reached,
            effective_limit,
            lines_truncated,
        );
        Ok(GrepToolResult {
            content: vec![text_content(&output)],
            details,
        })
    }

    /// Format a match with surrounding context lines.
    #[allow(clippy::too_many_arguments)]
    async fn format_block(
        &self,
        file_path: &str,
        line_number: usize,
        context_value: usize,
        is_directory: bool,
        search_path: &str,
        file_cache: &mut HashMap<String, Vec<String>>,
        lines_truncated: &mut bool,
    ) -> Vec<String> {
        let relative_path = format_path(file_path, is_directory, search_path);
        let lines = self.get_file_lines(file_path, file_cache).await;
        if lines.is_empty() {
            return vec![format!(
                "{relative_path}:{line_number}: (unable to read file)"
            )];
        }
        let mut block = Vec::new();
        let start = if context_value > 0 {
            line_number.saturating_sub(context_value).max(1)
        } else {
            line_number
        };
        let end = if context_value > 0 {
            (line_number + context_value).min(lines.len())
        } else {
            line_number
        };
        for current in start..=end {
            let line_text = lines.get(current - 1).map(String::as_str).unwrap_or("");
            let sanitized = line_text.replace('\r', "");
            let is_match_line = current == line_number;
            let lt = truncate_line(&sanitized, GREP_MAX_LINE_LENGTH);
            if lt.was_truncated {
                *lines_truncated = true;
            }
            if is_match_line {
                block.push(format!("{relative_path}:{current}: {}", lt.text));
            } else {
                block.push(format!("{relative_path}-{current}- {}", lt.text));
            }
        }
        block
    }

    /// Read and cache a file's lines (CRLF/CR normalized to LF).
    async fn get_file_lines(
        &self,
        file_path: &str,
        cache: &mut HashMap<String, Vec<String>>,
    ) -> Vec<String> {
        if let Some(lines) = cache.get(file_path) {
            return lines.clone();
        }
        let lines = match self.operations.read_file(Path::new(file_path)).await {
            Ok(content) => content
                .replace("\r\n", "\n")
                .replace('\r', "\n")
                .split('\n')
                .map(|s| s.to_string())
                .collect(),
            Err(_) => Vec::new(),
        };
        cache.insert(file_path.to_string(), lines.clone());
        lines
    }
}

struct RgMatch {
    file_path: String,
    line_number: usize,
    line_text: Option<String>,
}

/// Assemble formatted match lines into the final output, applying byte
/// truncation and appending actionable notices. Factored out for testing.
fn assemble_grep_output(
    output_lines: Vec<String>,
    match_limit_reached: bool,
    effective_limit: usize,
    lines_truncated: bool,
) -> (String, Option<GrepToolDetails>) {
    let raw_output = output_lines.join("\n");
    // Byte truncation only — the match limit already capped the row count.
    let truncation = truncate_head(
        &raw_output,
        TruncationOptions {
            max_lines: Some(usize::MAX),
            max_bytes: None,
        },
    );
    let mut output = truncation.content.clone();
    let mut details = GrepToolDetails::default();
    let mut notices: Vec<String> = Vec::new();

    if match_limit_reached {
        notices.push(format!(
            "{effective_limit} matches limit reached. Use limit={} for more, or refine pattern",
            effective_limit * 2
        ));
        details.match_limit_reached = Some(effective_limit);
    }
    if truncation.truncated {
        notices.push(format!("{} limit reached", format_size(DEFAULT_MAX_BYTES)));
        details.truncation = Some(truncation);
    }
    if lines_truncated {
        notices.push(format!(
            "Some lines truncated to {GREP_MAX_LINE_LENGTH} chars. Use read tool to see full lines"
        ));
        details.lines_truncated = true;
    }
    if !notices.is_empty() {
        output += &format!("\n\n[{}]", notices.join(". "));
    }

    let details = if details.is_empty() {
        None
    } else {
        Some(details)
    };
    (output, details)
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn text_content(text: &str) -> MessageContent {
    MessageContent::Text(TextContent {
        text: text.to_string(),
        text_signature: None,
    })
}

/// Format a matched file path relative to the search root when searching a
/// directory; otherwise just the basename.
fn format_path(file_path: &str, is_directory: bool, search_path: &str) -> String {
    if is_directory {
        let relative = relative_path(search_path, file_path);
        if !relative.is_empty() && !relative.starts_with("..") {
            return relative.replace('\\', "/");
        }
    }
    basename(file_path)
}

fn basename(path: &str) -> String {
    path.rsplit(['/', '\\']).next().unwrap_or(path).to_string()
}

/// Lexical `path.relative(base, target)`.
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

/// Locate `rg` on `PATH`.
fn resolve_rg() -> Option<PathBuf> {
    which("rg")
}

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
    fn format_path_relativizes_in_directory_mode() {
        assert_eq!(format_path("/repo/src/a.ts", true, "/repo"), "src/a.ts");
    }

    #[test]
    fn format_path_basename_for_single_file() {
        assert_eq!(
            format_path("/repo/src/a.ts", false, "/repo/src/a.ts"),
            "a.ts"
        );
    }

    #[test]
    fn assemble_appends_all_notices() {
        let lines = vec!["a.ts:1: match".to_string()];
        let (output, details) = assemble_grep_output(lines, true, 100, true);
        assert!(output.contains("100 matches limit reached"));
        assert!(output.contains("Some lines truncated to 500 chars"));
        let d = details.unwrap();
        assert_eq!(d.match_limit_reached, Some(100));
        assert!(d.lines_truncated);
    }

    #[test]
    fn assemble_no_notices_when_clean() {
        let lines = vec!["a.ts:1: match".to_string()];
        let (output, details) = assemble_grep_output(lines, false, 100, false);
        assert_eq!(output, "a.ts:1: match");
        assert!(details.is_none());
    }
}
