//! Edit tool — applies multiple non-overlapping string replacements to a file
//! with fuzzy matching, CRLF preservation, and BOM handling.
//!
//! Port of `packages/coding-agent/src/core/tools/edit.ts`.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use super::edit_diff::{
    Edit, apply_edits_to_normalized_content, detect_line_ending, generate_diff_string,
    generate_unified_patch, normalize_to_lf, restore_line_endings, strip_bom,
};
use super::path_utils::resolve_to_cwd;

// ---------------------------------------------------------------------------
// Pluggable operations (mirrors EditOperations)
// ---------------------------------------------------------------------------

/// Pluggable file I/O for the edit tool.
/// Override these to delegate file editing to remote systems (e.g. SSH).
pub trait EditOperations: Send + Sync {
    /// Read file contents as bytes.
    fn read_file(&self, absolute_path: &Path) -> Result<Vec<u8>, std::io::Error>;
    /// Write UTF-8 content to a file.
    fn write_file(&self, absolute_path: &Path, content: &str) -> Result<(), std::io::Error>;
    /// Check if file is readable and writable (return Err if not).
    fn access(&self, absolute_path: &Path) -> Result<(), std::io::Error>;
}

/// Default edit operations backed by the local filesystem.
pub struct LocalEditOperations;

impl EditOperations for LocalEditOperations {
    fn read_file(&self, absolute_path: &Path) -> Result<Vec<u8>, std::io::Error> {
        std::fs::read(absolute_path)
    }

    fn write_file(&self, absolute_path: &Path, content: &str) -> Result<(), std::io::Error> {
        std::fs::write(absolute_path, content)
    }

    fn access(&self, absolute_path: &Path) -> Result<(), std::io::Error> {
        // Check R_OK | W_OK
        let _ = std::fs::metadata(absolute_path)?;
        let metadata = std::fs::metadata(absolute_path)?;
        let _readonly = metadata.permissions().readonly();

        // On Unix, check write permission
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = metadata.permissions().mode();
            // Check if user can read and write
            let uid = unsafe { libc::getuid() };
            let file_uid = {
                use std::os::unix::fs::MetadataExt;
                metadata.uid()
            };
            let file_gid = {
                use std::os::unix::fs::MetadataExt;
                metadata.gid()
            };

            let has_perm = if file_uid == uid {
                mode & 0o600 == 0o600
            } else {
                // check group
                let gid = unsafe { libc::getgid() };
                let groups = getgroups();
                if file_gid == gid || groups.contains(&file_gid) {
                    mode & 0o060 == 0o060
                } else {
                    mode & 0o006 == 0o006
                }
            };

            if !has_perm {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::PermissionDenied,
                    "EACCES",
                ));
            }
            return Ok(());
        }

        // Non-Unix: just check readonly
        #[cfg(not(unix))]
        {
            if readonly {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::PermissionDenied,
                    "EACCES",
                ));
            }
            Ok(())
        }
    }
}

#[cfg(unix)]
fn getgroups() -> Vec<u32> {
    let mut groups = vec![0u32; 32];
    let n = unsafe { libc::getgroups(32, groups.as_mut_ptr()) };
    if n < 0 {
        return Vec::new();
    }
    groups.truncate(n as usize);
    groups
}

// ---------------------------------------------------------------------------
// EditToolInput
// ---------------------------------------------------------------------------

/// Input schema for the edit tool.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct EditToolInput {
    /// Path to the file to edit (relative or absolute).
    pub path: String,
    /// One or more targeted replacements.
    pub edits: Vec<Edit>,
    /// If true, perform all validation but do not write to disk.
    #[serde(default)]
    pub dry_run: bool,
}

// ---------------------------------------------------------------------------
// EditToolDetails
// ---------------------------------------------------------------------------

/// Details about the edit operation (returned to callers).
#[derive(Debug, Clone, serde::Serialize)]
pub struct EditToolDetails {
    /// Display-oriented diff of the changes made.
    pub diff: String,
    /// Standard unified patch of the changes made.
    pub patch: String,
    /// Line number of the first change in the new file (for editor navigation).
    pub first_changed_line: Option<usize>,
}

