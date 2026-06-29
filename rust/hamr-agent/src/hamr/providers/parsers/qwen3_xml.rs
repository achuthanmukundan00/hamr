//! Qwen3 XML tool-call parser.
//!
//! Parses Qwen3-Coder / Qwen3 XML-format tool calls:
//!
//! ```xml
//! <tool_call>
//! <function=get_weather>
//! <parameter=location>San Francisco</parameter>
//! <parameter=unit>celsius</parameter>
//! </function>
//! </tool_call>
//! ```
//!
//! Reference: vLLM docs/features/tool_calling.md → "Qwen3-Coder Models"
//!   Supported via --tool-call-parser qwen3_xml
//!
//! Ported from `packages/coding-agent/src/hamr/providers/parsers/qwen3-xml.ts`.

use regex::Regex;

use super::types::{ParsedToolCall, ToolCallParseResult, ToolCallParser};
use super::utils::{
    coerce_value, extract_delimited_blocks, generate_call_id, sanitize_reasoning_tags,
};

// ─── Constants ───────────────────────────────────────────

const PARSER_ID: &str = "qwen3_xml";
const DESCRIPTION: &str = "Qwen3-Coder / Qwen3 XML format: <tool_call><function=name><parameter=key>value</parameter></function></tool_call>";
const MODEL_FAMILIES: &[&str] = &["Qwen3", "Qwen3-Coder", "Qwen3.5", "Qwen3.6"];

// ─── Parser struct ───────────────────────────────────────

/// Qwen3 XML tool-call parser.
#[derive(Debug, Clone, Default)]
pub struct Qwen3XmlParser;

impl Qwen3XmlParser {
    pub fn new() -> Self {
        Self
    }
}

impl ToolCallParser for Qwen3XmlParser {
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
        let delimited = extract_delimited_blocks(&sanitized, "<tool_call>", "</tool_call>");

        if delimited.blocks.is_empty() {
            return ToolCallParseResult::ok(PARSER_ID, vec![], sanitized);
        }

        let mut calls: Vec<ParsedToolCall> = Vec::new();
        let mut warnings: Vec<String> = Vec::new();

        for (i, block) in delimited.blocks.iter().enumerate() {
            match parse_qwen_function_block(block, i) {
                ParseBlockResult::Ok { call } => {
                    if let Some(c) = call {
                        // Merge call-level warnings into the top-level warnings list.
                        if let Some(ref cw) = c.warnings {
                            warnings.extend(cw.clone());
                        }
                        calls.push(c);
                    }
                }
                ParseBlockResult::Err { error } => {
                    // Unrecoverable parse error in a tool-call block
                    return ToolCallParseResult::err(PARSER_ID, sanitized, error);
                }
            }
        }

        // Extract non-tool text from between blocks
        let mut non_tool_parts: Vec<&str> = Vec::new();
        let before = delimited.before.trim();
        if !before.is_empty() {
            non_tool_parts.push(before);
        }
        for b in &delimited.between {
            let trimmed = b.trim();
            if !trimmed.is_empty() {
                non_tool_parts.push(trimmed);
            }
        }
        let after = delimited.after.trim();
        if !after.is_empty() {
            non_tool_parts.push(after);
        }
        let non_tool_content = non_tool_parts.join("\n").trim().to_string();

        let mut result = ToolCallParseResult::ok(PARSER_ID, calls, non_tool_content);
        if !warnings.is_empty() {
            result.warnings = Some(warnings);
        }
        result
    }
}

// ─── Factory ─────────────────────────────────────────────

/// Create a new Qwen3XmlParser instance.
pub fn create_qwen3_xml_parser() -> Box<dyn ToolCallParser> {
    Box::new(Qwen3XmlParser::new())
}

// ─── Block parser ────────────────────────────────────────

enum ParseBlockResult {
    Ok { call: Option<ParsedToolCall> },
    Err { error: String },
}

