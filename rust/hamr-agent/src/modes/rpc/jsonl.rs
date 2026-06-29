//! Port of `packages/coding-agent/src/modes/rpc/jsonl.ts`
//!
//! Strict JSONL framing: LF-only record separator, no Unicode separator splitting.

/// Serialize a single strict JSONL record.
///
/// Framing is LF-only. Payload strings may contain other Unicode separators such as
/// U+2028 and U+2029. Clients must split records on `\n` only.
pub fn serialize_json_line(value: &serde_json::Value) -> String {
    let mut s = serde_json::to_string(value).expect("JSON serialization must succeed");
    s.push('\n');
    s
}

/// A JSONL line reader that splits on `\n` only.
///
/// This intentionally does not split on Unicode line separators (U+2028/U+2029)
/// which are valid inside JSON strings.
pub struct JsonlLineReader {
    buffer: String,
}

impl JsonlLineReader {
    pub fn new() -> Self {
        Self {
            buffer: String::new(),
        }
    }

    /// Feed a chunk of data. Returns any complete lines extracted.
    /// Trailing `\r` is stripped from each line.
    pub fn feed(&mut self, chunk: &str) -> Vec<String> {
        self.buffer.push_str(chunk);
        let mut lines = Vec::new();

        while let Some(newline_index) = self.buffer.find('\n') {
            let mut line = self.buffer[..newline_index].to_string();
            self.buffer = self.buffer[newline_index + 1..].to_string();

            // Strip trailing \r (CRLF support)
            if line.ends_with('\r') {
                line.pop();
            }

            lines.push(line);
        }

        lines
    }

    /// Signal end of stream. Returns any remaining data as a final line.
    pub fn end(&mut self) -> Option<String> {
        if self.buffer.is_empty() {
            None
        } else {
            let line = std::mem::take(&mut self.buffer);
            Some(line)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serialize_strict_jsonl() {
        let line = serialize_json_line(&serde_json::json!({"text": "a\u{2028}b\u{2029}c"}));
        assert!(line.contains("a\u{2028}b\u{2029}c"));
        assert!(line.ends_with('\n'));

        let parsed: serde_json::Value = serde_json::from_str(line.trim()).unwrap();
        assert_eq!(parsed, serde_json::json!({"text": "a\u{2028}b\u{2029}c"}));
    }

    #[test]
    fn test_lines_split_on_lf_only() {
        let mut reader = JsonlLineReader::new();
        let lines = reader.feed(&serialize_json_line(
            &serde_json::json!({"text": "a\u{2028}b\u{2029}c"}),
        ));
        reader.end(); // discard remainder

        assert_eq!(lines.len(), 1);
        let parsed: serde_json::Value = serde_json::from_str(&lines[0]).unwrap();
        assert_eq!(parsed, serde_json::json!({"text": "a\u{2028}b\u{2029}c"}));
    }

    #[test]
    fn test_handles_crlf_input() {
        let mut reader = JsonlLineReader::new();
        let lines = reader.feed("{\"a\":1}\r\n{\"b\":2}\r\n");
        reader.end();
        assert_eq!(lines, vec!["{\"a\":1}", "{\"b\":2}"]);
    }

    #[test]
    fn test_final_line_without_trailing_lf() {
        let mut reader = JsonlLineReader::new();
        let lines = reader.feed("{\"a\":1}");
        let final_line = reader.end();

        assert_eq!(lines.len(), 0);
        assert_eq!(final_line, Some("{\"a\":1}".to_string()));
    }
}
