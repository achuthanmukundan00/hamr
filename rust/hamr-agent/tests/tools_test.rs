//! Tests for hamr-agent tools — read tool.
//! Mirrors the test coverage from `packages/coding-agent/test/tools.test.ts`.

use hamr_agent::core::tools::read::*;
use std::fs;
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

fn temp_dir() -> PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let seq = COUNTER.fetch_add(1, Ordering::Relaxed);
    let id = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("hamr-tools-test-{id}-{seq}"));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn write_file(dir: &PathBuf, name: &str, content: &str) -> PathBuf {
    let path = dir.join(name);
    std::fs::write(&path, &content).unwrap();
    path
}

fn get_text_output(content: &[hamr_ai::types::MessageContent]) -> String {
    content
        .iter()
        .filter_map(|c| match c {
            hamr_ai::types::MessageContent::Text(t) => Some(t.text.as_str()),
            _ => None,
        })
        .collect::<Vec<&str>>()
        .join("\n")
}

// ---------------------------------------------------------------------------
// read tool
// ---------------------------------------------------------------------------

#[tokio::test]
async fn should_read_file_contents_that_fit_within_limits() {
    let dir = temp_dir();
    let test_file = write_file(&dir, "test.txt", "Hello, world!\nLine 2\nLine 3");
    let tool = ReadTool::new(&dir);

    let result = tool
        .execute(&ReadToolInput {
            path: test_file.to_string_lossy().into_owned(),
            offset: None,
            limit: None,
        })
        .await
        .unwrap();

    let output = get_text_output(&result.content);
    assert_eq!(output, "Hello, world!\nLine 2\nLine 3");
    // No truncation message since file fits within limits
    assert!(!output.contains("Use offset="));
    assert!(result.details.is_none());

    fs::remove_dir_all(&dir).ok();
}

#[tokio::test]
async fn should_handle_non_existent_files() {
    let dir = temp_dir();
    let nonexistent = dir.join("nonexistent.txt");
    let tool = ReadTool::new(&dir);

    let err = tool
        .execute(&ReadToolInput {
            path: nonexistent.to_string_lossy().into_owned(),
            offset: None,
            limit: None,
        })
        .await
        .unwrap_err();

    let msg = err.to_string();
    assert!(
        msg.to_lowercase().contains("not found") || msg.to_lowercase().contains("file not found")
    );

    fs::remove_dir_all(&dir).ok();
}

#[tokio::test]
async fn should_truncate_files_exceeding_line_limit() {
    let dir = temp_dir();
    let lines: Vec<String> = (1..=2500).map(|i| format!("Line {i}")).collect();
    let content = lines.join("\n");
    let test_file = write_file(&dir, "large.txt", &content);
    let tool = ReadTool::new(&dir);

    let result = tool
        .execute(&ReadToolInput {
            path: test_file.to_string_lossy().into_owned(),
            offset: None,
            limit: None,
        })
        .await
        .unwrap();

    let output = get_text_output(&result.content);

    assert!(output.contains("Line 1"));
    assert!(output.contains("Line 2000"));
    assert!(!output.contains("Line 2001"));
    assert!(output.contains("[Showing lines 1-2000 of 2500. Use offset=2001 to continue.]"));

    fs::remove_dir_all(&dir).ok();
}

#[tokio::test]
async fn should_truncate_when_byte_limit_exceeded() {
    let dir = temp_dir();
    // Create file that exceeds 50KB byte limit but has fewer than 2000 lines
    let lines: Vec<String> = (1..=500)
        .map(|i| format!("Line {i}: {}", "x".repeat(200)))
        .collect();
    let content = lines.join("\n");
    let test_file = write_file(&dir, "large-bytes.txt", &content);
    let tool = ReadTool::new(&dir);

    let result = tool
        .execute(&ReadToolInput {
            path: test_file.to_string_lossy().into_owned(),
            offset: None,
            limit: None,
        })
        .await
        .unwrap();

    let output = get_text_output(&result.content);

    assert!(output.contains("Line 1:"));
    // Should show byte limit message
    assert!(
        output.contains("[Showing lines 1-")
            && output.contains(" of 500 (")
            && output.contains(" limit). Use offset=")
    );

    fs::remove_dir_all(&dir).ok();
}

