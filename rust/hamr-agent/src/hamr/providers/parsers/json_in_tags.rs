//! Shared JSON-in-tags parser factory.
//!
//! Many model families use the same basic format: tool calls wrapped in
//! XML-style tags with JSON objects inside. This module provides a
//! factory for creating parsers for these families.
//!
//! Format:
//!   <tool_call>
//!   {"name": "func_name", "arguments": {"key": "value"}}
//!   </tool_call>
//!
//! Used by: Granite, InternLM, FunctionGemma, OLMo3, Jamba, MiniMax,
//! Kimi K2, Hunyuan, LongCat, GigaChat, and others.
//!
//! Reference: vLLM
//!   vllm/entrypoints/openai/tool_parsers/ — various parsers

use super::types::{Arguments, ParsedToolCall, ToolCallParseResult, ToolCallParser};
use super::utils::{
    extract_delimited_blocks, generate_call_id, safe_json_parse, sanitize_reasoning_tags,
};

// ─── Configuration ────────────────────────────────────────

pub struct JsonInTagsParserConfig {
    pub id: &'static str,
    pub description: &'static str,
    pub model_families: &'static [&'static str],
    /// Open tag (default: "<tool_call>").
    pub open_tag: Option<&'static str>,
    /// Close tag (default: "</tool_call>").
    pub close_tag: Option<&'static str>,
    /// Key for function name in JSON object (default: "name").
    pub name_key: Option<&'static str>,
    /// Key for arguments in JSON object (default: "arguments").
    pub args_key: Option<&'static str>,
}

// ─── Generic JSON-in-tags parser ─────────────────────────

struct GenericJsonInTagsParser {
    config: JsonInTagsParserConfig,
}

impl ToolCallParser for GenericJsonInTagsParser {
    fn id(&self) -> &str {
        self.config.id
    }

    fn description(&self) -> &str {
        self.config.description
    }

    fn model_families(&self) -> &[&str] {
        self.config.model_families
    }

    fn parse(&self, content: &str) -> ToolCallParseResult {
        let open_tag = self.config.open_tag.unwrap_or("<tool_call>");
        let close_tag = self.config.close_tag.unwrap_or("</tool_call>");
        let name_key = self.config.name_key.unwrap_or("name");
        let args_key = self.config.args_key.unwrap_or("arguments");

        let sanitized = sanitize_reasoning_tags(content);
        let delimited = extract_delimited_blocks(&sanitized, open_tag, close_tag);

        if delimited.blocks.is_empty() {
            return ToolCallParseResult::ok(self.config.id, vec![], &sanitized);
        }

        let mut calls: Vec<ParsedToolCall> = Vec::new();

        for (i, raw_block) in delimited.blocks.iter().enumerate() {
            let block = raw_block.trim();
            if block.is_empty() {
                continue;
            }

            let parsed = match safe_json_parse(block) {
                Ok(v) => v,
                Err(e) => {
                    return ToolCallParseResult::err(
                        self.config.id,
                        &sanitized,
                        format!("{} block {}: {}", self.config.id, i + 1, e),
                    );
                }
            };

            if !parsed.is_object() {
                return ToolCallParseResult::err(
                    self.config.id,
                    &sanitized,
                    format!("{} block {}: expected JSON object", self.config.id, i + 1),
                );
            }

            let obj = parsed.as_object().unwrap();

            // Resolve name
            let name = obj
                .get(name_key)
                .or_else(|| obj.get("tool_name"))
                .or_else(|| obj.get("function"))
                .and_then(|v| v.as_str())
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty());

            let name = match name {
                Some(n) => n,
                None => {
                    return ToolCallParseResult::err(
                        self.config.id,
                        &sanitized,
                        format!(
                            "{} block {}: missing \"{}\" field",
                            self.config.id,
                            i + 1,
                            name_key
                        ),
                    );
                }
            };

            // Resolve arguments
            let args = resolve_args(obj, args_key);

            // Resolve id
            let provided_id = obj
                .get("id")
                .or_else(|| obj.get("call_id"))
                .and_then(|v| v.as_str());

            calls.push(ParsedToolCall {
                id: generate_call_id(provided_id, Some(i + 1)),
                name,
                arguments: args,
                raw_source: Some(block.to_string()),
                parser_id: Some(self.config.id.to_string()),
                warnings: None,
            });
        }

