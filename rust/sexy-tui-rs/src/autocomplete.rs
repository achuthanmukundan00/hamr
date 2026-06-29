/// Autocomplete support for slash commands and file paths.
/// Faithful port of src/autocomplete.ts.
use crate::fuzzy::fuzzy_filter;
use std::path::Path;

const PATH_DELIMITERS: &[char] = &[' ', '\t', '"', '\'', '='];

// ── Helper functions ──────────────────────────────────────────────────────

fn to_display_path(value: &str) -> String {
    value.replace('\\', "/")
}

fn escape_regex(value: &str) -> String {
    let special = ['.', '*', '+', '?', '^', '$', '{', '}', '(', ')', '|', '[', ']', '\\'];
    let mut result = String::with_capacity(value.len());
    for ch in value.chars() {
        if special.contains(&ch) {
            result.push('\\');
        }
        result.push(ch);
    }
    result
}

fn build_fd_path_query(query: &str) -> String {
    let normalized = to_display_path(query);
    if !normalized.contains('/') {
        return normalized;
    }

    let has_trailing = normalized.ends_with('/');
    let trimmed = normalized.trim_matches('/');
    if trimmed.is_empty() {
        return normalized;
    }

    let separator_pattern = if cfg!(windows) { "[\\\\/]" } else { "/" };
    let segments: Vec<String> = trimmed
        .split('/')
        .filter(|s| !s.is_empty())
        .map(|s| escape_regex(s))
        .collect();
    if segments.is_empty() {
        return normalized;
    }

    let mut pattern = segments.join(separator_pattern);
    if has_trailing {
        pattern.push_str(separator_pattern);
    }
    pattern
}

fn find_last_delimiter(text: &str) -> isize {
    for (i, ch) in text.char_indices().rev() {
        if PATH_DELIMITERS.contains(&ch) {
            return i as isize;
        }
    }
    -1
}

fn find_unclosed_quote_start(text: &str) -> Option<usize> {
    let mut in_quotes = false;
    let mut quote_start = 0;

    for (i, ch) in text.char_indices() {
        if ch == '"' {
            in_quotes = !in_quotes;
            if in_quotes {
                quote_start = i;
            }
        }
    }

    if in_quotes {
        Some(quote_start)
    } else {
        None
    }
}

fn is_token_start(text: &str, index: usize) -> bool {
    index == 0 || text.as_bytes().get(index.wrapping_sub(1)).is_some_and(|&c| PATH_DELIMITERS.contains(&(c as char)))
}

fn extract_quoted_prefix(text: &str) -> Option<String> {
    let quote_start = find_unclosed_quote_start(text)?;

    if quote_start > 0 && text.as_bytes().get(quote_start - 1) == Some(&b'@') {
        if !is_token_start(text, quote_start - 1) {
            return None;
        }
        return Some(text[quote_start - 1..].to_string());
    }

    if !is_token_start(text, quote_start) {
        return None;
    }

    Some(text[quote_start..].to_string())
}

fn parse_path_prefix(prefix: &str) -> (String, bool, bool) {
    // Returns (raw_prefix, is_at_prefix, is_quoted_prefix)
    if prefix.starts_with("@\"") {
        (prefix[2..].to_string(), true, true)
    } else if prefix.starts_with('"') {
        (prefix[1..].to_string(), false, true)
    } else if prefix.starts_with('@') {
        (prefix[1..].to_string(), true, false)
    } else {
        (prefix.to_string(), false, false)
    }
}

fn build_completion_value(path: &str, _is_directory: bool, is_at_prefix: bool, is_quoted_prefix: bool) -> String {
    let needs_quotes = is_quoted_prefix || path.contains(' ');
    let prefix = if is_at_prefix { "@" } else { "" };

    if !needs_quotes {
        return format!("{}{}", prefix, path);
    }

    let open_quote = if is_at_prefix { "@\"" } else { "\"" };
    format!("{}{}\"", open_quote, path)
}

fn expand_home_path(path: &str, homedir_str: &str) -> String {
    if path.starts_with("~/") {
        let expanded = Path::new(homedir_str).join(&path[2..]);
        let expanded_str = expanded.to_string_lossy().to_string();
        if path.ends_with('/') && !expanded_str.ends_with('/') {
            format!("{}/", expanded_str)
        } else {
            expanded_str
        }
    } else if path == "~" {
        homedir_str.to_string()
    } else {
        path.to_string()
    }
}

