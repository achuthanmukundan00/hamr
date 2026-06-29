//! Generic multi-strategy fallback parser.
//!
//! Handles both JSON and XML tool-call formats so unrecognised model families
//! (Apodex, Qwen derivatives, …) are not silently broken.
//!
//! Strategies (tried in order):
//! 1. `<tool_call>` blocks → JSON first (Hermes-style), then XML (Qwen3-style)
//! 2. ```json fenced code blocks
//! 3. Bare JSON objects (last resort)
//!
//! Ported from `packages/coding-agent/src/hamr/providers/parsers/generic.ts`.

use super::registry::parse_with_tool_call_parser;
use super::types::{Arguments, ParsedToolCall, ToolCallParseResult, ToolCallParser};
use super::utils::{
    DelimitedResult, extract_delimited_blocks, generate_call_id, safe_json_parse,
    sanitize_reasoning_tags,
};

const PARSER_ID: &str = "generic";
const DESCRIPTION: &str =
    "Generic multi-strategy fallback: handles JSON (Hermes-style) and XML (Qwen3-style) tool calls";
const FAMILIES: &[&str] = &["Any", "Unknown", "Generic"];

// ─── Parser struct ────────────────────────────────────────

pub struct GenericParser;

impl GenericParser {
    pub fn new() -> Self {
        Self
    }
}

impl ToolCallParser for GenericParser {
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
        let mut calls: Vec<ParsedToolCall> = Vec::new();
        let warnings: Vec<String> = Vec::new();

        // ═══════════════════════════════════════════════════
        // Strategy 1: <tool_call>...</tool_call> blocks
        // ═══════════════════════════════════════════════════
        let tool_call_blocks = extract_delimited_blocks(&sanitized, "<tool_call>", "</tool_call>");
        let mut saw_malformed_block = false;

        for block in &tool_call_blocks.blocks {
            let trimmed = block.trim();
            if trimmed.is_empty() {
                continue;
            }

            match safe_json_parse(trimmed) {
                Ok(parsed) => {
                    if let serde_json::Value::Object(ref obj) = parsed {
                        // Check for array of tool_calls (OpenAI-style wrapped)
                        if let Some(serde_json::Value::Array(tool_calls)) = obj.get("tool_calls") {
                            for tc in tool_calls {
                                if let Some(tc_obj) = tc.as_object() {
                                    if let Some(fn_obj) =
                                        tc_obj.get("function").and_then(|v| v.as_object())
                                    {
                                        if let Some(serde_json::Value::String(name)) =
                                            fn_obj.get("name")
                                        {
                                            let args = parse_args_value(fn_obj.get("arguments"));
                                            let tc_id = tc_obj
                                                .get("id")
                                                .or_else(|| fn_obj.get("id"))
                                                .and_then(|v| v.as_str());
                                            calls.push(ParsedToolCall {
                                                id: generate_call_id(tc_id, Some(calls.len() + 1)),
                                                name: name.clone(),
                                                arguments: args,
                                                raw_source: Some(
                                                    serde_json::to_string(tc).unwrap_or_default(),
                                                ),
                                                parser_id: Some(PARSER_ID.to_string()),
                                                warnings: None,
                                            });
                                        }
                                    }
                                }
                            }
                            continue;
                        }

                        // Single named call
                        let name = obj
                            .get("name")
                            .or_else(|| obj.get("tool_name"))
                            .and_then(|v| v.as_str())
                            .map(|s| s.trim().to_string())
                            .filter(|s| !s.is_empty());

                        match name {
                            Some(n) => {
                                let args = parse_args_value(
                                    obj.get("arguments")
                                        .or_else(|| obj.get("parameters"))
                                        .or_else(|| obj.get("input")),
                                );
                                let call_id = obj
                                    .get("id")
                                    .or_else(|| obj.get("call_id"))
                                    .and_then(|v| v.as_str());
                                calls.push(ParsedToolCall {
                                    id: generate_call_id(call_id, Some(calls.len() + 1)),
                                    name: n,
                                    arguments: args,
                                    raw_source: Some(trimmed.to_string()),
                                    parser_id: Some(PARSER_ID.to_string()),
                                    warnings: None,
                                });
                            }
                            None => {
                                saw_malformed_block = true;
                            }
                        }
                    } else {
                        saw_malformed_block = true;
                    }
                }
                Err(_) => {
                    saw_malformed_block = true;
                }
            }
        }

