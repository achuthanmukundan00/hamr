//! Width-aware diff component matching "Claude Code" presentation.
//!
//! Port of `packages/coding-agent/src/modes/interactive/components/diff.ts`.
//!
//! Code is always syntax-highlighted on a neutral base, and additions/removals
//! are conveyed by a full-width background band (green/red) rather than by
//! recoloring the code itself.

use crate::modes::interactive::components::tui_shim::{Component, visible_width};
use crate::modes::interactive::theme::theme::{
    ThemeBg, get_language_from_path, highlight_code, theme,
};

/// A single logical diff row, normalized from either the internal edit-diff
/// format or a raw git unified diff.
#[derive(Debug, Clone)]
struct DiffRow {
    kind: DiffKind,
    /// Display line number (already stringified), or "" for meta/separator rows.
    line_num: String,
    content: String,
    /// Syntax-highlighting language for this row's content (per-file in multi-file diffs).
    lang: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum DiffKind {
    Added,
    Removed,
    Context,
    Meta,
}

/// Replace tabs with spaces for consistent column rendering.
fn replace_tabs(text: &str) -> String {
    text.replace('\t', "   ")
}

fn truncate_for_band(text: &str, width: u16, ellipsis: &str) -> String {
    if visible_width(text) <= width {
        return text.to_string();
    }
    let ellipsis_width = visible_width(ellipsis);
    if ellipsis_width >= width {
        let clipped = truncate_str(ellipsis, width, "");
        if clipped.ends_with("\x1b[0m") {
            return clipped[..clipped.len() - 4].to_string();
        }
        return clipped;
    }
    let effective = width.saturating_sub(ellipsis_width);
    let clipped = truncate_str(text, effective, "");
    let without_reset = if clipped.ends_with("\x1b[0m") {
        clipped[..clipped.len() - 4].to_string()
    } else {
        clipped
    };
    format!("{}{}", without_reset, ellipsis)
}

fn truncate_str(s: &str, width: u16, _ellipsis: &str) -> String {
    let mut out = String::new();
    let mut in_escape = false;
    let mut visible = 0u16;
    for ch in s.chars() {
        if in_escape {
            out.push(ch);
            if ch.is_ascii_alphabetic() && ch != '[' && ch != ';' {
                in_escape = false;
            }
        } else if ch == '\x1b' {
            in_escape = true;
            out.push(ch);
        } else {
            if visible >= width {
                break;
            }
            visible += 1;
            out.push(ch);
        }
    }
    out
}

/// Options for rendering a diff.
pub struct RenderDiffOptions {
    /// File path used to choose a syntax-highlighting language.
    pub file_path: Option<String>,
    /// Treat input as a raw git/unified diff rather than the internal format.
    pub unified: bool,
    /// Background the diff is painted onto.
    pub surround_bg: Option<ThemeBg>,
}

impl Default for RenderDiffOptions {
    fn default() -> Self {
        Self {
            file_path: None,
            unified: false,
            surround_bg: None,
        }
    }
}

/// Parse the internal edit-diff format emitted by `generateDiffString`.
/// Lines look like: "+123 content", "-123 content", " 123 content", "     ...".
fn parse_generated_diff(diff_text: &str) -> Vec<DiffRow> {
    let mut rows = Vec::new();
    for line in diff_text.lines() {
        // Match pattern: optional prefix (+,-,space) + optional space-padded number + space + content
        let chars: Vec<char> = line.chars().collect();
        if chars.is_empty() {
            rows.push(DiffRow {
                kind: DiffKind::Meta,
                line_num: String::new(),
                content: line.to_string(),
                lang: None,
            });
            continue;
        }
        let prefix = chars[0];
        if prefix != '+' && prefix != '-' && prefix != ' ' {
            rows.push(DiffRow {
                kind: DiffKind::Meta,
                line_num: String::new(),
                content: line.to_string(),
                lang: None,
            });
            continue;
        }

        // Try to extract number and content
        let rest: String = chars[1..].iter().collect();
        if let Some(space_idx) = rest.find(' ') {
            let num_part = rest[..space_idx].trim();
            let content = rest[space_idx + 1..].to_string();

            if content.trim() == "..." && num_part.is_empty() {
                rows.push(DiffRow {
                    kind: DiffKind::Meta,
                    line_num: String::new(),
                    content: "⋯".to_string(),
                    lang: None,
                });
                continue;
            }

            let kind = match prefix {
                '+' => DiffKind::Added,
                '-' => DiffKind::Removed,
                _ => DiffKind::Context,
            };
            rows.push(DiffRow {
                kind,
                line_num: num_part.to_string(),
                content,
                lang: None,
            });
        } else {
            rows.push(DiffRow {
                kind: DiffKind::Meta,
                line_num: String::new(),
                content: line.to_string(),
                lang: None,
            });
        }
    }
    rows
}

/// Detect whether a blob of text is a raw git/unified diff.
pub fn looks_like_unified_diff(text: &str) -> bool {
    text.contains("@@ -")
}

/// Parse a raw git unified diff into normalized rows.
fn parse_unified_diff(diff_text: &str) -> Vec<DiffRow> {
    let mut rows: Vec<DiffRow> = Vec::new();
    let mut old_line: i64 = 0;
    let mut new_line: i64 = 0;
    let mut lang: Option<String> = None;

    for line in diff_text.lines() {
        // Track the current file (and its language) as we cross file boundaries.
        if let Some(captured) = line.strip_prefix("+++ b/") {
            lang = get_language_from_path(captured);
            continue;
        }
        if line.starts_with("diff --git a/") {
            if let Some(after) = line.strip_prefix("diff --git a/") {
                if let Some(idx) = after.find(' ') {
                    let path = &after[..idx];
                    if path.ends_with(" b/") {
                        lang = get_language_from_path(&path[..path.len() - 3]);
                    }
                }
            }
            continue;
        }

        // Parse hunk header: @@ -old_start[,old_count] +new_start[,new_count] @@ [context]
        if line.starts_with("@@ -") {
            if let Some(at_idx) = line[4..].find(" +") {
                let old_str = &line[4..4 + at_idx];
                let after_plus = &line[5 + at_idx..];
                let new_str = if let Some(space_idx) = after_plus.find(' ') {
                    &after_plus[..space_idx]
                } else if let Some(at_idx2) = after_plus.find(" @@") {
                    &after_plus[..at_idx2]
                } else {
                    after_plus
                };

                old_line = old_str
                    .split(',')
                    .next()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0);
                new_line = new_str
                    .split(',')
                    .next()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0);

                if !rows.is_empty() {
                    rows.push(DiffRow {
                        kind: DiffKind::Meta,
                        line_num: String::new(),
                        content: "⋯".to_string(),
                        lang: lang.clone(),
                    });
                }
            }
            continue;
        }

