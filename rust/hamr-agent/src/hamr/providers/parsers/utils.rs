//! Shared parsing utilities for Hamr tool-call parsers.
//!
//! These are the building blocks used by individual parser implementations.
//! They handle reasoning-tag sanitization, JSON repair, safe value coercion,
//! call id generation, and safe Pythonic argument parsing.

use super::types::ParsedToolCall;
use std::sync::atomic::{AtomicUsize, Ordering};

static CALL_ID_COUNTER: AtomicUsize = AtomicUsize::new(0);

// --- Reasoning-tag sanitization -------------------------

/// Strip `<think>/<thinking>` reasoning tags from model output.
pub fn sanitize_reasoning_tags(content: &str) -> String {
    if !content.contains("<think") && !content.contains("</think") {
        return content.trim().to_string();
    }
    let mut result = content.to_string();
    // Multi-line: <think...>...content...</think>
    result = regex::Regex::new(r"(?s)<think\b[^>]*>.*?</think>")
        .unwrap()
        .replace_all(&result, "")
        .to_string();
    // Multi-line: <thinking...>...content...</thinking>
    result = regex::Regex::new(r"(?s)<thinking\b[^>]*>.*?</thinking>")
        .unwrap()
        .replace_all(&result, "")
        .to_string();
    // Closing tags only
    result = regex::Regex::new(r"</think(?:ing)?>")
        .unwrap()
        .replace_all(&result, "")
        .to_string();
    result.trim().to_string()
}

// --- JSON parsing with repair ---------------------------

/// Fast-path JSON parse: try native serde_json and return immediately on success.
pub fn fast_json_parse(raw: &str) -> Result<serde_json::Value, String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err("empty input".to_string());
    }
    serde_json::from_str(trimmed)
        .map_err(|_e| format!("invalid JSON: {}", &trimmed[..trimmed.len().min(120)]))
}

/// Repair unclosed braces/brackets by adding missing closing chars.
fn repair_unclosed(json: &str) -> Option<String> {
    let mut stack = Vec::new();
    let mut in_string = false;
    let mut escape = false;

    for ch in json.chars() {
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

        match ch {
            '{' => stack.push('}'),
            '[' => stack.push(']'),
            '}' | ']' => {
                if let Some(&top) = stack.last() {
                    if top == ch {
                        stack.pop();
                    }
                }
            }
            _ => {}
        }
    }

    if stack.is_empty() {
        return None;
    }
    if stack.len() > 10 {
        return None;
    }
    Some(json.to_string() + &stack.into_iter().rev().collect::<String>())
}

/// Extract a JSON object from text that may have prose around it.
fn extract_json_object(text: &str) -> Option<String> {
    let start = text.find('{')?;
    let mut depth = 0i32;
    let mut in_string = false;
    let mut escape = false;

    let chars: Vec<char> = text.chars().collect();
    for i in start..chars.len() {
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
        if ch == '{' {
            depth += 1;
        } else if ch == '}' {
            depth -= 1;
            if depth == 0 {
                return Some(text[start..=i].to_string());
            }
        }
    }
    None
}

/// Parse JSON with limited repair for common local-model mistakes.
pub fn safe_json_parse(raw: &str) -> Result<serde_json::Value, String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err("empty input".to_string());
    }

    // Fast path
    if let Ok(value) = fast_json_parse(raw) {
        return Ok(value);
    }

    // Repair 1: trailing commas — use a non-regex approach for edition 2024 compat
    let no_trailing: String = {
        let mut result = String::with_capacity(trimmed.len());
        let chars: Vec<char> = trimmed.chars().collect();
        let mut i = 0;
        while i < chars.len() {
            if chars[i] == ',' {
                // Peek ahead: if next non-whitespace is ] or }, skip the comma
                let mut j = i + 1;
                while j < chars.len() && chars[j].is_whitespace() {
                    j += 1;
                }
                if j < chars.len() && (chars[j] == ']' || chars[j] == '}') {
                    // Skip comma, emit nothing, continue
                    i += 1;
                    continue;
                }
            }
            result.push(chars[i]);
            i += 1;
        }
        result
    };
    if let Ok(value) = serde_json::from_str(&no_trailing) {
        return Ok(value);
    }

    // Repair 2: unclosed braces/brackets
    if let Some(repaired) = repair_unclosed(&no_trailing) {
        if let Ok(value) = serde_json::from_str(&repaired) {
            return Ok(value);
        }
    }

    // Repair 3: extract from surrounding text
    if let Some(extracted) = extract_json_object(trimmed) {
        if let Ok(value) = serde_json::from_str(&extracted) {
            return Ok(value);
        }
    }

    Err(format!(
        "could not parse JSON: {}",
        &trimmed[..trimmed.len().min(120)]
    ))
}

