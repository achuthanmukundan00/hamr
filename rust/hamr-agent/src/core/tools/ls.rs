//! Port of `packages/coding-agent/src/core/tools/ls.ts` — the ls tool.
//!
//! Lists directory contents sorted alphabetically (case-insensitive), with a
//! `/` suffix for directories, dotfiles included. Output is capped at a
//! configurable entry limit (default 500) or a byte limit, whichever hits first.
//!
//! Following the convention of the other ported tools, this module covers the
//! execute logic and types; TUI rendering (`renderCall`/`renderResult`) is
//! handled elsewhere.

use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::Arc;

use tokio::sync::watch;

use hamr_ai::types::{MessageContent, TextContent};

use super::path_utils::resolve_to_cwd;
use super::truncate::{
    DEFAULT_MAX_BYTES, TruncationOptions, TruncationResult, format_size, truncate_head,
};

const DEFAULT_LIMIT: usize = 500;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// Input parameters for the ls tool.
#[derive(Debug, Clone, serde::Deserialize, schemars::JsonSchema)]
pub struct LsToolInput {
    /// Directory to list (default: current directory).
    #[serde(default)]
    pub path: Option<String>,
    /// Maximum number of entries to return (default: 500).
    #[serde(default)]
    pub limit: Option<usize>,
}

/// Details attached to an ls result.
#[derive(Debug, Clone, Default)]
pub struct LsToolDetails {
    pub truncation: Option<TruncationResult>,
    pub entry_limit_reached: Option<usize>,
}

impl LsToolDetails {
    fn is_empty(&self) -> bool {
        self.truncation.is_none() && self.entry_limit_reached.is_none()
    }
}

/// Result of executing the ls tool.
#[derive(Debug, Clone)]
pub struct LsToolResult {
    pub content: Vec<MessageContent>,
    pub details: Option<LsToolDetails>,
}

/// Errors that can occur while listing a directory.
#[derive(Debug, thiserror::Error)]
pub enum LsToolError {
    #[error("Path not found: {0}")]
    PathNotFound(String),
    #[error("Not a directory: {0}")]
    NotADirectory(String),
    #[error("Cannot read directory: {source}")]
    CannotReadDir {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("Operation aborted")]
    Aborted,
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

// ---------------------------------------------------------------------------
// Pluggable operations
// ---------------------------------------------------------------------------

type LsFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// Pluggable filesystem operations for the ls tool. Override to delegate
/// directory listing to remote systems (for example SSH).
pub trait LsOperations: Send + Sync {
    /// Whether `path` exists.
    fn exists<'a>(&'a self, path: &'a Path) -> LsFuture<'a, bool>;
    /// Whether `path` is a directory. Errors if the path cannot be stat'd.
    fn is_directory<'a>(&'a self, path: &'a Path) -> LsFuture<'a, std::io::Result<bool>>;
    /// Names of the entries directly inside `path`.
    fn read_dir<'a>(&'a self, path: &'a Path) -> LsFuture<'a, std::io::Result<Vec<String>>>;
}

/// Default local-filesystem operations backed by `tokio::fs`.
pub struct LocalLsOperations;

impl LsOperations for LocalLsOperations {
    fn exists<'a>(&'a self, path: &'a Path) -> LsFuture<'a, bool> {
        Box::pin(async move { tokio::fs::try_exists(path).await.unwrap_or(false) })
    }

    fn is_directory<'a>(&'a self, path: &'a Path) -> LsFuture<'a, std::io::Result<bool>> {
        Box::pin(async move {
            let meta = tokio::fs::metadata(path).await?;
            Ok(meta.is_dir())
        })
    }

    fn read_dir<'a>(&'a self, path: &'a Path) -> LsFuture<'a, std::io::Result<Vec<String>>> {
        Box::pin(async move {
            let mut rd = tokio::fs::read_dir(path).await?;
            let mut names = Vec::new();
            while let Some(entry) = rd.next_entry().await? {
                names.push(entry.file_name().to_string_lossy().into_owned());
            }
            Ok(names)
        })
    }
}

// ---------------------------------------------------------------------------
// LsTool
// ---------------------------------------------------------------------------

/// The ls tool — lists directory entries from the filesystem.
pub struct LsTool {
    cwd: PathBuf,
    operations: Arc<dyn LsOperations>,
}

/// Options for constructing an [`LsTool`].
#[derive(Default)]
pub struct LsToolOptions {
    pub operations: Option<Arc<dyn LsOperations>>,
}

impl LsTool {
    /// Create a new ls tool rooted at `cwd`.
    pub fn new(cwd: &Path, options: LsToolOptions) -> Self {
        let operations = options
            .operations
            .unwrap_or_else(|| Arc::new(LocalLsOperations));
        Self {
            cwd: cwd.to_path_buf(),
            operations,
        }
    }