        // Skip remaining file-level headers
        if line.starts_with("index ")
            || line.starts_with("--- ")
            || line.starts_with("new file mode ")
            || line.starts_with("deleted file mode ")
            || line.starts_with("similarity ")
            || line.starts_with("rename ")
            || line.starts_with("old mode ")
            || line.starts_with("new mode ")
        {
            continue;
        }

        if let Some(content) = line.strip_prefix('+') {
            rows.push(DiffRow {
                kind: DiffKind::Added,
                line_num: new_line.to_string(),
                content: content.to_string(),
                lang: lang.clone(),
            });
            new_line += 1;
        } else if let Some(content) = line.strip_prefix('-') {
            rows.push(DiffRow {
                kind: DiffKind::Removed,
                line_num: old_line.to_string(),
                content: content.to_string(),
                lang: lang.clone(),
            });
            old_line += 1;
        } else if line.starts_with('\\') {
            // "\ No newline at end of file" — ignore
        } else {
            // Context line
            let content = if let Some(rest) = line.strip_prefix(' ') {
                rest.to_string()
            } else {
                line.to_string()
            };
            rows.push(DiffRow {
                kind: DiffKind::Context,
                line_num: new_line.to_string(),
                content,
                lang: lang.clone(),
            });
            old_line += 1;
            new_line += 1;
        }
    }

    rows
}