        let non_tool_content = build_non_tool_content(&delimited);

        ToolCallParseResult::ok(self.config.id, calls, non_tool_content)
    }
}

/// Resolve arguments from a JSON object, trying various key names.
fn resolve_args(obj: &serde_json::Map<String, serde_json::Value>, args_key: &str) -> Arguments {
    let raw_args = obj
        .get(args_key)
        .or_else(|| obj.get("parameters"))
        .or_else(|| obj.get("input"))
        .or_else(|| obj.get("args"));

    match raw_args {
        Some(serde_json::Value::String(s)) => {
            // Try to parse the string as JSON
            if let Ok(parsed) = safe_json_parse(s) {
                if parsed.is_object() {
                    return parsed;
                }
            }
            // Fallback: emit the string as a single-key object
            serde_json::json!({ "value": s })
        }
        Some(v @ serde_json::Value::Object(_)) => v.clone(),
        Some(v) => v.clone(),
        None => serde_json::Value::Object(serde_json::Map::new()),
    }
}

/// Build non-tool content from delimited result.
fn build_non_tool_content(delimited: &super::utils::DelimitedResult) -> String {
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
    parts.join("\n")
}

// ─── Factory ──────────────────────────────────────────────

/// Create a JSON-in-tags parser with the given config.
pub fn create_json_in_tags_parser(config: JsonInTagsParserConfig) -> Box<dyn ToolCallParser> {
    Box::new(GenericJsonInTagsParser { config })
}

// ─── Pre-configured parsers ───────────────────────────────

pub fn create_granite_parser() -> Box<dyn ToolCallParser> {
    create_json_in_tags_parser(JsonInTagsParserConfig {
        id: "granite",
        description: "Granite format: <tool_call>{\"name\":\"...\",\"arguments\":{...}}</tool_call>",
        model_families: &["Granite", "Granite 3", "IBM Granite"],
        open_tag: None,
        close_tag: None,
        name_key: None,
        args_key: None,
    })
}

pub fn create_granite4_parser() -> Box<dyn ToolCallParser> {
    create_json_in_tags_parser(JsonInTagsParserConfig {
        id: "granite4",
        description: "Granite 4 format: same as granite, Granite-4-specific variant",
        model_families: &["Granite 4"],
        open_tag: None,
        close_tag: None,
        name_key: None,
        args_key: None,
    })
}

pub fn create_granite20b_fc_parser() -> Box<dyn ToolCallParser> {
    create_json_in_tags_parser(JsonInTagsParserConfig {
        id: "granite-20b-fc",
        description: "Granite 20B Function Calling: <tool_call>{\"name\":\"...\",\"arguments\":{...}}</tool_call>",
        model_families: &["Granite 20B FC"],
        open_tag: None,
        close_tag: None,
        name_key: None,
        args_key: None,
    })
}

pub fn create_internlm_parser() -> Box<dyn ToolCallParser> {
    create_json_in_tags_parser(JsonInTagsParserConfig {
        id: "internlm",
        description: "InternLM format: <tool_call>{\"name\":\"...\",\"arguments\":{...}}</tool_call>",
        model_families: &["InternLM", "InternLM2", "InternLM3"],
        open_tag: None,
        close_tag: None,
        name_key: None,
        args_key: None,
    })
}

pub fn create_function_gemma_parser() -> Box<dyn ToolCallParser> {
    create_json_in_tags_parser(JsonInTagsParserConfig {
        id: "functiongemma",
        description: "FunctionGemma format: <tool_call>{\"name\":\"...\",\"arguments\":{...}}</tool_call>",
        model_families: &["FunctionGemma", "Gemma 2 Function Calling"],
        open_tag: None,
        close_tag: None,
        name_key: None,
        args_key: None,
    })
}

/// Factory for OLMo3 parser.
pub fn create_olmo3_parser() -> Box<dyn ToolCallParser> {
    Box::new(Olmo3Parser)
}

// ─── OLMo3 parser ────────────────────────────────────────

