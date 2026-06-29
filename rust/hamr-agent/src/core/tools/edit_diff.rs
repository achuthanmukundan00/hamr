//! Core edit-diff utilities: fuzzy matching, overlap detection, unified diff
//! generation.  Port of `packages/coding-agent/src/core/tools/edit-diff.ts`.

use std::path::Path;

use similar::{ChangeTag, TextDiff};

// ---------------------------------------------------------------------------
// Line ending helpers
// ---------------------------------------------------------------------------

/// Detect the dominant line ending in `content`.  Returns `\r\n` when a CRLF
/// appears before the first bare LF; otherwise `\n`.
pub fn detect_line_ending(content: &str) -> &'static str {
    let crlf_idx = content.find("\r\n");
    let lf_idx = content.find('\n');
    match (lf_idx, crlf_idx) {
        (None, _) => "\n",
        (Some(_), None) => "\n",
        (Some(lf), Some(crlf)) => {
            if crlf < lf {
                "\r\n"
            } else {
                "\n"
            }
        }
    }
}

/// Normalize all line endings to `\n`.
pub fn normalize_to_lf(text: &str) -> String {
    text.replace("\r\n", "\n").replace('\r', "\n")
}

/// Restore `\r\n` endings if that was the original line ending.
pub fn restore_line_endings(text: &str, ending: &str) -> String {
    if ending == "\r\n" {
        text.replace('\n', "\r\n")
    } else {
        text.to_string()
    }
}

// ---------------------------------------------------------------------------
// UTF-8 BOM
// ---------------------------------------------------------------------------

/// Strip a leading UTF-8 BOM (U+FEFF) if present.
/// Returns a tuple of `(bom_string, text_without_bom)`.
pub fn strip_bom(content: &str) -> (String, String) {
    if let Some(rest) = content.strip_prefix('\u{FEFF}') {
        ("\u{FEFF}".to_string(), rest.to_string())
    } else {
        (String::new(), content.to_string())
    }
}

// ---------------------------------------------------------------------------
// Fuzzy matching — Unicode normalization
// ---------------------------------------------------------------------------

/// Normalize text for fuzzy matching.  Progressive transformations:
/// 1. NFKC normalization
/// 2. Strip trailing whitespace from each line
/// 3. Smart single quotes → `'`
/// 4. Smart double quotes → `"`
/// 5. Unicode dashes/hyphens → `-`
/// 6. Special Unicode spaces → ` `
pub fn normalize_for_fuzzy_match(text: &str) -> String {
    let nfkc: String = unicode_normalization::UnicodeNormalization::nfkc(text).collect();

    // Strip trailing whitespace per line
    let lines: Vec<&str> = nfkc.split('\n').collect();
    let trimmed: Vec<String> = lines
        .iter()
        .map(|line| line.trim_end().to_string())
        .collect();
    let joined = trimmed.join("\n");

    // Smart single quotes → '
    let single = regex::Regex::new("[\u{2018}\u{2019}\u{201A}\u{201B}]").unwrap();
    let result = single.replace_all(&joined, "'");

    // Smart double quotes → "
    let double = regex::Regex::new("[\u{201C}\u{201D}\u{201E}\u{201F}]").unwrap();
    let result = double.replace_all(&result, "\"");

    // Dashes → -
    let dashes =
        regex::Regex::new("[\u{2010}\u{2011}\u{2012}\u{2013}\u{2014}\u{2015}\u{2212}]").unwrap();
    let result = dashes.replace_all(&result, "-");

    // Special Unicode spaces → regular space
    let spaces = regex::Regex::new("[\u{00A0}\u{2002}-\u{200A}\u{202F}\u{205F}\u{3000}]").unwrap();
    let result = spaces.replace_all(&result, " ");

    result.into_owned()
}

// ---------------------------------------------------------------------------
// Edit types
// ---------------------------------------------------------------------------

/// A single edit: find `oldText`, replace with `newText`.
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub struct Edit {
    pub old_text: String,
    pub new_text: String,
}

