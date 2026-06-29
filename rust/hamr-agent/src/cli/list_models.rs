//! List available models with optional fuzzy search.
//!
//! Port of `packages/coding-agent/src/cli/list-models.ts`.

use hamr_ai::types::Model;

use crate::core::auth_guidance::format_no_models_available_message;
use crate::core::model_registry::ModelRegistry;

// ---------------------------------------------------------------------------
// Fuzzy filter (inline — mirrors fuzzyFilter from @hamr/tui)
// ---------------------------------------------------------------------------

/// Very simple fuzzy filter: checks if every character in `pattern`
/// appears in order in `text` (case-insensitive).
fn fuzzy_matches(text: &str, pattern: &str) -> bool {
    let pattern = pattern.to_lowercase();
    let text = text.to_lowercase();
    let mut pattern_chars = pattern.chars();
    let mut next_char = pattern_chars.next();
    for ch in text.chars() {
        if let Some(pat) = next_char {
            if ch == pat {
                next_char = pattern_chars.next();
            }
        } else {
            return true;
        }
    }
    next_char.is_none()
}

/// Fuzzy filter models by search pattern.
fn fuzzy_filter_models<'a>(models: &'a [Model], pattern: &str) -> Vec<&'a Model> {
    models
        .iter()
        .filter(|m| {
            let combined = format!("{} {}", m.provider, m.id);
            fuzzy_matches(&combined, pattern)
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Pad a string to a given width with left alignment.
fn pad(s: &str, width: usize) -> String {
    format!("{:<width$}", s, width = width)
}

// ---------------------------------------------------------------------------
// Formatting
// ---------------------------------------------------------------------------

/// Format a number as human-readable (e.g., 200000 -> "200K", 1000000 -> "1M").
fn format_token_count(count: u64) -> String {
    if count >= 1_000_000 {
        let millions = count as f64 / 1_000_000.0;
        if millions.fract() == 0.0 {
            format!("{}M", millions as u64)
        } else {
            format!("{:.1}M", millions)
        }
    } else if count >= 1_000 {
        let thousands = count as f64 / 1_000.0;
        if thousands.fract() == 0.0 {
            format!("{}K", thousands as u64)
        } else {
            format!("{:.1}K", thousands)
        }
    } else {
        count.to_string()
    }
}

// ---------------------------------------------------------------------------
// List models
// ---------------------------------------------------------------------------

/// List available models, optionally filtered by search pattern.
pub async fn list_models(model_registry: &ModelRegistry, search_pattern: Option<&str>) {
    if let Some(load_error) = model_registry.get_error() {
        eprintln!(
            "\x1b[33mWarning: errors loading models.json:\n{}\x1b[0m",
            load_error
        );
    }

    let models = model_registry.get_available();

    if models.is_empty() {
        println!("{}", format_no_models_available_message());
        return;
    }

    // Apply fuzzy filter if search pattern provided
    let mut filtered: Vec<Model> = if let Some(pattern) = search_pattern {
        fuzzy_filter_models(&models, pattern)
            .into_iter()
            .cloned()
            .collect()
    } else {
        models.clone()
    };

    if filtered.is_empty() {
        println!("No models matching \"{}\"", search_pattern.unwrap_or(""));
        return;
    }

    // Sort by provider, then by model id
    filtered.sort_by(|a, b| {
        let provider_cmp = a.provider.cmp(&b.provider);
        if provider_cmp != std::cmp::Ordering::Equal {
            return provider_cmp;
        }
        a.id.cmp(&b.id)
    });

    // Calculate column widths
    #[derive(Debug)]
    struct Row {
        provider: String,
        model: String,
        context: String,
        max_out: String,
        thinking: String,
        images: String,
    }

    let rows: Vec<Row> = filtered
        .iter()
        .map(|m| Row {
            provider: m.provider.clone(),
            model: m.id.clone(),
            context: format_token_count(m.context_window),
            max_out: format_token_count(m.max_tokens),
            thinking: if m.reasoning { "yes" } else { "no" }.to_string(),
            images: if m.input.contains(&hamr_ai::types::InputModality::Image) {
                "yes"
            } else {
                "no"
            }
            .to_string(),
        })
        .collect();

    let headers = Row {
        provider: "provider".to_string(),
        model: "model".to_string(),
        context: "context".to_string(),
        max_out: "max-out".to_string(),
        thinking: "thinking".to_string(),
        images: "images".to_string(),
    };

    #[derive(Debug)]
    struct Widths {
        provider: usize,
        model: usize,
        context: usize,
        max_out: usize,
        thinking: usize,
        images: usize,
    }

    let widths = Widths {
        provider: std::cmp::max(
            headers.provider.len(),
            rows.iter().map(|r| r.provider.len()).max().unwrap_or(0),
        ),
        model: std::cmp::max(
            headers.model.len(),
            rows.iter().map(|r| r.model.len()).max().unwrap_or(0),
        ),
        context: std::cmp::max(
            headers.context.len(),
            rows.iter().map(|r| r.context.len()).max().unwrap_or(0),
        ),
        max_out: std::cmp::max(
            headers.max_out.len(),
            rows.iter().map(|r| r.max_out.len()).max().unwrap_or(0),
        ),
        thinking: std::cmp::max(
            headers.thinking.len(),
            rows.iter().map(|r| r.thinking.len()).max().unwrap_or(0),
        ),
        images: std::cmp::max(
            headers.images.len(),
            rows.iter().map(|r| r.images.len()).max().unwrap_or(0),
        ),
    };

    // Header
    println!(
        "{}  {}  {}  {}  {}  {}",
        pad(&headers.provider, widths.provider),
        pad(&headers.model, widths.model),
        pad(&headers.context, widths.context),
        pad(&headers.max_out, widths.max_out),
        pad(&headers.thinking, widths.thinking),
        pad(&headers.images, widths.images),
    );

    // Rows
    for row in &rows {
        println!(
            "{}  {}  {}  {}  {}  {}",
            pad(&row.provider, widths.provider),
            pad(&row.model, widths.model),
            pad(&row.context, widths.context),
            pad(&row.max_out, widths.max_out),
            pad(&row.thinking, widths.thinking),
            pad(&row.images, widths.images),
        );
    }
}
