//! Port of `packages/coding-agent/src/core/prompt-templates.ts`.
//!
//! Prompt template loading, argument substitution, and expansion.
//!
//! A *prompt template* is a Markdown file (`.md`) with optional YAML frontmatter
//! stored in a prompts directory. Templates support bash-style argument
//! placeholders (`$1`, `$2`, `$@`, `${N:-default}`, `${@:N}`, `${@:N:L}`) and
//! can be referenced in user input with `/template_name arg1 arg2`.

use std::path::{Path, PathBuf};

use crate::utils::frontmatter::parse_frontmatter;

use super::source_info::{
    SourceInfo, SourceScope, SyntheticSourceInfoOptions, create_synthetic_source_info,
};

/// A prompt template loaded from a Markdown file.
#[derive(Debug, Clone)]
pub struct PromptTemplate {
    /// File name without `.md` extension.
    pub name: String,
    /// Human-readable description (from frontmatter or first line of body).
    pub description: String,
    /// Hint about expected arguments (from frontmatter `argument-hint`).
    pub argument_hint: Option<String>,
    /// Body content (everything after frontmatter).
    pub content: String,
    /// Origin metadata.
    pub source_info: SourceInfo,
    /// Absolute filesystem path to the `.md` file.
    pub file_path: String,
}

/// Options for [`load_prompt_templates`].
pub struct LoadPromptTemplatesOptions {
    /// Working directory for project-local templates (typically `process::cwd()`).
    pub cwd: String,
    /// Agent config directory for global templates (e.g. `~/.hamr/agent/`).
    pub agent_dir: String,
    /// Explicit prompt template paths (files or directories).
    pub prompt_paths: Vec<String>,
    /// Whether to include the default prompt directories.
    pub include_defaults: bool,
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// The config directory name used for project-local prompts.
/// Mirrors `CONFIG_DIR_NAME` from the TS config module (default: `.hamr`).
const CONFIG_DIR_NAME: &str = ".hamr";

// ---------------------------------------------------------------------------
// Argument parsing (bash-style quoted strings)
// ---------------------------------------------------------------------------

/// Parse command arguments respecting quoted strings (bash-style).
///
/// Supports double and single quotes. Returns separate argument strings.
///
/// ```
/// # use hamr_agent::core::prompt_templates::parse_command_args;
/// let args = parse_command_args(r#"foo "bar baz" 'qux'"#);
/// assert_eq!(args, vec!["foo", "bar baz", "qux"]);
/// ```
pub fn parse_command_args(args_string: &str) -> Vec<String> {
    let mut args: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut in_quote: Option<char> = None;

    for c in args_string.chars() {
        if let Some(q) = in_quote {
            if c == q {
                in_quote = None;
            } else {
                current.push(c);
            }
        } else if c == '"' || c == '\'' {
            in_quote = Some(c);
        } else if c.is_whitespace() {
            if !current.is_empty() {
                args.push(std::mem::take(&mut current));
            }
        } else {
            current.push(c);
        }
    }

    if !current.is_empty() {
        args.push(current);
    }

    args
}

// ---------------------------------------------------------------------------
// Argument substitution ($1, $2, $@, ${N:-default}, ${@:N}, ${@:N:L})
// ---------------------------------------------------------------------------

/// Substitute argument placeholders in template content.
///
/// Supports:
/// - `$1`, `$2`, ... for positional arguments
/// - `$@` and `$ARGUMENTS` for all arguments joined by space
/// - `${N:-default}` for positional arg N with a default when missing/empty
/// - `${@:N}` for all arguments from Nth onwards (bash-style slicing, 1-indexed)
/// - `${@:N:L}` for L arguments starting from Nth (1-indexed)
///
/// Argument and default values containing patterns like `$1`, `$@`, or
/// `$ARGUMENTS` are NOT recursively substituted.
///
/// ```
/// # use hamr_agent::core::prompt_templates::substitute_args;
/// let result = substitute_args("Hello $1, you said $2", &["Alice".into(), "hi".into()]);
/// assert_eq!(result, "Hello Alice, you said hi");
///
/// let result = substitute_args("All: $@", &["a".into(), "b".into()]);
/// assert_eq!(result, "All: a b");
///
/// let result = substitute_args("${1:-default}", &[] as &[String]);
/// assert_eq!(result, "default");
/// ```
pub fn substitute_args(content: &str, args: &[String]) -> String {
    let all_args = args.join(" ");

    // Manual replacement loop matching TS behaviour precisely.
    let mut result = String::new();
    let chars: Vec<char> = content.chars().collect();
    let len = chars.len();
    let mut pos = 0;

    while pos < len {
        // Look for '$' — only start of a placeholder
        if chars[pos] != '$' {
            result.push(chars[pos]);
            pos += 1;
            continue;
        }

        // Peek ahead to determine placeholder type.
        // Build a &str from the remaining characters.
        let remaining: String = chars[pos..].iter().collect();
        let remaining = remaining.as_str();

        // ${...} complex placeholder
        if remaining.starts_with("${") {
            // Find the closing '}'
            if let Some(close_pos) = remaining.find('}') {
                let inner = &remaining[2..close_pos]; // between ${ and }
                if let Some(replacement) = substitute_one_complex(inner, args, &all_args) {
                    result.push_str(&replacement);
                    pos += close_pos + 1; // skip past '}'
                    continue;
                }
                // Pattern not recognized — leave ${...} as literal
                result.push_str(&remaining[..=close_pos]);
                pos += close_pos + 1;
                continue;
            }
        }

        // $ARGUMENTS (longest match first)
        if remaining.starts_with("$ARGUMENTS") {
            result.push_str(&all_args);
            pos += "$ARGUMENTS".len();
            continue;
        }

        // $@
        if remaining.starts_with("$@") {
            result.push_str(&all_args);
            pos += "$@".len();
            continue;
        }

        // $N (single or multi-digit number)
        if remaining.len() > 1 && remaining.as_bytes()[1].is_ascii_digit() {
            let num_end = 1 + remaining[1..]
                .chars()
                .take_while(|c| c.is_ascii_digit())
                .count();
            let num_str = &remaining[1..num_end];
            if let Ok(n) = num_str.parse::<usize>() {
                let value = args
                    .get(n.wrapping_sub(1))
                    .map(|s| s.as_str())
                    .unwrap_or("");
                result.push_str(value);
                pos += num_end;
                continue;
            }
        }

        // Not a placeholder — emit '$' as-is
        result.push('$');
        pos += 1;
    }

    result
}

/// Handle `${...}` inner content after stripping the `${` and `}`.
///
/// Returns `Some(replacement)` when a known pattern is matched and valid,
/// or `None` when the pattern should be left as-is (literal `${...}` in output).
fn substitute_one_complex(inner: &str, args: &[String], _all_args: &str) -> Option<String> {
    // ${N:-default} — N must be one or more digits like the TS regex `\d+`
    if let Some(sep) = inner.find(":-") {
        let num_str = &inner[..sep];
        let default = &inner[sep + 2..];
        if is_all_digits(num_str) {
            if let Ok(n) = num_str.parse::<usize>() {
                let index = n.wrapping_sub(1);
                let value = args.get(index).map(|s| s.as_str()).unwrap_or("");
                if value.is_empty() {
                    return Some(default.to_string());
                }
                return Some(value.to_string());
            }
        }
        // N is not a valid digit sequence — pattern doesn't match (like TS regex)
        return None;
    }

    // ${@:N:L} or ${@:N} — N must be one or more digits (like TS regex `\d+`)
    if let Some(slice_str) = inner.strip_prefix("@:") {
        let parts: Vec<&str> = slice_str.splitn(2, ':').collect();
        let start_str = parts[0];
        let length_str = parts.get(1).copied();

        // N must be non-empty and all digits (mirrors TS regex `\d+`)
        if start_str.is_empty() || !is_all_digits(start_str) {
            return None;
        }

        let mut start: usize = start_str.parse().unwrap_or(1);
        // Treat 0 as 1 (bash convention: args start at 1)
        if start == 0 {
            start = 1;
        }
        let start_index = start.wrapping_sub(1);

        if let Some(len_str) = length_str {
            // If L is present but not all digits, pattern doesn't match (TS regex requires `\d+`)
            if !is_all_digits(len_str) {
                return None;
            }
            if let Ok(len) = len_str.parse::<usize>() {
                if start_index < args.len() {
                    let end = (start_index + len).min(args.len());
                    return Some(args[start_index..end].join(" "));
                }
                return Some(String::new());
            }
        }

        // No length — return from start to end
        if start_index < args.len() {
            return Some(args[start_index..].join(" "));
        }
        return Some(String::new());
    }

    // Unknown pattern — don't substitute (TS regex wouldn't match)
    None
}

/// Check whether a string consists entirely of ASCII digits (at least one).
fn is_all_digits(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| c.is_ascii_digit())
}