// --- Call id generation ----------------------------------

/// Generate a deterministic-ish call id.
pub fn generate_call_id(provided: Option<&str>, index: Option<usize>) -> String {
    if let Some(p) = provided {
        let trimmed = p.trim();
        if !trimmed.is_empty() {
            let sanitized: String = trimmed
                .chars()
                .map(|c| {
                    if c.is_alphanumeric() || c == '-' || c == '_' {
                        c
                    } else {
                        '_'
                    }
                })
                .take(64)
                .collect();
            if !sanitized.is_empty() {
                return sanitized;
            }
        }
    }
    let idx = index.unwrap_or_else(|| CALL_ID_COUNTER.fetch_add(1, Ordering::Relaxed) + 1);
    format!("call_{}", idx)
}

/// Reset the call id counter (useful for tests).
pub fn reset_call_id_counter() {
    CALL_ID_COUNTER.store(0, Ordering::Relaxed);
}

// --- Safe value coercion ---------------------------------

/// Coerced value kind from string coercion.
#[derive(Debug, Clone, PartialEq)]
pub enum ValueKind {
    String(String),
    Number(f64),
    Bool(bool),
    Null,
    Object(serde_json::Value),
}

impl From<ValueKind> for serde_json::Value {
    fn from(val: ValueKind) -> Self {
        match val {
            ValueKind::String(s) => serde_json::Value::String(s),
            ValueKind::Number(n) => {
                // If the number is a whole integer, store as i64 for exact comparison.
                if n.fract() == 0.0
                    && n.is_finite()
                    && n >= (i64::MIN as f64)
                    && n <= (i64::MAX as f64)
                {
                    serde_json::Value::Number(serde_json::Number::from(n as i64))
                } else {
                    serde_json::Number::from_f64(n)
                        .map_or(serde_json::Value::Null, |n| serde_json::Value::Number(n))
                }
            }
            ValueKind::Bool(b) => serde_json::Value::Bool(b),
            ValueKind::Null => serde_json::Value::Null,
            ValueKind::Object(v) => v,
        }
    }
}

/// Coerce a string value to the appropriate type for common literal forms.
pub fn coerce_value(raw: &str) -> ValueKind {
    let trimmed = raw.trim();

    if trimmed == "true" {
        return ValueKind::Bool(true);
    }
    if trimmed == "false" {
        return ValueKind::Bool(false);
    }
    if trimmed == "null" || trimmed == "None" {
        return ValueKind::Null;
    }

    if regex::Regex::new(r"^-?(?:0|[1-9]\d*)(?:\.\d+)?(?:[eE][+-]?\d+)?$")
        .unwrap()
        .is_match(trimmed)
    {
        if let Ok(n) = trimmed.parse::<f64>() {
            return ValueKind::Number(n);
        }
    }

    if (trimmed.len() >= 2 && trimmed.starts_with('"') && trimmed.ends_with('"'))
        || (trimmed.len() >= 2 && trimmed.starts_with('\'') && trimmed.ends_with('\''))
    {
        return ValueKind::String(trimmed[1..trimmed.len() - 1].to_string());
    }

    if (trimmed.starts_with('{') && trimmed.ends_with('}'))
        || (trimmed.starts_with('[') && trimmed.ends_with(']'))
    {
        if let Ok(value) = safe_json_parse(trimmed) {
            return ValueKind::Object(value);
        }
    }

    ValueKind::String(trimmed.to_string())
}

// --- Helper for building parsed calls --------------------

pub struct CallOpts {
    pub id: Option<String>,
    pub index: Option<usize>,
    pub raw_source: Option<String>,
    pub parser_id: Option<String>,
    pub warnings: Vec<String>,
}