// ---------------------------------------------------------------------------
// EditTool
// ---------------------------------------------------------------------------

/// The edit tool.
pub struct EditTool {
    cwd: PathBuf,
    operations: Arc<dyn EditOperations>,
}

impl EditTool {
    /// Create a new edit tool with the given working directory and default filesystem operations.
    pub fn new(cwd: &Path) -> Self {
        Self {
            cwd: cwd.to_path_buf(),
            operations: Arc::new(LocalEditOperations),
        }
    }

    /// Create a new edit tool with custom file operations (e.g., for testing or SSH).
    pub fn with_operations(cwd: &Path, operations: Arc<dyn EditOperations>) -> Self {
        Self {
            cwd: cwd.to_path_buf(),
            operations,
        }
    }

    /// Execute the edit tool synchronously.
    pub fn execute(
        &self,
        input: EditToolInput,
        signal: Option<&std::sync::atomic::AtomicBool>,
    ) -> Result<EditToolDetails, String> {
        let EditToolInput {
            path,
            edits,
            dry_run,
        } = input;

        // Validate edits array
        if edits.is_empty() {
            return Err(
                "Edit tool input is invalid. edits must contain at least one replacement."
                    .to_string(),
            );
        }

        // Check for aborted signal
        let check_aborted = || -> Result<(), String> {
            if let Some(signal) = signal {
                if signal.load(std::sync::atomic::Ordering::Relaxed) {
                    return Err("Operation aborted".to_string());
                }
            }
            Ok(())
        };
        check_aborted()?;

        // Resolve path
        let absolute_path = resolve_to_cwd(&path, &self.cwd);

        // Check access
        self.operations.access(&absolute_path).map_err(|e| {
            let code = error_code(&e);
            if !code.is_empty() {
                format!("Could not edit file: {path}. Error code: {code}.")
            } else {
                format!("Could not edit file: {path}. Error: {e}.")
            }
        })?;
        check_aborted()?;

        // Read file bytes
        let buffer = self
            .operations
            .read_file(&absolute_path)
            .map_err(|e| format!("Could not read file: {path}. Error: {e}."))?;
        check_aborted()?;

        // Decode UTF-8
        let raw_content = String::from_utf8(buffer)
            .map_err(|e| format!("File {path} is not valid UTF-8: {e}"))?;

        // Strip BOM, detect line ending, normalize to LF
        let (bom, content) = strip_bom(&raw_content);
        let original_ending = detect_line_ending(&content);
        let normalized_content = normalize_to_lf(&content);

        // Apply edits
        let applied = apply_edits_to_normalized_content(&normalized_content, &edits, &path)?;
        check_aborted()?;

        // Restore BOM and line endings
        let final_content = bom + &restore_line_endings(&applied.new_content, original_ending);

        // Write to disk (unless dry run)
        if !dry_run {
            self.operations
                .write_file(&absolute_path, &final_content)
                .map_err(|e| {
                    let code = error_code(&e);
                    if !code.is_empty() {
                        format!("Could not write file: {path}. Error code: {code}.")
                    } else {
                        format!("Could not write file: {path}. Error: {e}.")
                    }
                })?;
            check_aborted()?;
        }

        // Generate diff
        let diff_result = generate_diff_string(&applied.base_content, &applied.new_content, 4);
        let patch = generate_unified_patch(&path, &applied.base_content, &applied.new_content, 4);

        Ok(EditToolDetails {
            diff: diff_result.diff,
            patch,
            first_changed_line: diff_result.first_changed_line,
        })
    }
}

