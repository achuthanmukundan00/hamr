//! Hermes tool-call parser.
//!
//! Parses Hermes-format tool calls used by NousResearch Hermes models,
//! Qwen2.5 models, and other Hermes-family models:
//!
//! ```text
//! <tool_call>
//! {"name": "get_weather", "arguments": {"location": "SF", "unit": "celsius"}}
//! </tool_call>
//! ```
//!
//! Each `<tool_call>` block contains a single JSON object with
//! `"name"` and `"arguments"` fields. Multiple blocks = multiple calls.
//!
//! Reference: vLLM docs/features/tool_calling.md → "Qwen Models"
//!   Qwen2.5 chat templates support Hermes-style tool use.
//!   vllm/entrypoints/openai/tool_parsers/hermes_tool_parser.py

use super::types::{ParsedToolCall, ToolCallParseResult, ToolCallParser};
use super::utils::{
    extract_delimited_blocks, generate_call_id, safe_json_parse, sanitize_reasoning_tags,
};

// ─── Parser constants ────────────────────────────────────

const PARSER_ID: &str = "hermes";
const DESCRIPTION: &str =
    "Hermes / Qwen2.5 format: <tool_call>{\"name\":\"...\",\"arguments\":{...}}</tool_call>";
const MODEL_FAMILIES: &[&str] = &["Hermes", "NousResearch Hermes", "OpenHermes", "Qwen2.5"];
const OPEN_TAG: &str = "<tool_call>";
const CLOSE_TAG: &str = "</tool_call>";

// ─── Parser implementation ───────────────────────────────

pub struct HermesParser;

impl HermesParser {
    pub fn new() -> Self {
        Self
    }
}

impl Default for HermesParser {
    fn default() -> Self {
        Self::new()
    }
}

impl ToolCallParser for HermesParser {
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
        let delimited = extract_delimited_blocks(&sanitized, OPEN_TAG, CLOSE_TAG);

        if delimited.blocks.is_empty() {
            return ToolCallParseResult::ok(PARSER_ID, vec![], sanitized);
        }

        let mut calls: Vec<ParsedToolCall> = Vec::new();
        let mut global_warnings: Vec<String> = Vec::new();

        for (i, block) in delimited.blocks.iter().enumerate() {
            let trimmed = block.trim();
            if trimmed.is_empty() {
                continue;
            }

            let parsed = match safe_json_parse(trimmed) {
                Ok(v) => v,
                Err(e) => {
                    return ToolCallParseResult::err(
                        PARSER_ID,
                        sanitized.clone(),
                        format!("Hermes tool_call block {}: {}", i + 1, e),
                    );
                }
            };

            // Must be a JSON object
            match &parsed {
                serde_json::Value::Object(_) => {}
                _ => {
                    return ToolCallParseResult::err(
                        PARSER_ID,
                        sanitized.clone(),
                        format!(
                            "Hermes tool_call block {}: expected JSON object, got {}",
                            i + 1,
                            match parsed {
                                serde_json::Value::Null => "null",
                                serde_json::Value::Bool(_) => "boolean",
                                serde_json::Value::Number(_) => "number",
                                serde_json::Value::String(_) => "string",
                                serde_json::Value::Array(_) => "array",
                                serde_json::Value::Object(_) => unreachable!(),
                            }
                        ),
                    );
                }
            }

            let obj = parsed.as_object().unwrap();

            // Extract name with fallbacks
            let name = obj
                .get("name")
                .or_else(|| obj.get("tool_name"))
                .or_else(|| obj.get("function"));
            let name = match name {
                Some(serde_json::Value::String(s)) if !s.trim().is_empty() => s.trim().to_string(),
                _ => {
                    return ToolCallParseResult::err(
                        PARSER_ID,
                        sanitized.clone(),
                        format!("Hermes tool_call block {}: missing \"name\" field", i + 1),
                    );
                }
            };

            // Extract arguments with fallbacks
            let raw_args = obj
                .get("arguments")
                .or_else(|| obj.get("parameters"))
                .or_else(|| obj.get("input"))
                .or_else(|| obj.get("args"));

            let args: serde_json::Value = match raw_args {
                // String: try to parse as JSON
                Some(serde_json::Value::String(s)) => match safe_json_parse(s) {
                    Ok(serde_json::Value::Object(_)) => serde_json::Value::Object(
                        safe_json_parse(s)
                            .ok()
                            .and_then(|v| {
                                if let serde_json::Value::Object(map) = v {
                                    Some(map)
                                } else {
                                    None
                                }
                            })
                            .unwrap_or_default(),
                    ),
                    _ => {
                        global_warnings.push(format!(
                            "Hermes block {}: arguments string could not be parsed as JSON object",
                            i + 1
                        ));
                        serde_json::Value::Object(serde_json::Map::new())
                    }
                },
                // Object: use directly
                Some(v @ serde_json::Value::Object(_)) => v.clone(),
                // Missing or wrong type: empty object
                _ => serde_json::Value::Object(serde_json::Map::new()),
            }
            // Ensure we always have an Object
            .as_object()
            .cloned()
            .map(serde_json::Value::Object)
            .unwrap_or_else(|| serde_json::Value::Object(serde_json::Map::new()));

            // Generate call id from model-provided id or call_id field
            let model_id = obj
                .get("id")
                .or_else(|| obj.get("call_id"))
                .and_then(|v| v.as_str());

            let per_call_warnings: Vec<String> = global_warnings.drain(..).collect();
            let warnings = if per_call_warnings.is_empty() {
                None
            } else {
                Some(per_call_warnings)
            };

            let call = ParsedToolCall {
                id: generate_call_id(model_id, Some(i + 1)),
                name,
                arguments: args,
                raw_source: Some(trimmed.to_string()),
                parser_id: Some(PARSER_ID.to_string()),
                warnings,
            };
            calls.push(call);
        }

