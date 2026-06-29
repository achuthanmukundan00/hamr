//! Startup UI components for the hamr CLI.
//!
//! Port of `packages/coding-agent/src/cli/startup-ui.ts`.
//!
//! TUI components (FirstTimeSetupComponent, ExtensionInputComponent,
//! ExtensionSelectorComponent) are not yet ported. The TUI functions use
//! stdin/stdout fallbacks. `should_run_first_time_setup` is fully implemented.

use std::io::{self, Write};
use std::path::Path;

use crate::config::{AGENT_DIR_NAME, HAMR_AGENT_DIR_ENV, HAMR_CONFIG_DIR};
use crate::core::experimental::are_experimental_features_enabled;
use crate::core::settings_manager::SettingsManager;

/// Official distribution identifiers.
/// Only distributions matching all three trigger first-time setup.
const OFFICIAL_PACKAGE_NAMES: &[&str] = &["@skaft/hamr", "@hamr/coding-agent"];
const OFFICIAL_APP_NAMES: &[&str] = &["hamr"];
const OFFICIAL_CONFIG_DIR_NAME: &str = ".hamr";

/// Metadata about the current distribution.
struct DistributionMetadata {
    package_name: String,
    app_name: String,
    config_dir_name: String,
}

/// Get the default settings path (global agent dir / settings.json).
fn get_default_settings_path() -> String {
    let agent_dir = if let Ok(dir) = std::env::var(HAMR_AGENT_DIR_ENV) {
        dir
    } else {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
        format!("{}/.config/hamr/{}", home, AGENT_DIR_NAME)
    };
    format!("{}/settings.json", agent_dir)
}

/// Get distribution metadata from environment / build constants.
fn get_distribution_metadata() -> DistributionMetadata {
    DistributionMetadata {
        package_name: std::env::var("HAMR_PACKAGE_NAME")
            .unwrap_or_else(|_| "@skaft/hamr".to_string()),
        app_name: std::env::var("HAMR_APP_NAME").unwrap_or_else(|_| "hamr".to_string()),
        config_dir_name: HAMR_CONFIG_DIR.to_string(),
    }
}

/// Check whether this is an official Hamr distribution.
fn is_official_distribution(meta: &DistributionMetadata) -> bool {
    OFFICIAL_PACKAGE_NAMES.contains(&meta.package_name.as_str())
        && OFFICIAL_APP_NAMES.contains(&meta.app_name.as_str())
        && meta.config_dir_name == OFFICIAL_CONFIG_DIR_NAME
}

/// Check whether first-time setup should run.
///
/// Conditions (all must be true):
/// - Official Hamr distribution (not a fork/rebrand)
/// - Experimental features are enabled (HAMR_EXPERIMENTAL=1 or PI_EXPERIMENTAL=1)
/// - Default agent directory is used (no HAMR_AGENT_DIR override)
/// - Settings file does not exist
pub fn should_run_first_time_setup(settings_path: Option<&str>) -> bool {
    let meta = get_distribution_metadata();
    if !is_official_distribution(&meta) {
        return false;
    }
    if !are_experimental_features_enabled() {
        return false;
    }
    if std::env::var(HAMR_AGENT_DIR_ENV).is_ok() {
        return false;
    }
    let path = settings_path.map(String::from).unwrap_or_else(get_default_settings_path);
    !Path::new(&path).exists()
}

/// A labeled choice for startup selectors.
#[derive(Debug, Clone)]
pub struct StartupOption<T> {
    pub label: String,
    pub value: T,
}

/// Show a startup selector using stdin/stdout (TUI fallback).
///
/// Prints options to stderr, reads selection from stdin.
/// Returns the selected value, or None if cancelled.
pub async fn show_startup_selector<T: Clone>(
    _settings_manager: &SettingsManager,
    title: &str,
    options: &[StartupOption<T>],
) -> Option<T> {
    if options.is_empty() {
        return None;
    }

    eprintln!("\n{}", title);
    for (i, opt) in options.iter().enumerate() {
        eprintln!("  [{}] {}", i + 1, opt.label);
    }
    eprint!("\nSelect (1-{}), or 0 to cancel: ", options.len());

    let _ = io::stderr().flush();

    let mut input = String::new();
    if io::stdin().read_line(&mut input).is_err() {
        return None;
    }
    let trimmed = input.trim();
    let selection: usize = match trimmed.parse() {
        Ok(n) => n,
        Err(_) => return None,
    };
    if selection == 0 || selection > options.len() {
        return None;
    }
    Some(options[selection - 1].value.clone())
}

/// Show the first-time setup dialog and persist the result.
///
/// TUI fallback: prompts for theme and analytics via stdin/stdout.
pub async fn show_first_time_setup(settings_manager: &mut SettingsManager) {
    eprintln!("\nWelcome to Hamr! Let's set up your environment.\n");

    // Theme selection
    eprintln!("Choose a theme:");
    eprintln!("  [1] hamr (default)");
    eprintln!("  [2] dark");
    eprintln!("  [3] light");
    eprint!("\nSelect (1-3) [1]: ");
    let _ = io::stderr().flush();

    let mut theme_input = String::new();
    let _ = io::stdin().read_line(&mut theme_input);
    let theme = match theme_input.trim() {
        "2" => "dark",
        "3" => "light",
        _ => "hamr",
    };
    settings_manager.set_theme(theme);

    // Analytics opt-in
    eprintln!("\nHelp improve Hamr by sharing anonymous usage data?");
    eprintln!("  This stores a tracking identifier in settings.json.");
    eprint!("  Opt in? (y/N): ");
    let _ = io::stderr().flush();

    let mut analytics_input = String::new();
    let _ = io::stdin().read_line(&mut analytics_input);
    let share_analytics = matches!(analytics_input.trim().to_lowercase().as_str(), "y" | "yes");
    settings_manager.set_enable_analytics(share_analytics);

    // set_theme and set_enable_analytics already persist via internal save()

    eprintln!("\nSetup complete!\n");
}

/// Show a startup text input prompt using stdin/stdout (TUI fallback).
///
/// Returns the entered text, or None if cancelled.
pub async fn show_startup_input(
    _settings_manager: &SettingsManager,
    title: &str,
    placeholder: Option<&str>,
) -> Option<String> {
    eprint!("\n{}", title);
    if let Some(ph) = placeholder {
        eprint!(" [{}]", ph);
    }
    eprint!(": ");
    let _ = io::stderr().flush();

    let mut input = String::new();
    if io::stdin().read_line(&mut input).is_err() {
        return None;
    }
    let trimmed = input.trim().to_string();
    if trimmed.is_empty() {
        placeholder.map(String::from)
    } else {
        Some(trimmed)
    }
}
