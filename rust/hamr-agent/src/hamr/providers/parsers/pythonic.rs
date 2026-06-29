//! Pythonic tool-call parser.
//!
//! Parses Python-list-format tool calls used by models that generate
//! Python syntax for function calls:
//!
//! ```text
//! [get_weather(city='San Francisco', metric='celsius'),
//!  get_weather(city='Seattle', metric='celsius')]
//! ```
//!
//! Also supports bare calls without the list wrapper:
//!
//! ```text
//! get_weather(city='San Francisco', metric='celsius')
//! ```
//!
//! This parser uses a safe tokenizer — it does NOT eval anything.
//! Supports parallel tool calls (multiple functions in one list).
//!
//! Variants:
//!   - pythonic: general Pythonic list format
//!   - llama4_pythonic: subset with Llama-4-specific handling
//!
//! Reference: vLLM docs/features/tool_calling.md → "Models with Pythonic Tool Calls"
//!   --tool-call-parser pythonic
//!   vllm/entrypoints/openai/tool_parsers/pythonic_tool_parser.py

use super::types::{ParsedToolCall, ToolCallParseResult, ToolCallParser};
use super::utils::{generate_call_id, parse_pythonic_args, sanitize_reasoning_tags};

// ─── Token types ──────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
enum TokenType {
    LParen,
    RParen,
    LBracket,
    RBracket,
    Comma,
    Name,
    /// Quoted string (single or double), including the quotes.
    Text,
    Whitespace,
}

#[derive(Debug, Clone)]
struct Token {
    tok_type: TokenType,
    value: String,
    pos: usize,
}

// ─── Tokenizer ────────────────────────────────────────────

fn tokenize(input: &str) -> Vec<Token> {
    let mut tokens = Vec::new();
    let chars: Vec<char> = input.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        let ch = chars[i];

        // Whitespace
        if ch.is_whitespace() {
            let start = i;
            while i < chars.len() && chars[i].is_whitespace() {
                i += 1;
            }
            tokens.push(Token {
                tok_type: TokenType::Whitespace,
                value: chars[start..i].iter().collect(),
                pos: start,
            });
            continue;
        }

        // Brackets / parens
        if ch == '(' {
            tokens.push(Token {
                tok_type: TokenType::LParen,
                value: "(".to_string(),
                pos: i,
            });
            i += 1;
            continue;
        }
        if ch == ')' {
            tokens.push(Token {
                tok_type: TokenType::RParen,
                value: ")".to_string(),
                pos: i,
            });
            i += 1;
            continue;
        }
        if ch == '[' {
            tokens.push(Token {
                tok_type: TokenType::LBracket,
                value: "[".to_string(),
                pos: i,
            });
            i += 1;
            continue;
        }
        if ch == ']' {
            tokens.push(Token {
                tok_type: TokenType::RBracket,
                value: "]".to_string(),
                pos: i,
            });
            i += 1;
            continue;
        }
        if ch == ',' {
            tokens.push(Token {
                tok_type: TokenType::Comma,
                value: ",".to_string(),
                pos: i,
            });
            i += 1;
            continue;
        }

        // Strings (single or double quoted)
        if ch == '\'' || ch == '"' {
            let quote = ch;
            let start = i;
            i += 1;
            while i < chars.len() && chars[i] != quote {
                if chars[i] == '\\' && i + 1 < chars.len() {
                    i += 1;
                }
                i += 1;
            }
            if i < chars.len() {
                i += 1; // closing quote
            }
            tokens.push(Token {
                tok_type: TokenType::Text,
                value: chars[start..i].iter().collect(),
                pos: start,
            });
            continue;
        }

        // Names / identifiers (including dotted names like "module.func")
        if ch.is_ascii_alphabetic() || ch == '_' {
            let start = i;
            while i < chars.len()
                && (chars[i].is_ascii_alphanumeric() || chars[i] == '_' || chars[i] == '.')
            {
                i += 1;
            }
            tokens.push(Token {
                tok_type: TokenType::Name,
                value: chars[start..i].iter().collect(),
                pos: start,
            });
            continue;
        }

        // Anything else as text
        tokens.push(Token {
            tok_type: TokenType::Text,
            value: ch.to_string(),
            pos: i,
        });
        i += 1;
    }

    tokens
}

