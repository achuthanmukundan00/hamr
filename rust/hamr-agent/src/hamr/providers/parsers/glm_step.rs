//! GLM and Step tool-call parsers.
//!
//! Ported from `packages/coding-agent/src/hamr/providers/parsers/glm-step.ts`.
//!
//! GLM 4.5/4.7 models and Step 3/3.5 models use specialized tool-call formats.
//!
//! GLM format (as documented by vLLM):
//!   Uses a function-call token followed by a JSON object:
//!   `<|tool_call|>{"name": "get_weather", "arguments": {"location": "SF"}}`
//!
//! Step 3 format:
//!   Uses XML-style tags similar to Qwen but with Step-specific markup.

use serde_json::Value;

use super::types::{ParsedToolCall, ToolCallParseResult, ToolCallParser};
use super::utils::{
    coerce_value, extract_delimited_blocks, generate_call_id, safe_json_parse,
    sanitize_reasoning_tags,
};

// ─── ParsedToolCall builder helper ───────────────────────

fn make_parsed_call(
    name: &str,
    args: Value,
    raw_source: &str,
    index: usize,
    parser_id: &str,
) -> ParsedToolCall {
    let args_obj = match args {
        Value::Object(map) => Value::Object(map),
        v => v,
    };
    ParsedToolCall {
        id: generate_call_id(None, Some(index)),
        name: name.to_string(),
        arguments: args_obj,
        raw_source: Some(raw_source.to_string()),
        parser_id: Some(parser_id.to_string()),
        warnings: None,
    }
}

// ─── JSON balanced-brace scanner ─────────────────────────

/// Find the end of a balanced JSON value starting at the first `{`.
/// Returns `Some(end_index_inclusive)` on success, `None` if unbalanced.
fn find_balanced_json_end(body: &str) -> Option<usize> {
    let mut depth: i32 = 0;
    let mut in_string = false;
    let mut escape = false;

    for (i, ch) in body.char_indices() {
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
        if ch == '{' {
            depth += 1;
        } else if ch == '}' {
            depth -= 1;
            if depth == 0 {
                return Some(i);
            }
        }
    }
    None
}

// ─── Special token block parser (GLM) ────────────────────

/// Parse `<|tool_call|>`-delimited blocks from content.
/// Each block is a JSON object with `name`/`arguments` fields.
fn parse_special_token_blocks(content: &str, token: &str, parser_id: &str) -> Vec<ParsedToolCall> {
    let mut calls: Vec<ParsedToolCall> = Vec::new();
    let mut remaining = content;
    let mut call_index = 0usize;

    loop {
        let idx = match remaining.find(token) {
            Some(p) => p,
            None => break,
        };

        // Advance past the token, then skip leading whitespace
        remaining = &remaining[idx + token.len()..];
        remaining = remaining.trim_start();

        if remaining.starts_with('{') {
            // Find balanced JSON
            if let Some(end) = find_balanced_json_end(remaining) {
                let json_str = &remaining[..=end];
                let parsed = safe_json_parse(json_str);
                if let Ok(serde_json::Value::Object(obj)) = parsed {
                    let name = obj
                        .get("name")
                        .or_else(|| obj.get("tool_name"))
                        .and_then(serde_json::Value::as_str)
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty());

                    if let Some(name) = name {
                        call_index += 1;
                        let raw_args = obj.get("arguments").or_else(|| obj.get("parameters"));
                        let args = match raw_args {
                            Some(serde_json::Value::String(s)) => {
                                if let Ok(serde_json::Value::Object(_)) = safe_json_parse(s) {
                                    safe_json_parse(s).unwrap_or(serde_json::Value::Object(
                                        serde_json::Map::new(),
                                    ))
                                } else {
                                    serde_json::Value::Object(serde_json::Map::new())
                                }
                            }
                            Some(serde_json::Value::Object(m)) => {
                                serde_json::Value::Object(m.clone())
                            }
                            Some(other) => other.clone(),
                            None => serde_json::Value::Object(serde_json::Map::new()),
                        };

                        calls.push(make_parsed_call(
                            &name, args, json_str, call_index, parser_id,
                        ));
                    }
                }
                remaining = &remaining[end + 1..];
            } else {
                break; // unclosed JSON — stop
            }
        } else {
            // No JSON after token — advance past any non-{ content
            if let Some(next_token) = remaining.find(token) {
                remaining = &remaining[next_token..];
            } else {
                break;
            }
        }
    }

    calls
}