#[tokio::test]
async fn should_handle_offset_parameter() {
    let dir = temp_dir();
    let lines: Vec<String> = (1..=100).map(|i| format!("Line {i}")).collect();
    let content = lines.join("\n");
    let test_file = write_file(&dir, "offset-test.txt", &content);
    let tool = ReadTool::new(&dir);

    let result = tool
        .execute(&ReadToolInput {
            path: test_file.to_string_lossy().into_owned(),
            offset: Some(51),
            limit: None,
        })
        .await
        .unwrap();

    let output = get_text_output(&result.content);

    assert!(!output.contains("Line 50"));
    assert!(output.contains("Line 51"));
    assert!(output.contains("Line 100"));
    // No truncation message since file fits within limits
    assert!(!output.contains("Use offset="));

    fs::remove_dir_all(&dir).ok();
}

#[tokio::test]
async fn should_handle_limit_parameter() {
    let dir = temp_dir();
    let lines: Vec<String> = (1..=100).map(|i| format!("Line {i}")).collect();
    let content = lines.join("\n");
    let test_file = write_file(&dir, "limit-test.txt", &content);
    let tool = ReadTool::new(&dir);

    let result = tool
        .execute(&ReadToolInput {
            path: test_file.to_string_lossy().into_owned(),
            offset: None,
            limit: Some(10),
        })
        .await
        .unwrap();

    let output = get_text_output(&result.content);

    assert!(output.contains("Line 1"));
    assert!(output.contains("Line 10"));
    assert!(!output.contains("Line 11"));
    assert!(output.contains("[90 more lines in file. Use offset=11 to continue.]"));

    fs::remove_dir_all(&dir).ok();
}

#[tokio::test]
async fn should_handle_offset_and_limit_together() {
    let dir = temp_dir();
    let lines: Vec<String> = (1..=100).map(|i| format!("Line {i}")).collect();
    let content = lines.join("\n");
    let test_file = write_file(&dir, "offset-limit-test.txt", &content);
    let tool = ReadTool::new(&dir);

    let result = tool
        .execute(&ReadToolInput {
            path: test_file.to_string_lossy().into_owned(),
            offset: Some(41),
            limit: Some(20),
        })
        .await
        .unwrap();

    let output = get_text_output(&result.content);

    assert!(!output.contains("Line 40"));
    assert!(output.contains("Line 41"));
    assert!(output.contains("Line 60"));
    assert!(!output.contains("Line 61"));
    assert!(output.contains("[40 more lines in file. Use offset=61 to continue.]"));

    fs::remove_dir_all(&dir).ok();
}

#[tokio::test]
async fn should_show_error_when_offset_is_beyond_file_length() {
    let dir = temp_dir();
    let test_file = write_file(&dir, "short.txt", "Line 1\nLine 2\nLine 3");
    let tool = ReadTool::new(&dir);

    let err = tool
        .execute(&ReadToolInput {
            path: test_file.to_string_lossy().into_owned(),
            offset: Some(100),
            limit: None,
        })
        .await
        .unwrap_err();

    let msg = err.to_string();
    assert!(msg.contains("Offset 100 is beyond end of file (3 lines total)"));

    fs::remove_dir_all(&dir).ok();
}

#[tokio::test]
async fn should_include_truncation_details_when_truncated() {
    let dir = temp_dir();
    let lines: Vec<String> = (1..=2500).map(|i| format!("Line {i}")).collect();
    let content = lines.join("\n");
    let test_file = write_file(&dir, "large-file.txt", &content);
    let tool = ReadTool::new(&dir);

    let result = tool
        .execute(&ReadToolInput {
            path: test_file.to_string_lossy().into_owned(),
            offset: None,
            limit: None,
        })
        .await
        .unwrap();

    assert!(result.details.is_some());
    let details = result.details.as_ref().unwrap();
    assert!(details.truncation.is_some());
    let trunc = details.truncation.as_ref().unwrap();
    assert!(trunc.truncated);
    assert_eq!(
        trunc.truncated_by,
        Some(hamr_agent::core::tools::truncate::TruncationLimit::Lines)
    );
    assert_eq!(trunc.total_lines, 2500);
    assert_eq!(trunc.output_lines, 2000);

    fs::remove_dir_all(&dir).ok();
}

