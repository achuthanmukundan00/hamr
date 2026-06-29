//! Port of `packages/ai/src/models.ts`.
//!
//! Model registry plus cost calculation and thinking-level resolution. The
//! registry data comes from [`crate::models_generated`] (a codegen target).

use std::collections::HashMap;
use std::sync::LazyLock;

use crate::models_generated;
use crate::types::{Model, ModelThinkingLevel, ThinkingLevelMap, Usage, UsageCost};

/// Provider → (model id → [`Model`]) registry, initialized once from the
/// generated catalogue.
static MODEL_REGISTRY: LazyLock<HashMap<String, HashMap<String, Model>>> =
    LazyLock::new(models_generated::models);

/// Look up a model by provider and id.
pub fn get_model(provider: &str, model_id: &str) -> Option<Model> {
    MODEL_REGISTRY.get(provider)?.get(model_id).cloned()
}

/// All registered provider names.
pub fn get_providers() -> Vec<String> {
    MODEL_REGISTRY.keys().cloned().collect()
}

/// All models registered for a provider.
pub fn get_models(provider: &str) -> Vec<Model> {
    MODEL_REGISTRY
        .get(provider)
        .map(|models| models.values().cloned().collect())
        .unwrap_or_default()
}

/// Compute the cost breakdown for `usage` under `model`, writing it back into
/// `usage.cost` and returning a copy.
///
/// Anthropic charges 2× base input for 1h cache writes.
pub fn calculate_cost(model: &Model, usage: &mut Usage) -> UsageCost {
    let long_write = usage.cache_write_1h.unwrap_or(0) as f64;
    let short_write = usage.cache_write as f64 - long_write;
    usage.cost.input = (model.cost.input / 1_000_000.0) * usage.input as f64;
    usage.cost.output = (model.cost.output / 1_000_000.0) * usage.output as f64;
    usage.cost.cache_read = (model.cost.cache_read / 1_000_000.0) * usage.cache_read as f64;
    usage.cost.cache_write =
        (model.cost.cache_write * short_write + model.cost.input * 2.0 * long_write) / 1_000_000.0;
    usage.cost.total =
        usage.cost.input + usage.cost.output + usage.cost.cache_read + usage.cost.cache_write;
    usage.cost.clone()
}

/// Look up a thinking level in a model's map.
///
/// - `None` → key absent (provider default).
/// - `Some(None)` → present `null` (unsupported).
/// - `Some(Some(value))` → mapped to a provider-specific value.
fn lookup_level(
    map: Option<&ThinkingLevelMap>,
    level: ModelThinkingLevel,
) -> Option<&Option<String>> {
    map.and_then(|m| m.get(&level))
}

/// The thinking levels a model supports.
pub fn get_supported_thinking_levels(model: &Model) -> Vec<ModelThinkingLevel> {
    if !model.reasoning {
        return vec![ModelThinkingLevel::Off];
    }

    ModelThinkingLevel::EXTENDED
        .into_iter()
        .filter(|&level| {
            let entry = lookup_level(model.thinking_level_map.as_ref(), level);
            match entry {
                // Present `null` → explicitly unsupported.
                Some(None) => false,
                // `xhigh` is only supported when explicitly mapped (not undefined).
                _ if level == ModelThinkingLevel::XHigh => entry.is_some(),
                _ => true,
            }
        })
        .collect()
}

/// Clamp a requested thinking level to the nearest level the model supports,
/// walking up the scale first, then down.
pub fn clamp_thinking_level(model: &Model, level: ModelThinkingLevel) -> ModelThinkingLevel {
    let available = get_supported_thinking_levels(model);
    if available.contains(&level) {
        return level;
    }

    let extended = ModelThinkingLevel::EXTENDED;
    let requested_index = match extended.iter().position(|&l| l == level) {
        Some(i) => i,
        None => {
            return available
                .first()
                .copied()
                .unwrap_or(ModelThinkingLevel::Off);
        }
    };

    for candidate in &extended[requested_index..] {
        if available.contains(candidate) {
            return *candidate;
        }
    }
    for candidate in extended[..requested_index].iter().rev() {
        if available.contains(candidate) {
            return *candidate;
        }
    }
    available
        .first()
        .copied()
        .unwrap_or(ModelThinkingLevel::Off)
}