// ─── Token helpers ────────────────────────────────────────

fn is_whitespace(tok: &Token) -> bool {
    tok.tok_type == TokenType::Whitespace
}

fn is_name(tok: &Token) -> bool {
    tok.tok_type == TokenType::Name
}

fn is_lparen(tok: &Token) -> bool {
    tok.tok_type == TokenType::LParen
}

// ─── Core parsing ─────────────────────────────────────────

/// Result of parsing a chunk of tokens into calls + remaining text.
struct PythonicParseResult {
    calls: Vec<ParsedToolCall>,
    remainder: String,
}

/// Recursively parse Pythonic function calls from a token stream.
///
/// Handles:
///   - Bare calls: `func_name(arg1='val', arg2=42)`
///   - List-wrapped calls: `[func1(...), func2(...)]`
///   - Nested lists: `[func1(...), [func2(...)]]`
///   - Trailing commas after calls and lists
fn parse_pythonic_calls(tokens: &[Token], parser_id: &str) -> PythonicParseResult {
    let mut calls: Vec<ParsedToolCall> = Vec::new();
    let mut non_call_tokens: Vec<&Token> = Vec::new();

    let mut i = 0;
    while i < tokens.len() {
        // Skip whitespace in non-call context
        while i < tokens.len() && is_whitespace(&tokens[i]) {
            non_call_tokens.push(&tokens[i]);
            i += 1;
        }
        if i >= tokens.len() {
            break;
        }

        // Check for function call pattern: name ( args )
        if is_name(&tokens[i]) && i + 1 < tokens.len() && is_lparen(&tokens[i + 1]) {
            let fn_name = tokens[i].value.clone();
            i += 2; // skip name and '('

            // Collect args until matching ')'
            let mut depth: i32 = 1;
            let arg_start = i;
            while i < tokens.len() && depth > 0 {
                match tokens[i].tok_type {
                    TokenType::LParen => depth += 1,
                    TokenType::RParen => {
                        depth -= 1;
                        if depth == 0 {
                            i += 1;
                            break;
                        }
                    }
                    _ => {}
                }
                i += 1;
            }

            if depth == 0 {
                // Successfully parsed a function call
                let args_str: String = tokens[arg_start..i - 1]
                    .iter()
                    .map(|t| t.value.as_str())
                    .collect();
                let parsed_args = parse_pythonic_args(&args_str);

                // Build arguments as a JSON object
                let mut args_map = serde_json::Map::new();
                for (k, v) in parsed_args {
                    args_map.insert(k, v);
                }

                calls.push(ParsedToolCall {
                    id: generate_call_id(None, Some(calls.len() + 1)),
                    name: fn_name,
                    arguments: serde_json::Value::Object(args_map),
                    raw_source: None,
                    parser_id: Some(parser_id.to_string()),
                    warnings: None,
                });

                // Check for comma after call
                while i < tokens.len() && is_whitespace(&tokens[i]) {
                    i += 1;
                }
                if i < tokens.len() && tokens[i].tok_type == TokenType::Comma {
                    i += 1; // skip comma
                }
                continue;
            }
        }

        // Check for list wrapper: [ call1, call2, ... ]
        if tokens[i].tok_type == TokenType::LBracket {
            let list_start = i;
            i += 1; // skip '['
            let mut depth: i32 = 1;
            let list_token_start = i;

            while i < tokens.len() && depth > 0 {
                match tokens[i].tok_type {
                    TokenType::LBracket => depth += 1,
                    TokenType::RBracket => {
                        depth -= 1;
                        if depth == 0 {
                            i += 1;
                            break;
                        }
                    }
                    _ => {}
                }
                i += 1;
            }

            if depth == 0 {
                // Recursively parse the list contents
                let inner = parse_pythonic_calls(&tokens[list_token_start..i - 1], parser_id);
                for call in inner.calls {
                    calls.push(call);
                }
                // Check for comma after list
                while i < tokens.len() && is_whitespace(&tokens[i]) {
                    i += 1;
                }
                if i < tokens.len() && tokens[i].tok_type == TokenType::Comma {
                    i += 1;
                }
                continue;
            }

            // Unclosed bracket — push remaining as non-call
            for j in list_start..i {
                non_call_tokens.push(&tokens[j]);
            }
            continue;
        }

        // Not a function call — accumulate as non-call text
        non_call_tokens.push(&tokens[i]);
        i += 1;
    }

    let remainder: String = non_call_tokens
        .iter()
        .map(|t| t.value.as_str())
        .collect::<String>()
        .trim()
        .to_string();

    PythonicParseResult { calls, remainder }
}

