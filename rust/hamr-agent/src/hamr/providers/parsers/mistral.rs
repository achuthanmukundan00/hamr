//! Mistral tool-call parser.
//!
//! Parses Mistral-format tool calls:
//!
//! ```text
//! [TOOL_CALLS][{"name": "get_weather", "arguments": {"location": "SF", "unit": "celsius"}}]
//! ```
//!
//! The model outputs a `[TOOL_CALLS]` prefix followed by a JSON array of
//! tool call objects. Each object has `"name"` and `"arguments"` fields.
//! Multiple calls can appear in the same JSON array.
//!
//! Variants:
//!   - mistral: standard Mistral format
//!
//! Reference: vLLM docs/features/tool_calling.md
//!   --tool-call-parser mistral
//!   vllm/entrypoints/openai/tool_parsers/mistral_tool_parser.py

use super::types::{ParsedToolCall, ToolCallParseResult, ToolCallParser};
use super::utils::{generate_call_id, safe_json_parse, sanitize_reasoning_tags};

const PARSER_ID: &str = "mistral";
const DESCRIPTION: &str =
    "Mistral format: [TOOL_CALLS][{\"name\":\"...\",\"arguments\":{...}}, ...]";
const MODEL_FAMILIES: &[&str] = &["Mistral", "Mixtral", "Mistral Nemo", "Codestral"];

const TOOL_CALLS_PREFIX: &str = "[TOOL_CALLS]";

// ─── Parser struct ────────────────────────────────────────

pub struct MistralParser;

impl MistralParser {
    pub fn new() -> Self {
        Self
    }
}

impl Default for MistralParser {
    fn default() -> Self {
        Self::new()
    }
}

impl ToolCallParser for MistralParser {
    fn id(&self) -> &str {
        PARSER_ID
    }

    fn description(&self) -> &str {
        DESCRIPTION
    }

    fn model_families(&self) -> &[&str] {
        MODEL_FAMILIES
    }

