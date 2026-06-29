//! Port of `packages/coding-agent/src/core/tools/path-utils.ts`.
//!
//! Path resolution helpers for the file tools, including macOS-specific
//! filename variant fallbacks (NFD, curly quotes, screenshot AM/PM spacing).
//!
//! These accept any `AsRef<Path>` input and return `PathBuf`, sitting one layer
//! above the string-based [`crate::utils::paths`] helpers.

use std::path::{Path, PathBuf};

use crate::utils::paths::{PathInputOptions, normalize_path, resolve_path};
use unicode_normalization::UnicodeNormalization;

const NARROW_NO_BREAK_SPACE: char = '\u{202F}';

fn read_opts() -> PathInputOptions {
    PathInputOptions {
        normalize_unicode_spaces: true,
        strip_at_prefix: true,
        ..Default::default()
    }
}

/// macOS screenshot names use a narrow no-break space before AM/PM. Replace a
/// regular " AM."/" PM." (any case) with the narrow-space variant.
fn try_macos_screenshot_path(file_path: &str) -> String {
    // Mirror the TS regex / (AM|PM)\./gi by scanning for " am."/" pm." case-insensitively.
    let chars: Vec<char> = file_path.chars().collect();
    let mut out = String::with_capacity(file_path.len());
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == ' '
            && i + 3 < chars.len()
            && (chars[i + 1] == 'A'
                || chars[i + 1] == 'a'
                || chars[i + 1] == 'P'
                || chars[i + 1] == 'p')
            && (chars[i + 2] == 'M' || chars[i + 2] == 'm')
            && chars[i + 3] == '.'
        {
            out.push(NARROW_NO_BREAK_SPACE);
            out.push(chars[i + 1]);
            out.push(chars[i + 2]);
            out.push('.');
            i += 4;
        } else {
            out.push(chars[i]);
            i += 1;
        }
    }
    out
}

/// macOS stores filenames in NFD (decomposed) form; convert user input to NFD.
fn try_nfd_variant(file_path: &str) -> String {
    file_path.nfd().collect()
}

/// macOS uses U+2019 (right single quotation mark) in screenshot names like
/// "Capture d'écran"; users typically type U+0027 (straight apostrophe).
fn try_curly_quote_variant(file_path: &str) -> String {
    file_path.replace('\'', "\u{2019}")
}

fn file_exists(file_path: &str) -> bool {
    std::fs::symlink_metadata(file_path).is_ok()
}

pub async fn path_exists(file_path: impl AsRef<Path>) -> bool {
    tokio::fs::symlink_metadata(file_path.as_ref())
        .await
        .is_ok()
}

/// Normalize a path for display/storage (unicode spaces + `@` prefix stripped).
/// Returns a `String` (mirrors the TS `expandPath`, which returns a string).
pub fn expand_path(file_path: impl AsRef<Path>) -> String {
    normalize_path(&file_path.as_ref().to_string_lossy(), &read_opts())
}

/// Resolve a path relative to the given cwd (handles `~` expansion and absolute
/// paths).
pub fn resolve_to_cwd(file_path: impl AsRef<Path>, cwd: impl AsRef<Path>) -> PathBuf {
    PathBuf::from(resolve_path(
        &file_path.as_ref().to_string_lossy(),
        Some(&cwd.as_ref().to_string_lossy()),
        &read_opts(),
    ))
}

/// Resolve a read path, trying macOS filename variants if the exact path is
/// missing. Synchronous variant.
pub fn resolve_read_path(file_path: impl AsRef<Path>, cwd: impl AsRef<Path>) -> PathBuf {
    let resolved = resolve_to_cwd(file_path, cwd);
    let resolved_str = resolved.to_string_lossy().into_owned();

    if file_exists(&resolved_str) {
        return resolved;
    }

    for variant in read_path_variants(&resolved_str) {
        if variant != resolved_str && file_exists(&variant) {
            return PathBuf::from(variant);
        }
    }

    resolved
}

/// Async variant of [`resolve_read_path`].
pub async fn resolve_read_path_async(
    file_path: impl AsRef<Path>,
    cwd: impl AsRef<Path>,
) -> PathBuf {
    let resolved = resolve_to_cwd(file_path, cwd);
    let resolved_str = resolved.to_string_lossy().into_owned();

    if path_exists(&resolved_str).await {
        return resolved;
    }

    for variant in read_path_variants(&resolved_str) {
        if variant != resolved_str && path_exists(&variant).await {
            return PathBuf::from(variant);
        }
    }

    resolved
}

/// Build the ordered list of macOS filename variants to try, matching the TS
/// fallback order: AM/PM, NFD, curly quote, NFD+curly.
fn read_path_variants(resolved: &str) -> Vec<String> {
    let nfd = try_nfd_variant(resolved);
    vec![
        try_macos_screenshot_path(resolved),
        nfd.clone(),
        try_curly_quote_variant(resolved),
        try_curly_quote_variant(&nfd),
    ]
}
