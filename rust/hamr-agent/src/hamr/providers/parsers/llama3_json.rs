//! Llama 3 JSON tool-call parser.
//!
//! Ported from `packages/coding-agent/src/hamr/providers/parsers/llama3-json.ts`.
//!
//! Parses Llama 3.x JSON-format tool calls:
//!
//!   <|python_tag|>{"name": "get_weather", "parameters": {"location": "SF", "unit": "celsius"}}
//!
//! The model outputs a `<|python_tag|>` prefix followed by a JSON object
//! with "name" and "parameters" fields. Multiple calls can appear as
//! separate `<|python_tag|>` blocks.
//!
//! vLLM also supports a custom chat template that wraps calls in a
//! `<|start_header_id|>assistant<|end_header_id|>` structure, but
//! the parser handles the raw `<|python_tag|>` blocks directly.
//!
//! Reference: vLLM docs/features/tool_calling.md
//!   --tool-call-parser llama3_json
//!   --chat-template examples/tool_chat_template_llama3.1_json.jinja
//!   vllm/entrypoints/openai/tool_parsers/llama_tool_parser.py

use super::types::{ParsedToolCall, ToolCallParseResult, ToolCallParser};
use super::utils::{generate_call_id, safe_json_parse};

const PARSER_ID: &str = "llama3_json";
const DESCRIPTION: &str =
    r#"Llama 3.x JSON format: <|python_tag|>{"name":"...","parameters":{...}}"#;
const MODEL_FAMILIES: &[&str] = &[
    "Llama 3",
    "Llama 3.1",
    "Llama 3.2",
    "Llama 3.3",
    "Meta Llama 3",
];

const PYTHON_TAG: &str = "<|python_tag|>";

// ─── Parser struct ───────────────────────────────────────

pub struct Llama3JsonParser;

impl Llama3JsonParser {
    pub fn new() -> Self {
        Self
    }
}

impl ToolCallParser for Llama3JsonParser {
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
        // Strip Llama header/footer tags if present (model output format)
        let text = content
            .replace("<|start_header_id|>assistant<|end_header_id|>", "")
            .replace("<|start_header_id|>assistant<|end_header_id|>", "") // case-insensitive via explicit
            .replace("<|eot_id|>", "")
            .trim()
            .to_string();

        // Also handle case variations by regex
        let re_start =
            regex::Regex::new(r"(?i)<\|start_header_id\|>assistant<\|end_header_id\|>").unwrap();
        let text = re_start.replace_all(&text, "").to_string();
        let re_eot = regex::Regex::new(r"(?i)<\|eot_id\|>").unwrap();
        let text = re_eot.replace_all(&text, "").trim().to_string();

        let mut calls: Vec<ParsedToolCall> = Vec::new();
        let mut remaining = text.as_str();
        let mut non_tool_parts: Vec<String> = Vec::new();

        while !remaining.is_empty() {
            let tag_idx = remaining.find(PYTHON_TAG);
            let tag_idx = match tag_idx {
                Some(idx) => idx,
                None => {
                    non_tool_parts.push(remaining.to_string());
                    break;
                }
            };

            // Text before the tag
            if tag_idx > 0 {
                non_tool_parts.push(remaining[..tag_idx].to_string());
            }
            remaining = &remaining[tag_idx + PYTHON_TAG.len()..];

            // Find the first non-whitespace character after the tag
            let json_start = remaining.find(|c: char| !c.is_whitespace());
            let json_start = match json_start {
                Some(idx) => idx,
                None => break,
            };
            remaining = &remaining[json_start..];

            if !remaining.starts_with('{') {
                // Not JSON — this is prose, treat as non-tool content
                let next_tag = remaining.find(PYTHON_TAG);
                match next_tag {
                    Some(idx) => {
                        non_tool_parts.push(remaining[..idx].to_string());
                        remaining = &remaining[idx..];
                        continue;
                    }
                    None => {
                        non_tool_parts.push(remaining.to_string());
                        break;
                    }
                }
            }

            // Extract the JSON object (balanced braces, string-aware)
            let json_end = find_balanced_close(remaining, '{', '}');
            let json_end = match json_end {
                Some(idx) => idx,
                None => {
                    non_tool_parts.push(remaining.to_string());
                    break;
                }
            };

            let json_str = &remaining[..=json_end];
            remaining = &remaining[json_end + 1..];

            let parsed = safe_json_parse(json_str);
            let parsed = match parsed {
                Ok(v) => v,
                Err(_) => {
                    // Malformed JSON — skip and continue
                    non_tool_parts.push(format!("{}{}", PYTHON_TAG, json_str));
                    continue;
                }
            };

            if !parsed.is_object() {
                non_tool_parts.push(format!("{}{}", PYTHON_TAG, json_str));
                continue;
            }

            let obj = parsed.as_object().unwrap();

            // Extract name from any of the common fields
            let name = obj
                .get("name")
                .or_else(|| obj.get("tool_name"))
                .or_else(|| obj.get("function"))
                .and_then(|v| v.as_str())
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty());

