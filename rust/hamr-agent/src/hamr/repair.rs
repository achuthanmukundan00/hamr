//! Port of `packages/coding-agent/src/hamr/repair.ts`.
//!
//! Tool-call repair for non-native function-calling models.
//!
//! When a model doesn't emit native `tool_use` blocks, this module inspects
//! the raw text output and extracts tool calls via the parser pipeline.
//!
//! Key exports:
//! - `parser_by_model`: registry mapping provider+model → parser id
//! - `repair_local_tool_calls()`: extract tool calls from assistant text
//! - `has_substantial_content()`: check if an assistant message event
//!   carries meaningful content (for streaming display decisions)
//! - `register_hamr_providers()`: apply startup config provider registrations

use std::collections::HashMap;
use std::sync::Mutex;

// ─── Global parser-by-model registry ─────────────────────────────────────────

/// Global map from provider+model key to parser id.
/// Mirror of `parserByModel` in the TS source.
///
/// Key format: `provider:modelId` (same as `modelKey()` output).
static PARSER_BY_MODEL: std::sync::LazyLock<Mutex<HashMap<String, String>>> =
    std::sync::LazyLock::new(|| Mutex::new(HashMap::new()));

/// Build a model key string: `provider:modelId`.
/// Mirror of `modelKey` from `helpers.ts`.
pub fn model_key(provider: &str, model_id: &str) -> String {
    format!("{}:{}", provider, model_id)
}

/// Set the parser for a given model.
pub fn set_parser_for_model(provider: &str, model_id: &str, parser_id: &str) {
    let mut guard = PARSER_BY_MODEL.lock().unwrap();
    guard.insert(model_key(provider, model_id), parser_id.to_string());
}

/// Get the parser id for a given model, if registered.
pub fn get_parser_for_model(provider: &str, model_id: &str) -> Option<String> {
    let guard = PARSER_BY_MODEL.lock().unwrap();
    guard.get(&model_key(provider, model_id)).cloned()
}

// ─── Parser resolution ───────────────────────────────────────────────────────

/// Resolve the parser id for a message or context.
///
/// Priority:
/// 1. Direct parser-by-model mapping
/// 2. Current model's parser mapping
/// 3. Auto-detection from model id
/// 4. "generic" fallback
///
/// Mirror of `parserFor` in the TS source.
pub fn parser_for(
    message_provider: &str,
    message_model: &str,
    ctx_model_provider: Option<&str>,
    ctx_model_id: Option<&str>,
) -> String {
    // Try direct mapping for the message
    if let Some(parser) = get_parser_for_model(message_provider, message_model) {
        return parser;
    }

    // Try current model's mapping
    if let (Some(provider), Some(model_id)) = (ctx_model_provider, ctx_model_id) {
        if let Some(parser) = get_parser_for_model(provider, model_id) {
            return parser;
        }

        // Try auto-detection from model id
        if let Some(parser) = crate::hamr::providers::parsers::types::detect_parser_id(model_id) {
            return parser.to_string();
        }
    }

    // Try auto-detection from message model
    if let Some(parser) = crate::hamr::providers::parsers::types::detect_parser_id(message_model) {
        return parser.to_string();
    }

    "generic".to_string()
}

// ─── Local tool-call repair ──────────────────────────────────────────────────

/// Attempt to extract tool calls from the assistant message text content.
///
/// When a model doesn't emit native `tool_use` blocks, this inspects the
/// raw text output and runs the tool-call parser pipeline.
///
/// Returns a repaired `AssistantMessage`-like struct with extracted tool calls,
/// or `None` if the message already has tool calls or has no text content.
///
/// Mirror of `repairLocalToolCalls` in the TS source.
///
/// When the full `AssistantMessage` type is ported, the return type will
/// be `Option<AssistantMessage>`.
pub struct RepairedMessage {
    pub content: Vec<RepairedContentBlock>,
    pub stop_reason: String,
    pub diagnostics: Vec<RepairDiagnostic>,
}