/// Strip all tool-call token blocks and everything between them.
/// Returns the remaining content with these blocks removed.
/// This implements the same logic as the TS pattern `<token>[\s\S]*?(?=<|$)`.
fn strip_token_blocks(content: &str, token: &str) -> String {
    let mut result = String::new();
    let mut pos = 0;
    loop {
        let idx = content[pos..].find(token);
        match idx {
            Some(start) => {
                // Copy everything before the token
                result.push_str(&content[pos..pos + start]);
                // Skip past the token
                let after_token = pos + start + token.len();
                // Skip everything until we hit '<' (next tag) or end of string
                let rest = &content[after_token..];
                let next_tag = rest.find('<').unwrap_or(rest.len());
                pos = after_token + next_tag;
            }
            None => {
                result.push_str(&content[pos..]);
                break;
            }
        }
    }
    result.trim().to_string()
}

/// Parse content using the `<|tool_call|>` token strategy (GLM 4.5/4.7).
fn parse_glm_with_special_tokens(
    content: &str,
    token: &str,
    parser_id: &str,
) -> ToolCallParseResult {
    let sanitized = sanitize_reasoning_tags(content);
    let calls = parse_special_token_blocks(&sanitized, token, parser_id);

    if !calls.is_empty() {
        // Remove all token-delimited blocks to extract non-tool content
        let non_tool = strip_token_blocks(&sanitized, token);
        return ToolCallParseResult::ok(parser_id, calls, non_tool);
    }

    // Fallback: Hermes-style
    parse_hermes_fallback(&sanitized, parser_id)
}

// ─── Hermes-style fallback ────────────────────────────────

/// Parse Hermes-style `<tool_call>` / `<function_call>` delimited blocks.
fn parse_hermes_fallback(sanitized: &str, parser_id: &str) -> ToolCallParseResult {
    let tc = extract_delimited_blocks(sanitized, "<tool_call>", "</tool_call>");
    let fc = extract_delimited_blocks(sanitized, "<function_call>", "</function_call>");
    let all_blocks: Vec<&str> = tc
        .blocks
        .iter()
        .chain(fc.blocks.iter())
        .map(|s| s.as_str())
        .collect();

    if all_blocks.is_empty() {
        return ToolCallParseResult::ok(parser_id, vec![], sanitized);
    }

    let mut calls: Vec<ParsedToolCall> = Vec::new();
    for (i, block) in all_blocks.iter().enumerate() {
        let block = block.trim();
        if block.is_empty() {
            continue;
        }
        let parsed = safe_json_parse(block);
        let obj = match parsed {
            Ok(Value::Object(o)) => o,
            Ok(_) => {
                return ToolCallParseResult::err(
                    parser_id,
                    sanitized,
                    format!("{} block {}: expected JSON object", parser_id, i + 1),
                );
            }
            Err(e) => {
                return ToolCallParseResult::err(
                    parser_id,
                    sanitized,
                    format!("{} block {}: {}", parser_id, i + 1, e),
                );
            }
        };

        let name = obj
            .get("name")
            .or_else(|| obj.get("tool_name"))
            .and_then(Value::as_str)
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());

        let name = match name {
            Some(n) => n,
            None => {
                return ToolCallParseResult::err(
                    parser_id,
                    sanitized,
                    format!(r#"{} block {}: missing "name""#, parser_id, i + 1),
                );
            }
        };

        let raw_args = obj.get("arguments").or_else(|| obj.get("parameters"));
        let args = match raw_args {
            Some(Value::String(s)) => {
                if let Ok(Value::Object(_)) = safe_json_parse(s) {
                    safe_json_parse(s).unwrap_or(Value::Object(serde_json::Map::new()))
                } else {
                    Value::Object(serde_json::Map::new())
                }
            }
            Some(Value::Object(m)) => Value::Object(m.clone()),
            Some(other) => other.clone(),
            None => Value::Object(serde_json::Map::new()),
        };

        calls.push(make_parsed_call(&name, args, block, i + 1, parser_id));
    }

    // Reconstruct non-tool content from all delimited segments
    let mut non_tool_parts: Vec<&str> = Vec::new();
    if !tc.before.trim().is_empty() {
        non_tool_parts.push(tc.before.trim());
    }
    for b in &tc.between {
        if !b.trim().is_empty() {
            non_tool_parts.push(b.trim());
        }
    }
    if !tc.after.trim().is_empty() {
        non_tool_parts.push(tc.after.trim());
    }
    if !fc.before.trim().is_empty() {
        non_tool_parts.push(fc.before.trim());
    }
    for b in &fc.between {
        if !b.trim().is_empty() {
            non_tool_parts.push(b.trim());
        }
    }
    if !fc.after.trim().is_empty() {
        non_tool_parts.push(fc.after.trim());
    }

    let non_tool = non_tool_parts.join("\n").trim().to_string();
    ToolCallParseResult::ok(parser_id, calls, non_tool)
}