        // ═══════════════════════════════════════════════════
        // Strategy 2: ```json fenced code blocks
        // ═══════════════════════════════════════════════════
        let fenced_blocks = extract_fenced_json_blocks(&sanitized);
        for block in &fenced_blocks {
            if let Ok(parsed) = safe_json_parse(block) {
                if let serde_json::Value::Object(ref obj) = parsed {
                    // Handle OpenAI-style tool_calls array inside the block
                    if let Some(serde_json::Value::Array(tool_calls)) = obj.get("tool_calls") {
                        for tc in tool_calls {
                            if let Some(tc_obj) = tc.as_object() {
                                if let Some(fn_obj) =
                                    tc_obj.get("function").and_then(|v| v.as_object())
                                {
                                    if let Some(serde_json::Value::String(name)) =
                                        fn_obj.get("name")
                                    {
                                        let args = parse_args_value(fn_obj.get("arguments"));
                                        let tc_id = tc_obj
                                            .get("id")
                                            .or_else(|| fn_obj.get("id"))
                                            .and_then(|v| v.as_str());
                                        calls.push(ParsedToolCall {
                                            id: generate_call_id(tc_id, Some(calls.len() + 1)),
                                            name: name.clone(),
                                            arguments: args,
                                            raw_source: Some(
                                                serde_json::to_string(tc).unwrap_or_default(),
                                            ),
                                            parser_id: Some(PARSER_ID.to_string()),
                                            warnings: Some(vec![
                                                "parsed from fenced code block".to_string(),
                                            ]),
                                        });
                                    }
                                }
                            }
                        }
                        continue;
                    }

                    // Handle single named call in fenced block
                    let name = obj
                        .get("name")
                        .or_else(|| obj.get("tool_name"))
                        .and_then(|v| v.as_str())
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty());

                    if let Some(n) = name {
                        let args = parse_args_value(
                            obj.get("arguments")
                                .or_else(|| obj.get("parameters"))
                                .or_else(|| obj.get("input")),
                        );
                        let call_id = obj
                            .get("id")
                            .or_else(|| obj.get("call_id"))
                            .and_then(|v| v.as_str());
                        calls.push(ParsedToolCall {
                            id: generate_call_id(call_id, Some(calls.len() + 1)),
                            name: n,
                            arguments: args,
                            raw_source: Some(block.clone()),
                            parser_id: Some(PARSER_ID.to_string()),
                            warnings: Some(vec!["parsed from fenced code block".to_string()]),
                        });
                    }
                }
            }
        }

        // ═══════════════════════════════════════════════════
        // Strategy 3: Bare JSON object (last resort)
        // ═══════════════════════════════════════════════════
        if calls.is_empty() {
            let trimmed = sanitized.trim();
            if trimmed.starts_with('{') {
                if let Ok(parsed) = safe_json_parse(trimmed) {
                    if let serde_json::Value::Object(ref obj) = parsed {
                        // Handle OpenAI-style tool_calls array
                        if let Some(serde_json::Value::Array(tool_calls)) = obj.get("tool_calls") {
                            for tc in tool_calls {
                                if let Some(tc_obj) = tc.as_object() {
                                    if let Some(fn_obj) =
                                        tc_obj.get("function").and_then(|v| v.as_object())
                                    {
                                        if let Some(serde_json::Value::String(name)) =
                                            fn_obj.get("name")
                                        {
                                            let args = parse_args_value(fn_obj.get("arguments"));
                                            let tc_id = tc_obj
                                                .get("id")
                                                .or_else(|| fn_obj.get("id"))
                                                .and_then(|v| v.as_str());
                                            calls.push(ParsedToolCall {
                                                id: generate_call_id(tc_id, Some(calls.len() + 1)),
                                                name: name.clone(),
                                                arguments: args,
                                                raw_source: Some(
                                                    serde_json::to_string(tc).unwrap_or_default(),
                                                ),
                                                parser_id: Some(PARSER_ID.to_string()),
                                                warnings: Some(vec![
                                                    "parsed from bare JSON text".to_string(),
                                                ]),
                                            });
                                        }
                                    }
                                }
                            }
                        } else {
                            let name = obj
                                .get("name")
                                .or_else(|| obj.get("tool_name"))
                                .and_then(|v| v.as_str())
                                .map(|s| s.trim().to_string())
                                .filter(|s| !s.is_empty());

                            if let Some(n) = name {
                                let args = parse_args_value(
                                    obj.get("arguments")
                                        .or_else(|| obj.get("parameters"))
                                        .or_else(|| obj.get("input")),
                                );
                                let call_id = obj
                                    .get("id")
                                    .or_else(|| obj.get("call_id"))
                                    .and_then(|v| v.as_str());
                                calls.push(ParsedToolCall {
                                    id: generate_call_id(call_id, Some(1)),
                                    name: n,
                                    arguments: args,
                                    raw_source: Some(trimmed.to_string()),
                                    parser_id: Some(PARSER_ID.to_string()),
                                    warnings: Some(vec!["parsed from bare JSON text".to_string()]),
                                });
                            }
                        }
                    }
                }
            }
        }

        // ═══════════════════════════════════════════════════
        // Strategy 1b: Qwen3 XML fallback for malformed blocks
        // ═══════════════════════════════════════════════════
        let mut xml_non_tool_content: Option<String> = None;
        if saw_malformed_block
            && calls.is_empty()
            && !tool_call_blocks.blocks.is_empty()
            && is_standalone_tool_call_content(&sanitized, &tool_call_blocks)
        {
            let xml_result = parse_with_tool_call_parser("qwen3_xml", &sanitized);
            if xml_result.ok && !xml_result.calls.is_empty() {
                for mut call in xml_result.calls {
                    call.parser_id = Some(PARSER_ID.to_string());
                    let mut ws = call.warnings.unwrap_or_default();
                    ws.push("parsed via qwen3_xml fallback in generic parser".to_string());
                    call.warnings = Some(ws);
                    calls.push(call);
                }
                saw_malformed_block = false;
                xml_non_tool_content = xml_result.content.into();
            } else {
                // Neither JSON nor XML could parse the blocks and they look
                // intentional → signal failure so the repair cascade can try.
                return ToolCallParseResult::err(
                    PARSER_ID,
                    sanitized,
                    "tool_call block contained malformed content (not valid JSON or Qwen XML)",
                );
            }
        }

        // ═══════════════════════════════════════════════════
        // Compute non-tool content
        // ═══════════════════════════════════════════════════
        let non_tool_content: String = if let Some(xml_content) = xml_non_tool_content {
            xml_content
        } else if !saw_malformed_block && !calls.is_empty() && !tool_call_blocks.blocks.is_empty() {
            // Blocks were cleanly parsed as JSON — strip them
            let parts: Vec<&str> = std::iter::once(tool_call_blocks.before.as_str())
                .chain(tool_call_blocks.between.iter().map(|s| s.as_str()))
                .chain(std::iter::once(tool_call_blocks.after.as_str()))
                .filter(|s| !s.trim().is_empty())
                .collect();
            parts.join("\n").trim().to_string()
        } else {
            // Blocks couldn't be parsed — strip blocks so model sees clean content
            let parts: Vec<&str> = std::iter::once(tool_call_blocks.before.as_str())
                .chain(tool_call_blocks.between.iter().map(|s| s.as_str()))
                .chain(std::iter::once(tool_call_blocks.after.as_str()))
                .filter(|s| !s.trim().is_empty())
                .collect();
            parts.join("\n").trim().to_string()
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

// ─── Factory ──────────────────────────────────────────────

pub fn create_generic_parser() -> Box<dyn ToolCallParser> {
    Box::new(GenericParser::new())
}

// ─── Helpers ──────────────────────────────────────────────

/// Parse a JSON Value that may be a JSON string (parse it again) or an object.
fn parse_args_value(raw: Option<&serde_json::Value>) -> Arguments {
    match raw {
        Some(serde_json::Value::String(s)) => {
            if let Ok(parsed) = safe_json_parse(s) {
                if let serde_json::Value::Object(_) = parsed {
                    return parsed;
                }
            }
            Arguments::Object(serde_json::Map::new())
        }
        Some(val @ serde_json::Value::Object(_)) => val.clone(),
        _ => Arguments::Object(serde_json::Map::new()),
    }
}

/// Extract content from ```json or ``` fenced code blocks.
fn extract_fenced_json_blocks(content: &str) -> Vec<String> {
    let re = regex::Regex::new(r"```(?:json)?\s*([\s\S]*?)\s*```").unwrap();
    re.captures_iter(content)
        .map(|cap| cap[1].to_string())
        .collect()
}

/// Check if the content is primarily a tool-call response (standalone blocks).
/// If tool_calls appear inside prose/fenced code, they are likely examples.
fn is_standalone_tool_call_content(content: &str, blocks: &DelimitedResult) -> bool {
    // If content is entirely within fenced code blocks, it's not a standalone tool call
    let fenced_re = regex::Regex::new(r"```[\s\S]*?```").unwrap();
    let fenced_content: Vec<&str> = fenced_re.find_iter(content).map(|m| m.as_str()).collect();

    for fenced in &fenced_content {
        if fenced.contains("<tool_call>") {
            return false;
        }
    }

    // If there's substantial non-whitespace content before the first block, not standalone
    if !blocks.before.trim().is_empty() {
        if blocks.before.trim().split_whitespace().count() > 5 {
            return false;
        }
    }

    // If there's text between blocks that looks like prose, not standalone
    for between in &blocks.between {
        if between.trim().split_whitespace().count() > 5 {
            return false;
        }
    }

    // If there's substantial text after the last block, not standalone
    if blocks.after.trim().split_whitespace().count() > 5 {
        return false;
    }

    true
}

// ─── Tests ────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_content() {
        let parser = GenericParser::new();
        let result = parser.parse("");
        assert!(result.ok);
        assert!(result.calls.is_empty());
    }

    #[test]
    fn test_no_tool_calls() {
        let parser = GenericParser::new();
        let result = parser.parse("Hello, how can I help you?");
        assert!(result.ok);
        assert!(result.calls.is_empty());
        assert_eq!(result.content, "Hello, how can I help you?");
    }

    #[test]
    fn test_single_tool_call_hermes_style() {
        let parser = GenericParser::new();
        let input =
            r#"<tool_call>{"name": "get_weather", "arguments": {"location": "NYC"}}</tool_call>"#;
        let result = parser.parse(input);
        assert!(result.ok);
        assert_eq!(result.calls.len(), 1);
        assert_eq!(result.calls[0].name, "get_weather");
        assert_eq!(result.parser_id, "generic");
    }

    #[test]
    fn test_tool_call_with_tool_name() {
        let parser = GenericParser::new();
        let input =
            r#"<tool_call>{"tool_name": "search", "arguments": {"query": "weather"}}</tool_call>"#;
        let result = parser.parse(input);
        assert!(result.ok);
        assert_eq!(result.calls.len(), 1);
        assert_eq!(result.calls[0].name, "search");
    }

    #[test]
    fn test_tool_call_with_parameters() {
        let parser = GenericParser::new();
        let input = r#"<tool_call>{"name": "search", "parameters": {"q": "hello"}}</tool_call>"#;
        let result = parser.parse(input);
        assert!(result.ok);
        assert_eq!(result.calls.len(), 1);
        assert_eq!(result.calls[0].name, "search");
    }

    #[test]
    fn test_tool_call_with_input() {
        let parser = GenericParser::new();
        let input = r#"<tool_call>{"name": "search", "input": {"q": "hello"}}</tool_call>"#;
        let result = parser.parse(input);
        assert!(result.ok);
        assert_eq!(result.calls.len(), 1);
        assert_eq!(result.calls[0].name, "search");
    }

    #[test]
    fn test_multiple_tool_calls() {
        let parser = GenericParser::new();
        let input = concat!(
            r#"<tool_call>{"name": "a", "arguments": {}}</tool_call>"#,
            "some text",
            r#"<tool_call>{"name": "b", "arguments": {}}</tool_call>"#,
        );
        let result = parser.parse(input);
        assert!(result.ok);
        assert_eq!(result.calls.len(), 2);
        assert_eq!(result.calls[0].name, "a");
        assert_eq!(result.calls[1].name, "b");
    }

    #[test]
    fn test_openai_tool_calls_array_in_block() {
        let parser = GenericParser::new();
        let input = format!(
            r#"<tool_call>{{"tool_calls": [{}]}}</tool_call>"#,
            r#"{"id": "call_1", "function": {"name": "get_weather", "arguments": "{\"loc\": \"NYC\"}"}}"#
        );
        let result = parser.parse(&input);
        assert!(result.ok);
        assert_eq!(result.calls.len(), 1);
        assert_eq!(result.calls[0].name, "get_weather");
    }

    #[test]
    fn test_fenced_json_block() {
        let parser = GenericParser::new();
        let input = format!(
            "{}{}{}",
            "Here is the result:\n```json\n",
            r#"{"name": "search", "arguments": {"q": "test"}}"#,
            "\n```\nEnd."
        );
        let result = parser.parse(&input);
        assert!(result.ok);
        assert_eq!(result.calls.len(), 1);
        assert_eq!(result.calls[0].name, "search");
        assert!(
            result.calls[0]
                .warnings
                .as_ref()
                .unwrap()
                .contains(&"parsed from fenced code block".to_string())
        );
    }

    #[test]
    fn test_fenced_json_block_no_lang() {
        let parser = GenericParser::new();
        let input = format!(
            "{}{}{}",
            "```\n", r#"{"name": "search", "arguments": {"q": "test"}}"#, "\n```"
        );
        let result = parser.parse(&input);
        assert!(result.ok);
        assert_eq!(result.calls.len(), 1);
        assert_eq!(result.calls[0].name, "search");
    }

    #[test]
    fn test_fenced_openai_tool_calls_array() {
        let parser = GenericParser::new();
        let input = format!(
            "{}{}{}",
            "```json\n",
            r#"{"tool_calls": [{"id": "1", "function": {"name": "f", "arguments": "{}"}}]}"#,
            "\n```"
        );
        let result = parser.parse(&input);
        assert!(result.ok);
        assert_eq!(result.calls.len(), 1);
        assert_eq!(result.calls[0].name, "f");
    }

    #[test]
    fn test_bare_json_object() {
        let parser = GenericParser::new();
        let input = r#"{"name": "get_weather", "arguments": {"location": "NYC"}}"#;
        let result = parser.parse(input);
        assert!(result.ok);
        assert_eq!(result.calls.len(), 1);
        assert_eq!(result.calls[0].name, "get_weather");
        assert!(
            result.calls[0]
                .warnings
                .as_ref()
                .unwrap()
                .contains(&"parsed from bare JSON text".to_string())
        );
    }

    #[test]
    fn test_bare_json_openai_array() {
        let parser = GenericParser::new();
        let input = format!(
            "{}",
            r#"{"tool_calls": [{"id": "1", "function": {"name": "f", "arguments": "{}"}}]}"#,
        );
        let result = parser.parse(&input);
        assert!(result.ok);
        assert_eq!(result.calls.len(), 1);
        assert_eq!(result.calls[0].name, "f");
    }

    #[test]
    fn test_bare_json_no_name_does_not_parse() {
        let parser = GenericParser::new();
        let input = r#"{"foo": "bar"}"#;
        let result = parser.parse(input);
        assert!(result.ok);
        assert!(result.calls.is_empty());
    }

    #[test]
    fn test_arguments_as_json_string() {
        let parser = GenericParser::new();
        let input = r#"<tool_call>{"name": "get_weather", "arguments": "{\"loc\": \"NYC\", \"unit\": \"c\"}"}</tool_call>"#;
        let result = parser.parse(input);
        assert!(result.ok);
        assert_eq!(result.calls.len(), 1);
        if let serde_json::Value::Object(ref args) = result.calls[0].arguments {
            assert_eq!(args.get("loc").and_then(|v| v.as_str()), Some("NYC"));
        } else {
            panic!("expected object arguments");
        }
    }

    #[test]
    fn test_with_call_id() {
        let parser = GenericParser::new();
        let input = r#"<tool_call>{"id": "my_id_123", "name": "fn", "arguments": {}}</tool_call>"#;
        let result = parser.parse(input);
        assert!(result.ok);
        assert_eq!(result.calls[0].id, "my_id_123");
    }

    #[test]
    fn test_with_call_id_via_call_id_key() {
        let parser = GenericParser::new();
        let input =
            r#"<tool_call>{"call_id": "cid_456", "name": "fn", "arguments": {}}</tool_call>"#;
        let result = parser.parse(input);
        assert!(result.ok);
        assert_eq!(result.calls[0].id, "cid_456");
    }

    #[test]
    fn test_non_tool_content_before_after() {
        let parser = GenericParser::new();
        let input = r#"Let me check the weather.
<tool_call>{"name": "get_weather", "arguments": {"loc": "NYC"}}</tool_call>
The result shows it's sunny."#;
        let result = parser.parse(input);
        assert!(result.ok);
        assert_eq!(result.calls.len(), 1);
        assert!(result.content.contains("Let me check"));
        assert!(result.content.contains("The result shows"));
        assert!(!result.content.contains("<tool_call>"));
    }

    #[test]
    fn test_malformed_block_no_fallback_when_prose() {
        // Malformed blocks with prose between them should NOT trigger standalone detection
        let parser = GenericParser::new();
        let input = concat!(
            r#"Here is an example of how tool calls work:"#,
            r#"<tool_call>This is not valid JSON</tool_call>"#,
            r#"You can see it uses XML-like tags."#,
        );
        let result = parser.parse(input);
        // With prose, we fall through to stripping blocks, should be ok with no calls
        assert!(result.ok);
        assert!(result.calls.is_empty());
    }

    #[test]
    fn test_sanitize_reasoning_tags_inside_content() {
        let parser = GenericParser::new();
        let input = r#"<think>Let me think</think><tool_call>{"name": "fn", "arguments": {}}</tool_call></tool_call>"#;
        let result = parser.parse(input);
        assert!(result.ok);
        assert_eq!(result.calls.len(), 1);
        assert_eq!(result.calls[0].name, "fn");
    }

    #[test]
    fn test_string_arguments_returns_empty_object_when_not_json() {
        let parser = GenericParser::new();
        let input = r#"<tool_call>{"name": "fn", "arguments": "not-json-string"}</tool_call>"#;
        let result = parser.parse(input);
        assert!(result.ok);
        assert_eq!(result.calls.len(), 1);
        // Non-JSON string arguments → empty object
        assert_eq!(
            result.calls[0].arguments,
            serde_json::Value::Object(serde_json::Map::new())
        );
    }

    #[test]
    fn test_empty_tool_call_block_skipped() {
        let parser = GenericParser::new();
        let input = format!(
            "<tool_call></tool_call><tool_call>{}</tool_call>",
            r#"{"name": "fn", "arguments": {}}"#
        );
        let result = parser.parse(&input);
        assert!(result.ok);
        assert_eq!(result.calls.len(), 1);
        assert_eq!(result.calls[0].name, "fn");
    }

    #[test]
    fn test_multiple_fenced_blocks() {
        let parser = GenericParser::new();
        let input = format!(
            "{}{}{}{}{}",
            "First:\n```json\n",
            r#"{"name": "a", "arguments": {}}"#,
            "\n```\nSecond:\n```\n",
            r#"{"name": "b", "arguments": {}}"#,
            "\n```"
        );
        let result = parser.parse(&input);
        assert!(result.ok);
        assert_eq!(result.calls.len(), 2);
        assert_eq!(result.calls[0].name, "a");
        assert_eq!(result.calls[1].name, "b");
    }

    #[test]
    fn test_prefers_tool_call_over_fenced() {
        let parser = GenericParser::new();
        let input = format!(
            "{}{}{}{}",
            r#"<tool_call>{"name": "from_tag", "arguments": {}}</tool_call>"#,
            "```json\n",
            r#"{"name": "from_fence", "arguments": {}}"#,
            "\n```"
        );
        let result = parser.parse(&input);
        assert!(result.ok);
        // Strategy 1 runs first - should capture the <tool_call> one
        // The fenced one is captured too - both should appear
        assert_eq!(result.calls.len(), 2);
    }

    #[test]
    fn test_factory_creates_parser() {
        let parser = create_generic_parser();
        assert_eq!(parser.id(), "generic");
        assert!(parser.description().len() > 0);
        assert_eq!(parser.model_families(), &["Any", "Unknown", "Generic"]);
    }

    #[test]
    fn test_extract_fenced_json_blocks() {
        let input = r#"Some text
```json
{"a": 1}
```
More text
```
{"b": 2}
```
End"#;
        let blocks = extract_fenced_json_blocks(input);
        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0].trim(), r#"{"a": 1}"#);
        assert_eq!(blocks[1].trim(), r#"{"b": 2}"#);
    }

    #[test]
    fn test_no_fenced_blocks() {
        let blocks = extract_fenced_json_blocks("no blocks here");
        assert!(blocks.is_empty());
    }

    #[test]
    fn test_is_standalone_tool_call_content_standalone() {
        let content = r#"
Some intro
<tool_call>{"name": "fn", "arguments": {}}</tool_call>
<tool_call>{"name": "fn2", "arguments": {}}</tool_call>
Summary"#;
        let blocks = extract_delimited_blocks(content, "<tool_call>", "</tool_call>");
        // blocks.before.split_whitespace().count() == 2 → "Some intro"
        // it's <= 5 words, so standalone = true
        assert!(is_standalone_tool_call_content(content, &blocks));
    }

    #[test]
    fn test_is_standalone_tool_call_content_not_standalone_long_preamble() {
        let content = r#"This is a long preamble that should make the content not be standalone.
<tool_call>{"name": "fn", "arguments": {}}</tool_call></tool_call>"#;
        let blocks = extract_delimited_blocks(content, "<tool_call>", "</tool_call>");
        // blocks.before.split_whitespace().count() > 5
        assert!(!is_standalone_tool_call_content(content, &blocks));
    }

    #[test]
    fn test_is_standalone_tool_call_content_inside_fence() {
        let content = r#"```json
<tool_call>{"name": "fn", "arguments": {}}</tool_call>
```"#;
        let blocks = extract_delimited_blocks(content, "<tool_call>", "</tool_call>");
        assert!(!is_standalone_tool_call_content(content, &blocks));
    }
}
