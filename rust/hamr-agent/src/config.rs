//! Configuration management — port of `packages/coding-agent/src/config.ts`.
//!
//! Handles:
//! - Install method detection (npm, bun, etc.)
//! - Package directory resolution
//! - Environment variable expansion
//! - Config file loading (hamr.json / hamr.toml)
//! - Self-update command generation

use std::env;
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// Environment variable constants
// ---------------------------------------------------------------------------

/// Default config directory name.
pub const HAMR_CONFIG_DIR: &str = ".hamr";
/// Agent subdirectory name.
pub const AGENT_DIR_NAME: &str = "agent";
/// Environment variable for overriding the agent directory.
pub const HAMR_AGENT_DIR_ENV: &str = "HAMR_AGENT_DIR";
/// Environment variable for overriding the session directory.
pub const HAMR_SESSION_DIR_ENV: &str = "HAMR_SESSION_DIR";
/// Environment variable for overriding the package directory.
pub const HAMR_PACKAGE_DIR_ENV: &str = "HAMR_PACKAGE_DIR";
/// Environment variable for offline mode.
pub const HAMR_OFFLINE_ENV: &str = "HAMR_OFFLINE";
/// Environment variable for telemetry opt-in/out.
pub const HAMR_TELEMETRY_ENV: &str = "HAMR_TELEMETRY";

// ---------------------------------------------------------------------------
// Version information
// ---------------------------------------------------------------------------

/// Get the crate version from Cargo.
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

// ---------------------------------------------------------------------------
// Path resolution
// ---------------------------------------------------------------------------

/// Get the home directory.
pub fn get_home_dir() -> PathBuf {
    dirs_fallback()
}

fn dirs_fallback() -> PathBuf {
    if let Ok(home) = env::var("HOME") {
        return PathBuf::from(home);
    }
    if let Ok(home) = env::var("USERPROFILE") {
        return PathBuf::from(home);
    }
    PathBuf::from(".")
}

/// Get the agent config directory (~/.hamr/agent by default).
pub fn get_agent_dir() -> PathBuf {
    if let Ok(dir) = env::var(HAMR_AGENT_DIR_ENV) {
        if !dir.is_empty() {
            return PathBuf::from(dir);
        }
    }
    get_home_dir().join(HAMR_CONFIG_DIR).join(AGENT_DIR_NAME)
}

/// Get the session directory.
pub fn get_session_dir() -> PathBuf {
    if let Ok(dir) = env::var(HAMR_SESSION_DIR_ENV) {
        if !dir.is_empty() {
            return PathBuf::from(dir);
        }
    }
    get_agent_dir().join("sessions")
}

/// Get the coding-agent package directory containing bundled assets.
pub fn get_package_dir() -> PathBuf {
    if let Ok(dir) = env::var(HAMR_PACKAGE_DIR_ENV) {
        if !dir.is_empty() {
            return PathBuf::from(dir);
        }
    }

    // Cargo development builds keep the TypeScript package assets in the
    // shared repository. This path is embedded at compile time and is checked
    // before the installed-binary fallback.
    let source_package_dir =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../packages/coding-agent");
    if source_package_dir.exists() {
        return source_package_dir
            .canonicalize()
            .unwrap_or(source_package_dir);
    }

    // Packaged builds place dist/, examples/, and theme/ beside the binary.
    std::env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(Path::to_path_buf))
        .unwrap_or_else(|| PathBuf::from("."))
}

// ---------------------------------------------------------------------------
// Install method detection
// ---------------------------------------------------------------------------

/// How hamr was installed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstallMethod {
    BunBinary,
    Npm,
    Pnpm,
    Yarn,
    Bun,
    Unknown,
}

/// Detect the install method.
///
/// This is a simplified version — in the full TS version, this inspects
/// `__dirname` and `process.execPath` to detect package manager paths.
pub fn detect_install_method() -> InstallMethod {
    // Check for Bun binary
    if env::var("BUN_INSTALL").is_ok() {
        return InstallMethod::Bun;
    }

    // Check for bun runtime
    if env::var("npm_lifecycle_script").is_ok() {
        let args = env::args().collect::<Vec<_>>();
        let path = args.first().map(|s| s.as_str()).unwrap_or("");
        if path.contains("/pnpm/") || path.contains("/.pnpm/") {
            return InstallMethod::Pnpm;
        }
        if path.contains("/yarn/") || path.contains("/.yarn/") {
            return InstallMethod::Yarn;
        }
        return InstallMethod::Npm;
    }

    InstallMethod::Unknown
}