/// Result of fuzzy-finding old_text in content.
#[derive(Debug, Clone)]
pub struct FuzzyMatchResult {
    /// Whether a match was found.
    pub found: bool,
    /// Start index in the content used for replacement.
    pub index: usize,
    /// Byte length of the matched text.
    pub match_length: usize,
    /// Whether fuzzy matching was used (false = exact match).
    pub used_fuzzy_match: bool,
    /// Content to use for replacement operations (original or fuzzy-normalized).
    pub content_for_replacement: String,
}

/// The result of applying edits: original (base) vs. modified (new).
#[derive(Debug, Clone)]
pub struct AppliedEditsResult {
    pub base_content: String,
    pub new_content: String,
}

// ---------------------------------------------------------------------------
// Finding matches
// ---------------------------------------------------------------------------

/// Find `old_text` in `content`, trying exact match first then fuzzy fallback.
pub fn fuzzy_find_text(content: &str, old_text: &str) -> FuzzyMatchResult {
    // 1. Exact match
    if let Some(exact_index) = content.find(old_text) {
        return FuzzyMatchResult {
            found: true,
            index: exact_index,
            match_length: old_text.len(),
            used_fuzzy_match: false,
            content_for_replacement: content.to_string(),
        };
    }

    // 2. Fuzzy match
    let fuzzy_content = normalize_for_fuzzy_match(content);
    let fuzzy_old_text = normalize_for_fuzzy_match(old_text);
    if let Some(fuzzy_index) = fuzzy_content.find(&fuzzy_old_text) {
        return FuzzyMatchResult {
            found: true,
            index: fuzzy_index,
            match_length: fuzzy_old_text.len(),
            used_fuzzy_match: true,
            content_for_replacement: fuzzy_content,
        };
    }

    FuzzyMatchResult {
        found: false,
        index: 0,
        match_length: 0,
        used_fuzzy_match: false,
        content_for_replacement: content.to_string(),
    }
}

/// Count occurrences of `old_text` in `content` using fuzzy normalization.
fn count_occurrences(content: &str, old_text: &str) -> usize {
    let fuzzy_content = normalize_for_fuzzy_match(content);
    let fuzzy_old_text = normalize_for_fuzzy_match(old_text);
    if fuzzy_old_text.is_empty() {
        return 0;
    }
    fuzzy_content.matches(&fuzzy_old_text).count()
}

// ---------------------------------------------------------------------------
// Error helpers
// ---------------------------------------------------------------------------

fn get_not_found_error(path: &str, edit_index: usize, total_edits: usize) -> String {
    if total_edits == 1 {
        format!(
            "Could not find the exact text in {path}. The old text must match exactly including all whitespace and newlines."
        )
    } else {
        format!(
            "Could not find edits[{edit_index}] in {path}. The oldText must match exactly including all whitespace and newlines."
        )
    }
}

fn get_duplicate_error(
    path: &str,
    edit_index: usize,
    total_edits: usize,
    occurrences: usize,
) -> String {
    if total_edits == 1 {
        format!(
            "Found {occurrences} occurrences of the text in {path}. The text must be unique. Please provide more context to make it unique."
        )
    } else {
        format!(
            "Found {occurrences} occurrences of edits[{edit_index}] in {path}. Each oldText must be unique. Please provide more context to make it unique."
        )
    }
}

fn get_empty_old_text_error(path: &str, edit_index: usize, total_edits: usize) -> String {
    if total_edits == 1 {
        format!("oldText must not be empty in {path}.")
    } else {
        format!("edits[{edit_index}].oldText must not be empty in {path}.")
    }
}

fn get_no_change_error(path: &str, total_edits: usize) -> String {
    if total_edits == 1 {
        format!(
            "No changes made to {path}. The replacement produced identical content. This might indicate an issue with special characters or the text not existing as expected."
        )
    } else {
        format!("No changes made to {path}. The replacements produced identical content.")
    }
}