// ─── Parser instances ─────────────────────────────────────

pub struct PythonicParser;

impl ToolCallParser for PythonicParser {
    fn id(&self) -> &str {
        "pythonic"
    }

    fn description(&self) -> &str {
        "Pythonic list format: [func_name(key=\"value\", ...), ...]"
    }

    fn model_families(&self) -> &[&str] {
        &["Llama 4", "Pythonic-capable models"]
    }

    fn parse(&self, content: &str) -> ToolCallParseResult {
        let sanitized = sanitize_reasoning_tags(content);
        let tokens = tokenize(&sanitized);
        let result = parse_pythonic_calls(&tokens, "pythonic");

        ToolCallParseResult::ok("pythonic", result.calls, result.remainder)
    }
}

pub struct Llama4PythonicParser;

impl ToolCallParser for Llama4PythonicParser {
    fn id(&self) -> &str {
        "llama4_pythonic"
    }

    fn description(&self) -> &str {
        "Llama 4 Pythonic format: same as pythonic but with Llama-4-specific patterns"
    }

    fn model_families(&self) -> &[&str] {
        &["Llama 4"]
    }

    fn parse(&self, content: &str) -> ToolCallParseResult {
        let sanitized = sanitize_reasoning_tags(content);

        // Llama 4 may wrap calls in <|python_tag|> — strip those
        let re = regex::Regex::new(r"(?i)<\|python_tag\|>").unwrap();
        let cleaned = re.replace_all(&sanitized, "").trim().to_string();

        let tokens = tokenize(&cleaned);
        let result = parse_pythonic_calls(&tokens, "llama4_pythonic");

        ToolCallParseResult::ok("llama4_pythonic", result.calls, result.remainder)
    }
}

// ─── Factory functions ────────────────────────────────────

pub fn create_pythonic_parser() -> Box<dyn ToolCallParser> {
    Box::new(PythonicParser)
}

pub fn create_llama4_pythonic_parser() -> Box<dyn ToolCallParser> {
    Box::new(Llama4PythonicParser)
}

