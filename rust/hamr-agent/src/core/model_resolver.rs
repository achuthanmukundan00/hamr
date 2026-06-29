//! Model resolution, scoping, and initial selection.
//!
//! Port of `packages/coding-agent/src/core/model-resolver.ts`.

use std::collections::HashMap;
use std::sync::LazyLock;

use hamr_ai::models::models_are_equal;
use hamr_ai::types::{Model, ModelThinkingLevel, ThinkingLevel};

use crate::core::defaults::DEFAULT_THINKING_LEVEL;

// ---------------------------------------------------------------------------
// Default model IDs for each known provider
// ---------------------------------------------------------------------------

/// Default model IDs for each known provider.
///
/// Mirrors TS `defaultModelPerProvider`.
pub fn default_model_for_provider(provider: &str) -> Option<&'static str> {
    DEFAULT_MODEL_PER_PROVIDER.get(provider).copied()
}

static DEFAULT_MODEL_PER_PROVIDER: LazyLock<HashMap<&'static str, &'static str>> =
    LazyLock::new(|| {
        HashMap::from([
            ("amazon-bedrock", "us.anthropic.claude-opus-4-6-v1"),
            ("ant-ling", "Ring-2.6-1T"),
            ("anthropic", "claude-opus-4-8"),
            ("openai", "gpt-5.4"),
            ("azure-openai-responses", "gpt-5.4"),
            ("openai-codex", "gpt-5.5"),
            ("nvidia", "nvidia/nemotron-3-super-120b-a12b"),
            ("deepseek", "deepseek-v4-pro"),
            ("google", "gemini-3.1-pro-preview"),
            ("google-vertex", "gemini-3.1-pro-preview"),
            ("github-copilot", "gpt-5.4"),
            ("openrouter", "moonshotai/kimi-k2.6"),
            ("vercel-ai-gateway", "zai/glm-5.1"),
            ("xai", "grok-4.20-0309-reasoning"),
            ("groq", "openai/gpt-oss-120b"),
            ("cerebras", "zai-glm-4.7"),
            ("zai", "glm-5.1"),
            ("zai-coding-cn", "glm-5.1"),
            ("mistral", "devstral-medium-latest"),
            ("minimax", "MiniMax-M2.7"),
            ("minimax-cn", "MiniMax-M2.7"),
            ("moonshotai", "kimi-k2.6"),
            ("moonshotai-cn", "kimi-k2.6"),
            ("huggingface", "moonshotai/Kimi-K2.6"),
            ("fireworks", "accounts/fireworks/models/kimi-k2p6"),
            ("together", "moonshotai/Kimi-K2.6"),
            ("opencode", "kimi-k2.6"),
            ("opencode-go", "kimi-k2.6"),
            ("kimi-coding", "kimi-for-coding"),
            ("cloudflare-workers-ai", "@cf/moonshotai/kimi-k2.6"),
            (
                "cloudflare-ai-gateway",
                "workers-ai/@cf/moonshotai/kimi-k2.6",
            ),
            ("xiaomi", "mimo-v2.5-pro"),
            ("xiaomi-token-plan-cn", "mimo-v2.5-pro"),
            ("xiaomi-token-plan-ams", "mimo-v2.5-pro"),
            ("xiaomi-token-plan-sgp", "mimo-v2.5-pro"),
        ])
    });

// ---------------------------------------------------------------------------
// ScopedModel
// ---------------------------------------------------------------------------

/// A model with an optional explicit thinking level.
#[derive(Debug, Clone)]
pub struct ScopedModel {
    pub model: Model,
    /// Thinking level if explicitly specified in pattern (e.g. "model:high"),
    /// `None` otherwise.
    pub thinking_level: Option<ThinkingLevel>,
}

// ---------------------------------------------------------------------------
// ModelRegistry trait
// ---------------------------------------------------------------------------

