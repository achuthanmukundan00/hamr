//! TUI session selector for the `--resume` flag.
//!
//! Port of `packages/coding-agent/src/cli/session-picker.ts`.
//!
//! TUI components (SessionSelectorComponent) are not yet ported.
//! This implementation uses stdin/stdout fallback when multiple sessions exist.

use std::future::Future;
use std::io::{self, Write};
use std::pin::Pin;
use std::sync::Arc;

/// Stub session info — will be replaced when SessionManager is ported.
#[derive(Debug, Clone)]
pub struct SessionInfo {
    pub id: String,
    pub path: String,
    pub cwd: String,
}

/// Session list progress callback type.
pub type SessionListProgress =
    dyn Fn(usize, usize) -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + Sync;

/// Loader function: returns a list of sessions, optionally reporting progress.
pub type SessionsLoader = Arc<
    dyn Fn(Option<&SessionListProgress>) -> Pin<Box<dyn Future<Output = Vec<SessionInfo>> + Send>>
        + Send
        + Sync,
>;

/// Show TUI session selector and return selected session path, or None if cancelled.
///
/// When TUI is not available, falls back to stdin/stdout selection.
/// If only one session exists, returns it directly.
pub async fn select_session(
    current_sessions_loader: SessionsLoader,
    all_sessions_loader: SessionsLoader,
) -> Option<String> {
    // Try current sessions first (project-scoped)
    let current = current_sessions_loader(None).await;

    let sessions = if current.len() > 1 {
        current
    } else {
        // Fall back to all sessions
        let all = all_sessions_loader(None).await;
        if all.is_empty() {
            eprintln!("No sessions found.");
            return None;
        }
        all
    };

    if sessions.is_empty() {
        eprintln!("No sessions found.");
        return None;
    }

    // Single session — return directly
    if sessions.len() == 1 {
        return Some(sessions[0].path.clone());
    }

    // Multiple sessions — show selector
    eprintln!("\nSelect a session to resume:\n");
    for (i, session) in sessions.iter().enumerate() {
        let display = if session.cwd.is_empty() {
            &session.id
        } else {
            &session.cwd
        };
        eprintln!("  [{}] {} ({})", i + 1, session.id, display);
    }
    eprint!("\nSelect (1-{}), or 0 to cancel: ", sessions.len());
    let _ = io::stderr().flush();

    let mut input = String::new();
    if io::stdin().read_line(&mut input).is_err() {
        return None;
    }
    let trimmed = input.trim();
    let selection: usize = match trimmed.parse() {
        Ok(n) => n,
        Err(_) => return None,
    };
    if selection == 0 || selection > sessions.len() {
        return None;
    }
    Some(sessions[selection - 1].path.clone())
}