impl Default for CallOpts {
    fn default() -> Self {
        Self {
            id: None,
            index: None,
            raw_source: None,
            parser_id: None,
            warnings: Vec::new(),
        }
    }
}

/// Build a ParsedToolCall from name, args map, and options.
pub fn make_call(name: &str, args: serde_json::Value, opts: &CallOpts) -> ParsedToolCall {
    let args_obj = match args {
        serde_json::Value::Object(map) => serde_json::Value::Object(map),
        v => v,
    };
    let warnings = if opts.warnings.is_empty() {
        None
    } else {
        Some(opts.warnings.clone())
    };
    ParsedToolCall {
        id: generate_call_id(opts.id.as_deref(), opts.index),
        name: name.to_string(),
        arguments: args_obj,
        raw_source: opts.raw_source.clone(),
        parser_id: opts.parser_id.clone(),
        warnings,
    }
}

// --- Safe Pythonic argument parsing ----------------------

#[derive(Debug, Clone, PartialEq)]
enum PyToken {
    Identifier(String),
    StringVal(String),
    Number(String),
    Operator(String),
    Name(String), // True, False, None
    Other(String),
}

/// Parse a Pythonic function-call argument string into key-value pairs.
pub fn parse_pythonic_args(args_str: &str) -> serde_json::Map<String, serde_json::Value> {
    let mut result = serde_json::Map::new();
    if args_str.trim().is_empty() {
        return result;
    }

    let tokens = tokenize_pythonic_args(args_str);
    let mut i = 0usize;

    while i < tokens.len() {
        if i >= tokens.len() || !matches!(&tokens[i], PyToken::Identifier(_)) {
            i += 1;
            continue;
        }
        let key = match &tokens[i] {
            PyToken::Identifier(s) => s.clone(),
            _ => return result,
        };
        i += 1;

        if i < tokens.len() {
            match &tokens[i] {
                PyToken::Operator(op) if op == "=" => {
                    i += 1;
                }
                _ => continue,
            }
        } else {
            continue;
        }

        if i < tokens.len() {
            let value = coerce_pythonic_value(&tokens[i]);
            result.insert(key, serde_json::Value::from(value));
            i += 1;
            if i < tokens.len() {
                if let PyToken::Operator(op) = &tokens[i] {
                    if op == "," {
                        i += 1;
                    }
                }
            }
        }
    }
    result
}

fn tokenize_pythonic_args(input: &str) -> Vec<PyToken> {
    let mut tokens = Vec::new();
    let chars: Vec<char> = input.chars().collect();
    let mut i = 0usize;

    while i < chars.len() {
        let ch = chars[i];

        // Whitespace
        if ch.is_whitespace() {
            i += 1;
            continue;
        }

        // Operator/comma/equal
        if ch == ',' || ch == '=' {
            tokens.push(PyToken::Operator(ch.to_string()));
            i += 1;
            continue;
        }

        // Single-quoted string
        if ch == '\'' {
            i += 1;
            let mut val = String::new();
            while i < chars.len() && chars[i] != '\'' {
                if chars[i] == '\\' && i + 1 < chars.len() {
                    i += 1;
                    val.push(chars[i]);
                } else {
                    val.push(chars[i]);
                }
                i += 1;
            }
            if i < chars.len() {
                i += 1;
            } // closing quote
            tokens.push(PyToken::StringVal(val));
            continue;
        }

        // Double-quoted string
        if ch == '"' {
            i += 1;
            let mut val = String::new();
            while i < chars.len() && chars[i] != '"' {
                if chars[i] == '\\' && i + 1 < chars.len() {
                    i += 1;
                    val.push(chars[i]);
                } else {
                    val.push(chars[i]);
                }
                i += 1;
            }
            if i < chars.len() {
                i += 1;
            } // closing quote
            tokens.push(PyToken::StringVal(val));
            continue;
        }

        // Number
        if ch == '-' || ch.is_ascii_digit() {
            let start = i;
            if ch == '-' {
                i += 1;
            }
            while i < chars.len()
                && (chars[i].is_ascii_digit()
                    || chars[i] == '.'
                    || chars[i] == 'e'
                    || chars[i] == 'E'
                    || chars[i] == '+'
                    || chars[i] == '-')
            {
                i += 1;
            }
            let num_str: String = chars[start..i].iter().collect();
            if regex::Regex::new(r"^-?(?:0|[1-9]\d*)(?:\.\d+)?(?:[eE][+-]?\d+)?$")
                .unwrap()
                .is_match(&num_str)
            {
                tokens.push(PyToken::Number(num_str));
                continue;
            }
            i = start;
        }

        // Identifier / name
        if ch.is_alphabetic() || ch == '_' {
            let start = i;
            while i < chars.len()
                && (chars[i].is_alphanumeric() || chars[i] == '_' || chars[i] == '.')
            {
                i += 1;
            }
            let val: String = chars[start..i].iter().collect();
            match val.as_str() {
                "True" => tokens.push(PyToken::Name("True".to_string())),
                "False" => tokens.push(PyToken::Name("False".to_string())),
                "None" => tokens.push(PyToken::Name("None".to_string())),
                _ => tokens.push(PyToken::Identifier(val)),
            }
            continue;
        }

        // Anything else
        tokens.push(PyToken::Other(ch.to_string()));
        i += 1;
    }
    tokens
}

