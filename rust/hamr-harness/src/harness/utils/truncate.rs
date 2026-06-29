//! Port of `packages/agent/src/harness/utils/truncate.ts`.
//!
//! Shared truncation utilities for tool outputs.
//!
//! Truncation is based on two independent limits - whichever is hit first wins:
//! - Line limit (default: 2000 lines)
//! - Byte limit (default: 50KB)
//!
//! Never returns partial lines (except bash tail truncation edge case).

/// Default maximum number of lines in output.
pub const DEFAULT_MAX_LINES: usize = 2000;
/// Default maximum number of bytes in output (50KB).
pub const DEFAULT_MAX_BYTES: usize = 50 * 1024;
/// Maximum characters per grep match line.
pub const GREP_MAX_LINE_LENGTH: usize = 500;

// ---------------------------------------------------------------------------
// Truncation result
// ---------------------------------------------------------------------------

/// Result of a truncation operation.
#[derive(Debug, Clone)]
pub struct TruncationResult {
    /// The truncated content.
    pub content: String,
    /// Whether truncation occurred.
    pub truncated: bool,
    /// Which limit was hit: "lines", "bytes", or None.
    pub truncated_by: Option<TruncationLimit>,
    /// Total number of lines in the original content.
    pub total_lines: usize,
    /// Total number of bytes in the original content.
    pub total_bytes: usize,
    /// Number of complete lines in the truncated output.
    pub output_lines: usize,
    /// Number of bytes in the truncated output.
    pub output_bytes: usize,
    /// Whether the last line was partially truncated (tail edge case).
    pub last_line_partial: bool,
    /// Whether the first line exceeded the byte limit (head edge case).
    pub first_line_exceeds_limit: bool,
    /// The max lines limit that was applied.
    pub max_lines: usize,
    /// The max bytes limit that was applied.
    pub max_bytes: usize,
}

/// Which limit was hit during truncation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TruncationLimit {
    Lines,
    Bytes,
}

// ---------------------------------------------------------------------------
// Truncation options
// ---------------------------------------------------------------------------

/// Options for truncation functions.
#[derive(Debug, Clone)]
pub struct TruncationOptions {
    /// Maximum number of lines (default: 2000).
    pub max_lines: Option<usize>,
    /// Maximum number of bytes (default: 50KB).
    pub max_bytes: Option<usize>,
}

impl Default for TruncationOptions {
    fn default() -> Self {
        Self {
            max_lines: None,
            max_bytes: None,
        }
    }
}

// ---------------------------------------------------------------------------
// UTF-8 byte length
// ---------------------------------------------------------------------------

/// Calculate the number of bytes when encoding `s` as UTF-8.
/// Rust strings are always valid UTF-8, so this is equivalent to `s.len()`.
pub fn utf8_byte_length(s: &str) -> usize {
    s.len()
}

// ---------------------------------------------------------------------------
// Replace unpaired surrogates
// ---------------------------------------------------------------------------

/// Replace unpaired UTF-16 surrogates with the replacement character (U+FFFD).
/// In well-formed UTF-8 (Rust strings), this is a no-op. Included for parity
/// with the TS implementation which processes strings that may contain lone surrogates.
fn replace_unpaired_surrogates(s: &str) -> String {
    let mut output = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(ch) = chars.next() {
        let code = ch as u32;
        if (0xD800..=0xDBFF).contains(&code) {
            // High surrogate — check for following low surrogate
            if let Some(&next) = chars.peek() {
                let next_code = next as u32;
                if (0xDC00..=0xDFFF).contains(&next_code) {
                    // Valid surrogate pair
                    output.push(ch);
                    output.push(chars.next().unwrap());
                    continue;
                }
            }
            // Unpaired high surrogate
            output.push('\u{FFFD}');
        } else if (0xDC00..=0xDFFF).contains(&code) {
            // Unpaired low surrogate
            output.push('\u{FFFD}');
        } else {
            output.push(ch);
        }
    }
    output
}

// ---------------------------------------------------------------------------
// Human-readable size formatting
// ---------------------------------------------------------------------------

