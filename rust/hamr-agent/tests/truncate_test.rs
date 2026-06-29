//! Comprehensive tests for hamr-agent truncate utilities.
//! Mirrors the test coverage from packages/coding-agent/src/core/tools/truncate.ts

use hamr_agent::core::tools::truncate::*;

// ---------------------------------------------------------------------------
// format_size
// ---------------------------------------------------------------------------

#[test]
fn format_size_bytes() {
    assert_eq!(format_size(0), "0B");
    assert_eq!(format_size(123), "123B");
    assert_eq!(format_size(1023), "1023B");
}

#[test]
fn format_size_kb() {
    assert_eq!(format_size(1024), "1.0KB");
    assert_eq!(format_size(1536), "1.5KB");
    assert_eq!(format_size(1024 * 1024 - 1), "1024.0KB");
}

#[test]
fn format_size_mb() {
    assert_eq!(format_size(1024 * 1024), "1.0MB");
    assert_eq!(format_size(2 * 1024 * 1024 + 300 * 1024), "2.3MB");
}

// ---------------------------------------------------------------------------
// split_lines_for_counting (indirectly tested via truncate_head/truncate_tail)
// ---------------------------------------------------------------------------

#[test]
fn split_lines_behavior_via_truncate() {
    // Trailing newline doesn't create extra empty line
    let content = "a\nb\nc\n";
    let result = truncate_head(
        content,
        TruncationOptions {
            max_lines: Some(100),
            max_bytes: Some(10000),
        },
    );
    assert_eq!(result.total_lines, 3);
    assert!(!result.truncated);

    // Empty content
    let result = truncate_head(
        "",
        TruncationOptions {
            max_lines: Some(100),
            max_bytes: Some(10000),
        },
    );
    assert_eq!(result.total_lines, 0);
    assert!(!result.truncated);

    // Single line
    let result = truncate_head(
        "hello",
        TruncationOptions {
            max_lines: Some(100),
            max_bytes: Some(10000),
        },
    );
    assert_eq!(result.total_lines, 1);

    // Single line with trailing newline
    let result = truncate_head(
        "hello\n",
        TruncationOptions {
            max_lines: Some(100),
            max_bytes: Some(10000),
        },
    );
    assert_eq!(result.total_lines, 1);
}

// ---------------------------------------------------------------------------
// truncate_head
// ---------------------------------------------------------------------------

#[test]
fn truncate_head_no_truncation() {
    let content = "line1\nline2\nline3";
    let result = truncate_head(
        content,
        TruncationOptions {
            max_lines: Some(10),
            max_bytes: Some(1024),
        },
    );
    assert!(!result.truncated);
    assert_eq!(result.truncated_by, None);
    assert_eq!(result.content, content);
    assert_eq!(result.total_lines, 3);
    assert_eq!(result.total_bytes, content.len());
    assert_eq!(result.output_lines, 3);
    assert_eq!(result.output_bytes, content.len());
    assert!(!result.first_line_exceeds_limit);
}

#[test]
fn truncate_head_line_limit() {
    let content = "line1\nline2\nline3\nline4\nline5";
    let result = truncate_head(
        content,
        TruncationOptions {
            max_lines: Some(3),
            max_bytes: Some(1024),
        },
    );
    assert!(result.truncated);
    assert_eq!(result.truncated_by, Some(TruncationLimit::Lines));
    assert_eq!(result.content, "line1\nline2\nline3");
    assert_eq!(result.total_lines, 5);
    assert_eq!(result.output_lines, 3);
}

#[test]
fn truncate_head_byte_limit() {
    // Each line "12345" is 5 bytes, "\n" is 1 byte → 6 bytes per line
    // First line has no preceding \n, so "12345" = 5, then \n + "54321" = 6 → total 11
    // Next would be \n + "12345" = 6 → 17 > 12 → truncate at 2 lines
    let content = "12345\n54321\n12345\n54321\n";
    let result = truncate_head(
        content,
        TruncationOptions {
            max_lines: Some(100),
            max_bytes: Some(12),
        },
    );
    assert!(result.truncated);
    assert_eq!(result.truncated_by, Some(TruncationLimit::Bytes));
    assert_eq!(result.content, "12345\n54321");
    assert_eq!(result.output_lines, 2);
}

