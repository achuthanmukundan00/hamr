//! HandoffManager — structured context handoff over FTS5 memory.
//!
//! Mirrors `packages/coding-agent/src/hamr/handoff/HandoffManager.ts`.
//!
//! Tracks handoff depth (max 3), generates structured manifests consumable
//! by subagents and future turns. Lean — uses HolographicMemory for all
//! storage.
//!
//! # Tool Registration
//!
//! The `register_handoff_tool` function depends on:
//! - `crate::core::extensions::types::ExtensionFactory` (not yet ported)
//! - `crate::core::extensions::types::define_tool` (not yet ported)
//!
//! Once the extension system is ported, the function body will compile.

use serde::{Deserialize, Serialize};

use crate::hamr::memory::holographic_memory::HolographicMemory;

// ─── Constants ───────────────────────────────────────────────────────────────

/// Maximum handoff depth. Attempts beyond this are rejected.
pub const MAX_DEPTH: u32 = 3;

// ─── Types ───────────────────────────────────────────────────────────────────

/// Why a handoff was triggered.
///
/// Mirror of TS `"context_exhaustion" | "task_delegation" | "explicit"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HandoffReason {
    ContextExhaustion,
    TaskDelegation,
    Explicit,
}

/// Options passed to `create_handoff`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HandoffOptions {
    /// Why the handoff was triggered.
    pub reason: HandoffReason,
    /// The task being handed off.
    pub task: String,
    /// Files modified in this session.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub files_changed: Option<Vec<String>>,
    /// Files read/inspected in this session.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub files_read: Option<Vec<String>>,
    /// Work still pending.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pending_work: Option<Vec<String>>,
    /// Orchestration context (subtask id, plan id, sibling summaries).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub orchestration_context: Option<String>,
}

/// A structured handoff result.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StructuredHandoff {
    pub handoff_id: String,
    pub parent_session_id: String,
    pub reason: HandoffReason,
    pub task: String,
    pub status: String,
    pub key_findings: Vec<String>,
    pub files_changed: Vec<String>,
    pub files_read: Vec<String>,
    pub pending_work: Vec<String>,
    pub suggested_search_terms: Vec<String>,
    pub depth: u32,
    pub created_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub orchestration_context: Option<String>,
}

// ─── HandoffManager ──────────────────────────────────────────────────────────

/// Manages handoff depth and generates structured handoff manifests.
///
/// Tracks depth across a chain of handoffs. Each child handoff increments
/// the depth. At `MAX_DEPTH` (3), further handoffs are rejected.
#[derive(Debug, Clone)]
pub struct HandoffManager {
    depth: u32,
}

impl HandoffManager {
    /// Create a new HandoffManager with the given initial depth (default 0).
    pub fn new(initial_depth: u32) -> Self {
        HandoffManager {
            depth: initial_depth.min(MAX_DEPTH),
        }
    }

    /// The current handoff depth.
    pub fn current_depth(&self) -> u32 {
        self.depth
    }

    /// Returns `true` if another handoff can be created (depth < `MAX_DEPTH`).
    pub fn can_handoff(&self) -> bool {
        self.depth < MAX_DEPTH
    }

