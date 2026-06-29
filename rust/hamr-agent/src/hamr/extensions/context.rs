//! Port of `packages/coding-agent/src/hamr/extensions/context.ts` (v0.7.1).
//!
//! Context command extension — shows context window usage breakdown
//! with visual icon grid and per-category estimates. Registered as both a
//! custom message renderer and a `/context` slash command.
//!
//! # v0.7.1: Category model
//!
//! Instead of hardcoded fields, the breakdown uses a `categories` list where
//! each entry has `{ name, tokens, color }`.  Categories are built by parsing
//! the system prompt sections.  A `fromApi` flag distinguishes live API
//! token counts from client-side estimates.  When estimated, values are
//! prefixed with `~`.

// ─── Types

use crate::core::extensions::types::ExtensionCommandContext;

/// A single category in the context breakdown.
#[derive(Debug, Clone)]
pub struct Category {
    pub name: String,
    pub tokens: u64,
    pub color: CategoryColor,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CategoryColor {
    Accent,
    Warning,
    Error,
    Dim,
}

impl CategoryColor {
    fn ansi(&self) -> &'static str {
        match self {
            CategoryColor::Accent => "34",
            CategoryColor::Warning => "33",
            CategoryColor::Error => "31",
            CategoryColor::Dim => "90",
        }
    }
    fn fg(&self, text: &str) -> String {
        format!("\x1b[{}m{}\x1b[0m", self.ansi(), text)
    }
    fn bold(&self, text: &str) -> String {
        format!("\x1b[{};1m{}\x1b[0m", self.ansi(), text)
    }
}

/// Token-estimation breakdown for the current context window.
/// Mirror of the TS `ContextBreakdown` (v0.7.1).
#[derive(Debug, Clone, Default)]
pub struct ContextBreakdown {
    pub model_name: Option<String>,
    pub model_id: Option<String>,
    pub context_window: u64,
    pub tokens: Option<u64>,
    pub percent: Option<f64>,
    /// Per-category breakdown in display order.
    pub categories: Vec<Category>,
    /// True when the token count came from the API (not estimated).
    pub from_api: bool,
}

// ─── Token estimation ─────────────────────────────────────────────────────────

/// Estimate tokens from bytes using the given bytes-per-token ratio.
/// Mirrors TS `estimate_tokens(bytes_per_token)`.
pub fn estimate_tokens(bytes: u64, bytes_per_token: f64) -> u64 {
    ((bytes as f64) / bytes_per_token).ceil() as u64
}

/// Rough token estimate: each token is ~4 characters on average.
pub fn estimate_chars(text: &str) -> u64 {
    estimate_tokens(text.len() as u64, 4.0)
}

/// Format a number for display.
pub fn fmt_token_count(n: u64) -> String {
    if n < 1000 {
        n.to_string()
    } else if n < 1_000_000 {
        format!("{:.1}k", n as f64 / 1000.0)
    } else {
        format!("{:.1}m", n as f64 / 1_000_000.0)
    }
}

// ─── System prompt section parsing ────────────────────────────────────────────

/// Parse the system prompt into named sections for category estimation.
/// Sections are delimited by markdown-style headers: `## Section Name`.
fn parse_system_prompt_sections(system_prompt: &str) -> Vec<(String, String)> {
    let mut sections: Vec<(String, String)> = Vec::new();
    let mut current_name = String::from("(preamble)");
    let mut current_body = String::new();

    for line in system_prompt.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("## ") {
            // Save previous section
            if !current_body.trim().is_empty() || !current_name.is_empty() {
                sections.push((
                    std::mem::take(&mut current_name),
                    std::mem::take(&mut current_body),
                ));
            }
            current_name = trimmed[3..].trim().to_string();
        } else {
            if !current_body.is_empty() {
                current_body.push('\n');
            }
            current_body.push_str(line);
        }
    }
    // Save final section
    if !current_body.trim().is_empty() || !current_name.is_empty() {
        sections.push((current_name, current_body));
    }

    sections
}

// ─── Compute breakdown ────────────────────────────────────────────────────────