// ---------------------------------------------------------------------------
// Core edit application
// ---------------------------------------------------------------------------

/// Apply one or more exact-text replacements to LF-normalized content.
///
/// All edits are matched against the same original content. Replacements are
/// applied in reverse order so offsets remain stable. If any edit needs fuzzy
/// matching, the entire operation runs in fuzzy-normalized content space.
pub fn apply_edits_to_normalized_content(
    normalized_content: &str,
    edits: &[Edit],
    path: &str,
) -> Result<AppliedEditsResult, String> {
    // LF-normalize old texts and new texts
    let normalized_edits: Vec<Edit> = edits
        .iter()
        .map(|e| Edit {
            old_text: normalize_to_lf(&e.old_text),
            new_text: normalize_to_lf(&e.new_text),
        })
        .collect();

    // 1. Reject empty old texts
    for (i, edit) in normalized_edits.iter().enumerate() {
        if edit.old_text.is_empty() {
            return Err(get_empty_old_text_error(path, i, normalized_edits.len()));
        }
    }

    // 2. Initial match pass to determine whether we need to switch to fuzzy space
    let initial_matches: Vec<FuzzyMatchResult> = normalized_edits
        .iter()
        .map(|edit| fuzzy_find_text(normalized_content, &edit.old_text))
        .collect();

    let base_content = if initial_matches.iter().any(|m| m.used_fuzzy_match) {
        normalize_for_fuzzy_match(normalized_content)
    } else {
        normalized_content.to_string()
    };

    // 3. Match and uniqueness check
    struct MatchedEdit {
        edit_index: usize,
        match_index: usize,
        match_length: usize,
        new_text: String,
    }

    let mut matched_edits: Vec<MatchedEdit> = Vec::new();
    for (i, edit) in normalized_edits.iter().enumerate() {
        let match_result = fuzzy_find_text(&base_content, &edit.old_text);
        if !match_result.found {
            return Err(get_not_found_error(path, i, normalized_edits.len()));
        }

        let occurrences = count_occurrences(&base_content, &edit.old_text);
        if occurrences > 1 {
            return Err(get_duplicate_error(
                path,
                i,
                normalized_edits.len(),
                occurrences,
            ));
        }

        matched_edits.push(MatchedEdit {
            edit_index: i,
            match_index: match_result.index,
            match_length: match_result.match_length,
            new_text: edit.new_text.clone(),
        });
    }

    // 4. Overlap detection (sort by position first)
    matched_edits.sort_by_key(|m| m.match_index);
    for i in 1..matched_edits.len() {
        let prev = &matched_edits[i - 1];
        let curr = &matched_edits[i];
        if prev.match_index + prev.match_length > curr.match_index {
            return Err(format!(
                "edits[{}] and edits[{}] overlap in {}. Merge them into one edit or target disjoint regions.",
                prev.edit_index, curr.edit_index, path
            ));
        }
    }

    // 5. Apply edits in reverse order (preserves offsets)
    let mut new_content = base_content.clone();
    for edit in matched_edits.iter().rev() {
        let before = &new_content[..edit.match_index];
        let after = &new_content[edit.match_index + edit.match_length..];
        new_content = before.to_string() + &edit.new_text + after;
    }

    if base_content == new_content {
        return Err(get_no_change_error(path, normalized_edits.len()));
    }

    Ok(AppliedEditsResult {
        base_content,
        new_content,
    })
}

// ---------------------------------------------------------------------------
// Diff generation with the `similar` crate
// ---------------------------------------------------------------------------

/// Generate a standard unified patch using the `similar` crate.
pub fn generate_unified_patch(
    file_path: &str,
    old_content: &str,
    new_content: &str,
    context_lines: usize,
) -> String {
    let diff = TextDiff::from_lines(old_content, new_content);
    let mut out = String::new();

    // Write header
    use std::fmt::Write;
    let _ = writeln!(out, "--- {file_path}");
    let _ = writeln!(out, "+++ {file_path}");

    for hunk in diff
        .unified_diff()
        .context_radius(context_lines)
        .iter_hunks()
    {
        let _ = write!(out, "{hunk}");
    }

    out
}