    /// Generate a structured handoff manifest combining FTS5 memory data
    /// with session-level metadata.
    ///
    /// Mirror of `createHandoff(sessionId, memory, options)` in TS.
    pub fn create_handoff(
        &self,
        session_id: &str,
        memory: Option<&HolographicMemory>,
        options: &HandoffOptions,
    ) -> StructuredHandoff {
        let memory_manifest = memory.map(|m| m.handoff(None));
        let handoff_id = format!(
            "handoff-{}-{}",
            chrono::Utc::now().timestamp_millis(),
            self.depth
        );

        // Merge memory-derived findings with caller-supplied context
        let mut key_findings: Vec<String> = memory_manifest
            .as_ref()
            .map(|m| m.key_findings.clone())
            .unwrap_or_default();
        if let Some(ref pending) = options.pending_work {
            for w in pending {
                key_findings.push(format!("pending: {}", w));
            }
        }
        key_findings.truncate(15);

        // filesChanged: deduplicate merge of options + memory filesTouched
        let mut files_changed: Vec<String> = options.files_changed.clone().unwrap_or_default();
        if let Some(ref manifest) = memory_manifest {
            for f in &manifest.files_touched {
                if !files_changed.contains(f) {
                    files_changed.push(f.clone());
                }
            }
        }
        files_changed.truncate(30);

        // filesRead: just dedup what the caller supplied
        let files_read: Vec<String> = {
            let mut seen = std::collections::HashSet::new();
            let mut v: Vec<String> = Vec::new();
            if let Some(ref r) = options.files_read {
                for f in r {
                    if seen.insert(f.clone()) {
                        v.push(f.clone());
                    }
                }
            }
            v.truncate(30);
            v
        };

        // Build status string
        let mut status_parts: Vec<String> = Vec::new();
        if let Some(ref manifest) = memory_manifest {
            status_parts.push(format!(
                "{} memory entries across {} turns",
                manifest.entry_count, manifest.turn_count
            ));
        }
        if !files_changed.is_empty() {
            status_parts.push(format!("{} files changed", files_changed.len()));
        }
        if !files_read.is_empty() {
            status_parts.push(format!("{} files read", files_read.len()));
        }
        let status = if status_parts.is_empty() {
            "no prior state".to_string()
        } else {
            status_parts.join("; ")
        };

        StructuredHandoff {
            handoff_id,
            parent_session_id: session_id.to_string(),
            reason: options.reason,
            task: options.task.clone(),
            status,
            key_findings,
            files_changed,
            files_read,
            pending_work: options.pending_work.clone().unwrap_or_default(),
            suggested_search_terms: memory_manifest
                .as_ref()
                .map(|m| m.suggested_search_terms.clone())
                .unwrap_or_default(),
            depth: self.depth,
            created_at: chrono::Utc::now().to_rfc3339(),
            orchestration_context: options.orchestration_context.clone(),
        }
    }

    /// Increment depth for a child handoff. Returns a new `HandoffManager`
    /// for the child.
    pub fn for_child(&self) -> Self {
        HandoffManager::new(self.depth + 1)
    }
}

// ─── Per-session registry ────────────────────────────────────────────────────

use std::collections::HashMap;
use std::sync::Mutex;

/// Global per-session HandoffManager instances.
///
/// Mirror of TS `const managersBySession = new Map<string, HandoffManager>()`.
static MANAGERS_BY_SESSION: std::sync::LazyLock<Mutex<HashMap<String, HandoffManager>>> =
    std::sync::LazyLock::new(|| Mutex::new(HashMap::new()));

/// Get the HandoffManager for a session (creating if absent), clone it out
/// of the lock so callers don't need to hold the mutex.
///
/// Mirror of TS `getManager(sessionId)`.
pub fn get_manager(session_id: &str) -> HandoffManager {
    let mut map = MANAGERS_BY_SESSION.lock().expect("handoff manager lock");
    map.entry(session_id.to_string())
        .or_insert_with(|| HandoffManager::new(0))
        .clone()
}

/// Update the HandoffManager for a session (e.g. after advancing depth).
///
/// Mirror of TS `managersBySession.set(sessionId, manager.forChild())`.
pub fn set_manager(session_id: &str, manager: HandoffManager) {
    let mut map = MANAGERS_BY_SESSION.lock().expect("handoff manager lock");
    map.insert(session_id.to_string(), manager);
}

/// Advance the depth for a session after a handoff is created.
pub fn advance_session(session_id: &str) {
    let mut map = MANAGERS_BY_SESSION.lock().expect("handoff manager lock");
    if let Some(manager) = map.get(session_id) {
        let child = manager.for_child();
        map.insert(session_id.to_string(), child);
    }
}

// ─── Tool registration ───────────────────────────────────────────────────────