/// Width-aware diff component.
pub struct DiffComponent {
    rows: Vec<DiffRow>,
    lang: Option<String>,
    num_width: usize,
    surround_bg: Option<ThemeBg>,
    cache_width: Option<u16>,
    cache_lines: Option<Vec<String>>,
}

impl DiffComponent {
    pub fn new(diff_text: &str, options: RenderDiffOptions) -> Self {
        let lang = options
            .file_path
            .as_ref()
            .and_then(|p| get_language_from_path(p));
        let rows = if options.unified {
            parse_unified_diff(diff_text)
        } else {
            parse_generated_diff(diff_text)
        };
        let num_width = rows.iter().map(|r| r.line_num.len()).max().unwrap_or(1);

        Self {
            rows,
            lang,
            num_width,
            surround_bg: options.surround_bg,
            cache_width: None,
            cache_lines: None,
        }
    }

    fn highlight_content(&self, content: &str, lang: Option<&str>) -> String {
        let text = replace_tabs(content);
        if text.is_empty() {
            return String::new();
        }
        match lang {
            Some(l) => {
                let highlighted = highlight_code(&text, l);
                highlighted.into_iter().next().unwrap_or(text)
            }
            None => text,
        }
    }

    fn render_row(&self, row: &DiffRow, width: u16) -> String {
        if row.kind == DiffKind::Meta {
            let text = theme().fg(
                "toolDiffContext",
                &format!("{}{}", " ".repeat(self.num_width + 2), row.content),
            );
            return crate::modes::interactive::components::tui_shim::truncate_to_width(
                &text, width, "",
            );
        }

        let sign = match row.kind {
            DiffKind::Added => "+",
            DiffKind::Removed => "-",
            _ => " ",
        };
        let sign_color = match row.kind {
            DiffKind::Added => "toolDiffAdded",
            DiffKind::Removed => "toolDiffRemoved",
            _ => "toolDiffContext",
        };

        let padded_num = if row.line_num.is_empty() {
            " ".repeat(self.num_width)
        } else {
            format!("{:>width$}", row.line_num, width = self.num_width)
        };
        let gutter = format!(
            "{}{} ",
            theme().fg(sign_color, sign),
            theme().fg("toolDiffContext", &padded_num)
        );

        let code =
            self.highlight_content(&row.content, row.lang.as_deref().or(self.lang.as_deref()));
        let line = truncate_for_band(&format!("{}{}", gutter, code), width, "...");

        match row.kind {
            DiffKind::Added => self.band(&line, width, "toolDiffAddedBg"),
            DiffKind::Removed => self.band(&line, width, "toolDiffRemovedBg"),
            _ => line,
        }
    }

    /// Paint a full-width background band behind a line.
    fn band(&self, content: &str, width: u16, bg_token: ThemeBg) -> String {
        let vis = visible_width(content);
        let pad = " ".repeat((width.saturating_sub(vis)) as usize);
        let close = self
            .surround_bg
            .map(|bg| theme().get_bg_ansi(bg))
            .unwrap_or_else(|| "\x1b[49m".to_string());
        format!("{}{}{}", theme().get_bg_ansi(bg_token), content, pad) + &close
    }
}

impl Component for DiffComponent {
    fn render(&self, width: u16) -> Vec<String> {
        if self.cache_lines.is_some() && self.cache_width == Some(width) {
            return self.cache_lines.clone().unwrap_or_default();
        }
        // Need to compute fresh — ideally we'd cache, but Component::render is &self.
        // In real integration this uses interior mutability.
        self.rows
            .iter()
            .map(|row| self.render_row(row, width))
            .collect()
    }

    fn invalidate(&mut self) {
        self.cache_width = None;
        self.cache_lines = None;
    }
}

/// Create a width-aware diff component.
pub fn create_diff_component(diff_text: &str, options: RenderDiffOptions) -> DiffComponent {
    DiffComponent::new(diff_text, options)
}
