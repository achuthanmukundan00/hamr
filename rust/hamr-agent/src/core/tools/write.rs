//! Port of `packages/coding-agent/src/core/tools/write.ts`.
//!
//! The write tool writes content to a file. Creates the file if it doesn't
//! exist, overwrites if it does. Automatically creates parent directories.

use std::path::PathBuf;

use crate::core::tools::path_guard::PathGuard;
use crate::core::tools::path_utils::resolve_to_cwd;

// ---------------------------------------------------------------------------
// Tool input
// ---------------------------------------------------------------------------

/// Input for the write tool — mirrors the TypeBox schema in write.ts.
#[derive(Debug, Clone, serde::Deserialize, schemars::JsonSchema)]
pub struct WriteToolInput {
    /// Path to the file to write (relative or absolute).
    pub path: String,
    /// Content to write to the file.
    pub content: String,
}

// ---------------------------------------------------------------------------
// Tool output
// ---------------------------------------------------------------------------

/// Successful output from the write tool.
#[derive(Debug, Clone, serde::Serialize)]
pub struct WriteToolOutput {
    /// Human-readable success message.
    pub message: String,
    /// Number of bytes written.
    pub bytes_written: usize,
}

// ---------------------------------------------------------------------------
// WriteTool
// ---------------------------------------------------------------------------

/// The write tool: writes content to a file, creating parent directories
/// as needed. Enforces path confinement via [`PathGuard`].
pub struct WriteTool {
    cwd: PathBuf,
    guard: PathGuard,
}

impl WriteTool {
    /// Create a new write tool rooted at `cwd`.
    pub fn new(cwd: impl Into<PathBuf>) -> Self {
        Self {
            cwd: cwd.into(),
            guard: PathGuard::default(),
        }
    }

    /// Create a new write tool with an explicit [`PathGuard`].
    pub fn with_guard(cwd: impl Into<PathBuf>, guard: PathGuard) -> Self {
        Self {
            cwd: cwd.into(),
            guard,
        }
    }
}

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

/// Errors that can occur during write tool execution.
#[derive(Debug, thiserror::Error)]
pub enum WriteToolError {
    /// The path is outside the sandbox or on the denylist.
    #[error("{0}")]
    PathGuard(String),

    /// I/O error while writing the file.
    #[error("Failed to write file '{path}': {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    /// I/O error while creating parent directories.
    #[error("Failed to create parent directories for '{path}': {source}")]
    Mkdir {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}

// ---------------------------------------------------------------------------
// Execution
// ---------------------------------------------------------------------------

impl WriteTool {
    /// Execute the write operation.
    ///
    /// 1. Resolves `input.path` relative to `self.cwd`.
    /// 2. Checks the resolved path against [`PathGuard`].
    /// 3. Creates parent directories recursively.
    /// 4. Writes `input.content` to the file.
    ///
    /// Returns a success message with byte count on success.
    pub fn execute(&self, input: &WriteToolInput) -> Result<WriteToolOutput, WriteToolError> {
        // 1. Resolve path
        let absolute_path = resolve_to_cwd(&input.path, &self.cwd);

        // 2. Path guard check
        self.guard
            .assert_writable(&absolute_path)
            .map_err(WriteToolError::PathGuard)?;

        // 3. Create parent directories
        if let Some(parent) = absolute_path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent).map_err(|e| WriteToolError::Mkdir {
                    path: absolute_path.clone(),
                    source: e,
                })?;
            }
        }

        // 4. Write file
        let bytes = input.content.as_bytes();
        std::fs::write(&absolute_path, bytes).map_err(|e| WriteToolError::Io {
            path: absolute_path.clone(),
            source: e,
        })?;

        let len = bytes.len();
        Ok(WriteToolOutput {
            message: format!("Successfully wrote {len} bytes to {}", input.path),
            bytes_written: len,
        })
    }
}

// ---------------------------------------------------------------------------
// Convenience constructor (mirrors createWriteTool in TS)
// ---------------------------------------------------------------------------