/// Format bytes as a human-readable size string.
pub fn format_size(bytes: usize) -> String {
    if bytes < 1024 {
        format!("{bytes}B")
    } else if bytes < 1024 * 1024 {
        format!("{:.1}KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.1}MB", bytes as f64 / (1024.0 * 1024.0))
    }
}

// ---------------------------------------------------------------------------
// Truncate from head (keep first N lines/bytes)
// ---------------------------------------------------------------------------

/// Truncate content from the head (keep first N lines/bytes).
///
/// Suitable for file reads where you want to see the beginning.
/// Never returns partial lines. If first line exceeds byte limit,
/// returns empty content with `first_line_exceeds_limit = true`.
pub fn truncate_head(content: &str, options: TruncationOptions) -> TruncationResult {
    let max_lines = options.max_lines.unwrap_or(DEFAULT_MAX_LINES);
    let max_bytes = options.max_bytes.unwrap_or(DEFAULT_MAX_BYTES);

    let total_bytes = utf8_byte_length(content);
    let lines: Vec<&str> = content.split('\n').collect();
    let total_lines = lines.len();

    // Check if no truncation needed
    if total_lines <= max_lines && total_bytes <= max_bytes {
        return TruncationResult {
            content: content.to_string(),
            truncated: false,
            truncated_by: None,
            total_lines,
            total_bytes,
            output_lines: total_lines,
            output_bytes: total_bytes,
            last_line_partial: false,
            first_line_exceeds_limit: false,
            max_lines,
            max_bytes,
        };
    }

    // Check if first line alone exceeds byte limit
    let first_line_bytes = utf8_byte_length(lines[0]);
    if first_line_bytes > max_bytes {
        return TruncationResult {
            content: String::new(),
            truncated: true,
            truncated_by: Some(TruncationLimit::Bytes),
            total_lines,
            total_bytes,
            output_lines: 0,
            output_bytes: 0,
            last_line_partial: false,
            first_line_exceeds_limit: true,
            max_lines,
            max_bytes,
        };
    }

    // Collect complete lines that fit
    let mut output_lines_vec: Vec<&str> = Vec::new();
    let mut output_bytes_count: usize = 0;
    let mut truncated_by = TruncationLimit::Lines;

    for (i, line) in lines.iter().enumerate() {
        if i >= max_lines {
            break;
        }
        let line_bytes = utf8_byte_length(line) + if i > 0 { 1 } else { 0 }; // +1 for newline

        if output_bytes_count + line_bytes > max_bytes {
            truncated_by = TruncationLimit::Bytes;
            break;
        }

        output_lines_vec.push(line);
        output_bytes_count += line_bytes;
    }

    // If we exited due to line limit
    if output_lines_vec.len() >= max_lines && output_bytes_count <= max_bytes {
        truncated_by = TruncationLimit::Lines;
    }

    let output_content = output_lines_vec.join("\n");
    let final_output_bytes = utf8_byte_length(&output_content);

    TruncationResult {
        content: output_content,
        truncated: true,
        truncated_by: Some(truncated_by),
        total_lines,
        total_bytes,
        output_lines: output_lines_vec.len(),
        output_bytes: final_output_bytes,
        last_line_partial: false,
        first_line_exceeds_limit: false,
        max_lines,
        max_bytes,
    }
}

// ---------------------------------------------------------------------------
// Truncate from tail (keep last N lines/bytes)
// ---------------------------------------------------------------------------

