//! Port of `packages/ai/src/utils/json-parse.ts`.
//!
//! Repair malformed JSON string literals and parse potentially-incomplete JSON
//! emitted during streaming. The TS code leans on the `partial-json` package
//! for incomplete input; here we use a best-effort structural completer that
//! closes open strings/containers, mirroring the "always return a value"
//! contract (falling back to an empty object).

use serde::de::DeserializeOwned;
use serde_json::Value;

const VALID_JSON_ESCAPES: &[char] = &['"', '\\', '/', 'b', 'f', 'n', 'r', 't', 'u'];

fn is_control_character(ch: char) -> bool {
    (ch as u32) <= 0x1f
}

fn escape_control_character(ch: char) -> String {
    match ch {
        '\u{08}' => "\\b".to_string(),
        '\u{0c}' => "\\f".to_string(),
        '\n' => "\\n".to_string(),
        '\r' => "\\r".to_string(),
        '\t' => "\\t".to_string(),
        _ => format!("\\u{:04x}", ch as u32),
    }
}

/// Repair malformed JSON string literals by escaping raw control characters
/// inside strings and doubling backslashes before invalid escape characters.
pub fn repair_json(json: &str) -> String {
    let chars: Vec<char> = json.chars().collect();
    let mut repaired = String::with_capacity(json.len());
    let mut in_string = false;
    let mut index = 0;

    while index < chars.len() {
        let ch = chars[index];

        if !in_string {
            repaired.push(ch);
            if ch == '"' {
                in_string = true;
            }
            index += 1;
            continue;
        }

        if ch == '"' {
            repaired.push(ch);
            in_string = false;
            index += 1;
            continue;
        }

        if ch == '\\' {
            let next = chars.get(index + 1).copied();
            match next {
                None => {
                    repaired.push_str("\\\\");
                    index += 1;
                    continue;
                }
                Some('u') if index + 6 <= chars.len() => {
                    let digits: String = chars[index + 2..index + 6].iter().collect();
                    if digits.chars().all(|c| c.is_ascii_hexdigit()) {
                        repaired.push_str("\\u");
                        repaired.push_str(&digits);
                        index += 6;
                        continue;
                    }
                }
                _ => {}
            }

            if let Some(nc) = next {
                if VALID_JSON_ESCAPES.contains(&nc) {
                    repaired.push('\\');
                    repaired.push(nc);
                    index += 2;
                    continue;
                }
            }

            repaired.push_str("\\\\");
            index += 1;
            continue;
        }

        if is_control_character(ch) {
            repaired.push_str(&escape_control_character(ch));
        } else {
            repaired.push(ch);
        }
        index += 1;
    }

    repaired
}

/// Parse JSON, retrying once with [`repair_json`] applied on failure.
pub fn parse_json_with_repair<T: DeserializeOwned>(json: &str) -> Result<T, serde_json::Error> {
    match serde_json::from_str::<T>(json) {
        Ok(value) => Ok(value),
        Err(error) => {
            let repaired = repair_json(json);
            if repaired != json {
                serde_json::from_str::<T>(&repaired)
            } else {
                Err(error)
            }
        }
    }
}

/// Best-effort completion of a structurally-incomplete JSON document.
///
/// Closes a dangling string, drops a trailing comma, supplies `null` for a
/// dangling key, and appends any unclosed `}`/`]`. Returns `None` if the
/// completed text still fails to parse.
fn parse_partial(input: &str) -> Option<Value> {
    let mut stack: Vec<char> = Vec::new();
    let mut in_string = false;
    let mut escaped = false;

    for ch in input.chars() {
        if in_string {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }
        match ch {
            '"' => in_string = true,
            '{' => stack.push('}'),
            '[' => stack.push(']'),
            '}' | ']' => {
                stack.pop();
            }
            _ => {}
        }
    }

    let mut out = input.to_string();

    if in_string {
        // Drop a dangling escape so the closing quote terminates the string.
        if escaped {
            out.pop();
        }
        out.push('"');
    }

    // Trim trailing whitespace before inspecting the last significant char.
    let trimmed_len = out.trim_end().len();
    out.truncate(trimmed_len);

    match out.chars().last() {
        Some(',') => {
            out.pop();
        }
        Some(':') => out.push_str("null"),
        _ => {}
    }

    while let Some(closer) = stack.pop() {
        out.push(closer);
    }

    serde_json::from_str::<Value>(&out).ok()
}

/// Attempt to parse potentially incomplete JSON during streaming.
///
/// Always returns a [`Value`] — an empty object if parsing ultimately fails.
pub fn parse_streaming_json(partial_json: Option<&str>) -> Value {
    let input = match partial_json {
        Some(s) if !s.trim().is_empty() => s,
        _ => return Value::Object(Default::default()),
    };

    if let Ok(value) = parse_json_with_repair::<Value>(input) {
        return value;
    }
    if let Some(value) = parse_partial(input) {
        return value;
    }
    if let Some(value) = parse_partial(&repair_json(input)) {
        return value;
    }
    Value::Object(Default::default())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn repairs_raw_newline_in_string() {
        let repaired = repair_json("{\"a\":\"line1\nline2\"}");
        assert_eq!(repaired, "{\"a\":\"line1\\nline2\"}");
        let value: Value = serde_json::from_str(&repaired).unwrap();
        assert_eq!(value["a"], "line1\nline2");
    }

    #[test]
    fn repairs_invalid_escape() {
        // `\x` is not a valid JSON escape — backslash should be doubled.
        let repaired = repair_json("{\"p\":\"C:\\x\"}");
        let value: Value = serde_json::from_str(&repaired).unwrap();
        assert_eq!(value["p"], "C:\\x");
    }

    #[test]
    fn preserves_valid_unicode_escape() {
        let repaired = repair_json("{\"a\":\"\\u00e9\"}");
        assert_eq!(repaired, "{\"a\":\"\\u00e9\"}");
    }

    #[test]
    fn parse_with_repair_succeeds_on_valid() {
        let value: Value = parse_json_with_repair("{\"x\":1}").unwrap();
        assert_eq!(value, json!({"x": 1}));
    }

    #[test]
    fn streaming_empty_returns_object() {
        assert_eq!(parse_streaming_json(None), json!({}));
        assert_eq!(parse_streaming_json(Some("   ")), json!({}));
    }

    #[test]
    fn streaming_complete_json() {
        assert_eq!(parse_streaming_json(Some("{\"a\":1}")), json!({"a": 1}));
    }

    #[test]
    fn streaming_incomplete_object() {
        assert_eq!(
            parse_streaming_json(Some("{\"a\":1,\"b\":2")),
            json!({"a": 1, "b": 2})
        );
    }

    #[test]
    fn streaming_incomplete_string() {
        assert_eq!(
            parse_streaming_json(Some("{\"a\":\"hel")),
            json!({"a": "hel"})
        );
    }

    #[test]
    fn streaming_dangling_key() {
        assert_eq!(parse_streaming_json(Some("{\"a\":")), json!({"a": null}));
    }

    #[test]
    fn streaming_trailing_comma() {
        assert_eq!(parse_streaming_json(Some("[1,2,")), json!([1, 2]));
    }
}