// ── fd directory walking ──────────────────────────────────────────────────

fn walk_directory_with_fd(base_dir: &str, fd_path: &str, query: &str, max_results: usize) -> Vec<(String, bool)> {
    let mut cmd = std::process::Command::new(fd_path);
    cmd.args([
        "--base-directory",
        base_dir,
        "--max-results",
        &max_results.to_string(),
        "--type",
        "f",
        "--type",
        "d",
        "--follow",
        "--hidden",
        "--exclude",
        ".git",
        "--exclude",
        ".git/*",
        "--exclude",
        ".git/**",
    ]);

    if to_display_path(query).contains('/') {
        cmd.arg("--full-path");
    }
    if !query.is_empty() {
        cmd.arg(build_fd_path_query(query));
    }

    let output = match cmd.output() {
        Ok(o) if o.status.success() => o,
        _ => return Vec::new(),
    };

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut results = Vec::new();
    for line in stdout.lines() {
        if line.is_empty() {
            continue;
        }
        let display_line = to_display_path(line);
        let has_trailing = display_line.ends_with('/');
        let normalized = if has_trailing {
            &display_line[..display_line.len() - 1]
        } else {
            &display_line
        };
        if normalized == ".git" || normalized.starts_with(".git/") || normalized.contains("/.git/") {
            continue;
        }
        results.push((display_line, has_trailing));
    }
    results
}

// ── Public types ──────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct AutocompleteItem {
    pub value: String,
    pub label: String,
    pub description: Option<String>,
}

pub struct SlashCommand {
    pub name: String,
    pub description: Option<String>,
    pub argument_hint: Option<String>,
    /// Function to get argument completions for this command
    pub get_argument_completions: Option<Box<dyn Fn(&str) -> Vec<AutocompleteItem> + Send + Sync>>,
}

impl SlashCommand {
    pub fn new(name: impl Into<String>) -> Self {
        SlashCommand {
            name: name.into(),
            description: None,
            argument_hint: None,
            get_argument_completions: None,
        }
    }
}

#[derive(Debug)]
pub struct AutocompleteSuggestions {
    pub items: Vec<AutocompleteItem>,
    pub prefix: String,
}

/// Result of applying an autocomplete selection.
pub struct CompletionResult {
    pub lines: Vec<String>,
    pub cursor_line: usize,
    pub cursor_col: usize,
}

/// Trait for autocomplete providers.
pub trait AutocompleteProvider {
    /// Characters that should naturally trigger this provider at token boundaries.
    fn trigger_characters(&self) -> Vec<String> {
        vec![]
    }

    /// Get autocomplete suggestions for current text/cursor position.
    fn get_suggestions(
        &self,
        lines: &[String],
        cursor_line: usize,
        cursor_col: usize,
        force: bool,
    ) -> Option<AutocompleteSuggestions>;

    /// Apply the selected item, returning new text and cursor position.
    fn apply_completion(
        &self,
        lines: &[String],
        cursor_line: usize,
        cursor_col: usize,
        item: &AutocompleteItem,
        prefix: &str,
    ) -> CompletionResult;

    /// Check if file completion should trigger for explicit Tab completion.
    fn should_trigger_file_completion(
        &self,
        lines: &[String],
        cursor_line: usize,
        cursor_col: usize,
    ) -> bool {
        let _ = (lines, cursor_line, cursor_col);
        true
    }
}

// ── CombinedAutocompleteProvider ───────────────────────────────────────────

pub struct CombinedAutocompleteProvider {
    commands: Vec<SlashCommand>,
    base_path: String,
    fd_path: Option<String>,
}

impl CombinedAutocompleteProvider {
    pub fn new(commands: Vec<SlashCommand>, base_path: String, fd_path: Option<String>) -> Self {
        CombinedAutocompleteProvider {
            commands,
            base_path,
            fd_path,
        }
    }
}

impl AutocompleteProvider for CombinedAutocompleteProvider {
    fn trigger_characters(&self) -> Vec<String> {
        vec!["/".into(), "@".into(), "#".into()]
    }