fn parse_qwen_function_block(block: &str, index: usize) -> ParseBlockResult {
    let trimmed = block.trim();

    // Find <function=NAME>...</function>
    let fn_re = Regex::new(r"(?i)<function=([^>\s]+)>\s*([\s\S]*?)\s*</function>")
        .expect("invalid function regex");

    let caps = match fn_re.captures(trimmed) {
        Some(c) => c,
        None => {
            return ParseBlockResult::Err {
                error: "Qwen tool_call block missing <function=...> wrapper".to_string(),
            };
        }
    };

    let fn_name = caps.get(1).map(|m| m.as_str().trim()).unwrap_or("");
    if fn_name.is_empty() {
        return ParseBlockResult::Err {
            error: "Qwen tool_call block missing <function=...> wrapper".to_string(),
        };
    }

    let args_body = caps.get(2).map(|m| m.as_str()).unwrap_or("");
    let mut args = serde_json::Map::new();
    let warnings: Vec<String> = Vec::new();

    // Parse <parameter=KEY>VALUE</parameter> tags
    let param_re = Regex::new(r"(?i)<parameter=([^>\s]+)>\s*([\s\S]*?)\s*</parameter>")
        .expect("invalid parameter regex");

    let mut found_any_param = false;

    for cap in param_re.captures_iter(args_body) {
        let key = cap.get(1).map(|m| m.as_str().trim()).unwrap_or("");
        if key.is_empty() {
            continue;
        }
        found_any_param = true;

        let raw_value = cap.get(2).map(|m| m.as_str().trim()).unwrap_or("");
        let coerced = coerce_value(raw_value);
        args.insert(key.to_string(), serde_json::Value::from(coerced));
    }

    // If the args body has content but no valid <parameter=...> tags, it's an error
    if !found_any_param && args_body.trim().len() > 0 {
        return ParseBlockResult::Err {
            error: "Qwen tool_call block contained malformed <parameter=...>".to_string(),
        };
    }

    let call = ParsedToolCall {
        id: generate_call_id(None, Some(index + 1)),
        name: fn_name.to_string(),
        arguments: serde_json::Value::Object(args),
        raw_source: Some(format!("<function={}>{}</function>", fn_name, args_body)),
        parser_id: Some(PARSER_ID.to_string()),
        warnings: if warnings.is_empty() {
            None
        } else {
            Some(warnings.clone())
        },
    };

    ParseBlockResult::Ok { call: Some(call) }
}

