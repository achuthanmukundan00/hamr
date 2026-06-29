//! Resolve configuration values that may be shell commands, environment variables, or literals.
//!
//! Used by auth_storage and model_registry.
//!
//! Config value resolution rules:
//! - If the value starts with `!`, the remainder is executed as a shell command (stdout is cached).
//! - Otherwise, `$VAR` and `${VAR}` references are interpolated from the environment.
//! - `$$` escapes to a literal `$` and `$!` escapes to a literal `!`.
//!
//! Ported from `packages/coding-agent/src/core/resolve-config-value.ts`.

use std::collections::HashMap;
use std::process::{Command, Stdio};
use std::sync::Mutex;
use std::thread;
use std::time::Duration;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// A part of a template value: either a literal string or an environment variable reference.
#[derive(Debug, Clone, PartialEq)]
pub enum TemplatePart {
    Literal(String),
    Env(String),
}

/// A resolved config value reference: either a shell command or a template with env var interpolation.
#[derive(Debug, Clone, PartialEq)]
pub enum ConfigValueReference {
    Command(String),
    Template(Vec<TemplatePart>),
}

// ---------------------------------------------------------------------------
// Regex helpers (avoid pulling in regex crate for simple character checks)
// ---------------------------------------------------------------------------

/// Matches a full valid environment variable name: `[A-Za-z_][A-Za-z0-9_]*`
fn is_env_var_name(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }
    let mut chars = s.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() || c == '_' => {}
        _ => return false,
    }
    for c in chars {
        if !c.is_ascii_alphanumeric() && c != '_' {
            return false;
        }
    }
    true
}

/// Returns the longest prefix of `s` that is a valid env var name (starting at index 0).
fn env_var_name_prefix(s: &str) -> Option<&str> {
    let mut end = 0;
    for (i, c) in s.char_indices() {
        if i == 0 {
            if !c.is_ascii_alphabetic() && c != '_' {
                return None;
            }
            end = c.len_utf8();
        } else if c.is_ascii_alphanumeric() || c == '_' {
            end = i + c.len_utf8();
        } else {
            break;
        }
    }
    if end == 0 { None } else { Some(&s[..end]) }
}

// ---------------------------------------------------------------------------
// Template parsing
// ---------------------------------------------------------------------------

/// Parse a template string into a list of `TemplatePart` values.
///
/// Handles `$VAR`, `${VAR}`, `$$`, and `$!` syntax.
pub fn parse_config_value_template(config: &str) -> Vec<TemplatePart> {
    let mut parts: Vec<TemplatePart> = Vec::new();
    let mut index = 0;
    let config_bytes = config.as_bytes();

    while index < config.len() {
        // Find next '$'
        let dollar_pos = match config[index..].find('$') {
            Some(p) => index + p,
            None => {
                // No more dollar signs — append rest as literal
                append_literal(&mut parts, &config[index..]);
                break;
            }
        };

        // Append text before '$'
        append_literal(&mut parts, &config[index..dollar_pos]);

        let after_dollar = dollar_pos + 1;
        if after_dollar >= config.len() {
            // Trailing '$' — treat as literal
            append_literal(&mut parts, "$");
            break;
        }

        let next_byte = config_bytes[after_dollar];

        // $$ or $! — literal escape
        if next_byte == b'$' || next_byte == b'!' {
            append_literal(&mut parts, &config[after_dollar..after_dollar + 1]);
            index = after_dollar + 1;
            continue;
        }

        // ${VAR}
        if next_byte == b'{' {
            let close = match config[after_dollar + 1..].find('}') {
                Some(p) => after_dollar + 1 + p,
                None => {
                    // No closing brace — treat '$' as literal
                    append_literal(&mut parts, "$");
                    index = dollar_pos + 1;
                    continue;
                }
            };
            let name = &config[after_dollar + 1..close];
            if is_env_var_name(name) {
                parts.push(TemplatePart::Env(name.to_string()));
            } else {
                // Invalid env var name — treat entire ${...} as literal
                append_literal(&mut parts, &config[dollar_pos..close + 1]);
            }
            index = close + 1;
            continue;
        }

        // $VAR (simple name)
        let remaining = &config[after_dollar..];
        if let Some(prefix) = env_var_name_prefix(remaining) {
            parts.push(TemplatePart::Env(prefix.to_string()));
            index = after_dollar + prefix.len();
            continue;
        }

        // Lone '$' not followed by a valid reference — literal
        append_literal(&mut parts, "$");
        index = dollar_pos + 1;
    }

    parts
}

