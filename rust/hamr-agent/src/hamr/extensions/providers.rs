//! Port of `packages/coding-agent/src/hamr/extensions/providers.ts`.
//!
//! Implements tool-call parsing and repair for non-native function-calling models.
//! Registered as a built-in extension.
//!
//! # Architecture
//!
//! Registers:
//! - `message_start` handler — cold-start timeout detection for relay models
//! - `message_update` handler — cold-start clear + rainbow frames
//! - `message_end` handler — repair local-model tool calls
//! - `turn_end` handler — surface turn failures as error notifications
//! - `session_shutdown` handler — clean up cold-start state
//!
//! Provider registration is done via `pi.register_provider()` based on
//! the hamr startup config (TOML).

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::core::extensions::types::*;

// ─── Constants ───────────────────────────────────────────────────────────────

const COLD_START_TIMEOUT_MS: u64 = 5_000;

// ─── Helpers ─────────────────────────────────────────────────────────────────

/// Condense a raw turn-error message (which may be a 502 HTML page) to one line.
fn summarize_turn_error(raw: Option<&str>) -> String {
    let raw = match raw {
        Some(s) => s,
        None => return "unknown error".to_string(),
    };
    // Strip HTML tags
    let stripped = raw
        .chars()
        .fold((String::new(), false), |(mut acc, in_tag), c| {
            match (in_tag, c) {
                (_, '<') => (acc, true),
                (true, '>') => (acc, false),
                (true, _) => (acc, true),
                (false, _) => {
                    acc.push(c);
                    (acc, false)
                }
            }
        })
        .0
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    let text = if stripped.is_empty() {
        raw.trim()
    } else {
        &stripped
    };
    if text.len() > 200 {
        format!("{}...", &text[..197])
    } else {
        text.to_string()
    }
}

// ─── Extension ───────────────────────────────────────────────────────────────

/// Extension name constant.
pub const EXTENSION_NAME: &str = "hamr-providers";

