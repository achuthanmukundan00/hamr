//! DeepSeek V3 / V3.1 tool-call parser.
//!
//! Ported from `packages/coding-agent/src/hamr/providers/parsers/deepseek.ts`.
//!
//! Parses DeepSeek tool-call format. DeepSeek models output tool calls in
//! a format similar to Hermes, using XML-style tags with JSON inside:
//!
//! ```text
//! <tool_call>
//! {"name": "get_weather", "arguments": {"location": "SF", "unit": "celsius"}}
//! </tool_call>
//! ```
//!
//! DeepSeek V3.1 may also use a `<｜tool▁call▁begin｜>...<｜tool▁call▁end｜>` format
//! with a special token prefix.
//!
//! DeepSeek reasoning models (R1) may emit tool calls inside `<think>` blocks
//! — the `sanitize_reasoning_tags` step strips those before parsing.
//!
//! Reference: vLLM
//!   --tool-call-parser deepseek_v3
//!   --tool-call-parser deepseek_v31
//!   vllm/entrypoints/openai/tool_parsers/deepseek_v3_tool_parser.py
//!   vllm/entrypoints/openai/tool_parsers/deepseek_v31_tool_parser.py

use super::types::{ParsedToolCall, ToolCallParseResult, ToolCallParser};
use super::utils::{
    extract_delimited_blocks, generate_call_id, safe_json_parse, sanitize_reasoning_tags,
};

// ─── Constants ───────────────────────────────────────────

const DSV3_ID: &str = "deepseek_v3";
const DSV3_DESC: &str = "DeepSeek V3 format: <tool_call>{...}</tool_call>";
const DSV3_FAMILIES: &[&str] = &["DeepSeek V3", "DeepSeek Chat", "DeepSeek R1", "DeepSeek"];

const DSV31_ID: &str = "deepseek_v31";
const DSV31_DESC: &str = "DeepSeek V3.1 format: special-token-delimited tool calls with JSON";
const DSV31_FAMILIES: &[&str] = &["DeepSeek V3.1"];

const DEEPSEEK_BEGIN: &str = "<｜tool▁call▁begin｜>";
const DEEPSEEK_END: &str = "<｜tool▁call▁end｜>";

// ─── DeepSeek V3 parser ──────────────────────────────────

#[derive(Debug, Clone, Default)]
pub struct DeepseekV3Parser;

impl DeepseekV3Parser {
    pub fn new() -> Self {
        Self
    }
}

impl ToolCallParser for DeepseekV3Parser {
    fn id(&self) -> &str {
        DSV3_ID
    }

    fn description(&self) -> &str {
        DSV3_DESC
    }

    fn model_families(&self) -> &[&str] {
        DSV3_FAMILIES
    }

    fn parse(&self, content: &str) -> ToolCallParseResult {
        let sanitized = sanitize_reasoning_tags(content);
        parse_deepseek_content(&sanitized, DSV3_ID)
    }
}

// ─── DeepSeek V3.1 parser ────────────────────────────────

#[derive(Debug, Clone, Default)]
pub struct DeepseekV31Parser;

impl DeepseekV31Parser {
    pub fn new() -> Self {
        Self
    }
}

impl ToolCallParser for DeepseekV31Parser {
    fn id(&self) -> &str {
        DSV31_ID
    }

    fn description(&self) -> &str {
        DSV31_DESC
    }

    fn model_families(&self) -> &[&str] {
        DSV31_FAMILIES
    }

    fn parse(&self, content: &str) -> ToolCallParseResult {
        let sanitized = sanitize_reasoning_tags(content);
        parse_deepseek_content(&sanitized, DSV31_ID)
    }
}

// ─── Shared implementation ───────────────────────────────

