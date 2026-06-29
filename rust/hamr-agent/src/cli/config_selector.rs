//! TUI config selector for `hamr config` command.
//!
//! Port of `packages/coding-agent/src/cli/config-selector.ts`.
//!
//! TUI components (ConfigSelectorComponent) are not yet ported.
//! This implementation provides a non-TUI fallback that displays
//! configuration information and exits cleanly.

use std::io::{self, Write};

use crate::core::package_manager::ResolvedPaths;
use crate::core::settings_manager::SettingsManager;

/// Options for showing the config selector TUI.
pub struct ConfigSelectorOptions {
    /// Resolved resource paths (extensions, skills, prompts, themes).
    pub resolved_paths: ResolvedPaths,
    pub settings_manager: SettingsManager,
    pub cwd: String,
    pub agent_dir: String,
}

/// Show the TUI config selector.
///
/// TUI fallback: displays configuration summary to stderr and returns.
pub async fn select_config(options: ConfigSelectorOptions) {
    eprintln!("\nHamr Configuration");
    eprintln!("==================\n");

    eprintln!("Working directory: {}", options.cwd);
    eprintln!("Agent directory:   {}", options.agent_dir);

    // Show resolved resources
    let rp = &options.resolved_paths;

    if !rp.extensions.is_empty() {
        eprintln!("\nExtensions ({}):", rp.extensions.len());
        for ext in &rp.extensions {
            eprintln!("  - {}", ext.path.display());
        }
    }

    if !rp.skills.is_empty() {
        eprintln!("\nSkills ({}):", rp.skills.len());
        for skill in &rp.skills {
            eprintln!("  - {}", skill.path.display());
        }
    }

    if !rp.prompts.is_empty() {
        eprintln!("\nPrompts ({}):", rp.prompts.len());
        for prompt in &rp.prompts {
            eprintln!("  - {}", prompt.path.display());
        }
    }

    if !rp.themes.is_empty() {
        eprintln!("\nThemes ({}):", rp.themes.len());
        for theme in &rp.themes {
            eprintln!("  - {}", theme.path.display());
        }
    }

    eprintln!("\n(Settings are managed via settings.json in the agent directory.)");
    let _ = io::stderr().flush();
}
