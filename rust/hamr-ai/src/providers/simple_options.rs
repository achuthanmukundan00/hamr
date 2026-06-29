//! Port of `packages/ai/src/providers/simple-options.ts`.
//!
//! Base option building and thinking-budget adjustment shared across providers.

use crate::types::{Model, SimpleStreamOptions, StreamOptions, ThinkingBudgets, ThinkingLevel};

/// Copy the base [`StreamOptions`] out of a [`SimpleStreamOptions`], applying an
/// explicit `api_key` override (falling back to the options' own key).
///
/// Mirrors the TS `buildBaseOptions(_model, options?, apiKey?)`.
pub fn build_base_options(
    _model: &Model,
    options: Option<&SimpleStreamOptions>,
    api_key: Option<&str>,
) -> StreamOptions {
    let mut base = options.map(|o| o.base.clone()).unwrap_or_default();
    // `apiKey || options?.apiKey`: explicit override wins, else keep base's key.
    base.api_key = api_key.map(|k| k.to_string()).or(base.api_key);
    base
}

/// Result of [`adjust_max_tokens_for_thinking`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AdjustedTokens {
    pub max_tokens: u64,
    pub thinking_budget: u64,
}

/// Clamp `xhigh` down to `high`; all other levels pass through unchanged.
///
/// Mirrors the TS `clampReasoning` (`Exclude<ThinkingLevel, "xhigh">`).
pub fn clamp_reasoning(effort: Option<ThinkingLevel>) -> Option<ThinkingLevel> {
    match effort {
        Some(ThinkingLevel::XHigh) => Some(ThinkingLevel::High),
        other => other,
    }
}

/// Resolve the token budget for a (non-xhigh) thinking level, applying custom
/// overrides over the defaults.
fn budget_for(level: ThinkingLevel, custom: Option<&ThinkingBudgets>) -> u64 {
    // Defaults from the TS `defaultBudgets`.
    let (default, override_value) = match level {
        ThinkingLevel::Minimal => (1024, custom.and_then(|b| b.minimal)),
        ThinkingLevel::Low => (2048, custom.and_then(|b| b.low)),
        ThinkingLevel::Medium => (8192, custom.and_then(|b| b.medium)),
        // `clamp_reasoning` guarantees we never see `XHigh` here; treat it as High.
        ThinkingLevel::High | ThinkingLevel::XHigh => (16384, custom.and_then(|b| b.high)),
    };
    override_value.unwrap_or(default)
}

/// Compute the output token cap and thinking budget for a reasoning request.
///
/// `base_max_tokens == None` means no explicit caller cap — use the model cap
/// and fit thinking inside it.
pub fn adjust_max_tokens_for_thinking(
    base_max_tokens: Option<u64>,
    model_max_tokens: u64,
    reasoning_level: ThinkingLevel,
    custom_budgets: Option<&ThinkingBudgets>,
) -> AdjustedTokens {
    const MIN_OUTPUT_TOKENS: u64 = 1024;

    let level = clamp_reasoning(Some(reasoning_level)).unwrap_or(ThinkingLevel::High);
    let mut thinking_budget = budget_for(level, custom_budgets);

    let max_tokens = match base_max_tokens {
        None => model_max_tokens,
        Some(base) => (base + thinking_budget).min(model_max_tokens),
    };

    if max_tokens <= thinking_budget {
        thinking_budget = max_tokens.saturating_sub(MIN_OUTPUT_TOKENS);
    }

    AdjustedTokens {
        max_tokens,
        thinking_budget,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clamp_reasoning_lowers_xhigh() {
        assert_eq!(
            clamp_reasoning(Some(ThinkingLevel::XHigh)),
            Some(ThinkingLevel::High)
        );
        assert_eq!(
            clamp_reasoning(Some(ThinkingLevel::Low)),
            Some(ThinkingLevel::Low)
        );
        assert_eq!(clamp_reasoning(None), None);
    }

    #[test]
    fn no_base_cap_uses_model_max() {
        // model max == medium budget (8192) → max_tokens <= budget → budget shrinks
        // to leave room for the minimum output tokens (8192 - 1024 = 7168).
        let r = adjust_max_tokens_for_thinking(None, 8192, ThinkingLevel::Medium, None);
        assert_eq!(r.max_tokens, 8192);
        assert_eq!(r.thinking_budget, 7168);
    }

    #[test]
    fn no_base_cap_with_headroom_keeps_full_budget() {
        let r = adjust_max_tokens_for_thinking(None, 100_000, ThinkingLevel::Medium, None);
        assert_eq!(r.max_tokens, 100_000);
        assert_eq!(r.thinking_budget, 8192);
    }

    #[test]
    fn base_cap_adds_budget_clamped_to_model_max() {
        let r = adjust_max_tokens_for_thinking(Some(2000), 100_000, ThinkingLevel::High, None);
        assert_eq!(r.max_tokens, 2000 + 16384);
        assert_eq!(r.thinking_budget, 16384);
    }

    #[test]
    fn budget_shrinks_when_max_below_budget() {
        // model max 4000, high budget 16384 → max_tokens=4000 <= budget → shrink.
        let r = adjust_max_tokens_for_thinking(None, 4000, ThinkingLevel::High, None);
        assert_eq!(r.max_tokens, 4000);
        assert_eq!(r.thinking_budget, 4000 - 1024);
    }

    #[test]
    fn custom_budget_overrides_default() {
        let custom = ThinkingBudgets {
            medium: Some(5000),
            ..Default::default()
        };
        let r = adjust_max_tokens_for_thinking(None, 100_000, ThinkingLevel::Medium, Some(&custom));
        assert_eq!(r.thinking_budget, 5000);
    }
}