/// Extract the OS error code name from a std::io::Error (e.g. "ENOENT", "EACCES").
fn error_code(e: &std::io::Error) -> &'static str {
    use std::io::ErrorKind;
    match e.kind() {
        ErrorKind::NotFound => "ENOENT",
        ErrorKind::PermissionDenied => "EACCES",
        _ => "",
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::atomic::AtomicBool;

    fn setup_test_dir() -> tempfile::TempDir {
        tempfile::tempdir().expect("create temp dir")
    }

    fn write_file(dir: &tempfile::TempDir, name: &str, content: &str) -> PathBuf {
        let path = dir.path().join(name);
        fs::write(&path, content).expect("write test file");
        path
    }

    #[test]
    fn test_single_edit_replaces_text() {
        let dir = setup_test_dir();
        let path = write_file(&dir, "test.txt", "Hello, world!");
        let relative = path.file_name().unwrap().to_str().unwrap();

        let tool = EditTool::new(dir.path());
        let result = tool
            .execute(
                EditToolInput {
                    path: relative.to_string(),
                    edits: vec![Edit {
                        old_text: "world".to_string(),
                        new_text: "testing".to_string(),
                    }],
                    dry_run: false,
                },
                None,
            )
            .expect("edit should succeed");

        assert!(result.diff.contains("testing"));
        assert!(result.patch.contains("--- "));
        assert!(result.patch.contains("+++ "));
        assert!(result.patch.contains("@@"));
        assert!(result.patch.contains("-Hello, world!"));
        assert!(result.patch.contains("+Hello, testing!"));

        let content = fs::read_to_string(&path).expect("read back");
        assert_eq!(content, "Hello, testing!");
    }

    #[test]
    fn test_fails_if_text_not_found() {
        let dir = setup_test_dir();
        let path = write_file(&dir, "test.txt", "Hello, world!");
        let relative = path.file_name().unwrap().to_str().unwrap();

        let tool = EditTool::new(dir.path());
        let err = tool
            .execute(
                EditToolInput {
                    path: relative.to_string(),
                    edits: vec![Edit {
                        old_text: "nonexistent".to_string(),
                        new_text: "testing".to_string(),
                    }],
                    dry_run: false,
                },
                None,
            )
            .unwrap_err();

        assert!(err.contains("Could not find the exact text"));
    }

    #[test]
    fn test_fails_with_enoent_for_missing_file() {
        let dir = setup_test_dir();

        let tool = EditTool::new(dir.path());
        let err = tool
            .execute(
                EditToolInput {
                    path: "missing.txt".to_string(),
                    edits: vec![Edit {
                        old_text: "hello".to_string(),
                        new_text: "world".to_string(),
                    }],
                    dry_run: false,
                },
                None,
            )
            .unwrap_err();

        assert!(err.contains("Could not edit file: missing.txt"));
        assert!(err.contains("ENOENT"));
    }

    #[test]
    fn test_fails_if_text_appears_multiple_times() {
        let dir = setup_test_dir();
        let path = write_file(&dir, "test.txt", "foo foo foo");
        let relative = path.file_name().unwrap().to_str().unwrap();

        let tool = EditTool::new(dir.path());
        let err = tool
            .execute(
                EditToolInput {
                    path: relative.to_string(),
                    edits: vec![Edit {
                        old_text: "foo".to_string(),
                        new_text: "bar".to_string(),
                    }],
                    dry_run: false,
                },
                None,
            )
            .unwrap_err();

        assert!(err.contains("Found 3 occurrences"));
    }

    #[test]
    fn test_multiple_disjoint_regions() {
        let dir = setup_test_dir();
        let path = write_file(&dir, "test.txt", "alpha\nbeta\ngamma\ndelta\n");
        let relative = path.file_name().unwrap().to_str().unwrap();

        let tool = EditTool::new(dir.path());
        let _result = tool
            .execute(
                EditToolInput {
                    path: relative.to_string(),
                    edits: vec![
                        Edit {
                            old_text: "alpha\n".to_string(),
                            new_text: "ALPHA\n".to_string(),
                        },
                        Edit {
                            old_text: "gamma\n".to_string(),
                            new_text: "GAMMA\n".to_string(),
                        },
                    ],
                    dry_run: false,
                },
                None,
            )
            .expect("edit should succeed");

        let content = fs::read_to_string(&path).expect("read back");
        assert_eq!(content, "ALPHA\nbeta\nGAMMA\ndelta\n");
    }

    #[test]
    fn test_edits_matched_against_original_not_incremental() {
        let dir = setup_test_dir();
        let path = write_file(&dir, "test.txt", "foo\nbar\nbaz\n");
        let relative = path.file_name().unwrap().to_str().unwrap();

        let tool = EditTool::new(dir.path());
        tool.execute(
            EditToolInput {
                path: relative.to_string(),
                edits: vec![
                    Edit {
                        old_text: "foo\n".to_string(),
                        new_text: "foo bar\n".to_string(),
                    },
                    Edit {
                        old_text: "bar\n".to_string(),
                        new_text: "BAR\n".to_string(),
                    },
                ],
                dry_run: false,
            },
            None,
        )
        .expect("edit should succeed");

        let content = fs::read_to_string(&path).expect("read back");
        assert_eq!(content, "foo bar\nBAR\nbaz\n");
    }

    #[test]
    fn test_fails_when_edits_is_empty() {
        let dir = setup_test_dir();
        write_file(&dir, "test.txt", "hello\nworld\n");

        let tool = EditTool::new(dir.path());
        let err = tool
            .execute(
                EditToolInput {
                    path: "test.txt".to_string(),
                    edits: vec![],
                    dry_run: false,
                },
                None,
            )
            .unwrap_err();

        assert!(err.contains("edits must contain at least one replacement"));
    }

    #[test]
    fn test_fails_when_edits_overlap() {
        let dir = setup_test_dir();
        let path = write_file(&dir, "test.txt", "one\ntwo\nthree\n");
        let relative = path.file_name().unwrap().to_str().unwrap();

        let tool = EditTool::new(dir.path());
        let err = tool
            .execute(
                EditToolInput {
                    path: relative.to_string(),
                    edits: vec![
                        Edit {
                            old_text: "one\ntwo\n".to_string(),
                            new_text: "ONE\nTWO\n".to_string(),
                        },
                        Edit {
                            old_text: "two\nthree\n".to_string(),
                            new_text: "TWO\nTHREE\n".to_string(),
                        },
                    ],
                    dry_run: false,
                },
                None,
            )
            .unwrap_err();

        assert!(err.contains("overlap"));
    }

    #[test]
    fn test_does_not_partially_apply_on_failure() {
        let dir = setup_test_dir();
        let path = write_file(&dir, "test.txt", "alpha\nbeta\ngamma\n");
        let relative = path.file_name().unwrap().to_str().unwrap();

        let tool = EditTool::new(dir.path());
        let err = tool
            .execute(
                EditToolInput {
                    path: relative.to_string(),
                    edits: vec![
                        Edit {
                            old_text: "alpha\n".to_string(),
                            new_text: "ALPHA\n".to_string(),
                        },
                        Edit {
                            old_text: "missing\n".to_string(),
                            new_text: "MISSING\n".to_string(),
                        },
                    ],
                    dry_run: false,
                },
                None,
            )
            .unwrap_err();

        assert!(err.contains("Could not find"));

        // File should be unchanged
        let content = fs::read_to_string(&path).expect("read back");
        assert_eq!(content, "alpha\nbeta\ngamma\n");
    }

    #[test]
    fn test_readonly_file_eacces() {
        let dir = setup_test_dir();
        let path = write_file(&dir, "test.txt", "hello\n");
        let relative = path.file_name().unwrap().to_str().unwrap();

        // Make read-only
        let mut perms = fs::metadata(&path).unwrap().permissions();
        perms.set_readonly(true);
        fs::set_permissions(&path, perms).unwrap();

        let tool = EditTool::new(dir.path());
        let err = tool
            .execute(
                EditToolInput {
                    path: relative.to_string(),
                    edits: vec![Edit {
                        old_text: "hello".to_string(),
                        new_text: "world".to_string(),
                    }],
                    dry_run: false,
                },
                None,
            )
            .unwrap_err();

        assert!(err.contains("EACCES"));
    }

    #[test]
    fn test_abort_signal() {
        let dir = setup_test_dir();
        write_file(&dir, "test.txt", "hello\n");

        let tool = EditTool::new(dir.path());
        let signal = AtomicBool::new(true); // already aborted

        let err = tool
            .execute(
                EditToolInput {
                    path: "test.txt".to_string(),
                    edits: vec![Edit {
                        old_text: "hello".to_string(),
                        new_text: "world".to_string(),
                    }],
                    dry_run: false,
                },
                Some(&signal),
            )
            .unwrap_err();

        assert!(err.contains("aborted"));
    }

    #[test]
    fn test_dry_run_does_not_write() {
        let dir = setup_test_dir();
        let path = write_file(&dir, "test.txt", "Hello, world!");
        let relative = path.file_name().unwrap().to_str().unwrap();

        let tool = EditTool::new(dir.path());
        let result = tool
            .execute(
                EditToolInput {
                    path: relative.to_string(),
                    edits: vec![Edit {
                        old_text: "world".to_string(),
                        new_text: "universe".to_string(),
                    }],
                    dry_run: true,
                },
                None,
            )
            .expect("dry run should succeed");

        assert!(result.diff.contains("universe"));

        // File should be unchanged
        let content = fs::read_to_string(&path).expect("read back");
        assert_eq!(content, "Hello, world!");
    }

    // -- fuzzy matching tests --

    #[test]
    fn test_fuzzy_trailing_whitespace() {
        let dir = setup_test_dir();
        let path = write_file(&dir, "test.txt", "line one   \nline two  \nline three\n");
        let relative = path.file_name().unwrap().to_str().unwrap();

        let tool = EditTool::new(dir.path());
        tool.execute(
            EditToolInput {
                path: relative.to_string(),
                edits: vec![Edit {
                    old_text: "line one\nline two\n".to_string(),
                    new_text: "replaced\n".to_string(),
                }],
                dry_run: false,
            },
            None,
        )
        .expect("fuzzy match should succeed");

        let content = fs::read_to_string(&path).expect("read back");
        assert_eq!(content, "replaced\nline three\n");
    }

    #[test]
    fn test_fuzzy_smart_quotes() {
        let dir = setup_test_dir();
        let path = write_file(&dir, "test.txt", "console.log(\u{2018}hello\u{2019});\n");
        let relative = path.file_name().unwrap().to_str().unwrap();

        let tool = EditTool::new(dir.path());
        tool.execute(
            EditToolInput {
                path: relative.to_string(),
                edits: vec![Edit {
                    old_text: "console.log('hello');".to_string(),
                    new_text: "console.log('world');".to_string(),
                }],
                dry_run: false,
            },
            None,
        )
        .expect("smart quote match should succeed");

        let content = fs::read_to_string(&path).expect("read back");
        assert!(content.contains("world"));
    }

    #[test]
    fn test_fuzzy_smart_double_quotes() {
        let dir = setup_test_dir();
        let path = write_file(
            &dir,
            "test.txt",
            "const msg = \u{201C}Hello World\u{201D};\n",
        );
        let relative = path.file_name().unwrap().to_str().unwrap();

        let tool = EditTool::new(dir.path());
        tool.execute(
            EditToolInput {
                path: relative.to_string(),
                edits: vec![Edit {
                    old_text: "const msg = \"Hello World\";".to_string(),
                    new_text: "const msg = \"Goodbye\";".to_string(),
                }],
                dry_run: false,
            },
            None,
        )
        .expect("smart double quote match should succeed");

        let content = fs::read_to_string(&path).expect("read back");
        assert!(content.contains("Goodbye"));
    }

    #[test]
    fn test_fuzzy_unicode_dashes() {
        let dir = setup_test_dir();
        let path = write_file(&dir, "test.txt", "range: 1\u{2013}5\nbreak\u{2014}here\n");
        let relative = path.file_name().unwrap().to_str().unwrap();

        let tool = EditTool::new(dir.path());
        tool.execute(
            EditToolInput {
                path: relative.to_string(),
                edits: vec![Edit {
                    old_text: "range: 1-5\nbreak-here".to_string(),
                    new_text: "range: 10-50\nbreak--here".to_string(),
                }],
                dry_run: false,
            },
            None,
        )
        .expect("dash match should succeed");

        let content = fs::read_to_string(&path).expect("read back");
        assert!(content.contains("10-50"));
    }

    #[test]
    fn test_fuzzy_nbsp() {
        let dir = setup_test_dir();
        let path = write_file(&dir, "test.txt", "hello\u{00A0}world\n");
        let relative = path.file_name().unwrap().to_str().unwrap();

        let tool = EditTool::new(dir.path());
        tool.execute(
            EditToolInput {
                path: relative.to_string(),
                edits: vec![Edit {
                    old_text: "hello world".to_string(),
                    new_text: "hello universe".to_string(),
                }],
                dry_run: false,
            },
            None,
        )
        .expect("nbsp match should succeed");

        let content = fs::read_to_string(&path).expect("read back");
        assert!(content.contains("universe"));
    }

    #[test]
    fn test_prefer_exact_over_fuzzy() {
        let dir = setup_test_dir();
        let path = write_file(&dir, "test.txt", "const x = 'exact';\nconst y = 'other';\n");
        let relative = path.file_name().unwrap().to_str().unwrap();

        let tool = EditTool::new(dir.path());
        tool.execute(
            EditToolInput {
                path: relative.to_string(),
                edits: vec![Edit {
                    old_text: "const x = 'exact';".to_string(),
                    new_text: "const x = 'changed';".to_string(),
                }],
                dry_run: false,
            },
            None,
        )
        .expect("exact match should be preferred");

        let content = fs::read_to_string(&path).expect("read back");
        assert_eq!(content, "const x = 'changed';\nconst y = 'other';\n");
    }

    #[test]
    fn test_fails_when_not_found_even_with_fuzzy() {
        let dir = setup_test_dir();
        let path = write_file(&dir, "test.txt", "completely different content\n");
        let relative = path.file_name().unwrap().to_str().unwrap();

        let tool = EditTool::new(dir.path());
        let err = tool
            .execute(
                EditToolInput {
                    path: relative.to_string(),
                    edits: vec![Edit {
                        old_text: "this does not exist".to_string(),
                        new_text: "replacement".to_string(),
                    }],
                    dry_run: false,
                },
                None,
            )
            .unwrap_err();

        assert!(err.contains("Could not find"));
    }

    // -- CRLF tests --

    #[test]
    fn test_crlf_preserved_after_edit() {
        let dir = setup_test_dir();
        let path = write_file(&dir, "test.txt", "first\r\nsecond\r\nthird\r\n");
        let relative = path.file_name().unwrap().to_str().unwrap();

        let tool = EditTool::new(dir.path());
        tool.execute(
            EditToolInput {
                path: relative.to_string(),
                edits: vec![Edit {
                    old_text: "second\n".to_string(),
                    new_text: "REPLACED\n".to_string(),
                }],
                dry_run: false,
            },
            None,
        )
        .expect("CRLF edit should succeed");

        let content = fs::read_to_string(&path).expect("read back");
        assert_eq!(content, "first\r\nREPLACED\r\nthird\r\n");
    }

    #[test]
    fn test_lf_preserved_after_edit() {
        let dir = setup_test_dir();
        let path = write_file(&dir, "test.txt", "first\nsecond\nthird\n");
        let relative = path.file_name().unwrap().to_str().unwrap();

        let tool = EditTool::new(dir.path());
        tool.execute(
            EditToolInput {
                path: relative.to_string(),
                edits: vec![Edit {
                    old_text: "second\n".to_string(),
                    new_text: "REPLACED\n".to_string(),
                }],
                dry_run: false,
            },
            None,
        )
        .expect("LF edit should succeed");

        let content = fs::read_to_string(&path).expect("read back");
        assert_eq!(content, "first\nREPLACED\nthird\n");
    }

    #[test]
    fn test_crlf_fuzzy_duplicates_detected() {
        let dir = setup_test_dir();
        let path = write_file(&dir, "test.txt", "hello\r\nworld\r\n---\r\nhello\nworld\n");
        let relative = path.file_name().unwrap().to_str().unwrap();

        let tool = EditTool::new(dir.path());
        let err = tool
            .execute(
                EditToolInput {
                    path: relative.to_string(),
                    edits: vec![Edit {
                        old_text: "hello\nworld\n".to_string(),
                        new_text: "replaced\n".to_string(),
                    }],
                    dry_run: false,
                },
                None,
            )
            .unwrap_err();

        assert!(err.contains("Found 2 occurrences"));
    }

    #[test]
    fn test_bom_preserved_after_edit() {
        let dir = setup_test_dir();
        let path = write_file(&dir, "test.txt", "\u{FEFF}first\r\nsecond\r\nthird\r\n");
        let relative = path.file_name().unwrap().to_str().unwrap();

        let tool = EditTool::new(dir.path());
        tool.execute(
            EditToolInput {
                path: relative.to_string(),
                edits: vec![Edit {
                    old_text: "second\n".to_string(),
                    new_text: "REPLACED\n".to_string(),
                }],
                dry_run: false,
            },
            None,
        )
        .expect("BOM edit should succeed");

        let content = fs::read_to_string(&path).expect("read back");
        assert_eq!(content, "\u{FEFF}first\r\nREPLACED\r\nthird\r\n");
    }

    #[test]
    fn test_bom_crlf_multi_edit() {
        let dir = setup_test_dir();
        let path = write_file(
            &dir,
            "test.txt",
            "\u{FEFF}first\r\nsecond\r\nthird\r\nfourth\r\n",
        );
        let relative = path.file_name().unwrap().to_str().unwrap();

        let tool = EditTool::new(dir.path());
        tool.execute(
            EditToolInput {
                path: relative.to_string(),
                edits: vec![
                    Edit {
                        old_text: "second\n".to_string(),
                        new_text: "SECOND\n".to_string(),
                    },
                    Edit {
                        old_text: "fourth\n".to_string(),
                        new_text: "FOURTH\n".to_string(),
                    },
                ],
                dry_run: false,
            },
            None,
        )
        .expect("BOM+CRLF multi edit should succeed");

        let content = fs::read_to_string(&path).expect("read back");
        assert_eq!(content, "\u{FEFF}first\r\nSECOND\r\nthird\r\nFOURTH\r\n");
    }

    #[test]
    fn test_collapse_large_gaps_in_multi_edit_diffs() {
        let dir = setup_test_dir();
        let lines: Vec<String> = (1..=600).map(|i| format!("line {:03}", i)).collect();
        let content = lines.join("\n") + "\n";
        let path = write_file(&dir, "test.txt", &content);
        let relative = path.file_name().unwrap().to_str().unwrap();

        let tool = EditTool::new(dir.path());
        let result = tool
            .execute(
                EditToolInput {
                    path: relative.to_string(),
                    edits: vec![
                        Edit {
                            old_text: "line 100\n".to_string(),
                            new_text: "LINE 100\n".to_string(),
                        },
                        Edit {
                            old_text: "line 300\n".to_string(),
                            new_text: "LINE 300\n".to_string(),
                        },
                        Edit {
                            old_text: "line 500\n".to_string(),
                            new_text: "LINE 500\n".to_string(),
                        },
                    ],
                    dry_run: false,
                },
                None,
            )
            .expect("multi-edit should succeed");

        let diff = &result.diff;
        assert!(diff.contains("LINE 100"));
        assert!(diff.contains("LINE 300"));
        assert!(diff.contains("LINE 500"));
        assert!(diff.contains("..."));
        assert!(!diff.contains("line 250"));
        // Should be much shorter than the full 600-line file
        assert!(diff.split('\n').count() < 50);
    }

    #[test]
    fn test_first_changed_line_in_details() {
        let dir = setup_test_dir();
        let path = write_file(&dir, "test.txt", "line 1\nline 2\nline 3\nline 4\n");
        let relative = path.file_name().unwrap().to_str().unwrap();

        let tool = EditTool::new(dir.path());
        let result = tool
            .execute(
                EditToolInput {
                    path: relative.to_string(),
                    edits: vec![Edit {
                        old_text: "line 2\n".to_string(),
                        new_text: "LINE TWO\n".to_string(),
                    }],
                    dry_run: false,
                },
                None,
            )
            .expect("edit should succeed");

        // line 2 is the first changed line (1-indexed)
        assert_eq!(result.first_changed_line, Some(2));
    }

    // -- Custom operations tests --

    #[test]
    fn test_custom_ops_generic_error() {
        struct FailingAccessOps;
        impl EditOperations for FailingAccessOps {
            fn read_file(&self, _: &Path) -> Result<Vec<u8>, std::io::Error> {
                Ok(b"hello\n".to_vec())
            }
            fn write_file(&self, _: &Path, _: &str) -> Result<(), std::io::Error> {
                Ok(())
            }
            fn access(&self, _: &Path) -> Result<(), std::io::Error> {
                Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "disk offline",
                ))
            }
        }

        let dir = setup_test_dir();
        let tool = EditTool::with_operations(dir.path(), Arc::new(FailingAccessOps));

        let err = tool
            .execute(
                EditToolInput {
                    path: "broken.txt".to_string(),
                    edits: vec![Edit {
                        old_text: "hello".to_string(),
                        new_text: "world".to_string(),
                    }],
                    dry_run: false,
                },
                None,
            )
            .unwrap_err();

        assert!(err.contains("Could not edit file: broken.txt"));
        assert!(err.contains("disk offline"));
    }

    #[test]
    #[cfg(unix)]
    fn test_readonly_file_permission_check() {
        use std::os::unix::fs::PermissionsExt;

        let dir = setup_test_dir();
        let path = write_file(&dir, "test.txt", "hello\n");

        // Remove write permission
        let mut perms = fs::metadata(&path).unwrap().permissions();
        perms.set_mode(0o444);
        fs::set_permissions(&path, perms).unwrap();

        let tool = EditTool::new(dir.path());
        let err = tool
            .execute(
                EditToolInput {
                    path: path.file_name().unwrap().to_str().unwrap().to_string(),
                    edits: vec![Edit {
                        old_text: "hello".to_string(),
                        new_text: "world".to_string(),
                    }],
                    dry_run: false,
                },
                None,
            )
            .unwrap_err();

        assert!(err.contains("EACCES"));
    }

    #[test]
    fn test_fuzzy_chinese_fullwidth_punctuation() {
        let dir = setup_test_dir();
        let path = write_file(
            &dir,
            "test.txt",
            "\u{4F60}\u{597D}\u{FF0C}\u{4E16}\u{754C}\n\u{4F60}\u{597D}\u{FF08}\u{4E16}\u{754C}\u{FF09}\n",
        );
        let relative = path.file_name().unwrap().to_str().unwrap();

        let tool = EditTool::new(dir.path());
        tool.execute(
            EditToolInput {
                path: relative.to_string(),
                edits: vec![Edit {
                    old_text:
                        "\u{4F60}\u{597D},\u{4E16}\u{754C}\n\u{4F60}\u{597D}(\u{4E16}\u{754C})\n"
                            .to_string(),
                    new_text: "\u{4F60}\u{597D}\u{FF0C}pi\n\u{4F60}\u{597D}(pi)\n".to_string(),
                }],
                dry_run: false,
            },
            None,
        )
        .expect("chinese punctuation fuzzy match should succeed");

        let content = fs::read_to_string(&path).expect("read back");
        assert!(content.contains("pi"));
    }

    #[test]
    fn test_fuzzy_unicode_compatibility_forms() {
        let dir = setup_test_dir();
        // Fullwidth letters and combining accent
        let path = write_file(
            &dir,
            "test.txt",
            "\u{FF21}\u{FF22}\u{FF23}\u{FF11}\u{FF12}\u{FF13}\ncafe\u{0301}\n",
        );
        let relative = path.file_name().unwrap().to_str().unwrap();

        let tool = EditTool::new(dir.path());
        tool.execute(
            EditToolInput {
                path: relative.to_string(),
                edits: vec![Edit {
                    old_text: "ABC123\ncafé\n".to_string(),
                    new_text: "XYZ789\ncoffee\n".to_string(),
                }],
                dry_run: false,
            },
            None,
        )
        .expect("unicode compatibility fuzzy match should succeed");

        let content = fs::read_to_string(&path).expect("read back");
        assert!(content.contains("XYZ789"));
    }

    #[test]
    fn test_fuzzy_duplicates_after_normalization() {
        let dir = setup_test_dir();
        // Lines that are identical after trailing whitespace is stripped
        let path = write_file(&dir, "test.txt", "hello world   \nhello world\n");
        let relative = path.file_name().unwrap().to_str().unwrap();

        let tool = EditTool::new(dir.path());
        let err = tool
            .execute(
                EditToolInput {
                    path: relative.to_string(),
                    edits: vec![Edit {
                        old_text: "hello world".to_string(),
                        new_text: "replaced".to_string(),
                    }],
                    dry_run: false,
                },
                None,
            )
            .unwrap_err();

        assert!(err.contains("Found 2 occurrences"));
    }
}
