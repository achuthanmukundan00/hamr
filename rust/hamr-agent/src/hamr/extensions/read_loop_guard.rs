//! Port of `packages/coding-agent/src/hamr/extensions/read-loop-guard.ts` (v0.7.1).
//!
//! Read-loop guard — detects when the agent is stuck calling the same tool with
//! semantically identical arguments and nudges it toward a different approach.
//!
//! # v0.7.1: Identity-key system
//!
//! Instead of counting read-only turns at `turn_end`, the guard hooks `tool_call`
//! and builds an **identity key** for each call. Two calls are "identical" if they
//! have the same tool name AND the same identity key. The key captures
//! semantically identical invocations while permitting progressive exploration:
//!
//! - `read`: keyed by (path, offset, limit) — different offsets are NOT a loop.
//! - `edit`: keyed by (path, first-oldText-hash) — same targeted edit is a loop.
//! - `write`: keyed by (path, content-prefix) — differentiate iterative writes.
//! - `bash`: keyed by the command string itself.
//! - Other tools: keyed by full arguments JSON.
//!
//! A rolling history of the last 12 calls is maintained. When the last 5 are all
//! identical, a steer nudge fires (at most once per 15 seconds).

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use crate::core::extensions::types::*;

// ─── Configuration ────────────────────────────────────────────────────────────

/// How many identical tool calls in a row trigger a nudge.
const LOOP_THRESHOLD: usize = 5;

/// Minimum time between nudges (ms).
const COOLDOWN_MS: u64 = 15_000;

/// Max history length for loop detection window.
const MAX_HISTORY: usize = 12;

// ─── Identity key builders ────────────────────────────────────────────────────

/// Build a stable identity string for a tool call.
///
/// Two tool calls are "identical" if they have the same tool name AND the same
/// identity key.  The key is chosen to capture semantically identical invocations
/// while permitting progressive exploration.
///
/// Mirrors TS `identityKey(toolName, input)`.
fn identity_key(tool_name: &str, input: &serde_json::Value) -> String {
    match tool_name {
        "read" => {
            let path = input.get("path").and_then(|v| v.as_str()).unwrap_or("");
            let offset = input
                .get("offset")
                .map(|v| v.to_string())
                .unwrap_or_default();
            let limit = input
                .get("limit")
                .map(|v| v.to_string())
                .unwrap_or_default();
            format!("read:{}:{}:{}", path, offset, limit)
        }
        "edit" => {
            let path = input.get("path").and_then(|v| v.as_str()).unwrap_or("");
            // `edits: Array<{ oldText, newText }>` — key off the first edit's oldText
            if let Some(edits) = input.get("edits").and_then(|v| v.as_array()) {
                if let Some(first) = edits.first() {
                    let old_text = first.get("oldText").and_then(|v| v.as_str()).unwrap_or("");
                    let truncated = &old_text[..old_text.len().min(120)];
                    return format!("edit:{}:{}", path, truncated);
                }
            }
            // Fallback for legacy format { path, oldText, newText } at top level
            let legacy = input.get("oldText").and_then(|v| v.as_str()).unwrap_or("");
            let truncated = &legacy[..legacy.len().min(120)];
            format!("edit:{}:{}", path, truncated)
        }
        "write" => {
            let path = input.get("path").and_then(|v| v.as_str()).unwrap_or("");
            // Include a content prefix so iterative writes with different content
            // don't trigger a false loop nudge
            let content = input.get("content").and_then(|v| v.as_str()).unwrap_or("");
            let truncated = &content[..content.len().min(80)];
            format!("write:{}:{}", path, truncated)
        }
        "bash" => {
            let command = input.get("command").and_then(|v| v.as_str()).unwrap_or("");
            format!("bash:{}", command)
        }
        _ => {
            // Other tools: keyed by full arguments JSON
            format!("{}:{}", tool_name, input)
        }
    }
}

// ─── Extension ───────────────────────────────────────────────────────────────

/// Extension name constant.
pub const EXTENSION_NAME: &str = "hamr-read-loop-guard";

