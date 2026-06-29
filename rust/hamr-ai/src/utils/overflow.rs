//! Port of `packages/ai/src/utils/overflow.ts`.
//!
//! Detect context-window overflow errors across providers. See the TS source
//! for the per-provider catalogue of error-message shapes each pattern targets.

use std::sync::LazyLock;

use regex::Regex;

use crate::types::{AssistantMessage, StopReason};

/// Compile a case-insensitive regex, panicking on an invalid literal pattern.
fn ci(pattern: &str) -> Regex {
    Regex::new(&format!("(?i){pattern}")).expect("overflow pattern is a valid regex")
}

/// Regex patterns that detect context overflow errors from different providers.
static OVERFLOW_PATTERNS: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    vec![
        ci(r"prompt is too long"),
        ci(r"request_too_large"),
        ci(r"input is too long for requested model"),
        ci(r"exceeds the context window"),
        ci(
            r"exceeds (?:the )?(?:model'?s )?maximum context length(?: of [\d,]+ tokens?|\s*\([\d,]+\))",
        ),
        ci(r"input token count.*exceeds the maximum"),
        ci(r"maximum prompt length is \d+"),
        ci(r"reduce the length of the messages"),
        ci(r"maximum context length is \d+ tokens"),
        ci(r"exceeds (?:the )?maximum allowed input length of [\d,]+ tokens?"),
        ci(r"input \(\d+ tokens\) is longer than the model'?s context length \(\d+ tokens\)"),
        ci(r"exceeds the limit of \d+"),
        ci(r"exceeds the available context size"),
        ci(r"greater than the context length"),
        ci(r"context window exceeds limit"),
        ci(r"exceeded model token limit"),
        ci(r"too large for model with \d+ maximum context length"),
        ci(r"model_context_window_exceeded"),
        ci(r"prompt too long; exceeded (?:max )?context length"),
        ci(r"context[_ ]length[_ ]exceeded"),
        ci(r"too many tokens"),
        ci(r"token limit exceeded"),
        ci(r"^4(?:00|13)\s*(?:status code)?\s*\(no body\)"),
    ]
});

/// Patterns that indicate non-overflow errors (rate limiting, server errors).
static NON_OVERFLOW_PATTERNS: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    vec![
        ci(r"^(Throttling error|Service unavailable):"),
        ci(r"rate limit"),
        ci(r"too many requests"),
    ]
});

/// Check if an assistant message represents a context overflow error.
///
/// `context_window` enables detection of silent overflow (z.ai style) and
/// length-stop overflow (Xiaomi MiMo style).
pub fn is_context_overflow(message: &AssistantMessage, context_window: Option<u64>) -> bool {
    // Case 1: Check error message patterns.
    if message.stop_reason == StopReason::Error {
        if let Some(error_message) = message.error_message.as_deref() {
            let is_non_overflow = NON_OVERFLOW_PATTERNS
                .iter()
                .any(|p| p.is_match(error_message));
            if !is_non_overflow && OVERFLOW_PATTERNS.iter().any(|p| p.is_match(error_message)) {
                return true;
            }
        }
    }

    if let Some(context_window) = context_window {
        // Case 2: Silent overflow (z.ai style) — successful but usage exceeds context.
        if message.stop_reason == StopReason::Stop {
            let input_tokens = message.usage.input + message.usage.cache_read;
            if input_tokens > context_window {
                return true;
            }
        }

        // Case 3: Length-stop overflow (Xiaomi MiMo style) — server truncates
        // oversized input to fit the context window, leaving no room for output.
        if message.stop_reason == StopReason::Length && message.usage.output == 0 {
            let input_tokens = message.usage.input + message.usage.cache_read;
            if (input_tokens as f64) >= (context_window as f64) * 0.99 {
                return true;
            }
        }
    }

    false
}

