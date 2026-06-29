//! Port of `packages/ai/src/utils/provider_env.ts`
//!
//! Resolve provider-scoped environment values with precedence:
//! 1. Provider-env override map (`ProviderEnv`)
//! 2. `std::env::var()` (regular process environment)
//!
//! The Bun sandbox fallback (`/proc/self/environ` read) from the TS is not
//! needed in Rust — `std::env::var()` works reliably across platforms.

use crate::types::ProviderEnv;

/// Resolve a provider env value from scoped overrides first, then
/// falling back to `std::env::var`.
///
/// Mirrors `getProviderEnvValue` in TS (without the Bun sandbox workaround).
pub fn get_provider_env_value(name: &str, env: Option<&ProviderEnv>) -> Option<String> {
    if let Some(provider_env) = env {
        if let Some(value) = provider_env.get(name) {
            return Some(value.clone());
        }
    }
    std::env::var(name).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prefers_provider_env_over_process_env() {
        unsafe { std::env::set_var("TEST_PROVIDER_ENV_VAR", "from-process") };
        let mut provider_env = ProviderEnv::new();
        provider_env.insert(
            "TEST_PROVIDER_ENV_VAR".to_string(),
            "from-provider".to_string(),
        );
        let result = get_provider_env_value("TEST_PROVIDER_ENV_VAR", Some(&provider_env));
        assert_eq!(result, Some("from-provider".to_string()));
        unsafe { std::env::remove_var("TEST_PROVIDER_ENV_VAR") };
    }

    #[test]
    fn test_falls_back_to_process_env() {
        unsafe { std::env::set_var("TEST_PROVIDER_ENV_VAR_2", "from-process") };
        let provider_env = ProviderEnv::new();
        let result = get_provider_env_value("TEST_PROVIDER_ENV_VAR_2", Some(&provider_env));
        assert_eq!(result, Some("from-process".to_string()));
        unsafe { std::env::remove_var("TEST_PROVIDER_ENV_VAR_2") };
    }

    #[test]
    fn test_returns_none_when_missing() {
        let result = get_provider_env_value("NONEXISTENT_VAR_12345", None);
        assert_eq!(result, None);
    }

    #[test]
    fn test_env_is_none_falls_back_to_process() {
        unsafe { std::env::set_var("TEST_PROVIDER_ENV_VAR_3", "from-process") };
        let result = get_provider_env_value("TEST_PROVIDER_ENV_VAR_3", None);
        assert_eq!(result, Some("from-process".to_string()));
        unsafe { std::env::remove_var("TEST_PROVIDER_ENV_VAR_3") };
    }
}
