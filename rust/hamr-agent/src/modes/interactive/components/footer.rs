//! Footer component that shows Hamr's syntax-style single-line status.
//!
//! Port of `packages/coding-agent/src/modes/interactive/components/footer.ts`.
//!
//! Shows activity on the left, context/spend/tokens/provider/model/thinking on the right.

use crate::modes::interactive::components::tui_shim::{
    Component, truncate_to_width, visible_width,
};
use crate::modes::interactive::theme::theme::theme;
use std::collections::HashMap;

/// Stub session type — represents the agent session state for footer rendering.
pub struct FooterAgentSession {
    pub is_streaming: bool,
    pub thinking_level: Option<String>,
    pub state: FooterSessionState,
}

/// Stub for session state used in footer rendering.
pub struct FooterSessionState {
    pub model: Option<FooterModelInfo>,
}

/// Stub model info for footer display.
pub struct FooterModelInfo {
    pub provider: String,
    pub name: Option<String>,
    pub id: String,
    pub reasoning: bool,
    pub context_window: f64,
}

/// Stub for usage stats.
pub struct UsageStats {
    pub total_input: f64,
    pub total_output: f64,
    pub total_cache_read: f64,
    pub total_cache_write: f64,
    pub total_cost: f64,
    pub latest_cache_hit_rate: Option<f64>,
}

/// Stub for context usage info.
pub struct ContextUsage {
    pub context_window: f64,
    pub percent: Option<f64>,
}

/// Stub for footer data provider (extensions).
pub struct FooterDataProvider {
    extension_statuses: HashMap<String, String>,
}

impl FooterDataProvider {
    pub fn new() -> Self {
        Self {
            extension_statuses: HashMap::new(),
        }
    }

    pub fn get_extension_statuses(&self) -> &HashMap<String, String> {
        &self.extension_statuses
    }
}

impl Default for FooterDataProvider {
    fn default() -> Self {
        Self::new()
    }
}