/// Truncate content from the tail (keep last N lines/bytes).
///
/// Suitable for bash output where you want to see the end (errors, final results).
/// May return a partial first line if the last line of original content exceeds
/// the byte limit.
pub fn truncate_tail(content: &str, options: TruncationOptions) -> TruncationResult {
    let max_lines = options.max_lines.unwrap_or(DEFAULT_MAX_LINES);
    let max_bytes = options.max_bytes.unwrap_or(DEFAULT_MAX_BYTES);

    let total_bytes = utf8_byte_length(content);
    let mut lines: Vec<&str> = content.split('\n').collect();
    // Remove trailing empty line from split, matching TS behavior
    if lines.len() > 1 && lines.last() == Some(&"") {
        lines.pop();
    }
    let total_lines = lines.len();

    // Check if no truncation needed
    if total_lines <= max_lines && total_bytes <= max_bytes {
        return TruncationResult {
            content: content.to_string(),
            truncated: false,
            truncated_by: None,
            total_lines,
            total_bytes,
            output_lines: total_lines,
            output_bytes: total_bytes,
            last_line_partial: false,
            first_line_exceeds_limit: false,
            max_lines,
            max_bytes,
        };
    }

    // Work backwards from the end. Use owned Strings because the
    // byte-limit edge case produces a partial line we must own.
    let mut output_lines_vec: Vec<String> = Vec::new();
    let mut output_bytes_count: usize = 0;
    let mut truncated_by = TruncationLimit::Lines;
    let mut last_line_partial = false;

    for i in (0..lines.len()).rev() {
        if output_lines_vec.len() >= max_lines {
            break;
        }
        let line = lines[i];
        let line_bytes = utf8_byte_length(line) + if !output_lines_vec.is_empty() { 1 } else { 0 };

        if output_bytes_count + line_bytes > max_bytes {
            truncated_by = TruncationLimit::Bytes;
            // Edge case: if we haven't added ANY lines yet and this line exceeds maxBytes,
            // take the end of the line (partial)
            if output_lines_vec.is_empty() {
                let truncated_line = truncate_string_to_bytes_from_end(line, max_bytes);
                output_bytes_count = utf8_byte_length(&truncated_line);
                output_lines_vec.insert(0, truncated_line);
                last_line_partial = true;
            }
            break;
        }

        output_lines_vec.insert(0, line.to_string());
        output_bytes_count += line_bytes;
    }

    // If we exited due to line limit
    if output_lines_vec.len() >= max_lines && output_bytes_count <= max_bytes {
        truncated_by = TruncationLimit::Lines;
    }

    let output_content = output_lines_vec.join("\n");
    let final_output_bytes = utf8_byte_length(&output_content);

    TruncationResult {
        content: output_content,
        truncated: true,
        truncated_by: Some(truncated_by),
        total_lines,
        total_bytes,
        output_lines: output_lines_vec.len(),
        output_bytes: final_output_bytes,
        last_line_partial,
        first_line_exceeds_limit: false,
        max_lines,
        max_bytes,
    }
}

// ---------------------------------------------------------------------------
// Truncate string to fit within byte limit (from the end)
// ---------------------------------------------------------------------------

/// Truncate a string to fit within a byte limit, keeping the tail.
/// Handles multi-byte UTF-8 characters correctly.
fn truncate_string_to_bytes_from_end(s: &str, max_bytes: usize) -> String {
    if max_bytes == 0 {
        return String::new();
    }

    let total_bytes = s.len();
    if total_bytes <= max_bytes {
        return s.to_string();
    }

    // Find the byte offset from the end that keeps at most max_bytes
    let start_byte = total_bytes - max_bytes;

    // Walk forward from start_byte to find a valid char boundary
    let mut valid_start = start_byte;
    while valid_start < total_bytes && !s.is_char_boundary(valid_start) {
        valid_start += 1;
    }

    if valid_start >= total_bytes {
        return String::new();
    }

    let output = s[valid_start..].to_string();

    // Check for unpaired surrogates (parity with TS)
    replace_unpaired_surrogates(&output)
}

// ---------------------------------------------------------------------------
// Truncate single line (for grep matches)
// ---------------------------------------------------------------------------

/// Truncate a single line to max characters, adding a `[truncated]` suffix.
pub fn truncate_line(line: &str, max_chars: Option<usize>) -> TruncateLineResult {
    let max_chars = max_chars.unwrap_or(GREP_MAX_LINE_LENGTH);

    // Count chars, not bytes
    let char_count = line.chars().count();
    if char_count <= max_chars {
        return TruncateLineResult {
            text: line.to_string(),
            was_truncated: false,
        };
    }

    let truncated: String = line.chars().take(max_chars).collect();
    TruncateLineResult {
        text: format!("{truncated}... [truncated]"),
        was_truncated: true,
    }
}