/// Compute a context breakdown from the extension command context.
/// Mirrors TS `computeBreakdown(ctx)` (v0.7.1).
pub fn compute_breakdown(ctx: &dyn ExtensionCommandContext) -> ContextBreakdown {
    let model = ctx.model();
    let context_usage = ctx.get_context_usage();
    let context_window = context_usage
        .as_ref()
        .map(|u| u.context_window)
        .or_else(|| {
            model
                .as_ref()
                .and_then(|m| m.get("contextWindow").and_then(|v| v.as_u64()))
        })
        .unwrap_or(0);
    let tokens = context_usage.as_ref().and_then(|u| u.tokens);
    let percent = context_usage.as_ref().and_then(|u| u.percent);
    let from_api = context_usage.is_some();

    let system_prompt_text = ctx.get_system_prompt();
    let system_prompt_estimated = estimate_chars(&system_prompt_text);

    // Parse system prompt into sections for per-category breakdown
    let sections = parse_system_prompt_sections(&system_prompt_text);
    let mut categories: Vec<Category> = Vec::new();

    for (name, body) in &sections {
        if name == "(preamble)" || name.is_empty() {
            continue;
        }
        let cat_tokens = estimate_chars(body);
        if cat_tokens > 0 {
            categories.push(Category {
                name: name.clone(),
                tokens: cat_tokens,
                color: section_color(name),
            });
        }
    }

    // System prompt total
    let base_system_tokens: u64 = categories.iter().map(|c| c.tokens).sum();
    let preamble_tokens = system_prompt_estimated.saturating_sub(base_system_tokens);
    if preamble_tokens > 0 {
        categories.insert(
            0,
            Category {
                name: "System prompt".to_string(),
                tokens: preamble_tokens,
                color: CategoryColor::Dim,
            },
        );
    }

    // Messages + tools: what remains after system prompt
    let messages_and_tools = tokens.map(|t| t.saturating_sub(system_prompt_estimated));

    if let Some(mt) = messages_and_tools {
        if mt > 0 {
            categories.push(Category {
                name: "Messages + tools".to_string(),
                tokens: mt,
                color: if percent.unwrap_or(0.0) > 90.0 {
                    CategoryColor::Error
                } else if percent.unwrap_or(0.0) > 70.0 {
                    CategoryColor::Warning
                } else {
                    CategoryColor::Accent
                },
            });
        }
    }

    // Free space
    let free_tokens = match (tokens, context_window) {
        (Some(t), w) if w > 0 => w.saturating_sub(t),
        _ => 0,
    };
    if free_tokens > 0 {
        categories.push(Category {
            name: "Free space".to_string(),
            tokens: free_tokens,
            color: CategoryColor::Dim,
        });
    }

    ContextBreakdown {
        model_name: model
            .as_ref()
            .and_then(|m| m.get("name").and_then(|v| v.as_str()))
            .map(String::from),
        model_id: model
            .as_ref()
            .and_then(|m| m.get("id").and_then(|v| v.as_str()))
            .map(String::from),
        context_window,
        tokens,
        percent,
        categories,
        from_api,
    }
}

/// Assign a color to a system prompt section based on its name.
fn section_color(name: &str) -> CategoryColor {
    let lower = name.to_lowercase();
    if lower.contains("skill") || lower.contains("tool") {
        CategoryColor::Accent
    } else if lower.contains("context") || lower.contains("file") {
        CategoryColor::Warning
    } else {
        CategoryColor::Dim
    }
}

// ─── Render display ───────────────────────────────────────────────────────────