    fn get_suggestions(
        &self,
        lines: &[String],
        cursor_line: usize,
        cursor_col: usize,
        force: bool,
    ) -> Option<AutocompleteSuggestions> {
        let current_line = lines.get(cursor_line).map(|s| s.as_str()).unwrap_or("");
        let text_before_cursor = &current_line[..cursor_col.min(current_line.len())];

        // 1. @ prefix → fuzzy file suggestions
        if let Some(at_prefix) = self.extract_at_prefix(text_before_cursor) {
            let (_raw_prefix, _is_at_prefix, is_quoted_prefix) = parse_path_prefix(&at_prefix);
            let suggestions = self.get_fuzzy_file_suggestions(&at_prefix, is_quoted_prefix);
            if suggestions.is_empty() {
                return None;
            }
            return Some(AutocompleteSuggestions {
                items: suggestions,
                prefix: at_prefix,
            });
        }

        // 2. Slash command (only if not forced — force goes to path completion)
        if !force && text_before_cursor.starts_with('/') {
            let space_idx = text_before_cursor.find(' ');

            if space_idx.is_none() {
                // Completing command name
                let query = &text_before_cursor[1..];
                let command_items: Vec<AutocompleteItem> = self.commands.iter().map(|cmd| {
                    let hint = cmd.argument_hint.as_deref();
                    let desc = cmd.description.as_deref().unwrap_or("");
                    let full_desc = match (hint, desc) {
                        (Some(h), d) if !d.is_empty() => format!("{} — {}", h, d),
                        (Some(h), _) => h.to_string(),
                        _ => desc.to_string(),
                    };
                    AutocompleteItem {
                        value: cmd.name.clone(),
                        label: cmd.name.clone(),
                        description: if full_desc.is_empty() { None } else { Some(full_desc) },
                    }
                }).collect();

                let filtered = fuzzy_filter(&command_items, query, |item: &AutocompleteItem| item.value.clone());
                if filtered.is_empty() {
                    return None;
                }
                let items: Vec<AutocompleteItem> = filtered.into_iter().map(|item| AutocompleteItem {
                    value: item.value.clone(),
                    label: item.label.clone(),
                    description: item.description.clone(),
                }).collect();

                return Some(AutocompleteSuggestions {
                    items,
                    prefix: text_before_cursor.to_string(),
                });
            }

            // Has space → argument completion
            let space_idx = space_idx.unwrap();
            let command_name = &text_before_cursor[1..space_idx];
            let argument_text = &text_before_cursor[space_idx + 1..];

            let command = self.commands.iter().find(|cmd| cmd.name == command_name);
            if let Some(cmd) = command {
                if let Some(ref arg_fn) = cmd.get_argument_completions {
                    let arg_items = arg_fn(argument_text);
                    if !arg_items.is_empty() {
                        return Some(AutocompleteSuggestions {
                            items: arg_items,
                            prefix: argument_text.to_string(),
                        });
                    }
                }
            }
            return None;
        }

        // 3. Path completion
        let path_match = self.extract_path_prefix(text_before_cursor, force);
        match path_match {
            None => None,
            Some(path_prefix) => {
                let suggestions = self.get_file_suggestions(&path_prefix);
                if suggestions.is_empty() {
                    return None;
                }
                Some(AutocompleteSuggestions {
                    items: suggestions,
                    prefix: path_prefix,
                })
            }
        }
    }

