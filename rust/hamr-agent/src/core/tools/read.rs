//! Port of `packages/coding-agent/src/core/tools/read.ts` — the read tool.
//!
//! Reads files (text or images) with offset/limit support and truncation.
//! Images are detected via magic bytes (not extension) and returned as base64.

use std::path::{Path, PathBuf};

use hamr_ai::types::{ImageContent, MessageContent, TextContent};

use super::path_utils::resolve_read_path_async;
use super::truncate::{self, DEFAULT_MAX_BYTES, TruncationOptions, format_size};

// ---------------------------------------------------------------------------
// Image magic-number detection (mirrors mime.ts)
// ---------------------------------------------------------------------------

const IMAGE_TYPE_SNIFF_BYTES: usize = 4100;

/// Detect a supported image MIME type from the raw leading bytes of a file.
///
/// Returns `None` for non-image content (or unsupported image formats like
/// animated PNG / JPEG 0xf7 vendor extension).
pub fn detect_image_mime_type(buffer: &[u8]) -> Option<&'static str> {
    if starts_with(buffer, &[0xff, 0xd8, 0xff]) {
        if buffer.len() > 3 && buffer[3] == 0xf7 {
            return None;
        }
        return Some("image/jpeg");
    }
    if starts_with(buffer, &PNG_SIGNATURE) {
        if is_png(buffer) && !is_animated_png(buffer) {
            return Some("image/png");
        }
        return None;
    }
    if starts_with_ascii(buffer, 0, "GIF") {
        return Some("image/gif");
    }
    if starts_with_ascii(buffer, 0, "RIFF") && starts_with_ascii(buffer, 8, "WEBP") {
        return Some("image/webp");
    }
    // BMP detection (not in TS mime.ts but added for completeness)
    if starts_with(buffer, &[0x42, 0x4D]) {
        return Some("image/bmp");
    }
    None
}

const PNG_SIGNATURE: [u8; 8] = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];

fn starts_with(buffer: &[u8], bytes: &[u8]) -> bool {
    buffer.len() >= bytes.len() && buffer[..bytes.len()] == *bytes
}

fn starts_with_ascii(buffer: &[u8], offset: usize, text: &str) -> bool {
    if buffer.len() < offset + text.len() {
        return false;
    }
    text.as_bytes()
        .iter()
        .enumerate()
        .all(|(i, &b)| buffer[offset + i] == b)
}

fn read_u32_be(buffer: &[u8], offset: usize) -> u32 {
    ((buffer.get(offset).copied().unwrap_or(0) as u32) << 24)
        | ((buffer.get(offset + 1).copied().unwrap_or(0) as u32) << 16)
        | ((buffer.get(offset + 2).copied().unwrap_or(0) as u32) << 8)
        | (buffer.get(offset + 3).copied().unwrap_or(0) as u32)
}

fn is_png(buffer: &[u8]) -> bool {
    buffer.len() >= 16
        && read_u32_be(buffer, PNG_SIGNATURE.len()) == 13
        && starts_with_ascii(buffer, 12, "IHDR")
}

fn is_animated_png(buffer: &[u8]) -> bool {
    let mut offset = PNG_SIGNATURE.len();
    while offset + 8 <= buffer.len() {
        let chunk_length = read_u32_be(buffer, offset) as usize;
        let chunk_type_offset = offset + 4;
        if starts_with_ascii(buffer, chunk_type_offset, "acTL") {
            return true;
        }
        if starts_with_ascii(buffer, chunk_type_offset, "IDAT") {
            return false;
        }
        let next_offset = offset + 8 + chunk_length + 4;
        if next_offset <= offset || next_offset > buffer.len() {
            return false;
        }
        offset = next_offset;
    }
    false
}

/// Read the first few KB of a file and detect its image MIME type.
pub async fn detect_image_mime_type_from_file(
    path: &Path,
) -> std::io::Result<Option<&'static str>> {
    let data = tokio::fs::read(path).await?;
    let sniff_len = IMAGE_TYPE_SNIFF_BYTES.min(data.len());
    Ok(detect_image_mime_type(&data[..sniff_len]))
}