#[test]
fn truncate_head_first_line_exceeds_bytes() {
    let content = "this is a very long first line that exceeds the byte limit\nline2";
    let result = truncate_head(
        content,
        TruncationOptions {
            max_lines: Some(100),
            max_bytes: Some(10),
        },
    );
    assert!(result.truncated);
    assert_eq!(result.truncated_by, Some(TruncationLimit::Bytes));
    assert_eq!(result.content, "");
    assert_eq!(result.output_lines, 0);
    assert_eq!(result.output_bytes, 0);
    assert!(result.first_line_exceeds_limit);
    assert!(result.total_lines > 0);
    assert!(result.total_bytes > 0);
}

#[test]
fn truncate_head_byte_limit_hits_line_boundary() {
    let content = "abc\ndef\nghi";
    // "abc" = 3 bytes, "abc\ndef" = 7 bytes
    let result = truncate_head(
        content,
        TruncationOptions {
            max_lines: Some(100),
            max_bytes: Some(7),
        },
    );
    assert_eq!(result.content, "abc\ndef");
    assert_eq!(result.output_lines, 2);
}

#[test]
fn truncate_head_defaults() {
    let content = "short\n";
    let result = truncate_head(content, TruncationOptions::default());
    assert!(!result.truncated);
    assert_eq!(result.max_lines, DEFAULT_MAX_LINES);
    assert_eq!(result.max_bytes, DEFAULT_MAX_BYTES);
}

#[test]
fn truncate_head_empty_content() {
    let result = truncate_head(
        "",
        TruncationOptions {
            max_lines: Some(10),
            max_bytes: Some(100),
        },
    );
    assert!(!result.truncated);
    assert_eq!(result.content, "");
    assert_eq!(result.total_lines, 0);
    assert_eq!(result.total_bytes, 0);
}

#[test]
fn truncate_head_utf8_multibyte() {
    // "café" is 5 bytes (c-a-f-é where é is 2 bytes), "olá" is 4 bytes
    // "café\n" = 6, "café\nolá" = 6 + 4 = 10
    // Next line "hello" would add "\n" (1) + "hello" (5) = 6 → 16 > 12 → stop
    let content = "café\nolá\nhello";
    let result = truncate_head(
        content,
        TruncationOptions {
            max_lines: Some(100),
            max_bytes: Some(12),
        },
    );
    assert_eq!(result.content, "café\nolá");
    assert_eq!(result.output_lines, 2);
}

#[test]
fn truncate_head_lines_trump_bytes_when_lines_first() {
    // Content where lines limit is hit before bytes limit
    let content = "a\nb\nc\nd\ne\nf";
    let result = truncate_head(
        content,
        TruncationOptions {
            max_lines: Some(3),
            max_bytes: Some(10000),
        },
    );
    assert!(result.truncated);
    assert_eq!(result.truncated_by, Some(TruncationLimit::Lines));
    assert_eq!(result.output_lines, 3);
}

// ---------------------------------------------------------------------------
// truncate_tail
// ---------------------------------------------------------------------------

#[test]
fn truncate_tail_no_truncation() {
    let content = "line1\nline2\nline3";
    let result = truncate_tail(
        content,
        TruncationOptions {
            max_lines: Some(10),
            max_bytes: Some(1024),
        },
    );
    assert!(!result.truncated);
    assert_eq!(result.truncated_by, None);
    assert_eq!(result.content, content);
    assert_eq!(result.total_lines, 3);
    assert_eq!(result.output_lines, 3);
}