            let name = match name {
                Some(n) => n,
                None => {
                    non_tool_parts.push(format!("{}{}", PYTHON_TAG, json_str));
                    continue;
                }
            };

            // Extract arguments from any of the common fields
            let raw_args = obj
                .get("parameters")
                .or_else(|| obj.get("arguments"))
                .or_else(|| obj.get("input"))
                .or_else(|| obj.get("args"));

            let args = if let Some(raw) = raw_args {
                match raw {
                    serde_json::Value::String(s) => {
                        // Try to parse string as JSON
                        match safe_json_parse(s) {
                            Ok(serde_json::Value::Object(_)) => raw.clone(),
                            _ => serde_json::Value::Object(serde_json::Map::new()),
                        }
                    }
                    serde_json::Value::Object(_) => raw.clone(),
                    _ => serde_json::Value::Object(serde_json::Map::new()),
                }
            } else {
                serde_json::Value::Object(serde_json::Map::new())
            };

            // Ensure args is an object
            let args = match args {
                serde_json::Value::Object(_) => args,
                _ => serde_json::Value::Object(serde_json::Map::new()),
            };

            let call_id = obj
                .get("id")
                .or_else(|| obj.get("call_id"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            calls.push(ParsedToolCall {
                id: generate_call_id(call_id.as_deref(), Some(calls.len() + 1)),
                name,
                arguments: args,
                raw_source: Some(json_str.to_string()),
                parser_id: Some(PARSER_ID.to_string()),
                warnings: None,
            });
        }

        let non_tool_content = non_tool_parts.join("").trim().to_string();

        ToolCallParseResult::ok(PARSER_ID, calls, non_tool_content)
    }
}

// ─── Factory ──────────────────────────────────────────────

pub fn create_llama3_json_parser() -> Box<dyn ToolCallParser> {
    Box::new(Llama3JsonParser::new())
}

// ─── Helpers ──────────────────────────────────────────────

/// Find the index of the closing brace/bracket, respecting string boundaries
/// and escape sequences.
fn find_balanced_close(text: &str, open: char, close: char) -> Option<usize> {
    let mut depth: i32 = 0;
    let mut in_string = false;
    let mut escape = false;

    for (i, ch) in text.char_indices() {
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
                return Some(i);
            }
        }
    }

    None
}