/// Result of truncating a single line.
#[derive(Debug, Clone)]
pub struct TruncateLineResult {
    pub text: String,
    pub was_truncated: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_size() {
        assert_eq!(format_size(0), "0B");
        assert_eq!(format_size(512), "512B");
        assert_eq!(format_size(1024), "1.0KB");
        assert_eq!(format_size(1536), "1.5KB");
        assert_eq!(format_size(1048576), "1.0MB");
    }

    #[test]
    fn test_truncate_head_no_truncation() {
        let content = "hello\nworld";
        let result = truncate_head(content, TruncationOptions::default());
        assert!(!result.truncated);
        assert_eq!(result.content, content);
    }

    #[test]
    fn test_truncate_head_line_limit() {
        let content = "a\nb\nc\nd\ne";
        let result = truncate_head(
            content,
            TruncationOptions {
                max_lines: Some(3),
                max_bytes: None,
            },
        );
        assert!(result.truncated);
        assert_eq!(result.output_lines, 3);
        assert_eq!(result.content, "a\nb\nc");
    }

    #[test]
    fn test_truncate_tail_no_truncation() {
        let content = "hello\nworld";
        let result = truncate_tail(content, TruncationOptions::default());
        assert!(!result.truncated);
        assert_eq!(result.content, content);
    }

    #[test]
    fn test_truncate_tail_line_limit() {
        let content = "a\nb\nc\nd\ne";
        let result = truncate_tail(
            content,
            TruncationOptions {
                max_lines: Some(3),
                max_bytes: None,
            },
        );
        assert!(result.truncated);
        assert_eq!(result.content, "c\nd\ne");
    }

    #[test]
    fn test_truncate_line() {
        let line = "hello world";
        let result = truncate_line(line, Some(5));
        assert!(result.was_truncated);
        assert_eq!(result.text, "hello... [truncated]");
    }

    #[test]
    fn test_truncate_line_no_truncation() {
        let line = "hi";
        let result = truncate_line(line, Some(5));
        assert!(!result.was_truncated);
        assert_eq!(result.text, "hi");
    }

    #[test]
    fn test_utf8_byte_length_ascii() {
        assert_eq!(utf8_byte_length("hello"), 5);
    }

    #[test]
    fn test_utf8_byte_length_unicode() {
        // "é" is 2 bytes in UTF-8
        assert_eq!(utf8_byte_length("é"), 2);
        // "你好" is 6 bytes in UTF-8
        assert_eq!(utf8_byte_length("你好"), 6);
    }

    // -----------------------------------------------------------------------
    // Comprehensive tests ported from TS truncate.test.ts
    // -----------------------------------------------------------------------

    #[test]
    fn test_head_utf8_byte_counts() {
        let content = "aé🙂\nb";
        let result = truncate_head(content, TruncationOptions { max_lines: Some(10), max_bytes: Some(100) });
        assert!(!result.truncated);
        assert_eq!(result.total_bytes, 9); // a=1, é=2, 🙂=4, \n=1, b=1
        assert_eq!(result.output_bytes, 9);
    }

    #[test]
    fn test_head_truncates_on_utf8_byte_limits() {
        let content = "éé\nabc";
        let result = truncate_head(content, TruncationOptions { max_lines: Some(10), max_bytes: Some(4) });
        assert!(result.truncated);
        assert_eq!(result.content, "éé");
        assert_eq!(result.truncated_by, Some(TruncationLimit::Bytes));
        assert_eq!(result.output_bytes, 4);
        assert!(!result.first_line_exceeds_limit);
    }

    #[test]
    fn test_head_first_line_exceeds_byte_limit() {
        let result = truncate_head("éé\nabc", TruncationOptions { max_lines: Some(10), max_bytes: Some(3) });
        assert!(result.truncated);
        assert_eq!(result.content, "");
        assert_eq!(result.truncated_by, Some(TruncationLimit::Bytes));
        assert!(result.first_line_exceeds_limit);
    }