/// A content block in a repaired message.
#[derive(Debug, Clone)]
pub enum RepairedContentBlock {
    Thinking {
        thinking: String,
    },
    Text {
        text: String,
    },
    ToolCall {
        id: String,
        name: String,
        arguments: serde_json::Value,
    },
}

/// A diagnostic entry from tool-call repair.
#[derive(Debug, Clone)]
pub struct RepairDiagnostic {
    pub diagnostic_type: String,
    pub timestamp: u64,
    pub details: RepairDiagnosticDetails,
}

#[derive(Debug, Clone)]
pub struct RepairDiagnosticDetails {
    pub source: String,
    pub message: String,
}

/// Check whether content blocks contain any tool calls.
pub fn has_tool_calls(content_blocks: &[RepairedContentBlock]) -> bool {
    content_blocks
        .iter()
        .any(|b| matches!(b, RepairedContentBlock::ToolCall { .. }))
}

/// Extract concatenated text from content blocks (excludes thinking).
pub fn get_assistant_text(content_blocks: &[RepairedContentBlock]) -> String {
    content_blocks
        .iter()
        .filter_map(|b| match b {
            RepairedContentBlock::Text { text } => Some(text.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("")
}

/// Extract concatenated thinking text from content blocks.
pub fn get_thinking_text(content_blocks: &[RepairedContentBlock]) -> Option<String> {
    let thinking: String = content_blocks
        .iter()
        .filter_map(|b| match b {
            RepairedContentBlock::Thinking { thinking } => Some(thinking.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("");
    if thinking.is_empty() {
        None
    } else {
        Some(thinking)
    }
}

/// Attempt to repair local tool calls from raw text + thinking content.
///
/// Mirror of `repairLocalToolCalls` in the TS source.
///
/// Parameters:
/// - `text`: the assistant text content (already extracted from message)
/// - `thinking`: optional reasoning/thinking content
/// - `provider`: message provider name
/// - `model`: message model id
/// - `ctx_provider`: current context model provider (optional)
/// - `ctx_model_id`: current context model id (optional)
///
/// Returns `None` if no tool calls could be extracted.
pub fn repair_local_tool_calls(
    text: &str,
    thinking: Option<&str>,
    provider: &str,
    model: &str,
    ctx_provider: Option<&str>,
    ctx_model_id: Option<&str>,
) -> Option<RepairedMessage> {
    if text.trim().is_empty() && thinking.map(|t| t.trim()).unwrap_or("").is_empty() {
        return None;
    }

    let parser_id = parser_for(provider, model, ctx_provider, ctx_model_id);
    let parsed = super::providers::tool_calls::parse_model_output(text, &parser_id, thinking);

    if parsed.tool_calls.is_empty() && parsed.warnings.is_empty() {
        return None;
    }

    // Build repaired message
    let mut content: Vec<RepairedContentBlock> = Vec::new();

    if let Some(ref reasoning) = parsed.reasoning {
        if !reasoning.is_empty() {
            content.push(RepairedContentBlock::Thinking {
                thinking: reasoning.clone(),
            });
        }
    }

    let assistant_trimmed = parsed.assistant_text.trim();
    if !assistant_trimmed.is_empty() {
        content.push(RepairedContentBlock::Text {
            text: assistant_trimmed.to_string(),
        });
    }

    for call in &parsed.tool_calls {
        content.push(RepairedContentBlock::ToolCall {
            id: call.id.clone(),
            name: call.name.clone(),
            arguments: call.arguments.clone(),
        });
    }

    let diagnostics: Vec<RepairDiagnostic> = parsed
        .warnings
        .iter()
        .map(|w| RepairDiagnostic {
            diagnostic_type: "hamr.tool_call_repair".to_string(),
            timestamp: chrono::Utc::now().timestamp_millis() as u64,
            details: RepairDiagnosticDetails {
                source: format!("{:?}", w.source).to_lowercase(),
                message: w.message.clone(),
            },
        })
        .collect();

    Some(RepairedMessage {
        content,
        stop_reason: "toolUse".to_string(),
        diagnostics,
    })
}

// ─── Streaming content inspection ────────────────────────────────────────────

/// Check whether an assistant message event carries substantial content
/// that should be displayed during streaming.
///
/// Mirror of `hasSubstantialContent` in the TS source.
pub fn has_substantial_content(event_type: &str, delta: Option<&str>) -> bool {
    match event_type {
        "start" | "done" | "error" => false,
        "text_delta" | "thinking_delta" | "toolcall_delta" => {
            delta.map(|d| d.trim().len() > 0).unwrap_or(false)
        }
        "text_start" | "thinking_start" | "toolcall_start" | "text_end" | "thinking_end"
        | "toolcall_end" => true,
        _ => false,
    }
}

// ─── Provider registration ───────────────────────────────────────────────────

/// Register hamr providers from startup config.
///
/// Mirror of `registerHamrProviders` in the TS source.
///
/// When the full startup config and ExtensionAPI types are ported, this will
/// accept `pi: &dyn ExtensionAPI` and `config: &HamrStartupConfig`.
pub async fn register_hamr_providers_stub() {
    // TODO: port when ExtensionAPI and HamrStartupConfig are ready.
    //
    // For each registration from buildHamrProviderRegistrations(config):
    //   1. Set parserByModel mappings
    //   2. Call pi.registerProvider(name, config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_key() {
        assert_eq!(model_key("anthropic", "claude-3"), "anthropic:claude-3");
        assert_eq!(model_key("", ""), ":");
    }

    #[test]
    fn test_parser_for_fallback_to_generic() {
        let result = parser_for("unknown", "unknown-model", None, None);
        assert_eq!(result, "generic");
    }

    #[test]
    fn test_parser_for_auto_detect_qwen() {
        // Auto-detection should find qwen3_xml
        let result = parser_for("relay", "qwen3-coder-7b", None, None);
        assert_eq!(result, "qwen3_xml");
    }

    #[test]
    fn test_parser_for_explicit_mapping() {
        set_parser_for_model("my-provider", "my-model", "custom-parser");
        let result = parser_for("my-provider", "my-model", None, None);
        assert_eq!(result, "custom-parser");
    }

    #[test]
    fn test_parser_for_ctx_model_priority() {
        // Message model doesn't match, but ctx model does — should use ctx model's parser
        set_parser_for_model("ctx-provider", "ctx-model", "ctx-parser");
        let result = parser_for(
            "msg-provider",
            "msg-model",
            Some("ctx-provider"),
            Some("ctx-model"),
        );
        assert_eq!(result, "ctx-parser");
    }

    #[test]
    fn test_has_substantial_content() {
        assert!(!has_substantial_content("start", None));
        assert!(!has_substantial_content("done", None));
        assert!(!has_substantial_content("error", None));

        assert!(has_substantial_content("text_start", None));
        assert!(has_substantial_content("text_end", None));

        assert!(!has_substantial_content("text_delta", Some("   ")));
        assert!(has_substantial_content("text_delta", Some("hello")));

        assert!(!has_substantial_content("unknown_event", None));
    }

    #[test]
    fn test_repair_local_tool_calls_empty() {
        let result = repair_local_tool_calls("", None, "relay", "some-model", None, None);
        assert!(result.is_none());
    }

    #[test]
    fn test_repair_local_tool_calls_no_tool_calls() {
        // With stub parser, this won't find any tool calls
        let result =
            repair_local_tool_calls("hello world", None, "relay", "some-model", None, None);
        // Stub parser returns empty tool_calls and may have error warnings
        // Depending on whether warnings are present, result may be Some or None
    }

    #[test]
    fn test_repair_local_tool_calls_empty_text_thinks() {
        // When both text and thinking are empty/whitespace, returns None
        let result = repair_local_tool_calls("  ", Some("  "), "test", "model", None, None);
        assert!(result.is_none());
    }

    #[test]
    fn test_repair_local_tool_calls_empty_content() {
        // When text is empty and no thinking, returns None
        let result = repair_local_tool_calls("", None, "test", "model", None, None);
        assert!(result.is_none());
    }

    #[test]
    fn test_has_substantial_content_start_done_error() {
        assert!(!has_substantial_content("start", None));
        assert!(!has_substantial_content("done", None));
        assert!(!has_substantial_content("error", None));
    }

    #[test]
    fn test_has_substantial_content_text_blocks() {
        assert!(has_substantial_content("text_start", None));
        assert!(has_substantial_content("text_end", None));
        assert!(has_substantial_content("thinking_start", None));
        assert!(has_substantial_content("toolcall_end", None));
    }

    #[test]
    fn test_has_substantial_content_text_delta_with_content() {
        assert!(!has_substantial_content("text_delta", Some("   ")));
        assert!(has_substantial_content("text_delta", Some("hello")));
    }

    #[test]
    fn test_has_tool_calls_stub() {
        // Stub always returns false
        let blocks = [RepairedContentBlock::Text {
            text: "hello".to_string(),
        }];
        assert!(!has_tool_calls(&blocks));
    }

    #[test]
    fn test_get_assistant_text() {
        let blocks = [RepairedContentBlock::Text {
            text: "hello".to_string(),
        }];
        assert_eq!(get_assistant_text(&blocks), "hello");
    }

    #[test]
    fn test_get_thinking_text() {
        let blocks = [RepairedContentBlock::Thinking {
            thinking: "hmm".to_string(),
        }];
        assert_eq!(get_thinking_text(&blocks), Some("hmm".to_string()));
    }

    #[test]
    fn test_has_tool_calls() {
        let blocks_no_tools = [RepairedContentBlock::Text {
            text: "hello".to_string(),
        }];
        assert!(!has_tool_calls(&blocks_no_tools));

        let blocks_with_tools = [
            RepairedContentBlock::Text { text: "hello".to_string() },
            RepairedContentBlock::ToolCall { id: "1".to_string(), name: "bash".to_string(), arguments: serde_json::json!({}) },
        ];
        assert!(has_tool_calls(&blocks_with_tools));
    }

    #[test]
    fn test_repair_local_tool_calls_text_only_no_thinking() {
        // When text has content but no thinking, and stub parser produces no tool calls
        let result = repair_local_tool_calls(
            "some text content",
            None,
            "relay",
            "qwen3-coder-7b", // auto-detect: qwen3_xml
            None,
            None,
        );
        // With qwen3_xml parser there may be warnings for unparseable content
        // or no warnings; either way it shouldn't panic
    }

    #[test]
    fn test_repair_local_tool_calls_with_thinking_text() {
        // Both text and thinking present, no tool calls in either
        let result = repair_local_tool_calls(
            "plain text",
            Some("thinking content"),
            "relay",
            "qwen3-coder-7b",
            None,
            None,
        );
        // Should not panic with both present
    }

    #[test]
    fn test_repair_diagnostics_format() {
        // Verify the diagnostic struct is populated correctly when warnings exist
        let result = repair_local_tool_calls(
            "some random text that should trigger parser warnings",
            None,
            "relay",
            "qwen3-coder-7b",
            None,
            None,
        );
        if let Some(ref msg) = result {
            assert_eq!(msg.stop_reason, "toolUse");
            for diag in &msg.diagnostics {
                assert_eq!(diag.diagnostic_type, "hamr.tool_call_repair");
            }
        }
    }

    #[test]
    fn test_model_key_edge_cases() {
        assert_eq!(model_key("provider", ""), "provider:");
        assert_eq!(model_key("", "model"), ":model");
    }

    #[test]
    fn test_set_parser_for_model_overrides() {
        set_parser_for_model("test", "m1", "parser-a");
        assert_eq!(
            get_parser_for_model("test", "m1"),
            Some("parser-a".to_string())
        );

        // Override with new parser
        set_parser_for_model("test", "m1", "parser-b");
        assert_eq!(
            get_parser_for_model("test", "m1"),
            Some("parser-b".to_string())
        );
    }

    #[test]
    fn test_get_parser_for_model_missing() {
        assert_eq!(get_parser_for_model("nonexistent", "nope"), None);
    }
}
