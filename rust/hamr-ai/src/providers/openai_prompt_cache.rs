//! Port of `packages/ai/src/providers/openai-prompt-cache.ts`.

/// Maximum length (in Unicode code points) of an OpenAI prompt cache key.
pub const OPENAI_PROMPT_CACHE_KEY_MAX_LENGTH: usize = 64;

/// Clamp a prompt cache key to [`OPENAI_PROMPT_CACHE_KEY_MAX_LENGTH`] code points.
///
/// Mirrors the TS `Array.from(key)` semantics: counts and truncates by Unicode
/// scalar value, not UTF-16 units or bytes.
pub fn clamp_openai_prompt_cache_key(key: Option<&str>) -> Option<String> {
    let key = key?;
    let chars: Vec<char> = key.chars().collect();
    if chars.len() <= OPENAI_PROMPT_CACHE_KEY_MAX_LENGTH {
        Some(key.to_string())
    } else {
        Some(chars[..OPENAI_PROMPT_CACHE_KEY_MAX_LENGTH].iter().collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn none_passes_through() {
        assert_eq!(clamp_openai_prompt_cache_key(None), None);
    }

    #[test]
    fn short_key_unchanged() {
        assert_eq!(
            clamp_openai_prompt_cache_key(Some("abc")),
            Some("abc".to_string())
        );
    }

    #[test]
    fn long_key_truncated_to_64_code_points() {
        let key: String = "x".repeat(100);
        let clamped = clamp_openai_prompt_cache_key(Some(&key)).unwrap();
        assert_eq!(clamped.chars().count(), 64);
    }

    #[test]
    fn truncates_by_code_point_not_byte() {
        let key: String = "é".repeat(70); // 2 bytes each
        let clamped = clamp_openai_prompt_cache_key(Some(&key)).unwrap();
        assert_eq!(clamped.chars().count(), 64);
    }
}