/// Append a literal string to the parts list, merging with the previous part if it is also literal.
fn append_literal(parts: &mut Vec<TemplatePart>, value: &str) {
    if value.is_empty() {
        return;
    }
    match parts.last_mut() {
        Some(TemplatePart::Literal(existing)) => {
            existing.push_str(value);
        }
        _ => {
            parts.push(TemplatePart::Literal(value.to_string()));
        }
    }
}

// ---------------------------------------------------------------------------
// Parsing
// ---------------------------------------------------------------------------

/// Parse a config value string into a `ConfigValueReference`.
///
/// If the string starts with `!`, it's treated as a command; otherwise as a template.
pub fn parse_config_value_reference(config: &str) -> ConfigValueReference {
    if config.starts_with('!') {
        return ConfigValueReference::Command(config.to_string());
    }
    ConfigValueReference::Template(parse_config_value_template(config))
}

// ---------------------------------------------------------------------------
// Cached command execution
// ---------------------------------------------------------------------------

/// Global cache for shell command results.
static COMMAND_RESULT_CACHE: std::sync::LazyLock<Mutex<HashMap<String, Option<String>>>> =
    std::sync::LazyLock::new(|| Mutex::new(HashMap::new()));

/// Clear the command result cache. Exported for testing.
pub fn clear_config_value_cache() {
    if let Ok(mut cache) = COMMAND_RESULT_CACHE.lock() {
        cache.clear();
    }
}

/// Execute a shell command (without `!` prefix) using the default system shell.
/// Runs the command in a separate thread with a 10-second timeout.
fn execute_command_unix(command: &str) -> Option<String> {
    let command = command.to_string();
    let (tx, rx) = std::sync::mpsc::channel();
    thread::spawn(move || {
        // Try /bin/bash -c, then sh -c
        for shell in &["/bin/bash", "sh"] {
            let result = Command::new(shell)
                .arg("-c")
                .arg(&command)
                .stdout(Stdio::piped())
                .stderr(Stdio::null())
                .stdin(Stdio::null())
                .output();

            match result {
                Ok(output) if output.status.success() => {
                    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
                    let _ = tx.send(Some(stdout));
                    return;
                }
                Ok(_) => {
                    // Non-zero exit — fall through to next shell
                    continue;
                }
                Err(_) => {
                    // Command not found — try next shell
                    continue;
                }
            }
        }
        let _ = tx.send(None);
    });

    // Wait with a 10-second timeout (matching TypeScript's timeout: 10000)
    match rx.recv_timeout(Duration::from_secs(10)) {
        Ok(result) => result,
        Err(_) => None,
    }
}

/// Execute a command (as raw string with `!` prefix) and cache the result.
fn execute_command(command_config: &str) -> Option<String> {
    // Check cache
    if let Ok(cache) = COMMAND_RESULT_CACHE.lock() {
        if let Some(val) = cache.get(command_config) {
            return val.clone();
        }
    }

    let command = &command_config[1..]; // strip '!'
    let result = execute_command_unix(command);

    // Store in cache
    if let Ok(mut cache) = COMMAND_RESULT_CACHE.lock() {
        cache.insert(command_config.to_string(), result.clone());
    }

    result
}

/// Execute a command without caching.
fn execute_command_uncached(command_config: &str) -> Option<String> {
    let command = &command_config[1..]; // strip '!'
    execute_command_unix(command)
}

// ---------------------------------------------------------------------------
// Environment variable resolution
// ---------------------------------------------------------------------------