// ---------------------------------------------------------------------------
// Read tool types
// ---------------------------------------------------------------------------

/// Read tool input parameters (mirrors TS ReadToolInput).
#[derive(Debug, Clone, serde::Deserialize, schemars::JsonSchema)]
pub struct ReadToolInput {
    /// Path to the file to read (relative or absolute).
    pub path: String,
    /// Line number to start reading from (1-indexed).
    #[serde(default)]
    pub offset: Option<usize>,
    /// Maximum number of lines to read.
    #[serde(default)]
    pub limit: Option<usize>,
}

/// Details attached to a read tool result.
#[derive(Debug, Clone)]
pub struct ReadToolDetails {
    pub truncation: Option<truncate::TruncationResult>,
}

/// The result of executing the read tool.
#[derive(Debug, Clone)]
pub struct ReadToolResult {
    pub content: Vec<MessageContent>,
    pub details: Option<ReadToolDetails>,
}

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

#[derive(Debug, thiserror::Error)]
pub enum ReadToolError {
    #[error("File not found: {path}")]
    FileNotFound {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("Permission denied: {path}")]
    PermissionDenied {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("Offset {offset} is beyond end of file ({total_lines} lines total)")]
    OffsetBeyondEof { offset: usize, total_lines: usize },
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

// ---------------------------------------------------------------------------
// ReadTool struct
// ---------------------------------------------------------------------------

/// The read tool — reads text and image files from the filesystem.
pub struct ReadTool {
    cwd: PathBuf,
}

impl ReadTool {
    /// Create a new read tool rooted at `cwd`.
    pub fn new(cwd: &Path) -> Self {
        Self {
            cwd: cwd.to_path_buf(),
        }
    }

    /// Execute the read tool with the given input.
    pub async fn execute(&self, input: &ReadToolInput) -> Result<ReadToolResult, ReadToolError> {
        let absolute_path = resolve_read_path_async(&input.path, &self.cwd).await;

        // Check file existence and readability.
        match tokio::fs::try_exists(&absolute_path).await {
            Ok(true) => {}
            Ok(false) => {
                return Err(ReadToolError::FileNotFound {
                    path: display_path(&absolute_path),
                    source: std::io::Error::from(std::io::ErrorKind::NotFound),
                });
            }
            Err(e) => {
                return Err(ReadToolError::FileNotFound {
                    path: display_path(&absolute_path),
                    source: e,
                });
            }
        }

        // Detect image MIME type (from magic bytes, not extension).
        let mime_type = detect_image_mime_type_from_file(&absolute_path)
            .await
            .ok()
            .flatten();

        if let Some(mime) = mime_type {
            self.read_image(&absolute_path, mime).await
        } else {
            self.read_text(&absolute_path, input.offset, input.limit)
                .await
        }
    }

    /// Read a file as an image.
    async fn read_image(
        &self,
        path: &Path,
        mime_type: &str,
    ) -> Result<ReadToolResult, ReadToolError> {
        let buffer = tokio::fs::read(path).await?;
        let data = base64_encode(&buffer);

        let text_note = format!("Read image file [{mime_type}]");
        let text = TextContent {
            text: text_note,
            text_signature: None,
        };
        let image = ImageContent {
            data,
            mime_type: mime_type.to_string(),
        };

        Ok(ReadToolResult {
            content: vec![MessageContent::Text(text), MessageContent::Image(image)],
            details: None,
        })
    }