fn coerce_pythonic_value(token: &PyToken) -> ValueKind {
    match token {
        PyToken::StringVal(s) => ValueKind::String(s.clone()),
        PyToken::Number(n) => {
            if let Ok(v) = n.parse::<f64>() {
                ValueKind::Number(v)
            } else {
                ValueKind::String(n.clone())
            }
        }
        PyToken::Name(n) => match n.as_str() {
            "True" => ValueKind::Bool(true),
            "False" => ValueKind::Bool(false),
            "None" => ValueKind::Null,
            _ => ValueKind::String(n.clone()),
        },
        PyToken::Identifier(s) => ValueKind::String(s.clone()),
        _ => ValueKind::String(match token {
            PyToken::Operator(o) => o.clone(),
            PyToken::Other(o) => o.clone(),
            _ => "unknown".to_string(),
        }),
    }
}

// --- Tool-call delimiter extraction ----------------------

pub struct DelimitedResult {
    pub before: String,
    pub blocks: Vec<String>,
    pub between: Vec<String>,
    pub after: String,
}

/// Split content into segments separated by tool-call blocks.
pub fn extract_delimited_blocks(content: &str, open_tag: &str, close_tag: &str) -> DelimitedResult {
    let mut blocks = Vec::new();
    let mut between = Vec::new();
    let mut after = String::new();

    let first_open = content.find(open_tag);
    if first_open.is_none() {
        return DelimitedResult {
            before: content.to_string(),
            blocks: Vec::new(),
            between: Vec::new(),
            after: String::new(),
        };
    }
    let before = content[..first_open.unwrap()].to_string();
    let mut remaining = &content[first_open.unwrap()..];

    loop {
        let open_idx = remaining.find(open_tag);
        if open_idx.is_none() {
            after = remaining.to_string();
            break;
        }

        let open_idx = open_idx.unwrap();
        if open_idx > 0 {
            between.push(remaining[..open_idx].to_string());
            remaining = &remaining[open_idx..];
            continue;
        }

        // open_idx == 0 -- we're at an open tag
        let close_idx = remaining[open_tag.len()..].find(close_tag);
        if close_idx.is_none() {
            between.push(remaining.to_string());
            break;
        }

        let close_idx = close_idx.unwrap() + open_tag.len();
        let block = remaining[open_tag.len()..close_idx].to_string();
        blocks.push(block);
        remaining = &remaining[close_idx + close_tag.len()..];
    }

    DelimitedResult {
        before,
        blocks,
        between,
        after,
    }
}

/// Extract all content that is NOT inside tool-call delimiters.
pub fn extract_non_tool_content(content: &str, open_tag: &str, close_tag: &str) -> String {
    let delimited = extract_delimited_blocks(content, open_tag, close_tag);

    let mut parts = Vec::new();
    if !delimited.before.trim().is_empty() {
        parts.push(delimited.before);
    }
    for b in &delimited.between {
        if !b.trim().is_empty() {
            parts.push(b.clone());
        }
    }
    if !delimited.after.trim().is_empty() {
        parts.push(delimited.after);
    }

    parts.join("\n").trim().to_string()
}