// ─── Qwen-style fallback ─────────────────────────────────

/// Parse Qwen-style `<tool_call>` blocks with `<function=...>` / `<parameter=...>` tags.
fn parse_qwen_fallback(sanitized: &str, parser_id: &str) -> ToolCallParseResult {
    let delimited = extract_delimited_blocks(sanitized, "<tool_call>", "</tool_call>");
    if delimited.blocks.is_empty() {
        return ToolCallParseResult::ok(parser_id, vec![], sanitized);
    }

    let mut calls: Vec<ParsedToolCall> = Vec::new();
    for (i, block) in delimited.blocks.iter().enumerate() {
        let fn_re =
            regex::Regex::new(r"(?i)<function=([^>\s]+)>\s*([\s\S]*?)\s*</function>").unwrap();
        let fn_match = fn_re.captures(block);
        let fn_name = fn_match.as_ref().and_then(|c| c.get(1));

        let fn_name = match fn_name {
            Some(m) => m.as_str().trim().to_string(),
            None => {
                return ToolCallParseResult::err(
                    parser_id,
                    sanitized,
                    format!("{} block {} missing <function=...>", parser_id, i + 1),
                );
            }
        };

        let param_re =
            regex::Regex::new(r"(?i)<parameter=([^>\s]+)>\s*([\s\S]*?)\s*</parameter>").unwrap();
        let mut args = serde_json::Map::new();
        for cap in param_re.captures_iter(block) {
            let key = cap.get(1).unwrap().as_str().trim().to_string();
            if key.is_empty() {
                continue;
            }
            let raw_val = cap.get(2).unwrap().as_str().trim();
            let coerced = coerce_value(raw_val);
            args.insert(key, Value::from(coerced));
        }

        calls.push(make_parsed_call(
            &fn_name,
            Value::Object(args),
            block,
            i + 1,
            parser_id,
        ));
    }

    let mut non_tool_parts: Vec<&str> = Vec::new();
    if !delimited.before.trim().is_empty() {
        non_tool_parts.push(delimited.before.trim());
    }
    for b in &delimited.between {
        if !b.trim().is_empty() {
            non_tool_parts.push(b.trim());
        }
    }
    if !delimited.after.trim().is_empty() {
        non_tool_parts.push(delimited.after.trim());
    }

    let non_tool = non_tool_parts.join("\n").trim().to_string();
    ToolCallParseResult::ok(parser_id, calls, non_tool)
}

// ─── GLM 4.5 Parser ──────────────────────────────────────

pub struct Glm45Parser;

impl Glm45Parser {
    pub fn new() -> Self {
        Self
    }
}

impl ToolCallParser for Glm45Parser {
    fn id(&self) -> &str {
        "glm45"
    }

    fn description(&self) -> &str {
        "GLM-4.5 format: special-token-delimited tool calls with JSON"
    }

    fn model_families(&self) -> &[&str] {
        &["GLM-4.5", "GLM-4", "ChatGLM"]
    }

    fn parse(&self, content: &str) -> ToolCallParseResult {
        parse_glm_with_special_tokens(content, "<|tool_call|>", "glm45")
    }
}

// ─── GLM 4.7 Parser ──────────────────────────────────────

pub struct Glm47Parser;

impl Glm47Parser {
    pub fn new() -> Self {
        Self
    }
}