    fn apply_completion(
        &self,
        lines: &[String],
        cursor_line: usize,
        cursor_col: usize,
        item: &AutocompleteItem,
        prefix: &str,
    ) -> CompletionResult {
        let current_line = lines.get(cursor_line).map(|s| s.as_str()).unwrap_or("");
        let before_prefix_len = if cursor_col >= prefix.len() {
            cursor_col - prefix.len()
        } else {
            0
        };
        let before_prefix = &current_line[..before_prefix_len.min(current_line.len())];
        let after_cursor = &current_line[cursor_col.min(current_line.len())..];

        let is_quoted_prefix = prefix.starts_with('"') || prefix.starts_with("@\"");
        let has_leading_quote_after = after_cursor.starts_with('"');
        let has_trailing_quote_in_item = item.value.ends_with('"');
        let adjusted_after = if is_quoted_prefix && has_trailing_quote_in_item && has_leading_quote_after {
            &after_cursor[1..]
        } else {
            after_cursor
        };

        // Check if we're completing a slash command
        let is_slash_command = prefix.starts_with('/') && before_prefix.trim().is_empty() && !prefix[1..].contains('/');
        if is_slash_command {
            let new_line = format!("{}/{}{} {}", before_prefix, item.value, "", adjusted_after.trim_start());
            let mut new_lines = lines.to_vec();
            new_lines[cursor_line] = new_line;
            return CompletionResult {
                lines: new_lines,
                cursor_line,
                cursor_col: before_prefix.len() + item.value.len() + 2, // +2 for "/" and space
            };
        }

        // Check if we're completing a file attachment (prefix starts with "@")
        if prefix.starts_with('@') {
            let is_directory = item.label.ends_with('/');
            let _suffix = if is_directory { "" } else { " " };
            let new_line = format!("{}{}{}", before_prefix, item.value, adjusted_after);
            let mut new_lines = lines.to_vec();
            new_lines[cursor_line] = new_line;
            let has_trailing_quote = item.value.ends_with('"');
            let cursor_offset = if is_directory && has_trailing_quote {
                item.value.len().saturating_sub(1)
            } else {
                item.value.len()
            };
            return CompletionResult {
                lines: new_lines,
                cursor_line,
                cursor_col: before_prefix.len() + cursor_offset,
            };
        }

        // Check if we're in a slash command context (before_prefix contains "/command ")
        if text_before_cursor(lines, cursor_line, cursor_col).contains('/') &&
           text_before_cursor(lines, cursor_line, cursor_col).contains(' ') {
            let new_line = format!("{}{}{}", before_prefix, item.value, adjusted_after);
            let mut new_lines = lines.to_vec();
            new_lines[cursor_line] = new_line;
            let is_directory = item.label.ends_with('/');
            let has_trailing_quote = item.value.ends_with('"');
            let cursor_offset = if is_directory && has_trailing_quote {
                item.value.len().saturating_sub(1)
            } else {
                item.value.len()
            };
            return CompletionResult {
                lines: new_lines,
                cursor_line,
                cursor_col: before_prefix.len() + cursor_offset,
            };
        }

        // For file paths, complete the path
        let new_line = format!("{}{}{}", before_prefix, item.value, adjusted_after);
        let mut new_lines = lines.to_vec();
        new_lines[cursor_line] = new_line;
        let is_directory = item.label.ends_with('/');
        let has_trailing_quote = item.value.ends_with('"');
        let cursor_offset = if is_directory && has_trailing_quote {
            item.value.len().saturating_sub(1)
        } else {
            item.value.len()
        };
        CompletionResult {
            lines: new_lines,
            cursor_line,
            cursor_col: before_prefix.len() + cursor_offset,
        }
    }

    fn should_trigger_file_completion(
        &self,
        lines: &[String],
        cursor_line: usize,
        cursor_col: usize,
    ) -> bool {
        let current_line = lines.get(cursor_line).map(|s| s.as_str()).unwrap_or("");
        let text_before_cursor = &current_line[..cursor_col.min(current_line.len())];

        // Don't trigger if we're typing a slash command at the start of the line
        if text_before_cursor.trim().starts_with('/') && !text_before_cursor.trim().contains(' ') {
            return false;
        }
        true
    }
}

// ── Private helper methods (implemented as free functions and inherent impl) ──

fn text_before_cursor(lines: &[String], cursor_line: usize, cursor_col: usize) -> String {
    lines.get(cursor_line)
        .map(|s| s[..cursor_col.min(s.len())].to_string())
        .unwrap_or_default()
}

impl CombinedAutocompleteProvider {
    fn extract_at_prefix(&self, text: &str) -> Option<String> {
        // Check for quoted @"..." or "..." prefix
        let quoted = extract_quoted_prefix(text);
        if let Some(ref q) = quoted {
            if q.starts_with('@') && q.len() > 1 && q.as_bytes().get(1) == Some(&b'"') {
                return Some(q.clone());
            }
        }

        // Check for @ at token boundary
        let last_delim = find_last_delimiter(text);
        let token_start = if last_delim == -1 { 0 } else { (last_delim + 1) as usize };

        if token_start < text.len() && text.as_bytes().get(token_start) == Some(&b'@') {
            return Some(text[token_start..].to_string());
        }

        None
    }