/// Resolve a single environment variable by name.
fn resolve_env_var(name: &str, env: Option<&HashMap<String, String>>) -> Option<String> {
    if let Some(env_map) = env {
        if let Some(val) = env_map.get(name) {
            return Some(val.clone());
        }
    }
    // Fall back to process environment
    std::env::var(name).ok()
}

/// Get all unique env var names referenced in a template.
fn get_template_env_var_names(parts: &[TemplatePart]) -> Vec<String> {
    let mut names: Vec<String> = Vec::new();
    for part in parts {
        if let TemplatePart::Env(name) = part {
            if !names.contains(name) {
                names.push(name.clone());
            }
        }
    }
    names
}

/// Resolve a template into its final string value by interpolating environment variables.
/// Returns `None` if any referenced env var is missing.
fn resolve_template(
    parts: &[TemplatePart],
    env: Option<&HashMap<String, String>>,
) -> Option<String> {
    let mut result = String::new();
    for part in parts {
        match part {
            TemplatePart::Literal(s) => result.push_str(s),
            TemplatePart::Env(name) => {
                let val = resolve_env_var(name, env)?;
                result.push_str(&val);
            }
        }
    }
    Some(result)
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Get the single env var name if the config is a simple env var reference (e.g., `$MY_KEY`);
/// returns `None` if the config is a command, template, or literal.
pub fn get_config_value_env_var_name(config: &str) -> Option<String> {
    let reference = parse_config_value_reference(config);
    match reference {
        ConfigValueReference::Command(_) => None,
        ConfigValueReference::Template(parts) => {
            if parts.len() == 1 {
                if let TemplatePart::Env(name) = &parts[0] {
                    return Some(name.clone());
                }
            }
            None
        }
    }
}

/// Get all environment variable names referenced in a config value.
pub fn get_config_value_env_var_names(config: &str) -> Vec<String> {
    let reference = parse_config_value_reference(config);
    match reference {
        ConfigValueReference::Command(_) => vec![],
        ConfigValueReference::Template(parts) => get_template_env_var_names(&parts),
    }
}

/// Get the names of environment variables referenced by `config` that are missing from `env`.
pub fn get_missing_config_value_env_var_names(
    config: &str,
    env: Option<&HashMap<String, String>>,
) -> Vec<String> {
    get_config_value_env_var_names(config)
        .into_iter()
        .filter(|name| resolve_env_var(name, env).is_none())
        .collect()
}

/// Returns `true` if the config value is a shell command (starts with `!`).
pub fn is_command_config_value(config: &str) -> bool {
    matches!(
        parse_config_value_reference(config),
        ConfigValueReference::Command(_)
    )
}

/// Returns `true` if the config value can be resolved (all referenced env vars are present).
pub fn is_config_value_configured(config: &str, env: Option<&HashMap<String, String>>) -> bool {
    get_missing_config_value_env_var_names(config, env).is_empty()
}

/// Resolve a config value (API key, header value, etc.) to an actual string.
///
/// - If `config` starts with `!`, executes the remainder as a shell command (cached).
/// - Interpolates `$ENV_VAR` or `${ENV_VAR}` references with environment variables.
/// - `$$` and `$!` produce literal `$` and `!` respectively.
/// - Otherwise treats the value as a literal.
pub fn resolve_config_value(config: &str, env: Option<&HashMap<String, String>>) -> Option<String> {
    let reference = parse_config_value_reference(config);
    match reference {
        ConfigValueReference::Command(ref cmd) => execute_command(cmd),
        ConfigValueReference::Template(ref parts) => resolve_template(parts, env),
    }
}

/// Resolve a config value without using the command cache.
pub fn resolve_config_value_uncached(
    config: &str,
    env: Option<&HashMap<String, String>>,
) -> Option<String> {
    let reference = parse_config_value_reference(config);
    match reference {
        ConfigValueReference::Command(ref cmd) => execute_command_uncached(cmd),
        ConfigValueReference::Template(ref parts) => resolve_template(parts, env),
    }
}

/// Resolve a config value, returning an error message if resolution fails.
pub fn resolve_config_value_or_throw(
    config: &str,
    description: &str,
    env: Option<&HashMap<String, String>>,
) -> Result<String, String> {
    let resolved = resolve_config_value_uncached(config, env);
    if let Some(val) = resolved {
        return Ok(val);
    }

    let reference = parse_config_value_reference(config);
    match reference {
        ConfigValueReference::Command(cmd) => Err(format!(
            "Failed to resolve {description} from shell command: {}",
            &cmd[1..]
        )),
        ConfigValueReference::Template(_parts) => {
            let missing = get_missing_config_value_env_var_names(config, env);
            if missing.len() == 1 {
                Err(format!(
                    "Failed to resolve {description} from environment variable: {}",
                    missing[0]
                ))
            } else if missing.len() > 1 {
                Err(format!(
                    "Failed to resolve {description} from environment variables: {}",
                    missing.join(", ")
                ))
            } else {
                Err(format!("Failed to resolve {description}"))
            }
        }
    }
}

/// Resolve all header values using the same resolution logic as API keys.
///
/// Returns `None` if headers is `None` or empty after resolution.
pub fn resolve_headers(
    headers: Option<&HashMap<String, String>>,
    env: Option<&HashMap<String, String>>,
) -> Option<HashMap<String, String>> {
    let headers = headers?;
    let mut resolved: HashMap<String, String> = HashMap::new();
    for (key, value) in headers {
        if let Some(resolved_value) = resolve_config_value(value, env) {
            if !resolved_value.is_empty() {
                resolved.insert(key.clone(), resolved_value);
            }
        }
    }
    if resolved.is_empty() {
        None
    } else {
        Some(resolved)
    }
}

/// Resolve all header values, throwing on any resolution failure.
///
/// Returns `None` if headers is `None` or empty after resolution.
pub fn resolve_headers_or_throw(
    headers: Option<&HashMap<String, String>>,
    description: &str,
    env: Option<&HashMap<String, String>>,
) -> Result<Option<HashMap<String, String>>, String> {
    let headers = match headers {
        Some(h) => h,
        None => return Ok(None),
    };
    let mut resolved: HashMap<String, String> = HashMap::new();
    for (key, value) in headers {
        let resolved_value =
            resolve_config_value_or_throw(value, &format!("{description} header \"{key}\""), env)?;
        resolved.insert(key.clone(), resolved_value);
    }
    if resolved.is_empty() {
        Ok(None)
    } else {
        Ok(Some(resolved))
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn empty_env() -> Option<&'static HashMap<String, String>> {
        None
    }

    fn env_with(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    // --- parse_config_value_reference ---

    #[test]
    fn test_parse_command() {
        assert_eq!(
            parse_config_value_reference("!echo hello"),
            ConfigValueReference::Command("!echo hello".to_string())
        );
    }

    #[test]
    fn test_parse_literal() {
        assert_eq!(
            parse_config_value_reference("hello world"),
            ConfigValueReference::Template(vec![TemplatePart::Literal("hello world".to_string())])
        );
    }

    #[test]
    fn test_parse_env_var() {
        assert_eq!(
            parse_config_value_reference("$HOME"),
            ConfigValueReference::Template(vec![TemplatePart::Env("HOME".to_string())])
        );
    }

    #[test]
    fn test_parse_env_var_braces() {
        assert_eq!(
            parse_config_value_reference("${HOME}"),
            ConfigValueReference::Template(vec![TemplatePart::Env("HOME".to_string())])
        );
    }

    #[test]
    fn test_parse_dollar_escape() {
        assert_eq!(
            parse_config_value_reference("$$"),
            ConfigValueReference::Template(vec![TemplatePart::Literal("$".to_string())])
        );
    }

    #[test]
    fn test_parse_dollar_bang_escape() {
        assert_eq!(
            parse_config_value_reference("$!"),
            ConfigValueReference::Template(vec![TemplatePart::Literal("!".to_string())])
        );
    }

    #[test]
    fn test_parse_mixed() {
        let result = parse_config_value_reference("hello $USER at ${HOSTNAME}");
        assert_eq!(
            result,
            ConfigValueReference::Template(vec![
                TemplatePart::Literal("hello ".to_string()),
                TemplatePart::Env("USER".to_string()),
                TemplatePart::Literal(" at ".to_string()),
                TemplatePart::Env("HOSTNAME".to_string()),
            ])
        );
    }

    #[test]
    fn test_parse_double_dollar_in_middle() {
        let result = parse_config_value_reference("prefix$$middle");
        assert_eq!(
            result,
            ConfigValueReference::Template(vec![TemplatePart::Literal(
                "prefix$middle".to_string()
            ),])
        );
    }

    // --- get_config_value_env_var_name ---

    #[test]
    fn test_simple_env_var_name() {
        assert_eq!(
            get_config_value_env_var_name("$MY_KEY"),
            Some("MY_KEY".to_string())
        );
    }

    #[test]
    fn test_simple_env_var_name_braces() {
        assert_eq!(
            get_config_value_env_var_name("${MY_KEY}"),
            Some("MY_KEY".to_string())
        );
    }

    #[test]
    fn test_not_simple_env_var() {
        assert_eq!(get_config_value_env_var_name("prefix$MY_KEY"), None);
        assert_eq!(get_config_value_env_var_name("!cmd"), None);
        assert_eq!(get_config_value_env_var_name("literal"), None);
    }

    // --- get_config_value_env_var_names ---

    #[test]
    fn test_env_var_names() {
        let names = get_config_value_env_var_names("$A and ${B} and $C");
        assert_eq!(names, vec!["A", "B", "C"]);
    }

    #[test]
    fn test_command_has_no_env_vars() {
        let names = get_config_value_env_var_names("!echo hi");
        assert!(names.is_empty());
    }

    // --- get_missing_config_value_env_var_names ---

    #[test]
    fn test_missing_env_var_names() {
        let env = env_with(&[("A", "1")]);
        let missing = get_missing_config_value_env_var_names("$A $B", Some(&env));
        assert_eq!(missing, vec!["B"]);
    }

    // --- is_command_config_value ---

    #[test]
    fn test_is_command() {
        assert!(is_command_config_value("!echo hi"));
        assert!(!is_command_config_value("echo hi"));
        assert!(!is_command_config_value("$VAR"));
    }

    // --- is_config_value_configured ---

    #[test]
    fn test_configured() {
        let env = env_with(&[("A", "1"), ("B", "2")]);
        assert!(is_config_value_configured("$A $B", Some(&env)));
        assert!(!is_config_value_configured("$A $C", Some(&env)));
    }

    // --- resolve_config_value / resolve_config_value_uncached ---

    #[test]
    fn test_resolve_literal() {
        assert_eq!(
            resolve_config_value("hello", empty_env()),
            Some("hello".to_string())
        );
    }

    #[test]
    fn test_resolve_env_var() {
        let env = env_with(&[("MY_KEY", "secret123")]);
        assert_eq!(
            resolve_config_value("${MY_KEY}", Some(&env)),
            Some("secret123".to_string())
        );
    }

    #[test]
    fn test_resolve_env_var_fallback_to_process() {
        // HOME should be available in any environment — resolves via process env
        let result = resolve_config_value("$HOME", empty_env());
        assert!(result.is_some());
    }

    #[test]
    fn test_resolve_missing_env_var() {
        let result = resolve_config_value("$NONEXISTENT_VAR_XYZ", empty_env());
        assert_eq!(result, None);
    }

    #[test]
    fn test_resolve_template_with_env() {
        let env = env_with(&[("USER", "alice"), ("HOST", "localhost")]);
        assert_eq!(
            resolve_config_value("$USER@${HOST}", Some(&env)),
            Some("alice@localhost".to_string())
        );
    }

    #[test]
    fn test_resolve_dollar_escape_in_template() {
        assert_eq!(
            resolve_config_value("$$VAR", empty_env()),
            Some("$VAR".to_string())
        );
    }

    #[test]
    fn test_resolve_dollar_bang_escape_in_template() {
        assert_eq!(
            resolve_config_value("$!command", empty_env()),
            Some("!command".to_string())
        );
    }

    #[test]
    fn test_resolve_command() {
        let result = resolve_config_value("!echo hello world", empty_env());
        assert_eq!(result, Some("hello world".to_string()));
    }

    #[test]
    fn test_resolve_command_failure() {
        let result = resolve_config_value("!nonexistent_command_xyz123", empty_env());
        assert_eq!(result, None);
    }

    #[test]
    fn test_resolve_uncached_not_same_as_cached() {
        // Clear cache first
        clear_config_value_cache();

        // resolve_config_value should cache
        let _ = resolve_config_value("!echo cached_test", empty_env());
        // resolve_config_value_uncached should work independently
        let result = resolve_config_value_uncached("!echo cached_test", empty_env());
        assert_eq!(result, Some("cached_test".to_string()));
    }

    // --- resolve_config_value_or_throw ---

    #[test]
    fn test_resolve_or_throw_ok() {
        let env = env_with(&[("KEY", "val")]);
        let result = resolve_config_value_or_throw("$KEY", "test", Some(&env));
        assert_eq!(result, Ok("val".to_string()));
    }

    #[test]
    fn test_resolve_or_throw_missing_env() {
        let result = resolve_config_value_or_throw("$MISSING", "test", empty_env());
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("MISSING"));
    }

    #[test]
    fn test_resolve_or_throw_missing_multiple_env() {
        let result = resolve_config_value_or_throw("$A $B", "test", empty_env());
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("A") && err.contains("B"));
    }

    #[test]
    fn test_resolve_or_throw_command() {
        let result = resolve_config_value_or_throw("!true", "test", empty_env());
        assert!(result.is_ok());
    }

    #[test]
    fn test_resolve_or_throw_command_failure() {
        let result = resolve_config_value_or_throw("!nonexistent_cmd_xyz", "test", empty_env());
        assert!(result.is_err());
    }

    // --- resolve_headers ---

    #[test]
    fn test_resolve_headers_literal() {
        let headers = env_with(&[("Authorization", "Bearer token123")]);
        let result = resolve_headers(Some(&headers), empty_env());
        assert_eq!(
            result,
            Some(env_with(&[("Authorization", "Bearer token123")]))
        );
    }

    #[test]
    fn test_resolve_headers_with_env() {
        let headers = env_with(&[("X-API-Key", "${API_KEY}")]);
        let env = env_with(&[("API_KEY", "supersecret")]);
        let result = resolve_headers(Some(&headers), Some(&env));
        assert_eq!(result, Some(env_with(&[("X-API-Key", "supersecret")])));
    }

    #[test]
    fn test_resolve_headers_none_input() {
        assert_eq!(resolve_headers(None, empty_env()), None);
    }

    #[test]
    fn test_resolve_headers_removes_unresolvable() {
        let headers = env_with(&[("Key", "$MISSING")]);
        let result = resolve_headers(Some(&headers), empty_env());
        assert_eq!(result, None);
    }

    // --- resolve_headers_or_throw ---

    #[test]
    fn test_resolve_headers_or_throw_ok() {
        let headers = env_with(&[("Key", "value")]);
        let result = resolve_headers_or_throw(Some(&headers), "test", empty_env());
        assert_eq!(result, Ok(Some(env_with(&[("Key", "value")]))));
    }

    #[test]
    fn test_resolve_headers_or_throw_none() {
        let result = resolve_headers_or_throw(None, "test", empty_env());
        assert_eq!(result, Ok(None));
    }

    #[test]
    fn test_resolve_headers_or_throw_error() {
        let headers = env_with(&[("Key", "$MISSING")]);
        let result = resolve_headers_or_throw(Some(&headers), "test", empty_env());
        assert!(result.is_err());
    }

    // --- clear_config_value_cache ---

    #[test]
    fn test_clear_cache() {
        // Run a command to populate cache
        let r1 = resolve_config_value("!echo cache_clear_test", empty_env());
        assert_eq!(r1, Some("cache_clear_test".to_string()));

        // Clear cache
        clear_config_value_cache();

        // Verify cache is cleared by re-running (should still work)
        let r2 = resolve_config_value("!echo cache_clear_test", empty_env());
        assert_eq!(r2, Some("cache_clear_test".to_string()));
    }
}