/// Sanitize text for display in a single-line status.
fn sanitize_status_text(text: &str) -> String {
    text.replace('\r', " ")
        .replace('\n', " ")
        .replace('\t', " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Format token count for display: <1K as number, 1K-10K as X.XK, etc.
pub fn format_tokens(count: f64) -> String {
    if count < 1000.0 {
        format!("{}", count as u64)
    } else if count < 10000.0 {
        format!("{:.1}K", count / 1000.0)
    } else if count < 1000000.0 {
        format!("{}K", (count / 1000.0).round() as u64)
    } else if count < 10000000.0 {
        format!("{:.1}M", count / 1000000.0)
    } else {
        format!("{}M", (count / 1000000.0).round() as u64)
    }
}

fn pct(used: f64, total: f64) -> String {
    if total <= 0.0 {
        return "0%".to_string();
    }
    format!("{}%", ((used / total) * 100.0).round() as u64)
}

/// Format the context part of the status bar.
pub fn format_context_part(
    tokens: Option<f64>,
    context_window: f64,
    percent: Option<f64>,
    compact: bool,
) -> Option<String> {
    if context_window <= 0.0 {
        return None;
    }
    match (tokens, percent) {
        (None, _) | (_, None) => Some(if compact {
            format!("? / {}", format_tokens(context_window))
        } else {
            format!("? used of {} tokens", format_tokens(context_window))
        }),
        (Some(t), Some(_)) => Some(if compact {
            format!(
                "{} / {}",
                pct(t, context_window),
                format_tokens(context_window)
            )
        } else {
            format!(
                "{} used of {} tokens",
                pct(t, context_window),
                format_tokens(context_window)
            )
        }),
    }
}

/// Format the accumulated cost for the status bar.
pub fn format_cost_part(
    total_cost: f64,
    input_price_per_million: f64,
    using_subscription: bool,
) -> Option<String> {
    if input_price_per_million <= 0.0 && total_cost <= 0.0 {
        return None;
    }
    let sub = if using_subscription { " (sub)" } else { "" };
    Some(format!("${:.3}{}", total_cost, sub))
}

/// Shorten a path for display in the footer.
pub fn format_cwd_for_footer(cwd: &str, home: Option<&str>) -> String {
    let home = match home {
        Some(h) => h,
        None => return cwd.to_string(),
    };

    // Simple prefix-based shortening (not full canonicalization)
    if cwd.starts_with(home) {
        let rel = &cwd[home.len()..];
        if rel.is_empty() {
            return "~".to_string();
        }
        return format!("~{}", rel);
    }

    cwd.to_string()
}

/// Render the "Working"/"Idle" activity text with optional animation.
fn render_activity_text(session: &FooterAgentSession) -> String {
    let text = if session.is_streaming {
        "Working"
    } else {
        "Idle"
    };

    if !session.is_streaming {
        return theme().fg("dim", text);
    }

    // In full TUI integration, animated shimmer/rainbow effects are applied.
    // For the stub, return a simple bold text.
    if session.is_streaming {
        theme().bold(text)
    } else {
        text.to_string()
    }
}

/// Render the right side of the footer with context, cost, tokens, model info.
fn render_right_side(_session: &FooterAgentSession, width: u16) -> String {
    let _compact = width < 100;
    // In full TUI integration, this renders the right-side stats bar.
    // For the stub, return a placeholder.
    theme().fg("dim", "status-bar")
}

/// The footer component that renders a single-line status bar.
pub struct FooterComponent {
    session: FooterAgentSession,
    footer_data: FooterDataProvider,
}

impl FooterComponent {
    /// Create a new footer component.
    pub fn new(session: FooterAgentSession, footer_data: FooterDataProvider) -> Self {
        Self {
            session,
            footer_data,
        }
    }

    /// Update the session reference.
    pub fn set_session(&mut self, session: FooterAgentSession) {
        self.session = session;
    }

    /// Called when auto-compact is toggled (no-op in stub).
    pub fn set_auto_compact_enabled(&mut self, _enabled: bool) {}

    /// Called to dispose any running timers.
    pub fn dispose(&mut self) {}
}

impl Component for FooterComponent {
    fn render(&self, width: u16) -> Vec<String> {
        let mut lines = Vec::new();

        if width < 40 {
            lines.push(truncate_to_width(
                &render_activity_text(&self.session),
                width,
                &theme().fg("dim", "..."),
            ));
            return lines;
        }

        let left = render_activity_text(&self.session);
        let right = render_right_side(&self.session, width);
        let left_width = visible_width(&left);
        let right_width = visible_width(&right);

        if right_width + 2 >= width {
            lines.push(truncate_to_width(&right, width, &theme().fg("dim", "...")));
        } else if right.is_empty() || left_width + right_width + 2 > width {
            let available_left = width.saturating_sub(right_width).saturating_sub(2);
            let trimmed_left = truncate_to_width(
                &left,
                (available_left as usize).max(1) as u16,
                &theme().fg("dim", "..."),
            );
            let gap = width.saturating_sub(visible_width(&trimmed_left) + right_width);
            lines.push(format!(
                "{}{}{}",
                trimmed_left,
                " ".repeat(gap as usize),
                right
            ));
        } else {
            lines.push(format!(
                "{}{}{}",
                left,
                " ".repeat((width - left_width - right_width) as usize),
                right
            ));
        }

        // Extension status lines
        let extension_statuses = self.footer_data.get_extension_statuses();
        if !extension_statuses.is_empty() {
            let mut sorted: Vec<_> = extension_statuses.iter().collect();
            sorted.sort_by_key(|(k, _)| *k);
            let status_line = sorted
                .iter()
                .map(|(_, text)| sanitize_status_text(text))
                .collect::<Vec<_>>()
                .join(" ");
            lines.push(truncate_to_width(
                &status_line,
                width,
                &theme().fg("dim", "..."),
            ));
        }

        lines
    }

    fn invalidate(&mut self) {
        // Rendered directly from session/provider state
    }
}