/// Create a [`WriteTool`] rooted at `cwd`.
///
/// Equivalent to `createWriteTool(cwd)` in the TypeScript source.
pub fn create_write_tool(cwd: impl Into<PathBuf>) -> WriteTool {
    WriteTool::new(cwd)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn tmp_dir() -> PathBuf {
        std::env::temp_dir().join(format!("write-tool-test-{}", std::process::id()))
    }

    fn setup_test_dir(name: &str) -> PathBuf {
        let dir = tmp_dir().join(name);
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn should_write_file_contents() {
        let cwd = setup_test_dir("should_write");
        let tool = WriteTool::new(&cwd);

        let input = WriteToolInput {
            path: "hello.txt".to_string(),
            content: "Hello, world!".to_string(),
        };

        let output = tool.execute(&input).unwrap();
        assert_eq!(output.bytes_written, 13);
        assert!(output.message.contains("hello.txt"));

        let written = std::fs::read_to_string(cwd.join("hello.txt")).unwrap();
        assert_eq!(written, "Hello, world!");

        // Clean up
        let _ = std::fs::remove_dir_all(cwd);
    }

    #[test]
    fn should_create_parent_directories() {
        let cwd = setup_test_dir("should_create_dirs");
        let tool = WriteTool::new(&cwd);

        let input = WriteToolInput {
            path: "deep/nested/dir/file.txt".to_string(),
            content: "nested content".to_string(),
        };

        let output = tool.execute(&input).unwrap();
        assert_eq!(output.bytes_written, 14);

        let written = std::fs::read_to_string(cwd.join("deep/nested/dir/file.txt")).unwrap();
        assert_eq!(written, "nested content");

        // Verify the directories exist
        assert!(cwd.join("deep").is_dir());
        assert!(cwd.join("deep/nested").is_dir());
        assert!(cwd.join("deep/nested/dir").is_dir());

        // Clean up
        let _ = std::fs::remove_dir_all(cwd);
    }

    #[test]
    fn should_overwrite_existing_file() {
        let cwd = setup_test_dir("should_overwrite");
        let tool = WriteTool::new(&cwd);

        let file_path = cwd.join("overwrite.txt");
        std::fs::write(&file_path, "original").unwrap();

        let input = WriteToolInput {
            path: "overwrite.txt".to_string(),
            content: "replaced".to_string(),
        };

        let output = tool.execute(&input).unwrap();
        assert_eq!(output.bytes_written, 8);

        let written = std::fs::read_to_string(&file_path).unwrap();
        assert_eq!(written, "replaced");

        // Clean up
        let _ = std::fs::remove_dir_all(cwd);
    }

    #[test]
    fn should_write_empty_content() {
        let cwd = setup_test_dir("should_write_empty");
        let tool = WriteTool::new(&cwd);

        let input = WriteToolInput {
            path: "empty.txt".to_string(),
            content: String::new(),
        };

        let output = tool.execute(&input).unwrap();
        assert_eq!(output.bytes_written, 0);

        let written = std::fs::read_to_string(cwd.join("empty.txt")).unwrap();
        assert_eq!(written, "");

        // Clean up
        let _ = std::fs::remove_dir_all(cwd);
    }

    #[test]
    fn should_resolve_relative_paths() {
        let cwd = setup_test_dir("should_resolve_relative");
        let tool = WriteTool::new(&cwd);

        let input = WriteToolInput {
            path: "./subdir/file.txt".to_string(),
            content: "relative".to_string(),
        };

        let output = tool.execute(&input).unwrap();
        assert_eq!(output.bytes_written, 8);

        let written = std::fs::read_to_string(cwd.join("subdir/file.txt")).unwrap();
        assert_eq!(written, "relative");

        // Clean up
        let _ = std::fs::remove_dir_all(cwd);
    }

    #[test]
    fn should_error_on_permission_denied() {
        let cwd = setup_test_dir("should_error_perm");
        let tool = WriteTool::new(&cwd);

        // Create a read-only directory
        let ro_dir = cwd.join("readonly");
        std::fs::create_dir(&ro_dir).unwrap();

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(&ro_dir).unwrap().permissions();
            perms.set_mode(0o444); // r--r--r--
            std::fs::set_permissions(&ro_dir, perms).unwrap();

            let input = WriteToolInput {
                path: "readonly/file.txt".to_string(),
                content: "should fail".to_string(),
            };

            let result = tool.execute(&input);
            assert!(result.is_err());
            match result.unwrap_err() {
                WriteToolError::Io { .. } => {} // expected
                other => panic!("expected Io error, got: {other}"),
            }

            // Restore permissions so we can clean up
            let mut perms = std::fs::metadata(&ro_dir).unwrap().permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(&ro_dir, perms).unwrap();
        }

        // Clean up
        let _ = std::fs::remove_dir_all(cwd);
    }

    #[test]
    fn should_error_on_path_outside_cwd_when_strict() {
        let cwd = setup_test_dir("should_error_outside_cwd");
        let guard = PathGuard::strict(&cwd);
        let tool = WriteTool::with_guard(&cwd, guard);

        // Try to write to an absolute path outside cwd
        let outside = std::env::temp_dir().join("outside-write.txt");
        let input = WriteToolInput {
            path: outside.to_string_lossy().to_string(),
            content: "outside".to_string(),
        };

        let result = tool.execute(&input);
        assert!(result.is_err());
        match result.unwrap_err() {
            WriteToolError::PathGuard(msg) => {
                assert!(msg.contains("outside") || msg.contains("sandbox"));
            }
            other => panic!("expected PathGuard error, got: {other}"),
        }

        // Clean up
        let _ = std::fs::remove_dir_all(cwd);
    }

    #[test]
    fn should_handle_utf8_content() {
        let cwd = setup_test_dir("should_handle_utf8");
        let tool = WriteTool::new(&cwd);

        let input = WriteToolInput {
            path: "utf8.txt".to_string(),
            content: "café résumé 🦀".to_string(),
        };

        let output = tool.execute(&input).unwrap();
        // "café résumé 🦀" — é is 2 bytes each, 🦀 is 4 bytes
        // c(1)a(1)f(1)é(2)' '(1)r(1)é(2)s(1)u(1)m(1)é(2)' '(1)🦀(4) = 19 bytes
        assert_eq!(output.bytes_written, 19);

        let written = std::fs::read_to_string(cwd.join("utf8.txt")).unwrap();
        assert_eq!(written, "café résumé 🦀");

        // Clean up
        let _ = std::fs::remove_dir_all(cwd);
    }
}