/// Register the `create_handoff` tool.
///
/// Mirror of TS `registerHandoffTool()`.
///
/// # Type Dependencies (to be ported)
///
/// Once `crate::core::extensions::types` defines `ExtensionAPI` and
/// `define_tool`, this function body can be uncommented:
///
/// ```ignore
/// pub fn register_handoff_tool(pi: &impl ExtensionAPI) {
///     pi.register_tool(define_tool(ToolDefinition {
///         name: "create_handoff".into(),
///         label: "Create handoff".into(),
///         description: "Create a structured handoff manifest...".into(),
///         prompt_snippet: "Use create_handoff to checkpoint state...".into(),
///         parameters: /* schemars::JsonSchema struct */,
///         execute: Arc::new(|_id, params, _signal, _on_update, ctx| {
///             Box::pin(async move {
///                 let session_id = ctx.session_manager.get_session_id();
///                 let manager = get_manager(&session_id);
///                 if !manager.can_handoff() {
///                     return AgentToolResult { is_error: true, .. };
///                 }
///                 let memory = get_memory(&ctx);
///                 let handoff = manager.create_handoff(&session_id, memory.as_ref(), &params);
///
///                 // Store in FTS5 memory
///                 if let Some(ref mem) = memory {
///                     mem.store(&MemoryEntry { ... });
///                 }
///
///                 // Advance depth
///                 set_manager(&session_id, manager.for_child());
///
///                 AgentToolResult {
///                     content: vec![MessageContent::Text(TextContent { text: format_handoff(&handoff) })],
///                     details: Some(serde_json::to_value(&handoff).unwrap()),
///                     ..Default::default()
///                 }
///             })
///         }),
///     }));
/// }
/// ```
pub fn register_handoff_tool() {
    // TODO: implement when ExtensionAPI is ported.
    // See the doc comment above for the full TS-equivalent logic.
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "hamr-memory")]
    use crate::hamr::memory::holographic_memory::MemoryEntry;
    #[cfg(feature = "hamr-memory")]
    use rusqlite::Connection;

    // ── HandoffManager tests ──────────────────────────────────────────────

    #[test]
    fn test_new_default_depth() {
        let manager = HandoffManager::new(0);
        assert_eq!(manager.current_depth(), 0);
        assert!(manager.can_handoff());
    }

    #[test]
    fn test_max_depth_cap() {
        let manager = HandoffManager::new(100);
        assert_eq!(manager.current_depth(), MAX_DEPTH);
        assert!(!manager.can_handoff());
    }

    #[test]
    fn test_for_child_increments() {
        let parent = HandoffManager::new(0);
        let child = parent.for_child();
        assert_eq!(child.current_depth(), 1);
        assert!(child.can_handoff());
    }

    #[test]
    fn test_for_child_at_max_depth() {
        let parent = HandoffManager::new(MAX_DEPTH);
        let child = parent.for_child();
        assert_eq!(child.current_depth(), MAX_DEPTH);
        assert!(!child.can_handoff());
    }

    #[test]
    fn test_create_handoff_basic() {
        let manager = HandoffManager::new(0);
        let options = HandoffOptions {
            reason: HandoffReason::Explicit,
            task: "Test task".into(),
            files_changed: Some(vec!["src/main.rs".into()]),
            files_read: Some(vec!["README.md".into()]),
            pending_work: Some(vec!["Write tests".into()]),
            orchestration_context: None,
        };

        let handoff = manager.create_handoff("session-1", None, &options);
        assert_eq!(handoff.parent_session_id, "session-1");
        assert_eq!(handoff.task, "Test task");
        assert_eq!(handoff.depth, 0);
        assert!(handoff.handoff_id.starts_with("handoff-"));
        assert!(handoff.files_changed.contains(&"src/main.rs".to_string()));
        assert!(handoff.files_read.contains(&"README.md".to_string()));
        assert!(handoff.pending_work.contains(&"Write tests".to_string()));
        assert!(
            handoff
                .key_findings
                .iter()
                .any(|f| f.contains("pending: Write tests"))
        );
    }

    #[test]
    fn test_create_handoff_with_memory() {
        #[cfg(feature = "hamr-memory")]
        {
            let conn = Connection::open_in_memory().unwrap();
            conn.execute_batch(
                "CREATE VIRTUAL TABLE IF NOT EXISTS memory_fts USING fts5(\
                     turn_id, session_id, role, tool_name, file_paths, content, domain_tags, \
                     tokenize='porter unicode61'\
                 )",
            )
            .unwrap();

            let mut memory = HolographicMemory::new(Some(conn));
            memory.store(&MemoryEntry {
                session_id: "session-1".into(),
                turn_id: 1,
                role: "assistant".into(),
                tool_name: None,
                file_paths: None,
                content: "I'll fix the bug in src/lib.rs".into(),
                domain_tags: None,
            });

            let manager = HandoffManager::new(0);
            let options = HandoffOptions {
                reason: HandoffReason::TaskDelegation,
                task: "Fix the bug".into(),
                files_changed: None,
                files_read: None,
                pending_work: None,
                orchestration_context: None,
            };

            let handoff = manager.create_handoff("session-1", Some(&memory), &options);
            assert_eq!(handoff.depth, 0);
            assert_eq!(handoff.reason, HandoffReason::TaskDelegation);
            assert!(handoff.status.contains("memory entries"));
            assert!(handoff.status.contains("turns"));
        }
    }

    #[test]
    fn test_create_handoff_no_memory_status() {
        let manager = HandoffManager::new(0);
        let options = HandoffOptions {
            reason: HandoffReason::Explicit,
            task: "simple".into(),
            files_changed: None,
            files_read: None,
            pending_work: None,
            orchestration_context: None,
        };

        let handoff = manager.create_handoff("s1", None, &options);
        assert_eq!(handoff.status, "no prior state");
    }

    #[test]
    fn test_create_handoff_files_changed_dedupe() {
        let manager = HandoffManager::new(0);
        let options = HandoffOptions {
            reason: HandoffReason::Explicit,
            task: "dedupe".into(),
            files_changed: Some(vec!["a.rs".into(), "b.rs".into()]),
            files_read: None,
            pending_work: None,
            orchestration_context: None,
        };

        let handoff = manager.create_handoff("s1", None, &options);
        assert_eq!(handoff.files_changed.len(), 2);
        // No duplicates
    }

    // ── Per-session registry tests ────────────────────────────────────────

    #[test]
    fn test_get_manager_creates_new() {
        let m = get_manager("fresh-session");
        assert_eq!(m.current_depth(), 0);
        assert!(m.can_handoff());
    }

    #[test]
    fn test_set_manager_updates() {
        let m = get_manager("update-session");
        assert_eq!(m.current_depth(), 0);

        set_manager("update-session", m.for_child());
        let m2 = get_manager("update-session");
        assert_eq!(m2.current_depth(), 1);
    }

    #[test]
    fn test_advance_session() {
        let _ = get_manager("advance-session");
        advance_session("advance-session");
        let m = get_manager("advance-session");
        assert_eq!(m.current_depth(), 1);
    }

    // ── Serialization tests ───────────────────────────────────────────────

    #[test]
    fn test_handoff_reason_serialization() {
        assert_eq!(
            serde_json::to_string(&HandoffReason::ContextExhaustion).unwrap(),
            "\"context_exhaustion\""
        );
        assert_eq!(
            serde_json::to_string(&HandoffReason::TaskDelegation).unwrap(),
            "\"task_delegation\""
        );
        assert_eq!(
            serde_json::to_string(&HandoffReason::Explicit).unwrap(),
            "\"explicit\""
        );
    }

    #[test]
    fn test_structured_handoff_serialization() {
        let handoff = StructuredHandoff {
            handoff_id: "handoff-123-0".into(),
            parent_session_id: "s1".into(),
            reason: HandoffReason::Explicit,
            task: "test".into(),
            status: "1 memory entries across 1 turns".into(),
            key_findings: vec!["found bug".into()],
            files_changed: vec!["src/main.rs".into()],
            files_read: vec![],
            pending_work: vec![],
            suggested_search_terms: vec!["bug".into()],
            depth: 0,
            created_at: "2024-01-01T00:00:00Z".into(),
            orchestration_context: None,
        };

        let json = serde_json::to_string(&handoff).unwrap();
        assert!(json.contains("\"handoffId\":\"handoff-123-0\""));
        assert!(json.contains("\"reason\":\"explicit\""));
        assert!(json.contains("\"keyFindings\":[\"found bug\"]"));
    }
}