    fn parse(&self, content: &str) -> ToolCallParseResult {
        let sanitized = sanitize_reasoning_tags(content);

        let prefix_idx = match sanitized.find(TOOL_CALLS_PREFIX) {
            Some(idx) => idx,
            None => {
                return ToolCallParseResult::ok(PARSER_ID, vec![], &sanitized);
            }
        };

        let before = sanitized[..prefix_idx].trim().to_string();
        let after_prefix = sanitized[prefix_idx + TOOL_CALLS_PREFIX.len()..]
            .trim()
            .to_string();

        if !after_prefix.starts_with('[') {
            return ToolCallParseResult::err(
                PARSER_ID,
                &sanitized,
                "Mistral: [TOOL_CALLS] not followed by JSON array",
            );
        }

        // Find matching closing bracket
        let close_idx = find_balanced_close(&after_prefix, '[', ']');
        let json_str = if close_idx != usize::MAX {
            after_prefix[..=close_idx].to_string()
        } else {
            after_prefix.clone()
        };
        let after_content = if close_idx != usize::MAX {
            after_prefix[close_idx + 1..].trim().to_string()
        } else {
            String::new()
        };

        let parsed = match safe_json_parse(&json_str) {
            Ok(v) => v,
            Err(e) => {
                return ToolCallParseResult::err(PARSER_ID, &sanitized, format!("Mistral: {}", e));
            }
        };

        let json_array = match parsed.as_array() {
            Some(arr) => arr,
            None => {
                return ToolCallParseResult::err(
                    PARSER_ID,
                    &sanitized,
                    "Mistral: [TOOL_CALLS] content is not a JSON array",
                );
            }
        };

        let mut calls: Vec<ParsedToolCall> = Vec::new();
        let mut warnings: Vec<String> = Vec::new();

        for (i, item) in json_array.iter().enumerate() {
            let obj = match item.as_object() {
                Some(o) => o,
                None => {
                    warnings.push(format!("Mistral item {}: not a JSON object, skipping", i));
                    continue;
                }
            };

            // Extract name: check "name", "tool_name", or "function" fields
            let name = obj
                .get("name")
                .or_else(|| obj.get("tool_name"))
                .or_else(|| obj.get("function"));

            let name_str = match name {
                Some(serde_json::Value::String(s)) => s.trim().to_string(),
                _ => {
                    warnings.push(format!("Mistral item {}: missing \"name\", skipping", i));
                    continue;
                }
            };

            if name_str.is_empty() {
                warnings.push(format!("Mistral item {}: missing \"name\", skipping", i));
                continue;
            }

            // Extract arguments: check "arguments", "parameters", or "input"
            let raw_args = obj
                .get("arguments")
                .or_else(|| obj.get("parameters"))
                .or_else(|| obj.get("input"));

            let args = if let Some(raw) = raw_args {
                match raw {
                    serde_json::Value::String(s) => match safe_json_parse(s) {
                        Ok(parsed @ serde_json::Value::Object(_)) => parsed.clone(),
                        _ => {
                            warnings
                                .push(format!("Mistral item {}: arguments string parse failed", i));
                            serde_json::Value::Object(serde_json::Map::new())
                        }
                    },
                    serde_json::Value::Object(_) => raw.clone(),
                    _ => serde_json::Value::Object(serde_json::Map::new()),
                }
            } else {
                serde_json::Value::Object(serde_json::Map::new())
            };

            // Generate call id
            let call_id = obj
                .get("id")
                .or_else(|| obj.get("call_id"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            let raw_source = Some(serde_json::to_string(item).unwrap_or_else(|_| String::new()));

            let call = ParsedToolCall {
                id: generate_call_id(call_id.as_deref(), Some(i + 1)),
                name: name_str,
                arguments: args,
                raw_source,
                parser_id: Some(PARSER_ID.to_string()),
                warnings: None,
            };

            calls.push(call);
        }

        // Build non-tool content from before and after
        let non_tool_content = if before.is_empty() && after_content.is_empty() {
            String::new()
        } else {
            let parts: Vec<&str> = [&before, &after_content]
                .iter()
                .filter(|s| !s.is_empty())
                .map(|s| s.as_str())
                .collect();
            parts.join("\n")
        };

        ToolCallParseResult {
            ok: true,
            parser_id: PARSER_ID.to_string(),
            calls,
            content: non_tool_content,
            error: None,
            warnings: if warnings.is_empty() {
                None
            } else {
                Some(warnings)
            },
        }
    }
}

// ─── Balanced-bracket finder ──────────────────────────────

/// Find the index of the closing bracket that balances the opening bracket,
/// accounting for string contents in JSON.
///
/// Returns `usize::MAX` if no balanced close is found.
fn find_balanced_close(text: &str, open: char, close: char) -> usize {
    let mut depth: i32 = 0;
    let mut in_string = false;
    let mut escape = false;

    let chars: Vec<char> = text.chars().collect();
    for i in 0..chars.len() {
        let ch = chars[i];
        if escape {
            escape = false;
            continue;
        }
        if ch == '\\' && in_string {
            escape = true;
            continue;
        }
        if ch == '"' {
            in_string = !in_string;
            continue;
        }
        if in_string {
            continue;
        }
        if ch == open {
            depth += 1;
        } else if ch == close {
            depth -= 1;
            if depth == 0 {
                return i;
            }
        }
    }

    usize::MAX
}

// ─── Factory ──────────────────────────────────────────────

/// Create a boxed `MistralParser`.
pub fn create_mistral_parser() -> Box<dyn ToolCallParser> {
    Box::new(MistralParser::new())
}

// ─── Tests ────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hamr::providers::parsers::utils::reset_call_id_counter;

    #[test]
    fn test_no_tool_calls_returns_ok_with_empty_calls() {
        let parser = MistralParser::new();
        let result = parser.parse("Hello, how can I help you?");
        assert!(result.ok);
        assert!(result.calls.is_empty());
        assert_eq!(result.content, "Hello, how can I help you?");
    }

    #[test]
    fn test_single_tool_call() {
        reset_call_id_counter();
        let parser = MistralParser::new();
        let input = r#"What's the weather? [TOOL_CALLS][{"name": "get_weather", "arguments": {"location": "San Francisco", "unit": "celsius"}}]"#;
        let result = parser.parse(input);
        assert!(result.ok);
        assert_eq!(result.calls.len(), 1);
        assert_eq!(result.calls[0].name, "get_weather");
        assert_eq!(
            result.calls[0].arguments,
            serde_json::json!({"location": "San Francisco", "unit": "celsius"})
        );
        assert_eq!(result.calls[0].parser_id.as_deref(), Some("mistral"));
        assert!(result.calls[0].raw_source.is_some());
        assert_eq!(result.content, "What's the weather?");
    }

    #[test]
    fn test_multiple_tool_calls() {
        reset_call_id_counter();
        let parser = MistralParser::new();
        let input = r#"[TOOL_CALLS][{"name": "get_weather", "arguments": {"location": "SF"}}, {"name": "get_time", "arguments": {"timezone": "PST"}}]"#;
        let result = parser.parse(input);
        assert!(result.ok);
        assert_eq!(result.calls.len(), 2);
        assert_eq!(result.calls[0].name, "get_weather");
        assert_eq!(result.calls[1].name, "get_time");
        assert!(result.content.is_empty());
    }

    #[test]
    fn test_tool_calls_not_followed_by_array_returns_error() {
        let parser = MistralParser::new();
        let input = "[TOOL_CALLS] hello";
        let result = parser.parse(input);
        assert!(!result.ok);
        assert!(
            result
                .error
                .as_deref()
                .unwrap()
                .contains("not followed by JSON array")
        );
    }

    #[test]
    fn test_non_array_json_returns_error() {
        let parser = MistralParser::new();
        let input = r#"[TOOL_CALLS]{"name": "test"}"#;
        let result = parser.parse(input);
        assert!(!result.ok);
        assert!(
            result
                .error
                .as_deref()
                .unwrap()
                .contains("not followed by JSON array")
        );
    }

    #[test]
    fn test_item_missing_name_skipped_with_warning() {
        let parser = MistralParser::new();
        let input = r#"[TOOL_CALLS][{"name": "valid"}, {"no_name": "test"}]"#;
        let result = parser.parse(input);
        assert!(result.ok);
        assert_eq!(result.calls.len(), 1);
        assert_eq!(result.calls[0].name, "valid");
        assert!(result.warnings.is_some());
        assert!(result.warnings.as_ref().unwrap()[0].contains("missing \"name\""));
    }

    #[test]
    fn test_fallback_field_names() {
        reset_call_id_counter();
        let parser = MistralParser::new();
        // tool_name variant
        let input = r#"[TOOL_CALLS][{"tool_name": "hello", "arguments": {"x": 1}}]"#;
        let result = parser.parse(input);
        assert!(result.ok);
        assert_eq!(result.calls[0].name, "hello");

        // function variant
        let input2 = r#"[TOOL_CALLS][{"function": "hello", "arguments": {"x": 1}}]"#;
        let result2 = parser.parse(input2);
        assert!(result2.ok);
        assert_eq!(result2.calls[0].name, "hello");
    }

    #[test]
    fn test_arguments_field_fallbacks() {
        reset_call_id_counter();
        let parser = MistralParser::new();
        // parameters field
        let input = r#"[TOOL_CALLS][{"name": "hello", "parameters": {"x": 1}}]"#;
        let result = parser.parse(input);
        assert!(result.ok);
        assert_eq!(result.calls[0].arguments, serde_json::json!({"x": 1}));

        // input field
        let input2 = r#"[TOOL_CALLS][{"name": "hello", "input": {"y": 2}}]"#;
        let result2 = parser.parse(input2);
        assert!(result2.ok);
        assert_eq!(result2.calls[0].arguments, serde_json::json!({"y": 2}));
    }

    #[test]
    fn test_arguments_as_string() {
        reset_call_id_counter();
        let parser = MistralParser::new();
        let input = r#"[TOOL_CALLS][{"name": "hello", "arguments": "{\"x\": 1}"}]"#;
        let result = parser.parse(input);
        assert!(result.ok);
        assert_eq!(result.calls[0].arguments, serde_json::json!({"x": 1}));
    }

    #[test]
    fn test_arguments_string_parse_failure_warns() {
        reset_call_id_counter();
        let parser = MistralParser::new();
        let input = r#"[TOOL_CALLS][{"name": "hello", "arguments": "not-json"}]"#;
        let result = parser.parse(input);
        assert!(result.ok);
        assert!(result.warnings.is_some());
        assert!(result.warnings.as_ref().unwrap()[0].contains("arguments string parse failed"));
    }

    #[test]
    fn test_call_id_from_field() {
        reset_call_id_counter();
        let parser = MistralParser::new();
        let input = r#"[TOOL_CALLS][{"id": "custom_123", "name": "hello", "arguments": {}}]"#;
        let result = parser.parse(input);
        assert!(result.ok);
        assert_eq!(result.calls[0].id, "custom_123");

        // call_id variant
        let input2 = r#"[TOOL_CALLS][{"call_id": "custom_456", "name": "hello", "arguments": {}}]"#;
        let result2 = parser.parse(input2);
        assert!(result2.ok);
        assert_eq!(result2.calls[0].id, "custom_456");
    }

    #[test]
    fn test_reasoning_tags_sanitized() {
        let parser = MistralParser::new();
        let input = "<think>thinking</think>[TOOL_CALLS][{\"name\": \"test\", \"arguments\": {}}]";
        let result = parser.parse(input);
        assert!(result.ok);
        assert!(!result.content.contains("<think"));
    }

    #[test]
    fn test_before_and_after_content() {
        let parser = MistralParser::new();
        let input =
            "Before text. [TOOL_CALLS][{\"name\": \"test\", \"arguments\": {}}] After text.";
        let result = parser.parse(input);
        assert!(result.ok);
        assert_eq!(result.calls.len(), 1);
        assert!(result.content.contains("Before text."));
        assert!(result.content.contains("After text."));
    }

    #[test]
    fn test_find_balanced_close() {
        assert_eq!(find_balanced_close("[hello]", '[', ']'), 6);
        assert_eq!(find_balanced_close("[[nested]]", '[', ']'), 9);
        assert_eq!(find_balanced_close(r#"["hello]world"]"#, '[', ']'), 14);
        // Unbalanced
        assert_eq!(find_balanced_close("[hello", '[', ']'), usize::MAX);
    }

    #[test]
    fn test_empty_content() {
        let parser = MistralParser::new();
        let result = parser.parse("");
        assert!(result.ok);
        assert!(result.calls.is_empty());
        assert!(result.content.is_empty());
    }

    #[test]
    fn test_only_tool_call_block() {
        reset_call_id_counter();
        let parser = MistralParser::new();
        let input = r#"[TOOL_CALLS][{"name": "test", "arguments": {}}]"#;
        let result = parser.parse(input);
        assert!(result.ok);
        assert_eq!(result.calls.len(), 1);
        assert!(result.content.is_empty());
    }

    #[test]
    fn test_malformed_json_still_returns_error() {
        let parser = MistralParser::new();
        let input = r#"[TOOL_CALLS][{"name": "test", "arguments": {bad"#;
        let result = parser.parse(input);
        assert!(!result.ok);
    }
}