        // Non-tool content: join before + between + after, filter empty, trim
        let non_tool_parts: Vec<&str> = [
            delimited.before.as_str(),
            // inter-block segments
        ]
        .into_iter()
        .chain(delimited.between.iter().map(|s| s.as_str()))
        .chain(std::iter::once(delimited.after.as_str()))
        .filter(|s| !s.trim().is_empty())
        .collect();

        let non_tool_content = if non_tool_parts.is_empty() {
            String::new()
        } else {
            non_tool_parts.join("\n").trim().to_string()
        };

        ToolCallParseResult::ok(PARSER_ID, calls, non_tool_content)
    }
}

// ─── Factory ─────────────────────────────────────────────

/// Create a new boxed Hermes parser.
pub fn create_hermes_parser() -> Box<dyn ToolCallParser> {
    Box::new(HermesParser::new())
}

// ─── Tests ───────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_content() {
        let parser = HermesParser::new();
        let result = parser.parse("");
        assert!(result.ok);
        assert_eq!(result.calls.len(), 0);
        assert!(result.content.is_empty());
    }

    #[test]
    fn test_no_tool_calls() {
        let parser = HermesParser::new();
        let result = parser.parse("Hello world");
        assert!(result.ok);
        assert_eq!(result.calls.len(), 0);
        assert_eq!(result.content, "Hello world");
    }

    #[test]
    fn test_single_tool_call_with_object_args() {
        let parser = HermesParser::new();
        let input = r#"<tool_call>{"name": "get_weather", "arguments": {"location": "SF", "unit": "celsius"}}</tool_call>"#;
        let result = parser.parse(input);
        assert!(result.ok);
        assert_eq!(result.calls.len(), 1);
        let call = &result.calls[0];
        assert_eq!(call.name, "get_weather");
        assert_eq!(call.arguments["location"], "SF");
        assert_eq!(call.arguments["unit"], "celsius");
        assert!(result.content.is_empty());
    }

    #[test]
    fn test_single_tool_call_with_string_args() {
        let parser = HermesParser::new();
        let input = r#"<tool_call>{"name": "get_weather", "arguments": "{\"location\": \"SF\"}"}</tool_call>"#;
        let result = parser.parse(input);
        assert!(result.ok);
        assert_eq!(result.calls.len(), 1);
        let call = &result.calls[0];
        assert_eq!(call.name, "get_weather");
        assert_eq!(call.arguments["location"], "SF");
    }

    #[test]
    fn test_string_args_parse_failure_produces_warning() {
        let parser = HermesParser::new();
        let input = r#"<tool_call>{"name": "get_weather", "arguments": "not-json"}</tool_call>"#;
        let result = parser.parse(input);
        assert!(result.ok);
        assert_eq!(result.calls.len(), 1);
        let call = &result.calls[0];
        assert_eq!(call.name, "get_weather");
        assert!(call.warnings.is_some());
        let warnings = call.warnings.as_ref().unwrap();
        assert!(warnings[0].contains("arguments string could not be parsed"));
    }

    #[test]
    fn test_multiple_tool_calls() {
        let parser = HermesParser::new();
        let input = concat!(
            r#"<tool_call>{"name": "get_weather", "arguments": {"location": "SF"}}</tool_call>"#,
            "some text between",
            r#"<tool_call>{"name": "get_time", "arguments": {"tz": "PST"}}</tool_call>"#,
        );
        let result = parser.parse(input);
        assert!(result.ok);
        assert_eq!(result.calls.len(), 2);
        assert_eq!(result.calls[0].name, "get_weather");
        assert_eq!(result.calls[1].name, "get_time");
        assert!(result.content.contains("some text between"));
    }

    #[test]
    fn test_fallback_tool_name() {
        let parser = HermesParser::new();
        let input = r#"<tool_call>{"tool_name": "get_weather", "arguments": {}}</tool_call>"#;
        let result = parser.parse(input);
        assert!(result.ok);
        assert_eq!(result.calls.len(), 1);
        assert_eq!(result.calls[0].name, "get_weather");
    }

    #[test]
    fn test_fallback_function() {
        let parser = HermesParser::new();
        let input = r#"<tool_call>{"function": "get_weather", "arguments": {}}</tool_call>"#;
        let result = parser.parse(input);
        assert!(result.ok);
        assert_eq!(result.calls.len(), 1);
        assert_eq!(result.calls[0].name, "get_weather");
    }

    #[test]
    fn test_fallback_parameters() {
        let parser = HermesParser::new();
        let input =
            r#"<tool_call>{"name": "get_weather", "parameters": {"location": "SF"}}</tool_call>"#;
        let result = parser.parse(input);
        assert!(result.ok);
        assert_eq!(result.calls[0].arguments["location"], "SF");
    }

    #[test]
    fn test_fallback_input() {
        let parser = HermesParser::new();
        let input =
            r#"<tool_call>{"name": "get_weather", "input": {"location": "SF"}}</tool_call>"#;
        let result = parser.parse(input);
        assert!(result.ok);
        assert_eq!(result.calls[0].arguments["location"], "SF");
    }

    #[test]
    fn test_fallback_args() {
        let parser = HermesParser::new();
        let input = r#"<tool_call>{"name": "get_weather", "args": {"location": "SF"}}</tool_call>"#;
        let result = parser.parse(input);
        assert!(result.ok);
        assert_eq!(result.calls[0].arguments["location"], "SF");
    }

    #[test]
    fn test_provided_call_id() {
        let parser = HermesParser::new();
        let input =
            r#"<tool_call>{"name": "foo", "arguments": {}, "id": "call_abc123"}</tool_call>"#;
        let result = parser.parse(input);
        assert!(result.ok);
        assert_eq!(result.calls[0].id, "call_abc123");
    }

    #[test]
    fn test_provided_call_id_fallback() {
        let parser = HermesParser::new();
        let input =
            r#"<tool_call>{"name": "foo", "arguments": {}, "call_id": "tool_42"}</tool_call>"#;
        let result = parser.parse(input);
        assert!(result.ok);
        assert_eq!(result.calls[0].id, "tool_42");
    }

    #[test]
    fn test_missing_name_returns_error() {
        let parser = HermesParser::new();
        let input = r#"<tool_call>{"arguments": {}}</tool_call>"#;
        let result = parser.parse(input);
        assert!(!result.ok);
        assert!(result.error.unwrap().contains("missing \"name\" field"));
    }

    #[test]
    fn test_non_object_json_returns_error() {
        let parser = HermesParser::new();
        let input = r#"<tool_call>"string value"</tool_call>"#;
        let result = parser.parse(input);
        assert!(!result.ok);
        assert!(result.error.unwrap().contains("expected JSON object"));
    }

    #[test]
    fn test_invalid_json_returns_error() {
        let parser = HermesParser::new();
        let input = r#"<tool_call>{bad json}</tool_call>"#;
        let result = parser.parse(input);
        assert!(!result.ok);
        assert!(result.error.unwrap().contains("could not parse"));
    }

    #[test]
    fn test_array_json_returns_error() {
        let parser = HermesParser::new();
        let input = r#"<tool_call>[{"name": "foo", "arguments": {}}]</tool_call>"#;
        let result = parser.parse(input);
        assert!(!result.ok);
        // Should fail because top-level is array, not object
    }

    #[test]
    fn test_surrounding_text_extracted_as_content() {
        let parser = HermesParser::new();
        let input = concat!(
            "Before text. ",
            r#"<tool_call>{"name": "get_weather", "arguments": {"location": "SF"}}</tool_call>"#,
            " After text."
        );
        let result = parser.parse(input);
        assert!(result.ok);
        assert_eq!(result.calls.len(), 1);
        let content = result.content;
        assert!(content.contains("Before text."));
        assert!(content.contains("After text."));
    }

    #[test]
    fn test_reasoning_tags_stripped() {
        let parser = HermesParser::new();
        let input = concat!(
            r#"<thinking>let me think</thinking>"#,
            r#"<tool_call>{"name": "foo", "arguments": {}}</tool_call>"#,
        );
        let result = parser.parse(input);
        assert!(result.ok);
        assert_eq!(result.calls.len(), 1);
        assert_eq!(result.calls[0].name, "foo");
    }

    #[test]
    fn test_empty_block_skipped() {
        let parser = HermesParser::new();
        let input = r#"<tool_call>  </tool_call>"#;
        let result = parser.parse(input);
        // Empty block (all whitespace) should be skipped — result is ok with 0 calls
        assert!(result.ok);
        assert!(result.calls.is_empty());
    }

    #[test]
    fn test_generated_call_id_when_missing() {
        let parser = HermesParser::new();
        let input = r#"<tool_call>{"name": "foo", "arguments": {}}</tool_call>"#;
        let result = parser.parse(input);
        assert!(result.ok);
        assert!(result.calls[0].id.starts_with("call_"));
    }

    #[test]
    fn test_parser_metadata() {
        let parser = HermesParser::new();
        assert_eq!(parser.id(), "hermes");
        assert!(parser.description().contains("Hermes"));
        assert!(parser.model_families().contains(&"Qwen2.5"));
    }

    #[test]
    fn test_factory() {
        let parser = create_hermes_parser();
        assert_eq!(parser.id(), "hermes");
    }
}