#[tokio::test]
async fn should_detect_image_mime_type_from_file_magic_not_extension() {
    let dir = temp_dir();

    // Minimum valid 1x1 PNG
    let png_signature: [u8; 8] = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
    let mut png_data = Vec::new();
    png_data.extend_from_slice(&png_signature);

    let mut ihdr_data = Vec::new();
    ihdr_data.extend_from_slice(&1u32.to_be_bytes()); // width
    ihdr_data.extend_from_slice(&1u32.to_be_bytes()); // height
    ihdr_data.push(8); // bit depth
    ihdr_data.push(0); // color type (grayscale)
    ihdr_data.push(0); // compression
    ihdr_data.push(0); // filter
    ihdr_data.push(0); // interlace

    png_data.extend_from_slice(&13u32.to_be_bytes());
    png_data.extend_from_slice(b"IHDR");
    png_data.extend_from_slice(&ihdr_data);
    let ihdr_crc = tools_test_crc32(b"IHDR", &ihdr_data);
    png_data.extend_from_slice(&ihdr_crc.to_be_bytes());

    let idat_raw = [0x78, 0x01, 0x01, 0x02, 0x00, 0xFD, 0xFF, 0x00, 0x00];
    png_data.extend_from_slice(&(idat_raw.len() as u32).to_be_bytes());
    png_data.extend_from_slice(b"IDAT");
    png_data.extend_from_slice(&idat_raw);
    let idat_crc = tools_test_crc32(b"IDAT", &idat_raw);
    png_data.extend_from_slice(&idat_crc.to_be_bytes());

    png_data.extend_from_slice(&0u32.to_be_bytes());
    png_data.extend_from_slice(b"IEND");
    let iend_crc = tools_test_crc32(b"IEND", &[]);
    png_data.extend_from_slice(&iend_crc.to_be_bytes());

    // Write as .txt file (not .png) — detection should be by magic, not extension
    let test_file = dir.join("image.txt");
    std::fs::write(&test_file, &png_data).unwrap();

    let tool = ReadTool::new(&dir);

    let result = tool
        .execute(&ReadToolInput {
            path: test_file.to_string_lossy().into_owned(),
            offset: None,
            limit: None,
        })
        .await
        .unwrap();

    let text = get_text_output(&result.content);
    assert!(text.contains("Read image file [image/png]"));

    let has_image = result
        .content
        .iter()
        .any(|c| matches!(c, hamr_ai::types::MessageContent::Image(_)));
    assert!(has_image, "Expected an image content block");

    fs::remove_dir_all(&dir).ok();
}

#[tokio::test]
async fn should_treat_files_with_image_extension_but_non_image_content_as_text() {
    let dir = temp_dir();
    let test_file = write_file(&dir, "not-an-image.png", "definitely not a png");
    let tool = ReadTool::new(&dir);

    let result = tool
        .execute(&ReadToolInput {
            path: test_file.to_string_lossy().into_owned(),
            offset: None,
            limit: None,
        })
        .await
        .unwrap();

    let output = get_text_output(&result.content);
    assert!(output.contains("definitely not a png"));

    let has_image = result
        .content
        .iter()
        .any(|c| matches!(c, hamr_ai::types::MessageContent::Image(_)));
    assert!(
        !has_image,
        "Should not have an image block for non-image content"
    );

    fs::remove_dir_all(&dir).ok();
}

// ---------------------------------------------------------------------------
// CRC32 helper for test PNG generation
// ---------------------------------------------------------------------------

fn tools_test_crc32(chunk_type: &[u8], chunk_data: &[u8]) -> u32 {
    let mut crc: u32 = 0xFFFF_FFFF;
    for &byte in chunk_type {
        crc = tools_test_crc_table(crc, byte);
    }
    for &byte in chunk_data {
        crc = tools_test_crc_table(crc, byte);
    }
    !crc
}

fn tools_test_crc_table(crc: u32, byte: u8) -> u32 {
    let mut c = crc ^ (byte as u32);
    for _ in 0..8 {
        if c & 1 != 0 {
            c = 0xEDB8_8320u32 ^ (c >> 1);
        } else {
            c >>= 1;
        }
    }
    c
}