/// OLMo3 tool-call parser.
///
/// OLMo3 wraps multiple tool calls in `<function_calls>...</function_calls>`
/// with individual `<function_call>` entries:
///
/// ```text
/// <function_calls>
/// <function_call>
/// {"name": "get_weather", "arguments": {"location": "SF"}}
/// </function_call>
/// <function_call>
/// {"name": "get_time", "arguments": {"timezone": "PST"}}
/// </function_call>
/// </function_calls>
/// ```
///
/// Reference: vLLM
///   --tool-call-parser olmo3
///   vllm/entrypoints/openai/tool_parsers/olmo3_tool_parser.py
struct Olmo3Parser;

impl ToolCallParser for Olmo3Parser {
    fn id(&self) -> &str {
        "olmo3"
    }

    fn description(&self) -> &str {
        "OLMo3 format: <function_calls><function_call>{\"name\":\"...\",\"arguments\":{...}}</function_call></function_calls>"
    }

    fn model_families(&self) -> &[&str] {
        &["OLMo3", "OLMo 3", "OLMoE"]
    }

    fn parse(&self, content: &str) -> ToolCallParseResult {
        let sanitized = sanitize_reasoning_tags(content);

        // First try <function_calls> wrapper
        let fc_delimited =
            extract_delimited_blocks(&sanitized, "<function_calls>", "</function_calls>");

        if !fc_delimited.blocks.is_empty() {
            let mut calls: Vec<ParsedToolCall> = Vec::new();

            for wrapper_block in &fc_delimited.blocks {
                // Extract individual <function_call> blocks
                let inner_delimited =
                    extract_delimited_blocks(wrapper_block, "<function_call>", "</function_call>");

                for (j, raw_block) in inner_delimited.blocks.iter().enumerate() {
                    let block = raw_block.trim();
                    if block.is_empty() {
                        continue;
                    }

                    let parsed = match safe_json_parse(block) {
                        Ok(v) => v,
                        Err(e) => {
                            return ToolCallParseResult::err(
                                "olmo3",
                                &sanitized,
                                format!("OLMo3 function_call block {}: {}", j + 1, e),
                            );
                        }
                    };

                    if !parsed.is_object() {
                        return ToolCallParseResult::err(
                            "olmo3",
                            &sanitized,
                            format!("OLMo3 function_call block {}: expected JSON object", j + 1),
                        );
                    }

                    let obj = parsed.as_object().unwrap();

                    let name = obj
                        .get("name")
                        .or_else(|| obj.get("tool_name"))
                        .and_then(|v| v.as_str())
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty());

                    let name = match name {
                        Some(n) => n,
                        None => {
                            return ToolCallParseResult::err(
                                "olmo3",
                                &sanitized,
                                format!("OLMo3 function_call block {}: missing \"name\"", j + 1),
                            );
                        }
                    };

                    let raw_args = obj.get("arguments").or_else(|| obj.get("parameters"));
                    let args = match raw_args {
                        Some(serde_json::Value::String(s)) => {
                            if let Ok(parsed) = safe_json_parse(s) {
                                if parsed.is_object() {
                                    parsed
                                } else {
                                    serde_json::json!({ "value": s })
                                }
                            } else {
                                serde_json::json!({ "value": s })
                            }
                        }
                        Some(v @ serde_json::Value::Object(_)) => v.clone(),
                        Some(v) => v.clone(),
                        None => serde_json::Value::Object(serde_json::Map::new()),
                    };

                    let provided_id = obj
                        .get("id")
                        .or_else(|| obj.get("call_id"))
                        .and_then(|v| v.as_str());

                    calls.push(ParsedToolCall {
                        id: generate_call_id(provided_id, Some(calls.len() + 1)),
                        name,
                        arguments: args,
                        raw_source: Some(block.to_string()),
                        parser_id: Some("olmo3".to_string()),
                        warnings: None,
                    });
                }
            }

            let non_tool_content = build_non_tool_content(&fc_delimited);
            return ToolCallParseResult::ok("olmo3", calls, non_tool_content);
        }

        // Fallback: try plain <tool_call> blocks (some OLMo3 variants)
        let tc_delimited = extract_delimited_blocks(&sanitized, "<tool_call>", "</tool_call>");
        if !tc_delimited.blocks.is_empty() {
            let fallback = create_json_in_tags_parser(JsonInTagsParserConfig {
                id: "olmo3",
                description: "",
                model_families: &[],
                open_tag: None,
                close_tag: None,
                name_key: None,
                args_key: None,
            });
            return fallback.parse(&sanitized);
        }