    /// Read a file as text, applying offset/limit and truncation.
    async fn read_text(
        &self,
        path: &Path,
        offset: Option<usize>,
        limit: Option<usize>,
    ) -> Result<ReadToolResult, ReadToolError> {
        let buffer = tokio::fs::read(path).await?;
        let text_content = String::from_utf8(buffer)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

        let all_lines = split_lines_retain_empty(&text_content);
        let total_file_lines = all_lines.len();

        // Apply offset (1-indexed → 0-indexed).
        let start_line = offset.map(|o| o.saturating_sub(1)).unwrap_or(0);

        // Check if offset is out of bounds.
        if start_line >= all_lines.len() {
            return Err(ReadToolError::OffsetBeyondEof {
                offset: offset.unwrap_or(1),
                total_lines: all_lines.len(),
            });
        }

        let start_line_display = start_line + 1;

        // Select content: honor user's limit first, then truncateHead.
        let (selected_content, user_limited_lines) = if let Some(lim) = limit {
            let end_line = (start_line + lim).min(all_lines.len());
            let sliced = all_lines[start_line..end_line].join("\n");
            (sliced, Some(end_line - start_line))
        } else {
            let sliced = all_lines[start_line..].join("\n");
            (sliced, None)
        };

        // Apply truncation (line + byte limits).
        let truncation = truncate::truncate_head(&selected_content, TruncationOptions::default());

        let output_text: String;
        let details: Option<ReadToolDetails>;

        if truncation.first_line_exceeds_limit {
            let first_line_bytes = all_lines[start_line].len();
            let first_line_size = format_size(first_line_bytes);
            let byte_limit = format_size(DEFAULT_MAX_BYTES);
            let display_path = display_path(path);
            output_text = format!(
                "[Line {start_line_display} is {first_line_size}, exceeds {byte_limit} limit. \
                 Use bash: sed -n '{start_line_display}p' {display_path} | head -c {}]",
                DEFAULT_MAX_BYTES
            );
            details = Some(ReadToolDetails {
                truncation: Some(truncation),
            });
        } else if truncation.truncated {
            let end_line_display = start_line_display + truncation.output_lines - 1;
            let next_offset = end_line_display + 1;
            if truncation.truncated_by == Some(truncate::TruncationLimit::Lines) {
                output_text = format!(
                    "{}\n\n[Showing lines {}-{} of {}. Use offset={} to continue.]",
                    truncation.content,
                    start_line_display,
                    end_line_display,
                    total_file_lines,
                    next_offset
                );
            } else {
                let byte_limit = format_size(DEFAULT_MAX_BYTES);
                output_text = format!(
                    "{}\n\n[Showing lines {}-{} of {} ({} limit). Use offset={} to continue.]",
                    truncation.content,
                    start_line_display,
                    end_line_display,
                    total_file_lines,
                    byte_limit,
                    next_offset
                );
            }
            details = Some(ReadToolDetails {
                truncation: Some(truncation),
            });
        } else if let Some(user_lines) = user_limited_lines {
            details = None;
            if start_line + user_lines < all_lines.len() {
                let remaining = all_lines.len() - (start_line + user_lines);
                let next_offset = start_line + user_lines + 1;
                output_text = format!(
                    "{}\n\n[{} more lines in file. Use offset={} to continue.]",
                    truncation.content, remaining, next_offset
                );
            } else {
                output_text = truncation.content;
            }
        } else {
            details = None;
            output_text = truncation.content;
        }

        Ok(ReadToolResult {
            content: vec![MessageContent::Text(TextContent {
                text: output_text,
                text_signature: None,
            })],
            details,
        })
    }
}

// ---------------------------------------------------------------------------
// Convenience constructor (mirrors createReadTool)
// ---------------------------------------------------------------------------

/// Create a [`ReadTool`] rooted at `cwd`.
pub fn create_read_tool(cwd: &Path) -> ReadTool {
    ReadTool::new(cwd)
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Split text into lines, preserving empty trailing lines.
/// Unlike `split('\n')` in JS, Rust's `.split('\n')` already preserves
/// trailing empty strings, so this mirrors the JS behavior.
fn split_lines_retain_empty(content: &str) -> Vec<&str> {
    // We want to match the JS behavior:
    // - `"a\nb\nc".split('\n')` → ["a", "b", "c"]
    // - `"hello".split('\n')` → ["hello"]
    // Rust's split with no trimming does the same.
    content.split('\n').collect()
}

/// Encode bytes to base64 (no padding, standard alphabet).
fn base64_encode(data: &[u8]) -> String {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.encode(data)
}

fn display_path(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // Image magic detection
    // -----------------------------------------------------------------------

    #[test]
    fn test_detect_png() {
        // Minimum valid PNG: 8-byte signature + IHDR chunk (length=13, type=IHDR)
        let mut buf = Vec::new();
        buf.extend_from_slice(&PNG_SIGNATURE); // 8 bytes
        buf.extend_from_slice(&13u32.to_be_bytes()); // IHDR length = 13
        buf.extend_from_slice(b"IHDR"); // IHDR type
        // + 13 bytes IHDR data + 4 crc
        buf.extend_from_slice(&[0u8; 17]);
        assert_eq!(detect_image_mime_type(&buf), Some("image/png"));
    }

    #[test]
    fn test_detect_jpeg() {
        let buf = [0xff, 0xd8, 0xff, 0xe0, 0x00, 0x10];
        assert_eq!(detect_image_mime_type(&buf), Some("image/jpeg"));
    }

    #[test]
    fn test_detect_jpeg_xf7_rejected() {
        let buf = [0xff, 0xd8, 0xff, 0xf7];
        assert_eq!(detect_image_mime_type(&buf), None);
    }

    #[test]
    fn test_detect_gif() {
        let buf = b"GIF89a\x01\x00\x01\x00";
        assert_eq!(detect_image_mime_type(buf), Some("image/gif"));
    }

    #[test]
    fn test_detect_webp() {
        let mut buf = Vec::new();
        buf.extend_from_slice(b"RIFF\x00\x00\x00\x00WEBP");
        assert_eq!(detect_image_mime_type(&buf), Some("image/webp"));
    }

    #[test]
    fn test_detect_bmp() {
        let buf = [0x42, 0x4D, 0x00, 0x00];
        assert_eq!(detect_image_mime_type(&buf), Some("image/bmp"));
    }

    #[test]
    fn test_detect_non_image() {
        let buf = b"hello world\nthis is text";
        assert_eq!(detect_image_mime_type(buf), None);
    }

    #[test]
    fn test_is_animated_png_returns_false_for_static() {
        let mut buf = Vec::new();
        buf.extend_from_slice(&PNG_SIGNATURE);
        buf.extend_from_slice(&13u32.to_be_bytes());
        buf.extend_from_slice(b"IHDR");
        buf.extend_from_slice(&[0u8; 13 + 4]); // IHDR data + CRC
        buf.extend_from_slice(&0u32.to_be_bytes());
        buf.extend_from_slice(b"IDAT");
        buf.extend_from_slice(&[0u8; 4]);
        assert!(!is_animated_png(&buf));
    }

    #[test]
    fn test_is_animated_png_detects_actl() {
        let mut buf = Vec::new();
        buf.extend_from_slice(&PNG_SIGNATURE);
        buf.extend_from_slice(&13u32.to_be_bytes());
        buf.extend_from_slice(b"IHDR");
        buf.extend_from_slice(&[0u8; 13 + 4]); // IHDR data + CRC
        buf.extend_from_slice(&0u32.to_be_bytes());
        buf.extend_from_slice(b"acTL");
        buf.extend_from_slice(&[0u8; 4]);
        assert!(is_animated_png(&buf));
    }

    // -----------------------------------------------------------------------
    // ReadTool::read_text (via execute) — integration-style tests
    // -----------------------------------------------------------------------

    fn temp_dir() -> PathBuf {
        // A nanosecond timestamp alone is NOT unique: cargo runs these tests on
        // parallel threads and two can land in the same clock tick, sharing a
        // dir — then one test's trailing `remove_dir_all` deletes another's file
        // mid-read (intermittent failures). An atomic counter guarantees a
        // distinct path per call within the process.
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let id = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let seq = COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("hamr-read-test-{id}-{seq}"));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn write_file(dir: &Path, name: &str, content: &str) -> PathBuf {
        let path = dir.join(name);
        std::fs::write(&path, content).unwrap();
        path
    }

    fn get_text_output(result: &ReadToolResult) -> String {
        result
            .content
            .iter()
            .filter_map(|c| match c {
                MessageContent::Text(t) => Some(t.text.as_str()),
                _ => None,
            })
            .collect::<Vec<&str>>()
            .join("\n")
    }

    #[tokio::test]
    async fn test_read_fits_within_limits() {
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

        let output = get_text_output(&result);
        assert_eq!(output, "Hello, world!\nLine 2\nLine 3");
        assert!(!output.contains("Use offset="));
        assert!(result.details.is_none());

        std::fs::remove_dir_all(&dir).ok();
    }

    #[tokio::test]
    async fn test_read_nonexistent_file() {
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
        assert!(msg.contains("not found") || msg.contains("File not found"));

        std::fs::remove_dir_all(&dir).ok();
    }

    #[tokio::test]
    async fn test_read_truncates_exceeding_lines() {
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

        let output = get_text_output(&result);
        assert!(output.contains("Line 1"));
        assert!(output.contains("Line 2000"));
        assert!(!output.contains("Line 2001"));
        assert!(output.contains("[Showing lines 1-2000 of 2500. Use offset=2001 to continue.]"));

        std::fs::remove_dir_all(&dir).ok();
    }

    #[tokio::test]
    async fn test_read_truncates_exceeding_bytes() {
        let dir = temp_dir();
        // Each line is ~200 bytes, 500 lines → roughly 100KB, exceeds 50KB limit.
        let big_line = "x".repeat(200);
        let lines: Vec<String> = (1..=500).map(|i| format!("Line {i}: {big_line}")).collect();
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

        let output = get_text_output(&result);
        assert!(output.contains("Line 1:"));
        // Should mention byte limit and have continuation notice
        assert!(
            output.contains("[Showing lines 1-") && output.contains("50.0KB limit). Use offset=")
        );

        std::fs::remove_dir_all(&dir).ok();
    }

    #[tokio::test]
    async fn test_read_with_offset() {
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

        let output = get_text_output(&result);
        assert!(!output.contains("Line 50"));
        assert!(output.contains("Line 51"));
        assert!(output.contains("Line 100"));
        assert!(!output.contains("Use offset="));

        std::fs::remove_dir_all(&dir).ok();
    }

    #[tokio::test]
    async fn test_read_with_limit() {
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

        let output = get_text_output(&result);
        assert!(output.contains("Line 1"));
        assert!(output.contains("Line 10"));
        assert!(!output.contains("Line 11"));
        assert!(output.contains("[90 more lines in file. Use offset=11 to continue.]"));

        std::fs::remove_dir_all(&dir).ok();
    }

    #[tokio::test]
    async fn test_read_offset_and_limit_together() {
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

        let output = get_text_output(&result);
        assert!(!output.contains("Line 40"));
        assert!(output.contains("Line 41"));
        assert!(output.contains("Line 60"));
        assert!(!output.contains("Line 61"));
        assert!(output.contains("[40 more lines in file. Use offset=61 to continue.]"));

        std::fs::remove_dir_all(&dir).ok();
    }

    #[tokio::test]
    async fn test_read_offset_beyond_eof() {
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

        std::fs::remove_dir_all(&dir).ok();
    }

    #[tokio::test]
    async fn test_read_truncation_details() {
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
        assert_eq!(trunc.truncated_by, Some(truncate::TruncationLimit::Lines));
        assert_eq!(trunc.total_lines, 2500);
        assert_eq!(trunc.output_lines, 2000);

        std::fs::remove_dir_all(&dir).ok();
    }

    /// Test detecting image MIME type from file magic (not extension).
    #[tokio::test]
    async fn test_detect_image_from_magic_not_extension() {
        let dir = temp_dir();

        // Build a valid 1x1 PNG from scratch.
        // PNG signature + IHDR (13 bytes, 1x1 8-bit grayscale) + IDAT + IEND
        let mut png_data = Vec::new();
        png_data.extend_from_slice(&PNG_SIGNATURE);

        // IHDR: width=1, height=1, bit_depth=8, color_type=0 (grayscale)
        let mut ihdr_data = Vec::new();
        ihdr_data.extend_from_slice(&1u32.to_be_bytes()); // width
        ihdr_data.extend_from_slice(&1u32.to_be_bytes()); // height
        ihdr_data.push(8); // bit depth
        ihdr_data.push(0); // color type (grayscale)
        ihdr_data.push(0); // compression
        ihdr_data.push(0); // filter
        ihdr_data.push(0); // interlace

        // IHDR chunk
        png_data.extend_from_slice(&13u32.to_be_bytes()); // length
        png_data.extend_from_slice(b"IHDR");
        png_data.extend_from_slice(&ihdr_data);
        // CRC for IHDR (fake — tools will still detect it as PNG by signature+IHDR)
        let ihdr_crc = crc32(b"IHDR", &ihdr_data);
        png_data.extend_from_slice(&ihdr_crc.to_be_bytes());

        // IDAT chunk: zlib-compressed filtered row
        // For 1x1 grayscale with filter byte 0: raw = [0x00, 0x00]
        // zlib compressed (RFC 1950 + deflate):
        // zlib header: 0x78 0x01 (no dict, level 0)
        // deflate: 0x01 (final block, no compression)
        // len: 2, nlen: 0xFFFD, data: 0x00 0x00
        let idat_raw = [0x78, 0x01, 0x01, 0x02, 0x00, 0xFD, 0xFF, 0x00, 0x00];
        png_data.extend_from_slice(&(idat_raw.len() as u32).to_be_bytes());
        png_data.extend_from_slice(b"IDAT");
        png_data.extend_from_slice(&idat_raw);
        let idat_crc = crc32(b"IDAT", &idat_raw);
        png_data.extend_from_slice(&idat_crc.to_be_bytes());

        // IEND chunk
        png_data.extend_from_slice(&0u32.to_be_bytes());
        png_data.extend_from_slice(b"IEND");
        let iend_crc = crc32(b"IEND", &[]);
        png_data.extend_from_slice(&iend_crc.to_be_bytes());

        // Write it as a .txt file (not .png) — detection should be by magic, not extension.
        let test_file = write_file(&dir, "image.txt", unsafe {
            std::str::from_utf8_unchecked(&png_data)
        });

        let tool = ReadTool::new(&dir);
        let result = tool
            .execute(&ReadToolInput {
                path: test_file.to_string_lossy().into_owned(),
                offset: None,
                limit: None,
            })
            .await
            .unwrap();

        // First content block should be text describing the image.
        let text = get_text_output(&result);
        assert!(text.contains("Read image file [image/png]"));

        // Second content block should be an image.
        let has_image = result
            .content
            .iter()
            .any(|c| matches!(c, MessageContent::Image(_)));
        assert!(has_image, "Expected an image content block");

        std::fs::remove_dir_all(&dir).ok();
    }

    /// Files with image extension but non-image content should be treated as text.
    #[tokio::test]
    async fn test_image_extension_but_text_content_treated_as_text() {
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

        let output = get_text_output(&result);
        assert!(output.contains("definitely not a png"));

        let has_image = result
            .content
            .iter()
            .any(|c| matches!(c, MessageContent::Image(_)));
        assert!(
            !has_image,
            "Should not have an image block for non-image content"
        );

        std::fs::remove_dir_all(&dir).ok();
    }
}

/// Very simple CRC-32 implementation for test PNG generation.
#[cfg(test)]
fn crc32(chunk_type: &[u8], chunk_data: &[u8]) -> u32 {
    // CRC-32 using IEEE polynomial
    let mut crc: u32 = 0xFFFF_FFFF;
    // Process the chunk type bytes
    for &byte in chunk_type {
        crc = crc_table_entry(crc, byte);
    }
    // Process the chunk data bytes
    for &byte in chunk_data {
        crc = crc_table_entry(crc, byte);
    }
    !crc
}

#[cfg(test)]
fn crc_table_entry(crc: u32, byte: u8) -> u32 {
    // IEEE 802.3 CRC-32 polynomial
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
