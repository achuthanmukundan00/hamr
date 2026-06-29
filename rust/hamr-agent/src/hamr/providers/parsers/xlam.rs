//! xLAM tool-call parser.
//!
//! Parses xLAM-format tool calls. The xLAM family uses Hermes-compatible
//! XML-style tags with JSON tool call objects:
//!
//! ```text
//! <tool_call>
//! {"name": "get_weather", "arguments": {"location": "SF", "unit": "celsius"}}
//! </tool_call>
//! ```
//!
//! Reference: vLLM
//!   --tool-call-parser xlam
//!   vllm/entrypoints/openai/tool_parsers/xlam_tool_parser.py
//!
//! Note: xLAM also supports a second format using plain function name then args:
//! ```text
//! get_weather
//! {"location": "SF", "unit": "celsius"}
//! ```
//! This parser handles both formats.

use super::types::{ParsedToolCall, ToolCallParseResult, ToolCallParser};
use super::utils::{
    extract_delimited_blocks, generate_call_id, safe_json_parse, sanitize_reasoning_tags,
};

static PARSER_ID: &str = "xlam";
static DESCRIPTION: &str = "xLAM format: <tool_call>{\"name\":\"...\",\"arguments\":{...}}</tool_call> or bare function+JSON";
static FAMILIES: &[&str] = &["xLAM"];

/// Build a `ParsedToolCall` from a JSON object found inside `<tool_call>` tags.
fn build_call(
    name: String,
    obj: &serde_json::Map<String, serde_json::Value>,
    raw: &str,
    index: usize,
) -> ParsedToolCall {
    let args = match obj
        .get("arguments")
        .or_else(|| obj.get("parameters"))
        .or_else(|| obj.get("input"))
    {
        Some(serde_json::Value::String(s)) => match safe_json_parse(s) {
            Ok(serde_json::Value::Object(_)) => {
                safe_json_parse(s).unwrap_or(serde_json::Value::Object(serde_json::Map::new()))
            }
            _ => serde_json::Value::Object(serde_json::Map::new()),
        },
        Some(v @ serde_json::Value::Object(_)) => v.clone(),
        _ => serde_json::Value::Object(serde_json::Map::new()),
    };

    let provided_id = obj
        .get("id")
        .or_else(|| obj.get("call_id"))
        .and_then(|v| v.as_str());
    let id = generate_call_id(provided_id, Some(index + 1));

    ParsedToolCall {
        id,
        name,
        arguments: args,
        raw_source: Some(raw.to_string()),
        parser_id: Some(PARSER_ID.to_string()),
        warnings: None,
    }
}

fn name_regex() -> regex::Regex {
    regex::Regex::new(r"^[a-zA-Z_]\w*$").unwrap()
}

/// The xLAM parser implementation.
#[derive(Debug, Clone)]
pub struct XlamParser;

impl ToolCallParser for XlamParser {
    fn id(&self) -> &str {
        PARSER_ID
    }

    fn description(&self) -> &str {
        DESCRIPTION
    }

    fn model_families(&self) -> &[&str] {
        FAMILIES
    }

    fn parse(&self, content: &str) -> ToolCallParseResult {
        let sanitized = sanitize_reasoning_tags(content);

        // Try <tool_call>...</tool_call> delimiters first
        let delimited = extract_delimited_blocks(&sanitized, "<tool_call>", "</tool_call>");

        if !delimited.blocks.is_empty() {
            let mut calls: Vec<ParsedToolCall> = Vec::new();

            for (i, block) in delimited.blocks.iter().enumerate() {
                let trimmed = block.trim();
                if trimmed.is_empty() {
                    continue;
                }

                // xLAM may use JSON object inside the tag
                let parsed = safe_json_parse(trimmed);
                if let Ok(serde_json::Value::Object(ref obj)) = parsed {
                    let name = obj
                        .get("name")
                        .or_else(|| obj.get("tool_name"))
                        .and_then(|v| v.as_str())
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty());

                    if let Some(name) = name {
                        calls.push(build_call(name, obj, block, i));
                        continue;
                    }
                }

                // xLAM alternate format: function name on first line, JSON args on subsequent lines
                let lines: Vec<&str> = trimmed
                    .split('\n')
                    .map(|l| l.trim())
                    .filter(|l| !l.is_empty())
                    .collect();

                if !lines.is_empty() {
                    let fn_name = lines[0];
                    if name_regex().is_match(fn_name) {
                        let args_json = lines[1..].join("\n");
                        let args = if args_json.trim().is_empty() {
                            serde_json::Value::Object(serde_json::Map::new())
                        } else {
                            match safe_json_parse(&args_json) {
                                Ok(v @ serde_json::Value::Object(_)) => v,
                                _ => serde_json::Value::Object(serde_json::Map::new()),
                            }
                        };

                        calls.push(ParsedToolCall {
                            id: generate_call_id(None, Some(i + 1)),
                            name: fn_name.to_string(),
                            arguments: args,
                            raw_source: Some(block.clone()),
                            parser_id: Some(PARSER_ID.to_string()),
                            warnings: None,
                        });
                        continue;
                    }
                }

                // Unrecognized format in block — treat as error
                return ToolCallParseResult::err(
                    PARSER_ID,
                    &sanitized,
                    format!("xLAM block {}: unrecognized format", i + 1),
                );
            }

            let non_tool_content = [
                delimited.before.as_str(),
                // Join all "between" segments
            ]
            .into_iter()
            .chain(
                delimited
                    .between
                    .iter()
                    .map(|s| s.as_str())
                    .collect::<Vec<&str>>(),
            )
            .chain(std::iter::once(delimited.after.as_str()))
            .filter(|s| !s.trim().is_empty())
            .collect::<Vec<&str>>()
            .join("\n")
            .trim()
            .to_string();

            return ToolCallParseResult::ok(PARSER_ID, calls, non_tool_content);
        }