// ---------------------------------------------------------------------------
// Loading templates from files and directories
// ---------------------------------------------------------------------------

/// Load a single prompt template from a Markdown file.
fn load_template_from_file(file_path: &Path, source_info: SourceInfo) -> Option<PromptTemplate> {
    let raw_content = std::fs::read_to_string(file_path).ok()?;
    let pf = parse_frontmatter::<serde_json::Value>(&raw_content);

    // Name is file stem (no .md extension)
    let name = file_path
        .file_stem()
        .and_then(|s| s.to_str())
        .map(|s| s.to_string())
        .unwrap_or_default();

    // Get description from frontmatter or first non-empty line
    let description = if let Some(desc) = pf.frontmatter.get("description").and_then(|v| v.as_str())
    {
        if !desc.is_empty() {
            desc.to_string()
        } else {
            first_line_description(&pf.body)
        }
    } else {
        first_line_description(&pf.body)
    };

    // Get argument-hint from frontmatter
    let argument_hint = pf
        .frontmatter
        .get("argument-hint")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string());

    Some(PromptTemplate {
        name,
        description,
        argument_hint,
        content: pf.body,
        source_info,
        file_path: file_path.to_string_lossy().into_owned(),
    })
}

/// Extract description from the first non-empty line of body text.
fn first_line_description(body: &str) -> String {
    if let Some(first_line) = body.lines().find(|line| !line.trim().is_empty()) {
        let truncated: String = first_line.chars().take(60).collect();
        if first_line.len() > 60 {
            format!("{}...", truncated)
        } else {
            truncated
        }
    } else {
        String::new()
    }
}

/// Scan a directory for `.md` files (non-recursive) and load them as templates.
fn load_templates_from_dir(
    dir: &Path,
    get_source_info: &dyn Fn(&Path) -> SourceInfo,
) -> Vec<PromptTemplate> {
    let mut templates = Vec::new();

    if !dir.exists() {
        return templates;
    }

    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(_) => return templates,
    };

    for entry in entries.flatten() {
        let path = entry.path();

        // Check if entry is a file (following symlinks)
        let is_file = if entry.file_type().map(|t| t.is_file()).unwrap_or(false) {
            true
        } else if entry.file_type().map(|t| t.is_symlink()).unwrap_or(false) {
            // For symlinks, resolve and check
            std::fs::metadata(&path)
                .ok()
                .map(|m| m.is_file())
                .unwrap_or(false)
        } else {
            false
        };

        if is_file && path.extension().and_then(|s| s.to_str()) == Some("md") {
            if let Some(template) = load_template_from_file(&path, get_source_info(&path)) {
                templates.push(template);
            }
        }
    }

    templates
}