/// Try to find a pattern anywhere in the text.
/// Returns (match_text, before, after) or None.
pub fn find_pattern<'a>(
    content: &'a str,
    regex: &regex::Regex,
) -> Option<(&'a str, &'a str, &'a str)> {
    let m = regex.find(content)?;
    Some((
        &content[m.start()..m.end()],
        &content[..m.start()],
        &content[m.end()..],
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_reasoning_tags_empty() {
        assert_eq!(sanitize_reasoning_tags("hello world"), "hello world");
    }

    #[test]
    fn test_sanitize_reasoning_tags_think() {
        let input = "<think>let me think about this</think>hello world";
        let result = sanitize_reasoning_tags(input);
        assert!(!result.contains("<think"));
        assert!(result.contains("hello world"));
    }

    #[test]
    fn test_fast_json_parse_valid() {
        let result = fast_json_parse(r#"{"a": 1}"#);
        assert!(result.is_ok());
    }

    #[test]
    fn test_fast_json_parse_invalid() {
        let result = fast_json_parse(r#"{"a: 1}"#);
        assert!(result.is_err());
    }

    #[test]
    fn test_safe_json_parse_trailing_comma() {
        let result = safe_json_parse(r#"{"a": 1,}"#);
        assert!(result.is_ok());
    }

    #[test]
    fn test_coerce_value_boolean() {
        assert_eq!(coerce_value("true"), ValueKind::Bool(true));
        assert_eq!(coerce_value("false"), ValueKind::Bool(false));
    }

    #[test]
    fn test_coerce_value_null() {
        assert_eq!(coerce_value("null"), ValueKind::Null);
        assert_eq!(coerce_value("None"), ValueKind::Null);
    }

    #[test]
    fn test_coerce_value_number() {
        match coerce_value("42") {
            ValueKind::Number(n) => assert_eq!(n, 42.0),
            v => panic!("expected Number, got {:?}", v),
        }
    }

    #[test]
    fn test_coerce_value_quoted_string() {
        assert_eq!(
            coerce_value(r#""hello""#),
            ValueKind::String("hello".to_string())
        );
        assert_eq!(
            coerce_value("'hello'"),
            ValueKind::String("hello".to_string())
        );
    }

    #[test]
    fn test_parse_pythonic_args() {
        let result = parse_pythonic_args(r#"location="San Francisco", unit='celsius', count=42"#);
        assert_eq!(result.get("location").unwrap(), "San Francisco");
        assert_eq!(result.get("unit").unwrap(), "celsius");
        assert_eq!(result.get("count").unwrap(), 42);
    }

    #[test]
    fn test_extract_delimited_blocks() {
        let content = "before<call>args1</call>middle<call>args2</call>after";
        let result = extract_delimited_blocks(content, "<call>", "</call>");
        assert_eq!(result.before, "before");
        assert_eq!(result.blocks.len(), 2);
        assert_eq!(result.blocks[0], "args1");
        assert_eq!(result.blocks[1], "args2");
        assert_eq!(result.after, "after");
    }

    #[test]
    fn test_extract_delimited_blocks_no_blocks() {
        let content = "just text";
        let result = extract_delimited_blocks(content, "<call>", "</call>");
        assert_eq!(result.before, "just text");
        assert!(result.blocks.is_empty());
    }

    #[test]
    fn test_extract_non_tool_content() {
        let content = "prose<call>args</call>more prose<call>args2</call>end";
        let result = extract_non_tool_content(content, "<call>", "</call>");
        assert!(result.contains("prose"));
        assert!(result.contains("more prose"));
        assert!(result.contains("end"));
        assert!(!result.contains("<call>"));
    }

    #[test]
    fn test_generate_call_id_provided() {
        let id = generate_call_id(Some("tool_1"), None);
        assert_eq!(id, "tool_1");
    }

    #[test]
    fn test_generate_call_id_generated() {
        reset_call_id_counter();
        let id = generate_call_id(None, None);
        assert!(id.starts_with("call_"));
    }

    #[test]
    fn test_generate_call_id_special_chars() {
        let id = generate_call_id(Some("tool!@#"), None);
        assert_eq!(id, "tool___");
    }
}