    fn extract_path_prefix(&self, text: &str, force_extract: bool) -> Option<String> {
        // Check for quoted prefix first
        if let Some(quoted) = extract_quoted_prefix(text) {
            return Some(quoted);
        }

        let last_delim = find_last_delimiter(text);
        let path_prefix = if last_delim == -1 {
            text.to_string()
        } else {
            text[(last_delim + 1) as usize..].to_string()
        };

        // For forced extraction (Tab key), always return something
        if force_extract {
            return Some(path_prefix);
        }

        // For natural triggers, return if it looks like a path
        if path_prefix.contains('/') || path_prefix.starts_with('.') || path_prefix.starts_with("~/") {
            return Some(path_prefix);
        }

        // Return empty string only after a space (not for completely empty text)
        if path_prefix.is_empty() && text.ends_with(' ') {
            return Some(path_prefix);
        }

        None
    }

    fn resolve_scoped_fuzzy_query(&self, raw_query: &str) -> Option<(String, String, String)> {
        let normalized = to_display_path(raw_query);
        let slash_idx = normalized.rfind('/')?;

        let display_base = normalized[..=slash_idx].to_string();
        let query = normalized[slash_idx + 1..].to_string();

        let base_dir = if display_base.starts_with("~/") {
            let home = std::env::var("HOME").unwrap_or_default();
            Path::new(&home).join(&display_base[2..]).to_string_lossy().to_string()
        } else if display_base.starts_with('/') {
            display_base.clone()
        } else {
            Path::new(&self.base_path).join(&display_base).to_string_lossy().to_string()
        };

        if Path::new(&base_dir).is_dir() {
            Some((base_dir, query, display_base))
        } else {
            None
        }
    }

    fn scoped_path_for_display(display_base: &str, relative_path: &str) -> String {
        let normalized = to_display_path(relative_path);
        if display_base == "/" {
            format!("/{}", normalized)
        } else {
            format!("{}{}", to_display_path(display_base), normalized)
        }
    }