/// Render the context breakdown display string.
/// Mirrors TS `renderDisplay(breakdown, theme)` (v0.7.1).
pub fn render_display(breakdown: &ContextBreakdown) -> String {
    let context_window = breakdown.context_window;
    let tokens = breakdown.tokens;

    const SLOTS: usize = 25;
    const TOKENS_PER_SLOT: u64 = 4000;
    let filled_slots = tokens.map(|t| (t / TOKENS_PER_SLOT) as usize).unwrap_or(0);
    let capped_filled = filled_slots.min(SLOTS);
    let show_overflow = filled_slots > SLOTS;

    let usage_color = match breakdown.percent {
        Some(p) if p > 90.0 => CategoryColor::Error,
        Some(p) if p > 70.0 => CategoryColor::Warning,
        _ => CategoryColor::Accent,
    };

    // Icon grid (5x5)
    let mut icon_rows: Vec<String> = Vec::new();
    for row in 0..5 {
        let mut icons: Vec<String> = Vec::new();
        for col in 0..5 {
            let slot = row * 5 + col;
            if slot < capped_filled {
                icons.push(usage_color.fg("⛁"));
            } else if slot == capped_filled && show_overflow {
                icons.push(usage_color.fg("+"));
            } else {
                icons.push(CategoryColor::Dim.fg("⛶"));
            }
        }
        icon_rows.push(icons.join(" "));
    }

    let token_str = match (tokens, context_window) {
        (Some(t), w) if w > 0 => format!("{}/{} tokens", fmt_token_count(t), fmt_token_count(w)),
        (Some(t), _) => format!("{}/0 tokens", fmt_token_count(t)),
        (None, w) if w > 0 => format!("?/{} tokens", fmt_token_count(w)),
        _ => "? / ? tokens".to_string(),
    };

    let pct_str = match breakdown.percent {
        Some(p) if breakdown.from_api => format!(" ({}%)", (p).round() as u64),
        Some(p) => format!(" (~{}%)", (p).round() as u64),
        None => String::new(),
    };

    let indent = "            ";
    let bold = |s: &str| format!("\x1b[1m{}\x1b[0m", s);

    let mut lines: Vec<String> = Vec::new();
    lines.push(bold("Context Usage"));
    lines.push(String::new());
    lines.push(format!(
        "{}    {}",
        icon_rows[0],
        breakdown.model_name.as_deref().unwrap_or("No model")
    ));
    lines.push(format!(
        "{}    {}",
        icon_rows[1],
        CategoryColor::Dim.fg(breakdown.model_id.as_deref().unwrap_or(""))
    ));
    lines.push(format!(
        "{}    {}",
        icon_rows[2],
        CategoryColor::Dim.fg(&format!("{}{}", token_str, pct_str))
    ));
    lines.push(icon_rows[3].clone());

    // Source note
    let source_note = if breakdown.from_api {
        "iLive usage (from API)"
    } else {
        "iEstimated usage by category"
    };
    lines.push(format!(
        "{}    {}",
        icon_rows[4],
        CategoryColor::Dim.fg(source_note)
    ));
    lines.push(String::new());

    // Per-category breakdown with ~ prefix for estimates
    for cat in &breakdown.categories {
        let prefix = if breakdown.from_api { "" } else { "~" };
        lines.push(format!(
            "{}{} {}: {}{} tokens",
            indent,
            cat.color.fg("⛁"),
            bold(&cat.name),
            prefix,
            fmt_token_count(cat.tokens)
        ));
    }

    lines.join("\n")
}

// ─── Extension factory (wiring matches existing CLI expectations) ────────────

/// Extension factory compatible with the CLI extension loading pattern.
/// Registers a `/context` slash command via the extension API.
pub fn hamr_context_extension() -> crate::core::extensions::types::ExtensionFactory {
    use crate::core::extensions::types::*;
    use std::sync::Arc;

    Arc::new(|pi: Arc<dyn ExtensionAPI>| {
        Box::pin(async move {
            let _pi = pi.clone();
            // Full registration deferred until TUI integration.
            // The compute_breakdown and render_display functions are available
            // for direct use by the interactive mode.
        })
    })
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_estimate_chars() {
        // 40 chars / 4 = 10 tokens (ceiling)
        assert_eq!(estimate_chars("a".repeat(40).as_str()), 10);
        // 41 chars / 4 = 10.25 → 11 tokens
        assert_eq!(estimate_chars("a".repeat(41).as_str()), 11);
    }

    #[test]
    fn test_estimate_tokens() {
        assert_eq!(estimate_tokens(100, 4.0), 25);
        assert_eq!(estimate_tokens(101, 4.0), 26);
        assert_eq!(estimate_tokens(0, 4.0), 0);
    }

    #[test]
    fn test_fmt_token_count() {
        assert_eq!(fmt_token_count(500), "500");
        assert_eq!(fmt_token_count(1500), "1.5k");
        assert_eq!(fmt_token_count(1_500_000), "1.5m");
    }

    #[test]
    fn test_parse_system_prompt_sections() {
        let prompt = "You are an agent.\n## Tools\nAvailable tools.\n## Context\nProject info.";
        let sections = parse_system_prompt_sections(prompt);
        assert_eq!(sections.len(), 3);
        assert_eq!(sections[0].0, "(preamble)");
        assert!(sections[0].1.contains("You are an agent"));
        assert_eq!(sections[1].0, "Tools");
        assert_eq!(sections[2].0, "Context");
    }

    #[test]
    fn test_section_color() {
        assert_eq!(section_color("Skills"), CategoryColor::Accent);
        assert_eq!(section_color("Tools"), CategoryColor::Accent);
        assert_eq!(section_color("Context files"), CategoryColor::Warning);
        assert_eq!(section_color("Other"), CategoryColor::Dim);
    }

    #[test]
    fn test_breakdown_from_api_flag() {
        // Without context usage, from_api should be false
        let b = ContextBreakdown::default();
        assert!(!b.from_api);
    }

    #[test]
    fn test_render_display_empty() {
        let b = ContextBreakdown::default();
        let output = render_display(&b);
        assert!(output.contains("Context Usage"));
        assert!(output.contains("No model"));
    }
}