/// Try Hermes-style `<tool_call>...</tool_call>` delimiters first, then
/// fall back to special token delimiters `<｜tool▁call▁begin｜>...<｜tool▁call▁end｜>`.
fn parse_deepseek_content(sanitized: &str, parser_id: &str) -> ToolCallParseResult {
    // Try <tool_call>...</tool_call> delimiters first (Hermes-compatible style)
    let delimited = extract_delimited_blocks(sanitized, "<tool_call>", "</tool_call>");

    if !delimited.blocks.is_empty() {
        return parse_hermes_style_blocks(&delimited, parser_id);
    }

    // Try special token delimiters
    let special_delimited = extract_delimited_blocks(sanitized, DEEPSEEK_BEGIN, DEEPSEEK_END);

    if !special_delimited.blocks.is_empty() {
        return parse_hermes_style_blocks(&special_delimited, parser_id);
    }

    ToolCallParseResult::ok(parser_id, Vec::new(), sanitized)
}

/// Parse blocks extracted from tool-call delimiters. Each block is expected
/// to be a JSON object with at least a `"name"` field and optionally
/// `"arguments"` / `"parameters"` / `"input"` fields.
fn parse_hermes_style_blocks(
    delimited: &super::utils::DelimitedResult,
    parser_id: &str,
) -> ToolCallParseResult {
    let mut calls: Vec<ParsedToolCall> = Vec::new();
    let mut warnings: Vec<String> = Vec::new();

    for (i, block) in delimited.blocks.iter().enumerate() {
        let trimmed = block.trim();
        if trimmed.is_empty() {
            continue;
        }

        // Assemble non-tool content for error reporting
        let non_tool_content = assemble_non_tool_content(delimited);

        // Parse the JSON block
        let parsed = match safe_json_parse(trimmed) {
            Ok(v) => v,
            Err(e) => {
                return ToolCallParseResult::err(
                    parser_id,
                    &non_tool_content,
                    format!("DeepSeek tool_call block {}: {}", i + 1, e),
                );
            }
        };

        // Must be a JSON object, not array or primitive
        let obj = match parsed.as_object() {
            Some(o) => o,
            None => {
                return ToolCallParseResult::err(
                    parser_id,
                    &non_tool_content,
                    format!("DeepSeek tool_call block {}: expected JSON object", i + 1),
                );
            }
        };

        // Extract the tool name from one of the common keys
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
                return ToolCallParseResult::err(
                    parser_id,
                    &non_tool_content,
                    format!("DeepSeek tool_call block {}: missing \"name\"", i + 1),
                );
            }
        };

        // Extract arguments (can be "arguments", "parameters", or "input")
        let raw_args = obj
            .get("arguments")
            .or_else(|| obj.get("parameters"))
            .or_else(|| obj.get("input"));

        let args: serde_json::Value = match raw_args {
            Some(serde_json::Value::String(s)) => {
                // String arguments need parsing
                match safe_json_parse(s) {
                    Ok(parsed_args) if parsed_args.is_object() => parsed_args,
                    _ => {
                        warnings.push(format!(
                            "DeepSeek block {}: arguments string parse failed",
                            i + 1
                        ));
                        serde_json::Value::Object(serde_json::Map::new())
                    }
                }
            }
            Some(v @ serde_json::Value::Object(_)) => v.clone(),
            Some(other) => other.clone(), // preserve the value as-is
            None => serde_json::Value::Object(serde_json::Map::new()),
        };

        // Extract optional call id
        let call_id = obj
            .get("id")
            .or_else(|| obj.get("call_id"))
            .and_then(|v| v.as_str());

        let generated_id = generate_call_id(call_id, Some(i + 1));

        let mut call = ParsedToolCall::new(generated_id, name, args);
        call.raw_source = Some(trimmed.to_string());
        call.parser_id = Some(parser_id.to_string());

        calls.push(call);
    }

    let non_tool_content = assemble_non_tool_content(delimited);

    let mut result = ToolCallParseResult::ok(parser_id, calls, non_tool_content);
    if !warnings.is_empty() {
        result.warnings = Some(warnings);
    }
    result
}