impl ToolCallParser for Glm47Parser {
    fn id(&self) -> &str {
        "glm47"
    }

    fn description(&self) -> &str {
        "GLM-4.7 format: special-token-delimited tool calls with JSON"
    }

    fn model_families(&self) -> &[&str] {
        &["GLM-4.7"]
    }

    fn parse(&self, content: &str) -> ToolCallParseResult {
        parse_glm_with_special_tokens(content, "<|tool_call|>", "glm47")
    }
}

// ─── Step 3 Parser ───────────────────────────────────────

pub struct Step3Parser;

impl Step3Parser {
    pub fn new() -> Self {
        Self
    }
}

impl ToolCallParser for Step3Parser {
    fn id(&self) -> &str {
        "step3"
    }

    fn description(&self) -> &str {
        "Step 3 format: XML/tag-delimited tool calls"
    }

    fn model_families(&self) -> &[&str] {
        &["Step 3", "Step-3"]
    }

    fn parse(&self, content: &str) -> ToolCallParseResult {
        let sanitized = sanitize_reasoning_tags(content);

        // Step models may use <tool_call> or <function_call> tags
        let tc = extract_delimited_blocks(&sanitized, "<tool_call>", "</tool_call>");
        let fc = extract_delimited_blocks(&sanitized, "<function_call>", "</function_call>");
        let has_tc_blocks = !tc.blocks.is_empty();
        let has_fc_blocks = !fc.blocks.is_empty();

        if has_tc_blocks || has_fc_blocks {
            // Try Hermes-style (JSON) first
            let hermes_result = parse_hermes_fallback(&sanitized, "step3");
            if hermes_result.ok && !hermes_result.calls.is_empty() {
                return hermes_result;
            }
            // If Hermes found blocks but JSON parsing failed, try Qwen-style XML
            if !hermes_result.ok {
                return parse_qwen_fallback(&sanitized, "step3");
            }
            // Hermes succeeded with no calls (empty blocks) — fall through to Qwen
        }

        // Fallback: try Qwen-style XML
        parse_qwen_fallback(&sanitized, "step3")
    }
}

// ─── Step 3.5 Parser ─────────────────────────────────────

pub struct Step3p5Parser;

impl Step3p5Parser {
    pub fn new() -> Self {
        Self
    }
}

impl ToolCallParser for Step3p5Parser {
    fn id(&self) -> &str {
        "step3p5"
    }

    fn description(&self) -> &str {
        "Step 3.5 format: XML/tag-delimited tool calls"
    }

    fn model_families(&self) -> &[&str] {
        &["Step 3.5", "Step-3.5"]
    }

    fn parse(&self, content: &str) -> ToolCallParseResult {
        let sanitized = sanitize_reasoning_tags(content);

        let tc = extract_delimited_blocks(&sanitized, "<tool_call>", "</tool_call>");
        let fc = extract_delimited_blocks(&sanitized, "<function_call>", "</function_call>");
        let has_tc_blocks = !tc.blocks.is_empty();
        let has_fc_blocks = !fc.blocks.is_empty();

        if has_tc_blocks || has_fc_blocks {
            // Try Hermes-style (JSON) first
            let hermes_result = parse_hermes_fallback(&sanitized, "step3p5");
            if hermes_result.ok && !hermes_result.calls.is_empty() {
                return hermes_result;
            }
            // If Hermes found blocks but JSON parsing failed, try Qwen-style XML
            if !hermes_result.ok {
                return parse_qwen_fallback(&sanitized, "step3p5");
            }
            // Hermes succeeded with no calls (empty blocks) — fall through to Qwen
        }

        parse_qwen_fallback(&sanitized, "step3p5")
    }
}

// ─── Factories ───────────────────────────────────────────

pub fn create_glm45_parser() -> Box<dyn ToolCallParser> {
    Box::new(Glm45Parser::new())
}

pub fn create_glm47_parser() -> Box<dyn ToolCallParser> {
    Box::new(Glm47Parser::new())
}

pub fn create_step3_parser() -> Box<dyn ToolCallParser> {
    Box::new(Step3Parser::new())
}

pub fn create_step3p5_parser() -> Box<dyn ToolCallParser> {
    Box::new(Step3p5Parser::new())
}