// ─── Tests ─────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tokenize_simple() {
        let tokens = tokenize("foo(bar)");
        assert_eq!(tokens.len(), 4);
        assert_eq!(tokens[0].value, "foo");
        assert_eq!(tokens[1].value, "(");
        assert_eq!(tokens[2].value, "bar");
        assert_eq!(tokens[3].value, ")");
    }

    #[test]
    fn test_tokenize_with_strings() {
        let tokens = tokenize(r#"x('hello', "world")"#);
        assert_eq!(tokens.len(), 7);
        // x, (, 'hello', comma, whitespace, "world", )
        assert_eq!(tokens[0].value, "x");
        assert_eq!(tokens[2].value, "'hello'");
        assert_eq!(tokens[5].value, "\"world\"");
    }

    #[test]
    fn test_tokenize_brackets() {
        let tokens = tokenize("[a, b]");
        assert_eq!(tokens[0].value, "[");
        assert_eq!(tokens[5].value, "]");
    }

    #[test]
    fn test_parse_single_call() {
        let parser = PythonicParser;
        let result = parser.parse(r#"get_weather(city='San Francisco', metric='celsius')"#);
        assert!(result.ok);
        assert_eq!(result.calls.len(), 1);
        assert_eq!(result.calls[0].name, "get_weather");
    }

    #[test]
    fn test_parse_bare_call_with_args() {
        let parser = PythonicParser;
        let result = parser.parse(r#"get_weather(city='San Francisco', metric='celsius')"#);
        assert_eq!(result.calls.len(), 1);
        let call = &result.calls[0];
        assert_eq!(call.name, "get_weather");
        let args = call.arguments.as_object().unwrap();
        assert_eq!(
            args.get("city").and_then(|v| v.as_str()),
            Some("San Francisco")
        );
        assert_eq!(args.get("metric").and_then(|v| v.as_str()), Some("celsius"));
    }

    #[test]
    fn test_parse_list_wrapper() {
        let parser = PythonicParser;
        let result = parser.parse(
            r#"[get_weather(city='San Francisco', metric='celsius'),
               get_weather(city='Seattle', metric='celsius')]"#,
        );
        assert!(result.ok);
        assert_eq!(result.calls.len(), 2);
        assert_eq!(result.calls[0].name, "get_weather");
        assert_eq!(result.calls[1].name, "get_weather");
    }

    #[test]
    fn test_parse_list_wrapper_remainder_empty() {
        let parser = PythonicParser;
        let result = parser.parse(r#"[get_weather(city='SF')]"#);
        assert!(result.content.is_empty());
    }

    #[test]
    fn test_parse_list_with_prose_before() {
        let parser = PythonicParser;
        let result = parser.parse(
            r#"Let me check the weather.
[get_weather(city='San Francisco', metric='celsius')]
I'll look at Seattle too."#,
        );
        assert!(result.ok);
        assert_eq!(result.calls.len(), 1);
        assert_eq!(result.calls[0].name, "get_weather");
        assert!(result.content.contains("Let me check the weather"));
        assert!(result.content.contains("I'll look at Seattle too."));
    }

    #[test]
    fn test_parse_no_calls() {
        let parser = PythonicParser;
        let result = parser.parse("Just some regular text.");
        assert!(result.ok);
        assert_eq!(result.calls.len(), 0);
        assert_eq!(result.content, "Just some regular text.");
    }

    #[test]
    fn test_parse_empty() {
        let parser = PythonicParser;
        let result = parser.parse("");
        assert!(result.ok);
        assert_eq!(result.calls.len(), 0);
    }

    #[test]
    fn test_parse_nested_lists() {
        let parser = PythonicParser;
        // Nested list: outer list contains inner list with calls
        let result = parser.parse(r#"[[get_weather(city='SF')]]"#);
        assert!(result.ok);
        assert_eq!(result.calls.len(), 1);
    }

    #[test]
    fn test_parse_numeric_args() {
        let parser = PythonicParser;
        let result = parser.parse(r#"add(a=1, b=2)"#);
        assert!(result.ok);
        let args = result.calls[0].arguments.as_object().unwrap();
        assert_eq!(args.get("a").and_then(|v| v.as_f64()), Some(1.0));
        assert_eq!(args.get("b").and_then(|v| v.as_f64()), Some(2.0));
    }

    #[test]
    fn test_parse_bool_args() {
        let parser = PythonicParser;
        let result = parser.parse(r#"set_enabled(active=True, verbose=False)"#);
        assert!(result.ok);
        let args = result.calls[0].arguments.as_object().unwrap();
        assert_eq!(args.get("active").and_then(|v| v.as_bool()), Some(true));
        assert_eq!(args.get("verbose").and_then(|v| v.as_bool()), Some(false));
    }

    #[test]
    fn test_parse_none_arg() {
        let parser = PythonicParser;
        let result = parser.parse(r#"set_value(x=None)"#);
        assert!(result.ok);
        let args = result.calls[0].arguments.as_object().unwrap();
        assert!(args.get("x").unwrap().is_null());
    }

    #[test]
    fn test_parse_dotted_name() {
        let parser = PythonicParser;
        let result = parser.parse(r#"module.func(a=1)"#);
        assert!(result.ok);
        assert_eq!(result.calls[0].name, "module.func");
    }

    #[test]
    fn test_parse_llama4_pythonic_tag() {
        let parser = Llama4PythonicParser;
        let result = parser.parse(r#"<|python_tag|>get_weather(city='SF')"#);
        assert!(result.ok);
        assert_eq!(result.calls.len(), 1);
        assert_eq!(result.calls[0].name, "get_weather");
    }

    #[test]
    fn test_parse_llama4_pythonic_tag_with_list() {
        let parser = Llama4PythonicParser;
        let result = parser.parse(r#"<|python_tag|>[get_weather(city='SF')]"#);
        assert!(result.ok);
        assert_eq!(result.calls.len(), 1);
        assert_eq!(result.calls[0].name, "get_weather");
    }

    #[test]
    fn test_sanitize_reasoning_tags_preserved() {
        let parser = PythonicParser;
        let result =
            parser.parse(r#"<thinking>I need to check weather</thinking>get_weather(city='SF')"#);
        assert!(result.ok);
        assert_eq!(result.calls.len(), 1);
        // Reasoning tag should be stripped
        assert!(!result.content.contains("<thinking>"));
    }

    #[test]
    fn test_parser_id_set() {
        let parser = PythonicParser;
        let result = parser.parse(r#"f()"#);
        assert_eq!(result.calls[0].parser_id.as_deref(), Some("pythonic"));

        let parser2 = Llama4PythonicParser;
        let result2 = parser2.parse(r#"f()"#);
        assert_eq!(
            result2.calls[0].parser_id.as_deref(),
            Some("llama4_pythonic")
        );
    }

    #[test]
    fn test_call_id_generated() {
        let parser = PythonicParser;
        let result = parser.parse(r#"f()"#);
        assert!(result.calls[0].id.starts_with("call_"));
    }

    #[test]
    fn test_parse_unclosed_list() {
        // Unclosed bracket should remain as content
        let parser = PythonicParser;
        let result = parser.parse(r#"[get_weather(city='SF')"#);
        // The parser may or may not extract the call depending on fallback handling
        // At minimum, it should not crash
        assert!(result.ok);
    }

    #[test]
    fn test_parse_trailing_comma() {
        let parser = PythonicParser;
        let result = parser.parse(r#"[f(a=1), g(b=2),]"#);
        assert!(result.ok);
        assert_eq!(result.calls.len(), 2);
    }

    #[test]
    fn test_generate_call_id_with_index() {
        let id1 = generate_call_id(None, Some(1));
        let id2 = generate_call_id(None, Some(2));
        assert_eq!(id1, "call_1");
        assert_eq!(id2, "call_2");
    }

    #[test]
    fn test_parse_multiple_calls_same_line() {
        let parser = PythonicParser;
        let result = parser.parse(r#"f(a=1) g(b=2)"#);
        assert!(result.ok);
        assert_eq!(result.calls.len(), 2);
    }

    #[test]
    fn test_parse_list_wrapper_with_prose_inside() {
        let parser = PythonicParser;
        let result = parser.parse(r#"[f(a=1), "some text", g(b=2)]"#);
        assert!(result.ok);
        assert_eq!(result.calls.len(), 2);
    }

    #[test]
    fn test_factory_functions() {
        let p1 = create_pythonic_parser();
        assert_eq!(p1.id(), "pythonic");

        let p2 = create_llama4_pythonic_parser();
        assert_eq!(p2.id(), "llama4_pythonic");
    }
}