/// Creates the hamr providers extension.
///
/// Port of `hamrProvidersExtension` in the TS source.
///
/// Registers hamr's configured providers, repairs local-model tool calls,
/// drives the relay cold-start indicator, and surfaces turn failures.
pub fn hamr_providers_extension() -> ExtensionFactory {
    Arc::new(|pi: Arc<dyn ExtensionAPI>| {
        Box::pin(async move {
            let has_received_content = Arc::new(AtomicBool::new(false));

            // Message start cold-start detection
            let _pi_clone = pi.clone();
            let has_content_clone = has_received_content.clone();
            pi.on(
                "message_start",
                Arc::new(
                    move |event: serde_json::Value, _ctx: Arc<dyn ExtensionContext>| {
                        let has_content = has_content_clone.clone();
                        Box::pin(async move {
                            let is_assistant = event
                                .get("message")
                                .and_then(|m| m.get("role"))
                                .and_then(|r| r.as_str())
                                == Some("assistant");
                            if is_assistant {
                                has_content.store(false, Ordering::Relaxed);
                                // Cold-start timer logic — in full impl, this sets
                                // a timer that fires a working indicator after 5s.
                                // TODO: implement timer-based cold start detection
                                // when tokio timers / UI context is available.
                            }
                            None
                        })
                    },
                ),
            );

            // Message update: clear cold-start, set rainbow frames
            let has_content_for_update = has_received_content.clone();
            let _pi_for_update = pi.clone();
            pi.on(
                "message_update",
                Arc::new(
                    move |event: serde_json::Value, _ctx: Arc<dyn ExtensionContext>| {
                        let has_content = has_content_for_update.clone();
                        Box::pin(async move {
                            let is_assistant = event
                                .get("message")
                                .and_then(|m| m.get("role"))
                                .and_then(|r| r.as_str())
                                == Some("assistant");
                            if !is_assistant {
                                return None;
                            }

                            // Check for substantial content — port of hasSubstantialContent
                            let assistant_event = event.get("assistantMessageEvent");
                            let has_substantial = assistant_event
                                .and_then(|e| e.get("type"))
                                .and_then(|t| t.as_str())
                                .map(|t| t == "content_block_delta" || t == "content_block_start")
                                .unwrap_or(false);
                            // Also check if there's a text delta
                            let has_text = assistant_event
                                .and_then(|e| e.get("delta"))
                                .and_then(|d| d.get("text"))
                                .and_then(|t| t.as_str())
                                .map(|s| !s.is_empty())
                                .unwrap_or(false);

                            if has_substantial || has_text {
                                has_content.store(true, Ordering::Relaxed);
                                // TODO: set working indicator frames via UI
                            }

                            None
                        })
                    },
                ),
            );

            // Session shutdown cleanup
            pi.on(
                "session_shutdown",
                Arc::new(
                    |_event: serde_json::Value, _ctx: Arc<dyn ExtensionContext>| {
                        Box::pin(async move {
                            // Cleanup is handled by dropping atomics
                            None
                        })
                    },
                ),
            );

            // Message end: repair local-model tool calls
            pi.on(
                "message_end",
                Arc::new(
                    |event: serde_json::Value, _ctx: Arc<dyn ExtensionContext>| {
                        Box::pin(async move {
                            let is_assistant = event
                                .get("message")
                                .and_then(|m| m.get("role"))
                                .and_then(|r| r.as_str())
                                == Some("assistant");
                            if !is_assistant {
                                return None;
                            }

                            // Use the repair module to fix tool calls
                            let message = event.get("message")?;
                            let content = message.get("content")?.as_array()?;

                            // Extract text and thinking from content blocks
                            let mut text = String::new();
                            let mut thinking = String::new();
                            for block in content {
                                match block.get("type").and_then(|t| t.as_str()) {
                                    Some("text") => {
                                        if let Some(t) = block.get("text").and_then(|v| v.as_str()) {
                                            text.push_str(t);
                                        }
                                    }
                                    Some("thinking") => {
                                        if let Some(t) = block.get("thinking").and_then(|v| v.as_str()) {
                                            thinking.push_str(t);
                                        }
                                    }
                                    _ => {}
                                }
                            }

                            if text.is_empty() && thinking.is_empty() {
                                return None;
                            }

                            // Get provider/model from message or context
                            let provider = message
                                .get("provider")
                                .and_then(|v| v.as_str())
                                .unwrap_or("");
                            let model = message
                                .get("model")
                                .and_then(|v| v.as_str())
                                .unwrap_or("");

                            let repaired = crate::hamr::repair::repair_local_tool_calls(
                                &text,
                                if thinking.is_empty() { None } else { Some(&thinking) },
                                provider,
                                model,
                                None,
                                None,
                            );

                            if let Some(msg) = repaired {
                                // Convert repaired message back to the event format
                                let mut new_content = serde_json::json!([]);
                                let arr = new_content.as_array_mut().unwrap();
                                for block in &msg.content {
                                    match block {
                                        crate::hamr::repair::RepairedContentBlock::Text { text } => {
                                            arr.push(serde_json::json!({"type": "text", "text": text}));
                                        }
                                        crate::hamr::repair::RepairedContentBlock::Thinking { thinking } => {
                                            arr.push(serde_json::json!({"type": "thinking", "thinking": thinking}));
                                        }
                                        crate::hamr::repair::RepairedContentBlock::ToolCall { id, name, arguments } => {
                                            arr.push(serde_json::json!({
                                                "type": "tool_use",
                                                "id": id,
                                                "name": name,
                                                "input": arguments
                                            }));
                                        }
                                    }
                                }
                                Some(serde_json::json!({"message": {"content": new_content}}))
                            } else {
                                None
                            }
                        })
                    },
                ),
            );

            // Turn end: surface turn failures
            pi.on(
                "turn_end",
                Arc::new(|event: serde_json::Value, ctx: Arc<dyn ExtensionContext>| {
                    Box::pin(async move {
                        let message = event.get("message")?;
                        let role = message.get("role").and_then(|r| r.as_str())?;
                        if role != "assistant" {
                            return None;
                        }
                        let stop_reason = message
                            .get("stopReason")
                            .and_then(|r| r.as_str())
                            .unwrap_or("");
                        if stop_reason == "error" {
                            let error_msg = message.get("errorMessage").and_then(|e| e.as_str());
                            let summary = summarize_turn_error(error_msg);
                            ctx.ui().notify(
                                &format!("Model request failed: {}", summary),
                                Some("error"),
                            );
                        }
                        None
                    })
                }),
            );
        })
    })
}