/// Get the overflow patterns for testing purposes.
pub fn get_overflow_patterns() -> Vec<&'static Regex> {
    OVERFLOW_PATTERNS.iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Usage, UsageCost};
    use chrono::Utc;

    fn usage(input: u64, cache_read: u64, output: u64) -> Usage {
        Usage {
            input,
            output,
            cache_read,
            cache_write: 0,
            cache_write_1h: None,
            total_tokens: input + cache_read + output,
            cost: UsageCost {
                input: 0.0,
                output: 0.0,
                cache_read: 0.0,
                cache_write: 0.0,
                total: 0.0,
            },
        }
    }

    fn msg(stop_reason: StopReason, error: Option<&str>, usage: Usage) -> AssistantMessage {
        AssistantMessage {
            role: crate::types::MessageRole::Assistant,
            content: vec![],
            api: "anthropic".into(),
            provider: "anthropic".into(),
            model: "claude".into(),
            response_model: None,
            response_id: None,
            usage,
            stop_reason,
            error_message: error.map(String::from),
            diagnostics: None,
            timestamp: Utc::now(),
        }
    }

    #[test]
    fn detects_anthropic_overflow() {
        let m = msg(
            StopReason::Error,
            Some("prompt is too long: 213462 tokens > 200000 maximum"),
            usage(0, 0, 0),
        );
        assert!(is_context_overflow(&m, None));
    }

    #[test]
    fn excludes_throttling() {
        // `formatBedrockError` emits the human-readable "Throttling error:" prefix,
        // which the NON_OVERFLOW_PATTERNS exclude even though it contains "too many tokens".
        let m = msg(
            StopReason::Error,
            Some("Throttling error: Too many tokens, please wait"),
            usage(0, 0, 0),
        );
        assert!(!is_context_overflow(&m, None));
    }

    #[test]
    fn detects_silent_overflow() {
        let m = msg(StopReason::Stop, None, usage(150_000, 60_000, 100));
        assert!(is_context_overflow(&m, Some(200_000)));
        assert!(!is_context_overflow(&m, None));
    }

    #[test]
    fn detects_length_stop_overflow() {
        let m = msg(StopReason::Length, None, usage(199_000, 1_000, 0));
        assert!(is_context_overflow(&m, Some(200_000)));
    }

    #[test]
    fn get_patterns_returns_full_set() {
        assert_eq!(get_overflow_patterns().len(), 23);
    }

    #[test]
    fn detects_explicit_ollama_prompt_too_long() {
        let m = msg(
            StopReason::Error,
            Some("400 `prompt too long; exceeded max context length by 100918 tokens`"),
            usage(0, 0, 0),
        );
        assert!(is_context_overflow(&m, Some(32768)));
    }

    #[test]
    fn detects_together_ai_context_length_error() {
        let m = msg(
            StopReason::Error,
            Some(
                "400 The input (516368 tokens) is longer than the model's context length (262144 tokens).",
            ),
            usage(0, 0, 0),
        );
        assert!(is_context_overflow(&m, Some(262144)));
    }

    #[test]
    fn detects_litellm_wrapped_openai_max_context_length() {
        let m = msg(
            StopReason::Error,
            Some(
                "Error: 503 litellm.ServiceUnavailableError: litellm.MidStreamFallbackError: \
                 litellm.APIConnectionError: APIConnectionError: OpenAIException - \
                 Requested token count exceeds the model's maximum context length of 131072 tokens.",
            ),
            usage(0, 0, 0),
        );
        assert!(is_context_overflow(&m, Some(131072)));
    }

    #[test]
    fn detects_openai_parenthesized_max_context_length() {
        let m = msg(
            StopReason::Error,
            Some(
                "Error: 400 Input length (265330) exceeds model's maximum context length (262144).",
            ),
            usage(0, 0, 0),
        );
        assert!(is_context_overflow(&m, Some(262144)));
    }

    #[test]
    fn detects_openrouter_poolside_max_allowed_input() {
        let m = msg(
            StopReason::Error,
            Some(
                "Provider returned error: Input length 131393 exceeds the maximum allowed input length of 131040 tokens.",
            ),
            usage(0, 0, 0),
        );
        assert!(is_context_overflow(&m, Some(131072)));
    }

    #[test]
    fn excludes_generic_non_overflow_ollama_error() {
        let m = msg(
            StopReason::Error,
            Some("500 `model runner crashed unexpectedly`"),
            usage(0, 0, 0),
        );
        assert!(!is_context_overflow(&m, Some(32768)));
    }

    #[test]
    fn excludes_bedrock_service_unavailable() {
        let m = msg(
            StopReason::Error,
            Some("Service unavailable: The service is temporarily unavailable."),
            usage(0, 0, 0),
        );
        assert!(!is_context_overflow(&m, Some(200000)));
    }

    #[test]
    fn excludes_generic_rate_limit() {
        let m = msg(
            StopReason::Error,
            Some("Rate limit exceeded, please retry after 30 seconds."),
            usage(0, 0, 0),
        );
        assert!(!is_context_overflow(&m, Some(200000)));
    }

    #[test]
    fn excludes_http_429_style_error() {
        let m = msg(
            StopReason::Error,
            Some("Too many requests. Please slow down."),
            usage(0, 0, 0),
        );
        assert!(!is_context_overflow(&m, Some(200000)));
    }

    #[test]
    fn detects_xiaomi_style_length_stop_overflow() {
        // Xiaomi truncates input to fill context, leaving output=0
        let m = msg(StopReason::Length, None, usage(58, 1048512, 0));
        assert!(is_context_overflow(&m, Some(1048576)));
        // context_window = input + cache_read = 58 + 1048512 = 1048570 ≅ 99.999% of 1048576
    }

    #[test]
    fn excludes_normal_length_stop_with_output() {
        let m = msg(StopReason::Length, None, usage(1000, 0, 4096));
        assert!(!is_context_overflow(&m, Some(200000)));
    }

    #[test]
    fn excludes_length_stop_far_below_context() {
        let m = msg(StopReason::Length, None, usage(100, 0, 0));
        assert!(!is_context_overflow(&m, Some(200000)));
    }
}
