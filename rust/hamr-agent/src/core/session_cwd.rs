//! Session working directory validation.
//!
//! Validates that a session's stored cwd still exists on disk.
//! Ported from `packages/coding-agent/src/core/session-cwd.ts`.

use std::path::Path;

/// A detected mismatch between a session's stored cwd and the current filesystem.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionCwdIssue {
    pub session_file: Option<String>,
    pub session_cwd: String,
    pub fallback_cwd: String,
}

/// Minimal interface for extracting cwd and session path from a session manager.
pub trait SessionCwdSource {
    fn get_cwd(&self) -> String;
    fn get_session_file(&self) -> Option<String>;
}

/// Check whether a session's stored cwd exists. Returns `None` when everything is
/// fine, or an issue descriptor when the stored cwd is missing.
pub fn get_missing_session_cwd_issue(
    session_manager: &dyn SessionCwdSource,
    fallback_cwd: &str,
) -> Option<SessionCwdIssue> {
    let session_file = session_manager.get_session_file()?;
    let session_cwd = session_manager.get_cwd();

    if session_cwd.is_empty() || Path::new(&session_cwd).exists() {
        return None;
    }

    Some(SessionCwdIssue {
        session_file: Some(session_file),
        session_cwd,
        fallback_cwd: fallback_cwd.to_string(),
    })
}

/// Format a human-readable error message for a missing session cwd.
pub fn format_missing_session_cwd_error(issue: &SessionCwdIssue) -> String {
    let session_file_line = issue
        .session_file
        .as_ref()
        .map(|f| format!("\nSession file: {f}"))
        .unwrap_or_default();
    format!(
        "Stored session working directory does not exist: {}{session_file_line}\nCurrent working directory: {}",
        issue.session_cwd, issue.fallback_cwd
    )
}

/// Format a compact prompt for cwd resolution (used in TUI prompts).
pub fn format_missing_session_cwd_prompt(issue: &SessionCwdIssue) -> String {
    format!(
        "cwd from session file does not exist\n{}\n\ncontinue in current cwd\n{}",
        issue.session_cwd, issue.fallback_cwd
    )
}

/// Error thrown when a session's stored cwd does not exist.
#[derive(Debug, Clone)]
pub struct MissingSessionCwdError {
    pub issue: SessionCwdIssue,
}

impl MissingSessionCwdError {
    pub fn new(issue: SessionCwdIssue) -> Self {
        Self { issue }
    }
}

impl std::fmt::Display for MissingSessionCwdError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", format_missing_session_cwd_error(&self.issue))
    }
}

impl std::error::Error for MissingSessionCwdError {}

/// Assert that the session's stored cwd exists, or throw `MissingSessionCwdError`.
pub fn assert_session_cwd_exists(
    session_manager: &dyn SessionCwdSource,
    fallback_cwd: &str,
) -> Result<(), MissingSessionCwdError> {
    match get_missing_session_cwd_issue(session_manager, fallback_cwd) {
        Some(issue) => Err(MissingSessionCwdError::new(issue)),
        None => Ok(()),
    }
}

// ---------------------------------------------------------------------------
// Tests — verbatim from TS test expectations
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    struct TestSessionCwd {
        cwd: String,
        session_file: Option<String>,
    }

    impl SessionCwdSource for TestSessionCwd {
        fn get_cwd(&self) -> String {
            self.cwd.clone()
        }
        fn get_session_file(&self) -> Option<String> {
            self.session_file.clone()
        }
    }

    /// Create a temp dir that is guaranteed to exist, returning its path.
    fn make_temp_dir(prefix: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("{}-{}", prefix, uuid::Uuid::new_v4()));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn test_detects_missing_session_cwd() {
        let fallback = make_temp_dir("pi-session-cwd-fallback");
        let missing = fallback.join("does-not-exist");
        let session_dir = make_temp_dir("pi-session-cwd-session-dir");
        let session_file = session_dir.join("session.jsonl");

        let mgr = TestSessionCwd {
            cwd: missing.to_string_lossy().to_string(),
            session_file: Some(session_file.to_string_lossy().to_string()),
        };

        let issue = get_missing_session_cwd_issue(&mgr, &fallback.to_string_lossy()).unwrap();
        assert_eq!(issue.session_file, mgr.session_file);
        assert_eq!(issue.session_cwd, missing.to_string_lossy());
        assert_eq!(issue.fallback_cwd, fallback.to_string_lossy());
    }

    #[test]
    fn test_no_issue_when_cwd_exists() {
        let existing = make_temp_dir("pi-session-cwd-exists");
        let mgr = TestSessionCwd {
            cwd: existing.to_string_lossy().to_string(),
            session_file: Some("/tmp/session.jsonl".to_string()),
        };
        let issue = get_missing_session_cwd_issue(&mgr, "/tmp");
        assert!(issue.is_none());
    }

    #[test]
    fn test_no_issue_when_no_session_file() {
        let mgr = TestSessionCwd {
            cwd: "/does/not/exist".to_string(),
            session_file: None,
        };
        let issue = get_missing_session_cwd_issue(&mgr, "/tmp");
        assert!(issue.is_none());
    }

    #[test]
    fn test_assert_session_cwd_exists_throws() {
        let fallback = make_temp_dir("pi-session-cwd-assert");
        let missing = fallback.join("does-not-exist");
        let mgr = TestSessionCwd {
            cwd: missing.to_string_lossy().to_string(),
            session_file: Some("/tmp/session.jsonl".to_string()),
        };
        let result = assert_session_cwd_exists(&mgr, &fallback.to_string_lossy());
        assert!(result.is_err());
    }

    #[test]
    fn test_assert_session_cwd_exists_ok() {
        let existing = make_temp_dir("pi-session-cwd-assert-ok");
        let mgr = TestSessionCwd {
            cwd: existing.to_string_lossy().to_string(),
            session_file: Some("/tmp/session.jsonl".to_string()),
        };
        let result = assert_session_cwd_exists(&mgr, &existing.to_string_lossy());
        assert!(result.is_ok());
    }
}
