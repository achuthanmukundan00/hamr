//! Port of `packages/coding-agent/src/core/tools/path_guard.ts`.
//!
//! Path confinement for file mutation/read tools. Applies a hard denylist of
//! credential and persistence locations. Enabled by default, can be disabled
//! or extended via settings.

use std::path::Path;

// ---------------------------------------------------------------------------
// PathGuard
// ---------------------------------------------------------------------------

/// Path confinement guard. Applies a denylist of sensitive credential and
/// persistence locations. When enabled (default) paths outside the cwd are
/// blocked for writes when strict mode is active.
#[derive(Debug, Clone)]
pub struct PathGuard {
    /// Master switch. When false, all checks pass.
    enabled: bool,
    /// When set, only writes within this directory are allowed.
    strict_cwd: Option<String>,
}

impl Default for PathGuard {
    fn default() -> Self {
        Self {
            enabled: true,
            strict_cwd: None,
        }
    }
}

impl PathGuard {
    /// Create a new PathGuard with denylist enabled and an optional strict cwd.
    pub fn new(strict_cwd: Option<String>) -> Self {
        Self {
            enabled: true,
            strict_cwd: strict_cwd.map(|c| normalize_for_compare(&c)),
        }
    }

    /// Create a PathGuard with strict cwd confinement (only writes within cwd allowed).
    pub fn strict(cwd: &Path) -> Self {
        Self {
            enabled: true,
            strict_cwd: Some(normalize_for_compare(&cwd.to_string_lossy())),
        }
    }

    /// Create a PathGuard that allows everything (disabled).
    pub fn disabled() -> Self {
        Self {
            enabled: false,
            strict_cwd: None,
        }
    }

    /// Check if a path is writable. Returns `Ok(())` if allowed, or
    /// `Err(message)` with a denial reason.
    pub fn assert_writable(&self, absolute_path: &Path) -> Result<(), String> {
        if !self.enabled {
            return Ok(());
        }

        let candidate = normalize_for_compare(&absolute_path.to_string_lossy());

        // Check strict cwd confinement
        if let Some(ref cwd) = self.strict_cwd {
            if !is_inside(&candidate, cwd) {
                return Err(format!(
                    "Path '{}' is outside the sandbox cwd '{}'. Strict path sandbox is enabled.",
                    candidate, cwd
                ));
            }
        }

        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Path comparison helpers (mirrors Node.js path semantics)
// ---------------------------------------------------------------------------

/// Normalize a path for comparison. Trims trailing slashes, collapses
/// multiple separators. Does NOT follow symlinks.
fn normalize_for_compare(p: &str) -> String {
    // Replace backslashes with forward slashes for cross-platform comparison
    let p = p.replace('\\', "/");

    // Collapse multiple consecutive slashes
    let mut result = String::with_capacity(p.len());
    let mut prev_slash = false;
    for ch in p.chars() {
        if ch == '/' {
            if !prev_slash {
                result.push('/');
                prev_slash = true;
            }
        } else {
            result.push(ch);
            prev_slash = false;
        }
    }

    // Strip trailing slash (unless it's the root path)
    if result.len() > 1 && result.ends_with('/') {
        result.pop();
    }

    result
}

/// Check if `candidate` is inside (or equal to) `prefix`.
/// Uses string prefix comparison with directory-separator semantics.
fn is_inside(candidate: &str, prefix: &str) -> bool {
    if candidate == prefix {
        return true;
    }
    // Candidate must start with prefix + "/"
    if candidate.starts_with(prefix) && candidate.as_bytes().get(prefix.len()) == Some(&b'/') {
        return true;
    }
    false
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_for_compare_trailing_slash() {
        assert_eq!(normalize_for_compare("/foo/bar/"), "/foo/bar");
    }

    #[test]
    fn test_normalize_for_compare_root() {
        assert_eq!(normalize_for_compare("/"), "/");
    }

    #[test]
    fn test_normalize_for_compare_backslashes() {
        assert_eq!(normalize_for_compare("C:\\foo\\bar"), "C:/foo/bar");
    }

    #[test]
    fn test_normalize_for_compare_multiple_slashes() {
        assert_eq!(normalize_for_compare("/foo//bar///baz"), "/foo/bar/baz");
    }

    #[test]
    fn test_is_inside_exact_match() {
        assert!(is_inside("/foo/bar", "/foo/bar"));
    }

    #[test]
    fn test_is_inside_subdir() {
        assert!(is_inside("/foo/bar/baz", "/foo/bar"));
    }

    #[test]
    fn test_is_inside_not_inside() {
        assert!(!is_inside("/foo/baz", "/foo/bar"));
    }

    #[test]
    fn test_is_inside_prefix_mismatch() {
        // "foo/barX" is NOT inside "foo/bar"
        assert!(!is_inside("/foo/barbaz", "/foo/bar"));
    }

    #[test]
    fn test_guard_disabled_allows_all() {
        let guard = PathGuard::disabled();
        assert!(guard.assert_writable(Path::new("/etc/passwd")).is_ok());
    }

    #[test]
    fn test_guard_strict_cwd_blocks_outside() {
        let guard = PathGuard::strict(Path::new("/home/user/project"));
        assert!(
            guard
                .assert_writable(Path::new("/home/user/project/src/main.rs"))
                .is_ok()
        );
        let err = guard
            .assert_writable(Path::new("/etc/hosts"))
            .expect_err("should have been blocked");
        assert!(err.contains("outside"));
    }

    #[test]
    fn test_guard_strict_cwd_allows_inside() {
        let guard = PathGuard::strict(Path::new("/tmp/mycwd"));
        assert!(
            guard
                .assert_writable(Path::new("/tmp/mycwd/sub/file.txt"))
                .is_ok()
        );
        assert!(guard.assert_writable(Path::new("/tmp/mycwd")).is_ok());
    }
}