        ToolCallParseResult::ok(PARSER_ID, vec![], &sanitized)
    }
}

// ─── Factory ──────────────────────────────────────────────

#[must_use]
pub fn create_xlam_parser() -> Box<dyn ToolCallParser> {
    Box::new(XlamParser)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_xlam_empty_content() {
        let parser = XlamParser;
        let result = parser.parse("");
        assert!(result.ok);
        assert!(result.calls.is_empty());
    }

    #[test]
    fn test_xlam_no_tool_calls() {
        let parser = XlamParser;
        let result = parser.parse("Hello, how can I help you?");
        assert!(result.ok);
        assert!(result.calls.is_empty());
        assert!(result.content.contains("Hello"));
    }

    #[test]
    fn test_xlam_single_tool_call_json() {
        let parser = XlamParser;
        let result = parser.parse(
            r#"<tool_call>{"name": "get_weather", "arguments": {"location": "SF", "unit": "celsius"}}</tool_call>"#,
        );
        assert!(
            result.ok,
            "result should be ok, got error: {:?}",
            result.error
        );
        assert_eq!(result.calls.len(), 1);
        assert_eq!(result.calls[0].name, "get_weather");
        assert_eq!(
            result.calls[0].arguments,
            serde_json::json!({"location": "SF", "unit": "celsius"})
        );
    }

    #[test]
    fn test_xlam_tool_name_field() {
        let parser = XlamParser;
        let result = parser.parse(
            r#"<tool_call>{"tool_name": "get_weather", "arguments": {"location": "NYC"}}</tool_call>"#,
        );
        assert!(result.ok);
        assert_eq!(result.calls.len(), 1);
        assert_eq!(result.calls[0].name, "get_weather");
    }

    #[test]
    fn test_xlam_call_id_field() {
        let parser = XlamParser;
        let result = parser.parse(
            r#"<tool_call>{"id": "my_call_123", "name": "get_weather", "arguments": {"location": "SF"}}</tool_call>"#,
        );
        assert!(result.ok);
        assert_eq!(result.calls.len(), 1);
        assert_eq!(result.calls[0].id, "my_call_123");
    }

    #[test]
    fn test_xlam_call_id_via_call_id_field() {
        let parser = XlamParser;
        let result = parser.parse(
            r#"<tool_call>{"call_id": "cid_456", "name": "get_weather", "arguments": {"location": "SF"}}</tool_call>"#,
        );
        assert!(result.ok);
        assert_eq!(result.calls.len(), 1);
        assert_eq!(result.calls[0].id, "cid_456");
    }

    #[test]
    fn test_xlam_args_as_string() {
        let parser = XlamParser;
        let result = parser.parse(
            r#"<tool_call>{"name": "get_weather", "arguments": "{\"location\": \"SF\"}"}</tool_call>"#,
        );
        assert!(result.ok);
        assert_eq!(result.calls.len(), 1);
        assert_eq!(
            result.calls[0].arguments,
            serde_json::json!({"location": "SF"})
        );
    }

    #[test]
    fn test_xlam_parameters_field() {
        let parser = XlamParser;
        let result = parser.parse(
            r#"<tool_call>{"name": "get_weather", "parameters": {"location": "SF"}}</tool_call>"#,
        );
        assert!(result.ok);
        assert_eq!(result.calls.len(), 1);
        assert_eq!(
            result.calls[0].arguments,
            serde_json::json!({"location": "SF"})
        );
    }

    #[test]
    fn test_xlam_input_field() {
        let parser = XlamParser;
        let result = parser.parse(
            r#"<tool_call>{"name": "get_weather", "input": {"location": "SF"}}</tool_call>"#,
        );
        assert!(result.ok);
        assert_eq!(result.calls.len(), 1);
        assert_eq!(
            result.calls[0].arguments,
            serde_json::json!({"location": "SF"})
        );
    }

    #[test]
    fn test_xlam_alternate_format() {
        let parser = XlamParser;
        let result = parser.parse(
            r#"<tool_call>get_weather
{"location": "SF", "unit": "celsius"}</tool_call>"#,
        );
        assert!(result.ok);
        assert_eq!(result.calls.len(), 1);
        assert_eq!(result.calls[0].name, "get_weather");
        assert_eq!(
            result.calls[0].arguments,
            serde_json::json!({"location": "SF", "unit": "celsius"})
        );
    }

    #[test]
    fn test_xlam_alternate_format_no_args() {
        let parser = XlamParser;
        let result = parser.parse(r#"<tool_call>get_weather</tool_call>"#);
        assert!(result.ok);
        assert_eq!(result.calls.len(), 1);
        assert_eq!(result.calls[0].name, "get_weather");
        assert_eq!(
            result.calls[0].arguments,
            serde_json::Value::Object(serde_json::Map::new())
        );
    }

    #[test]
    fn test_xlam_multiple_calls() {
        let parser = XlamParser;
        let content = r#"
Some text before.
<tool_call>{"name": "get_weather", "arguments": {"location": "SF"}}</tool_call>
Some text in between.
<tool_call>{"name": "get_time", "arguments": {"timezone": "PST"}}</tool_call>
Some text after.
"#;
        let result = parser.parse(content);
        assert!(result.ok);
        assert_eq!(result.calls.len(), 2);
        assert_eq!(result.calls[0].name, "get_weather");
        assert_eq!(result.calls[1].name, "get_time");

        // Non-tool content should be preserved
        assert!(result.content.contains("Some text before"));
        assert!(result.content.contains("Some text in between"));
        assert!(result.content.contains("Some text after"));
    }

    #[test]
    fn test_xlam_unrecognized_format_error() {
        let parser = XlamParser;
        let result = parser.parse(r#"<tool_call>garbage data here {{not json}}</tool_call>"#);
        assert!(!result.ok);
        assert!(result.error.is_some());
        assert!(
            result
                .error
                .as_ref()
                .unwrap()
                .contains("unrecognized format")
        );
    }

    #[test]
    fn test_xlam_thinking_stripped() {
        let parser = XlamParser;
        let result = parser.parse(
            r#"<think>I need to check the weather</think>
<tool_call>{"name": "get_weather", "arguments": {"location": "SF"}}</tool_call>"#,
        );
        assert!(result.ok);
        assert_eq!(result.calls.len(), 1);
        assert_eq!(result.calls[0].name, "get_weather");
        // <think> block should be removed from non-tool content
        assert!(!result.content.contains("<think>"));
    }

    #[test]
    fn test_xlam_non_object_json_in_tag() {
        let parser = XlamParser;
        let result = parser.parse(r#"<tool_call>"just a string"</tool_call>"#);
        assert!(!result.ok);
    }

    #[test]
    fn test_xlam_bare_text_only() {
        let parser = XlamParser;
        let result = parser.parse("I am a helpful assistant.");
        assert!(result.ok);
        assert!(result.calls.is_empty());
        assert_eq!(result.content, "I am a helpful assistant.");
    }

    #[test]
    fn test_xlam_sanitize_handles_thinking_tags() {
        let parser = XlamParser;
        let result = parser.parse(
            r#"<thinking>let me compute</thinking>
<tool_call>{"name": "calculate", "arguments": {"expr": "2+2"}}</tool_call>"#,
        );
        assert!(result.ok);
        assert_eq!(result.calls.len(), 1);
        assert_eq!(result.calls[0].name, "calculate");
    }

    #[test]
    fn test_xlam_alternate_format_invalid_function_name() {
        let parser = XlamParser;
        // A name that doesn't match /^[a-zA-Z_]\w*$/ — starts with digit
        let result = parser.parse(
            r#"<tool_call>123invalid
{"location": "SF"}</tool_call>"#,
        );
        // Should fail as unrecognized format
        assert!(!result.ok);
    }

    #[test]
    fn test_factory() {
        let parser = create_xlam_parser();
        assert_eq!(parser.id(), "xlam");
        assert_eq!(parser.description(), DESCRIPTION);
        assert_eq!(parser.model_families(), &["xLAM"]);
    }

    #[test]
    fn test_xlam_empty_tag() {
        let parser = XlamParser;
        let result = parser.parse(r#"<tool_call>  </tool_call>"#);
        assert!(result.ok);
        assert!(result.calls.is_empty());
    }

    #[test]
    fn test_xlam_raw_source_preserved() {
        let parser = XlamParser;
        let result = parser.parse(r#"<tool_call>{"name": "fn1", "arguments": {}}</tool_call>"#);
        assert!(result.ok);
        assert_eq!(result.calls.len(), 1);
        assert_eq!(
            result.calls[0].raw_source.as_deref(),
            Some(r#"{"name": "fn1", "arguments": {}}"#)
        );
    }

    #[test]
    fn test_xlam_parser_id_in_call() {
        let parser = XlamParser;
        let result = parser.parse(r#"<tool_call>{"name": "fn1", "arguments": {}}</tool_call>"#);
        assert!(result.ok);
        assert_eq!(result.calls[0].parser_id.as_deref(), Some("xlam"));
    }
}