    fn get_file_suggestions(&self, prefix: &str) -> Vec<AutocompleteItem> {
        let homedir_str = std::env::var("HOME").unwrap_or_default();
        let (raw_prefix, is_at_prefix, is_quoted_prefix) = parse_path_prefix(prefix);
        let mut expanded_prefix = raw_prefix.clone();

        // Handle home directory expansion
        if expanded_prefix.starts_with('~') {
            expanded_prefix = expand_home_path(&expanded_prefix, &homedir_str);
        }

        let is_root_prefix = raw_prefix.is_empty()
            || raw_prefix == "./"
            || raw_prefix == "../"
            || raw_prefix == "~"
            || raw_prefix == "~/"
            || raw_prefix == "/"
            || (is_at_prefix && raw_prefix.is_empty());

        let (search_dir, search_prefix) = if is_root_prefix {
            let dir = if raw_prefix.starts_with('~') || expanded_prefix.starts_with('/') {
                expanded_prefix.clone()
            } else {
                Path::new(&self.base_path).join(&expanded_prefix).to_string_lossy().to_string()
            };
            (dir, String::new())
        } else if raw_prefix.ends_with('/') {
            let dir = if raw_prefix.starts_with('~') || expanded_prefix.starts_with('/') {
                expanded_prefix.clone()
            } else {
                Path::new(&self.base_path).join(&expanded_prefix).to_string_lossy().to_string()
            };
            (dir, String::new())
        } else {
            let p = Path::new(&expanded_prefix);
            let dir = p.parent().map(|d| d.to_string_lossy().to_string()).unwrap_or_default();
            let file = p.file_name().and_then(|n| n.to_str()).map(|s| s.to_string()).unwrap_or_default();
            let dir = if raw_prefix.starts_with('~') || expanded_prefix.starts_with('/') {
                if dir.is_empty() { expanded_prefix.clone() } else { dir }
            } else {
                if dir.is_empty() {
                    self.base_path.clone()
                } else {
                    Path::new(&self.base_path).join(&dir).to_string_lossy().to_string()
                }
            };
            (dir, file)
        };

        let entries = match std::fs::read_dir(&search_dir) {
            Ok(e) => e,
            Err(_) => return Vec::new(),
        };

        let mut suggestions: Vec<AutocompleteItem> = Vec::new();
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if !name.to_lowercase().starts_with(&search_prefix.to_lowercase()) {
                continue;
            }

            // Check if entry is a directory (or a symlink pointing to a directory)
            let is_directory = entry.file_type().map(|t| t.is_dir()).unwrap_or(false)
                || (entry.file_type().map(|t| t.is_symlink()).unwrap_or(false)
                    && std::fs::metadata(entry.path()).map(|m| m.is_dir()).unwrap_or(false));

            let relative_path: String = {
                let dp = &raw_prefix;
                if dp.ends_with('/') {
                    format!("{}{}", dp, name)
                } else if dp.contains('/') || dp.contains('\\') {
                    if dp.starts_with("~/") {
                        let home_relative = &dp[2..];
                        let dir_part = Path::new(home_relative).parent().and_then(|d| d.to_str()).unwrap_or("");
                        if dir_part.is_empty() || dir_part == "." {
                            format!("~/{}{}", name, if is_directory { "/" } else { "" })
                        } else {
                            format!("~/{}/{}{}", dir_part, name, if is_directory { "/" } else { "" })
                        }
                    } else if dp.starts_with('/') {
                        let p = Path::new(dp);
                        let dir_part = p.parent().and_then(|d| d.to_str()).unwrap_or("");
                        if dir_part == "/" || dir_part.is_empty() {
                            format!("/{}{}", name, if is_directory { "/" } else { "" })
                        } else {
                            format!("{}/{}{}", dir_part, name, if is_directory { "/" } else { "" })
                        }
                    } else {
                        let p = Path::new(dp);
                        let dir_part = p.parent().and_then(|d| d.to_str()).unwrap_or("");
                        let mut rp = if dir_part.is_empty() {
                            name.clone()
                        } else {
                            format!("{}/{}", dir_part, name)
                        };
                        // Preserve ./ prefix
                        if dp.starts_with("./") && !rp.starts_with("./") {
                            rp = format!("./{}", rp);
                        }
                        if is_directory {
                            rp.push('/');
                        }
                        rp
                    }
                } else {
                    if dp.starts_with('~') {
                        format!("~/{}{}", name, if is_directory { "/" } else { "" })
                    } else {
                        format!("{}{}", name, if is_directory { "/" } else { "" })
                    }
                }
            };

            let relative_path = to_display_path(&relative_path);
            let path_value = if is_directory {
                format!("{}/", relative_path.trim_end_matches('/'))
            } else {
                relative_path.clone()
            };
            let value = build_completion_value(&path_value, is_directory, is_at_prefix, is_quoted_prefix);

            suggestions.push(AutocompleteItem {
                value,
                label: if is_directory { format!("{}/", name) } else { name },
                description: None,
            });
        }

        // Sort directories first, then alphabetically
        suggestions.sort_by(|a, b| {
            let a_dir = a.label.ends_with('/');
            let b_dir = b.label.ends_with('/');
            if a_dir && !b_dir {
                return std::cmp::Ordering::Less;
            }
            if !a_dir && b_dir {
                return std::cmp::Ordering::Greater;
            }
            a.label.to_lowercase().cmp(&b.label.to_lowercase())
        });