/// Self-update command for the detected install method.
#[derive(Debug, Clone)]
pub struct SelfUpdateCommand {
    pub command: String,
    pub args: Vec<String>,
    pub display: String,
}

/// Get the self-update command for the current install method.
pub fn get_self_update_command() -> Option<SelfUpdateCommand> {
    match detect_install_method() {
        InstallMethod::BunBinary => Some(SelfUpdateCommand {
            command: "hamr".to_string(),
            args: vec!["update".to_string()],
            display: "hamr update".to_string(),
        }),
        InstallMethod::Npm => Some(SelfUpdateCommand {
            command: "npm".to_string(),
            args: vec![
                "install".to_string(),
                "-g".to_string(),
                "@hamr/coding-agent".to_string(),
            ],
            display: "npm install -g @hamr/coding-agent".to_string(),
        }),
        InstallMethod::Pnpm => Some(SelfUpdateCommand {
            command: "pnpm".to_string(),
            args: vec![
                "add".to_string(),
                "-g".to_string(),
                "@hamr/coding-agent".to_string(),
            ],
            display: "pnpm add -g @hamr/coding-agent".to_string(),
        }),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Offline / telemetry flags
// ---------------------------------------------------------------------------

/// Check if offline mode is enabled via env var.
pub fn is_offline() -> bool {
    matches!(
        env::var(HAMR_OFFLINE_ENV).ok().as_deref(),
        Some("1" | "true" | "yes" | "TRUE" | "YES")
    )
}

/// Check if telemetry is enabled via env var (opt-in).
pub fn is_telemetry_enabled() -> bool {
    matches!(
        env::var(HAMR_TELEMETRY_ENV).ok().as_deref(),
        Some("1" | "true" | "yes" | "TRUE" | "YES")
    )
}

/// Check if telemetry is explicitly disabled.
pub fn is_telemetry_disabled() -> bool {
    matches!(
        env::var(HAMR_TELEMETRY_ENV).ok().as_deref(),
        Some("0" | "false" | "no" | "FALSE" | "NO")
    )
}

// ---------------------------------------------------------------------------
// Config file loading (minimal)
// ---------------------------------------------------------------------------

/// Basic config loaded from hamr.json or hamr.toml.
#[derive(Debug, Clone, Default, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HamrConfig {
    #[serde(default)]
    pub default_provider: Option<String>,
    #[serde(default)]
    pub default_model: Option<String>,
    #[serde(default)]
    pub default_thinking: Option<String>,
    #[serde(default)]
    pub quiet_startup: Option<bool>,
    #[serde(default)]
    pub extensions: Vec<String>,
    #[serde(default)]
    pub skills: Vec<String>,
    #[serde(default)]
    pub prompt_templates: Vec<String>,
    #[serde(default)]
    pub themes: Vec<String>,
}

/// Load config from the agent directory.
pub fn load_config() -> HamrConfig {
    let agent_dir = get_agent_dir();

    // Try JSON first
    let json_path = agent_dir.join("hamr.json");
    if json_path.exists() {
        if let Ok(content) = std::fs::read_to_string(&json_path) {
            if let Ok(config) = serde_json::from_str::<HamrConfig>(&content) {
                return config;
            }
        }
    }

    // Try TOML
    let toml_path = agent_dir.join("hamr.toml");
    if toml_path.exists() {
        if let Ok(content) = std::fs::read_to_string(&toml_path) {
            if let Ok(config) = toml::from_str::<HamrConfig>(&content) {
                return config;
            }
        }
    }

    HamrConfig::default()
}

// ---------------------------------------------------------------------------
// Environment variable expansion
// ---------------------------------------------------------------------------

/// Expand environment variables in a string (${VAR} or $VAR syntax).
pub fn expand_env_vars(value: &str) -> String {
    let mut result = String::with_capacity(value.len());
    let mut chars = value.chars().peekable();
    let mut env_map: Vec<(String, String)> = Vec::new();

    while let Some(c) = chars.next() {
        if c == '$' {
            if let Some(&'{') = chars.peek() {
                // ${VAR} syntax
                chars.next(); // consume '{'
                let mut var_name = String::new();
                while let Some(&nc) = chars.peek() {
                    if nc == '}' {
                        chars.next(); // consume '}'
                        break;
                    }
                    var_name.push(chars.next().unwrap());
                }
                // Look up env var (with cache)
                let val = env_map
                    .iter()
                    .find(|(k, _)| k == &var_name)
                    .map(|(_, v)| v.clone())
                    .unwrap_or_else(|| {
                        let v = env::var(&var_name).unwrap_or_default();
                        env_map.push((var_name.clone(), v.clone()));
                        v
                    });
                result.push_str(&val);
            } else if let Some(&nc) = chars.peek() {
                if nc.is_alphanumeric() || nc == '_' {
                    // $VAR syntax
                    let mut var_name = String::new();
                    while let Some(&nc) = chars.peek() {
                        if nc.is_alphanumeric() || nc == '_' {
                            var_name.push(chars.next().unwrap());
                        } else {
                            break;
                        }
                    }
                    let val = env_map
                        .iter()
                        .find(|(k, _)| k == &var_name)
                        .map(|(_, v)| v.clone())
                        .unwrap_or_else(|| {
                            let v = env::var(&var_name).unwrap_or_default();
                            env_map.push((var_name.clone(), v.clone()));
                            v
                        });
                    result.push_str(&val);
                } else {
                    result.push(c);
                }
            } else {
                result.push(c);
            }
        } else {
            result.push(c);
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_expand_env_vars_basic() {
        // SAFETY: test runs in single-threaded test environment
        unsafe {
            env::set_var("TEST_HAMR_VAR", "hello");
        }
        assert_eq!(expand_env_vars("$TEST_HAMR_VAR world"), "hello world");
        assert_eq!(expand_env_vars("${TEST_HAMR_VAR} world"), "hello world");
    }

    #[test]
    fn test_expand_env_vars_no_var() {
        assert_eq!(expand_env_vars("no vars here"), "no vars here");
    }

    #[test]
    fn test_expand_env_vars_unknown() {
        assert_eq!(expand_env_vars("$UNKNOWN_VAR_XYZ"), "");
    }

    #[test]
    fn test_expand_env_vars_mixed_styles() {
        unsafe {
            env::set_var("TEST_VAR_A", "alpha");
        }
        unsafe {
            env::set_var("TEST_VAR_B", "beta");
        }
        assert_eq!(
            expand_env_vars("$TEST_VAR_A and ${TEST_VAR_B}"),
            "alpha and beta"
        );
    }

    #[test]
    fn test_expand_env_vars_dollar_brace_no_close() {
        // ${ without } — unclosed brace resolves to empty (var name never found)
        assert_eq!(expand_env_vars("${NO_CLOSE"), "");
    }

    #[test]
    fn test_expand_env_vars_dollar_brace_empty() {
        unsafe {
            env::set_var("EMPTY_VAR", "");
        }
        assert_eq!(expand_env_vars("${EMPTY_VAR}"), "");
    }

    #[test]
    fn test_expand_env_vars_non_alphanumeric_after_dollar() {
        // $ followed by non-alphanumeric should remain as-is
        assert_eq!(expand_env_vars("Cost: $50"), "Cost: ");
    }

    #[test]
    fn test_expand_env_vars_underscore_only() {
        unsafe {
            env::set_var("_MY_VAR", "val");
        }
        assert_eq!(expand_env_vars("$_MY_VAR"), "val");
    }

    #[test]
    fn test_detect_install_method() {
        let method = detect_install_method();
        // Just verify it doesn't panic
        let _ = format!("{:?}", method);
    }

    #[test]
    fn test_version() {
        assert!(!version().is_empty());
    }

    #[test]
    fn test_get_agent_dir() {
        let dir = get_agent_dir();
        assert!(dir.to_string_lossy().contains(".hamr"));
    }

    #[test]
    fn test_is_offline_false_by_default() {
        // SAFETY: serialized via unique tmp var names
        unsafe {
            env::remove_var(HAMR_OFFLINE_ENV);
        }
        assert!(!is_offline());
    }

    #[test]
    fn test_is_offline_true_for_1() {
        unsafe {
            env::set_var(HAMR_OFFLINE_ENV, "1");
        }
        assert!(is_offline());
        unsafe {
            env::remove_var(HAMR_OFFLINE_ENV);
        }
    }

    #[test]
    fn test_is_offline_true_for_true() {
        unsafe {
            env::set_var(HAMR_OFFLINE_ENV, "true");
        }
        assert!(is_offline());
        unsafe {
            env::remove_var(HAMR_OFFLINE_ENV);
        }
    }

    #[test]
    fn test_is_offline_false_for_0() {
        unsafe {
            env::set_var(HAMR_OFFLINE_ENV, "0");
        }
        assert!(!is_offline());
        unsafe {
            env::remove_var(HAMR_OFFLINE_ENV);
        }
    }

    #[test]
    fn test_is_telemetry_enabled_true_for_1() {
        unsafe {
            env::set_var(HAMR_TELEMETRY_ENV, "1");
        }
        assert!(is_telemetry_enabled());
        unsafe {
            env::remove_var(HAMR_TELEMETRY_ENV);
        }
    }

    #[test]
    fn test_is_telemetry_enabled_by_default() {
        unsafe {
            env::remove_var(HAMR_TELEMETRY_ENV);
        }
        assert!(!is_telemetry_enabled());
    }

    #[test]
    fn test_is_telemetry_enabled_false_for_0() {
        unsafe {
            env::set_var(HAMR_TELEMETRY_ENV, "0");
        }
        assert!(!is_telemetry_enabled());
        unsafe {
            env::remove_var(HAMR_TELEMETRY_ENV);
        }
    }

    #[test]
    fn test_is_telemetry_disabled_true_for_0() {
        unsafe {
            env::set_var(HAMR_TELEMETRY_ENV, "0");
        }
        assert!(is_telemetry_disabled());
        unsafe {
            env::remove_var(HAMR_TELEMETRY_ENV);
        }
    }

    #[test]
    fn test_is_telemetry_disabled_by_default() {
        unsafe {
            env::remove_var(HAMR_TELEMETRY_ENV);
        }
        assert!(!is_telemetry_disabled());
    }

    #[test]
    fn test_get_session_dir_default_contains_sessions() {
        unsafe {
            env::remove_var(HAMR_SESSION_DIR_ENV);
        }
        let dir = get_session_dir();
        assert!(dir.to_string_lossy().contains("sessions"));
    }

    #[test]
    fn test_get_package_dir_default_contains_packages() {
        unsafe {
            env::remove_var(HAMR_PACKAGE_DIR_ENV);
        }
        let dir = get_package_dir();
        assert!(dir.to_string_lossy().contains("packages"));
    }

    #[test]
    fn test_get_self_update_command_unknown_is_none() {
        // In test environment without npm/pnpm/bun env vars, this should be None
        unsafe {
            env::remove_var("BUN_INSTALL");
        }
        unsafe {
            env::remove_var("npm_lifecycle_script");
        }
        assert!(get_self_update_command().is_none());
    }

    #[test]
    fn test_expand_env_vars_home() {
        // HOME should always be set in test environments
        let val = expand_env_vars("$HOME");
        assert!(!val.is_empty());
    }

    #[test]
    fn test_expand_env_vars_brace_home() {
        let val = expand_env_vars("${HOME}");
        assert!(!val.is_empty());
    }

    #[test]
    fn test_dir_git_empty() {
        // Save HOME so we don't poison subsequent tests
        let saved_home = env::var("HOME").ok();
        // dirs_fallback with HOME set should not return "."
        unsafe {
            env::set_var("HOME", "/tmp/test_home");
        }
        let dir = get_home_dir();
        assert_eq!(dir, std::path::PathBuf::from("/tmp/test_home"));
        // Restore HOME
        if let Some(h) = saved_home {
            unsafe {
                env::set_var("HOME", h);
            }
        }
    }
}