// ─── Tests ────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_single_tool_call() {
        let parser = Llama3JsonParser::new();
        let input = r#"<|python_tag|>{"name": "get_weather", "parameters": {"location": "SF", "unit": "celsius"}}"#;
        let result = parser.parse(input);
        assert!(result.ok);
        assert_eq!(result.calls.len(), 1);
        assert_eq!(result.calls[0].name, "get_weather");
        assert_eq!(result.calls[0].arguments["location"], "SF");
        assert_eq!(result.calls[0].arguments["unit"], "celsius");
        assert!(result.content.is_empty());
    }

    #[test]
    fn test_parse_multiple_tool_calls() {
        let parser = Llama3JsonParser::new();
        let input = concat!(
            r#"<|python_tag|>{"name": "get_weather", "parameters": {"location": "SF"}}"#,
            r#"<|python_tag|>{"name": "get_time", "parameters": {"tz": "PST"}}"#,
        );
        let result = parser.parse(input);
        assert!(result.ok);
        assert_eq!(result.calls.len(), 2);
        assert_eq!(result.calls[0].name, "get_weather");
        assert_eq!(result.calls[1].name, "get_time");
    }

    #[test]
    fn test_parse_with_prose_between() {
        let parser = Llama3JsonParser::new();
        let input = concat!(
            r#"Let me look that up."#,
            r#"<|python_tag|>{"name": "get_weather", "parameters": {"location": "SF"}}"#,
            r#"Here's the result:"#,
        );
        let result = parser.parse(input);
        assert!(result.ok);
        assert_eq!(result.calls.len(), 1);
        assert_eq!(result.calls[0].name, "get_weather");
        assert!(result.content.contains("Let me look that up."));
        assert!(result.content.contains("Here's the result:"));
    }

    #[test]
    fn test_parse_no_tool_calls() {
        let parser = Llama3JsonParser::new();
        let input = "Hello, how can I help you today?";
        let result = parser.parse(input);
        assert!(result.ok);
        assert_eq!(result.calls.len(), 0);
        assert_eq!(result.content, "Hello, how can I help you today?");
    }

    #[test]
    fn test_parse_with_header_footer_tags() {
        let parser = Llama3JsonParser::new();
        let input = concat!(
            r#"<|start_header_id|>assistant<|end_header_id|>"#,
            r#"<|python_tag|>{"name": "get_weather", "parameters": {"location": "SF"}}"#,
            r#"<|eot_id|>"#,
        );
        let result = parser.parse(input);
        assert!(result.ok);
        assert_eq!(result.calls.len(), 1);
        assert_eq!(result.calls[0].name, "get_weather");
    }

    #[test]
    fn test_parse_with_prose_before_first_tag() {
        let parser = Llama3JsonParser::new();
        let input = concat!(
            r#"I'll check the weather."#,
            r#"<|python_tag|>{"name": "get_weather", "parameters": {"location": "SF"}}"#,
        );
        let result = parser.parse(input);
        assert!(result.ok);
        assert_eq!(result.calls.len(), 1);
        assert!(result.content.contains("I'll check the weather."));
    }

    #[test]
    fn test_parse_uses_tool_name_field() {
        let parser = Llama3JsonParser::new();
        let input = r#"<|python_tag|>{"tool_name": "search", "parameters": {"query": "weather"}}"#;
        let result = parser.parse(input);
        assert!(result.ok);
        assert_eq!(result.calls.len(), 1);
        assert_eq!(result.calls[0].name, "search");
    }

    #[test]
    fn test_parse_uses_function_field() {
        let parser = Llama3JsonParser::new();
        let input = r#"<|python_tag|>{"function": "search", "parameters": {"query": "weather"}}"#;
        let result = parser.parse(input);
        assert!(result.ok);
        assert_eq!(result.calls.len(), 1);
        assert_eq!(result.calls[0].name, "search");
    }

    #[test]
    fn test_parse_string_arguments() {
        let parser = Llama3JsonParser::new();
        let input = r#"<|python_tag|>{"name": "search", "parameters": "{\"query\": \"weather\"}"}"#;
        let result = parser.parse(input);
        assert!(result.ok);
        assert_eq!(result.calls.len(), 1);
        assert_eq!(result.calls[0].name, "search");
    }

    #[test]
    fn test_parse_malformed_json_skips() {
        let parser = Llama3JsonParser::new();
        let input = r#"<|python_tag|>{"name": "good", "parameters": {}}"#;
        let result = parser.parse(input);
        assert!(result.ok);
        assert_eq!(result.calls.len(), 1);
        assert_eq!(result.calls[0].name, "good");
    }

    #[test]
    fn test_parse_empty_name_skips() {
        let parser = Llama3JsonParser::new();
        let input = r#"<|python_tag|>{"name": "", "parameters": {}}"#;
        let result = parser.parse(input);
        assert!(result.ok);
        assert_eq!(result.calls.len(), 0);
    }

    #[test]
    fn test_parse_non_dict_json_skips() {
        let parser = Llama3JsonParser::new();
        let input = r#"<|python_tag|>[1, 2, 3]"#;
        let result = parser.parse(input);
        assert!(result.ok);
        assert_eq!(result.calls.len(), 0);
    }

    #[test]
    fn test_parse_non_json_text_after_tag_treated_as_prose() {
        let parser = Llama3JsonParser::new();
        let input =
            r#"<|python_tag|>some prose text here<|python_tag|>{"name": "real", "parameters": {}}"#;
        let result = parser.parse(input);
        assert!(result.ok);
        assert_eq!(result.calls.len(), 1);
        assert_eq!(result.calls[0].name, "real");
        assert!(result.content.contains("some prose text here"));
    }

    #[test]
    fn test_parse_with_nested_braces_in_strings() {
        let parser = Llama3JsonParser::new();
        let input = r#"<|python_tag|>{"name": "test", "parameters": {"data": "a{b}c"}}"#;
        let result = parser.parse(input);
        assert!(result.ok);
        assert_eq!(result.calls.len(), 1);
        assert_eq!(result.calls[0].arguments["data"], "a{b}c");
    }

    #[test]
    fn test_parse_uses_arguments_field() {
        let parser = Llama3JsonParser::new();
        let input = r#"<|python_tag|>{"name": "search", "arguments": {"query": "weather"}}"#;
        let result = parser.parse(input);
        assert!(result.ok);
        assert_eq!(result.calls.len(), 1);
        assert_eq!(result.calls[0].arguments["query"], "weather");
    }

    #[test]
    fn test_parse_uses_input_field() {
        let parser = Llama3JsonParser::new();
        let input = r#"<|python_tag|>{"name": "search", "input": {"query": "weather"}}"#;
        let result = parser.parse(input);
        assert!(result.ok);
        assert_eq!(result.calls.len(), 1);
        assert_eq!(result.calls[0].arguments["query"], "weather");
    }

    #[test]
    fn test_parse_with_call_id() {
        let parser = Llama3JsonParser::new();
        let input = r#"<|python_tag|>{"id": "call_123", "name": "search", "parameters": {"query": "weather"}}"#;
        let result = parser.parse(input);
        assert!(result.ok);
        assert_eq!(result.calls.len(), 1);
        assert_eq!(result.calls[0].id, "call_123");
    }

    #[test]
    fn test_parse_with_call_id_field() {
        let parser = Llama3JsonParser::new();
        let input = r#"<|python_tag|>{"call_id": "call_456", "name": "search", "parameters": {"query": "weather"}}"#;
        let result = parser.parse(input);
        assert!(result.ok);
        assert_eq!(result.calls.len(), 1);
        assert_eq!(result.calls[0].id, "call_456");
    }

    #[test]
    fn test_parse_raw_source_and_parser_id() {
        let parser = Llama3JsonParser::new();
        let input = r#"<|python_tag|>{"name": "test", "parameters": {}}"#;
        let result = parser.parse(input);
        assert!(result.ok);
        assert_eq!(result.calls.len(), 1);
        assert!(result.calls[0].raw_source.is_some());
        assert_eq!(
            result.calls[0].raw_source.as_deref().unwrap(),
            r#"{"name": "test", "parameters": {}}"#
        );
        assert_eq!(result.calls[0].parser_id.as_deref().unwrap(), "llama3_json");
    }

    #[test]
    fn test_parse_case_insensitive_tags() {
        let parser = Llama3JsonParser::new();
        let input = r#"<|START_HEADER_ID|>assistant<|END_HEADER_ID|><|python_tag|>{"name": "test", "parameters": {}}"#;
        let result = parser.parse(input);
        assert!(result.ok);
        assert_eq!(result.calls.len(), 1);
    }

    #[test]
    fn test_generated_call_ids() {
        let parser = Llama3JsonParser::new();
        let input = concat!(
            r#"<|python_tag|>{"name": "a", "parameters": {}}"#,
            r#"<|python_tag|>{"name": "b", "parameters": {}}"#,
        );
        let result = parser.parse(input);
        assert_eq!(result.calls[0].id, "call_1");
        assert_eq!(result.calls[1].id, "call_2");
    }

    #[test]
    fn test_balanced_close_handles_strings() {
        assert_eq!(find_balanced_close(r#"{"a": "b{c}d"}"#, '{', '}'), Some(13));
    }

    #[test]
    fn test_balanced_close_handles_escaped_quotes() {
        assert_eq!(find_balanced_close(r#"{"a": "b\"c"}"#, '{', '}'), Some(12));
    }

    #[test]
    fn test_balanced_close_unmatched_returns_none() {
        assert_eq!(find_balanced_close(r#"{"a": 1"#, '{', '}'), None);
    }

    #[test]
    fn test_parser_id_and_description() {
        let parser = Llama3JsonParser::new();
        assert_eq!(parser.id(), "llama3_json");
        assert!(parser.description().contains("Llama 3"));
        assert!(parser.model_families().contains(&"Llama 3"));
    }
}
