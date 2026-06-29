//! Port of `packages/coding-agent/src/modes/interactive/env_expand.ts`.
//!
//! Expand environment-variable references in a custom-endpoint config value
//! before it is used for a discovery network call.

use regex::Regex;
use std::collections::HashMap;

/// Expand environment-variable references in a custom-endpoint config value
/// before it is used for a discovery network call.
///
/// Secret headers are stored as a single-dollar reference derived from the
/// header key (see `saveEndpointToModelsJson` in interactive-mode.ts), e.g.
/// header `CF-Access-Client-Id` is saved as the literal string
/// `$CF_ACCESS_CLIENT_ID`. A double-dollar `$$VAR` form is also accepted
/// defensively: one leading `$` is stripped and the remaining `$VAR` is
/// expanded normally.
///
/// Both `${VAR}` and `$VAR` forms are expanded from `env`. References to
/// unset variables expand to "".
pub fn expand_env_for_discovery(value: &str, env: &HashMap<String, String>) -> String {
    let mut value = value.to_owned();

    // Defensive: a `$$VAR` form strips one leading `$` so the remaining
    // `${VAR}` or `$VAR` is expanded normally.
    if value.starts_with("$$") {
        value = value[1..].to_owned();
    }

    // Expand ${VAR} form
    let re_braced = Regex::new(r"\$\{(\w+)\}").unwrap();
    let result = re_braced.replace_all(&value, |caps: &regex::Captures| {
        let name = caps.get(1).unwrap().as_str();
        env.get(name).cloned().unwrap_or_default()
    });

    // Expand $VAR form
    let re_simple = Regex::new(r"\$(\w+)").unwrap();
    let result = re_simple.replace_all(&result, |caps: &regex::Captures| {
        let name = caps.get(1).unwrap().as_str();
        env.get(name).cloned().unwrap_or_default()
    });

    result.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_env(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    #[test]
    fn test_braced_var() {
        let env = make_env(&[("TOKEN", "secret123")]);
        assert_eq!(
            expand_env_for_discovery("Bearer ${TOKEN}", &env),
            "Bearer secret123"
        );
    }

    #[test]
    fn test_simple_var() {
        let env = make_env(&[("KEY", "mykey")]);
        assert_eq!(expand_env_for_discovery("$KEY", &env), "mykey");
    }

    #[test]
    fn test_double_dollar() {
        let env = make_env(&[("SECRET", "sauce")]);
        assert_eq!(expand_env_for_discovery("$$SECRET", &env), "sauce");
    }

    #[test]
    fn test_double_dollar_braced() {
        let env = make_env(&[("SECRET", "sauce")]);
        assert_eq!(expand_env_for_discovery("$${SECRET}", &env), "sauce");
    }

    #[test]
    fn test_unset_var_expands_to_empty() {
        let env = make_env(&[]);
        assert_eq!(
            expand_env_for_discovery("prefix ${MISSING} suffix", &env),
            "prefix  suffix"
        );
    }

    #[test]
    fn test_multiple_vars() {
        let env = make_env(&[("A", "1"), ("B", "2")]);
        assert_eq!(expand_env_for_discovery("$A ${B}", &env), "1 2");
    }

    #[test]
    fn test_no_var_passes_through() {
        let env = make_env(&[]);
        assert_eq!(
            expand_env_for_discovery("no vars here", &env),
            "no vars here"
        );
    }
}
