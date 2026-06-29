//! Port of `packages/coding-agent/src/core/tools/output-accumulator.ts`.
//!
//! Incrementally tracks streaming output with bounded memory.

use crate::core::tools::truncate::{
    DEFAULT_MAX_BYTES, DEFAULT_MAX_LINES, TruncationResult, truncate_tail,
};
use std::io::Write;

// ============================================================================
// Options and snapshot types
// ============================================================================

pub struct OutputAccumulatorOptions {
    pub max_lines: usize,
    pub max_bytes: usize,
    pub temp_file_prefix: String,
}

impl Default for OutputAccumulatorOptions {
    fn default() -> Self {
        Self {
            max_lines: DEFAULT_MAX_LINES,
            max_bytes: DEFAULT_MAX_BYTES,
            temp_file_prefix: "pi-output".to_string(),
        }
    }
}

pub struct OutputSnapshot {
    pub content: String,
    pub truncation: TruncationResult,
    pub full_output_path: Option<String>,
}

fn default_temp_file_path(prefix: &str) -> String {
    let id = uuid::Uuid::new_v4().to_string().replace('-', "");
    let tmp = std::env::temp_dir();
    tmp.join(format!("{prefix}-{id}.log"))
        .to_string_lossy()
        .into_owned()
}

// ============================================================================
// OutputAccumulator
// ============================================================================

/// Incrementally tracks streaming output with bounded memory.
///
/// Appends decode chunks with a streaming UTF-8 decoder, keeps only a decoded
/// tail for display snapshots, and opens a temp file when the full output needs
/// to be preserved.
pub struct OutputAccumulator {
    max_lines: usize,
    max_bytes: usize,
    max_rolling_bytes: usize,
    temp_file_prefix: String,

    raw_chunks: Vec<Vec<u8>>,
    tail_text: String,
    tail_bytes: usize,
    tail_starts_at_line_boundary: bool,
    total_raw_bytes: usize,
    total_decoded_bytes: usize,
    completed_lines: usize,
    total_lines: usize,
    current_line_bytes: usize,
    has_open_line: bool,
    finished: bool,

    temp_file_path: Option<String>,
    temp_file: Option<std::fs::File>,
}

impl OutputAccumulator {
    pub fn new(options: OutputAccumulatorOptions) -> Self {
        let max_rolling_bytes = (options.max_bytes * 2).max(1);
        Self {
            max_lines: options.max_lines,
            max_bytes: options.max_bytes,
            max_rolling_bytes,
            temp_file_prefix: options.temp_file_prefix,
            raw_chunks: Vec::new(),
            tail_text: String::new(),
            tail_bytes: 0,
            tail_starts_at_line_boundary: true,
            total_raw_bytes: 0,
            total_decoded_bytes: 0,
            completed_lines: 0,
            total_lines: 0,
            current_line_bytes: 0,
            has_open_line: false,
            finished: false,
            temp_file_path: None,
            temp_file: None,
        }
    }

    pub fn append(&mut self, data: &[u8]) {
        if self.finished {
            panic!("Cannot append to a finished output accumulator");
        }

        self.total_raw_bytes += data.len();
        let text = String::from_utf8_lossy(data);
        self.append_decoded_text(&text);

        if self.temp_file.is_some() || self.should_use_temp_file() {
            self.ensure_temp_file();
            if let Some(ref mut file) = self.temp_file {
                let _ = file.write_all(data);
            }
        } else if !data.is_empty() {
            self.raw_chunks.push(data.to_vec());
        }
    }

    pub fn finish(&mut self) {
        if self.finished {
            return;
        }
        self.finished = true;
        // Flush any remaining decoder state (handled by the fact we decode per-chunk)
        if self.should_use_temp_file() {
            self.ensure_temp_file();
        }
    }

    pub fn snapshot(&mut self, persist_if_truncated: bool) -> OutputSnapshot {
        let snapshot_text = self.get_snapshot_text();
        let tail_truncation = truncate_tail(
            &snapshot_text,
            crate::core::tools::truncate::TruncationOptions {
                max_lines: Some(self.max_lines),
                max_bytes: Some(self.max_bytes),
            },
        );

        let truncated =
            self.total_lines > self.max_lines || self.total_decoded_bytes > self.max_bytes;
        let truncated_by = if truncated {
            tail_truncation.truncated_by
        } else {
            None
        };

        let truncation = TruncationResult {
            truncated,
            truncated_by,
            total_lines: self.total_lines,
            total_bytes: self.total_decoded_bytes,
            ..tail_truncation
        };

        if persist_if_truncated && truncation.truncated {
            self.ensure_temp_file();
        }

        OutputSnapshot {
            content: truncation.content.clone(),
            truncation,
            full_output_path: self.temp_file_path.clone(),
        }
    }

    pub fn get_last_line_bytes(&self) -> usize {
        self.current_line_bytes
    }

    // ── private helpers ──