    /// List the directory described by `input`.
    pub async fn execute(
        &self,
        input: &LsToolInput,
        abort_rx: Option<&watch::Receiver<bool>>,
    ) -> Result<LsToolResult, LsToolError> {
        if is_aborted(abort_rx) {
            return Err(LsToolError::Aborted);
        }

        let dir_path = resolve_to_cwd(input.path.as_deref().unwrap_or("."), &self.cwd);
        let effective_limit = input.limit.unwrap_or(DEFAULT_LIMIT);
        let display = dir_path.to_string_lossy().into_owned();

        // Check existence.
        if !self.operations.exists(&dir_path).await {
            return Err(LsToolError::PathNotFound(display));
        }

        // Check it is a directory.
        let is_dir = self
            .operations
            .is_directory(&dir_path)
            .await
            .map_err(|source| LsToolError::CannotReadDir {
                path: display.clone(),
                source,
            })?;
        if !is_dir {
            return Err(LsToolError::NotADirectory(display));
        }

        // Read entries.
        let mut entries = self
            .operations
            .read_dir(&dir_path)
            .await
            .map_err(|source| LsToolError::CannotReadDir {
                path: display.clone(),
                source,
            })?;

        // Sort alphabetically, case-insensitive.
        entries.sort_by(|a, b| a.to_lowercase().cmp(&b.to_lowercase()));

        // Format entries with directory indicators, capping at the entry limit.
        let mut results: Vec<String> = Vec::new();
        let mut entry_limit_reached = false;
        for entry in &entries {
            if is_aborted(abort_rx) {
                return Err(LsToolError::Aborted);
            }
            if results.len() >= effective_limit {
                entry_limit_reached = true;
                break;
            }
            let full_path = dir_path.join(entry);
            let suffix = match self.operations.is_directory(&full_path).await {
                Ok(true) => "/",
                Ok(false) => "",
                // Skip entries we cannot stat.
                Err(_) => continue,
            };
            results.push(format!("{entry}{suffix}"));
        }

        if results.is_empty() {
            return Ok(LsToolResult {
                content: vec![text_content("(empty directory)")],
                details: None,
            });
        }

        let (output, details) = build_ls_output(results, entry_limit_reached, effective_limit);
        Ok(LsToolResult {
            content: vec![text_content(&output)],
            details,
        })
    }
}

/// Build the textual output and details from formatted entries, applying byte
/// truncation and appending actionable notices. Factored out for testing.
fn build_ls_output(
    results: Vec<String>,
    entry_limit_reached: bool,
    effective_limit: usize,
) -> (String, Option<LsToolDetails>) {
    let raw_output = results.join("\n");
    // Apply byte truncation only — entry count is already capped above, so the
    // line limit is effectively disabled.
    let truncation = truncate_head(
        &raw_output,
        TruncationOptions {
            max_lines: Some(usize::MAX),
            max_bytes: None,
        },
    );
    let mut output = truncation.content.clone();
    let mut details = LsToolDetails::default();
    let mut notices: Vec<String> = Vec::new();

    if entry_limit_reached {
        notices.push(format!(
            "{effective_limit} entries limit reached. Use limit={} for more",
            effective_limit * 2
        ));
        details.entry_limit_reached = Some(effective_limit);
    }
    if truncation.truncated {
        notices.push(format!("{} limit reached", format_size(DEFAULT_MAX_BYTES)));
        details.truncation = Some(truncation);
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

/// Create an [`LsTool`] rooted at `cwd` with default local operations.
pub fn create_ls_tool(cwd: &Path, options: Option<LsToolOptions>) -> LsTool {
    LsTool::new(cwd, options.unwrap_or_default())
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

fn is_aborted(abort_rx: Option<&watch::Receiver<bool>>) -> bool {
    abort_rx.map(|rx| *rx.borrow()).unwrap_or(false)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entry_limit_notice_and_details() {
        let entries: Vec<String> = (0..5).map(|i| format!("f{i}")).collect();
        let (output, details) = build_ls_output(entries, true, 5);
        assert!(output.contains("5 entries limit reached. Use limit=10 for more"));
        let details = details.expect("details present when limit reached");
        assert_eq!(details.entry_limit_reached, Some(5));
        assert!(details.truncation.is_none());
    }

    #[test]
    fn no_notice_when_within_limits() {
        let entries = vec!["a/".to_string(), "b".to_string()];
        let (output, details) = build_ls_output(entries, false, 500);
        assert_eq!(output, "a/\nb");
        assert!(details.is_none());
    }
}