        ToolCallParseResult::ok("olmo3", vec![], &sanitized)
    }
}

// ─── Jamba parser ─────────────────────────────────────────

pub fn create_jamba_parser() -> Box<dyn ToolCallParser> {
    create_json_in_tags_parser(JsonInTagsParserConfig {
        id: "jamba",
        description: "Jamba format: <tool_call>{\"name\":\"...\",\"arguments\":{...}}</tool_call>",
        model_families: &["Jamba", "AI21 Jamba"],
        open_tag: None,
        close_tag: None,
        name_key: None,
        args_key: None,
    })
}

// ─── MiniMax parser ───────────────────────────────────────

pub fn create_minimax_parser() -> Box<dyn ToolCallParser> {
    create_json_in_tags_parser(JsonInTagsParserConfig {
        id: "minimax",
        description: "MiniMax format: <tool_call>{\"name\":\"...\",\"arguments\":{...}}</tool_call>",
        model_families: &["MiniMax"],
        open_tag: None,
        close_tag: None,
        name_key: None,
        args_key: None,
    })
}

// ─── Kimi K2 parser ───────────────────────────────────────

pub fn create_kimi_k2_parser() -> Box<dyn ToolCallParser> {
    create_json_in_tags_parser(JsonInTagsParserConfig {
        id: "kimi_k2",
        description: "Kimi K2 format: <tool_call>{\"name\":\"...\",\"arguments\":{...}}</tool_call>",
        model_families: &["Kimi K2", "Moonshot Kimi"],
        open_tag: None,
        close_tag: None,
        name_key: None,
        args_key: None,
    })
}

// ─── Hunyuan A13B parser ──────────────────────────────────

pub fn create_hunyuan_a13b_parser() -> Box<dyn ToolCallParser> {
    create_json_in_tags_parser(JsonInTagsParserConfig {
        id: "hunyuan_a13b",
        description: "Hunyuan A13B format: <tool_call>{\"name\":\"...\",\"arguments\":{...}}</tool_call>",
        model_families: &["Hunyuan", "Tencent Hunyuan"],
        open_tag: None,
        close_tag: None,
        name_key: None,
        args_key: None,
    })
}

// ─── LongCat parser ───────────────────────────────────────

pub fn create_longcat_parser() -> Box<dyn ToolCallParser> {
    create_json_in_tags_parser(JsonInTagsParserConfig {
        id: "longcat",
        description: "LongCat format: <tool_call>{\"name\":\"...\",\"arguments\":{...}}</tool_call>",
        model_families: &["LongCat"],
        open_tag: None,
        close_tag: None,
        name_key: None,
        args_key: None,
    })
}

// ─── GigaChat 3 parser ────────────────────────────────────

/// GigaChat 3 tool-call parser.
///
/// GigaChat 3 uses a custom format with `<function>` tags:
///
/// ```text
/// <function=func_name>{"key": "value"}</function>
/// ```
///
/// Reference: vLLM
///   --tool-call-parser gigachat3
pub fn create_gigachat3_parser() -> Box<dyn ToolCallParser> {
    Box::new(GigaChat3Parser)
}

struct GigaChat3Parser;

impl ToolCallParser for GigaChat3Parser {
    fn id(&self) -> &str {
        "gigachat3"
    }

    fn description(&self) -> &str {
        "GigaChat 3 format: <function=name>{\"key\":\"value\"}</function>"
    }

    fn model_families(&self) -> &[&str] {
        &["GigaChat", "GigaChat 3"]
    }