/// Check whether `target` is under (or equal to) `root`.
fn is_under_path(target: &Path, root: &Path) -> bool {
    let normalized_target = std::fs::canonicalize(target).unwrap_or_else(|_| target.to_path_buf());
    let normalized_root = std::fs::canonicalize(root).unwrap_or_else(|_| root.to_path_buf());

    if normalized_target == normalized_root {
        return true;
    }

    normalized_target.starts_with(&normalized_root)
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Load all prompt templates from:
///
/// 1. **Global**: `agent_dir/prompts/`
/// 2. **Project**: `cwd/{CONFIG_DIR_NAME}/prompts/` (typically `.hamr/prompts/`)
/// 3. **Explicit paths** — individual files or directories
///
/// When `include_defaults` is true, (1) and (2) are included. Explicit paths
/// are always processed.
pub fn load_prompt_templates(options: LoadPromptTemplatesOptions) -> Vec<PromptTemplate> {
    let resolved_cwd = PathBuf::from(&options.cwd);
    let resolved_agent_dir = PathBuf::from(&options.agent_dir);

    let global_prompts_dir = resolved_agent_dir.join("prompts");
    let project_prompts_dir = resolved_cwd.join(CONFIG_DIR_NAME).join("prompts");

    let mut templates = Vec::new();

    // Build a closure that creates SourceInfo based on which directory the
    // file lives under.
    let get_source_info = |resolved_path: &Path| -> SourceInfo {
        if is_under_path(resolved_path, &global_prompts_dir) {
            create_synthetic_source_info(
                &resolved_path.to_string_lossy(),
                SyntheticSourceInfoOptions {
                    source: "local".to_string(),
                    scope: Some(SourceScope::User),
                    origin: None,
                    base_dir: Some(global_prompts_dir.to_string_lossy().into_owned()),
                },
            )
        } else if is_under_path(resolved_path, &project_prompts_dir) {
            create_synthetic_source_info(
                &resolved_path.to_string_lossy(),
                SyntheticSourceInfoOptions {
                    source: "local".to_string(),
                    scope: Some(SourceScope::Project),
                    origin: None,
                    base_dir: Some(project_prompts_dir.to_string_lossy().into_owned()),
                },
            )
        } else {
            let base_dir = if resolved_path.is_dir() {
                resolved_path.to_string_lossy().into_owned()
            } else {
                resolved_path
                    .parent()
                    .map(|p| p.to_string_lossy().into_owned())
                    .unwrap_or_default()
            };
            create_synthetic_source_info(
                &resolved_path.to_string_lossy(),
                SyntheticSourceInfoOptions {
                    source: "local".to_string(),
                    scope: None,
                    origin: None,
                    base_dir: Some(base_dir),
                },
            )
        }
    };

    // 1 & 2. Default directories
    if options.include_defaults {
        templates.extend(load_templates_from_dir(
            &global_prompts_dir,
            &get_source_info,
        ));
        templates.extend(load_templates_from_dir(
            &project_prompts_dir,
            &get_source_info,
        ));
    }

    // 3. Explicit prompt paths
    for raw_path in &options.prompt_paths {
        let resolved_path = resolve_prompt_path(raw_path, &resolved_cwd);
        if !resolved_path.exists() {
            continue;
        }

        if resolved_path.is_dir() {
            templates.extend(load_templates_from_dir(&resolved_path, &get_source_info));
        } else if resolved_path.is_file()
            && resolved_path.extension().and_then(|s| s.to_str()) == Some("md")
        {
            if let Some(template) =
                load_template_from_file(&resolved_path, get_source_info(&resolved_path))
            {
                templates.push(template);
            }
        }
    }

    templates
}

/// Resolve an explicit prompt path relative to `cwd`.
///
/// This mirrors the TS `resolvePath(rawPath, resolvedCwd, {trim: true})` call.
fn resolve_prompt_path(raw_path: &str, cwd: &Path) -> PathBuf {
    let trimmed = raw_path.trim();
    let path = PathBuf::from(trimmed);
    if path.is_absolute() {
        // Still normalize `.` and `..`
        if let Ok(canonicalized) = path.canonicalize() {
            return canonicalized;
        }
        // Fallback: lexical normalization
        let components: Vec<_> = path.components().collect();
        let mut result = PathBuf::new();
        for comp in components {
            match comp {
                std::path::Component::CurDir => {}
                std::path::Component::ParentDir => {
                    result.pop();
                }
                other => result.push(other),
            }
        }
        result
    } else {
        let joined = cwd.join(&path);
        if let Ok(canonicalized) = joined.canonicalize() {
            return canonicalized;
        }
        joined
    }
}

/// Expand a prompt template reference in user input.
///
/// If `text` starts with `/`, it is treated as a template reference of the form
/// `/template_name arg1 arg2 ...`. The matching template's content is returned
/// with arguments substituted. If no template matches, the original text is
/// returned unchanged.
///
/// ```ignore
/// # use hamr_agent::core::prompt_templates::{PromptTemplate, expand_prompt_template, create_synthetic_source_info, SourceInfo, SourceOrigin, SourceScope, SyntheticSourceInfoOptions};
/// let template = PromptTemplate {
///     name: "greet".into(),
///     description: "Greets someone".into(),
///     argument_hint: None,
///     content: "Hello $1!".into(),
///     source_info: create_synthetic_source_info("/fake/path", SyntheticSourceInfoOptions {
///         source: "local".into(),
///         scope: Some(SourceScope::Temporary),
///         origin: Some(SourceOrigin::TopLevel),
///         base_dir: None,
///     }),
///     file_path: "/fake/path/greet.md".into(),
/// };
/// let result = expand_prompt_template("/greet World", &[template]);
/// assert_eq!(result, "Hello World!");
/// ```
pub fn expand_prompt_template(text: &str, templates: &[PromptTemplate]) -> String {
    if !text.starts_with('/') {
        return text.to_string();
    }

    // Match: /template_name followed by optional args (rest of string)
    let rest = &text[1..]; // strip leading '/'
    let (template_name, args_string) = if let Some(pos) = rest.find(|c: char| c.is_whitespace()) {
        let (name, rest_str) = rest.split_at(pos);
        (name, rest_str.trim())
    } else {
        (rest, "")
    };

    if template_name.is_empty() {
        return text.to_string();
    }

    if let Some(template) = templates.iter().find(|t| t.name == template_name) {
        let args = parse_command_args(args_string);
        return substitute_args(&template.content, &args);
    }

    text.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::source_info::SourceOrigin;

    // -----------------------------------------------------------------------
    // parse_command_args
    // -----------------------------------------------------------------------

    #[test]
    fn test_parse_simple_args() {
        assert_eq!(parse_command_args("hello world"), vec!["hello", "world"]);
    }

    #[test]
    fn test_parse_double_quoted() {
        assert_eq!(
            parse_command_args(r#"foo "bar baz" qux"#),
            vec!["foo", "bar baz", "qux"]
        );
    }

    #[test]
    fn test_parse_single_quoted() {
        assert_eq!(
            parse_command_args(r#"foo 'bar baz' qux"#),
            vec!["foo", "bar baz", "qux"]
        );
    }

    #[test]
    fn test_parse_mixed_quotes() {
        // Matches TS behavior: trailing " after single-quoted 'c"d' starts a new
        // double quote, but content is empty so no extra arg is emitted.
        assert_eq!(parse_command_args(r#""a'b" 'c"d'"#), vec!["a'b", "c\"d"]);
    }

    #[test]
    fn test_parse_empty_string() {
        let empty: Vec<String> = vec![];
        assert_eq!(parse_command_args(""), empty);
        assert_eq!(parse_command_args("   "), empty);
    }

    #[test]
    fn test_parse_trailing_content() {
        assert_eq!(parse_command_args("only"), vec!["only"]);
    }

    // -----------------------------------------------------------------------
    // substitute_args
    // -----------------------------------------------------------------------

    #[test]
    fn test_basic_positional() {
        let result = substitute_args("Hello $1, you said $2", &["Alice".into(), "hi".into()]);
        assert_eq!(result, "Hello Alice, you said hi");
    }

    #[test]
    fn test_all_args_dollar_at() {
        let result = substitute_args("All: $@", &["a".into(), "b".into()]);
        assert_eq!(result, "All: a b");
    }

    #[test]
    fn test_all_args_arguments() {
        let result = substitute_args("Args: $ARGUMENTS", &["x".into(), "y".into()]);
        assert_eq!(result, "Args: x y");
    }

    #[test]
    fn test_all_args_identically() {
        // $@ and $ARGUMENTS should produce the same result
        let args = &["foo".into(), "bar".into(), "baz".into()];
        assert_eq!(
            substitute_args("Test: $@", args),
            substitute_args("Test: $ARGUMENTS", args)
        );
    }

    #[test]
    fn test_default_value() {
        let result = substitute_args("${1:-default}", &[] as &[String]);
        assert_eq!(result, "default");
    }

    #[test]
    fn test_default_value_with_arg_present() {
        let result = substitute_args("${1:-default}", &["hello".into()]);
        assert_eq!(result, "hello");
    }

    #[test]
    fn test_default_value_with_empty_arg() {
        let result = substitute_args("${1:-default}", &["".into()]);
        assert_eq!(result, "default");
    }

    #[test]
    fn test_slice_from_n() {
        let result = substitute_args("${@:2}", &["a".into(), "b".into(), "c".into()]);
        assert_eq!(result, "b c");
    }

    #[test]
    fn test_slice_with_length() {
        let result = substitute_args(
            "${@:2:2}",
            &["a".into(), "b".into(), "c".into(), "d".into()],
        );
        assert_eq!(result, "b c");
    }

    #[test]
    fn test_slice_zero_start() {
        let result = substitute_args("${@:0:2}", &["a".into(), "b".into(), "c".into()]);
        assert_eq!(result, "a b");
    }

    #[test]
    fn test_missing_positional() {
        let result = substitute_args("$3 is empty", &["a".into(), "b".into()]);
        assert_eq!(result, " is empty");
    }

    #[test]
    fn test_no_placeholder() {
        let result = substitute_args("plain text", &[] as &[String]);
        assert_eq!(result, "plain text");
    }

    #[test]
    fn test_non_digit_n_in_default_pattern_preserved() {
        // TS regex requires \d+ for N; non-digit means no match → literal text
        let result = substitute_args("${abc:-hello}", &["world".into()]);
        assert_eq!(result, "${abc:-hello}");
    }

    #[test]
    fn test_non_digit_n_in_slice_preserved() {
        // ${@:abc:2} — N is not digits, TS regex doesn't match → literal
        let result = substitute_args("${@:abc:2}", &["a".into(), "b".into()]);
        assert_eq!(result, "${@:abc:2}");
    }

    #[test]
    fn test_non_digit_length_in_slice_preserved() {
        // ${@:2:abc} — L is not digits, TS regex doesn't match → literal
        let result = substitute_args("${@:2:abc}", &["a".into(), "b".into(), "c".into()]);
        assert_eq!(result, "${@:2:abc}");
    }

    #[test]
    fn test_empty_slice_start_preserved() {
        // ${@:} — empty N, TS regex requires \d+ → no match → literal
        let result = substitute_args("${@:}", &["a".into(), "b".into()]);
        assert_eq!(result, "${@:}");
    }

    #[test]
    fn test_unrecognized_complex_pattern_preserved() {
        // ${unknown} — not a known pattern → literal
        let result = substitute_args("prefix ${unknown} suffix", &[] as &[String]);
        assert_eq!(result, "prefix ${unknown} suffix");
    }

    #[test]
    fn test_dollar_sign_not_placeholder() {
        // Standalone $ followed by digits IS a placeholder (matches TS regex \d+)
        let result = substitute_args("Cost: $50", &[] as &[String]);
        assert_eq!(result, "Cost: ");
    }

    #[test]
    fn test_default_with_colons_preserved() {
        // Default value containing ":-" should be preserved in output
        let result = substitute_args("${1:-value:-with:-colons}", &[] as &[String]);
        assert_eq!(result, "value:-with:-colons");
    }

    #[test]
    fn test_no_recursive_substitution_in_arg_values() {
        // CRITICAL: argument values containing patterns should remain literal
        let result = substitute_args("$ARGUMENTS", &["$1".into(), "$ARGUMENTS".into()]);
        assert_eq!(result, "$1 $ARGUMENTS");
        let result = substitute_args("$@", &["$100".into(), "$1".into()]);
        assert_eq!(result, "$100 $1");
        let result = substitute_args("$ARGUMENTS", &["$100".into(), "$1".into()]);
        assert_eq!(result, "$100 $1");
    }

    #[test]
    fn test_mixed_positional_and_wildcard() {
        let result = substitute_args("$1: $ARGUMENTS", &["prefix".into(), "a".into(), "b".into()]);
        assert_eq!(result, "prefix: prefix a b");
        let result = substitute_args("$1: $@", &["prefix".into(), "a".into(), "b".into()]);
        assert_eq!(result, "prefix: prefix a b");
    }

    #[test]
    fn test_empty_args_array() {
        assert_eq!(
            substitute_args("Test: $ARGUMENTS", &[] as &[String]),
            "Test: "
        );
        assert_eq!(substitute_args("Test: $@", &[] as &[String]), "Test: ");
        assert_eq!(substitute_args("Test: $1", &[] as &[String]), "Test: ");
    }

    #[test]
    fn test_multiple_occurrences_of_wildcard() {
        let result = substitute_args("$ARGUMENTS and $ARGUMENTS", &["a".into(), "b".into()]);
        assert_eq!(result, "a b and a b");
        let result = substitute_args("$@ and $@", &["a".into(), "b".into()]);
        assert_eq!(result, "a b and a b");
        let result = substitute_args("$@ and $ARGUMENTS", &["a".into(), "b".into()]);
        assert_eq!(result, "a b and a b");
    }

    #[test]
    fn test_special_chars_in_arguments() {
        let result = substitute_args("$1 $2: $ARGUMENTS", &["arg100".into(), "@user".into()]);
        assert_eq!(result, "arg100 @user: arg100 @user");
    }

    #[test]
    fn test_out_of_range_positional() {
        let result = substitute_args("$1 $2 $3 $4 $5", &["a".into(), "b".into()]);
        assert_eq!(result, "a b   ");
    }

    #[test]
    fn test_unicode_in_arguments() {
        let result = substitute_args("$ARGUMENTS", &["日本語".into(), "🎉".into(), "café".into()]);
        assert_eq!(result, "日本語 🎉 café");
    }

    #[test]
    fn test_newlines_in_argument_values() {
        let result = substitute_args("$1 $2", &["line1\nline2".into(), "tab\tthere".into()]);
        assert_eq!(result, "line1\nline2 tab\tthere");
    }

    #[test]
    fn test_consecutive_dollar_patterns() {
        let result = substitute_args("$1$2", &["a".into(), "b".into()]);
        assert_eq!(result, "ab");
    }

    #[test]
    fn test_zero_index_is_empty() {
        let result = substitute_args("$0", &["a".into(), "b".into()]);
        assert_eq!(result, "");
    }

    #[test]
    fn test_decimal_number_in_pattern() {
        let result = substitute_args("$1.5", &["a".into()]);
        assert_eq!(result, "a.5");
    }

    #[test]
    fn test_wildcard_as_part_of_word() {
        let result = substitute_args("pre$ARGUMENTS", &["a".into(), "b".into()]);
        assert_eq!(result, "prea b");
        let result = substitute_args("pre$@", &["a".into(), "b".into()]);
        assert_eq!(result, "prea b");
    }

    #[test]
    fn test_empty_in_middle_of_args() {
        let result = substitute_args("$ARGUMENTS", &["a".into(), "".into(), "c".into()]);
        assert_eq!(result, "a  c");
    }

    #[test]
    fn test_trailing_and_leading_spaces_in_args() {
        let result = substitute_args("$ARGUMENTS", &["  leading  ".into(), "trailing  ".into()]);
        assert_eq!(result, "  leading   trailing  ");
    }

    #[test]
    fn test_non_matching_patterns_preserved() {
        let result = substitute_args("$A $$ $ $ARGS", &["a".into()]);
        assert_eq!(result, "$A $$ $ $ARGS");
    }

    #[test]
    fn test_case_variations_case_sensitive() {
        let result = substitute_args(
            "$arguments $Arguments $ARGUMENTS",
            &["a".into(), "b".into()],
        );
        assert_eq!(result, "$arguments $Arguments a b");
    }

    #[test]
    fn test_long_argument_list() {
        let args: Vec<String> = (0..100).map(|i| format!("arg{i}")).collect();
        let result = substitute_args("$ARGUMENTS", &args);
        assert_eq!(result, args.join(" "));
    }

    #[test]
    fn test_multi_digit_positional() {
        let args: Vec<String> = (0..15).map(|i| format!("val{i}")).collect();
        assert_eq!(substitute_args("$10 $12 $15", &args), "val9 val11 val14");
    }

    #[test]
    fn test_mixed_numbered_and_wildcard() {
        let result = substitute_args(
            "$1: $@ ($ARGUMENTS)",
            &["first".into(), "second".into(), "third".into()],
        );
        assert_eq!(result, "first: first second third (first second third)");
    }

    #[test]
    fn test_only_placeholders() {
        let result = substitute_args("$1 $2 $@", &["a".into(), "b".into(), "c".into()]);
        assert_eq!(result, "a b a b c");
    }

    // -----------------------------------------------------------------------
    // Positional defaults
    // -----------------------------------------------------------------------

    #[test]
    fn test_default_missing_positional() {
        let result = substitute_args("List exactly ${1:-7} next steps", &[] as &[String]);
        assert_eq!(result, "List exactly 7 next steps");
    }

    #[test]
    fn test_default_with_arg_present() {
        let result = substitute_args("List exactly ${1:-7} next steps", &["3".into()]);
        assert_eq!(result, "List exactly 3 next steps");
    }

    #[test]
    fn test_multiple_positional_defaults() {
        let result = substitute_args("${1:-7} ${2:-brief}", &[] as &[String]);
        assert_eq!(result, "7 brief");
        let result = substitute_args("${1:-7} ${2:-brief}", &["3".into()]);
        assert_eq!(result, "3 brief");
        let result = substitute_args("${1:-7} ${2:-brief}", &["3".into(), "verbose".into()]);
        assert_eq!(result, "3 verbose");
    }

    #[test]
    fn test_default_no_recursion_in_args_or_defaults() {
        let result = substitute_args("${1:-7}", &["$ARGUMENTS".into()]);
        assert_eq!(result, "$ARGUMENTS");
        let result = substitute_args("${1:-7}", &["$1".into()]);
        assert_eq!(result, "$1");
        let result = substitute_args("${3:-$ARGUMENTS}", &["a".into(), "b".into()]);
        assert_eq!(result, "$ARGUMENTS");
    }

    #[test]
    fn test_default_with_spaces() {
        let result = substitute_args("${1:-seven steps}", &[] as &[String]);
        assert_eq!(result, "seven steps");
    }

    #[test]
    fn test_out_of_range_default() {
        let result = substitute_args("${3:-fallback}", &["a".into(), "b".into()]);
        assert_eq!(result, "fallback");
    }

    #[test]
    fn test_mix_default_with_placeholders() {
        let result = substitute_args("$1 ${2:-x} $ARGUMENTS", &["a".into()]);
        assert_eq!(result, "a x a");
    }

    // -----------------------------------------------------------------------
    // Array slicing (bash-style)
    // -----------------------------------------------------------------------

    #[test]
    fn test_slice_from_n_various() {
        assert_eq!(
            substitute_args("${@:2}", &["a".into(), "b".into(), "c".into(), "d".into()]),
            "b c d"
        );
        assert_eq!(
            substitute_args("${@:1}", &["a".into(), "b".into(), "c".into()]),
            "a b c"
        );
        assert_eq!(
            substitute_args("${@:3}", &["a".into(), "b".into(), "c".into(), "d".into()]),
            "c d"
        );
    }

    #[test]
    fn test_slice_with_length_various() {
        assert_eq!(
            substitute_args(
                "${@:2:2}",
                &["a".into(), "b".into(), "c".into(), "d".into()]
            ),
            "b c"
        );
        assert_eq!(
            substitute_args("${@:1:1}", &["a".into(), "b".into(), "c".into()]),
            "a"
        );
        assert_eq!(
            substitute_args(
                "${@:3:1}",
                &["a".into(), "b".into(), "c".into(), "d".into()]
            ),
            "c"
        );
        assert_eq!(
            substitute_args(
                "${@:2:3}",
                &["a".into(), "b".into(), "c".into(), "d".into(), "e".into()]
            ),
            "b c d"
        );
    }

    #[test]
    fn test_slice_out_of_range() {
        assert_eq!(substitute_args("${@:99}", &["a".into(), "b".into()]), "");
        assert_eq!(substitute_args("${@:5}", &["a".into(), "b".into()]), "");
        assert_eq!(substitute_args("${@:10:5}", &["a".into(), "b".into()]), "");
    }

    #[test]
    fn test_slice_zero_length() {
        assert_eq!(
            substitute_args("${@:2:0}", &["a".into(), "b".into(), "c".into()]),
            ""
        );
        assert_eq!(substitute_args("${@:1:0}", &["a".into(), "b".into()]), "");
    }

    #[test]
    fn test_slice_exceeding_array() {
        assert_eq!(
            substitute_args("${@:2:99}", &["a".into(), "b".into(), "c".into()]),
            "b c"
        );
        assert_eq!(
            substitute_args("${@:1:10}", &["a".into(), "b".into()]),
            "a b"
        );
    }

    #[test]
    fn test_slice_before_wildcard() {
        let result = substitute_args("${@:2} vs $@", &["a".into(), "b".into(), "c".into()]);
        assert_eq!(result, "b c vs a b c");
        let result = substitute_args(
            "First: ${@:1:1}, All: $@",
            &["x".into(), "y".into(), "z".into()],
        );
        assert_eq!(result, "First: x, All: x y z");
    }

    #[test]
    fn test_slice_no_recursive_substitution() {
        let result = substitute_args("${@:1}", &["${@:2}".into(), "test".into()]);
        assert_eq!(result, "${@:2} test");
        let result = substitute_args("${@:2}", &["a".into(), "${@:3}".into(), "c".into()]);
        assert_eq!(result, "${@:3} c");
    }

    #[test]
    fn test_slice_mixed_with_positional() {
        let result = substitute_args("$1: ${@:2}", &["cmd".into(), "arg1".into(), "arg2".into()]);
        assert_eq!(result, "cmd: arg1 arg2");
        let result = substitute_args(
            "$1 $2 ${@:3}",
            &["a".into(), "b".into(), "c".into(), "d".into()],
        );
        assert_eq!(result, "a b c d");
    }

    #[test]
    fn test_slice_treat_zero_as_one() {
        assert_eq!(
            substitute_args("${@:0}", &["a".into(), "b".into(), "c".into()]),
            "a b c"
        );
    }

    #[test]
    fn test_slice_empty_args_array() {
        assert_eq!(substitute_args("${@:2}", &[] as &[String]), "");
        assert_eq!(substitute_args("${@:1}", &[] as &[String]), "");
    }

    #[test]
    fn test_slice_single_arg() {
        assert_eq!(substitute_args("${@:1}", &["only".into()]), "only");
        assert_eq!(substitute_args("${@:2}", &["only".into()]), "");
    }

    #[test]
    fn test_slice_in_middle_of_text() {
        let result = substitute_args(
            "Process ${@:2} with $1",
            &["tool".into(), "file1".into(), "file2".into()],
        );
        assert_eq!(result, "Process file1 file2 with tool");
    }

    #[test]
    fn test_multiple_slices() {
        let result = substitute_args("${@:1:1} and ${@:2}", &["a".into(), "b".into(), "c".into()]);
        assert_eq!(result, "a and b c");
        let result = substitute_args(
            "${@:1:2} vs ${@:3:2}",
            &["a".into(), "b".into(), "c".into(), "d".into(), "e".into()],
        );
        assert_eq!(result, "a b vs c d");
    }

    #[test]
    fn test_slice_with_no_spacing() {
        let result = substitute_args("prefix${@:2}suffix", &["a".into(), "b".into(), "c".into()]);
        assert_eq!(result, "prefixb csuffix");
    }

    #[test]
    fn test_slice_large_length() {
        let args: Vec<String> = (1..=10).map(|i| format!("arg{i}")).collect();
        let result = substitute_args("${@:5:100}", &args);
        assert_eq!(result, "arg5 arg6 arg7 arg8 arg9 arg10");
    }

    // -----------------------------------------------------------------------
    // parse_command_args edge cases
    // -----------------------------------------------------------------------

    #[test]
    fn test_parse_extra_spaces() {
        assert_eq!(parse_command_args("a  b   c"), vec!["a", "b", "c"]);
    }

    #[test]
    fn test_parse_tabs_as_separators() {
        assert_eq!(parse_command_args("a\tb\tc"), vec!["a", "b", "c"]);
    }

    #[test]
    fn test_parse_special_characters() {
        assert_eq!(
            parse_command_args("$100 @user #tag"),
            vec!["$100", "@user", "#tag"]
        );
    }

    #[test]
    fn test_parse_unicode() {
        assert_eq!(
            parse_command_args("日本語 🎉 café"),
            vec!["日本語", "🎉", "café"]
        );
    }

    #[test]
    fn test_parse_newlines_in_quotes() {
        assert_eq!(
            parse_command_args("\"line1\nline2\" second"),
            vec!["line1\nline2", "second"]
        );
    }

    #[test]
    fn test_parse_unquoted_newlines_as_separators() {
        assert_eq!(
            parse_command_args("label-2\n\nHere is some description #2."),
            vec!["label-2", "Here", "is", "some", "description", "#2."]
        );
    }

    #[test]
    fn test_parse_mixed_unquoted_whitespace() {
        assert_eq!(parse_command_args("a\n\n\tb  c"), vec!["a", "b", "c"]);
    }

    #[test]
    fn test_parse_trailing_spaces() {
        assert_eq!(parse_command_args("a b c   "), vec!["a", "b", "c"]);
    }

    #[test]
    fn test_parse_leading_spaces() {
        assert_eq!(parse_command_args("   a b c"), vec!["a", "b", "c"]);
    }

    #[test]
    fn test_parse_quoted_empty_string_skipped() {
        assert_eq!(parse_command_args("\"\" \" \""), vec![" "]);
    }

    // -----------------------------------------------------------------------
    // expand_prompt_template
    // -----------------------------------------------------------------------

    fn make_template(name: &str, content: &str) -> PromptTemplate {
        PromptTemplate {
            name: name.to_string(),
            description: String::new(),
            argument_hint: None,
            content: content.to_string(),
            source_info: create_synthetic_source_info(
                "/fake/path",
                SyntheticSourceInfoOptions {
                    source: "local".into(),
                    scope: Some(SourceScope::Temporary),
                    origin: Some(SourceOrigin::TopLevel),
                    base_dir: None,
                },
            ),
            file_path: format!("/fake/path/{}.md", name),
        }
    }

    #[test]
    fn test_expand_simple() {
        let templates = vec![make_template("greet", "Hello $1!")];
        assert_eq!(
            expand_prompt_template("/greet World", &templates),
            "Hello World!"
        );
    }

    #[test]
    fn test_expand_no_args() {
        let templates = vec![make_template("hello", "Hello!")];
        assert_eq!(expand_prompt_template("/hello", &templates), "Hello!");
    }

    #[test]
    fn test_expand_no_match_returns_original() {
        let templates = vec![];
        assert_eq!(
            expand_prompt_template("/unknown arg1", &templates),
            "/unknown arg1"
        );
    }

    #[test]
    fn test_expand_non_template_input() {
        let templates = vec![];
        assert_eq!(
            expand_prompt_template("just normal text", &templates),
            "just normal text"
        );
    }

    #[test]
    fn test_expand_just_slash() {
        let templates = vec![];
        assert_eq!(expand_prompt_template("/", &templates), "/");
    }

    #[test]
    fn test_expand_multi_arg() {
        let templates = vec![make_template("repeat", "$1 $2 $3")];
        assert_eq!(expand_prompt_template("/repeat a b c", &templates), "a b c");
    }

    // -----------------------------------------------------------------------
    // load_template_from_file (integration-style)
    // -----------------------------------------------------------------------

    #[test]
    fn test_load_template_file() {
        use std::io::Write;
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("test_template.md");
        let mut f = std::fs::File::create(&file_path).unwrap();
        writeln!(
            f,
            "---\ndescription: A test template\nargument-hint: \"<input>\"\n---\n\nThis is the body $1"
        )
        .unwrap();
        drop(f);

        // Create minimal SourceInfo
        let source_info = create_synthetic_source_info(
            &file_path.to_string_lossy(),
            SyntheticSourceInfoOptions {
                source: "local".into(),
                scope: Some(SourceScope::Temporary),
                origin: None,
                base_dir: None,
            },
        );

        let template = load_template_from_file(&file_path, source_info).unwrap();
        assert_eq!(template.name, "test_template");
        assert_eq!(template.description, "A test template");
        assert_eq!(template.argument_hint.as_deref(), Some("<input>"));
        assert_eq!(template.content, "This is the body $1");
    }

    #[test]
    fn test_load_template_no_frontmatter() {
        use std::io::Write;
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("simple.md");
        let mut f = std::fs::File::create(&file_path).unwrap();
        writeln!(f, "First line of body\nSecond line").unwrap();
        drop(f);

        let source_info = create_synthetic_source_info(
            &file_path.to_string_lossy(),
            SyntheticSourceInfoOptions {
                source: "local".into(),
                scope: None,
                origin: None,
                base_dir: None,
            },
        );

        let template = load_template_from_file(&file_path, source_info).unwrap();
        assert_eq!(template.name, "simple");
        // Description from first line (up to 60 chars)
        assert_eq!(template.description, "First line of body");
        assert!(template.argument_hint.is_none());
        // writeln! appends a newline, so the body includes trailing \n
        assert_eq!(template.content, "First line of body\nSecond line\n");
    }

    #[test]
    fn test_load_templates_from_dir() {
        use std::io::Write;
        let dir = tempfile::tempdir().unwrap();
        let prompts_dir = dir.path().join("prompts");
        std::fs::create_dir_all(&prompts_dir).unwrap();

        let mut f1 = std::fs::File::create(prompts_dir.join("a.md")).unwrap();
        writeln!(f1, "---\ndescription: Template A\n---\n\nContent A").unwrap();
        let mut f2 = std::fs::File::create(prompts_dir.join("b.md")).unwrap();
        writeln!(f2, "---\ndescription: Template B\n---\n\nContent B").unwrap();
        drop(f1);
        drop(f2);

        let get_info = |p: &Path| -> SourceInfo {
            create_synthetic_source_info(
                &p.to_string_lossy(),
                SyntheticSourceInfoOptions {
                    source: "local".into(),
                    scope: Some(SourceScope::Temporary),
                    origin: None,
                    base_dir: None,
                },
            )
        };

        let templates = load_templates_from_dir(&prompts_dir, &get_info);
        assert_eq!(templates.len(), 2);

        let names: Vec<&str> = templates.iter().map(|t| t.name.as_str()).collect();
        assert!(names.contains(&"a"));
        assert!(names.contains(&"b"));
    }

    #[test]
    fn test_is_under_path() {
        // Use temp dirs for real filesystem paths
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("prompts");
        std::fs::create_dir_all(&root).unwrap();
        let child = root.join("my-template.md");
        std::fs::write(&child, "content").unwrap();

        assert!(is_under_path(&child, &root));

        let unrelated = Path::new("/tmp/other.md");
        assert!(!is_under_path(&unrelated, &root));
    }

    #[test]
    fn test_load_templates_from_missing_dir() {
        let dir = Path::new("/nonexistent/dir/that/will/never/exist");
        let get_info = |p: &Path| -> SourceInfo {
            create_synthetic_source_info(
                &p.to_string_lossy(),
                SyntheticSourceInfoOptions {
                    source: "local".into(),
                    scope: None,
                    origin: None,
                    base_dir: None,
                },
            )
        };
        let templates = load_templates_from_dir(dir, &get_info);
        assert!(templates.is_empty());
    }

    #[test]
    fn test_empty_frontmatter_description() {
        use std::io::Write;
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("test.md");
        let mut f = std::fs::File::create(&file_path).unwrap();
        writeln!(f, "---\ndescription: \"\"\n---\n\nFirst body line").unwrap();
        drop(f);

        let source_info = create_synthetic_source_info(
            &file_path.to_string_lossy(),
            SyntheticSourceInfoOptions {
                source: "local".into(),
                scope: Some(SourceScope::Temporary),
                origin: None,
                base_dir: None,
            },
        );
        let template = load_template_from_file(&file_path, source_info).unwrap();
        assert_eq!(template.description, "First body line");
    }

    // -----------------------------------------------------------------------
    // expand_prompt_template with substitution integration
    // -----------------------------------------------------------------------

    #[test]
    fn test_expand_with_substitution() {
        let templates = vec![make_template("summarize", "Summarize $1 for me")];
        // $1 only captures the first arg "the", not the full arg string "the code"
        let result = expand_prompt_template("/summarize the code", &templates);
        assert_eq!(result, "Summarize the for me");
    }

    #[test]
    fn test_expand_with_all_args() {
        let templates = vec![make_template("list", "Items: $@")];
        let result = expand_prompt_template("/list a b c", &templates);
        assert_eq!(result, "Items: a b c");
    }

    #[test]
    fn test_expand_with_default_value() {
        let templates = vec![make_template("greet", "Hello ${1:-friend}!")];
        let result = expand_prompt_template("/greet", &templates);
        assert_eq!(result, "Hello friend!");
    }

    #[test]
    fn test_expand_with_default_value_arg_present() {
        let templates = vec![make_template("greet", "Hello ${1:-friend}!")];
        let result = expand_prompt_template("/greet World", &templates);
        assert_eq!(result, "Hello World!");
    }

    // -----------------------------------------------------------------------
    // expand_prompt_template integration with newline in args
    // -----------------------------------------------------------------------

    #[test]
    fn test_expand_with_newline_separated_args() {
        let templates = vec![make_template("arg-test", "- arg1: $1\n- rest: ${@:2}")];
        let result = expand_prompt_template(
            "/arg-test label-2\n\nHere is some description #2.",
            &templates,
        );
        assert_eq!(
            result,
            "- arg1: label-2\n- rest: Here is some description #2."
        );
    }

    #[test]
    fn test_expand_command_separated_from_args_by_newline() {
        let templates = vec![make_template("arg-test", "arg1: $1")];
        let result = expand_prompt_template("/arg-test\nlabel-2", &templates);
        assert_eq!(result, "arg1: label-2");
    }

    // -----------------------------------------------------------------------
    // parseCommandArgs + substituteArgs integration
    // -----------------------------------------------------------------------

    #[test]
    fn test_parse_and_substitute_together() {
        let input = r#"Button "onClick handler" "disabled support""#;
        let args = parse_command_args(input);
        let template = "Create component $1 with features: $ARGUMENTS";
        let result = substitute_args(template, &args);
        assert_eq!(
            result,
            "Create component Button with features: Button onClick handler disabled support"
        );
    }

    #[test]
    fn test_parse_and_substitute_readme_example() {
        let input = r#"Button "onClick handler" "disabled support""#;
        let args = parse_command_args(input);
        let template = "Create a React component named $1 with features: $ARGUMENTS";
        let result = substitute_args(template, &args);
        assert_eq!(
            result,
            "Create a React component named Button with features: Button onClick handler disabled support"
        );
    }

    #[test]
    fn test_parse_and_substitute_same_with_both_wildcards() {
        let args = parse_command_args("feature1 feature2 feature3");
        let r1 = substitute_args("Implement: $@", &args);
        let r2 = substitute_args("Implement: $ARGUMENTS", &args);
        assert_eq!(r1, r2);
    }

    // -----------------------------------------------------------------------
    // argument-hint frontmatter loading (integration-style)
    // -----------------------------------------------------------------------

    #[test]
    fn test_load_template_with_argument_hint() {
        use std::io::Write;
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("pr.md");
        let mut f = std::fs::File::create(&file_path).unwrap();
        writeln!(
            f,
            "---\ndescription: Review PRs\nargument-hint: \"<PR-URL>\"\n---\n\nYou are given PR URLs: $@"
        )
        .unwrap();
        drop(f);

        let source_info = create_synthetic_source_info(
            &file_path.to_string_lossy(),
            SyntheticSourceInfoOptions {
                source: "local".into(),
                scope: Some(SourceScope::Temporary),
                origin: None,
                base_dir: None,
            },
        );

        let template = load_template_from_file(&file_path, source_info).unwrap();
        assert_eq!(template.argument_hint.as_deref(), Some("<PR-URL>"));
    }

    #[test]
    fn test_load_template_without_argument_hint() {
        use std::io::Write;
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("simple.md");
        let mut f = std::fs::File::create(&file_path).unwrap();
        writeln!(f, "---\ndescription: A simple template\n---\n\nBody here").unwrap();
        drop(f);

        let source_info = create_synthetic_source_info(
            &file_path.to_string_lossy(),
            SyntheticSourceInfoOptions {
                source: "local".into(),
                scope: Some(SourceScope::Temporary),
                origin: None,
                base_dir: None,
            },
        );

        let template = load_template_from_file(&file_path, source_info).unwrap();
        assert!(template.argument_hint.is_none());
    }

    #[test]
    fn test_load_template_empty_argument_hint() {
        use std::io::Write;
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("empty-hint.md");
        let mut f = std::fs::File::create(&file_path).unwrap();
        writeln!(
            f,
            "---\ndescription: Empty hint\nargument-hint: \"\"\n---\n\nDo something"
        )
        .unwrap();
        drop(f);

        let source_info = create_synthetic_source_info(
            &file_path.to_string_lossy(),
            SyntheticSourceInfoOptions {
                source: "local".into(),
                scope: Some(SourceScope::Temporary),
                origin: None,
                base_dir: None,
            },
        );

        let template = load_template_from_file(&file_path, source_info).unwrap();
        assert!(template.argument_hint.is_none());
    }
}