/// Generate a display-oriented diff string with line numbers and context.
///
/// Returns both the diff string and the first changed line number (1-indexed
/// in the new file).
pub fn generate_diff_string(
    old_content: &str,
    new_content: &str,
    context_lines: usize,
) -> DiffStringResult {
    let diff = TextDiff::from_lines(old_content, new_content);
    let changes = diff.iter_all_changes();

    // Count lines in old and new for line-number width
    let old_line_count = old_content.lines().count();
    let new_line_count = new_content.lines().count();
    let max_line_num = old_line_count.max(new_line_count);
    let line_num_width = max_line_num.to_string().len();

    let mut output: Vec<String> = Vec::new();
    let mut old_line: usize = 1;
    let mut new_line: usize = 1;
    let mut first_changed_line: Option<usize> = None;

    // We need to process groups of equal/deletion/insertion changes to replicate
    // the TS behavior which groups contiguous diffs and collapses large context gaps.
    // The TS uses `diffLines` which returns hunks of "equal", "added", "removed".
    // `similar`'s `iter_all_changes()` gives us individual lines with ChangeTag.

    // Collect changes into groups (consecutive with same tag type)
    #[derive(Debug)]
    enum Group<'a> {
        Equal(Vec<&'a str>),
        Changed(Vec<(ChangeTag, &'a str)>),
    }

    let mut groups: Vec<Group> = Vec::new();
    let mut current_equal: Vec<&str> = Vec::new();
    let mut current_changed: Vec<(ChangeTag, &str)> = Vec::new();

    for change in changes {
        let tag = change.tag();
        let value = change.value(); // change.value() is &str for line diffs

        match tag {
            ChangeTag::Equal => {
                if !current_changed.is_empty() {
                    groups.push(Group::Changed(std::mem::take(&mut current_changed)));
                }
                current_equal.push(value);
            }
            ChangeTag::Delete | ChangeTag::Insert => {
                if !current_equal.is_empty() {
                    groups.push(Group::Equal(std::mem::take(&mut current_equal)));
                }
                current_changed.push((tag, value));
            }
        }
    }
    if !current_equal.is_empty() {
        groups.push(Group::Equal(std::mem::take(&mut current_equal)));
    }
    if !current_changed.is_empty() {
        groups.push(Group::Changed(std::mem::take(&mut current_changed)));
    }

    let mut last_was_change = false;

    for (gi, group) in groups.iter().enumerate() {
        match group {
            Group::Changed(parts) => {
                // Capture first changed line
                if first_changed_line.is_none() {
                    first_changed_line = Some(new_line);
                }

                for (tag, line) in parts {
                    let stripped = line.strip_suffix('\n').unwrap_or(line);
                    match tag {
                        ChangeTag::Delete => {
                            let ln = format_ln(old_line, line_num_width);
                            output.push(format!("-{ln} {stripped}"));
                            old_line += 1;
                        }
                        ChangeTag::Insert => {
                            let ln = format_ln(new_line, line_num_width);
                            output.push(format!("+{ln} {stripped}"));
                            new_line += 1;
                        }
                        _ => {}
                    }
                }
                last_was_change = true;
            }
            Group::Equal(lines) => {
                let next_group_is_change =
                    gi + 1 < groups.len() && matches!(groups[gi + 1], Group::Changed(_));
                let has_leading_change = last_was_change;
                let has_trailing_change = next_group_is_change;

                if has_leading_change && has_trailing_change {
                    // Surrounded by changes — show context on both sides
                    if lines.len() <= context_lines * 2 {
                        for line in lines {
                            let stripped = line.strip_suffix('\n').unwrap_or(line);
                            let ln = format_ln(old_line, line_num_width);
                            output.push(format!(" {ln} {stripped}"));
                            old_line += 1;
                            new_line += 1;
                        }
                    } else {
                        let leading = &lines[..context_lines];
                        let trailing = &lines[lines.len() - context_lines..];
                        let skipped = lines.len() - leading.len() - trailing.len();

                        for line in leading {
                            let stripped = line.strip_suffix('\n').unwrap_or(line);
                            let ln = format_ln(old_line, line_num_width);
                            output.push(format!(" {ln} {stripped}"));
                            old_line += 1;
                            new_line += 1;
                        }

                        output.push(format!(" {} ...", " ".repeat(line_num_width)));
                        old_line += skipped;
                        new_line += skipped;

                        for line in trailing {
                            let stripped = line.strip_suffix('\n').unwrap_or(line);
                            let ln = format_ln(old_line, line_num_width);
                            output.push(format!(" {ln} {stripped}"));
                            old_line += 1;
                            new_line += 1;
                        }
                    }
                } else if has_leading_change {
                    let shown = &lines[..context_lines.min(lines.len())];
                    let skipped = lines.len() - shown.len();

                    for line in shown {
                        let stripped = line.strip_suffix('\n').unwrap_or(line);
                        let ln = format_ln(old_line, line_num_width);
                        output.push(format!(" {ln} {stripped}"));
                        old_line += 1;
                        new_line += 1;
                    }

                    if skipped > 0 {
                        output.push(format!(" {} ...", " ".repeat(line_num_width)));
                        old_line += skipped;
                        new_line += skipped;
                    }
                } else if has_trailing_change {
                    let skipped = if lines.len() > context_lines {
                        lines.len() - context_lines
                    } else {
                        0
                    };

                    if skipped > 0 {
                        output.push(format!(" {} ...", " ".repeat(line_num_width)));
                        old_line += skipped;
                        new_line += skipped;
                    }

                    for line in &lines[skipped..] {
                        let stripped = line.strip_suffix('\n').unwrap_or(line);
                        let ln = format_ln(old_line, line_num_width);
                        output.push(format!(" {ln} {stripped}"));
                        old_line += 1;
                        new_line += 1;
                    }
                } else {
                    // Remote context — skip entirely
                    old_line += lines.len();
                    new_line += lines.len();
                }

                last_was_change = false;
            }
        }
    }

    DiffStringResult {
        diff: output.join("\n"),
        first_changed_line,
    }
}

fn format_ln(line_num: usize, width: usize) -> String {
    format!("{:>width$}", line_num)
}

/// Result of `generate_diff_string`.
#[derive(Debug, Clone)]
pub struct DiffStringResult {
    pub diff: String,
    pub first_changed_line: Option<usize>,
}

// ---------------------------------------------------------------------------
// Preview (async)
// ---------------------------------------------------------------------------

/// Preview diff for one or more edits without applying them to disk.
/// Used for TUI preview rendering.
pub async fn compute_edits_diff(
    file_path: &str,
    edits: &[Edit],
    cwd: &Path,
) -> Result<DiffStringResult, String> {
    let absolute_path = crate::core::tools::path_utils::resolve_to_cwd(file_path, cwd);

    // Check if file exists and is readable
    if let Err(e) = tokio::fs::metadata(&absolute_path).await {
        let msg = if e.kind() == std::io::ErrorKind::NotFound {
            format!("Could not edit file: {file_path}. Error code: ENOENT.")
        } else if e.kind() == std::io::ErrorKind::PermissionDenied {
            format!("Could not edit file: {file_path}. Error code: EACCES.")
        } else {
            format!("Could not edit file: {file_path}. Error: {e}.")
        };
        return Err(msg);
    }

    // Read the file
    let raw = match tokio::fs::read_to_string(&absolute_path).await {
        Ok(s) => s,
        Err(e) => {
            let code = e.kind();
            let msg = if code == std::io::ErrorKind::PermissionDenied {
                format!("Could not edit file: {file_path}. Error code: EACCES.")
            } else {
                format!("Could not edit file: {file_path}. Error: {e}.")
            };
            return Err(msg);
        }
    };

    let (_bom, content) = strip_bom(&raw);
    let normalized = normalize_to_lf(&content);
    let result = apply_edits_to_normalized_content(&normalized, edits, file_path)?;

    Ok(generate_diff_string(
        &result.base_content,
        &result.new_content,
        4,
    ))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -- line endings --

    #[test]
    fn test_detect_line_ending_lf() {
        assert_eq!(detect_line_ending("hello\nworld\n"), "\n");
    }

    #[test]
    fn test_detect_line_ending_crlf() {
        assert_eq!(detect_line_ending("hello\r\nworld\r\n"), "\r\n");
    }

    #[test]
    fn test_detect_line_ending_crlf_before_lf() {
        // CRLF appears first, so it's the dominant ending
        assert_eq!(detect_line_ending("a\r\nb\nc\r\n"), "\r\n");
    }

    #[test]
    fn test_normalize_to_lf() {
        assert_eq!(normalize_to_lf("a\r\nb\r\nc"), "a\nb\nc");
        assert_eq!(normalize_to_lf("a\rb"), "a\nb");
    }

    #[test]
    fn test_restore_line_endings() {
        let original = "hello\nworld";
        assert_eq!(restore_line_endings(original, "\r\n"), "hello\r\nworld");
        assert_eq!(restore_line_endings(original, "\n"), "hello\nworld");
    }

    // -- BOM --

    #[test]
    fn test_strip_bom_present() {
        let (bom, text) = strip_bom("\u{FEFF}hello");
        assert_eq!(bom, "\u{FEFF}");
        assert_eq!(text, "hello");
    }

    #[test]
    fn test_strip_bom_absent() {
        let (bom, text) = strip_bom("hello");
        assert_eq!(bom, "");
        assert_eq!(text, "hello");
    }

    // -- fuzzy matching --

    #[test]
    fn test_normalize_trailing_whitespace() {
        let input = "line 1   \nline 2  \n";
        let normalized = normalize_for_fuzzy_match(input);
        assert_eq!(normalized, "line 1\nline 2\n");
    }

    #[test]
    fn test_normalize_smart_quotes() {
        let input = "it\u{2018}s \u{201C}hello\u{201D}";
        let normalized = normalize_for_fuzzy_match(input);
        assert_eq!(normalized, "it's \"hello\"");
    }

    #[test]
    fn test_normalize_dashes() {
        let input = "a\u{2013}b\u{2014}c";
        let normalized = normalize_for_fuzzy_match(input);
        assert_eq!(normalized, "a-b-c");
    }

    #[test]
    fn test_normalize_nbsp() {
        let input = "hello\u{00A0}world";
        let normalized = normalize_for_fuzzy_match(input);
        assert_eq!(normalized, "hello world");
    }

    #[test]
    fn test_normalize_nfkc() {
        // Fullwidth ASCII → normal ASCII
        let input = "\u{FF21}\u{FF22}\u{FF23}"; // ＡＢＣ
        let normalized = normalize_for_fuzzy_match(input);
        assert_eq!(normalized, "ABC");
    }

    #[test]
    fn test_fuzzy_find_exact_preferred() {
        let content = "hello world\nfoo bar\n";
        let result = fuzzy_find_text(content, "hello world");
        assert!(result.found);
        assert!(!result.used_fuzzy_match);
        assert_eq!(result.index, 0);
    }

    #[test]
    fn test_fuzzy_find_trailing_ws() {
        let content = "hello   \nworld\n";
        let result = fuzzy_find_text(content, "hello\nworld");
        assert!(result.found);
        assert!(result.used_fuzzy_match);
    }

    #[test]
    fn test_fuzzy_find_smart_quotes() {
        let content = "it\u{2018}s a test\n";
        let result = fuzzy_find_text(content, "it's a test");
        assert!(result.found);
        assert!(result.used_fuzzy_match);
    }

    #[test]
    fn test_fuzzy_find_not_found() {
        let content = "something else entirely\n";
        let result = fuzzy_find_text(content, "nonexistent");
        assert!(!result.found);
    }

    // -- apply edits --

    #[test]
    fn test_apply_single_edit() {
        let content = "hello world\n";
        let edits = vec![Edit {
            old_text: "world".to_string(),
            new_text: "universe".to_string(),
        }];
        let result = apply_edits_to_normalized_content(content, &edits, "test.txt").unwrap();
        assert_eq!(result.new_content, "hello universe\n");
    }

    #[test]
    fn test_apply_multiple_disjoint_edits() {
        let content = "alpha\nbeta\ngamma\ndelta\n";
        let edits = vec![
            Edit {
                old_text: "alpha\n".to_string(),
                new_text: "ALPHA\n".to_string(),
            },
            Edit {
                old_text: "gamma\n".to_string(),
                new_text: "GAMMA\n".to_string(),
            },
        ];
        let result = apply_edits_to_normalized_content(content, &edits, "test.txt").unwrap();
        assert_eq!(result.new_content, "ALPHA\nbeta\nGAMMA\ndelta\n");
    }

    #[test]
    fn test_apply_edits_not_found() {
        let content = "hello\n";
        let edits = vec![Edit {
            old_text: "goodbye".to_string(),
            new_text: "x".to_string(),
        }];
        let err = apply_edits_to_normalized_content(content, &edits, "test.txt").unwrap_err();
        assert!(err.contains("Could not find"));
    }

    #[test]
    fn test_apply_edits_duplicate() {
        let content = "foo foo foo\n";
        let edits = vec![Edit {
            old_text: "foo".to_string(),
            new_text: "bar".to_string(),
        }];
        let err = apply_edits_to_normalized_content(content, &edits, "test.txt").unwrap_err();
        assert!(err.contains("Found 3 occurrences"));
    }

    #[test]
    fn test_apply_edits_empty_old_text() {
        let content = "hello\n";
        let edits = vec![Edit {
            old_text: "".to_string(),
            new_text: "x".to_string(),
        }];
        let err = apply_edits_to_normalized_content(content, &edits, "test.txt").unwrap_err();
        assert!(err.contains("must not be empty"));
    }

    #[test]
    fn test_apply_edits_overlap() {
        let content = "one\ntwo\nthree\n";
        let edits = vec![
            Edit {
                old_text: "one\ntwo\n".to_string(),
                new_text: "ONE\nTWO\n".to_string(),
            },
            Edit {
                old_text: "two\nthree\n".to_string(),
                new_text: "TWO\nTHREE\n".to_string(),
            },
        ];
        let err = apply_edits_to_normalized_content(content, &edits, "test.txt").unwrap_err();
        assert!(err.contains("overlap"));
    }

    #[test]
    fn test_apply_edits_no_change() {
        let content = "hello\n";
        let edits = vec![Edit {
            old_text: "hello".to_string(),
            new_text: "hello".to_string(),
        }];
        let err = apply_edits_to_normalized_content(content, &edits, "test.txt").unwrap_err();
        assert!(err.contains("No changes made"));
    }

    #[test]
    fn test_apply_edits_matches_original_not_incremental() {
        // TS spec: "Should match edits against the original file, not incrementally"
        let content = "foo\nbar\nbaz\n";
        let edits = vec![
            Edit {
                old_text: "foo\n".to_string(),
                new_text: "foo bar\n".to_string(),
            },
            Edit {
                old_text: "bar\n".to_string(),
                new_text: "BAR\n".to_string(),
            },
        ];
        let result = apply_edits_to_normalized_content(content, &edits, "test.txt").unwrap();
        // If incremental: "foo bar\n" replaces "foo\n", then "bar\n" in result would
        // match inside "foo bar\n" — but it should match original "bar\n" separately.
        assert_eq!(result.new_content, "foo bar\nBAR\nbaz\n");
    }

    #[test]
    fn test_apply_edits_fuzzy_multi() {
        let content = "console.log(\u{2018}hello\u{2019});\nhello\u{00A0}world\n";
        let edits = vec![
            Edit {
                old_text: "console.log('hello');\n".to_string(),
                new_text: "console.log('world');\n".to_string(),
            },
            Edit {
                old_text: "hello world\n".to_string(),
                new_text: "hello universe\n".to_string(),
            },
        ];
        let result = apply_edits_to_normalized_content(content, &edits, "test.txt").unwrap();
        assert_eq!(
            result.new_content,
            "console.log('world');\nhello universe\n"
        );
    }

    #[test]
    fn test_apply_edits_fuzzy_duplicates() {
        let content = "hello world   \nhello world\n";
        let edits = vec![Edit {
            old_text: "hello world".to_string(),
            new_text: "replaced".to_string(),
        }];
        let err = apply_edits_to_normalized_content(content, &edits, "test.txt").unwrap_err();
        assert!(err.contains("Found 2 occurrences"));
    }

    #[test]
    fn test_apply_edits_crlf_matching() {
        // CRLF content, LF old_text — should match after normalization
        let content = normalize_to_lf("line one\r\nline two\r\nline three\r\n");
        let edits = vec![Edit {
            old_text: "line two\n".to_string(),
            new_text: "replaced line\n".to_string(),
        }];
        let result = apply_edits_to_normalized_content(&content, &edits, "test.txt").unwrap();
        assert_eq!(result.new_content, "line one\nreplaced line\nline three\n");
    }

    // -- unified patch & diff string --

    #[test]
    fn test_generate_unified_patch() {
        let patch = generate_unified_patch("test.txt", "hello world\n", "hello universe\n", 4);
        assert!(patch.contains("--- test.txt"));
        assert!(patch.contains("+++ test.txt"));
        assert!(patch.contains("-hello world"));
        assert!(patch.contains("+hello universe"));
    }

    #[test]
    fn test_generate_diff_string() {
        let result = generate_diff_string("alpha\nbeta\ngamma\n", "ALPHA\nbeta\nGAMMA\n", 4);
        assert!(result.diff.contains("ALPHA"));
        assert!(result.diff.contains("GAMMA"));
        // Lines are prefixed with the change sign + padded line number (TS format),
        // e.g. "-1 alpha" / "+1 ALPHA".
        assert!(result.diff.contains("-1 alpha"));
        assert!(result.diff.contains("+1 ALPHA"));
    }

    #[test]
    fn test_generate_diff_string_first_changed_line() {
        let result = generate_diff_string(
            "line 1\nline 2\nline 3\nline 4\n",
            "line 1\nLINE 2\nline 3\nLINE 4\n",
            4,
        );
        assert_eq!(result.first_changed_line, Some(2));
    }

    #[test]
    fn test_generate_diff_string_collapses_large_gaps() {
        let lines: Vec<String> = (1..=600).map(|i| format!("line {:03}", i)).collect();
        let old_content = lines.join("\n") + "\n";

        let mut new_lines = lines.clone();
        new_lines[99] = "LINE 100".to_string();
        new_lines[299] = "LINE 300".to_string();
        new_lines[499] = "LINE 500".to_string();
        let new_content = new_lines.join("\n") + "\n";

        let result = generate_diff_string(&old_content, &new_content, 4);
        assert!(result.diff.contains("LINE 100"));
        assert!(result.diff.contains("LINE 300"));
        assert!(result.diff.contains("LINE 500"));
        assert!(result.diff.contains("..."));
    }

    #[test]
    fn test_normalize_fuzzy_chinese_punctuation() {
        // Chinese comma (fullwidth) should match ASCII comma after NFKC
        let content = "你好，世界\n";
        let result = fuzzy_find_text(content, "你好,世界");
        assert!(result.found);
    }

    #[test]
    fn test_normalize_unicode_compatibility() {
        // Fullwidth letters + combining accent → NFC equivalent
        let content = "\u{FF21}\u{FF22}\u{FF23}123\ncafe\u{0301}\n";
        let result = fuzzy_find_text(content, "ABC123\ncafé\n");
        assert!(result.found);
    }
}