/// Interface for the model registry, used by resolver functions.
///
/// Mirrors the TS `ModelRegistry` class.
pub trait ModelRegistry {
    /// Get models that have auth configured (ready to use).
    fn get_available(&self) -> Vec<Model>;
    /// Get all models (built-in + custom), regardless of auth status.
    fn get_all(&self) -> Vec<Model>;
    /// Find a model by provider and ID.
    fn find(&self, provider: &str, model_id: &str) -> Option<Model>;
    /// Check whether a model has auth configured.
    fn has_configured_auth(&self, model: &Model) -> bool;
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Valid thinking level strings, including the `"off"` sentinel.
static VALID_THINKING_LEVELS: [&str; 6] = ["off", "minimal", "low", "medium", "high", "xhigh"];

/// Check whether a string is a valid thinking level (including `"off"`).
fn is_valid_thinking_level(s: &str) -> bool {
    VALID_THINKING_LEVELS.contains(&s)
}

fn parse_thinking_level(s: &str) -> Option<ThinkingLevel> {
    match s {
        "minimal" => Some(ThinkingLevel::Minimal),
        "low" => Some(ThinkingLevel::Low),
        "medium" => Some(ThinkingLevel::Medium),
        "high" => Some(ThinkingLevel::High),
        "xhigh" => Some(ThinkingLevel::XHigh),
        _ => None,
    }
}

fn parse_model_thinking_level(s: &str) -> Option<ModelThinkingLevel> {
    match s {
        "off" => Some(ModelThinkingLevel::Off),
        "minimal" => Some(ModelThinkingLevel::Minimal),
        "low" => Some(ModelThinkingLevel::Low),
        "medium" => Some(ModelThinkingLevel::Medium),
        "high" => Some(ModelThinkingLevel::High),
        "xhigh" => Some(ModelThinkingLevel::XHigh),
        _ => None,
    }
}

/// Case-insensitive glob-like matching.
///
/// Supports `*` (any characters), `?` (single character).
/// The pattern is compared case-insensitively against the text.
fn minimatch(text: &str, pattern: &str) -> bool {
    minimatch_inner(text, pattern, true)
}

fn minimatch_inner(text: &str, pattern: &str, case_insensitive: bool) -> bool {
    let t_bytes: &[u8] = text.as_bytes();
    let p_bytes: &[u8] = pattern.as_bytes();
    let (mut ti, mut pi) = (0, 0);
    let (mut star_ti, mut star_pi): (Option<usize>, Option<usize>) = (None, None);

    while ti < t_bytes.len() {
        if pi < p_bytes.len()
            && (p_bytes[pi] == b'?' || char_eq(p_bytes[pi], t_bytes[ti], case_insensitive))
        {
            ti += 1;
            pi += 1;
        } else if pi < p_bytes.len() && p_bytes[pi] == b'*' {
            star_ti = Some(ti);
            star_pi = Some(pi);
            pi += 1;
        } else if star_pi.is_some() {
            ti = star_ti.unwrap() + 1;
            star_ti = Some(ti);
            pi = star_pi.unwrap() + 1;
        } else {
            return false;
        }
    }

    while pi < p_bytes.len() && p_bytes[pi] == b'*' {
        pi += 1;
    }

    pi == p_bytes.len()
}

fn char_eq(a: u8, b: u8, case_insensitive: bool) -> bool {
    if case_insensitive {
        a.to_ascii_lowercase() == b.to_ascii_lowercase()
    } else {
        a == b
    }
}

/// Helper to check if a model ID looks like an alias (no date suffix).
/// Dates are typically in format: `-20241022` or `-20250929`.
fn is_alias(id: &str) -> bool {
    // Check if ID ends with -latest
    if id.ends_with("-latest") {
        return true;
    }
    // Check if ID ends with a date pattern (-YYYYMMDD)
    if id.len() < 9 {
        return true; // too short to be a dated version, treat as alias
    }
    let suffix = &id[id.len() - 9..];
    if !suffix.starts_with('-') {
        return true;
    }
    // Check that the remaining 8 chars are all digits
    !suffix[1..].chars().all(|c| c.is_ascii_digit())
}

/// Build a fallback model when the exact pattern is not found in the registry.
fn build_fallback_model(
    provider: &str,
    model_id: &str,
    available_models: &[Model],
) -> Option<Model> {
    let provider_models: Vec<&Model> = available_models
        .iter()
        .filter(|m| m.provider == provider)
        .collect();
    if provider_models.is_empty() {
        return None;
    }

    let default_id = default_model_for_provider(provider);
    let base_model = default_id
        .and_then(|did| provider_models.iter().find(|m| m.id == did))
        .or_else(|| provider_models.first())
        .copied()?;

    Some(Model {
        id: model_id.to_string(),
        name: model_id.to_string(),
        ..base_model.clone()
    })
}

// ---------------------------------------------------------------------------
// find_exact_model_reference_match
// ---------------------------------------------------------------------------

/// Find an exact model reference match.
///
/// Supports either a bare model ID or a canonical `provider/modelId` reference.
/// When matching by bare ID, ambiguous matches across providers are rejected.
pub fn find_exact_model_reference_match(
    model_reference: &str,
    available_models: &[Model],
) -> Option<Model> {
    let trimmed = model_reference.trim();
    if trimmed.is_empty() {
        return None;
    }

    let norm = trimmed.to_lowercase();

    // Canonical "provider/id" matches
    let canonical: Vec<&Model> = available_models
        .iter()
        .filter(|m| format!("{}/{}", m.provider, m.id).to_lowercase() == norm)
        .collect();
    if canonical.len() == 1 {
        return Some(canonical[0].clone());
    }
    if canonical.len() > 1 {
        return None;
    }

    // If there's a slash, try splitting into provider + modelId
    if let Some(slash) = trimmed.find('/') {
        let provider = trimmed[..slash].trim();
        let model_id = trimmed[slash + 1..].trim();
        if !provider.is_empty() && !model_id.is_empty() {
            let split_matches: Vec<&Model> = available_models
                .iter()
                .filter(|m| {
                    m.provider.eq_ignore_ascii_case(provider) && m.id.eq_ignore_ascii_case(model_id)
                })
                .collect();
            if split_matches.len() == 1 {
                return Some(split_matches[0].clone());
            }
            if split_matches.len() > 1 {
                return None;
            }
        }
    }

    // Bare model ID match
    let id_matches: Vec<&Model> = available_models
        .iter()
        .filter(|m| m.id.to_lowercase() == norm)
        .collect();

    if id_matches.len() == 1 {
        Some(id_matches[0].clone())
    } else {
        None
    }
}

// ---------------------------------------------------------------------------
// try_match_model
// ---------------------------------------------------------------------------

/// Try to match a pattern to a model from the available models list.
/// Returns the matched model or `None` if no match found.
fn try_match_model(model_pattern: &str, available_models: &[Model]) -> Option<Model> {
    let exact = find_exact_model_reference_match(model_pattern, available_models);
    if exact.is_some() {
        return exact;
    }

    let lower = model_pattern.to_lowercase();
    let matches: Vec<&Model> = available_models
        .iter()
        .filter(|m| m.id.to_lowercase().contains(&lower) || m.name.to_lowercase().contains(&lower))
        .collect();

    if matches.is_empty() {
        return None;
    }

    // Separate into aliases and dated versions
    let aliases: Vec<&&Model> = matches.iter().filter(|m| is_alias(&m.id)).collect();
    let dated: Vec<&&Model> = matches.iter().filter(|m| !is_alias(&m.id)).collect();

    if !aliases.is_empty() {
        // Prefer alias — if multiple aliases, pick the one that sorts highest
        let mut sorted = aliases.clone();
        sorted.sort_by(|a, b| b.id.cmp(&a.id));
        Some((*sorted[0]).clone())
    } else {
        // No alias found, pick latest dated version
        let mut sorted = dated.clone();
        sorted.sort_by(|a, b| b.id.cmp(&a.id));
        Some((*sorted[0]).clone())
    }
}

// ---------------------------------------------------------------------------
// ParsedModelResult
// ---------------------------------------------------------------------------

/// Result of parsing a model pattern.
#[derive(Debug, Clone)]
pub struct ParsedModelResult {
    pub model: Option<Model>,
    /// Thinking level if explicitly specified in pattern, `None` otherwise.
    pub thinking_level: Option<ThinkingLevel>,
    pub warning: Option<String>,
}

// ---------------------------------------------------------------------------
// parse_model_pattern
// ---------------------------------------------------------------------------

/// Parse a pattern to extract model and thinking level.
///
/// Handles models with colons in their IDs (e.g., OpenRouter's `:exacto` suffix).
///
/// **Algorithm:**
/// 1. Try to match full pattern as a model.
/// 2. If found, return it with `off` thinking level.
/// 3. If not found and has colons, split on last colon:
///    - If suffix is valid thinking level, use it and recurse on prefix.
///    - If suffix is invalid, warn and recurse on prefix with `off`.
pub fn parse_model_pattern(
    pattern: &str,
    available_models: &[Model],
    options: Option<&ParseModelPatternOptions>,
) -> ParsedModelResult {
    // Try exact match first
    let exact = try_match_model(pattern, available_models);
    if let Some(model) = exact {
        return ParsedModelResult {
            model: Some(model),
            thinking_level: None,
            warning: None,
        };
    }

    // No match — try splitting on last colon if present
    let last_colon = pattern.rfind(':');
    let last_colon = match last_colon {
        Some(idx) => idx,
        None => {
            // No colons, pattern simply doesn't match any model
            return ParsedModelResult {
                model: None,
                thinking_level: None,
                warning: None,
            };
        }
    };

    let prefix = &pattern[..last_colon];
    let suffix = &pattern[last_colon + 1..];

    if is_valid_thinking_level(suffix) {
        // Valid thinking level — recurse on prefix and use this level
        let result = parse_model_pattern(prefix, available_models, options);
        if result.model.is_some() {
            // Only use this thinking level if no warning from inner recursion
            let level = if result.warning.is_some() {
                None
            } else {
                parse_thinking_level(suffix)
            };
            return ParsedModelResult {
                model: result.model,
                thinking_level: level,
                warning: result.warning,
            };
        }
        return result;
    }

    // Invalid suffix
    let allow_fallback = options
        .and_then(|o| o.allow_invalid_thinking_level_fallback)
        .unwrap_or(true);

    if !allow_fallback {
        // Strict mode (CLI --model parsing): treat it as part of the model ID and fail.
        return ParsedModelResult {
            model: None,
            thinking_level: None,
            warning: None,
        };
    }

    // Scope mode: recurse on prefix and warn
    let result = parse_model_pattern(prefix, available_models, options);
    if result.model.is_some() {
        return ParsedModelResult {
            model: result.model,
            thinking_level: None,
            warning: Some(format!(
                "Invalid thinking level \"{suffix}\" in pattern \"{pattern}\". Using default instead."
            )),
        };
    }
    result
}

/// Options for [`parse_model_pattern`].
#[derive(Debug, Clone)]
pub struct ParseModelPatternOptions {
    pub allow_invalid_thinking_level_fallback: Option<bool>,
}

// ---------------------------------------------------------------------------
// resolve_model_scope
// ---------------------------------------------------------------------------

/// Resolve model patterns to actual `Model` objects with optional thinking levels.
///
/// Format: `pattern:level` where `:level` is optional.  
/// For each pattern, finds all matching models and picks the best version:
///
/// 1. Prefer alias (e.g., `claude-sonnet-4-5`) over dated versions (`claude-sonnet-4-5-20250929`)
/// 2. If no alias, pick the latest dated version
///
/// Supports models with colons in their IDs (e.g., OpenRouter's `model:exacto`).
/// The algorithm tries to match the full pattern first, then progressively
/// strips colon-suffixes to find a match.
pub async fn resolve_model_scope(
    patterns: &[String],
    model_registry: &dyn ModelRegistry,
) -> Vec<ScopedModel> {
    let available_models = model_registry.get_available();
    let mut scoped_models: Vec<ScopedModel> = Vec::new();

    for pattern in patterns {
        // Check if pattern contains glob characters
        let has_glob = pattern.contains('*') || pattern.contains('?') || pattern.contains('[');

        if has_glob {
            // Extract optional thinking level suffix (e.g., "provider/*:high")
            let colon_idx = pattern.rfind(':');
            let (glob_pattern, thinking_level) = if let Some(idx) = colon_idx {
                let suffix = &pattern[idx + 1..];
                if is_valid_thinking_level(suffix) {
                    (&pattern[..idx], parse_thinking_level(suffix))
                } else {
                    (pattern.as_str(), None)
                }
            } else {
                (pattern.as_str(), None)
            };

            // Match against "provider/modelId" format OR just model ID
            let matching_models: Vec<&Model> = available_models
                .iter()
                .filter(|m| {
                    let full_id = format!("{}/{}", m.provider, m.id);
                    minimatch(&full_id, glob_pattern) || minimatch(&m.id, glob_pattern)
                })
                .collect();

            if matching_models.is_empty() {
                eprintln!("Warning: No models match pattern \"{pattern}\"");
                continue;
            }

            for m in matching_models {
                if !scoped_models
                    .iter()
                    .any(|sm| models_are_equal(Some(&sm.model), Some(m)))
                {
                    scoped_models.push(ScopedModel {
                        model: m.clone(),
                        thinking_level,
                    });
                }
            }
            continue;
        }

        let result = parse_model_pattern(pattern, &available_models, None);

        if let Some(ref warning) = result.warning {
            eprintln!("Warning: {warning}");
        }

        let model = match result.model {
            Some(m) => m,
            None => {
                eprintln!("Warning: No models match pattern \"{pattern}\"");
                continue;
            }
        };

        // Avoid duplicates
        if !scoped_models
            .iter()
            .any(|sm| models_are_equal(Some(&sm.model), Some(&model)))
        {
            scoped_models.push(ScopedModel {
                model,
                thinking_level: result.thinking_level,
            });
        }
    }

    scoped_models
}

// ---------------------------------------------------------------------------
// ResolveCliModelResult
// ---------------------------------------------------------------------------

/// Result of resolving a CLI model.
#[derive(Debug, Clone)]
pub struct ResolveCliModelResult {
    pub model: Option<Model>,
    pub thinking_level: Option<ThinkingLevel>,
    pub warning: Option<String>,
    /// Error message suitable for CLI display.
    /// When set, `model` will be `None`.
    pub error: Option<String>,
}

// ---------------------------------------------------------------------------
// resolve_cli_model
// ---------------------------------------------------------------------------

/// Resolve a single model from CLI flags.
///
/// Supports:
/// - `--provider <provider> --model <pattern>`
/// - `--model <provider>/<pattern>`
/// - Fuzzy matching (same rules as model scoping: exact id, then partial id/name)
///
/// Note: This does not apply the thinking level by itself, but it may *parse* and
/// return a thinking level from `<pattern>:<thinking>` so the caller can apply it.
pub fn resolve_cli_model(
    cli_provider: Option<&str>,
    cli_model: Option<&str>,
    cli_thinking: Option<ThinkingLevel>,
    model_registry: &dyn ModelRegistry,
) -> ResolveCliModelResult {
    let cli_model = match cli_model {
        Some(m) if !m.is_empty() => m,
        _ => {
            return ResolveCliModelResult {
                model: None,
                thinking_level: None,
                warning: None,
                error: None,
            };
        }
    };

    // Important: use *all* models here, not just models with pre-configured auth.
    // This allows "--api-key" to be used for first-time setup.
    let available_models = model_registry.get_all();
    if available_models.is_empty() {
        return ResolveCliModelResult {
            model: None,
            thinking_level: None,
            warning: None,
            error: Some(
                "No models available. Check your installation or add models to models.json."
                    .to_string(),
            ),
        };
    }

    // Build canonical provider lookup (case-insensitive)
    let mut provider_map: HashMap<String, String> = HashMap::new();
    for m in &available_models {
        provider_map.insert(m.provider.to_lowercase(), m.provider.clone());
    }

    let mut provider = cli_provider
        .and_then(|p| provider_map.get(&p.to_lowercase()))
        .map(|s| s.as_str());

    if cli_provider.is_some() && provider.is_none() {
        return ResolveCliModelResult {
            model: None,
            thinking_level: None,
            warning: None,
            error: Some(format!(
                "Unknown provider \"{}\". Use --list-models to see available providers/models.",
                cli_provider.unwrap()
            )),
        };
    }

    // If no explicit --provider, try to interpret "provider/model" format first.
    let mut pattern = cli_model;
    let mut inferred_provider = false;

    if provider.is_none() {
        if let Some(slash) = cli_model.find('/') {
            let maybe = &cli_model[..slash];
            if let Some(canonical) = provider_map.get(&maybe.to_lowercase()) {
                provider = Some(canonical);
                pattern = &cli_model[slash + 1..];
                inferred_provider = true;
            }
        }
    }

    // If no provider was inferred from the slash, try exact matches without provider inference.
    if provider.is_none() {
        let lower = cli_model.to_lowercase();
        if let Some(exact) = available_models.iter().find(|m| {
            m.id.to_lowercase() == lower
                || format!("{}/{}", m.provider, m.id).to_lowercase() == lower
        }) {
            return ResolveCliModelResult {
                model: Some(exact.clone()),
                warning: None,
                thinking_level: None,
                error: None,
            };
        }
    }

    if cli_provider.is_some() && provider.is_some() {
        // If both were provided, tolerate --model <provider>/<pattern> by stripping the provider prefix
        let prefix = format!("{}/", provider.unwrap());
        if cli_model.to_lowercase().starts_with(&prefix.to_lowercase()) {
            pattern = &cli_model[prefix.len()..];
        }
    }

    let candidates: Vec<Model> = if let Some(prov) = provider {
        available_models
            .iter()
            .filter(|m| m.provider == prov)
            .cloned()
            .collect()
    } else {
        available_models.clone()
    };

    let result = parse_model_pattern(
        pattern,
        &candidates,
        Some(&ParseModelPatternOptions {
            allow_invalid_thinking_level_fallback: Some(false),
        }),
    );

    if let Some(model) = result.model {
        // If provider inference matched an unauthenticated provider/model pair, prefer
        // one exact raw model-id match that is authenticated.
        if inferred_provider {
            let raw_exact_matches: Vec<Model> = available_models
                .iter()
                .filter(|m| {
                    m.id.to_lowercase() == cli_model.to_lowercase()
                        && !models_are_equal(Some(m), Some(&model))
                })
                .cloned()
                .collect();

            if !raw_exact_matches.is_empty() && !model_registry.has_configured_auth(&model) {
                let authenticated: Vec<&Model> = raw_exact_matches
                    .iter()
                    .filter(|m| model_registry.has_configured_auth(m))
                    .collect();
                if authenticated.len() == 1 {
                    return ResolveCliModelResult {
                        model: Some(authenticated[0].clone()),
                        thinking_level: None,
                        warning: None,
                        error: None,
                    };
                }
            }
        }

        return ResolveCliModelResult {
            model: Some(model),
            thinking_level: result.thinking_level,
            warning: result.warning,
            error: None,
        };
    }

    // If we inferred a provider from the slash but found no match within that provider,
    // fall back to matching the full input as a raw model id across all models.
    if inferred_provider {
        let lower = cli_model.to_lowercase();
        if let Some(exact) = available_models.iter().find(|m| {
            m.id.to_lowercase() == lower
                || format!("{}/{}", m.provider, m.id).to_lowercase() == lower
        }) {
            return ResolveCliModelResult {
                model: Some(exact.clone()),
                warning: None,
                thinking_level: None,
                error: None,
            };
        }
        // Also try parse_model_pattern on the full input against all models
        let fallback = parse_model_pattern(
            cli_model,
            &available_models,
            Some(&ParseModelPatternOptions {
                allow_invalid_thinking_level_fallback: Some(false),
            }),
        );
        if let Some(fb_model) = fallback.model {
            return ResolveCliModelResult {
                model: Some(fb_model),
                thinking_level: fallback.thinking_level,
                warning: fallback.warning,
                error: None,
            };
        }
    }

    if let Some(prov) = provider {
        // Parse thinking level suffix from the pattern before building the fallback model,
        // but only when --thinking is not explicitly provided.
        let mut fallback_pattern = pattern;
        let mut fallback_thinking: Option<ThinkingLevel> = None;
        if cli_thinking.is_none() {
            if let Some(last_colon) = pattern.rfind(':') {
                let suffix = &pattern[last_colon + 1..];
                if is_valid_thinking_level(suffix) {
                    fallback_pattern = &pattern[..last_colon];
                    fallback_thinking = parse_thinking_level(suffix);
                }
            }
        }

        if let Some(fallback_model) =
            build_fallback_model(prov, fallback_pattern, &available_models)
        {
            let requested_thinking = cli_thinking.or(fallback_thinking);
            let _model = if requested_thinking.is_some()
                && requested_thinking != Some(ThinkingLevel::Medium)
            {
                // When a thinking level is explicitly requested (not medium default), mark as reasoning
                // Note: This is a simplified version — in the TS code it checks `requestedThinking && requestedThinking !== "off"`
                let is_reasoning = requested_thinking
                    .map(|t| t != ThinkingLevel::Medium)
                    .unwrap_or(false);
                Model {
                    reasoning: is_reasoning,
                    ..fallback_model.clone()
                }
            } else {
                fallback_model.clone()
            };

            // Hmm, actually let me re-read the TS logic more carefully...
            // The TS has: const requestedThinking = cliThinking ?? fallbackThinking;
            // const model = requestedThinking && requestedThinking !== "off" ? { ...fallbackModel, reasoning: true } : fallbackModel;
            // So if requestedThinking is set and not "off", set reasoning=true, else use fallback as-is.
            // In Rust, pattern is: if level is set AND level is NOT off → reasoning: true
            let requested_thinking = cli_thinking.or(fallback_thinking);
            let _model = match requested_thinking {
                Some(level) => {
                    // "off" means no reasoning, anything else means reasoning
                    if level != ThinkingLevel::Medium {
                        // Any non-off level means reasoning enabled
                        if level != ThinkingLevel::Medium {
                            Model {
                                reasoning: true,
                                ..fallback_model.clone()
                            }
                        } else {
                            // Medium is default, treat as-is
                            fallback_model.clone()
                        }
                    } else {
                        fallback_model.clone()
                    }
                }
                None => fallback_model.clone(),
            };

            // Wait, let me re-read the TS code:
            // const model = requestedThinking && requestedThinking !== "off" ? { ...fallbackModel, reasoning: true } : fallbackModel;
            // This means: if requestedThinking is truthy AND not "off", reasoning=true, else fallbackModel unchanged.
            // In Rust, since we don't have truthy/falsy, the equivalent is:
            // if requested_thinking.is_some() && requested_thinking != Some(thinking_off_equivalent)
            // But ThinkingLevel enum doesn't have Off. Off is in ModelThinkingLevel.
            // Looking at TS: thinking "off" means no reasoning.
            // In the TS code, requestedThinking is ThinkingLevel, not ModelThinkingLevel, so "off" can't happen here.
            // Actually, it can because the parsing from cli_thinking... let me look again:
            // cliThinking is ThinkingLevel (from CLI flags). "off" is a ModelThinkingLevel, not ThinkingLevel.
            // So if cli_thinking is None and fallback_thinking parsed a suffix, it parses to ThinkingLevel.
            // So requestedThinking will never be "off" here. The "off" check is a no-op in practice for ThinkingLevel.
            // Let me simplify: just follow the TS logic literally.
            let ts_model = {
                let requested = cli_thinking.or(fallback_thinking);
                if requested.is_some() {
                    // In TS: requestedThinking && requestedThinking !== "off"
                    // Since ThinkingLevel can't be "off", this is just `requested.is_some()`
                    Model {
                        reasoning: true,
                        ..fallback_model.clone()
                    }
                } else {
                    fallback_model.clone()
                }
            };

            let fallback_warning = match &result.warning {
                Some(w) => format!(
                    "{w} Model \"{fallback_pattern}\" not found for provider \"{prov}\". Using custom model id."
                ),
                None => format!(
                    "Model \"{fallback_pattern}\" not found for provider \"{prov}\". Using custom model id."
                ),
            };

            return ResolveCliModelResult {
                model: Some(ts_model),
                thinking_level: fallback_thinking,
                warning: Some(fallback_warning),
                error: None,
            };
        }
    }

    let display = match provider {
        Some(p) => format!("{p}/{pattern}"),
        None => cli_model.to_string(),
    };

    ResolveCliModelResult {
        model: None,
        thinking_level: None,
        warning: result.warning,
        error: Some(format!(
            "Model \"{display}\" not found. Use --list-models to see available models."
        )),
    }
}

// ---------------------------------------------------------------------------
// InitialModelResult
// ---------------------------------------------------------------------------

/// Result of finding the initial model.
#[derive(Debug, Clone)]
pub struct InitialModelResult {
    pub model: Option<Model>,
    pub thinking_level: ThinkingLevel,
    pub fallback_message: Option<String>,
}

// ---------------------------------------------------------------------------
// find_initial_model
// ---------------------------------------------------------------------------

/// Find the initial model to use based on priority:
///
/// 1. CLI args (provider + model)  
/// 2. First model from scoped models (if not continuing/resuming)  
/// 3. Restored from session (if continuing/resuming)  
/// 4. Saved default from settings  
/// 5. First available model with valid API key
pub async fn find_initial_model(
    cli_provider: Option<&str>,
    cli_model: Option<&str>,
    scoped_models: &[ScopedModel],
    is_continuing: bool,
    default_provider: Option<&str>,
    default_model_id: Option<&str>,
    default_thinking_level: Option<ThinkingLevel>,
    model_registry: &dyn ModelRegistry,
) -> InitialModelResult {
    // 1. CLI args take priority
    if cli_provider.is_some() && cli_model.is_some() {
        let resolved = resolve_cli_model(cli_provider, cli_model, None, model_registry);
        if let Some(ref error) = resolved.error {
            eprintln!("\x1b[31m{error}\x1b[0m");
            std::process::exit(1);
        }
        if resolved.model.is_some() {
            return InitialModelResult {
                model: resolved.model,
                thinking_level: DEFAULT_THINKING_LEVEL,
                fallback_message: None,
            };
        }
    }

    // 2. Use first model from scoped models (skip if continuing/resuming)
    if !scoped_models.is_empty() && !is_continuing {
        return InitialModelResult {
            model: Some(scoped_models[0].model.clone()),
            thinking_level: scoped_models[0]
                .thinking_level
                .or(default_thinking_level)
                .unwrap_or(DEFAULT_THINKING_LEVEL),
            fallback_message: None,
        };
    }

    // 3. Try saved default from settings
    if let (Some(prov), Some(mid)) = (default_provider, default_model_id) {
        if let Some(found) = model_registry.find(prov, mid) {
            let level = default_thinking_level.unwrap_or(DEFAULT_THINKING_LEVEL);
            return InitialModelResult {
                model: Some(found),
                thinking_level: level,
                fallback_message: None,
            };
        }
    }

    // 4. Try first available model with valid API key
    let available_models = model_registry.get_available();

    if !available_models.is_empty() {
        // Try to find a default model from known providers
        let providers: Vec<String> = DEFAULT_MODEL_PER_PROVIDER
            .keys()
            .map(|k| k.to_string())
            .collect();
        for provider in &providers {
            if let Some(default_id) = default_model_for_provider(provider) {
                if let Some(match_model) = available_models
                    .iter()
                    .find(|m| m.provider == provider.as_str() && m.id == default_id)
                {
                    return InitialModelResult {
                        model: Some(match_model.clone()),
                        thinking_level: DEFAULT_THINKING_LEVEL,
                        fallback_message: None,
                    };
                }
            }
        }

        // If no default found, use first available
        return InitialModelResult {
            model: Some(available_models[0].clone()),
            thinking_level: DEFAULT_THINKING_LEVEL,
            fallback_message: None,
        };
    }

    // 5. No model found
    InitialModelResult {
        model: None,
        thinking_level: DEFAULT_THINKING_LEVEL,
        fallback_message: None,
    }
}

// ---------------------------------------------------------------------------
// restore_model_from_session
// ---------------------------------------------------------------------------

/// Restore model from session, with fallback to available models.
///
/// If the saved model can be found and has auth configured, use it.
/// Otherwise fall back to the current model or any available model.
pub async fn restore_model_from_session(
    saved_provider: &str,
    saved_model_id: &str,
    current_model: Option<&Model>,
    should_print_messages: bool,
    model_registry: &dyn ModelRegistry,
) -> RestoreModelResult {
    let restored_model = model_registry.find(saved_provider, saved_model_id);

    // Check if restored model exists and still has auth configured
    let has_configured_auth = restored_model
        .as_ref()
        .map(|m| model_registry.has_configured_auth(m))
        .unwrap_or(false);

    if restored_model.is_some() && has_configured_auth {
        if should_print_messages {
            eprintln!(
                "\x1b[2mRestored model: {}/{}\x1b[0m",
                saved_provider, saved_model_id
            );
        }
        return RestoreModelResult {
            model: restored_model,
            fallback_message: None,
        };
    }

    // Model not found or no API key — fall back
    let reason = if restored_model.is_none() {
        "model no longer exists"
    } else {
        "no auth configured"
    };

    if should_print_messages {
        eprintln!(
            "\x1b[33mWarning: Could not restore model {}/{} ({}).\x1b[0m",
            saved_provider, saved_model_id, reason
        );
    }

    // If we already have a model, use it as fallback
    if let Some(cm) = current_model {
        if should_print_messages {
            eprintln!("\x1b[2mFalling back to: {}/{}\x1b[0m", cm.provider, cm.id);
        }
        return RestoreModelResult {
            model: Some(cm.clone()),
            fallback_message: Some(format!(
                "Could not restore model {}/{} ({}). Using {}/{}",
                saved_provider, saved_model_id, reason, cm.provider, cm.id
            )),
        };
    }

    // Try to find any available model
    let available_models = model_registry.get_available();

    if !available_models.is_empty() {
        // Try to find a default model from known providers
        let mut fallback_model: Option<Model> = None;
        let providers: Vec<String> = DEFAULT_MODEL_PER_PROVIDER
            .keys()
            .map(|k| k.to_string())
            .collect();
        for provider in &providers {
            if let Some(default_id) = default_model_for_provider(provider) {
                if let Some(match_model) = available_models
                    .iter()
                    .find(|m| m.provider == provider.as_str() && m.id == default_id)
                {
                    fallback_model = Some(match_model.clone());
                    break;
                }
            }
        }

        // If no default found, use first available
        let fb = fallback_model.unwrap_or_else(|| available_models[0].clone());

        if should_print_messages {
            eprintln!("\x1b[2mFalling back to: {}/{}\x1b[0m", fb.provider, fb.id);
        }

        return RestoreModelResult {
            model: Some(fb.clone()),
            fallback_message: Some(format!(
                "Could not restore model {}/{} ({}). Using {}/{}",
                saved_provider, saved_model_id, reason, fb.provider, fb.id
            )),
        };
    }

    // No models available
    RestoreModelResult {
        model: None,
        fallback_message: None,
    }
}

/// Result from restoring a model from session.
pub struct RestoreModelResult {
    pub model: Option<Model>,
    pub fallback_message: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper to create a test model quickly.
    fn test_model(provider: &str, id: &str, api: hamr_ai::types::Api) -> Model {
        Model {
            id: id.into(),
            name: id.into(),
            api,
            provider: provider.into(),
            base_url: format!("https://{}.example.com", provider),
            reasoning: false,
            thinking_level_map: None,
            input: vec![hamr_ai::types::InputModality::Text],
            cost: hamr_ai::types::ModelCost {
                input: 1.0,
                output: 2.0,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 128000,
            max_tokens: 4096,
            headers: None,
            compat: None,
        }
    }

    /// Mock registry that simply returns a fixed set of models.
    struct MockRegistry {
        all_models: Vec<Model>,
        available_models: Vec<Model>,
    }

    impl MockRegistry {
        fn new(models: Vec<Model>) -> Self {
            let a = models.clone();
            Self {
                all_models: models,
                available_models: a,
            }
        }
    }

    impl ModelRegistry for MockRegistry {
        fn get_available(&self) -> Vec<Model> {
            self.available_models.clone()
        }
        fn get_all(&self) -> Vec<Model> {
            self.all_models.clone()
        }
        fn find(&self, provider: &str, model_id: &str) -> Option<Model> {
            self.all_models
                .iter()
                .find(|m| m.provider == provider && m.id == model_id)
                .cloned()
        }
        fn has_configured_auth(&self, _model: &Model) -> bool {
            true
        }
    }

    fn anthropic_models() -> Vec<Model> {
        vec![
            test_model(
                "anthropic",
                "claude-sonnet-4-5",
                hamr_ai::types::Api::AnthropicMessages,
            ),
            test_model(
                "anthropic",
                "claude-opus-4-8",
                hamr_ai::types::Api::AnthropicMessages,
            ),
        ]
    }

    fn all_test_models() -> Vec<Model> {
        let mut models = anthropic_models();
        models.push(test_model(
            "openai",
            "gpt-4o",
            hamr_ai::types::Api::OpenAiCompletions,
        ));
        models.push(test_model(
            "openrouter",
            "qwen/qwen3-coder:exacto",
            hamr_ai::types::Api::AnthropicMessages,
        ));
        models.push(test_model(
            "openrouter",
            "openai/gpt-4o:extended",
            hamr_ai::types::Api::AnthropicMessages,
        ));
        models
    }

    // --- default_model_for_provider ---

    #[test]
    fn test_default_model_for_anthropic() {
        let model = default_model_for_provider("anthropic");
        assert!(model.is_some());
    }

    #[test]
    fn test_default_model_for_unknown() {
        let model = default_model_for_provider("nonexistent");
        assert!(model.is_none());
    }

    // --- is_valid_thinking_level ---

    #[test]
    fn test_is_valid_thinking_level() {
        assert!(is_valid_thinking_level("off"));
        assert!(is_valid_thinking_level("low"));
        assert!(is_valid_thinking_level("medium"));
        assert!(is_valid_thinking_level("high"));
        assert!(!is_valid_thinking_level("invalid"));
        assert!(!is_valid_thinking_level(""));
    }

    // --- is_alias ---

    #[test]
    fn test_is_alias_latest() {
        assert!(is_alias("claude-sonnet-4-5-latest"));
    }

    #[test]
    fn test_is_alias_short_id() {
        assert!(is_alias("gpt-4"));
    }

    #[test]
    fn test_is_alias_dated_version() {
        assert!(!is_alias("claude-3-5-sonnet-20241022"));
    }

    #[test]
    fn test_is_alias_invalid_date_suffix() {
        assert!(is_alias("claude-3-5-sonnet-notadate"));
    }

    // --- minimatch ---

    #[test]
    fn test_minimatch_exact() {
        assert!(minimatch("hello", "hello"));
    }

    #[test]
    fn test_minimatch_case_insensitive() {
        assert!(minimatch("HELLO", "hello"));
    }

    #[test]
    fn test_minimatch_star_wildcard() {
        assert!(minimatch("hello world", "hello*"));
    }

    #[test]
    fn test_minimatch_question_mark() {
        assert!(minimatch("hello", "he?lo"));
    }

    #[test]
    fn test_minimatch_no_match() {
        assert!(!minimatch("hello", "world"));
    }

    #[test]
    fn test_minimatch_prefix_only() {
        assert!(!minimatch("hello", "hellox"));
    }

    #[test]
    fn test_minimatch_star_matches_everything() {
        assert!(minimatch("anything goes here", "*"));
    }

    // --- parse_thinking_level ---

    #[test]
    fn test_parse_thinking_level_low() {
        let tl = parse_thinking_level("low");
        assert!(tl.is_some());
    }

    #[test]
    fn test_parse_thinking_level_high() {
        let tl = parse_thinking_level("high");
        assert!(tl.is_some());
    }

    #[test]
    fn test_parse_thinking_level_invalid() {
        assert_eq!(parse_thinking_level("bogus"), None);
    }

    // --- parse_model_thinking_level ---

    #[test]
    fn test_parse_model_thinking_level_off() {
        let mtl = parse_model_thinking_level("off");
        assert!(mtl.is_some());
        assert!(matches!(mtl.unwrap(), ModelThinkingLevel::Off));
    }

    #[test]
    fn test_parse_model_thinking_level_high() {
        let mtl = parse_model_thinking_level("high");
        assert!(mtl.is_some());
    }

    #[test]
    fn test_parse_model_thinking_level_invalid() {
        assert_eq!(parse_model_thinking_level("bogus"), None);
    }

    // --- find_exact_model_reference_match ---

    #[test]
    fn test_find_exact_model_reference_match_not_found() {
        let result = find_exact_model_reference_match("nonexistent-model", &[]);
        assert!(result.is_none());
    }

    #[test]
    fn test_find_exact_model_reference_match_empty() {
        let result = find_exact_model_reference_match("", &[]);
        assert!(result.is_none());
    }

    #[test]
    fn test_find_exact_model_reference_match_bare_id() {
        let result = find_exact_model_reference_match("gpt-4o", &all_test_models());
        assert!(result.is_some());
        assert_eq!(result.unwrap().id, "gpt-4o");
    }

    #[test]
    fn test_find_exact_model_reference_match_canonical() {
        let result =
            find_exact_model_reference_match("anthropic/claude-sonnet-4-5", &all_test_models());
        assert!(result.is_some());
        assert_eq!(result.unwrap().id, "claude-sonnet-4-5");
    }

    #[test]
    fn test_find_exact_model_reference_match_ambiguous_bare_id() {
        // Two models with same id but different providers should be ambiguous
        let models = vec![
            test_model("p1", "same-id", hamr_ai::types::Api::AnthropicMessages),
            test_model("p2", "same-id", hamr_ai::types::Api::OpenAiCompletions),
        ];
        assert!(find_exact_model_reference_match("same-id", &models).is_none());
    }

    // --- build_fallback_model ---

    #[test]
    fn test_build_fallback_model_uses_default_when_available() {
        let models = all_test_models();
        let result = build_fallback_model("anthropic", "custom-model-id", &models);
        assert!(result.is_some());
        let m = result.unwrap();
        assert_eq!(m.id, "custom-model-id");
        // Should have base_url etc. from the default model
        assert!(m.base_url.contains("anthropic"));
    }

    #[test]
    fn test_build_fallback_model_no_provider_models() {
        let result = build_fallback_model("unknown-provider", "some-model", &all_test_models());
        assert!(result.is_none());
    }

    // --- parse_model_pattern ---

    #[test]
    fn test_parse_model_pattern_exact_match() {
        let models = all_test_models();
        let result = parse_model_pattern("claude-sonnet-4-5", &models, None);
        assert!(result.model.is_some());
        assert_eq!(result.model.unwrap().id, "claude-sonnet-4-5");
        assert!(result.thinking_level.is_none());
        assert!(result.warning.is_none());
    }

    #[test]
    fn test_parse_model_pattern_partial_match() {
        let models = all_test_models();
        let result = parse_model_pattern("sonnet", &models, None);
        assert!(result.model.is_some());
        assert_eq!(result.model.unwrap().id, "claude-sonnet-4-5");
    }

    #[test]
    fn test_parse_model_pattern_no_match() {
        let result = parse_model_pattern("nonexistent", &all_test_models(), None);
        assert!(result.model.is_none());
        assert!(result.thinking_level.is_none());
    }

    #[test]
    fn test_parse_model_pattern_with_valid_thinking_level() {
        let models = all_test_models();
        let result = parse_model_pattern("sonnet:high", &models, None);
        assert!(result.model.is_some());
        assert_eq!(result.model.unwrap().id, "claude-sonnet-4-5");
        assert_eq!(result.thinking_level, Some(ThinkingLevel::High));
        assert!(result.warning.is_none());
    }

    #[test]
    fn test_parse_model_pattern_with_invalid_thinking_level() {
        let models = all_test_models();
        let result = parse_model_pattern("sonnet:random", &models, None);
        assert!(result.model.is_some());
        assert_eq!(result.model.unwrap().id, "claude-sonnet-4-5");
        assert!(result.thinking_level.is_none());
        assert!(result.warning.is_some());
        assert!(
            result
                .warning
                .as_ref()
                .unwrap()
                .contains("Invalid thinking level")
        );
    }

    #[test]
    fn test_parse_model_pattern_openrouter_model_with_colon() {
        let models = all_test_models();
        let result = parse_model_pattern("qwen/qwen3-coder:exacto", &models, None);
        assert!(result.model.is_some());
        assert_eq!(result.model.unwrap().id, "qwen/qwen3-coder:exacto");
        assert!(result.thinking_level.is_none());
    }

    #[test]
    fn test_parse_model_pattern_openrouter_with_thinking() {
        let models = all_test_models();
        let result = parse_model_pattern("qwen/qwen3-coder:exacto:high", &models, None);
        assert!(result.model.is_some());
        assert_eq!(result.model.unwrap().id, "qwen/qwen3-coder:exacto");
        assert_eq!(result.thinking_level, Some(ThinkingLevel::High));
    }

    #[test]
    fn test_parse_model_pattern_strict_mode_invalid_suffix() {
        let models = all_test_models();
        let result = parse_model_pattern(
            "sonnet:random",
            &models,
            Some(&ParseModelPatternOptions {
                allow_invalid_thinking_level_fallback: Some(false),
            }),
        );
        // In strict mode, invalid suffix should NOT match
        assert!(result.model.is_none());
    }

    // --- resolve_cli_model ---

    #[test]
    fn test_resolve_cli_model_no_models() {
        let registry = MockRegistry::new(vec![]);
        let result = resolve_cli_model(None, Some("gpt-4"), None, &registry);
        assert!(result.model.is_none());
        assert!(result.error.is_some());
        assert!(
            result
                .error
                .as_ref()
                .unwrap()
                .contains("No models available")
        );
    }

    #[test]
    fn test_resolve_cli_model_provider_model() {
        let registry = MockRegistry::new(all_test_models());
        let result = resolve_cli_model(Some("anthropic"), Some("sonnet"), None, &registry);
        assert!(result.model.is_some());
        assert_eq!(result.model.as_ref().unwrap().provider, "anthropic");
    }

    #[test]
    fn test_resolve_cli_model_provider_slash_pattern() {
        let registry = MockRegistry::new(all_test_models());
        let result = resolve_cli_model(None, Some("anthropic/claude-sonnet-4-5"), None, &registry);
        assert!(result.model.is_some());
        assert_eq!(result.model.as_ref().unwrap().provider, "anthropic");
        assert_eq!(result.model.as_ref().unwrap().id, "claude-sonnet-4-5");
    }

    #[test]
    fn test_resolve_cli_model_with_thinking_in_pattern() {
        let registry = MockRegistry::new(anthropic_models());
        let result = resolve_cli_model(None, Some("claude-sonnet-4-5:high"), None, &registry);
        assert!(result.model.is_some());
        assert_eq!(result.model.as_ref().unwrap().id, "claude-sonnet-4-5");
        assert_eq!(result.thinking_level, Some(ThinkingLevel::High));
    }

    #[test]
    fn test_resolve_cli_model_unknown_provider() {
        let registry = MockRegistry::new(all_test_models());
        let result = resolve_cli_model(Some("nonexistent"), Some("model"), None, &registry);
        assert!(result.model.is_none());
        assert!(result.error.is_some());
        assert!(result.error.as_ref().unwrap().contains("Unknown provider"));
    }
}