    fn parse(&self, content: &str) -> ToolCallParseResult {
        let sanitized = sanitize_reasoning_tags(content);
        let fn_regex = regex::Regex::new(r"<function=([^>\s]+)>\s*([\s\S]*?)\s*</function>")
            .expect("invalid regex");

        let mut calls: Vec<ParsedToolCall> = Vec::new();
        let mut non_tool_parts: Vec<String> = Vec::new();
        let mut last_end = 0usize;

        for cap in fn_regex.captures_iter(&sanitized) {
            let m = cap.get(0).expect("match group 0");
            // Text before this match
            if m.start() > last_end {
                non_tool_parts.push(sanitized[last_end..m.start()].to_string());
            }
            last_end = m.end();

            let fn_name = cap
                .get(1)
                .map(|s| s.as_str().trim().to_string())
                .unwrap_or_default();

            if fn_name.is_empty() {
                continue;
            }

            let body = cap
                .get(2)
                .map(|s| s.as_str().trim().to_string())
                .unwrap_or_default();

            let args = if body.is_empty() {
                serde_json::Value::Object(serde_json::Map::new())
            } else {
                match safe_json_parse(&body) {
                    Ok(v) if v.is_object() => v,
                    _ => serde_json::Value::Object(serde_json::Map::new()),
                }
            };

            calls.push(ParsedToolCall {
                id: generate_call_id(None, Some(calls.len() + 1)),
                name: fn_name,
                arguments: args,
                raw_source: Some(m.as_str().to_string()),
                parser_id: Some("gigachat3".to_string()),
                warnings: None,
            });
        }

        // Remaining text after last match
        if last_end < sanitized.len() {
            non_tool_parts.push(sanitized[last_end..].to_string());
        }

        let content = non_tool_parts.concat().trim().to_string();
        ToolCallParseResult::ok("gigachat3", calls, content)
    }
}

// ─── OpenAI passthrough parser ────────────────────────────

/// OpenAI tool-call parser.
///
/// This is a no-op parser for vLLM's `--tool-call-parser openai` mode.
/// In this mode, vLLM returns native OpenAI-format tool_calls in the
/// API response, so no text parsing is needed.
///
/// Hamr handles OpenAI tool_calls natively via the client.
/// This parser is a passthrough for explicit config:
///   tool_call_parser = "openai"
pub fn create_openai_passthrough_parser() -> Box<dyn ToolCallParser> {
    Box::new(OpenaiPassthroughParser)
}

struct OpenaiPassthroughParser;

impl ToolCallParser for OpenaiPassthroughParser {
    fn id(&self) -> &str {
        "openai"
    }

    fn description(&self) -> &str {
        "OpenAI native tool calls (no text parsing needed — tool_calls arrive via API response)"
    }

    fn model_families(&self) -> &[&str] {
        &["OpenAI", "GPT-4", "GPT-4o", "GPT-oss"]
    }

    fn parse(&self, content: &str) -> ToolCallParseResult {
        // OpenAI tool calls arrive via the API's tool_calls field, not in text content.
        // Any tool-call markup in the text is likely an artifact, skip it.
        ToolCallParseResult::ok("openai", vec![], content)
    }
}