#[test]
fn truncate_tail_line_limit() {
    let content = "line1\nline2\nline3\nline4\nline5";
    let result = truncate_tail(
        content,
        TruncationOptions {
            max_lines: Some(3),
            max_bytes: Some(1024),
        },
    );
    assert!(result.truncated);
    assert_eq!(result.truncated_by, Some(TruncationLimit::Lines));
    assert_eq!(result.content, "line3\nline4\nline5");
    assert_eq!(result.output_lines, 3);
}

#[test]
fn truncate_tail_byte_limit() {
    // From end: "54321" (5) + "\n" (1) + "12345" (5) = 11. Next: "\n" (1) + "54321" (5) = 6 → 17 > 12
    let content = "12345\n54321\n12345\n54321\n";
    let result = truncate_tail(
        content,
        TruncationOptions {
            max_lines: Some(100),
            max_bytes: Some(12),
        },
    );
    assert!(result.truncated);
    assert_eq!(result.truncated_by, Some(TruncationLimit::Bytes));
    assert_eq!(result.content, "12345\n54321");
    assert_eq!(result.output_lines, 2);
}

#[test]
fn truncate_tail_partial_first_line() {
    // Last line exceeds byte limit — should return partial end of that line
    let content = "short\nthis is a very very very long last line that far exceeds the limit";
    let result = truncate_tail(
        content,
        TruncationOptions {
            max_lines: Some(100),
            max_bytes: Some(10),
        },
    );
    assert!(result.truncated);
    assert_eq!(result.truncated_by, Some(TruncationLimit::Bytes));
    assert!(result.last_line_partial);
    // The partial line should be the last N chars (≤ 10 bytes), char-boundary-aligned
    assert!(!result.content.contains('\n'));
    assert!(!result.content.is_empty());
    assert!(result.content.len() <= 10);
}

#[test]
fn truncate_tail_byte_limit_no_partial() {
    let content = "aaaa\nbbbb\ncccc\ndddd";
    // "dddd"=4, "\n"+"cccc"=5 → 9, "\n"+"bbbb"=5 → 14 > 12
    let result = truncate_tail(
        content,
        TruncationOptions {
            max_lines: Some(100),
            max_bytes: Some(12),
        },
    );
    assert!(result.truncated);
    assert_eq!(result.truncated_by, Some(TruncationLimit::Bytes));
    assert!(!result.last_line_partial);
    assert_eq!(result.content, "cccc\ndddd");
    assert_eq!(result.output_lines, 2);
}

#[test]
fn truncate_tail_defaults() {
    let content = "short\n";
    let result = truncate_tail(content, TruncationOptions::default());
    assert!(!result.truncated);
    assert_eq!(result.max_lines, DEFAULT_MAX_LINES);
    assert_eq!(result.max_bytes, DEFAULT_MAX_BYTES);
}

#[test]
fn truncate_tail_empty_content() {
    let result = truncate_tail(
        "",
        TruncationOptions {
            max_lines: Some(10),
            max_bytes: Some(100),
        },
    );
    assert!(!result.truncated);
    assert_eq!(result.content, "");
    assert_eq!(result.total_lines, 0);
    assert_eq!(result.total_bytes, 0);
}

#[test]
fn truncate_tail_utf8_multibyte() {
    let content = "hello\ncafé\nolá";
    let result = truncate_tail(
        content,
        TruncationOptions {
            max_lines: Some(100),
            max_bytes: Some(12),
        },
    );
    assert_eq!(result.content, "café\nolá");
    assert_eq!(result.output_lines, 2);
}

#[test]
fn truncate_tail_partial_utf8_boundary() {
    // Last line has multi-byte chars. Partial truncation must respect UTF-8 boundaries.
    // "xaébc" = [0x78, 0x61, 0xC3, 0xA9, 0x62, 0x63] = 6 bytes
    // max_bytes=3 → need last 3 bytes from end: [0xA9, 0x62, 0x63]
    // But 0xA9 is a continuation byte → advance to 0x62 → "bc"
    let content = "xaébc";
    let result = truncate_tail(
        content,
        TruncationOptions {
            max_lines: Some(100),
            max_bytes: Some(3),
        },
    );
    assert!(result.truncated);
    assert!(result.last_line_partial);
    assert_eq!(result.content, "bc");
}