        suggestions
    }

    fn score_entry(file_path: &str, query: &str, is_directory: bool) -> i32 {
        let file_name = Path::new(file_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(file_path);
        let lower_file = file_name.to_lowercase();
        let lower_query = query.to_lowercase();

        let mut score = if lower_file == lower_query {
            100
        } else if lower_file.starts_with(&lower_query) {
            80
        } else if lower_file.contains(&lower_query) {
            50
        } else if file_path.to_lowercase().contains(&lower_query) {
            30
        } else {
            0
        };

        if is_directory && score > 0 {
            score += 10;
        }
        score
    }

    fn get_fuzzy_file_suggestions(&self, query: &str, is_quoted_prefix: bool) -> Vec<AutocompleteItem> {
        let fd_path = match self.fd_path {
            Some(ref p) => p.clone(),
            None => return Vec::new(),
        };

        let scoped = self.resolve_scoped_fuzzy_query(query);
        let (fd_base_dir, fd_query) = match &scoped {
            Some((base, q, _)) => (base.clone(), q.clone()),
            None => (self.base_path.clone(), query.to_string()),
        };

        let _raw_query = match &scoped {
            Some((_, _, db)) => {
                // Reconstruct original query for matching
                let q_part = query.strip_prefix(db.as_str()).unwrap_or(query);
                if !q_part.is_empty() { q_part.to_string() } else { fd_query.clone() }
            }
            None => query.to_string(),
        };

        let entries = walk_directory_with_fd(&fd_base_dir, &fd_path, &fd_query, 100);
        if entries.is_empty() {
            return Vec::new();
        }

        let mut scored: Vec<(i32, String, bool)> = entries
            .into_iter()
            .map(|(path, is_dir)| {
                let score = if fd_query.is_empty() {
                    1
                } else {
                    Self::score_entry(&path, &fd_query, is_dir)
                };
                (score, path, is_dir)
            })
            .filter(|(score, _, _)| *score > 0)
            .collect();

        scored.sort_by(|a, b| b.0.cmp(&a.0));
        let top = scored.into_iter().take(20);

        let mut suggestions = Vec::new();
        for (_, entry_path, is_directory) in top {
            let path_no_slash = if is_directory && entry_path.ends_with('/') {
                entry_path[..entry_path.len() - 1].to_string()
            } else {
                entry_path.clone()
            };
            let display_path = match &scoped {
                Some((_, _, db)) => Self::scoped_path_for_display(db, &path_no_slash),
                None => path_no_slash.clone(),
            };
            let entry_name = Path::new(&path_no_slash)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(&path_no_slash)
                .to_string();
            let completion_path = if is_directory {
                format!("{}/", display_path)
            } else {
                display_path.clone()
            };
            let value = build_completion_value(&completion_path, is_directory, true, is_quoted_prefix);

            suggestions.push(AutocompleteItem {
                value,
                label: if is_directory { format!("{}/", entry_name) } else { entry_name },
                description: Some(display_path),
            });
        }
        suggestions
    }

    /// Legacy API for tests — get suggestions from flat text + cursor position.
    #[allow(dead_code)]
    pub fn get_suggestions_legacy(&self, text: &str, cursor_pos: usize) -> Option<AutocompleteSuggestions> {
        let lines: Vec<String> = text.lines().map(|l| l.to_string()).collect();
        let cursor_line = 0;
        let cursor_col = cursor_pos;
        self.get_suggestions(&lines, cursor_line, cursor_col, false)
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_last_delimiter() {
        assert_eq!(find_last_delimiter("hello world"), 5);
        assert_eq!(find_last_delimiter("hello"), -1);
        assert_eq!(find_last_delimiter(""), -1);
    }

    #[test]
    fn test_find_unclosed_quote_start() {
        assert_eq!(find_unclosed_quote_start(r#"hello "world"#), Some(6));
        assert_eq!(find_unclosed_quote_start(r#"hello "world" "#), None);
    }

    #[test]
    fn test_is_token_start() {
        assert!(is_token_start("@file", 0));
        assert!(is_token_start("hello @file", 6));
        assert!(!is_token_start("hello@file", 5));
    }

    #[test]
    fn test_parse_path_prefix() {
        let (raw, at, quoted) = parse_path_prefix("@\"src/");
        assert_eq!(raw, "src/");
        assert!(at);
        assert!(quoted);

        let (raw, at, quoted) = parse_path_prefix("@src");
        assert_eq!(raw, "src");
        assert!(at);
        assert!(!quoted);

        let (raw, at, quoted) = parse_path_prefix("\"src/");
        assert_eq!(raw, "src/");
        assert!(!at);
        assert!(quoted);
    }

    #[test]
    fn test_build_completion_value() {
        assert_eq!(build_completion_value("file.txt", false, false, false), "file.txt");
        assert_eq!(build_completion_value("file.txt", false, true, false), "@file.txt");
        assert_eq!(build_completion_value("my file.txt", false, false, true), "\"my file.txt\"");
        assert_eq!(build_completion_value("my file.txt", false, true, true), "@\"my file.txt\"");
    }

    #[test]
    fn test_expand_home_path() {
        let home = "/home/user";
        assert_eq!(expand_home_path("~/docs", home), "/home/user/docs");
        assert_eq!(expand_home_path("~/", home), "/home/user/");
        assert_eq!(expand_home_path("~", home), "/home/user");
        assert_eq!(expand_home_path("/abs/path", home), "/abs/path");
    }
}
