//! Tests for `hamr-agent::core::tools::path_utils`.
//! Ported faithfully from `packages/coding-agent/test/path-utils.test.ts`.

use hamr_agent::core::tools::path_utils::{expand_path, resolve_read_path, resolve_to_cwd};
use std::fs;
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

fn tmp() -> PathBuf {
    std::env::temp_dir()
}

fn make_temp_dir(prefix: &str) -> PathBuf {
    let id = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = tmp().join(format!("{}-{}", prefix, id));
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn clean_dir(dir: &Path) {
    let _ = fs::remove_dir_all(dir);
}

fn write_file(dir: &Path, name: &str, content: &str) {
    fs::write(dir.join(name), content).unwrap();
}

// ---------------------------------------------------------------------------
// expand_path tests
// ---------------------------------------------------------------------------

#[test]
fn should_expand_tilde_to_home_directory() {
    let result = expand_path("~");
    assert!(!result.contains('~'));
}

#[test]
fn should_expand_tilde_slash_to_home_directory() {
    let result = expand_path("~/Documents/file.txt");
    assert!(!result.contains("~/"));
}

#[test]
fn should_keep_tilde_prefixed_filenames_literal() {
    assert_eq!(expand_path("~draft.md"), "~draft.md");
    assert_eq!(expand_path("@~draft.md"), "~draft.md");
}

#[test]
fn should_normalize_unicode_spaces() {
    let with_nbsp = "file\u{00A0}name.txt";
    let result = expand_path(with_nbsp);
    assert_eq!(result, "file name.txt");
}

// ---------------------------------------------------------------------------
// resolve_to_cwd tests
// ---------------------------------------------------------------------------

#[test]
fn should_resolve_absolute_paths_as_is() {
    let cwd = tmp().join("some").join("cwd");
    let absolute = tmp().join("absolute").join("path").join("file.txt");
    let result = resolve_to_cwd(absolute.to_str().unwrap(), &cwd);
    assert_eq!(result, absolute);
}

#[test]
fn should_resolve_relative_paths_against_cwd() {
    let cwd = Path::new("/some/cwd");
    let result = resolve_to_cwd("relative/file.txt", cwd);
    assert!(result.to_str().unwrap().contains("relative"));
    assert!(result.to_str().unwrap().contains("file.txt"));
}

#[test]
fn should_resolve_tilde_prefixed_filenames_against_cwd() {
    let cwd = tmp().join("pi-path-utils-cwd");
    let r1 = resolve_to_cwd("~draft.md", &cwd);
    assert_eq!(r1, cwd.join("~draft.md"));
    let r2 = resolve_to_cwd("@~draft.md", &cwd);
    assert_eq!(r2, cwd.join("~draft.md"));
}

// ---------------------------------------------------------------------------
// resolve_read_path tests
// ---------------------------------------------------------------------------

#[test]
fn should_resolve_existing_file_path() {
    let temp_dir = make_temp_dir("path-utils-test-");
    let file_name = "test-file.txt";
    write_file(&temp_dir, file_name, "content");

    let result = resolve_read_path(file_name, &temp_dir);
    assert_eq!(result, temp_dir.join(file_name));

    clean_dir(&temp_dir);
}

#[test]
fn should_handle_nfc_vs_nfd_unicode_normalization() {
    let temp_dir = make_temp_dir("path-utils-nfc-test-");
    // NFD: e (U+0065) + combining acute accent (U+0301)
    let nfd_file_name = "file\u{0065}\u{0301}.txt";
    // NFC: é as single character (U+00E9)
    let nfc_file_name = "file\u{00E9}.txt";

    assert_ne!(nfd_file_name, nfc_file_name);

    // Create file with NFD name
    write_file(&temp_dir, nfd_file_name, "content");

    // User provides NFC path - should find the file
    let result = resolve_read_path(nfc_file_name, &temp_dir);
    assert!(
        result
            .to_str()
            .unwrap()
            .contains(temp_dir.to_str().unwrap())
    );
    assert!(result.to_str().unwrap().contains(".txt"));

    clean_dir(&temp_dir);
}

#[test]
fn should_handle_curly_vs_straight_quotes() {
    let temp_dir = make_temp_dir("path-utils-quote-test-");
    let curly_quote_name = "Capture d\u{2019}cran.txt";
    let straight_quote_name = "Capture d'cran.txt";

    assert_ne!(curly_quote_name, straight_quote_name);
    write_file(&temp_dir, curly_quote_name, "content");

    let result = resolve_read_path(straight_quote_name, &temp_dir);
    assert_eq!(result, temp_dir.join(curly_quote_name));

    clean_dir(&temp_dir);
}

#[test]
fn should_handle_macos_screenshot_ampm_variant() {
    let temp_dir = make_temp_dir("path-utils-screenshot-test-");
    let macos_name = "Screenshot 2024-01-01 at 10.00.00\u{202F}AM.png";
    let user_name = "Screenshot 2024-01-01 at 10.00.00 AM.png";

    write_file(&temp_dir, macos_name, "content");

    let result = resolve_read_path(user_name, &temp_dir);
    assert_eq!(result, temp_dir.join(macos_name));

    clean_dir(&temp_dir);
}

#[test]
fn should_handle_macos_screenshot_lowercase_ampm_variant() {
    let temp_dir = make_temp_dir("path-utils-screenshot-lc-test-");
    let macos_name = "Screenshot 2024-01-01 at 10.00.00\u{202F}am.png";
    let user_name = "Screenshot 2024-01-01 at 10.00.00 am.png";

    write_file(&temp_dir, macos_name, "content");

    let result = resolve_read_path(user_name, &temp_dir);
    assert_eq!(result, temp_dir.join(macos_name));

    clean_dir(&temp_dir);
}

#[test]
fn should_handle_combined_nfc_curly_quote() {
    let temp_dir = make_temp_dir("path-utils-combined-test-");
    let nfc_curly_name = "Capture d\u{2019}\u{00E9}cran.txt";
    let nfc_straight_name = "Capture d'\u{00E9}cran.txt";

    assert_ne!(nfc_curly_name, nfc_straight_name);
    write_file(&temp_dir, nfc_curly_name, "content");

    let result = resolve_read_path(nfc_straight_name, &temp_dir);
    assert_eq!(result, temp_dir.join(nfc_curly_name));

    clean_dir(&temp_dir);
}
