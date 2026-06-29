//! CLI commands for package management — port of `packages/coding-agent/src/package-manager-cli.ts`.
//!
//! Handles `hamr install`, `hamr remove`, `hamr uninstall`, `hamr update`, `hamr list` commands.
//! These commands manage extension and resource packages installed from git or npm.

use std::path::Path;

use crate::config::get_self_update_command;
use crate::core::package_manager::{DefaultPackageManager, InstallOptions, PackageManager, PackageManagerOptions};
use crate::core::settings_manager::{SettingsManager, SettingsManagerCreateOptions};

/// Result of a package management CLI command.
#[derive(Debug)]
pub enum PackageCliResult {
    /// Success.
    Success,
    /// Failure with exit code.
    Failure(i32),
    /// Command not applicable (show help).
    Help,
}

/// Handle a package management CLI command.
///
/// Dispatches to the appropriate handler based on the command.
/// Returns `None` if no package command was provided.
pub async fn handle_package_command(
    command: &str,
    args: &[String],
    cwd: &Path,
) -> Option<PackageCliResult> {
    match command {
        "install" => Some(handle_install(args, cwd).await),
        "remove" | "uninstall" => Some(handle_remove(args, cwd).await),
        "update" => Some(handle_update(args, cwd).await),
        "list" => Some(handle_list(args, cwd).await),
        "config" => Some(handle_config(args, cwd).await),
        _ => None,
    }
}

/// Create a PackageManager from the current environment.
fn create_package_manager(cwd: &Path) -> Result<(DefaultPackageManager, SettingsManager), String> {
    let agent_dir = crate::config::get_agent_dir();
    let agent_dir_str = agent_dir.to_string_lossy().to_string();
    let cwd_str = cwd.to_string_lossy().to_string();

    let settings_manager = SettingsManager::create(&cwd_str, &agent_dir_str, SettingsManagerCreateOptions::default());

    let options = PackageManagerOptions {
        cwd: cwd.to_path_buf(),
        agent_dir,
    };
    let pm = DefaultPackageManager::new(options, settings_manager.clone());
    Ok((pm, settings_manager))
}

async fn handle_install(args: &[String], cwd: &Path) -> PackageCliResult {
    let source = match args.first() {
        Some(s) if !s.starts_with('-') => s.clone(),
        _ => {
            eprintln!("Usage: hamr install <source> [-l]");
            eprintln!("  source    Git URL, npm package, or local path");
            eprintln!("  -l        Install locally (project-level instead of global)");
            return PackageCliResult::Failure(1);
        }
    };

    let local = args.iter().any(|a| a == "-l" || a == "--local");

    let (pm, _sm) = match create_package_manager(cwd) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("Error: {}", e);
            return PackageCliResult::Failure(1);
        }
    };

    let options = InstallOptions { local };

    match pm.install_and_persist(&source, Some(options)).await {
        Ok(()) => {
            eprintln!("Installed: {}", source);
            PackageCliResult::Success
        }
        Err(e) => {
            eprintln!("Failed to install {}: {}", source, e);
            PackageCliResult::Failure(1)
        }
    }
}

async fn handle_remove(args: &[String], cwd: &Path) -> PackageCliResult {
    let source = match args.first() {
        Some(s) if !s.starts_with('-') => s.clone(),
        _ => {
            eprintln!("Usage: hamr remove <source> [-l]");
            return PackageCliResult::Failure(1);
        }
    };

    let local = args.iter().any(|a| a == "-l" || a == "--local");

    let (pm, _sm) = match create_package_manager(cwd) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("Error: {}", e);
            return PackageCliResult::Failure(1);
        }
    };

    let options = InstallOptions { local };

    match pm.remove_and_persist(&source, Some(options)).await {
        Ok(removed) => {
            if removed {
                eprintln!("Removed: {}", source);
            } else {
                eprintln!("Package not found: {}", source);
            }
            PackageCliResult::Success
        }
        Err(e) => {
            eprintln!("Failed to remove {}: {}", source, e);
            PackageCliResult::Failure(1)
        }
    }
}

async fn handle_update(args: &[String], cwd: &Path) -> PackageCliResult {
    let target = args.first().map(|s| s.as_str()).unwrap_or("all");

    match target {
        "self" | "hamr" => {
            let cmd = get_self_update_command();
            match cmd {
                Some(c) => {
                    eprintln!("Run: {}", c.display);
                    PackageCliResult::Success
                }
                None => {
                    eprintln!("Self-update not supported for this install method");
                    PackageCliResult::Failure(1)
                }
            }
        }
        source => {
            let (pm, _sm) = match create_package_manager(cwd) {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("Error: {}", e);
                    return PackageCliResult::Failure(1);
                }
            };

            let target_str = if source == "all" { None } else { Some(source) };
            match pm.update(target_str).await {
                Ok(()) => {
                    eprintln!("Updated: {}", target);
                    PackageCliResult::Success
                }
                Err(e) => {
                    eprintln!("Failed to update {}: {}", target, e);
                    PackageCliResult::Failure(1)
                }
            }
        }
    }
}

async fn handle_list(_args: &[String], cwd: &Path) -> PackageCliResult {
    let (pm, _sm) = match create_package_manager(cwd) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("Error: {}", e);
            return PackageCliResult::Failure(1);
        }
    };

    let packages = pm.list_configured_packages();

    if packages.is_empty() {
        eprintln!("No packages installed.");
    } else {
        eprintln!("Installed packages ({}):\n", packages.len());
        for pkg in &packages {
            eprintln!("  - {}", pkg.source);
        }
    }

    PackageCliResult::Success
}

async fn handle_config(_args: &[String], cwd: &Path) -> PackageCliResult {
    let agent_dir = crate::config::get_agent_dir();
    eprintln!("\nHamr Package Configuration");
    eprintln!("=========================\n");
    eprintln!("Working directory: {}", cwd.display());
    eprintln!("Agent directory:   {}", agent_dir.display());
    eprintln!("\nEdit settings.json in the agent directory to configure packages.");

    PackageCliResult::Success
}