/// Concatenate content fragments that are not inside tool-call blocks.
fn assemble_non_tool_content(delimited: &super::utils::DelimitedResult) -> String {
    let mut parts: Vec<&str> = Vec::new();
    let before = delimited.before.trim();
    if !before.is_empty() {
        parts.push(before);
    }
    for b in &delimited.between {
        let trimmed = b.trim();
        if !trimmed.is_empty() {
            parts.push(trimmed);
        }
    }
    let after = delimited.after.trim();
    if !after.is_empty() {
        parts.push(after);
    }
    parts.join("\n").trim().to_string()
}

// ─── Factory functions ───────────────────────────────────

/// Create a DeepSeek V3 parser instance.
pub fn create_deepseek_v3_parser() -> Box<dyn ToolCallParser> {
    Box::new(DeepseekV3Parser::new())
}

/// Create a DeepSeek V3.1 parser instance.
pub fn create_deepseek_v31_parser() -> Box<dyn ToolCallParser> {
    Box::new(DeepseekV31Parser::new())
}

// ─── Tests ───────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ─── V3 parser ──────────────────────────────────────

    #[test]
    fn test_v3_single_tool_call() {
        let parser = DeepseekV3Parser::new();
        let content = r#"I'll check the weather.

<tool_call>
{"name": "get_weather", "arguments": {"location": "San Francisco", "unit": "celsius"}}
</tool_call>"#;

        let result = parser.parse(content);
        assert!(result.ok, "expected ok, got: {:?}", result.error);
        assert_eq!(result.calls.len(), 1);
        assert_eq!(result.calls[0].name, "get_weather");
        assert_eq!(result.calls[0].arguments["location"], "San Francisco");
        assert_eq!(result.calls[0].arguments["unit"], "celsius");
        assert!(result.content.contains("I'll check the weather."));
        assert_eq!(result.parser_id, "deepseek_v3");
    }

    #[test]
    fn test_v3_multiple_tool_calls() {
        let parser = DeepseekV3Parser::new();
        let content = r#"Let me do two things.

<tool_call>
{"name": "get_weather", "arguments": {"location": "NYC"}}
</tool_call>
<tool_call>
{"name": "get_time", "arguments": {"timezone": "America/New_York"}}
</tool_call>
Done."#;

        let result = parser.parse(content);
        assert!(result.ok);
        assert_eq!(result.calls.len(), 2);
        assert_eq!(result.calls[0].name, "get_weather");
        assert_eq!(result.calls[1].name, "get_time");
        assert!(result.content.contains("Let me do two things."));
        assert!(result.content.contains("Done."));
    }

    #[test]
    fn test_v3_no_tool_calls() {
        let parser = DeepseekV3Parser::new();
        let content = "Just a regular response with no tool calls.";
        let result = parser.parse(content);
        assert!(result.ok);
        assert_eq!(result.calls.len(), 0);
        assert_eq!(result.content, content);
    }

    #[test]
    fn test_v3_strips_reasoning() {
        let parser = DeepseekV3Parser::new();
        let content = r#"<think>I need to check the weather first.</think>

<tool_call>
{"name": "get_weather", "arguments": {"location": "SF"}}
</tool_call>"#;

        let result = parser.parse(content);
        assert!(result.ok);
        assert_eq!(result.calls.len(), 1);
        assert_eq!(result.calls[0].name, "get_weather");
        // Reasoning tag should be stripped
        assert!(!result.content.contains("<think>"));
    }

    #[test]
    fn test_v3_missing_name() {
        let parser = DeepseekV3Parser::new();
        let content = r#"
<tool_call>
{"arguments": {"location": "SF"}}
</tool_call>"#;

        let result = parser.parse(content);
        assert!(!result.ok, "expected error for missing name");
        assert!(result.error.as_ref().unwrap().contains(r#"missing "name""#));
    }

    #[test]
    fn test_v3_invalid_json_in_block() {
        let parser = DeepseekV3Parser::new();
        let content = r#"
<tool_call>
{invalid json here}
</tool_call>"#;

        let result = parser.parse(content);
        assert!(!result.ok, "expected error for invalid JSON");
        assert!(result.error.as_ref().unwrap().contains("could not parse"));
    }

    #[test]
    fn test_v3_empty_block() {
        let parser = DeepseekV3Parser::new();
        let content = r#"some text<tool_call></tool_call>more text"#;
        let result = parser.parse(content);
        assert!(result.ok);
        assert_eq!(result.calls.len(), 0);
        assert!(result.content.contains("some text"));
        assert!(result.content.contains("more text"));
    }

    #[test]
    fn test_v3_arguments_as_string() {
        let parser = DeepseekV3Parser::new();
        let content = r#"
<tool_call>
{"name": "search", "arguments": "{\"query\": \"hello world\"}"}
</tool_call>"#;

        let result = parser.parse(content);
        assert!(result.ok);
        assert_eq!(result.calls.len(), 1);
        assert_eq!(result.calls[0].name, "search");
        assert_eq!(result.calls[0].arguments["query"], "hello world");
    }

    #[test]
    fn test_v3_call_id_from_block() {
        let parser = DeepseekV3Parser::new();
        let content = r#"
<tool_call>
{"id": "call_abc123", "name": "get_weather", "arguments": {"loc": "SF"}}
</tool_call>"#;

        let result = parser.parse(content);
        assert!(result.ok);
        assert_eq!(result.calls.len(), 1);
        assert_eq!(result.calls[0].id, "call_abc123");
    }

    #[test]
    fn test_v3_call_id_fallback() {
        let parser = DeepseekV3Parser::new();
        let content = r#"
<tool_call>
{"name": "get_weather", "arguments": {"loc": "SF"}}
</tool_call>"#;

        let result = parser.parse(content);
        assert!(result.ok);
        assert_eq!(result.calls.len(), 1);
        // Should fall back to call_1
        assert_eq!(result.calls[0].id, "call_1");
    }

    #[test]
    fn test_v3_parser_id_detection() {
        // V3 parser should be detected for deepseek-v3 models
        // (tested via types::detect_parser_id)
        assert_eq!(
            crate::hamr::providers::parsers::types::detect_parser_id("deepseek-v3"),
            Some("deepseek_v3")
        );
        assert_eq!(
            crate::hamr::providers::parsers::types::detect_parser_id("deepseek-r1"),
            Some("deepseek_v3")
        );
        assert_eq!(
            crate::hamr::providers::parsers::types::detect_parser_id("deepseek"),
            Some("deepseek_v3")
        );
    }

    // ─── V3.1 parser ────────────────────────────────────

    #[test]
    fn test_v31_special_tokens() {
        let parser = DeepseekV31Parser::new();
        let content = format!(
            r#"I'll check the weather.

{begin}
{{"name": "get_weather", "arguments": {{"location": "SF", "unit": "celsius"}}}}
{end}"#,
            begin = DEEPSEEK_BEGIN,
            end = DEEPSEEK_END
        );

        let result = parser.parse(&content);
        assert!(result.ok, "expected ok, got: {:?}", result.error);
        assert_eq!(result.calls.len(), 1);
        assert_eq!(result.calls[0].name, "get_weather");
        assert_eq!(result.calls[0].arguments["location"], "SF");
    }

    #[test]
    fn test_v31_parser_id_detection() {
        assert_eq!(
            crate::hamr::providers::parsers::types::detect_parser_id("deepseek-v3.1"),
            Some("deepseek_v31")
        );
    }

    // ─── Factory tests ──────────────────────────────────

    #[test]
    fn test_create_parsers() {
        let v3 = create_deepseek_v3_parser();
        assert_eq!(v3.id(), "deepseek_v3");

        let v31 = create_deepseek_v31_parser();
        assert_eq!(v31.id(), "deepseek_v31");
    }

    // ─── Edge cases ─────────────────────────────────────

    #[test]
    fn test_no_delimiters_returns_content_as_is() {
        let parser = DeepseekV3Parser::new();
        let content = "Just plain text without any tool calls.";
        let result = parser.parse(content);
        assert!(result.ok);
        assert_eq!(result.calls.len(), 0);
        assert_eq!(result.content, content);
    }

    #[test]
    fn test_whitespace_only_content() {
        let parser = DeepseekV3Parser::new();
        let result = parser.parse("   \n  \t  ");
        assert!(result.ok);
        assert_eq!(result.calls.len(), 0);
    }

    #[test]
    fn test_tool_call_with_alternative_keys() {
        let parser = DeepseekV3Parser::new();
        // Use "function" instead of "name", and "parameters" instead of "arguments"
        let content = r#"
<tool_call>
{"function": "get_weather", "parameters": {"location": "NYC"}}
</tool_call>"#;

        let result = parser.parse(content);
        assert!(result.ok);
        assert_eq!(result.calls.len(), 1);
        assert_eq!(result.calls[0].name, "get_weather");
        assert_eq!(result.calls[0].arguments["location"], "NYC");
    }

    #[test]
    fn test_tool_call_with_input_key() {
        let parser = DeepseekV3Parser::new();
        let content = r#"
<tool_call>
{"name": "search", "input": {"q": "rust"}}
</tool_call>"#;

        let result = parser.parse(content);
        assert!(result.ok);
        assert_eq!(result.calls.len(), 1);
        assert_eq!(result.calls[0].name, "search");
        assert_eq!(result.calls[0].arguments["q"], "rust");
    }

    #[test]
    fn test_v3_non_object_json_block() {
        let parser = DeepseekV3Parser::new();
        // Array is not a valid tool call
        let content = r#"
<tool_call>
["not", "an", "object"]
</tool_call>"#;

        let result = parser.parse(content);
        assert!(!result.ok);
        assert!(
            result
                .error
                .as_ref()
                .unwrap()
                .contains("expected JSON object")
        );
    }

    #[test]
    fn test_v31_fallback_to_regular_tags() {
        let parser = DeepseekV31Parser::new();
        // V3.1 parser should also handle regular <tool_call> tags
        let content = r#"
<tool_call>
{"name": "test", "arguments": {"val": 1}}
</tool_call>"#;

        let result = parser.parse(content);
        assert!(result.ok);
        assert_eq!(result.calls.len(), 1);
        assert_eq!(result.calls[0].name, "test");
    }

    #[test]
    fn test_multiple_blocks_with_between_content() {
        let parser = DeepseekV3Parser::new();
        let content = r#"First thought.

<tool_call>
{"name": "search", "arguments": {"q": "a"}}
</tool_call>

Some intermediate reasoning.

<tool_call>
{"name": "search", "arguments": {"q": "b"}}
</tool_call>

Final answer."#;

        let result = parser.parse(content);
        assert!(result.ok);
        assert_eq!(result.calls.len(), 2);
        assert!(result.content.contains("First thought."));
        assert!(result.content.contains("Some intermediate reasoning."));
        assert!(result.content.contains("Final answer."));
    }

    #[test]
    fn test_raw_source_on_calls() {
        let parser = DeepseekV3Parser::new();
        let content = r#"
<tool_call>
{"name": "test", "arguments": {}}
</tool_call>"#;

        let result = parser.parse(content);
        assert!(result.ok);
        assert_eq!(result.calls.len(), 1);
        let raw = result.calls[0].raw_source.as_deref().unwrap();
        assert!(raw.contains("test"));
        assert!(raw.contains("arguments"));
    }

    #[test]
    fn test_parser_id_on_calls() {
        let parser = DeepseekV3Parser::new();
        let content = r#"
<tool_call>
{"name": "test", "arguments": {}}
</tool_call>"#;

        let result = parser.parse(content);
        assert!(result.ok);
        assert_eq!(result.calls[0].parser_id.as_deref().unwrap(), "deepseek_v3");
    }

    #[test]
    fn test_unclosed_delimiters() {
        let parser = DeepseekV3Parser::new();
        let content = r#"
<tool_call>
{"name": "test", "arguments": {}}
"#;

        let result = parser.parse(content);
        // Without a closing tag, no blocks should be extracted
        assert!(result.ok);
        assert_eq!(result.calls.len(), 0);
        // Content should remain
        assert!(result.content.contains("test"));
    }
}