// ─── Tests ───────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hamr::providers::parsers::utils::reset_call_id_counter;

    #[test]
    fn test_id_and_description() {
        let parser = Qwen3XmlParser::new();
        assert_eq!(parser.id(), "qwen3_xml");
        assert!(parser.description().contains("Qwen3"));
        assert_eq!(
            parser.model_families(),
            &["Qwen3", "Qwen3-Coder", "Qwen3.5", "Qwen3.6"]
        );
    }

    #[test]
    fn test_no_tool_calls_returns_empty() {
        reset_call_id_counter();
        let parser = Qwen3XmlParser::new();
        let result = parser.parse("Hello, how can I help you?");
        assert!(result.ok);
        assert!(result.calls.is_empty());
        assert_eq!(result.content, "Hello, how can I help you?");
    }

    #[test]
    fn test_single_tool_call() {
        reset_call_id_counter();
        let parser = Qwen3XmlParser::new();
        let input = r#"
<tool_call>
<function=get_weather>
<parameter=location>San Francisco</parameter>
<parameter=unit>celsius</parameter>
</function>
</tool_call>
"#;
        let result = parser.parse(input);
        assert!(result.ok, "parse failed: {:?}", result.error);
        assert_eq!(result.calls.len(), 1);

        let call = &result.calls[0];
        assert_eq!(call.name, "get_weather");
        assert_eq!(call.arguments["location"], "San Francisco");
        assert_eq!(call.arguments["unit"], "celsius");
        assert_eq!(call.parser_id.as_deref(), Some("qwen3_xml"));
    }

    #[test]
    fn test_missing_function_wrapper_returns_error() {
        reset_call_id_counter();
        let parser = Qwen3XmlParser::new();
        let input = r#"
<tool_call>
<parameter=location>Paris</parameter>
</tool_call>
"#;
        let result = parser.parse(input);
        assert!(!result.ok);
        assert!(result.error.unwrap().contains("function"));
    }

    #[test]
    fn test_malformed_parameter_returns_error() {
        reset_call_id_counter();
        let parser = Qwen3XmlParser::new();
        let input = r#"
<tool_call>
<function=search>
some random text that is not a parameter tag
</function>
</tool_call>
"#;
        let result = parser.parse(input);
        assert!(!result.ok);
        assert!(result.error.unwrap().contains("malformed"));
    }

    #[test]
    fn test_value_coercion_boolean() {
        reset_call_id_counter();
        let parser = Qwen3XmlParser::new();
        let input = r#"
<tool_call>
<function=set_enabled>
<parameter=active>true</parameter>
</function>
</tool_call>
"#;
        let result = parser.parse(input);
        assert!(result.ok);
        assert_eq!(result.calls[0].arguments["active"], true);
    }

    #[test]
    fn test_value_coercion_number() {
        reset_call_id_counter();
        let parser = Qwen3XmlParser::new();
        let input = r#"
<tool_call>
<function=set_count>
<parameter=count>42</parameter>
</function>
</tool_call>
"#;
        let result = parser.parse(input);
        assert!(result.ok);
        assert_eq!(result.calls[0].arguments["count"], 42);
    }

    #[test]
    fn test_value_coercion_null() {
        reset_call_id_counter();
        let parser = Qwen3XmlParser::new();
        let input = r#"
<tool_call>
<function=clear>
<parameter=value>null</parameter>
</function>
</tool_call>
"#;
        let result = parser.parse(input);
        assert!(result.ok);
        assert_eq!(result.calls[0].arguments["value"], serde_json::Value::Null);
    }

    #[test]
    fn test_multiple_tool_calls() {
        reset_call_id_counter();
        let parser = Qwen3XmlParser::new();
        let input = r#"
<tool_call>
<function=get_weather>
<parameter=location>Tokyo</parameter>
</function>
</tool_call>
some prose
<tool_call>
<function=get_time>
<parameter=tz>UTC</parameter>
</function>
</tool_call>
"#;
        let result = parser.parse(input);
        assert!(result.ok);
        assert_eq!(result.calls.len(), 2);
        assert_eq!(result.calls[0].name, "get_weather");
        assert_eq!(result.calls[0].arguments["location"], "Tokyo");
        assert_eq!(result.calls[1].name, "get_time");
        assert_eq!(result.calls[1].arguments["tz"], "UTC");
        assert_eq!(result.content.trim(), "some prose");
    }

    #[test]
    fn test_content_between_calls_is_preserved() {
        reset_call_id_counter();
        let parser = Qwen3XmlParser::new();
        let input = "before<tool_call><function=foo><parameter=x>1</parameter></function></tool_call>between<tool_call><function=bar><parameter=y>2</parameter></function></tool_call>after";
        let result = parser.parse(input);
        assert!(result.ok);
        assert_eq!(result.calls.len(), 2);
        assert_eq!(result.content, "before\nbetween\nafter");
    }

    #[test]
    fn test_reasoning_tags_are_stripped() {
        reset_call_id_counter();
        let parser = Qwen3XmlParser::new();
        let input = "<think>Let me think...</think>\n<tool_call>\n<function=search>\n<parameter=query>rust</parameter>\n</function>\n</tool_call>";
        let result = parser.parse(input);
        assert!(result.ok);
        assert_eq!(result.calls.len(), 1);
        assert_eq!(result.calls[0].name, "search");
        // The "Let me think..." part is stripped, content should be empty
        assert_eq!(result.content, "");
    }

    #[test]
    fn test_empty_content() {
        reset_call_id_counter();
        let parser = Qwen3XmlParser::new();
        let result = parser.parse("");
        assert!(result.ok);
        assert!(result.calls.is_empty());
        assert_eq!(result.content, "");
    }

    #[test]
    fn test_whitespace_only() {
        reset_call_id_counter();
        let parser = Qwen3XmlParser::new();
        let result = parser.parse("   \n\n  ");
        assert!(result.ok);
        assert!(result.calls.is_empty());
        assert_eq!(result.content, "");
    }

    #[test]
    fn test_function_name_with_various_chars() {
        reset_call_id_counter();
        let parser = Qwen3XmlParser::new();
        let input = r#"
<tool_call>
<function=my_custom_tool_123>
<parameter=arg1>value1</parameter>
</function>
</tool_call>
"#;
        let result = parser.parse(input);
        assert!(result.ok);
        assert_eq!(result.calls.len(), 1);
        assert_eq!(result.calls[0].name, "my_custom_tool_123");
    }

    #[test]
    fn test_nested_xml_in_parameter_value_is_treated_as_string() {
        reset_call_id_counter();
        let parser = Qwen3XmlParser::new();
        let input = r#"
<tool_call>
<function=process>
<parameter=data>{"nested": {"key": "value"}}</parameter>
</function>
</tool_call>
"#;
        let result = parser.parse(input);
        assert!(result.ok);
        // coerce_value should detect the JSON object
        assert_eq!(result.calls[0].arguments["data"]["nested"]["key"], "value");
    }

    #[test]
    fn test_call_id_generation() {
        reset_call_id_counter();
        let parser = Qwen3XmlParser::new();
        let input = r#"
<tool_call>
<function=foo><parameter=x>1</parameter></function>
</tool_call>
<tool_call>
<function=bar><parameter=y>2</parameter></function>
</tool_call>
"#;
        let result = parser.parse(input);
        assert!(result.ok);
        assert_eq!(result.calls.len(), 2);
        // First call gets call_1, second gets call_2
        assert!(result.calls[0].id.starts_with("call_"));
        assert!(result.calls[1].id.starts_with("call_"));
        assert_ne!(result.calls[0].id, result.calls[1].id);
    }

    #[test]
    fn test_raw_source_is_set() {
        reset_call_id_counter();
        let parser = Qwen3XmlParser::new();
        let input = r#"
<tool_call>
<function=test_fn>
<parameter=key>val</parameter>
</function>
</tool_call>
"#;
        let result = parser.parse(input);
        assert!(result.ok);
        let raw = result.calls[0].raw_source.as_deref().unwrap();
        assert!(raw.starts_with("<function=test_fn>"));
        assert!(raw.ends_with("</function>"));
    }
}