// ─── Tests ─────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::super::utils::reset_call_id_counter;
    use super::*;

    #[test]
    fn test_granite_parser_empty() {
        reset_call_id_counter();
        let parser = create_granite_parser();
        let result = parser.parse("Hello, how can I help?");
        assert!(result.ok);
        assert!(result.calls.is_empty());
        assert_eq!(result.content, "Hello, how can I help?");
    }

    #[test]
    fn test_granite_parser_single_call() {
        reset_call_id_counter();
        let parser = create_granite_parser();
        let result = parser.parse(
            r#"Some text <tool_call>
            {"name": "get_weather", "arguments": {"location": "SF"}}
            </tool_call> more text"#,
        );
        assert!(result.ok, "parse error: {:?}", result.error);
        assert_eq!(result.calls.len(), 1);
        assert_eq!(result.calls[0].name, "get_weather");
        assert_eq!(
            result.calls[0].arguments,
            serde_json::json!({"location": "SF"})
        );
        let content = result.content.trim();
        assert!(content.contains("Some text"), "content: {:?}", content);
        assert!(content.contains("more text"), "content: {:?}", content);
    }

    #[test]
    fn test_granite_parser_missing_name() {
        reset_call_id_counter();
        let parser = create_granite_parser();
        let result = parser.parse(r#"<tool_call>{"args": {}}</tool_call>"#);
        assert!(!result.ok);
        assert!(result.error.unwrap().contains("missing"));
    }

    #[test]
    fn test_granite_parser_multiple_calls() {
        reset_call_id_counter();
        let parser = create_granite_parser();
        let result = parser.parse(
            r#"<tool_call>
            {"name": "get_weather", "arguments": {"location": "SF"}}
            </tool_call>
            <tool_call>
            {"name": "get_time", "arguments": {"timezone": "PST"}}
            </tool_call>"#,
        );
        assert!(result.ok);
        assert_eq!(result.calls.len(), 2);
        assert_eq!(result.calls[0].name, "get_weather");
        assert_eq!(result.calls[1].name, "get_time");
    }

    #[test]
    fn test_olmo3_parser_function_calls() {
        reset_call_id_counter();
        let parser = create_olmo3_parser();
        let result = parser.parse(
            r#"some text
            <function_calls>
            <function_call>
            {"name": "get_weather", "arguments": {"location": "SF"}}
            </function_call>
            <function_call>
            {"name": "get_time", "arguments": {"timezone": "PST"}}
            </function_call>
            </function_calls>
            more text"#,
        );
        assert!(result.ok, "parse error: {:?}", result.error);
        assert_eq!(result.calls.len(), 2);
        assert_eq!(result.calls[0].name, "get_weather");
        assert_eq!(result.calls[1].name, "get_time");
        assert!(result.content.contains("some text"));
        assert!(result.content.contains("more text"));
    }

    #[test]
    fn test_olmo3_parser_fallback_tool_call() {
        reset_call_id_counter();
        let parser = create_olmo3_parser();
        let result = parser.parse(
            r#"<tool_call>
            {"name": "get_weather", "arguments": {"location": "SF"}}
            </tool_call>"#,
        );
        assert!(result.ok, "parse error: {:?}", result.error);
        assert_eq!(result.calls.len(), 1);
        assert_eq!(result.calls[0].name, "get_weather");
    }

    #[test]
    fn test_olmo3_parser_no_calls() {
        reset_call_id_counter();
        let parser = create_olmo3_parser();
        let result = parser.parse("Hello, how can I help?");
        assert!(result.ok);
        assert!(result.calls.is_empty());
    }

    #[test]
    fn test_jamba_parser() {
        reset_call_id_counter();
        let parser = create_jamba_parser();
        let result = parser
            .parse(r#"<tool_call>{"name": "search", "arguments": {"query": "rust"}}</tool_call>"#);
        assert!(result.ok, "parse error: {:?}", result.error);
        assert_eq!(result.calls.len(), 1);
        assert_eq!(result.calls[0].name, "search");
    }

    #[test]
    fn test_minimax_parser() {
        reset_call_id_counter();
        let parser = create_minimax_parser();
        let result = parser.parse(
            r#"<tool_call>{"name": "translate", "arguments": {"text": "hello"}}</tool_call>"#,
        );
        assert!(result.ok);
        assert_eq!(result.calls[0].name, "translate");
    }

    #[test]
    fn test_kimi_k2_parser() {
        reset_call_id_counter();
        let parser = create_kimi_k2_parser();
        let result = parser
            .parse(r#"<tool_call>{"name": "analyze", "arguments": {"data": "test"}}</tool_call>"#);
        assert!(result.ok);
        assert_eq!(result.calls[0].name, "analyze");
    }

    #[test]
    fn test_hunyuan_a13b_parser() {
        reset_call_id_counter();
        let parser = create_hunyuan_a13b_parser();
        let result = parser.parse(
            r#"<tool_call>{"name": "generate", "arguments": {"prompt": "hello"}}</tool_call>"#,
        );
        assert!(result.ok);
        assert_eq!(result.calls[0].name, "generate");
    }

    #[test]
    fn test_longcat_parser() {
        reset_call_id_counter();
        let parser = create_longcat_parser();
        let result = parser
            .parse(r#"<tool_call>{"name": "process", "arguments": {"input": "data"}}</tool_call>"#);
        assert!(result.ok);
        assert_eq!(result.calls[0].name, "process");
    }

    #[test]
    fn test_gigachat3_parser() {
        reset_call_id_counter();
        let parser = create_gigachat3_parser();
        let result =
            parser.parse(r#"Hello <function=get_weather>{"location": "SF"}</function> end"#);
        assert!(result.ok, "parse error: {:?}", result.error);
        assert_eq!(result.calls.len(), 1);
        assert_eq!(result.calls[0].name, "get_weather");
        assert_eq!(
            result.calls[0].arguments,
            serde_json::json!({"location": "SF"})
        );
        let content = result.content.trim();
        assert!(content.contains("Hello"), "content: {:?}", content);
        assert!(content.contains("end"), "content: {:?}", content);
    }

    #[test]
    fn test_gigachat3_parser_multiple() {
        reset_call_id_counter();
        let parser = create_gigachat3_parser();
        let result = parser.parse(
            r#"<function=search>{"q": "rust"}</function> <function=open>{"path": "/tmp"}</function>"#,
        );
        assert!(result.ok);
        assert_eq!(result.calls.len(), 2);
        assert_eq!(result.calls[0].name, "search");
        assert_eq!(result.calls[1].name, "open");
    }

    #[test]
    fn test_gigachat3_parser_no_body() {
        reset_call_id_counter();
        let parser = create_gigachat3_parser();
        let result = parser.parse(r#"<function=noop></function>"#);
        assert!(result.ok);
        assert_eq!(result.calls.len(), 1);
        assert_eq!(result.calls[0].name, "noop");
        assert_eq!(result.calls[0].arguments, serde_json::json!({}));
    }

    #[test]
    fn test_openai_passthrough_parser() {
        reset_call_id_counter();
        let parser = create_openai_passthrough_parser();
        let result = parser.parse("Hello world");
        assert!(result.ok);
        assert!(result.calls.is_empty());
        assert_eq!(result.content, "Hello world");
    }

    #[test]
    fn test_granite_parser_with_think_tags() {
        reset_call_id_counter();
        let parser = create_granite_parser();
        let result = parser.parse(
            r#"<think>Let me think...</think>
            <tool_call>
            {"name": "get_weather", "arguments": {"location": "SF"}}
            </tool_call>"#,
        );
        assert!(result.ok, "parse error: {:?}", result.error);
        assert_eq!(result.calls.len(), 1);
        assert_eq!(result.calls[0].name, "get_weather");
        // Reasoning tags should be stripped from content
        assert!(!result.content.contains("<think>"));
    }

    #[test]
    fn test_granite_parser_alternate_name_key() {
        reset_call_id_counter();
        // Test using tool_name fallback
        let parser = create_json_in_tags_parser(JsonInTagsParserConfig {
            id: "test_alt",
            description: "Test alt name key",
            model_families: &[],
            open_tag: Some("<tool>"),
            close_tag: Some("</tool>"),
            name_key: Some("name"),
            args_key: None,
        });
        let result =
            parser.parse(r#"<tool>{"tool_name": "test_fn", "arguments": {"x": 1}}</tool>"#);
        assert!(result.ok);
        assert_eq!(result.calls[0].name, "test_fn");
    }

    #[test]
    fn test_granite_parser_string_args() {
        reset_call_id_counter();
        let parser = create_granite_parser();
        let result = parser.parse(
            r#"<tool_call>
            {"name": "test", "arguments": "{\"x\": 1}"}
            </tool_call>"#,
        );
        assert!(result.ok, "parse error: {:?}", result.error);
        assert_eq!(result.calls.len(), 1);
        assert_eq!(result.calls[0].arguments, serde_json::json!({"x": 1}));
    }

    #[test]
    fn test_granite4_parser() {
        reset_call_id_counter();
        let parser = create_granite4_parser();
        assert_eq!(parser.id(), "granite4");
        let result = parser.parse(r#"<tool_call>{"name": "fn", "arguments": {}}</tool_call>"#);
        assert!(result.ok);
        assert_eq!(result.calls[0].name, "fn");
    }

    #[test]
    fn test_granite20b_fc_parser() {
        reset_call_id_counter();
        let parser = create_granite20b_fc_parser();
        assert_eq!(parser.id(), "granite-20b-fc");
        let result = parser.parse(r#"<tool_call>{"name": "fn", "arguments": {}}</tool_call>"#);
        assert!(result.ok);
        assert_eq!(result.calls[0].name, "fn");
    }

    #[test]
    fn test_internlm_parser() {
        reset_call_id_counter();
        let parser = create_internlm_parser();
        assert_eq!(parser.id(), "internlm");
        let result = parser.parse(r#"<tool_call>{"name": "fn", "arguments": {}}</tool_call>"#);
        assert!(result.ok);
        assert_eq!(result.calls[0].name, "fn");
    }

    #[test]
    fn test_function_gemma_parser() {
        reset_call_id_counter();
        let parser = create_function_gemma_parser();
        assert_eq!(parser.id(), "functiongemma");
        let result = parser.parse(r#"<tool_call>{"name": "fn", "arguments": {}}</tool_call>"#);
        assert!(result.ok);
        assert_eq!(result.calls[0].name, "fn");
    }

    #[test]
    fn test_nested_tag_content_preserved() {
        reset_call_id_counter();
        let parser = create_granite_parser();
        let result = parser.parse(
            r#"before <tool_call>
            {"name": "test", "arguments": {"nested": {"a": 1}}}
            </tool_call> after"#,
        );
        assert!(result.ok);
        assert_eq!(result.calls.len(), 1);
        assert_eq!(
            result.calls[0].arguments,
            serde_json::json!({"nested": {"a": 1}})
        );
        let content = result.content.trim();
        assert!(content.contains("before"));
        assert!(content.contains("after"));
    }

    #[test]
    fn test_args_from_parameters_key() {
        reset_call_id_counter();
        let parser = create_granite_parser();
        let result =
            parser.parse(r#"<tool_call>{"name": "test", "parameters": {"x": 42}}</tool_call>"#);
        assert!(result.ok);
        assert_eq!(result.calls[0].arguments, serde_json::json!({"x": 42}));
    }

    #[test]
    fn test_args_from_input_key() {
        reset_call_id_counter();
        let parser = create_granite_parser();
        let result = parser.parse(r#"<tool_call>{"name": "test", "input": {"x": 42}}</tool_call>"#);
        assert!(result.ok);
        assert_eq!(result.calls[0].arguments, serde_json::json!({"x": 42}));
    }

    #[test]
    fn test_args_from_args_key() {
        reset_call_id_counter();
        let parser = create_granite_parser();
        let result = parser.parse(r#"<tool_call>{"name": "test", "args": {"x": 42}}</tool_call>"#);
        assert!(result.ok);
        assert_eq!(result.calls[0].arguments, serde_json::json!({"x": 42}));
    }

    #[test]
    fn test_call_id_from_object() {
        reset_call_id_counter();
        let parser = create_granite_parser();
        let result = parser.parse(
            r#"<tool_call>{"id": "my_call_1", "name": "test", "arguments": {}}</tool_call>"#,
        );
        assert!(result.ok);
        assert_eq!(result.calls[0].id, "my_call_1");
    }

    #[test]
    fn test_call_id_from_call_id() {
        reset_call_id_counter();
        let parser = create_granite_parser();
        let result = parser.parse(
            r#"<tool_call>{"call_id": "my_call_2", "name": "test", "arguments": {}}</tool_call>"#,
        );
        assert!(result.ok);
        assert_eq!(result.calls[0].id, "my_call_2");
    }

    #[test]
    fn test_function_fallback() {
        reset_call_id_counter();
        let parser = create_granite_parser();
        let result =
            parser.parse(r#"<tool_call>{"function": "my_func", "arguments": {}}</tool_call>"#);
        assert!(result.ok);
        assert_eq!(result.calls[0].name, "my_func");
    }

    #[test]
    fn test_invalid_json_in_block() {
        reset_call_id_counter();
        let parser = create_granite_parser();
        let result = parser.parse(r#"<tool_call>not json at all</tool_call>"#);
        assert!(!result.ok);
        assert!(result.error.unwrap().contains("could not parse JSON"));
    }

    #[test]
    fn test_non_object_json() {
        reset_call_id_counter();
        let parser = create_granite_parser();
        let result = parser.parse(r#"<tool_call>"just a string"</tool_call>"#);
        assert!(!result.ok);
        assert!(result.error.unwrap().contains("expected JSON object"));
    }

    #[test]
    fn test_empty_block_skipped() {
        reset_call_id_counter();
        let parser = create_granite_parser();
        let result = parser.parse(r#"<tool_call></tool_call>"#);
        assert!(result.ok);
        assert!(result.calls.is_empty());
    }

    #[test]
    fn test_mixed_content_no_tags() {
        reset_call_id_counter();
        let parser = create_granite_parser();
        let result = parser.parse("Just regular assistant text with no tool calls.");
        assert!(result.ok);
        assert!(result.calls.is_empty());
        assert_eq!(
            result.content,
            "Just regular assistant text with no tool calls."
        );
    }
}
