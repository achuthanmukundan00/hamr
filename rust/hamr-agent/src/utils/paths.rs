//! Port of `packages/coding-agent/src/utils/paths.ts`.
//!
//! Path normalization and resolution helpers shared across tools. Resolution
//! mirrors Node's `path.resolve`/`path.isAbsolute` semantics (POSIX) and is
//! purely lexical — it never touches the filesystem except `canonicalize_path`.

use std::path::PathBuf;

/// Unicode space variants that get normalized to a regular ASCII space.
/// Matches the TS regex `/[  -   　]/g`.
fn is_unicode_space(c: char) -> bool {
    matches!(
        c,
        '\u{00A0}' | '\u{2000}'..='\u{200A}' | '\u{202F}' | '\u{205F}' | '\u{3000}'
    )
}

/// Options controlling how a raw path string is normalized.
#[derive(Debug, Clone, Default)]
pub struct PathInputOptions {
    /// Trim leading/trailing whitespace before normalization.
    pub trim: bool,
    /// Expand leading `~` to a home directory. Defaults to true (see [`expand_tilde`]).
    pub expand_tilde: Option<bool>,
    /// Home directory used for `~` expansion. Defaults to `$HOME`.
    pub home_dir: Option<String>,
    /// Strip a leading `@`, used for CLI `@file` paths.
    pub strip_at_prefix: bool,
    /// Normalize unicode space variants to regular spaces.
    pub normalize_unicode_spaces: bool,
}

impl PathInputOptions {
    fn expand_tilde(&self) -> bool {
        self.expand_tilde.unwrap_or(true)
    }
}

fn home_dir() -> String {
    std::env::var("HOME").unwrap_or_default()
}

fn current_dir() -> String {
    std::env::current_dir()
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_else(|_| "/".to_string())
}

/// POSIX `path.isAbsolute`.
fn is_absolute(path: &str) -> bool {
    path.starts_with('/')
}

/// Lexically normalize an absolute path, resolving `.` and `..` segments
/// without touching the filesystem (matches `path.resolve` cleanup).
fn normalize_absolute(path: &str) -> String {
    let mut stack: Vec<&str> = Vec::new();
    for comp in path.split('/') {
        match comp {
            "" | "." => {}
            ".." => {
                stack.pop();
            }
            other => stack.push(other),
        }
    }
    let mut result = String::from("/");
    result.push_str(&stack.join("/"));
    result
}

/// Mirror of Node's `path.resolve(...segments)`: process segments right-to-left
/// until an absolute one is found, prepending the cwd if none is.
fn node_resolve(segments: &[&str]) -> String {
    let mut resolved = String::new();
    let mut is_abs = false;
    for seg in segments.iter().rev() {
        if seg.is_empty() {
            continue;
        }
        resolved = if resolved.is_empty() {
            seg.to_string()
        } else {
            format!("{seg}/{resolved}")
        };
        if is_absolute(seg) {
            is_abs = true;
            break;
        }
    }
    if !is_abs {
        let cwd = current_dir();
        resolved = if resolved.is_empty() {
            cwd
        } else {
            format!("{cwd}/{resolved}")
        };
    }
    normalize_absolute(&resolved)
}

/// Resolve a path to its canonical (real) form, following symlinks.
/// Falls back to the raw path if resolution fails (e.g. the target does not
/// exist yet), so callers never crash on missing filesystem entries.
pub fn canonicalize_path(path: &str) -> String {
    std::fs::canonicalize(path)
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_else(|_| path.to_string())
}

/// Returns true if the value is NOT a package source (npm:, git:, etc.) or a
/// remote URL protocol. Bare names, relative paths, and `file:` URLs are local.
pub fn is_local_path(value: &str) -> bool {
    let trimmed = value.trim();
    if trimmed.starts_with("npm:")
        || trimmed.starts_with("git:")
        || trimmed.starts_with("github:")
        || trimmed.starts_with("http:")
        || trimmed.starts_with("https:")
        || trimmed.starts_with("ssh:")
    {
        return false;
    }
    true
}

/// Convert a `file://` URL to a filesystem path. Minimal port of Node's
/// `fileURLToPath` covering the POSIX cases that reach this code.
fn file_url_to_path(url: &str) -> String {
    // Strip scheme and optional `localhost`/empty authority: file:///a → /a
    let rest = url.strip_prefix("file://").unwrap_or(url);
    let path = rest.strip_prefix("localhost").unwrap_or(rest);
    // After the authority there must be a leading slash.
    let path = if path.starts_with('/') { path } else { rest };
    percent_decode(path)
}

fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            let hi = (bytes[i + 1] as char).to_digit(16);
            let lo = (bytes[i + 2] as char).to_digit(16);
            if let (Some(h), Some(l)) = (hi, lo) {
                out.push((h * 16 + l) as u8);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Normalize a raw path string per the given options (no filesystem access
/// except `$HOME` lookup).
pub fn normalize_path(input: &str, options: &PathInputOptions) -> String {
    let mut normalized = if options.trim {
        input.trim().to_string()
    } else {
        input.to_string()
    };

    if options.normalize_unicode_spaces {
        normalized = normalized
            .chars()
            .map(|c| if is_unicode_space(c) { ' ' } else { c })
            .collect();
    }

    if options.strip_at_prefix && normalized.starts_with('@') {
        normalized = normalized[1..].to_string();
    }

    if options.expand_tilde() {
        let home = options.home_dir.clone().unwrap_or_else(home_dir);
        if normalized == "~" {
            return home;
        }
        if let Some(rest) = normalized.strip_prefix("~/") {
            let mut p = PathBuf::from(&home);
            p.push(rest);
            return p.to_string_lossy().into_owned();
        }
    }

    if normalized.starts_with("file://") {
        return file_url_to_path(&normalized);
    }

    normalized
}

/// Resolve `input` against `base_dir` (defaulting to cwd), applying the given
/// normalization options to `input`.
pub fn resolve_path(input: &str, base_dir: Option<&str>, options: &PathInputOptions) -> String {
    let normalized = normalize_path(input, options);
    let base = base_dir.map(|b| b.to_string()).unwrap_or_else(current_dir);
    let normalized_base = normalize_path(&base, &PathInputOptions::default());
    if is_absolute(&normalized) {
        node_resolve(&[&normalized])
    } else {
        node_resolve(&[&normalized_base, &normalized])
    }
}

/// Return the path relative to `cwd` if it lives inside it, else `None`.
pub fn get_cwd_relative_path(file_path: &str, cwd: &str) -> Option<String> {
    let resolved_cwd = resolve_path(cwd, None, &PathInputOptions::default());
    let resolved_path = resolve_path(file_path, Some(&resolved_cwd), &PathInputOptions::default());

    let cwd_path = std::path::Path::new(&resolved_cwd);
    let target = std::path::Path::new(&resolved_path);
    let relative = pathdiff_relative(target, cwd_path)?;
    let relative_str = relative.to_string_lossy().into_owned();

    let inside = relative_str.is_empty()
        || (relative_str != ".."
            && !relative_str.starts_with("../")
            && !is_absolute(&relative_str));

    if inside {
        Some(if relative_str.is_empty() {
            ".".to_string()
        } else {
            relative_str
        })
    } else {
        None
    }
}

/// Lexical `path.relative(from, to)` for already-normalized absolute paths.
fn pathdiff_relative(to: &std::path::Path, from: &std::path::Path) -> Option<PathBuf> {
    let from_comps: Vec<_> = from.components().collect();
    let to_comps: Vec<_> = to.components().collect();

    let mut i = 0;
    while i < from_comps.len() && i < to_comps.len() && from_comps[i] == to_comps[i] {
        i += 1;
    }

    let mut result = PathBuf::new();
    for _ in i..from_comps.len() {
        result.push("..");
    }
    for comp in &to_comps[i..] {
        result.push(comp.as_os_str());
    }
    Some(result)
}

/// Format a path relative to cwd when inside it, otherwise absolute; always
/// using forward slashes.
pub fn format_path_relative_to_cwd_or_absolute(file_path: &str, cwd: &str) -> String {
    let absolute_path = resolve_path(file_path, Some(cwd), &PathInputOptions::default());
    let formatted =
        get_cwd_relative_path(&absolute_path, cwd).unwrap_or_else(|| absolute_path.clone());
    formatted.replace(std::path::MAIN_SEPARATOR, "/")
}

/// Mark a path so it is excluded from common cloud-sync providers (Dropbox,
/// macOS File Provider). Best-effort; failures are ignored.
pub fn mark_path_ignored_by_cloud_sync(path: &str) {
    let attrs: &[&str] = if cfg!(target_os = "macos") {
        &["com.dropbox.ignored", "com.apple.fileprovider.ignore#P"]
    } else if cfg!(target_os = "linux") {
        &["user.com.dropbox.ignored"]
    } else {
        &[]
    };

    for attr in attrs {
        let _ = if cfg!(target_os = "macos") {
            std::process::Command::new("xattr")
                .args(["-w", attr, "1", path])
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status()
        } else {
            std::process::Command::new("setfattr")
                .args(["-n", attr, "-v", "1", path])
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status()
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // is_local_path
    // -----------------------------------------------------------------------

    #[test]
    fn test_is_local_path_bare_name() {
        assert!(is_local_path("my-package"));
    }

    #[test]
    fn test_is_local_path_relative() {
        assert!(is_local_path("./foo"));
    }

    #[test]
    fn test_is_local_path_file_url() {
        assert!(is_local_path("file:///tmp/foo"));
    }

    #[test]
    fn test_is_not_local_path_npm() {
        assert!(!is_local_path("npm:package"));
    }

    #[test]
    fn test_is_not_local_path_git() {
        assert!(!is_local_path("git://repo"));
    }

    #[test]
    fn test_is_not_local_path_https() {
        assert!(!is_local_path("https://example.com"));
    }

    #[test]
    fn test_is_not_local_path_ssh() {
        assert!(!is_local_path("ssh://git@host/repo"));
    }

    #[test]
    fn test_is_not_local_path_github_prefix() {
        assert!(!is_local_path("github:user/repo"));
    }

    #[test]
    fn test_is_not_local_path_http() {
        assert!(!is_local_path("http://example.com"));
    }

    // -----------------------------------------------------------------------
    // normalize_path
    // -----------------------------------------------------------------------

    #[test]
    fn test_normalize_path_expands_home_tilde() {
        let result = normalize_path("~", &PathInputOptions::default());
        let home = home_dir();
        assert_eq!(result, home);
    }

    #[test]
    fn test_normalize_path_expands_tilde_slash() {
        let result = normalize_path("~/Documents/file.txt", &PathInputOptions::default());
        let home = home_dir();
        assert!(result.starts_with(&home));
        assert!(result.ends_with("Documents/file.txt"));
    }

    #[test]
    fn test_normalize_path_keeps_tilde_prefixed_literal() {
        let result = normalize_path("~draft.md", &PathInputOptions::default());
        assert_eq!(result, "~draft.md");
    }

    #[test]
    fn test_normalize_path_trims_whitespace() {
        let opts = PathInputOptions {
            trim: true,
            ..Default::default()
        };
        let result = normalize_path("  /tmp/file  ", &opts);
        assert_eq!(result, "/tmp/file");
    }

    #[test]
    fn test_normalize_path_strips_at_prefix() {
        let opts = PathInputOptions {
            strip_at_prefix: true,
            ..Default::default()
        };
        let result = normalize_path("@file.txt", &opts);
        assert_eq!(result, "file.txt");
    }

    #[test]
    fn test_normalize_path_file_url() {
        let result = normalize_path("file:///tmp/file.txt", &PathInputOptions::default());
        assert_eq!(result, "/tmp/file.txt");
    }

    #[test]
    fn test_normalize_path_unicode_spaces() {
        let opts = PathInputOptions {
            normalize_unicode_spaces: true,
            ..Default::default()
        };
        let result = normalize_path("file\u{00A0}name.txt", &opts);
        assert_eq!(result, "file name.txt");
    }

    #[test]
    fn test_normalize_path_expand_tilde_disabled() {
        let opts = PathInputOptions {
            expand_tilde: Some(false),
            ..Default::default()
        };
        let result = normalize_path("~/path", &opts);
        assert_eq!(result, "~/path");
    }

    // -----------------------------------------------------------------------
    // resolve_path
    // -----------------------------------------------------------------------

    #[test]
    fn test_resolve_path_absolute() {
        let result = resolve_path("/tmp/foo.txt", None, &PathInputOptions::default());
        assert_eq!(result, "/tmp/foo.txt");
    }

    #[test]
    fn test_resolve_path_relative() {
        let result = resolve_path(
            "subdir/file.txt",
            Some("/base"),
            &PathInputOptions::default(),
        );
        assert_eq!(result, "/base/subdir/file.txt");
    }

    #[test]
    fn test_resolve_path_dot_dot() {
        let result = resolve_path(
            "../other/file.txt",
            Some("/base/sub"),
            &PathInputOptions::default(),
        );
        assert_eq!(result, "/base/other/file.txt");
    }

    #[test]
    fn test_resolve_path_file_url() {
        let result = resolve_path(
            "file:///tmp/file.txt",
            Some("/base"),
            &PathInputOptions::default(),
        );
        assert_eq!(result, "/tmp/file.txt");
    }

    #[test]
    fn test_resolve_path_empty_segment() {
        let result = resolve_path("", Some("/base"), &PathInputOptions::default());
        assert_eq!(result, "/base");
    }

    // -----------------------------------------------------------------------
    // canonicalize_path (functionality test, not filesystem-dependent)
    // -----------------------------------------------------------------------

    #[test]
    fn test_canonicalize_path_nonexistent_falls_back() {
        let path = "/tmp/hamr-nonexistent-path-XXXXXXXX";
        let result = canonicalize_path(path);
        assert_eq!(result, path);
    }

    // -----------------------------------------------------------------------
    // get_cwd_relative_path
    // -----------------------------------------------------------------------

    #[test]
    fn test_get_cwd_relative_path_inside_cwd() {
        let result = get_cwd_relative_path("/base/sub/file.txt", "/base").unwrap();
        assert_eq!(result, "sub/file.txt");
    }

    #[test]
    fn test_get_cwd_relative_path_outside_cwd() {
        let result = get_cwd_relative_path("/other/file.txt", "/base");
        assert!(result.is_none());
    }

    #[test]
    fn test_get_cwd_relative_path_equal_to_cwd() {
        let result = get_cwd_relative_path("/base", "/base").unwrap();
        assert_eq!(result, ".");
    }

    // -----------------------------------------------------------------------
    // format_path_relative_to_cwd_or_absolute
    // -----------------------------------------------------------------------

    #[test]
    fn test_format_path_relative_inside_cwd() {
        let result = format_path_relative_to_cwd_or_absolute("/base/sub/file.txt", "/base");
        assert_eq!(result, "sub/file.txt");
    }

    #[test]
    fn test_format_path_absolute_outside_cwd() {
        let result = format_path_relative_to_cwd_or_absolute("/other/file.txt", "/base");
        assert_eq!(result, "/other/file.txt");
    }

    // -----------------------------------------------------------------------
    // is_absolute
    // -----------------------------------------------------------------------

    #[test]
    fn test_is_absolute_true() {
        assert!(is_absolute("/"));
        assert!(is_absolute("/tmp"));
    }

    #[test]
    fn test_is_absolute_false() {
        assert!(!is_absolute(""));
        assert!(!is_absolute("relative"));
        assert!(!is_absolute("./foo"));
    }

    // -----------------------------------------------------------------------
    // percent_decode
    // -----------------------------------------------------------------------

    #[test]
    fn test_percent_decode_basic() {
        assert_eq!(percent_decode("hello%20world"), "hello world");
    }

    #[test]
    fn test_percent_decode_no_encode() {
        assert_eq!(percent_decode("plain"), "plain");
    }

    #[test]
    fn test_percent_decode_invalid_escape() {
        assert_eq!(percent_decode("%ZZ"), "%ZZ");
    }

    // -----------------------------------------------------------------------
    // file_url_to_path
    // -----------------------------------------------------------------------

    #[test]
    fn test_file_url_to_path_basic() {
        assert_eq!(file_url_to_path("file:///tmp/file.txt"), "/tmp/file.txt");
    }

    // -----------------------------------------------------------------------
    // normalize_absolute (lexical)
    // -----------------------------------------------------------------------

    #[test]
    fn test_normalize_absolute_dot_dot() {
        assert_eq!(normalize_absolute("/a/b/../c"), "/a/c");
    }

    #[test]
    fn test_normalize_absolute_trailing_slash_becomes_no_trailing() {
        let result = normalize_absolute("/a/b/");
        assert_eq!(result, "/a/b");
    }

    #[test]
    fn test_normalize_absolute_double_slash() {
        let result = normalize_absolute("//a/b");
        assert_eq!(result, "/a/b");
    }

    // -----------------------------------------------------------------------
    // node_resolve
    // -----------------------------------------------------------------------

    #[test]
    fn test_node_resolve_absolute_wins() {
        let result = node_resolve(&["/abs/path", "rel/path"]);
        assert_eq!(result, "/abs/path/rel/path");
    }
}