/// Creates the read-loop guard extension (v0.7.1 identity-key system).
///
/// Hooks `tool_call` events, building identity keys for each call.  When the
/// last 5 calls in the rolling history are all identical, fires a steer nudge
/// (at most once per 15 seconds).
///
/// Port of `hamrReadLoopGuardExtension` in the TS source (v0.7.1).
pub fn hamr_read_loop_guard_extension() -> ExtensionFactory {
    Arc::new(|pi: Arc<dyn ExtensionAPI>| {
        Box::pin(async move {
            let history: Arc<Mutex<Vec<String>>> =
                Arc::new(Mutex::new(Vec::with_capacity(MAX_HISTORY)));
            let last_nudge = Arc::new(AtomicU64::new(0));

            let pi_outer = pi.clone();
            pi.on(
                "tool_call",
                Arc::new({
                    move |event: serde_json::Value, ctx: Arc<dyn ExtensionContext>| {
                        let history = history.clone();
                        let last_nudge = last_nudge.clone();
                        let pi = pi_outer.clone();
                        Box::pin(async move {
                            let tool_name = event
                                .get("toolName")
                                .and_then(|v| v.as_str())
                                .unwrap_or("unknown");
                            let input = event.get("input").cloned().unwrap_or(serde_json::Value::Null);

                            let key = identity_key(tool_name, &input);

                            let mut hist = history.lock().unwrap();
                            hist.push(key.clone());
                            if hist.len() > MAX_HISTORY {
                                hist.remove(0);
                            }

                            // Check: are the last LOOP_THRESHOLD entries all identical?
                            if hist.len() >= LOOP_THRESHOLD {
                                let start = hist.len() - LOOP_THRESHOLD;
                                let recent = &hist[start..];
                                let all_same = recent.iter().all(|k| k == &key);

                                if all_same {
                                    let now = std::time::SystemTime::now()
                                        .duration_since(std::time::UNIX_EPOCH)
                                        .unwrap_or_default()
                                        .as_millis() as u64;
                                    let last = last_nudge.load(Ordering::Relaxed);

                                    if now.saturating_sub(last) > COOLDOWN_MS {
                                        last_nudge.store(now, Ordering::Relaxed);
                                        if !ctx.is_idle() {
                                            pi.send_user_message(
                                                SendUserContent::Text(format!(
                                                    "(You've called \"{}\" with identical arguments {} times in a row. \
                                                     This looks like a loop — try a different approach.)",
                                                    tool_name, LOOP_THRESHOLD
                                                )),
                                                Some(SendUserOptions {
                                                    deliver_as: Some("steer".to_string()),
                                                }),
                                            );
                                        }
                                    }
                                }
                            }

                            None
                        })
                    }
                }),
            );
        })
    })
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identity_key_read() {
        let input = serde_json::json!({
            "path": "/src/main.rs",
            "offset": 100,
            "limit": 50
        });
        let key = identity_key("read", &input);
        assert_eq!(key, "read:/src/main.rs:100:50");
    }

    #[test]
    fn test_identity_key_read_different_offset_not_loop() {
        let key1 = identity_key(
            "read",
            &serde_json::json!({"path": "f", "offset": 0, "limit": 10}),
        );
        let key2 = identity_key(
            "read",
            &serde_json::json!({"path": "f", "offset": 10, "limit": 10}),
        );
        assert_ne!(key1, key2);
    }

    #[test]
    fn test_identity_key_edit() {
        let input = serde_json::json!({
            "path": "/src/main.rs",
            "edits": [{"oldText": "hello world", "newText": "goodbye"}]
        });
        let key = identity_key("edit", &input);
        assert!(key.starts_with("edit:/src/main.rs:hello world"));
    }

    #[test]
    fn test_identity_key_edit_truncates_old_text() {
        let long_text = "x".repeat(200);
        let input = serde_json::json!({
            "path": "/f",
            "edits": [{"oldText": &long_text, "newText": "y"}]
        });
        let key = identity_key("edit", &input);
        // oldText truncated to 120 chars
        assert!(key.len() <= 130); // "edit:/f:" + 120 chars
    }

    #[test]
    fn test_identity_key_edit_legacy_format() {
        let input = serde_json::json!({
            "path": "/f",
            "oldText": "old content",
            "newText": "new content"
        });
        let key = identity_key("edit", &input);
        assert!(key.contains("old content"));
    }

    #[test]
    fn test_identity_key_write() {
        let input = serde_json::json!({
            "path": "/output.txt",
            "content": "This is the first 80 characters of the file content"
        });
        let key = identity_key("write", &input);
        assert!(key.starts_with("write:/output.txt:"));
    }

    #[test]
    fn test_identity_key_bash() {
        let input = serde_json::json!({"command": "ls -la"});
        let key = identity_key("bash", &input);
        assert_eq!(key, "bash:ls -la");
    }

    #[test]
    fn test_identity_key_other_tool() {
        let input = serde_json::json!({"url": "https://example.com", "prompt": "test"});
        let key = identity_key("web_fetch", &input);
        assert!(key.contains("web_fetch"));
        assert!(key.contains("example.com"));
    }
}