/// Compare two models by id and provider. Returns `false` if either is `None`.
pub fn models_are_equal(a: Option<&Model>, b: Option<&Model>) -> bool {
    match (a, b) {
        (Some(a), Some(b)) => a.id == b.id && a.provider == b.provider,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Api, ModelCost};

    fn model(reasoning: bool, map: Option<ThinkingLevelMap>) -> Model {
        Model {
            id: "m".into(),
            name: "M".into(),
            api: Api::AnthropicMessages,
            provider: "anthropic".into(),
            base_url: "https://example".into(),
            reasoning,
            thinking_level_map: map,
            input: vec![],
            cost: ModelCost {
                input: 3.0,
                output: 15.0,
                cache_read: 0.3,
                cache_write: 3.75,
            },
            context_window: 200_000,
            max_tokens: 8192,
            headers: None,
            compat: None,
        }
    }

    fn map(pairs: &[(ModelThinkingLevel, Option<&str>)]) -> ThinkingLevelMap {
        pairs
            .iter()
            .map(|(k, v)| (*k, v.map(String::from)))
            .collect()
    }

    #[test]
    fn empty_registry_lookup() {
        assert!(get_model("nope", "nope").is_none());
        assert!(get_models("nope").is_empty());
    }

    #[test]
    fn non_reasoning_model_only_off() {
        assert_eq!(
            get_supported_thinking_levels(&model(false, None)),
            vec![ModelThinkingLevel::Off]
        );
    }

    #[test]
    fn reasoning_model_excludes_xhigh_unless_mapped() {
        let levels = get_supported_thinking_levels(&model(true, None));
        assert!(levels.contains(&ModelThinkingLevel::High));
        assert!(!levels.contains(&ModelThinkingLevel::XHigh));
    }

    #[test]
    fn xhigh_included_when_mapped() {
        let m = model(true, Some(map(&[(ModelThinkingLevel::XHigh, Some("max"))])));
        assert!(get_supported_thinking_levels(&m).contains(&ModelThinkingLevel::XHigh));
    }

    #[test]
    fn null_level_marks_unsupported() {
        let m = model(true, Some(map(&[(ModelThinkingLevel::Low, None)])));
        assert!(!get_supported_thinking_levels(&m).contains(&ModelThinkingLevel::Low));
    }

    #[test]
    fn clamp_walks_down_to_supported() {
        // xhigh unmapped → clamp xhigh down to high.
        let m = model(true, None);
        assert_eq!(
            clamp_thinking_level(&m, ModelThinkingLevel::XHigh),
            ModelThinkingLevel::High
        );
    }

    #[test]
    fn clamp_returns_off_for_non_reasoning() {
        let m = model(false, None);
        assert_eq!(
            clamp_thinking_level(&m, ModelThinkingLevel::High),
            ModelThinkingLevel::Off
        );
    }

    #[test]
    fn cost_calculation_matches_formula() {
        let m = model(false, None);
        let mut usage = Usage {
            input: 1_000_000,
            output: 1_000_000,
            cache_read: 0,
            cache_write: 0,
            cache_write_1h: None,
            total_tokens: 2_000_000,
            cost: UsageCost {
                input: 0.0,
                output: 0.0,
                cache_read: 0.0,
                cache_write: 0.0,
                total: 0.0,
            },
        };
        let cost = calculate_cost(&m, &mut usage);
        assert!((cost.input - 3.0).abs() < 1e-9);
        assert!((cost.output - 15.0).abs() < 1e-9);
        assert!((cost.total - 18.0).abs() < 1e-9);
    }

    #[test]
    fn cost_1h_cache_write_doubles_input_rate() {
        let m = model(false, None);
        let mut usage = Usage {
            input: 0,
            output: 0,
            cache_read: 0,
            cache_write: 1_000_000,
            cache_write_1h: Some(1_000_000),
            total_tokens: 1_000_000,
            cost: UsageCost {
                input: 0.0,
                output: 0.0,
                cache_read: 0.0,
                cache_write: 0.0,
                total: 0.0,
            },
        };
        let cost = calculate_cost(&m, &mut usage);
        // All 1h: input rate (3.0) * 2 = 6.0 per million.
        assert!((cost.cache_write - 6.0).abs() < 1e-9);
    }

    #[test]
    fn models_equal_by_id_and_provider() {
        let a = model(false, None);
        let b = model(false, None);
        assert!(models_are_equal(Some(&a), Some(&b)));
        assert!(!models_are_equal(Some(&a), None));
        assert!(!models_are_equal(None, None));
    }

    #[test]
    fn cache_write_1h_serializes_camel_case() {
        let usage = Usage {
            input: 0,
            output: 0,
            cache_read: 0,
            cache_write: 5,
            cache_write_1h: Some(2),
            total_tokens: 0,
            cost: UsageCost {
                input: 0.0,
                output: 0.0,
                cache_read: 0.0,
                cache_write: 0.0,
                total: 0.0,
            },
        };
        let json = serde_json::to_string(&usage).unwrap();
        assert!(json.contains("\"cacheWrite1h\":2"), "got {json}");
        assert!(json.contains("\"cacheRead\""), "got {json}");
    }

    #[test]
    fn copilot_thinking_level_map_includes_xhigh() {
        // GitHub Copilot models remap thinking levels (e.g., minimal → low, xhigh → max)
        let m = Model {
            id: "claude-opus-4.7".into(),
            name: "Claude Opus 4.7".into(),
            api: Api::AnthropicMessages,
            provider: "github-copilot".into(),
            base_url: "https://api.individual.githubcopilot.com".into(),
            reasoning: true,
            thinking_level_map: Some(
                vec![
                    (ModelThinkingLevel::Minimal, Some("low".to_string())),
                    (ModelThinkingLevel::XHigh, Some("xhigh".to_string())),
                ]
                .into_iter()
                .collect(),
            ),
            input: vec![],
            cost: ModelCost {
                input: 0.0,
                output: 0.0,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 200_000,
            max_tokens: 8192,
            headers: None,
            compat: None,
        };
        assert!(get_supported_thinking_levels(&m).contains(&ModelThinkingLevel::XHigh));
        // 'minimal' is in the map with a value, so it is a supported level
        assert!(get_supported_thinking_levels(&m).contains(&ModelThinkingLevel::Minimal));
    }

    #[test]
    fn copilot_sonnet_thinking_level_map() {
        let m = Model {
            id: "claude-sonnet-4.6".into(),
            name: "Claude Sonnet 4.6".into(),
            api: Api::AnthropicMessages,
            provider: "github-copilot".into(),
            base_url: "https://api.individual.githubcopilot.com".into(),
            reasoning: true,
            thinking_level_map: Some(
                vec![
                    (ModelThinkingLevel::Minimal, Some("low".to_string())),
                    (ModelThinkingLevel::XHigh, Some("max".to_string())),
                ]
                .into_iter()
                .collect(),
            ),
            input: vec![],
            cost: ModelCost {
                input: 0.0,
                output: 0.0,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 200_000,
            max_tokens: 8192,
            headers: None,
            compat: None,
        };
        let levels = get_supported_thinking_levels(&m);
        assert!(levels.contains(&ModelThinkingLevel::XHigh));
        assert!(levels.contains(&ModelThinkingLevel::Off));
    }
}