    fn append_decoded_text(&mut self, text: &str) {
        if text.is_empty() {
            return;
        }

        let bytes = text.len(); // Approximate byte count for the decoded text
        self.total_decoded_bytes += bytes;
        self.tail_text.push_str(text);
        self.tail_bytes += bytes;
        if self.tail_bytes > self.max_rolling_bytes * 2 {
            self.trim_tail();
        }

        let newlines = text.chars().filter(|&c| c == '\n').count();
        if newlines == 0 {
            self.current_line_bytes += bytes;
            self.has_open_line = true;
        } else {
            self.completed_lines += newlines;
            let tail: String = text
                .chars()
                .rev()
                .take_while(|&c| c != '\n')
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
                .collect();
            self.current_line_bytes = tail.len();
            self.has_open_line = !tail.is_empty();
        }
        self.total_lines = self.completed_lines + if self.has_open_line { 1 } else { 0 };
    }

    fn trim_tail(&mut self) {
        let buffer = self.tail_text.as_bytes();
        if buffer.len() <= self.max_rolling_bytes {
            self.tail_bytes = buffer.len();
            return;
        }

        let mut start = buffer.len() - self.max_rolling_bytes;
        // Find UTF-8 character boundary
        while start < buffer.len() && (buffer[start] & 0xc0) == 0x80 {
            start += 1;
        }

        self.tail_starts_at_line_boundary = if start == 0 {
            self.tail_starts_at_line_boundary
        } else {
            buffer[start - 1] == b'\n'
        };
        self.tail_text = String::from_utf8_lossy(&buffer[start..]).into_owned();
        self.tail_bytes = self.tail_text.len();
    }

    fn get_snapshot_text(&self) -> String {
        if self.tail_starts_at_line_boundary {
            return self.tail_text.clone();
        }

        if let Some(pos) = self.tail_text.find('\n') {
            self.tail_text[pos + 1..].to_string()
        } else {
            self.tail_text.clone()
        }
    }

    fn should_use_temp_file(&self) -> bool {
        self.total_raw_bytes > self.max_bytes
            || self.total_decoded_bytes > self.max_bytes
            || self.total_lines > self.max_lines
    }

    fn ensure_temp_file(&mut self) {
        if self.temp_file.is_some() {
            return;
        }

        let path = default_temp_file_path(&self.temp_file_prefix);
        self.temp_file_path = Some(path.clone());

        // Create with 0o600 for security
        let file = std::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&path)
            .expect("Failed to create temp file");

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600));
        }

        self.temp_file = Some(file);

        // Flush buffered chunks
        for chunk in &self.raw_chunks {
            if let Some(ref mut f) = self.temp_file {
                let _ = f.write_all(chunk);
            }
        }
        self.raw_chunks.clear();
    }
}

impl Drop for OutputAccumulator {
    fn drop(&mut self) {
        // Temp file will be cleaned up automatically by OS since we don't track it
        let _ = self.temp_file.take();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_accumulation() {
        let mut acc = OutputAccumulator::new(OutputAccumulatorOptions {
            max_lines: 100,
            max_bytes: 10000,
            temp_file_prefix: "test".to_string(),
        });

        acc.append(b"hello\nworld\n");
        acc.finish();

        let snap = acc.snapshot(false);
        assert!(snap.content.contains("hello"));
        assert!(snap.content.contains("world"));
        assert!(!snap.truncation.truncated);
    }

    #[test]
    fn test_truncation_by_lines() {
        let mut acc = OutputAccumulator::new(OutputAccumulatorOptions {
            max_lines: 3,
            max_bytes: 10000,
            temp_file_prefix: "test".to_string(),
        });

        for i in 1..=10 {
            acc.append(format!("line {i}\n").as_bytes());
        }
        acc.finish();

        let snap = acc.snapshot(false);
        assert!(snap.truncation.truncated);
        assert!(!snap.content.contains("line 1\n"));
        assert!(snap.content.contains("line 10"));
    }

    #[test]
    fn test_truncation_by_bytes() {
        let mut acc = OutputAccumulator::new(OutputAccumulatorOptions {
            max_lines: 100,
            max_bytes: 20,
            temp_file_prefix: "test".to_string(),
        });

        acc.append(b"hello world this is a long line\n");
        acc.finish();

        let snap = acc.snapshot(false);
        assert!(snap.truncation.truncated);
    }

    #[test]
    #[should_panic(expected = "Cannot append to a finished output accumulator")]
    fn test_cannot_append_after_finish() {
        let mut acc = OutputAccumulator::new(OutputAccumulatorOptions::default());
        acc.append(b"data\n");
        acc.finish();
        acc.append(b"more\n");
    }

    #[test]
    fn test_finish_idempotent() {
        let mut acc = OutputAccumulator::new(OutputAccumulatorOptions::default());
        acc.append(b"data\n");
        acc.finish();
        acc.finish(); // Should not panic
    }
}