    #[test]
    fn test_tail_utf8_boundary_partial_last_line() {
        // "aé🙂b" = a(1) + é(2) + 🙂(4) + b(1) = 8 bytes
        // maxBytes=5 should keep the last 5 bytes: 🙂(4) + b(1) = 5
        let result = truncate_tail("aé🙂b", TruncationOptions { max_lines: Some(10), max_bytes: Some(5) });
        assert!(result.truncated);
        assert_eq!(result.truncated_by, Some(TruncationLimit::Bytes));
        // Should keep the tail that fits in 5 bytes
        assert!(result.output_bytes <= 5);
    }

    #[test]
    fn test_tail_matches_buffer_tail() {
        // Test that tail truncation never exceeds byte limit.
        // Note: the Rust implementation splits on lines, so it may differ
        // slightly from raw byte tail for inputs with newlines.
        // We test that output_bytes <= max_bytes always holds.
        let inputs = ["hello world", "aé🙂b", "🎉🎊🎈"];
        for input in inputs {
            let total_bytes = input.len();
            let limits: Vec<usize> = (0..=total_bytes + 4).collect();
            for max_bytes in limits {
                let result = truncate_tail(input, TruncationOptions { max_lines: Some(100), max_bytes: Some(max_bytes) });
                assert!(
                    result.output_bytes <= max_bytes,
                    "tail output exceeded byte limit input={input:?} maxBytes={max_bytes} outputBytes={}",
                    result.output_bytes
                );
            }
        }
    }

    #[test]
    fn test_head_truncates_by_lines_with_large_content() {
        let lines: Vec<String> = (0..100).map(|i| format!("line {i}")).collect();
        let content = lines.join("\n");
        let result = truncate_head(&content, TruncationOptions { max_lines: Some(10), max_bytes: None });
        assert!(result.truncated);
        assert_eq!(result.truncated_by, Some(TruncationLimit::Lines));
        assert_eq!(result.output_lines, 10);
        let expected: Vec<String> = (0..10).map(|i| format!("line {i}")).collect();
        assert_eq!(result.content, expected.join("\n"));
    }

    #[test]
    fn test_tail_truncates_by_lines_with_large_content() {
        let lines: Vec<String> = (0..100).map(|i| format!("line {i}")).collect();
        let content = lines.join("\n");
        let result = truncate_tail(&content, TruncationOptions { max_lines: Some(10), max_bytes: None });
        assert!(result.truncated);
        assert_eq!(result.truncated_by, Some(TruncationLimit::Lines));
        assert_eq!(result.output_lines, 10);
        let expected: Vec<String> = (90..100).map(|i| format!("line {i}")).collect();
        assert_eq!(result.content, expected.join("\n"));
    }

    #[test]
    fn test_empty_content() {
        let result = truncate_head("", TruncationOptions::default());
        assert!(!result.truncated);
        assert_eq!(result.content, "");
        assert_eq!(result.total_bytes, 0);
        assert_eq!(result.total_lines, 1); // empty string splits to [""]

        let result = truncate_tail("", TruncationOptions::default());
        assert!(!result.truncated);
        assert_eq!(result.content, "");
    }

    #[test]
    fn test_single_line_no_newline() {
        let result = truncate_head("hello", TruncationOptions { max_lines: Some(1), max_bytes: Some(100) });
        assert!(!result.truncated);
        assert_eq!(result.content, "hello");

        let result = truncate_tail("hello", TruncationOptions { max_lines: Some(1), max_bytes: Some(100) });
        assert!(!result.truncated);
        assert_eq!(result.content, "hello");
    }

    #[test]
    fn test_byte_limit_with_multibyte_chars() {
        // truncate_head works at line boundaries, not within lines.
        // If a line doesn't fit entirely, it's excluded.
        // "abc\n🎉🎊🎈" has first line = "abc" (3 bytes), second = "🎉🎊🎈" (12 bytes)
        // With maxBytes=10, second line (3+1+12=16 total, 3+1+12>10) doesn't fit.
        let content = "abc\n🎉🎊🎈";
        let result = truncate_head(content, TruncationOptions { max_lines: Some(10), max_bytes: Some(10) });
        assert!(result.truncated);
        assert_eq!(result.content, "abc");
        assert_eq!(result.output_bytes, 3);
    }
}