// ---------------------------------------------------------------------------
// truncate_line
// ---------------------------------------------------------------------------

#[test]
fn truncate_line_no_truncation() {
    let result = truncate_line("short", GREP_MAX_LINE_LENGTH);
    assert!(!result.was_truncated);
    assert_eq!(result.text, "short");
}

#[test]
fn truncate_line_with_truncation() {
    let long_line = "x".repeat(600);
    let result = truncate_line(&long_line, GREP_MAX_LINE_LENGTH);
    assert!(result.was_truncated);
    assert!(result.text.starts_with(&"x".repeat(500)));
    assert!(result.text.contains("... [truncated]"));
}

#[test]
fn truncate_line_exact_boundary() {
    let line = "a".repeat(GREP_MAX_LINE_LENGTH);
    let result = truncate_line(&line, GREP_MAX_LINE_LENGTH);
    assert!(!result.was_truncated);
    assert_eq!(result.text, line);
}

#[test]
fn truncate_line_custom_max_chars() {
    let line = "hello world this is a test";
    let result = truncate_line(line, 10);
    assert!(result.was_truncated);
    assert_eq!(result.text, "hello worl... [truncated]");
}

#[test]
fn truncate_line_empty() {
    let result = truncate_line("", GREP_MAX_LINE_LENGTH);
    assert!(!result.was_truncated);
    assert_eq!(result.text, "");
}

#[test]
fn truncate_line_multibyte_chars() {
    // "café" has 4 chars but 5 bytes; the char-count limit is about characters, not bytes
    let result = truncate_line("café", 4);
    assert!(!result.was_truncated);
    assert_eq!(result.text, "café");

    let result = truncate_line("café", 3);
    assert!(result.was_truncated);
    assert_eq!(result.text, "caf... [truncated]");
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

#[test]
fn constants_match_typescript() {
    assert_eq!(DEFAULT_MAX_LINES, 2000);
    assert_eq!(DEFAULT_MAX_BYTES, 50 * 1024);
    assert_eq!(GREP_MAX_LINE_LENGTH, 500);
}

// ---------------------------------------------------------------------------
// TruncationResult fields
// ---------------------------------------------------------------------------

#[test]
fn truncation_result_no_truncation_fields() {
    let content = "hello";
    let result = truncate_head(
        content,
        TruncationOptions {
            max_lines: Some(10),
            max_bytes: Some(100),
        },
    );
    assert_eq!(result.total_lines, 1);
    assert_eq!(result.total_bytes, content.len());
    assert_eq!(result.output_lines, 1);
    assert_eq!(result.output_bytes, content.len());
    assert!(!result.last_line_partial);
    assert!(!result.first_line_exceeds_limit);
    assert_eq!(result.max_lines, 10);
    assert_eq!(result.max_bytes, 100);
}

#[test]
fn truncation_result_head_truncated_fields() {
    let content = "line1\nline2\nline3\nline4\nline5";
    let result = truncate_head(
        content,
        TruncationOptions {
            max_lines: Some(2),
            max_bytes: Some(1024),
        },
    );
    assert!(result.truncated);
    assert_eq!(result.truncated_by, Some(TruncationLimit::Lines));
    assert_eq!(result.total_lines, 5);
    assert_eq!(result.output_lines, 2);
    assert!(!result.last_line_partial);
    assert!(!result.first_line_exceeds_limit);
}

#[test]
fn truncation_result_tail_fields() {
    let content = "line1\nline2\nline3\nline4\nline5";
    let result = truncate_tail(
        content,
        TruncationOptions {
            max_lines: Some(2),
            max_bytes: Some(1024),
        },
    );
    assert!(result.truncated);
    assert_eq!(result.truncated_by, Some(TruncationLimit::Lines));
    assert_eq!(result.total_lines, 5);
    assert_eq!(result.output_lines, 2);
    assert!(!result.last_line_partial);
    assert!(!result.first_line_exceeds_limit);
}