// ─── Tests ───────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── GLM 4.5 ─────────────────────────────────────────

    #[test]
    fn test_glm45_parse_basic() {
        let parser = Glm45Parser::new();
        let input =
            r#"hello <|tool_call|>{"name": "get_weather", "arguments": {"location": "SF"}} world"#;
        let result = parser.parse(input);
        assert!(result.ok, "expected ok, got error: {:?}", result.error);
        assert_eq!(result.calls.len(), 1);
        assert_eq!(result.calls[0].name, "get_weather");
        assert_eq!(result.calls[0].parser_id.as_deref(), Some("glm45"));
        let args = &result.calls[0].arguments;
        assert_eq!(args.get("location").and_then(Value::as_str), Some("SF"));
        assert_eq!(result.content, "hello"); // non-tool content preserved
    }

    #[test]
    fn test_glm45_parse_no_calls() {
        let parser = Glm45Parser::new();
        let input = "just some regular text without any tool calls";
        let result = parser.parse(input);
        assert!(result.ok);
        assert!(result.calls.is_empty());
        assert_eq!(
            result.content,
            "just some regular text without any tool calls"
        );
    }

    #[test]
    fn test_glm45_parse_multiple_calls() {
        let parser = Glm45Parser::new();
        let input = concat!(
            r#"before <|tool_call|>{"name": "func_a", "arguments": {"x": 1}}</s>"#,
            r#"<|tool_call|>{"name": "func_b", "arguments": {"y": 2}} after"#,
        );
        let result = parser.parse(input);
        assert!(result.ok, "expected ok, got error: {:?}", result.error);
        assert_eq!(result.calls.len(), 2);
        assert_eq!(result.calls[0].name, "func_a");
        assert_eq!(result.calls[1].name, "func_b");
        assert_eq!(result.content, "before </s>");
    }

    #[test]
    fn test_glm45_parse_tool_name_variant() {
        let parser = Glm45Parser::new();
        let input = r#"<|tool_call|>{"tool_name": "my_tool", "arguments": {"p": "v"}}"#;
        let result = parser.parse(input);
        assert!(result.ok, "expected ok, got error: {:?}", result.error);
        assert_eq!(result.calls.len(), 1);
        assert_eq!(result.calls[0].name, "my_tool");
    }

    #[test]
    fn test_glm45_parse_parameters_variant() {
        let parser = Glm45Parser::new();
        let input = r#"<|tool_call|>{"name": "test", "parameters": {"a": 1}}"#;
        let result = parser.parse(input);
        assert!(result.ok, "expected ok, got error: {:?}", result.error);
        assert_eq!(result.calls.len(), 1);
        let args = &result.calls[0].arguments;
        assert_eq!(args.get("a").and_then(Value::as_u64), Some(1));
    }

    #[test]
    fn test_glm45_parse_args_as_string() {
        let parser = Glm45Parser::new();
        let input = r#"<|tool_call|>{"name": "test", "arguments": "{\"a\": 1}"}"#;
        let result = parser.parse(input);
        assert!(result.ok, "expected ok, got error: {:?}", result.error);
        assert_eq!(result.calls.len(), 1);
        let args = &result.calls[0].arguments;
        assert_eq!(args.get("a").and_then(Value::as_u64), Some(1));
    }

    #[test]
    fn test_glm45_parse_reasoning_tags_stripped() {
        let parser = Glm45Parser::new();
        let input = concat!(
            r#"<think>reasoning</think>"#,
            r#"<|tool_call|>{"name": "f", "arguments": {}}"#,
        );
        let result = parser.parse(input);
        assert!(result.ok);
        assert_eq!(result.calls.len(), 1);
        assert_eq!(result.calls[0].name, "f");
    }

    // ── GLM 4.7 ─────────────────────────────────────────

    #[test]
    fn test_glm47_parse_basic() {
        let parser = Glm47Parser::new();
        let input = r#"<|tool_call|>{"name": "search", "arguments": {"q": "hamr"}}"#;
        let result = parser.parse(input);
        assert!(result.ok, "expected ok, got error: {:?}", result.error);
        assert_eq!(result.calls.len(), 1);
        assert_eq!(result.calls[0].name, "search");
        assert_eq!(result.calls[0].parser_id.as_deref(), Some("glm47"));
    }

    #[test]
    fn test_glm47_parse_no_calls() {
        let parser = Glm47Parser::new();
        let result = parser.parse("plain text");
        assert!(result.ok);
        assert!(result.calls.is_empty());
    }

    // ── Step 3 ───────────────────────────────────────────

    #[test]
    fn test_step3_parse_hermes_fallback() {
        let parser = Step3Parser::new();
        let input = r#"before <tool_call>{"name": "get_weather", "arguments": {"loc": "NYC"}}</tool_call> after"#;
        let result = parser.parse(input);
        assert!(result.ok, "expected ok, got error: {:?}", result.error);
        assert_eq!(result.calls.len(), 1);
        assert_eq!(result.calls[0].name, "get_weather");
    }

    #[test]
    fn test_step3_parse_function_call_tag() {
        let parser = Step3Parser::new();
        let input = r#"<function_call>{"name": "f", "arguments": {}}</function_call>"#;
        let result = parser.parse(input);
        assert!(result.ok, "expected ok, got error: {:?}", result.error);
        assert_eq!(result.calls.len(), 1);
        assert_eq!(result.calls[0].name, "f");
    }

    #[test]
    fn test_step3_parse_qwen_fallback() {
        let parser = Step3Parser::new();
        let input = concat!(
            r#"<tool_call><function=search><parameter=q>"hamr"</parameter></function></tool_call>"#,
        );
        let result = parser.parse(input);
        assert!(result.ok, "expected ok, got error: {:?}", result.error);
        assert_eq!(result.calls.len(), 1);
        assert_eq!(result.calls[0].name, "search");
    }

    #[test]
    fn test_step3_parse_qwen_multiple_params() {
        let parser = Step3Parser::new();
        let input = concat!(
            "<tool_call>",
            "<function=query_data>",
            "<parameter=endpoint>/api/v1</parameter>",
            r#"<parameter=format>"json"</parameter>"#,
            "<parameter=limit>100</parameter>",
            "</function>",
            "</tool_call>",
        );
        let result = parser.parse(input);
        assert!(result.ok, "expected ok, got error: {:?}", result.error);
        assert_eq!(result.calls.len(), 1);
        assert_eq!(result.calls[0].name, "query_data");
        let args = &result.calls[0].arguments;
        assert_eq!(
            args.get("endpoint").and_then(Value::as_str),
            Some("/api/v1")
        );
        assert_eq!(args.get("format").and_then(Value::as_str), Some("json"));
        assert_eq!(args.get("limit").and_then(Value::as_u64), Some(100));
    }

    #[test]
    fn test_step3_parse_no_calls() {
        let parser = Step3Parser::new();
        let result = parser.parse("just text");
        assert!(result.ok);
        assert!(result.calls.is_empty());
    }

    #[test]
    fn test_step3_parse_missing_function_name() {
        let parser = Step3Parser::new();
        let input = r#"<tool_call><function=><parameter=x>1</parameter></function></tool_call>"#;
        let result = parser.parse(input);
        assert!(!result.ok, "expected parse failure for empty function name");
        assert!(result.error.unwrap().contains("missing <function=...>"));
    }

    // ── Step 3.5 ─────────────────────────────────────────

    #[test]
    fn test_step3p5_parse_hermes() {
        let parser = Step3p5Parser::new();
        let input = r#"<tool_call>{"name": "compute", "arguments": {"expr": "2+2"}}</tool_call>"#;
        let result = parser.parse(input);
        assert!(result.ok, "expected ok, got error: {:?}", result.error);
        assert_eq!(result.calls.len(), 1);
        assert_eq!(result.calls[0].name, "compute");
        assert_eq!(result.calls[0].parser_id.as_deref(), Some("step3p5"));
    }

    #[test]
    fn test_step3p5_parse_qwen() {
        let parser = Step3p5Parser::new();
        let input = r#"<tool_call><function=run><parameter=cmd>"ls -la"</parameter></function></tool_call>"#;
        let result = parser.parse(input);
        assert!(result.ok, "expected ok, got error: {:?}", result.error);
        assert_eq!(result.calls.len(), 1);
        assert_eq!(result.calls[0].name, "run");
    }

    #[test]
    fn test_step3p5_parse_no_calls() {
        let parser = Step3p5Parser::new();
        let result = parser.parse("just text");
        assert!(result.ok);
        assert!(result.calls.is_empty());
    }

    // ── Edge cases ───────────────────────────────────────

    #[test]
    fn test_empty_content() {
        let glm_parser = Glm45Parser::new();
        let result = glm_parser.parse("");
        assert!(result.ok);
        assert!(result.calls.is_empty());

        let step_parser = Step3Parser::new();
        let result = step_parser.parse("");
        assert!(result.ok);
        assert!(result.calls.is_empty());
    }

    #[test]
    fn test_malformed_json_in_token() {
        let parser = Glm45Parser::new();
        let input = r#"<|tool_call|>{bad json here}"#;
        let result = parser.parse(input);
        assert!(result.ok, "malformed json should not cause parse failure");
        assert!(result.calls.is_empty());
    }

    #[test]
    fn test_glm45_with_hermes_fallback() {
        let parser = Glm45Parser::new();
        // No special token — should fall back to Hermes
        let input = r#"<tool_call>{"name": "f", "arguments": {}}</tool_call>"#;
        let result = parser.parse(input);
        assert!(result.ok, "expected hermes fallback to succeed");
        assert_eq!(result.calls.len(), 1);
        assert_eq!(result.calls[0].name, "f");
    }

    #[test]
    fn test_step3_parse_reasoning_tags_stripped() {
        let parser = Step3Parser::new();
        let input = concat!(
            r#"<thinking>step by step</thinking>"#,
            r#"<tool_call>{"name": "f", "arguments": {"x": 1}}</tool_call>"#,
        );
        let result = parser.parse(input);
        assert!(result.ok);
        assert_eq!(result.calls.len(), 1);
        assert_eq!(result.calls[0].name, "f");
    }

    #[test]
    fn test_hermes_fallback_with_both_tags() {
        let parser = Glm45Parser::new();
        let input = concat!(
            r#"before <tool_call>{"name": "a", "arguments": {}}</tool_call>"#,
            r#" between <function_call>{"name": "b", "arguments": {}}</function_call> after"#,
        );
        let result = parser.parse(input);
        assert!(result.ok, "expected ok, got error: {:?}", result.error);
        assert_eq!(result.calls.len(), 2);
        assert_eq!(result.calls[0].name, "a");
        assert_eq!(result.calls[1].name, "b");
    }

    #[test]
    fn test_qwen_fallback_non_tool_content() {
        let parser = Step3Parser::new();
        let input = concat!(
            r#"Let me search for that."#,
            r#"<tool_call><function=search><parameter=q>"hamr"</parameter></function></tool_call>"#,
            r#" Here are the results."#,
        );
        let result = parser.parse(input);
        assert!(result.ok, "expected ok, got error: {:?}", result.error);
        assert_eq!(result.calls.len(), 1);
        assert_eq!(result.calls[0].name, "search");
        // Non-tool content should be preserved
        assert!(result.content.contains("Let me search for that."));
        assert!(result.content.contains("Here are the results."));
    }

    #[test]
    fn test_special_token_non_tool_content() {
        let parser = Glm45Parser::new();
        let input = concat!(
            r#"I'll check the weather."#,
            r#"<|tool_call|>{"name": "get_weather", "arguments": {"location": "SF"}}"#,
            r#" The result shows 72°F."#,
        );
        let result = parser.parse(input);
        assert!(result.ok, "expected ok, got error: {:?}", result.error);
        assert_eq!(result.calls.len(), 1);
        assert!(result.content.contains("I'll check the weather."));
        // Text after the tool-call token and before the next < is consumed
        assert!(!result.content.contains("The result shows 72"));
    }

    // ── Factories ────────────────────────────────────────

    #[test]
    fn test_factories() {
        let glm45 = create_glm45_parser();
        assert_eq!(glm45.id(), "glm45");

        let glm47 = create_glm47_parser();
        assert_eq!(glm47.id(), "glm47");

        let step3 = create_step3_parser();
        assert_eq!(step3.id(), "step3");

        let step3p5 = create_step3p5_parser();
        assert_eq!(step3p5.id(), "step3p5");
    }
}
