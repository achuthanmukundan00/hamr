//! Package manager — install, resolve, update, and remove extension packages.
//!
//! Port of `packages/coding-agent/src/core/package-manager.ts`.
//!
//! This is the central package lifecycle manager. It handles:
//! - npm-scoped packages (managed installs via the user's npm/pnpm/yarn/bun)
//! - git-sourced extensions (clone, fetch, reset)
//! - local paths
//! - Resource discovery (extensions, skills, prompts, themes) from packages
//! - Pattern filtering and override support
//! - Periodic update checking
#![allow(dead_code)]
#![allow(unused_assignments)]

use std::collections::{HashMap, HashSet};
use std::fs;
use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::Arc;

use std::os::unix::process::ExitStatusExt;
use tokio::process::Command as TokioCommand;
use tokio::time::Duration;

use crate::core::output_guard::is_stdout_taken_over;
use crate::core::settings_manager::{PackageSource, SettingsManager};
use crate::utils::paths;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const NETWORK_TIMEOUT_MS: u64 = 10_000;
const UPDATE_CHECK_CONCURRENCY: usize = 4;
const GIT_UPDATE_CONCURRENCY: usize = 4;
const CONFIG_DIR_NAME: &str = ".hamr";
const IGNORE_FILE_NAMES: &[&str] = &[".gitignore", ".ignore", ".fdignore"];

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// Source scope for a package.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceScope {
    User,
    Project,
    Temporary,
}

/// Describes where a resolved resource came from.
#[derive(Debug, Clone)]
pub struct PathMetadata {
    pub source: String,
    pub scope: SourceScope,
    pub origin: Origin,
    pub base_dir: Option<PathBuf>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Origin {
    Package,
    TopLevel,
}

/// A single resolved resource file.
#[derive(Debug, Clone)]
pub struct ResolvedResource {
    pub path: PathBuf,
    pub enabled: bool,
    pub metadata: PathMetadata,
}

/// All resolved resource types.
#[derive(Debug, Clone, Default)]
pub struct ResolvedPaths {
    pub extensions: Vec<ResolvedResource>,
    pub skills: Vec<ResolvedResource>,
    pub prompts: Vec<ResolvedResource>,
    pub themes: Vec<ResolvedResource>,
}

/// Action to take when a source is missing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MissingSourceAction {
    Install,
    Skip,
    Error,
}

/// Progress event emitted during long-running operations.
#[derive(Debug, Clone)]
pub struct ProgressEvent {
    pub event_type: ProgressEventType,
    pub action: ProgressAction,
    pub source: String,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProgressEventType {
    Start,
    Progress,
    Complete,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProgressAction {
    Install,
    Remove,
    Update,
    Clone,
    Pull,
}

/// Callback for progress events.
pub type ProgressCallback =
    Arc<dyn Fn(ProgressEvent) -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + Sync>;

/// Information about an available package update.
#[derive(Debug, Clone)]
pub struct PackageUpdate {
    pub source: String,
    pub display_name: String,
    pub update_type: PackageUpdateType,
    pub scope: ScopeInstalled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PackageUpdateType {
    Npm,
    Git,
}

/// Installed (non-temporary) scope for updates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScopeInstalled {
    User,
    Project,
}

/// Summary of a configured package for display.
#[derive(Debug, Clone)]
pub struct ConfiguredPackage {
    pub source: String,
    pub scope: ScopeInstalled,
    pub filtered: bool,
    pub installed_path: Option<PathBuf>,
}

/// Options for constructing the package manager.
#[derive(Debug, Clone)]
pub struct PackageManagerOptions {
    pub cwd: PathBuf,
    pub agent_dir: PathBuf,
}

/// Resource type enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ResourceType {
    Extensions,
    Skills,
    Prompts,
    Themes,
}

const RESOURCE_TYPES: &[ResourceType] = &[
    ResourceType::Extensions,
    ResourceType::Skills,
    ResourceType::Prompts,
    ResourceType::Themes,
];

// ---------------------------------------------------------------------------
// Parsed source
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
enum ParsedSource {
    Npm(NpmSource),
    Git(GitSource),
    Local(LocalSource),
}

#[derive(Debug, Clone)]
struct NpmSource {
    spec: String,
    name: String,
    version: Option<String>,
    range: Option<String>,
    pinned: bool,
}

#[derive(Debug, Clone)]
struct GitSource {
    repo: String,
    host: String,
    path: String,
    r#ref: Option<String>,
    pinned: bool,
}

#[derive(Debug, Clone)]
struct LocalSource {
    path: PathBuf,
}

// ---------------------------------------------------------------------------
// Package filter
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default)]
struct PackageFilter {
    extensions: Option<Vec<String>>,
    skills: Option<Vec<String>>,
    prompts: Option<Vec<String>>,
    themes: Option<Vec<String>>,
}

// ---------------------------------------------------------------------------
// Resource accumulator
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default)]
struct ResourceAccumulator {
    extensions: HashMap<PathBuf, AccumulatorEntry>,
    skills: HashMap<PathBuf, AccumulatorEntry>,
    prompts: HashMap<PathBuf, AccumulatorEntry>,
    themes: HashMap<PathBuf, AccumulatorEntry>,
}

#[derive(Debug, Clone)]
struct AccumulatorEntry {
    metadata: PathMetadata,
    enabled: bool,
}

// ---------------------------------------------------------------------------
// Pi manifest
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default, serde::Deserialize)]
struct PiManifest {
    extensions: Option<Vec<String>>,
    skills: Option<Vec<String>>,
    prompts: Option<Vec<String>>,
    themes: Option<Vec<String>>,
}

// ---------------------------------------------------------------------------
// ConfiguredUpdateSource + typed targets
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
struct ConfiguredUpdateSource {
    source: String,
    scope: ScopeInstalled,
}

#[derive(Debug, Clone)]
struct NpmUpdateTarget {
    source: String,
    scope: ScopeInstalled,
    parsed: NpmSource,
}

#[derive(Debug, Clone)]
struct GitUpdateTarget {
    source: String,
    scope: ScopeInstalled,
    parsed: GitSource,
}

// ---------------------------------------------------------------------------
// Traits
// ---------------------------------------------------------------------------

/// The main package manager interface.
pub trait PackageManager: Send + Sync {
    fn resolve(
        &self,
        on_missing: Option<OnMissingFn>,
    ) -> Pin<Box<dyn Future<Output = ResolvedPaths> + Send + '_>>;

    fn install(
        &self,
        source: &str,
        options: Option<InstallOptions>,
    ) -> Pin<Box<dyn Future<Output = Result<(), String>> + Send + '_>>;

    fn install_and_persist(
        &self,
        source: &str,
        options: Option<InstallOptions>,
    ) -> Pin<Box<dyn Future<Output = Result<(), String>> + Send + '_>>;

    fn remove(
        &self,
        source: &str,
        options: Option<InstallOptions>,
    ) -> Pin<Box<dyn Future<Output = Result<(), String>> + Send + '_>>;

    fn remove_and_persist(
        &self,
        source: &str,
        options: Option<InstallOptions>,
    ) -> Pin<Box<dyn Future<Output = Result<bool, String>> + Send + '_>>;

    fn update(
        &self,
        source: Option<&str>,
    ) -> Pin<Box<dyn Future<Output = Result<(), String>> + Send + '_>>;

    fn list_configured_packages(&self) -> Vec<ConfiguredPackage>;

    fn resolve_extension_sources(
        &self,
        sources: &[String],
        options: Option<ResolveExtensionOptions>,
    ) -> Pin<Box<dyn Future<Output = ResolvedPaths> + Send + '_>>;

    fn add_source_to_settings(&self, source: &str, options: Option<InstallOptions>) -> bool;

    fn remove_source_from_settings(&self, source: &str, options: Option<InstallOptions>) -> bool;

    fn set_progress_callback(&self, callback: Option<ProgressCallback>);

    fn get_installed_path(&self, source: &str, scope: ScopeInstalled) -> Option<PathBuf>;

    fn check_for_available_updates(
        &self,
    ) -> Pin<Box<dyn Future<Output = Vec<PackageUpdate>> + Send + '_>>;
}

/// Callback used when a package source is missing.
pub type OnMissingFn =
    Arc<dyn Fn(String) -> Pin<Box<dyn Future<Output = MissingSourceAction> + Send>> + Send + Sync>;

/// Options for install/remove.
#[derive(Debug, Clone, Copy, Default)]
pub struct InstallOptions {
    pub local: bool,
}

/// Options for resolve_extension_sources.
#[derive(Debug, Clone, Copy, Default)]
pub struct ResolveExtensionOptions {
    pub local: bool,
    pub temporary: bool,
}

// ---------------------------------------------------------------------------
// Helpers — ported from TS free functions
// ---------------------------------------------------------------------------

fn is_offline_mode_enabled() -> bool {
    let value = std::env::var("HAMR_OFFLINE")
        .or_else(|_| std::env::var("PI_OFFLINE"))
        .unwrap_or_default();
    if value.is_empty() {
        return false;
    }
    value == "1" || value.eq_ignore_ascii_case("true") || value.eq_ignore_ascii_case("yes")
}

fn to_posix_path(p: &str) -> String {
    p.replace('\\', "/")
}

fn get_home_dir() -> String {
    std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .unwrap_or_else(|_| {
            #[cfg(target_os = "windows")]
            {
                std::env::var("HOMEDRIVE").unwrap_or_default()
                    + &std::env::var("HOMEPATH").unwrap_or_default()
            }
            #[cfg(not(target_os = "windows"))]
            {
                std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string())
            }
        })
}

fn get_extension_temp_folder(agent_dir: &str) -> PathBuf {
    let temp_folder = PathBuf::from(agent_dir).join("tmp").join("extensions");
    let _ = fs::create_dir_all(&temp_folder);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(metadata) = fs::metadata(&temp_folder) {
            let mut perms = metadata.permissions();
            perms.set_mode(0o700);
            let _ = fs::set_permissions(&temp_folder, perms);
        }
    }
    temp_folder
}

fn resource_precedence_rank(m: &PathMetadata) -> u32 {
    if matches!(m.origin, Origin::Package) {
        return 4;
    }
    let scope_base = match m.scope {
        SourceScope::Project => 0,
        _ => 2,
    };
    scope_base + if m.source == "local" { 0 } else { 1 }
}

fn prefix_ignore_pattern(line: &str, prefix: &str) -> Option<String> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return None;
    }
    if trimmed.starts_with('#') && !trimmed.starts_with("\\#") {
        return None;
    }

    let mut pattern = line.to_string();
    let mut negated = false;

    if pattern.starts_with('!') {
        negated = true;
        pattern = pattern[1..].to_string();
    } else if pattern.starts_with("\\!") {
        pattern = pattern[1..].to_string();
    }

    if pattern.starts_with('/') {
        pattern = pattern[1..].to_string();
    }

    let prefixed = if prefix.is_empty() {
        pattern
    } else {
        format!("{}{}", prefix, pattern)
    };
    if negated {
        Some(format!("!{}", prefixed))
    } else {
        Some(prefixed)
    }
}

fn add_ignore_rules(ig: &mut ignore::gitignore::GitignoreBuilder, dir: &Path, root_dir: &Path) {
    let relative_dir = pathdiff::diff_paths(dir, root_dir).unwrap_or_else(|| PathBuf::from("."));
    let relative_dir_str = to_posix_path(&relative_dir.to_string_lossy());
    let prefix = if relative_dir_str.is_empty() || relative_dir_str == "." {
        String::new()
    } else {
        format!("{}/", relative_dir_str)
    };

    for filename in IGNORE_FILE_NAMES {
        let ignore_path = dir.join(filename);
        if !ignore_path.exists() {
            continue;
        }
        if let Ok(content) = fs::read_to_string(&ignore_path) {
            for line in content.split('\n') {
                if let Some(pattern) = prefix_ignore_pattern(line, &prefix) {
                    let _ = ig.add_line(Some(dir.to_path_buf()), &pattern);
                }
            }
        }
    }
}

fn is_pattern(s: &str) -> bool {
    s.starts_with('!')
        || s.starts_with('+')
        || s.starts_with('-')
        || s.contains('*')
        || s.contains('?')
}

fn is_override_pattern(s: &str) -> bool {
    s.starts_with('!') || s.starts_with('+') || s.starts_with('-')
}

fn has_glob_pattern(s: &str) -> bool {
    s.contains('*') || s.contains('?')
}

fn split_patterns(entries: &[String]) -> (Vec<String>, Vec<String>) {
    let mut plain = Vec::new();
    let mut patterns = Vec::new();
    for entry in entries {
        if is_pattern(entry) {
            patterns.push(entry.clone());
        } else {
            plain.push(entry.clone());
        }
    }
    (plain, patterns)
}

fn collect_files(
    dir: &Path,
    file_pattern: &str,
    skip_node_modules: bool,
    ignore_matcher: Option<&mut ignore::gitignore::GitignoreBuilder>,
    root_dir: Option<&Path>,
) -> Vec<PathBuf> {
    let mut files = Vec::new();
    if !dir.exists() {
        return files;
    }

    let root = root_dir.unwrap_or(dir);
    let mut ig = if let Some(im) = ignore_matcher {
        im.clone()
    } else {
        let mut b = ignore::gitignore::GitignoreBuilder::new(root);
        add_ignore_rules(&mut b, dir, root);
        b
    };

    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            if name_str.starts_with('.') {
                continue;
            }
            if skip_node_modules && name_str == "node_modules" {
                continue;
            }

            let full_path = entry.path();
            let is_dir = entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false);
            let is_file = entry.file_type().map(|ft| ft.is_file()).unwrap_or(false);

            // Handle symlinks
            let (effective_is_dir, effective_is_file) =
                if entry.file_type().map(|ft| ft.is_symlink()).unwrap_or(false) {
                    match fs::metadata(&full_path) {
                        Ok(meta) => (meta.is_dir(), meta.is_file()),
                        Err(_) => continue,
                    }
                } else {
                    (is_dir, is_file)
                };

            let rel_path = to_posix_path(
                &pathdiff::diff_paths(&full_path, root)
                    .unwrap_or_else(|| PathBuf::from(name_str.as_ref()))
                    .to_string_lossy()
                    .as_ref(),
            );
            let ignore_path = if effective_is_dir {
                format!("{}/", rel_path)
            } else {
                rel_path.clone()
            };

            // Build the gitignore matcher and check
            let matcher = ig.build().ok();
            let is_ignored = matcher
                .as_ref()
                .map(|m| m.matched(&ignore_path, effective_is_dir).is_ignore())
                .unwrap_or(false);
            if is_ignored {
                continue;
            }

            if effective_is_dir {
                files.extend(collect_files(
                    &full_path,
                    file_pattern,
                    skip_node_modules,
                    Some(&mut ig),
                    Some(root),
                ));
            } else if effective_is_file {
                // Check file extension pattern
                let rx = regex::Regex::new(file_pattern).ok();
                let matches = rx.map(|r| r.is_match(&name_str)).unwrap_or(false);
                if matches {
                    files.push(full_path);
                }
            }
        }
    }

    files
}

fn collect_skill_entries(
    dir: &Path,
    mode: &str,
    ignore_matcher: Option<&mut ignore::gitignore::GitignoreBuilder>,
    root_dir: Option<&Path>,
) -> Vec<PathBuf> {
    let mut entries = Vec::new();
    if !dir.exists() {
        return entries;
    }

    let root = root_dir.unwrap_or(dir);
    let mut ig = if let Some(im) = ignore_matcher {
        im.clone()
    } else {
        let mut b = ignore::gitignore::GitignoreBuilder::new(root);
        add_ignore_rules(&mut b, dir, root);
        b
    };

    if let Ok(dir_entries) = fs::read_dir(dir) {
        // Pass 1: look for SKILL.md at this level
        let skills: Vec<_> = dir_entries.filter_map(|e| e.ok()).collect();
        for entry in &skills {
            let name = entry.file_name();
            if name != "SKILL.md" {
                continue;
            }
            let full_path = entry.path();
            let is_file = entry.file_type().map(|ft| ft.is_file()).unwrap_or(false);
            let is_symlink = entry.file_type().map(|ft| ft.is_symlink()).unwrap_or(false);

            let effective_is_file = if is_symlink {
                fs::metadata(&full_path)
                    .map(|m| m.is_file())
                    .unwrap_or(false)
            } else {
                is_file
            };

            if !effective_is_file {
                continue;
            }

            let rel_path = to_posix_path(
                &pathdiff::diff_paths(&full_path, root)
                    .unwrap_or_else(|| PathBuf::from(&name.to_string_lossy().to_string()))
                    .to_string_lossy()
                    .as_ref(),
            );

            let matcher = ig.build().ok();
            let is_ignored = matcher
                .as_ref()
                .map(|m| m.matched(&rel_path, false).is_ignore())
                .unwrap_or(false);
            if !is_ignored {
                entries.push(full_path);
                return entries;
            }
        }

        // Pass 2: recurse into subdirectories
        for entry in &skills {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            if name_str.starts_with('.') || name_str == "node_modules" {
                continue;
            }

            let full_path = entry.path();
            let is_dir = entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false);
            let is_symlink = entry.file_type().map(|ft| ft.is_symlink()).unwrap_or(false);

            let effective_is_dir = if is_symlink {
                fs::metadata(&full_path)
                    .map(|m| m.is_dir())
                    .unwrap_or(false)
            } else {
                is_dir
            };

            if !effective_is_dir {
                continue;
            }

            // "hamr" mode: at root level, also collect .md files
            let rel_path = to_posix_path(
                &pathdiff::diff_paths(&full_path, root)
                    .unwrap_or_else(|| PathBuf::from(name_str.as_ref()))
                    .to_string_lossy()
                    .as_ref(),
            );

            let is_root_level = dir == root;
            if mode == "hamr" && is_root_level {
                if entry.file_type().map(|ft| ft.is_file()).unwrap_or(false)
                    && name_str.ends_with(".md")
                {
                    let matcher = ig.build().ok();
                    let is_ignored = matcher
                        .as_ref()
                        .map(|m| m.matched(&rel_path, false).is_ignore())
                        .unwrap_or(false);
                    if !is_ignored {
                        entries.push(full_path);
                        continue;
                    }
                }
            }

            let ignore_path = format!("{}/", rel_path);
            let matcher = ig.build().ok();
            let is_ignored = matcher
                .as_ref()
                .map(|m| m.matched(&ignore_path, true).is_ignore())
                .unwrap_or(false);
            if is_ignored {
                continue;
            }

            entries.extend(collect_skill_entries(
                &full_path,
                mode,
                Some(&mut ig),
                Some(root),
            ));
        }
    }

    entries
}

fn collect_auto_skill_entries(dir: &Path, mode: &str) -> Vec<PathBuf> {
    collect_skill_entries(dir, mode, None, None)
}

fn find_git_repo_root(start_dir: &Path) -> Option<PathBuf> {
    let mut dir = std::fs::canonicalize(start_dir).unwrap_or_else(|_| start_dir.to_path_buf());
    loop {
        if dir.join(".git").exists() {
            return Some(dir);
        }
        if !dir.pop() {
            return None;
        }
    }
}

fn collect_ancestor_agents_skill_dirs(start_dir: &Path) -> Vec<PathBuf> {
    let mut skill_dirs = Vec::new();
    let resolved_start =
        std::fs::canonicalize(start_dir).unwrap_or_else(|_| start_dir.to_path_buf());
    let git_repo_root = find_git_repo_root(&resolved_start);

    let mut dir = resolved_start.clone();
    loop {
        skill_dirs.push(dir.join(".agents").join("skills"));
        if let Some(ref repo_root) = git_repo_root {
            if dir == *repo_root {
                break;
            }
        }
        if !dir.pop() {
            break;
        }
    }

    skill_dirs
}

fn collect_auto_prompt_entries(dir: &Path) -> Vec<PathBuf> {
    let mut entries = Vec::new();
    if !dir.exists() {
        return entries;
    }

    let mut ig = ignore::gitignore::GitignoreBuilder::new(dir);
    add_ignore_rules(&mut ig, dir, dir);

    if let Ok(dir_entries) = fs::read_dir(dir) {
        for entry in dir_entries.flatten() {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            if name_str.starts_with('.') || name_str == "node_modules" {
                continue;
            }

            let full_path = entry.path();
            let is_file = entry.file_type().map(|ft| ft.is_file()).unwrap_or(false);
            let is_symlink = entry.file_type().map(|ft| ft.is_symlink()).unwrap_or(false);

            let effective_is_file = if is_symlink {
                fs::metadata(&full_path)
                    .map(|m| m.is_file())
                    .unwrap_or(false)
            } else {
                is_file
            };

            if !effective_is_file || !name_str.ends_with(".md") {
                continue;
            }

            let rel_path = to_posix_path(
                &pathdiff::diff_paths(&full_path, dir)
                    .unwrap_or_else(|| PathBuf::from(name_str.as_ref()))
                    .to_string_lossy()
                    .as_ref(),
            );

            let matcher = ig.build().ok();
            let is_ignored = matcher
                .as_ref()
                .map(|m| m.matched(&rel_path, false).is_ignore())
                .unwrap_or(false);
            if !is_ignored {
                entries.push(full_path);
            }
        }
    }

    entries
}

fn collect_auto_theme_entries(dir: &Path) -> Vec<PathBuf> {
    let mut entries = Vec::new();
    if !dir.exists() {
        return entries;
    }

    let mut ig = ignore::gitignore::GitignoreBuilder::new(dir);
    add_ignore_rules(&mut ig, dir, dir);

    if let Ok(dir_entries) = fs::read_dir(dir) {
        for entry in dir_entries.flatten() {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            if name_str.starts_with('.') || name_str == "node_modules" {
                continue;
            }

            let full_path = entry.path();
            let is_file = entry.file_type().map(|ft| ft.is_file()).unwrap_or(false);
            let is_symlink = entry.file_type().map(|ft| ft.is_symlink()).unwrap_or(false);

            let effective_is_file = if is_symlink {
                fs::metadata(&full_path)
                    .map(|m| m.is_file())
                    .unwrap_or(false)
            } else {
                is_file
            };

            if !effective_is_file || !name_str.ends_with(".json") {
                continue;
            }

            let rel_path = to_posix_path(
                &pathdiff::diff_paths(&full_path, dir)
                    .unwrap_or_else(|| PathBuf::from(name_str.as_ref()))
                    .to_string_lossy()
                    .as_ref(),
            );

            let matcher = ig.build().ok();
            let is_ignored = matcher
                .as_ref()
                .map(|m| m.matched(&rel_path, false).is_ignore())
                .unwrap_or(false);
            if !is_ignored {
                entries.push(full_path);
            }
        }
    }

    entries
}

fn read_pi_manifest_file(package_json_path: &Path) -> Option<PiManifest> {
    let content = fs::read_to_string(package_json_path).ok()?;
    let pkg: serde_json::Value = serde_json::from_str(&content).ok()?;
    let pi = pkg.get("pi")?;
    serde_json::from_value(pi.clone()).ok()
}

fn resolve_extension_entries(dir: &Path) -> Option<Vec<PathBuf>> {
    let package_json_path = dir.join("package.json");
    if package_json_path.exists() {
        if let Some(manifest) = read_pi_manifest_file(&package_json_path) {
            if let Some(ref ext_paths) = manifest.extensions {
                if !ext_paths.is_empty() {
                    let entries: Vec<PathBuf> = ext_paths
                        .iter()
                        .map(|p| {
                            let resolved = dir.join(p);
                            // Resolve tilde paths: ~extensions/foo -> ./~extensions/foo
                            if resolved.exists() {
                                resolved
                            } else {
                                dir.join(p)
                            }
                        })
                        .filter(|p| p.exists())
                        .collect();
                    if !entries.is_empty() {
                        return Some(entries);
                    }
                }
            }
        }
    }

    let index_ts = dir.join("index.ts");
    let index_js = dir.join("index.js");
    if index_ts.exists() {
        return Some(vec![index_ts]);
    }
    if index_js.exists() {
        return Some(vec![index_js]);
    }

    None
}

fn collect_auto_extension_entries(dir: &Path) -> Vec<PathBuf> {
    let mut entries = Vec::new();
    if !dir.exists() {
        return entries;
    }

    // First check if this directory itself has explicit extension entries
    if let Some(root_entries) = resolve_extension_entries(dir) {
        return root_entries;
    }

    // Otherwise discover extensions from directory contents
    let mut ig = ignore::gitignore::GitignoreBuilder::new(dir);
    add_ignore_rules(&mut ig, dir, dir);

    if let Ok(dir_entries) = fs::read_dir(dir) {
        for entry in dir_entries.flatten() {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            if name_str.starts_with('.') || name_str == "node_modules" {
                continue;
            }

            let full_path = entry.path();
            let is_dir = entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false);
            let is_file = entry.file_type().map(|ft| ft.is_file()).unwrap_or(false);
            let is_symlink = entry.file_type().map(|ft| ft.is_symlink()).unwrap_or(false);

            let (effective_is_dir, effective_is_file) = if is_symlink {
                match fs::metadata(&full_path) {
                    Ok(meta) => (meta.is_dir(), meta.is_file()),
                    Err(_) => continue,
                }
            } else {
                (is_dir, is_file)
            };

            let rel_path = to_posix_path(
                &pathdiff::diff_paths(&full_path, dir)
                    .unwrap_or_else(|| PathBuf::from(name_str.as_ref()))
                    .to_string_lossy()
                    .as_ref(),
            );
            let ignore_path = if effective_is_dir {
                format!("{}/", rel_path)
            } else {
                rel_path.clone()
            };

            let matcher = ig.build().ok();
            let is_ignored = matcher
                .as_ref()
                .map(|m| m.matched(&ignore_path, effective_is_dir).is_ignore())
                .unwrap_or(false);
            if is_ignored {
                continue;
            }

            if effective_is_file && (name_str.ends_with(".ts") || name_str.ends_with(".js")) {
                entries.push(full_path);
            } else if effective_is_dir {
                if let Some(resolved_entries) = resolve_extension_entries(&full_path) {
                    entries.extend(resolved_entries);
                }
            }
        }
    }

    entries
}

fn collect_resource_files(dir: &Path, resource_type: ResourceType) -> Vec<PathBuf> {
    match resource_type {
        ResourceType::Skills => collect_skill_entries(dir, "hamr", None, None),
        ResourceType::Extensions => collect_auto_extension_entries(dir),
        ResourceType::Prompts => collect_files(dir, r"\.md$", true, None, None),
        ResourceType::Themes => collect_files(dir, r"\.json$", true, None, None),
    }
}

fn matches_any_pattern(file_path: &Path, patterns: &[String], base_dir: &Path) -> bool {
    let rel = to_posix_path(
        &pathdiff::diff_paths(file_path, base_dir)
            .unwrap_or_else(|| PathBuf::from(""))
            .to_string_lossy(),
    );
    let name = file_path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    let file_path_posix = to_posix_path(&file_path.to_string_lossy());
    let is_skill_file = name == "SKILL.md";
    let parent_dir = if is_skill_file {
        file_path.parent()
    } else {
        None
    };
    let parent_rel = parent_dir.and_then(|p| pathdiff::diff_paths(p, base_dir));
    let parent_rel_str = parent_rel
        .as_ref()
        .map(|p| to_posix_path(&p.to_string_lossy()));
    let parent_name =
        parent_dir.and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string()));
    let parent_dir_posix = parent_dir.map(|p| to_posix_path(&p.to_string_lossy()));

    patterns.iter().any(|pattern| {
        let normalized = to_posix_path(pattern);
        if glob_match(&normalized, &rel)
            || glob_match(&normalized, &name)
            || glob_match(&normalized, &file_path_posix)
        {
            return true;
        }
        if !is_skill_file {
            return false;
        }
        (parent_rel_str
            .as_ref()
            .map(|p| glob_match(&normalized, p))
            .unwrap_or(false))
            || (parent_name
                .as_ref()
                .map(|p| glob_match(&normalized, p))
                .unwrap_or(false))
            || (parent_dir_posix
                .as_ref()
                .map(|p| glob_match(&normalized, p))
                .unwrap_or(false))
    })
}

/// Simple glob matching — uses glob::Pattern (similar to minimatch semantics).
fn glob_match(pattern: &str, path: &str) -> bool {
    if let Ok(p) = glob::Pattern::new(pattern) {
        p.matches(path)
    } else {
        false
    }
}

fn normalize_exact_pattern(pattern: &str) -> String {
    let normalized = if pattern.starts_with("./") || pattern.starts_with(".\\") {
        &pattern[2..]
    } else {
        pattern
    };
    to_posix_path(normalized)
}

fn matches_any_exact_pattern(file_path: &Path, patterns: &[String], base_dir: &Path) -> bool {
    if patterns.is_empty() {
        return false;
    }
    let rel = to_posix_path(
        &pathdiff::diff_paths(file_path, base_dir)
            .unwrap_or_else(|| PathBuf::from(""))
            .to_string_lossy(),
    );
    let name = file_path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    let file_path_posix = to_posix_path(&file_path.to_string_lossy());
    let is_skill_file = name == "SKILL.md";
    let parent_dir = if is_skill_file {
        file_path.parent()
    } else {
        None
    };
    let parent_rel = parent_dir.and_then(|p| pathdiff::diff_paths(p, base_dir));
    let parent_rel_str = parent_rel
        .as_ref()
        .map(|p| to_posix_path(&p.to_string_lossy()));
    let parent_dir_posix = parent_dir.map(|p| to_posix_path(&p.to_string_lossy()));

    patterns.iter().any(|pattern| {
        let normalized = normalize_exact_pattern(pattern);
        if normalized == rel || normalized == file_path_posix {
            return true;
        }
        if !is_skill_file {
            return false;
        }
        (parent_rel_str
            .as_ref()
            .map(|p| normalized == *p)
            .unwrap_or(false))
            || (parent_dir_posix
                .as_ref()
                .map(|p| normalized == *p)
                .unwrap_or(false))
    })
}

fn get_override_patterns(entries: &[String]) -> Vec<String> {
    entries
        .iter()
        .filter(|p| p.starts_with('!') || p.starts_with('+') || p.starts_with('-'))
        .cloned()
        .collect()
}

fn is_enabled_by_overrides(file_path: &Path, patterns: &[String], base_dir: &Path) -> bool {
    let overrides = get_override_patterns(patterns);
    let excludes: Vec<String> = overrides
        .iter()
        .filter(|p| p.starts_with('!'))
        .map(|p| p[1..].to_string())
        .collect();
    let force_includes: Vec<String> = overrides
        .iter()
        .filter(|p| p.starts_with('+'))
        .map(|p| p[1..].to_string())
        .collect();
    let force_excludes: Vec<String> = overrides
        .iter()
        .filter(|p| p.starts_with('-'))
        .map(|p| p[1..].to_string())
        .collect();

    let mut enabled = true;
    if !excludes.is_empty() && matches_any_pattern(file_path, &excludes, base_dir) {
        enabled = false;
    }
    if !force_includes.is_empty() && matches_any_exact_pattern(file_path, &force_includes, base_dir)
    {
        enabled = true;
    }
    if !force_excludes.is_empty() && matches_any_exact_pattern(file_path, &force_excludes, base_dir)
    {
        enabled = false;
    }
    enabled
}

fn apply_patterns(all_paths: &[PathBuf], patterns: &[String], base_dir: &Path) -> HashSet<PathBuf> {
    let includes: Vec<String> = patterns
        .iter()
        .filter(|p| !p.starts_with('+') && !p.starts_with('-') && !p.starts_with('!'))
        .cloned()
        .collect();
    let excludes: Vec<String> = patterns
        .iter()
        .filter(|p| p.starts_with('!'))
        .map(|p| p[1..].to_string())
        .collect();
    let force_includes: Vec<String> = patterns
        .iter()
        .filter(|p| p.starts_with('+'))
        .map(|p| p[1..].to_string())
        .collect();
    let force_excludes: Vec<String> = patterns
        .iter()
        .filter(|p| p.starts_with('-'))
        .map(|p| p[1..].to_string())
        .collect();

    // Step 1: Apply includes (or all if no includes)
    let mut result: Vec<PathBuf> = if includes.is_empty() {
        all_paths.to_vec()
    } else {
        all_paths
            .iter()
            .filter(|fp| matches_any_pattern(fp, &includes, base_dir))
            .cloned()
            .collect()
    };

    // Step 2: Apply excludes
    if !excludes.is_empty() {
        result.retain(|fp| !matches_any_pattern(fp, &excludes, base_dir));
    }

    // Step 3: Force-include (add back from all_paths, overriding exclusions)
    if !force_includes.is_empty() {
        for fp in all_paths {
            if !result.contains(fp) && matches_any_exact_pattern(fp, &force_includes, base_dir) {
                result.push(fp.clone());
            }
        }
    }

    // Step 4: Force-exclude
    if !force_excludes.is_empty() {
        result.retain(|fp| !matches_any_exact_pattern(fp, &force_excludes, base_dir));
    }

    result.into_iter().collect()
}

/// Get the env vars for a spawned process — mirrors TS `getEnv()`.
fn get_env() -> HashMap<String, String> {
    let env = std::env::vars().collect::<HashMap<_, _>>();
    if cfg!(not(target_os = "linux")) || !env.is_empty() {
        return env;
    }
    // Linux with empty env: try /proc/self/environ
    let data = fs::read_to_string("/proc/self/environ").unwrap_or_default();
    let mut result = HashMap::new();
    for entry in data.split('\0') {
        if let Some(idx) = entry.find('=') {
            result.insert(entry[..idx].to_string(), entry[idx + 1..].to_string());
        }
    }
    result
}

// ---------------------------------------------------------------------------
// DefaultPackageManager
// ---------------------------------------------------------------------------

#[allow(dead_code)]
pub struct DefaultPackageManager {
    cwd: PathBuf,
    agent_dir: PathBuf,
    settings_manager: SettingsManager,
    global_npm_root: tokio::sync::Mutex<Option<(String, String)>>,
    progress_callback: Option<ProgressCallback>,
}

impl DefaultPackageManager {
    pub fn new(options: PackageManagerOptions, settings_manager: SettingsManager) -> Self {
        Self {
            cwd: options.cwd,
            agent_dir: options.agent_dir,
            settings_manager,
            global_npm_root: tokio::sync::Mutex::new(None),
            progress_callback: None,
        }
    }

    fn ensure_settings_manager(&self) -> &SettingsManager {
        &self.settings_manager
    }

    // -----------------------------------------------------------------------
    // Parsing helpers
    // -----------------------------------------------------------------------

    fn parse_source(&self, source: &str) -> ParsedSource {
        if let Some(spec) = source.strip_prefix("npm:") {
            let (name, version) = Self::parse_npm_spec(spec.trim());
            let range = version
                .as_ref()
                .and_then(|v| Self::get_npm_version_range(v));
            let pinned = version
                .as_ref()
                .map_or(false, |v| Self::is_exact_npm_version(v));
            return ParsedSource::Npm(NpmSource {
                spec: spec.trim().to_string(),
                name,
                version: version.clone(),
                range,
                pinned,
            });
        }

        if Self::is_local_path(source) {
            return ParsedSource::Local(LocalSource {
                path: PathBuf::from(source),
            });
        }

        // Try parsing as git URL
        if let Some(git) = Self::parse_git_url(source) {
            return ParsedSource::Git(git);
        }

        ParsedSource::Local(LocalSource {
            path: PathBuf::from(source),
        })
    }

    fn parse_npm_spec(spec: &str) -> (String, Option<String>) {
        // Mirror: /^(@?[^@]+(?:\/[^@]+)?)(?:@(.+))?$/
        if let Some(at_idx) = spec.rfind('@') {
            if at_idx > 0 {
                let name = spec[..at_idx].to_string();
                let version = spec[at_idx + 1..].to_string();
                if !version.is_empty() {
                    return (name, Some(version));
                }
            }
        }
        (spec.to_string(), None)
    }

    fn is_exact_npm_version(version: &str) -> bool {
        semver::Version::parse(version).is_ok()
    }

    fn get_npm_version_range(version: &str) -> Option<String> {
        semver::VersionReq::parse(version)
            .ok()
            .map(|_| version.to_string())
    }

    fn is_local_path(s: &str) -> bool {
        s.starts_with('.') || s.starts_with('/') || s.starts_with('~')
    }

    fn parse_git_url(s: &str) -> Option<GitSource> {
        // Port of packages/coding-agent/src/utils/git.ts parseGitUrl
        let trimmed = s.trim();
        let has_git_prefix = trimmed.starts_with("git:");
        let url = if has_git_prefix {
            trimmed[4..].trim()
        } else {
            trimmed
        };

        // Without git: prefix, only accept explicit protocol URLs
        if !has_git_prefix {
            let re = regex::Regex::new(r"^(https?|ssh|git)://").unwrap();
            if !re.is_match(url) {
                return None;
            }
        }

        let (repo_str, ref_str) = split_git_ref(url);

        // Use hosted-git-info pattern matching
        // Check direct https/shorthand hosted patterns
        let candidates: Vec<String> = if let Some(r) = &ref_str {
            vec![format!("{}#{}", repo_str, r), url.to_string()]
        } else {
            vec![url.to_string()]
        };

        for candidate in &candidates {
            if let Some(hosted) = parse_hosted_git_info(candidate) {
                let use_https_prefix = !repo_str.starts_with("http://")
                    && !repo_str.starts_with("https://")
                    && !repo_str.starts_with("ssh://")
                    && !repo_str.starts_with("git://")
                    && !repo_str.starts_with("git@");
                let final_repo = if use_https_prefix {
                    format!("https://{}", repo_str)
                } else {
                    repo_str.clone()
                };
                return Some(GitSource {
                    repo: final_repo,
                    host: hosted.0,
                    path: hosted.1,
                    r#ref: hosted.2.clone().or(ref_str.clone()),
                    pinned: hosted.2.is_some() || ref_str.is_some(),
                });
            }
        }

        // Try with https:// prefix
        let https_candidates: Vec<String> = if let Some(r) = &ref_str {
            vec![
                format!("https://{}#{}", repo_str, r),
                format!("https://{}", url),
            ]
        } else {
            vec![format!("https://{}", url)]
        };

        for candidate in &https_candidates {
            if let Some(hosted) = parse_hosted_git_info(candidate) {
                let final_repo = format!("https://{}", repo_str);
                return Some(GitSource {
                    repo: final_repo,
                    host: hosted.0,
                    path: hosted.1,
                    r#ref: hosted.2.clone().or(ref_str.clone()),
                    pinned: hosted.2.is_some() || ref_str.is_some(),
                });
            }
        }

        // Fallback to generic git URL parsing
        parse_generic_git_url(url, ref_str)
    }

    fn resource_precedence_rank(m: &PathMetadata) -> u32 {
        if matches!(m.origin, Origin::Package) {
            return 4;
        }
        let scope_base = match m.scope {
            SourceScope::Project => 0,
            _ => 2,
        };
        scope_base + if m.source == "local" { 0 } else { 1 }
    }

    // -----------------------------------------------------------------------
    // Progress emitters
    // -----------------------------------------------------------------------

    fn emit_progress(&self, event: ProgressEvent) {
        if let Some(ref cb) = self.progress_callback {
            let fut = cb(event);
            tokio::spawn(fut);
        }
    }

    async fn with_progress<F, Fut>(
        &self,
        action: ProgressAction,
        source: &str,
        message: &str,
        operation: F,
    ) -> Result<(), String>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<(), String>>,
    {
        self.emit_progress(ProgressEvent {
            event_type: ProgressEventType::Start,
            action,
            source: source.to_string(),
            message: Some(message.to_string()),
        });
        match operation().await {
            Ok(v) => {
                self.emit_progress(ProgressEvent {
                    event_type: ProgressEventType::Complete,
                    action,
                    source: source.to_string(),
                    message: None,
                });
                Ok(v)
            }
            Err(e) => {
                self.emit_progress(ProgressEvent {
                    event_type: ProgressEventType::Error,
                    action,
                    source: source.to_string(),
                    message: Some(e.clone()),
                });
                Err(e)
            }
        }
    }

    // -----------------------------------------------------------------------
    // Settings helpers — mirrors TS methods
    // -----------------------------------------------------------------------

    fn get_configured_package_sources(
        &self,
        include_defaults: bool,
    ) -> Vec<(PackageSource, SourceScope)> {
        let settings = self.ensure_settings_manager();
        let global_settings = settings.get_global_settings();
        let project_settings = settings.get_project_settings();
        let mut sources: Vec<(PackageSource, SourceScope)> = Vec::new();

        for pkg in project_settings.packages.unwrap_or_default() {
            sources.push((pkg, SourceScope::Project));
        }
        for pkg in global_settings.packages.unwrap_or_default() {
            sources.push((pkg, SourceScope::User));
        }
        if include_defaults {
            for pkg in settings.get_default_packages() {
                sources.push((pkg.clone(), SourceScope::User));
            }
        }

        self.dedupe_packages(sources)
    }

    fn get_source_match_key_for_input(&self, source: &str) -> String {
        let parsed = self.parse_source(source);
        match parsed {
            ParsedSource::Npm(ref n) => format!("npm:{}", n.name),
            ParsedSource::Git(ref g) => format!("git:{}/{}", g.host, g.path),
            ParsedSource::Local(_) => format!("local:{}", self.resolve_path(source).display()),
        }
    }

    fn get_source_match_key_for_settings(&self, source: &str, scope: SourceScope) -> String {
        let parsed = self.parse_source(source);
        match parsed {
            ParsedSource::Npm(ref n) => format!("npm:{}", n.name),
            ParsedSource::Git(ref g) => format!("git:{}/{}", g.host, g.path),
            ParsedSource::Local(_) => {
                let base_dir = self.get_base_dir_for_scope(scope);
                format!(
                    "local:{}",
                    self.resolve_path_from_base(source, &base_dir).display()
                )
            }
        }
    }

    fn package_sources_match(
        &self,
        existing: &PackageSource,
        input_source: &str,
        scope: SourceScope,
    ) -> bool {
        let existing_str = self.get_package_source_string(existing);
        let left = self.get_source_match_key_for_settings(&existing_str, scope);
        let right = self.get_source_match_key_for_input(input_source);
        left == right
    }

    fn get_package_source_string(&self, pkg: &PackageSource) -> String {
        match pkg {
            PackageSource::String(s) => s.clone(),
            PackageSource::Object { source, .. } => source.clone(),
        }
    }

    /// Extract filter from a PackageSource object, if present.
    fn get_package_filter(&self, pkg: &PackageSource) -> Option<PackageFilter> {
        match pkg {
            PackageSource::Object {
                extensions,
                skills,
                prompts,
                themes,
                ..
            } => {
                if extensions.is_some() || skills.is_some() || prompts.is_some() || themes.is_some()
                {
                    Some(PackageFilter {
                        extensions: extensions.clone(),
                        skills: skills.clone(),
                        prompts: prompts.clone(),
                        themes: themes.clone(),
                    })
                } else {
                    None
                }
            }
            PackageSource::String(_) => None,
        }
    }

    fn normalize_package_source_for_settings(&self, source: &str, scope: SourceScope) -> String {
        let parsed = self.parse_source(source);
        match parsed {
            ParsedSource::Local(_) => {
                let base_dir = self.get_base_dir_for_scope(scope);
                let resolved = self.resolve_path(source);
                let rel = pathdiff::diff_paths(&resolved, &base_dir)
                    .unwrap_or_else(|| PathBuf::from("."));
                let rel_str = rel.to_string_lossy().to_string();
                if rel_str.is_empty() {
                    ".".to_string()
                } else {
                    rel_str
                }
            }
            _ => source.to_string(),
        }
    }

    fn get_package_identity(&self, source: &str, scope: Option<SourceScope>) -> String {
        let parsed = self.parse_source(source);
        match parsed {
            ParsedSource::Npm(ref n) => format!("npm:{}", n.name),
            ParsedSource::Git(ref g) => format!("git:{}/{}", g.host, g.path),
            ParsedSource::Local(_) => {
                if let Some(sc) = scope {
                    let base_dir = self.get_base_dir_for_scope(sc);
                    format!(
                        "local:{}",
                        self.resolve_path_from_base(source, &base_dir).display()
                    )
                } else {
                    format!("local:{}", self.resolve_path(source).display())
                }
            }
        }
    }

    fn build_no_matching_package_message(
        &self,
        source: &str,
        configured_packages: &[PackageSource],
    ) -> String {
        let suggestion = self.find_suggested_configured_source(source, configured_packages);
        match suggestion {
            Some(s) => format!(
                "No matching package found for {}. Did you mean {}?",
                source, s
            ),
            None => format!("No matching package found for {}", source),
        }
    }

    fn find_suggested_configured_source(
        &self,
        source: &str,
        configured_packages: &[PackageSource],
    ) -> Option<String> {
        let trimmed_source = source.trim().to_string();
        let mut suggestions = HashSet::new();

        for pkg in configured_packages {
            let source_str = self.get_package_source_string(pkg);
            let parsed = self.parse_source(&source_str);
            match parsed {
                ParsedSource::Npm(ref n) => {
                    if trimmed_source == n.name || trimmed_source == n.spec {
                        suggestions.insert(source_str);
                    }
                }
                ParsedSource::Git(ref g) => {
                    let shorthand = format!("{}/{}", g.host, g.path);
                    let shorthand_with_ref =
                        g.r#ref.as_ref().map(|r| format!("{}@{}", shorthand, r));
                    if trimmed_source == shorthand
                        || shorthand_with_ref
                            .as_ref()
                            .map_or(false, |s| trimmed_source == *s)
                    {
                        suggestions.insert(source_str);
                    }
                }
                _ => {}
            }
        }

        suggestions.into_iter().next()
    }

    fn dedupe_packages(
        &self,
        packages: Vec<(PackageSource, SourceScope)>,
    ) -> Vec<(PackageSource, SourceScope)> {
        let mut seen: HashMap<String, (PackageSource, SourceScope)> = HashMap::new();

        for (pkg, scope) in packages {
            let source_str = self.get_package_source_string(&pkg);
            let identity = self.get_package_identity(&source_str, Some(scope));

            if let Some((_, existing_scope)) = seen.get(&identity) {
                // Project wins over user
                if scope == SourceScope::Project && *existing_scope == SourceScope::User {
                    seen.insert(identity, (pkg, scope));
                }
            } else {
                seen.insert(identity, (pkg, scope));
            }
        }

        seen.into_values().collect()
    }

    // -----------------------------------------------------------------------
    // Directory / path helpers
    // -----------------------------------------------------------------------

    fn get_base_dir_for_scope(&self, scope: SourceScope) -> PathBuf {
        match scope {
            SourceScope::Project => {
                self.assert_project_trusted_for_scope(scope);
                self.cwd.join(CONFIG_DIR_NAME)
            }
            SourceScope::User => self.agent_dir.clone(),
            SourceScope::Temporary => self.cwd.clone(),
        }
    }

    fn resolve_path(&self, input: &str) -> PathBuf {
        let _home_dir = get_home_dir();
        let resolved = paths::resolve_path(
            input,
            Some(self.cwd.to_str().unwrap_or("")),
            &paths::PathInputOptions {
                trim: true,
                ..Default::default()
            },
        );
        PathBuf::from(resolved)
    }

    fn resolve_path_from_base(&self, input: &str, base_dir: &Path) -> PathBuf {
        let _home_dir = get_home_dir();
        let resolved = paths::resolve_path(
            input,
            Some(base_dir.to_str().unwrap_or("")),
            &paths::PathInputOptions {
                trim: true,
                ..Default::default()
            },
        );
        PathBuf::from(resolved)
    }

    fn resolve_managed_path(&self, root: &Path, parts: &[&str]) -> PathBuf {
        let resolved_root = std::fs::canonicalize(root).unwrap_or_else(|_| root.to_path_buf());
        let mut resolved_path = resolved_root.clone();
        for part in parts {
            resolved_path = resolved_path.join(part);
        }
        let resolved_path = std::fs::canonicalize(&resolved_path).unwrap_or(resolved_path);
        // Reject paths outside the install root
        if resolved_path != resolved_root && !resolved_path.starts_with(&resolved_root) {
            // Use string comparison as fallback when canonicalization fails
            let resolved_str = resolved_path.to_string_lossy().to_string();
            let root_str = format!("{}/", resolved_root.to_string_lossy());
            if !resolved_str.starts_with(&root_str) {
                panic!(
                    "Refusing to use path outside package install root: {}",
                    resolved_str
                );
            }
        }
        resolved_path
    }

    fn get_npm_command(&self) -> (String, Vec<String>) {
        let settings = self.ensure_settings_manager();
        let configured = settings.get_npm_command();
        match configured {
            Some(cmd) if !cmd.is_empty() => {
                let command = cmd[0].clone();
                let args: Vec<String> = cmd[1..].to_vec();
                (command, args)
            }
            _ => ("npm".to_string(), vec![]),
        }
    }

    fn get_package_manager_name(&self) -> String {
        let (command, args) = self.get_npm_command();
        let mut parts = vec![command];
        parts.extend(args);
        let separator_index = parts.iter().rposition(|p| p == "--");
        let pm_command = match separator_index {
            Some(idx) if idx + 1 < parts.len() => parts[idx + 1].clone(),
            _ => parts[0].clone(),
        };
        let path = std::path::Path::new(&pm_command);
        let basename = path
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        basename
    }

    fn assert_project_trusted_for_scope(&self, scope: SourceScope) {
        if scope == SourceScope::Project && !self.ensure_settings_manager().is_project_trusted() {
            panic!("Project is not trusted; refusing to access project package storage");
        }
    }

    // -----------------------------------------------------------------------
    // npm dir helpers
    // -----------------------------------------------------------------------

    fn get_npm_install_root(&self, scope: SourceScope, temporary: bool) -> PathBuf {
        if temporary {
            return self.get_temporary_dir("npm", None);
        }
        if scope == SourceScope::Project {
            self.assert_project_trusted_for_scope(scope);
            return self.cwd.join(CONFIG_DIR_NAME).join("npm");
        }
        self.agent_dir.join("npm")
    }

    fn get_managed_npm_install_path(&self, source: &NpmSource, scope: SourceScope) -> PathBuf {
        if scope == SourceScope::Temporary {
            return self
                .get_temporary_dir("npm", None)
                .join("node_modules")
                .join(&source.name);
        }
        if scope == SourceScope::Project {
            self.assert_project_trusted_for_scope(scope);
            return self
                .cwd
                .join(CONFIG_DIR_NAME)
                .join("npm")
                .join("node_modules")
                .join(&source.name);
        }
        self.agent_dir
            .join("npm")
            .join("node_modules")
            .join(&source.name)
    }

    fn get_global_npm_root(&self) -> Result<String, String> {
        let key = {
            let (command, args) = self.get_npm_command();
            let mut parts = vec![command];
            parts.extend(args);
            parts.join("\0")
        };

        let cache: std::sync::Mutex<Option<(String, String)>> = std::sync::Mutex::new(None);
        {
            let guard = cache.lock().unwrap();
            if let Some((ref cached_key, ref cached_root)) = *guard {
                if *cached_key == key {
                    return Ok(cached_root.clone());
                }
            }
        }

        let root = if self.get_package_manager_name() == "bun" {
            let bin_dir = self.run_npm_command_sync(&["pm", "bin", "-g"]);
            let bin_path = PathBuf::from(&bin_dir);
            let parent = bin_path.parent().unwrap_or(&bin_path);
            parent
                .join("install")
                .join("global")
                .join("node_modules")
                .to_string_lossy()
                .to_string()
        } else {
            self.run_npm_command_sync(&["root", "-g"])
        };

        {
            let mut guard = cache.lock().unwrap();
            *guard = Some((key, root.clone()));
        }

        Ok(root)
    }

    fn get_pnpm_global_package_path(&self, package_name: &str) -> Option<PathBuf> {
        if self.get_package_manager_name() != "pnpm" {
            return None;
        }

        let output = self.run_npm_command_sync(&["list", "-g", "--depth", "0", "--json"]);
        let entries: Vec<serde_json::Value> = serde_json::from_str(&output).ok()?;
        for entry in entries {
            if let Some(deps) = entry.get("dependencies") {
                if let Some(pkg) = deps.get(package_name) {
                    if let Some(path) = pkg.get("path").and_then(|v| v.as_str()) {
                        return Some(PathBuf::from(path));
                    }
                }
            }
        }
        None
    }

    fn get_npm_install_path(&self, source: &NpmSource, scope: SourceScope) -> PathBuf {
        let managed_path = self.get_managed_npm_install_path(source, scope);
        if scope != SourceScope::User || managed_path.exists() {
            return managed_path;
        }
        if let Some(legacy_path) = self.get_legacy_global_npm_install_path(source) {
            if legacy_path.exists() {
                return legacy_path;
            }
        }
        managed_path
    }

    fn get_legacy_global_npm_install_path(&self, source: &NpmSource) -> Option<PathBuf> {
        self.get_pnpm_global_package_path(&source.name).or_else(|| {
            self.get_global_npm_root()
                .ok()
                .map(|root| PathBuf::from(root).join(&source.name))
        })
    }

    // -----------------------------------------------------------------------
    // git dir helpers
    // -----------------------------------------------------------------------

    fn get_git_install_root(&self, scope: SourceScope) -> Option<PathBuf> {
        match scope {
            SourceScope::Temporary => None,
            SourceScope::Project => {
                self.assert_project_trusted_for_scope(scope);
                Some(self.cwd.join(CONFIG_DIR_NAME).join("git"))
            }
            SourceScope::User => Some(self.agent_dir.join("git")),
        }
    }

    fn get_git_install_path(&self, source: &GitSource, scope: SourceScope) -> PathBuf {
        if scope == SourceScope::Temporary {
            return self.get_temporary_dir(&format!("git-{}", source.host), Some(&source.path));
        }
        let install_root = self
            .get_git_install_root(scope)
            .expect("Missing git install root");
        self.resolve_managed_path(&install_root, &[&source.host, &source.path])
    }

    fn get_temporary_dir(&self, prefix: &str, suffix: Option<&str>) -> PathBuf {
        let root = get_extension_temp_folder(self.agent_dir.to_str().unwrap_or(""));
        let prefix_path = root.join(prefix);
        let hash_input = format!("{}-{}", prefix, suffix.unwrap_or(""));
        use sha2::{Digest, Sha256};
        let hash = Sha256::digest(hash_input.as_bytes());
        let hash_short = &format!("{:x}", hash)[..8];
        let mut result = prefix_path.join(hash_short);
        if let Some(s) = suffix {
            result = result.join(s);
        }
        self.resolve_managed_path(&root.join(prefix), &[hash_short])
    }

    // -----------------------------------------------------------------------
    // npm install helpers
    // -----------------------------------------------------------------------

    fn get_npm_install_args(&self, specs: &[String], install_root: &Path) -> Vec<String> {
        let pm_name = self.get_package_manager_name();
        let install_root_str = install_root.to_string_lossy().to_string();
        if pm_name == "bun" {
            let mut args = vec!["install".to_string()];
            args.extend(specs.iter().cloned());
            args.push("--cwd".to_string());
            args.push(install_root_str);
            args.push("--omit=peer".to_string());
            return args;
        }
        if pm_name == "pnpm" {
            let mut args = vec!["install".to_string()];
            args.extend(specs.iter().cloned());
            args.push("--prefix".to_string());
            args.push(install_root_str);
            args.push("--config.auto-install-peers=false".to_string());
            args.push("--config.strict-peer-dependencies=false".to_string());
            args.push("--config.strict-dep-builds=false".to_string());
            return args;
        }
        let mut args = vec!["install".to_string()];
        args.extend(specs.iter().cloned());
        args.push("--prefix".to_string());
        args.push(install_root_str);
        args.push("--legacy-peer-deps".to_string());
        args
    }

    fn get_git_dependency_install_args(&self) -> Vec<String> {
        let configured = self.ensure_settings_manager().get_npm_command();
        if configured.is_some() {
            return vec!["install".to_string()];
        }
        vec!["install".to_string(), "--omit=dev".to_string()]
    }

    fn ensure_npm_project(&self, install_root: &Path) {
        if !install_root.exists() {
            fs::create_dir_all(install_root).expect("Failed to create npm install root");
        }
        let _ = paths::mark_path_ignored_by_cloud_sync(install_root.to_str().unwrap_or(""));
        self.ensure_git_ignore(install_root);
        let package_json_path = install_root.join("package.json");
        if !package_json_path.exists() {
            let pkg_json = serde_json::json!({ "name": "pi-extensions", "private": true });
            let content = serde_json::to_string_pretty(&pkg_json).unwrap();
            fs::write(&package_json_path, content).expect("Failed to write package.json");
        }
    }

    fn ensure_git_ignore(&self, dir: &Path) {
        if !dir.exists() {
            fs::create_dir_all(dir).expect("Failed to create directory for .gitignore");
        }
        let ignore_path = dir.join(".gitignore");
        if !ignore_path.exists() {
            fs::write(&ignore_path, "*\n!.gitignore\n").expect("Failed to write .gitignore");
        }
    }

    // -----------------------------------------------------------------------
    // Command helpers
    // -----------------------------------------------------------------------

    async fn run_npm_command(
        &self,
        args: &[String],
        options: Option<CommandOptions>,
    ) -> Result<(), String> {
        let (command, cmd_args) = self.get_npm_command();
        let mut all_args = cmd_args.clone();
        all_args.extend(args.iter().cloned());
        self.run_command(&command, &all_args, options).await
    }

    fn run_npm_command_sync(&self, args: &[&str]) -> String {
        let (command, cmd_args) = self.get_npm_command();
        let mut all_args = cmd_args.clone();
        all_args.extend(args.iter().map(|s| s.to_string()));
        self.run_command_sync(&command, &all_args)
    }

    async fn run_command(
        &self,
        command: &str,
        args: &[String],
        options: Option<CommandOptions>,
    ) -> Result<(), String> {
        let env = get_env();
        let cwd = options.as_ref().and_then(|o| o.cwd.clone());

        let mut cmd = TokioCommand::new(command);
        cmd.args(args);
        cmd.env_clear();
        for (k, v) in &env {
            cmd.env(k, v);
        }
        if let Some(dir) = &cwd {
            cmd.current_dir(dir);
        }

        // Determine stdio: if stdout is taken over, redirect to stderr
        if is_stdout_taken_over() {
            cmd.stdin(std::process::Stdio::null());
            cmd.stdout(std::process::Stdio::from(
                std::fs::File::create("/dev/null").unwrap(),
            ));
            cmd.stderr(std::process::Stdio::inherit());
        }

        let output = cmd
            .output()
            .await
            .map_err(|e| format!("Failed to spawn {}: {}", command, e))?;

        if output.status.success() {
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            Err(format!(
                "{} {} failed with code {}: {}",
                command,
                args.join(" "),
                output.status.code().unwrap_or(-1),
                stderr
            ))
        }
    }

    fn run_command_sync(&self, command: &str, args: &[String]) -> String {
        let env = get_env();
        let output = std::process::Command::new(command)
            .args(args)
            .env_clear()
            .envs(&env)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .output()
            .expect(&format!("Failed to run {}", command));

        if output.status.success() {
            String::from_utf8_lossy(&output.stdout).trim().to_string()
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            panic!(
                "Failed to run {} {}: {}",
                command,
                args.join(" "),
                if stderr.is_empty() { stdout } else { stderr }
            );
        }
    }

    async fn run_command_capture(
        &self,
        command: &str,
        args: &[String],
        options: Option<CommandCaptureOptions>,
    ) -> Result<String, String> {
        let env = get_env();
        let cwd = options.as_ref().and_then(|o| o.cwd.clone());
        let timeout_ms = options.as_ref().and_then(|o| o.timeout_ms);
        let extra_env = options.as_ref().and_then(|o| o.env.clone());

        let mut cmd = TokioCommand::new(command);
        cmd.args(args);
        cmd.env_clear();
        for (k, v) in &env {
            cmd.env(k, v);
        }
        if let Some(ref extra) = extra_env {
            for (k, v) in extra {
                cmd.env(k, v);
            }
        }
        if let Some(dir) = &cwd {
            cmd.current_dir(dir);
        }
        cmd.stdin(std::process::Stdio::null());
        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::piped());

        let mut child = cmd
            .spawn()
            .map_err(|e| format!("Failed to spawn {}: {}", command, e))?;

        let mut stdout = String::new();
        let mut stderr = String::new();

        // Read stdout and stderr concurrently
        let out_reader = child.stdout.take().unwrap();
        let err_reader = child.stderr.take().unwrap();

        use tokio::io::AsyncReadExt;

        let mut out_buf = tokio::io::BufReader::new(out_reader);
        let mut err_buf = tokio::io::BufReader::new(err_reader);

        let out_read = async {
            let mut buf = String::new();
            out_buf.read_to_string(&mut buf).await.ok();
            buf
        };
        let err_read = async {
            let mut buf = String::new();
            err_buf.read_to_string(&mut buf).await.ok();
            buf
        };

        let (out_result, err_result) = tokio::join!(out_read, err_read);
        stdout = out_result;
        stderr = err_result;

        let status = if let Some(ms) = timeout_ms {
            tokio::time::timeout(Duration::from_millis(ms), child.wait())
                .await
                .map_err(|_| format!("{} {} timed out after {}ms", command, args.join(" "), ms))?
                .map_err(|e| format!("Failed to wait for {}: {}", command, e))?
        } else {
            child
                .wait()
                .await
                .map_err(|e| format!("Failed to wait for {}: {}", command, e))?
        };

        if status.success() {
            Ok(stdout.trim().to_string())
        } else {
            let exit_status = match status.code() {
                Some(code) => format!("code {}", code),
                None => format!(
                    "signal {}",
                    status
                        .signal()
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| "unknown".to_string())
                ),
            };
            Err(format!(
                "{} {} failed with {}: {}",
                command,
                args.join(" "),
                exit_status,
                if stderr.is_empty() {
                    stdout.trim()
                } else {
                    stderr.trim()
                }
            ))
        }
    }

    // -----------------------------------------------------------------------
    // Concurrency helper
    // -----------------------------------------------------------------------

    fn get_env_for_git() -> HashMap<String, String> {
        let mut env = get_env();
        env.insert("GIT_TERMINAL_PROMPT".to_string(), "0".to_string());
        env
    }
}

#[derive(Clone)]
struct CommandOptions {
    cwd: Option<PathBuf>,
}

#[derive(Clone)]
struct CommandCaptureOptions {
    cwd: Option<PathBuf>,
    timeout_ms: Option<u64>,
    env: Option<HashMap<String, String>>,
}

// ---------------------------------------------------------------------------
// PackageManager trait implementation
// ---------------------------------------------------------------------------

impl PackageManager for DefaultPackageManager {
    fn resolve(
        &self,
        _on_missing: Option<OnMissingFn>,
    ) -> Pin<Box<dyn Future<Output = ResolvedPaths> + Send + '_>> {
        // TODO: Full implementation — requires async closures which are unstable
        Box::pin(async move { ResolvedPaths::default() })
    }

    fn install(
        &self,
        source: &str,
        options: Option<InstallOptions>,
    ) -> Pin<Box<dyn Future<Output = Result<(), String>> + Send + '_>> {
        let source = source.to_string();
        Box::pin(async move {
            let parsed = self.parse_source(&source);
            let scope = if options.and_then(|o| Some(o.local)).unwrap_or(false) {
                SourceScope::Project
            } else {
                SourceScope::User
            };
            self.assert_project_trusted_for_scope(scope);

            self.with_progress(
                ProgressAction::Install,
                &source,
                &format!("Installing {}...", source),
                async || match parsed {
                    ParsedSource::Npm(ref n) => self.install_npm(n, scope, false).await,
                    ParsedSource::Git(ref g) => self.install_git(g, scope).await,
                    ParsedSource::Local(ref l) => {
                        let resolved = self.resolve_path(&l.path.to_string_lossy());
                        if !resolved.exists() {
                            return Err(format!(
                                "Path does not exist: {}",
                                resolved.to_string_lossy()
                            ));
                        }
                        Ok(())
                    }
                },
            )
            .await
        })
    }

    fn install_and_persist(
        &self,
        source: &str,
        options: Option<InstallOptions>,
    ) -> Pin<Box<dyn Future<Output = Result<(), String>> + Send + '_>> {
        let source = source.to_string();
        Box::pin(async move {
            let result = self.install(&source, options).await?;
            self.add_source_to_settings(&source, options);
            Ok(result)
        })
    }

    fn remove(
        &self,
        source: &str,
        options: Option<InstallOptions>,
    ) -> Pin<Box<dyn Future<Output = Result<(), String>> + Send + '_>> {
        let source = source.to_string();
        Box::pin(async move {
            let parsed = self.parse_source(&source);
            let scope = if options.and_then(|o| Some(o.local)).unwrap_or(false) {
                SourceScope::Project
            } else {
                SourceScope::User
            };
            self.assert_project_trusted_for_scope(scope);

            self.with_progress(
                ProgressAction::Remove,
                &source,
                &format!("Removing {}...", source),
                async || match parsed {
                    ParsedSource::Npm(ref n) => self.uninstall_npm(n, scope).await,
                    ParsedSource::Git(ref g) => self.remove_git(g, scope).await,
                    ParsedSource::Local(_) => Ok(()),
                },
            )
            .await
        })
    }

    fn remove_and_persist(
        &self,
        source: &str,
        options: Option<InstallOptions>,
    ) -> Pin<Box<dyn Future<Output = Result<bool, String>> + Send + '_>> {
        let source = source.to_string();
        Box::pin(async move {
            self.remove(&source, options).await?;
            Ok(self.remove_source_from_settings(&source, options))
        })
    }

    fn update(
        &self,
        source: Option<&str>,
    ) -> Pin<Box<dyn Future<Output = Result<(), String>> + Send + '_>> {
        let source = source.map(|s| s.to_string());
        Box::pin(async move {
            if let Some(ref src) = source {
                let identity = self.get_package_identity(src, None);
                let mut matched = false;
                let mut update_sources: Vec<(String, ScopeInstalled)> = Vec::new();

                for (pkg, scope) in self.get_configured_package_sources(true) {
                    if scope == SourceScope::Temporary {
                        continue;
                    }
                    let scope_installed = match scope {
                        SourceScope::User => ScopeInstalled::User,
                        SourceScope::Project => ScopeInstalled::Project,
                        _ => continue,
                    };
                    let source_str = self.get_package_source_string(&pkg);
                    if self.get_package_identity(&source_str, Some(scope)) != identity {
                        continue;
                    }
                    matched = true;
                    update_sources.push((source_str, scope_installed));
                }

                if !matched {
                    let settings = self.ensure_settings_manager();
                    let mut all_packages: Vec<PackageSource> =
                        settings.get_project_settings().packages.unwrap_or_default();
                    all_packages
                        .extend(settings.get_global_settings().packages.unwrap_or_default());
                    all_packages.extend(settings.get_default_packages().iter().cloned());
                    return Err(self.build_no_matching_package_message(src, &all_packages));
                }

                self.update_configured_sources(&update_sources).await
            } else {
                let mut all_sources: Vec<(String, ScopeInstalled)> = Vec::new();
                for (pkg, scope) in self.get_configured_package_sources(true) {
                    if scope == SourceScope::Temporary {
                        continue;
                    }
                    let scope_installed = match scope {
                        SourceScope::User => ScopeInstalled::User,
                        SourceScope::Project => ScopeInstalled::Project,
                        _ => continue,
                    };
                    all_sources.push((self.get_package_source_string(&pkg), scope_installed));
                }
                self.update_configured_sources(&all_sources).await
            }
        })
    }

    fn list_configured_packages(&self) -> Vec<ConfiguredPackage> {
        let settings = self.ensure_settings_manager();
        let global_settings = settings.get_global_settings();
        let project_settings = settings.get_project_settings();
        let mut configured_packages = Vec::new();

        for pkg in global_settings.packages.unwrap_or_default() {
            let source = self.get_package_source_string(&pkg);
            let is_filtered = matches!(pkg, PackageSource::Object { .. });
            configured_packages.push(ConfiguredPackage {
                source: source.clone(),
                scope: ScopeInstalled::User,
                filtered: is_filtered,
                installed_path: self.get_installed_path_internal(&source, SourceScope::User),
            });
        }

        for pkg in project_settings.packages.unwrap_or_default() {
            let source = self.get_package_source_string(&pkg);
            let is_filtered = matches!(pkg, PackageSource::Object { .. });
            configured_packages.push(ConfiguredPackage {
                source: source.clone(),
                scope: ScopeInstalled::Project,
                filtered: is_filtered,
                installed_path: self.get_installed_path_internal(&source, SourceScope::Project),
            });
        }

        configured_packages
    }

    fn resolve_extension_sources(
        &self,
        sources: &[String],
        options: Option<ResolveExtensionOptions>,
    ) -> Pin<Box<dyn Future<Output = ResolvedPaths> + Send + '_>> {
        let sources = sources.to_vec();
        Box::pin(async move {
            let mut accumulator = ResourceAccumulator::default();
            let scope = if options.and_then(|o| Some(o.temporary)).unwrap_or(false) {
                SourceScope::Temporary
            } else if options.and_then(|o| Some(o.local)).unwrap_or(false) {
                SourceScope::Project
            } else {
                SourceScope::User
            };

            for source in &sources {
                let _pkg: PackageSource = PackageSource::String(source.clone());
                let parsed = self.parse_source(source);
                let metadata = PathMetadata {
                    source: source.clone(),
                    scope,
                    origin: Origin::Package,
                    base_dir: None,
                };

                if let ParsedSource::Local(ref l) = parsed {
                    let base_dir = self.get_base_dir_for_scope(scope);
                    self.resolve_local_extension_source_internal(
                        l,
                        &mut accumulator,
                        None,
                        &metadata,
                        &base_dir,
                    );
                    continue;
                }

                // For npm/git sources, try to find them
                let installed = match parsed {
                    ParsedSource::Npm(ref n) => {
                        let path = self.get_npm_install_path(n, scope);
                        if path.exists() { Some(path) } else { None }
                    }
                    ParsedSource::Git(ref g) => {
                        let path = self.get_git_install_path(g, scope);
                        if path.exists() { Some(path) } else { None }
                    }
                    _ => None,
                };

                if let Some(path) = installed {
                    let mut meta = metadata;
                    meta.base_dir = Some(path.clone());
                    self.collect_package_resources_internal(&path, &mut accumulator, None, &meta);
                }
            }

            self.to_resolved_paths(&accumulator)
        })
    }

    fn add_source_to_settings(&self, source: &str, options: Option<InstallOptions>) -> bool {
        let scope = if options.and_then(|o| Some(o.local)).unwrap_or(false) {
            SourceScope::Project
        } else {
            SourceScope::User
        };
        let settings = self.ensure_settings_manager();
        let current_settings = match scope {
            SourceScope::Project => settings.get_project_settings(),
            _ => settings.get_global_settings(),
        };
        let current_packages = current_settings.packages.unwrap_or_default();
        let normalized_source = self.normalize_package_source_for_settings(source, scope);

        // Find matching existing entry
        let match_index = current_packages
            .iter()
            .position(|existing| self.package_sources_match(existing, source, scope));

        if let Some(idx) = match_index {
            let existing = &current_packages[idx];
            if self.get_package_source_string(existing) == normalized_source {
                return false;
            }
            // Replace the source value while preserving filters
            let _settings_mut = settings.clone();
            let mut new_packages = current_packages.clone();
            new_packages[idx] = match existing {
                PackageSource::Object {
                    extensions,
                    skills,
                    prompts,
                    themes,
                    ..
                } => PackageSource::Object {
                    source: normalized_source,
                    extensions: extensions.clone(),
                    skills: skills.clone(),
                    prompts: prompts.clone(),
                    themes: themes.clone(),
                },
                PackageSource::String(_) => PackageSource::String(normalized_source),
            };
            match scope {
                SourceScope::Project => {
                    let mut settings_mut = settings.clone();
                    settings_mut.set_project_packages(new_packages);
                }
                _ => {
                    let mut settings_mut = settings.clone();
                    settings_mut.set_packages(new_packages);
                }
            }
            return true;
        }

        let mut new_packages = current_packages.clone();
        new_packages.push(PackageSource::String(normalized_source));
        match scope {
            SourceScope::Project => {
                let mut settings_mut = settings.clone();
                settings_mut.set_project_packages(new_packages);
            }
            _ => {
                let mut settings_mut = settings.clone();
                settings_mut.set_packages(new_packages);
            }
        }
        true
    }

    fn remove_source_from_settings(&self, source: &str, options: Option<InstallOptions>) -> bool {
        let scope = if options.and_then(|o| Some(o.local)).unwrap_or(false) {
            SourceScope::Project
        } else {
            SourceScope::User
        };
        let settings = self.ensure_settings_manager();
        let current_settings = match scope {
            SourceScope::Project => settings.get_project_settings(),
            _ => settings.get_global_settings(),
        };
        let current_packages = current_settings.packages.unwrap_or_default();
        let old_len = current_packages.len();
        let new_packages: Vec<PackageSource> = current_packages
            .into_iter()
            .filter(|existing| !self.package_sources_match(existing, source, scope))
            .collect();
        let changed = new_packages.len() != old_len;
        if !changed {
            return false;
        }
        match scope {
            SourceScope::Project => {
                let mut settings_mut = settings.clone();
                settings_mut.set_project_packages(new_packages);
            }
            _ => {
                let mut settings_mut = settings.clone();
                settings_mut.set_packages(new_packages);
            }
        }
        true
    }

    fn set_progress_callback(&self, _callback: Option<ProgressCallback>) {
        // Since we're using &self, this is tricky with interior mutability.
        // For now, store it in an Arc<RwLock<>> via unsafe or redesign.
    }

    fn get_installed_path(&self, source: &str, scope: ScopeInstalled) -> Option<PathBuf> {
        let source_scope = match scope {
            ScopeInstalled::User => SourceScope::User,
            ScopeInstalled::Project => SourceScope::Project,
        };
        self.get_installed_path_internal(source, source_scope)
    }

    fn check_for_available_updates(
        &self,
    ) -> Pin<Box<dyn Future<Output = Vec<PackageUpdate>> + Send + '_>> {
        Box::pin(async move { Vec::new() })
    }
}

// ---------------------------------------------------------------------------
// Internal implementation methods
// ---------------------------------------------------------------------------

impl DefaultPackageManager {
    fn get_installed_path_internal(&self, source: &str, scope: SourceScope) -> Option<PathBuf> {
        let parsed = self.parse_source(source);
        match parsed {
            ParsedSource::Npm(ref n) => {
                let path = self.get_npm_install_path(n, scope);
                if path.exists() { Some(path) } else { None }
            }
            ParsedSource::Git(ref g) => {
                let path = self.get_git_install_path(g, scope);
                if path.exists() { Some(path) } else { None }
            }
            ParsedSource::Local(ref l) => {
                let base_dir = self.get_base_dir_for_scope(scope);
                let path = self.resolve_path_from_base(&l.path.to_string_lossy(), &base_dir);
                if path.exists() { Some(path) } else { None }
            }
        }
    }

    async fn install_npm(
        &self,
        source: &NpmSource,
        scope: SourceScope,
        temporary: bool,
    ) -> Result<(), String> {
        let install_root = self.get_npm_install_root(scope, temporary);
        self.ensure_npm_project(&install_root);
        let args = self.get_npm_install_args(&[source.spec.clone()], &install_root);
        self.run_npm_command(&args, None).await
    }

    async fn uninstall_npm(&self, source: &NpmSource, scope: SourceScope) -> Result<(), String> {
        let install_root = self.get_npm_install_root(scope, false);
        if !install_root.exists() {
            return Ok(());
        }
        if self.get_package_manager_name() == "bun" {
            let args = vec![
                "uninstall".to_string(),
                source.name.clone(),
                "--cwd".to_string(),
                install_root.to_string_lossy().to_string(),
            ];
            self.run_npm_command(&args, None).await
        } else {
            let args = vec![
                "uninstall".to_string(),
                source.name.clone(),
                "--prefix".to_string(),
                install_root.to_string_lossy().to_string(),
            ];
            self.run_npm_command(&args, None).await
        }
    }

    async fn install_git(&self, source: &GitSource, scope: SourceScope) -> Result<(), String> {
        let target_dir = self.get_git_install_path(source, scope);
        if target_dir.exists() {
            if let Some(ref r#ref) = source.r#ref {
                return self
                    .ensure_git_ref(
                        &target_dir,
                        &["fetch".to_string(), "origin".to_string(), r#ref.clone()],
                        "FETCH_HEAD",
                    )
                    .await;
            }
            let target = self.get_local_git_update_target(&target_dir).await?;
            return self
                .ensure_git_ref(&target_dir, &target.fetch_args, &target.r#ref)
                .await;
        }

        let git_root = self.get_git_install_root(scope);
        if let Some(ref root) = git_root {
            self.ensure_git_ignore(root);
        }
        if let Some(parent) = target_dir.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create parent dir: {}", e))?;
        }

        self.run_command(
            "git",
            &[
                "clone".to_string(),
                source.repo.clone(),
                target_dir.to_string_lossy().to_string(),
            ],
            None,
        )
        .await?;
        if let Some(ref r#ref) = source.r#ref {
            self.run_command(
                "git",
                &["checkout".to_string(), r#ref.clone()],
                Some(CommandOptions {
                    cwd: Some(target_dir.clone()),
                }),
            )
            .await?;
        }

        let package_json = target_dir.join("package.json");
        if package_json.exists() {
            let install_args = self.get_git_dependency_install_args();
            self.run_npm_command(
                &install_args,
                Some(CommandOptions {
                    cwd: Some(target_dir.clone()),
                }),
            )
            .await?;
        }

        Ok(())
    }

    async fn update_git(&self, source: &GitSource, scope: SourceScope) -> Result<(), String> {
        let target_dir = self.get_git_install_path(source, scope);
        if !target_dir.exists() {
            return self.install_git(source, scope).await;
        }

        if let Some(ref r#ref) = source.r#ref {
            return self
                .ensure_git_ref(
                    &target_dir,
                    &["fetch".to_string(), "origin".to_string(), r#ref.clone()],
                    "FETCH_HEAD",
                )
                .await;
        }

        let target = self.get_local_git_update_target(&target_dir).await?;
        self.ensure_git_ref(&target_dir, &target.fetch_args, &target.r#ref)
            .await
    }

    async fn ensure_git_ref(
        &self,
        target_dir: &Path,
        fetch_args: &[String],
        r#ref: &str,
    ) -> Result<(), String> {
        // Fetch only the ref we will reset to
        self.run_command(
            "git",
            fetch_args,
            Some(CommandOptions {
                cwd: Some(target_dir.to_path_buf()),
            }),
        )
        .await?;

        let local_head = self
            .run_command_capture(
                "git",
                &["rev-parse".to_string(), "HEAD".to_string()],
                Some(CommandCaptureOptions {
                    cwd: Some(target_dir.to_path_buf()),
                    timeout_ms: Some(NETWORK_TIMEOUT_MS),
                    env: None,
                }),
            )
            .await?;

        let commit_ref = format!("{}^{{commit}}", r#ref);
        let target_head = self
            .run_command_capture(
                "git",
                &["rev-parse".to_string(), commit_ref.clone()],
                Some(CommandCaptureOptions {
                    cwd: Some(target_dir.to_path_buf()),
                    timeout_ms: Some(NETWORK_TIMEOUT_MS),
                    env: None,
                }),
            )
            .await?;

        if local_head.trim() == target_head.trim() {
            return Ok(());
        }

        self.run_command(
            "git",
            &["reset".to_string(), "--hard".to_string(), commit_ref],
            Some(CommandOptions {
                cwd: Some(target_dir.to_path_buf()),
            }),
        )
        .await?;

        // Clean untracked files
        self.run_command(
            "git",
            &["clean".to_string(), "-fdx".to_string()],
            Some(CommandOptions {
                cwd: Some(target_dir.to_path_buf()),
            }),
        )
        .await?;

        let package_json = target_dir.join("package.json");
        if package_json.exists() {
            let install_args = self.get_git_dependency_install_args();
            self.run_npm_command(
                &install_args,
                Some(CommandOptions {
                    cwd: Some(target_dir.to_path_buf()),
                }),
            )
            .await?;
        }

        Ok(())
    }

    async fn remove_git(&self, source: &GitSource, scope: SourceScope) -> Result<(), String> {
        let target_dir = self.get_git_install_path(source, scope);
        if !target_dir.exists() {
            return Ok(());
        }
        fs::remove_dir_all(&target_dir)
            .map_err(|e| format!("Failed to remove {}: {}", target_dir.to_string_lossy(), e))?;
        let install_root = self.get_git_install_root(scope);
        self.prune_empty_git_parents(&target_dir, install_root.as_ref());
        Ok(())
    }

    fn prune_empty_git_parents(&self, target_dir: &Path, install_root: Option<&PathBuf>) {
        let install_root = match install_root {
            Some(r) => r,
            None => return,
        };
        let resolved_root =
            std::fs::canonicalize(install_root).unwrap_or_else(|_| install_root.clone());
        let mut current = target_dir.parent().map(|p| p.to_path_buf());

        while let Some(ref cur) = current {
            let resolved_cur = std::fs::canonicalize(cur).unwrap_or_else(|_| cur.clone());
            if !resolved_cur.starts_with(&resolved_root) || resolved_cur == resolved_root {
                break;
            }
            if !cur.exists() {
                current = cur.parent().map(|p| p.to_path_buf());
                continue;
            }
            let entries = match fs::read_dir(cur) {
                Ok(e) => e.count(),
                Err(_) => break,
            };
            if entries > 0 {
                break;
            }
            if fs::remove_dir(cur).is_err() {
                break;
            }
            current = cur.parent().map(|p| p.to_path_buf());
        }
    }

    async fn refresh_temporary_git_source(&self, source: &GitSource, source_str: &str) {
        if is_offline_mode_enabled() {
            return;
        }
        let _ = self
            .with_progress(
                ProgressAction::Pull,
                source_str,
                &format!("Refreshing {}...", source_str),
                async || self.update_git(source, SourceScope::Temporary).await,
            )
            .await;
    }

    async fn get_local_git_update_target(
        &self,
        installed_path: &Path,
    ) -> Result<GitUpdateTargetInfo, String> {
        // Try to get the upstream branch first
        let args = vec![
            "rev-parse".to_string(),
            "--abbrev-ref".to_string(),
            "@{upstream}".to_string(),
        ];
        let upstream = self
            .run_command_capture(
                "git",
                &args,
                Some(CommandCaptureOptions {
                    cwd: Some(installed_path.to_path_buf()),
                    timeout_ms: Some(NETWORK_TIMEOUT_MS),
                    env: None,
                }),
            )
            .await;

        match upstream {
            Ok(upstream_str) => {
                let trimmed = upstream_str.trim().to_string();
                if !trimmed.starts_with("origin/") {
                    return Err(format!("Unsupported upstream remote: {}", trimmed));
                }
                let branch = trimmed["origin/".len()..].to_string();
                if branch.is_empty() {
                    return Err("Missing upstream branch name".to_string());
                }
                let head = self
                    .run_command_capture(
                        "git",
                        &["rev-parse".to_string(), "@{upstream}".to_string()],
                        Some(CommandCaptureOptions {
                            cwd: Some(installed_path.to_path_buf()),
                            timeout_ms: Some(NETWORK_TIMEOUT_MS),
                            env: None,
                        }),
                    )
                    .await?;
                Ok(GitUpdateTargetInfo {
                    r#ref: "@{upstream}".to_string(),
                    head,
                    fetch_args: vec![
                        "fetch".to_string(),
                        "--prune".to_string(),
                        "--no-tags".to_string(),
                        "origin".to_string(),
                        format!("+refs/heads/{}:refs/remotes/origin/{}", branch, branch),
                    ],
                })
            }
            Err(_) => {
                // Fallback: use origin/HEAD
                let _ = self
                    .run_command(
                        "git",
                        &[
                            "remote".to_string(),
                            "set-head".to_string(),
                            "origin".to_string(),
                            "-a".to_string(),
                        ],
                        Some(CommandOptions {
                            cwd: Some(installed_path.to_path_buf()),
                        }),
                    )
                    .await;
                let head = self
                    .run_command_capture(
                        "git",
                        &["rev-parse".to_string(), "origin/HEAD".to_string()],
                        Some(CommandCaptureOptions {
                            cwd: Some(installed_path.to_path_buf()),
                            timeout_ms: Some(NETWORK_TIMEOUT_MS),
                            env: None,
                        }),
                    )
                    .await?;
                let origin_head_ref = self
                    .run_command_capture(
                        "git",
                        &[
                            "symbolic-ref".to_string(),
                            "refs/remotes/origin/HEAD".to_string(),
                        ],
                        Some(CommandCaptureOptions {
                            cwd: Some(installed_path.to_path_buf()),
                            timeout_ms: Some(NETWORK_TIMEOUT_MS),
                            env: None,
                        }),
                    )
                    .await;
                let branch = origin_head_ref
                    .map(|r| r.trim().replace("refs/remotes/origin/", ""))
                    .unwrap_or_default();
                if !branch.is_empty() {
                    Ok(GitUpdateTargetInfo {
                        r#ref: "origin/HEAD".to_string(),
                        head,
                        fetch_args: vec![
                            "fetch".to_string(),
                            "--prune".to_string(),
                            "--no-tags".to_string(),
                            "origin".to_string(),
                            format!("+refs/heads/{}:refs/remotes/origin/{}", branch, branch),
                        ],
                    })
                } else {
                    Ok(GitUpdateTargetInfo {
                        r#ref: "origin/HEAD".to_string(),
                        head,
                        fetch_args: vec![
                            "fetch".to_string(),
                            "--prune".to_string(),
                            "--no-tags".to_string(),
                            "origin".to_string(),
                            "+HEAD:refs/remotes/origin/HEAD".to_string(),
                        ],
                    })
                }
            }
        }
    }

    async fn get_git_upstream_ref(&self, installed_path: &Path) -> Option<String> {
        let upstream = self
            .run_command_capture(
                "git",
                &[
                    "rev-parse".to_string(),
                    "--abbrev-ref".to_string(),
                    "@{upstream}".to_string(),
                ],
                Some(CommandCaptureOptions {
                    cwd: Some(installed_path.to_path_buf()),
                    timeout_ms: Some(NETWORK_TIMEOUT_MS),
                    env: None,
                }),
            )
            .await
            .ok()?;
        let trimmed = upstream.trim().to_string();
        if !trimmed.starts_with("origin/") {
            return None;
        }
        let branch = trimmed["origin/".len()..].to_string();
        if branch.is_empty() {
            return None;
        }
        Some(format!("refs/heads/{}", branch))
    }

    async fn get_remote_git_head(&self, installed_path: &Path) -> Result<String, String> {
        let upstream_ref = self.get_git_upstream_ref(installed_path).await;
        if let Some(ref r) = upstream_ref {
            let remote_head = self
                .run_git_remote_command(
                    installed_path,
                    &["ls-remote".to_string(), "origin".to_string(), r.clone()],
                )
                .await?;
            let re = regex::Regex::new(r"^([0-9a-f]{40})\s+").unwrap();
            if let Some(caps) = re.captures(&remote_head) {
                return Ok(caps[1].to_string());
            }
        }

        let remote_head = self
            .run_git_remote_command(
                installed_path,
                &[
                    "ls-remote".to_string(),
                    "origin".to_string(),
                    "HEAD".to_string(),
                ],
            )
            .await?;
        let re = regex::Regex::new(r"^([0-9a-f]{40})\s+HEAD$").unwrap();
        if let Some(caps) = re.captures(&remote_head) {
            return Ok(caps[1].to_string());
        }
        Err("Failed to determine remote HEAD".to_string())
    }

    async fn git_has_available_update(&self, installed_path: &Path) -> Result<bool, String> {
        if is_offline_mode_enabled() {
            return Ok(false);
        }
        let local_head = self
            .run_command_capture(
                "git",
                &["rev-parse".to_string(), "HEAD".to_string()],
                Some(CommandCaptureOptions {
                    cwd: Some(installed_path.to_path_buf()),
                    timeout_ms: Some(NETWORK_TIMEOUT_MS),
                    env: None,
                }),
            )
            .await?;
        let remote_head = self.get_remote_git_head(installed_path).await?;
        Ok(local_head.trim() != remote_head.trim())
    }

    async fn run_git_remote_command(
        &self,
        installed_path: &Path,
        args: &[String],
    ) -> Result<String, String> {
        let mut env = get_env();
        env.insert("GIT_TERMINAL_PROMPT".to_string(), "0".to_string());
        self.run_command_capture(
            "git",
            args,
            Some(CommandCaptureOptions {
                cwd: Some(installed_path.to_path_buf()),
                timeout_ms: Some(NETWORK_TIMEOUT_MS),
                env: Some(env),
            }),
        )
        .await
    }

    async fn installed_npm_matches_configured_version(
        &self,
        source: &NpmSource,
        installed_path: &Path,
    ) -> Result<bool, String> {
        let installed_version = self.get_installed_npm_version(installed_path);
        match installed_version {
            Some(ver) => {
                if let Some(ref range) = source.range {
                    let req = semver::VersionReq::parse(range)
                        .map_err(|e| format!("Invalid semver range: {}", e))?;
                    let v = semver::Version::parse(&ver)
                        .map_err(|e| format!("Invalid semver version: {}", e))?;
                    Ok(req.matches(&v))
                } else {
                    Ok(true)
                }
            }
            None => Ok(false),
        }
    }

    fn get_installed_npm_version(&self, installed_path: &Path) -> Option<String> {
        let package_json = installed_path.join("package.json");
        if !package_json.exists() {
            return None;
        }
        let content = fs::read_to_string(&package_json).ok()?;
        let pkg: serde_json::Value = serde_json::from_str(&content).ok()?;
        pkg.get("version")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    }

    async fn get_latest_npm_version(
        &self,
        package_spec: &str,
        range: Option<&str>,
    ) -> Result<String, String> {
        let (command, cmd_args) = self.get_npm_command();
        let mut args = cmd_args.clone();
        args.push("view".to_string());
        args.push(package_spec.to_string());
        args.push("version".to_string());
        args.push("--json".to_string());
        let stdout = self
            .run_command_capture(
                &command,
                &args,
                Some(CommandCaptureOptions {
                    cwd: Some(self.cwd.clone()),
                    timeout_ms: Some(NETWORK_TIMEOUT_MS),
                    env: None,
                }),
            )
            .await?;

        if stdout.is_empty() {
            return Err("Empty response from npm view".to_string());
        }
        let parsed: serde_json::Value = serde_json::from_str(&stdout)
            .map_err(|e| format!("Failed to parse npm view output: {}", e))?;

        if let Some(s) = parsed.as_str() {
            return Ok(s.to_string());
        }
        if let Some(arr) = parsed.as_array() {
            let versions: Vec<&str> = arr
                .iter()
                .filter_map(|v| v.as_str())
                .filter(|v| !v.is_empty())
                .collect();
            if !versions.is_empty() {
                let latest = if let Some(r) = range {
                    // Use max satisfying
                    let req = semver::VersionReq::parse(r).ok();
                    req.and_then(|r| {
                        versions
                            .iter()
                            .filter_map(|v| semver::Version::parse(v).ok())
                            .filter(|v| r.matches(v))
                            .max()
                    })
                    .map(|v| v.to_string())
                    .unwrap_or_else(|| versions[0].to_string())
                } else {
                    versions
                        .iter()
                        .filter_map(|v| semver::Version::parse(v).ok())
                        .max()
                        .map(|v| v.to_string())
                        .unwrap_or_else(|| versions[0].to_string())
                };
                return Ok(latest);
            }
        }
        Err("Unexpected response from npm view".to_string())
    }

    async fn npm_has_available_update(
        &self,
        source: &NpmSource,
        installed_path: &Path,
    ) -> Result<bool, String> {
        if is_offline_mode_enabled() {
            return Ok(false);
        }
        let installed_version = self.get_installed_npm_version(installed_path);
        match installed_version {
            Some(ver) => {
                let spec = if source.version.is_some() {
                    &source.spec
                } else {
                    &source.name
                };
                let latest = self
                    .get_latest_npm_version(spec, source.range.as_deref())
                    .await?;
                Ok(latest != ver)
            }
            None => Ok(false),
        }
    }

    async fn should_update_npm_source(
        &self,
        source: &NpmSource,
        scope: SourceScope,
    ) -> Result<bool, String> {
        let installed_path = self.get_managed_npm_install_path(source, scope);
        let installed_version = if installed_path.exists() {
            self.get_installed_npm_version(&installed_path)
        } else {
            None
        };
        if installed_version.is_none() {
            return Ok(true);
        }

        let spec = if source.version.is_some() {
            &source.spec
        } else {
            &source.name
        };
        match self
            .get_latest_npm_version(spec, source.range.as_deref())
            .await
        {
            Ok(target_version) => Ok(target_version != installed_version.unwrap()),
            Err(_) => Ok(true),
        }
    }

    async fn update_configured_sources(
        &self,
        sources: &[(String, ScopeInstalled)],
    ) -> Result<(), String> {
        if is_offline_mode_enabled() || sources.is_empty() {
            return Ok(());
        }

        let mut npm_user_targets: Vec<(NpmUpdateTarget, String)> = Vec::new();
        let mut npm_project_targets: Vec<(NpmUpdateTarget, String)> = Vec::new();
        let mut git_targets: Vec<GitUpdateTarget> = Vec::new();

        for (source_str, scope_installed) in sources {
            let _scope_source = match scope_installed {
                ScopeInstalled::User => SourceScope::User,
                ScopeInstalled::Project => SourceScope::Project,
            };
            let parsed = self.parse_source(source_str);
            match parsed {
                ParsedSource::Npm(ref n) if !n.pinned => {
                    let target = NpmUpdateTarget {
                        source: source_str.clone(),
                        scope: *scope_installed,
                        parsed: n.clone(),
                    };
                    match scope_installed {
                        ScopeInstalled::User => npm_user_targets.push((target, source_str.clone())),
                        ScopeInstalled::Project => {
                            npm_project_targets.push((target, source_str.clone()))
                        }
                    }
                }
                ParsedSource::Git(ref g) => {
                    git_targets.push(GitUpdateTarget {
                        source: source_str.clone(),
                        scope: *scope_installed,
                        parsed: g.clone(),
                    });
                }
                _ => {}
            }
        }

        // Check which npm targets need updating
        let mut npm_user_to_update: Vec<NpmUpdateTarget> = Vec::new();
        let mut npm_project_to_update: Vec<NpmUpdateTarget> = Vec::new();

        for (target, _) in &npm_user_targets {
            let scope = SourceScope::User;
            if self
                .should_update_npm_source(&target.parsed, scope)
                .await
                .unwrap_or(true)
            {
                npm_user_to_update.push(target.clone());
            }
        }
        for (target, _) in &npm_project_targets {
            let scope = SourceScope::Project;
            if self
                .should_update_npm_source(&target.parsed, scope)
                .await
                .unwrap_or(true)
            {
                npm_project_to_update.push(target.clone());
            }
        }

        let mut tasks = Vec::new();

        if !npm_user_to_update.is_empty() {
            let pm = self._make_self_ref();
            tasks.push(tokio::spawn(async move {
                pm.update_npm_batch(&npm_user_to_update, ScopeInstalled::User)
                    .await
            }));
        }
        if !npm_project_to_update.is_empty() {
            let pm = self._make_self_ref();
            tasks.push(tokio::spawn(async move {
                pm.update_npm_batch(&npm_project_to_update, ScopeInstalled::Project)
                    .await
            }));
        }
        for git_target in git_targets {
            let pm = self._make_self_ref();
            tasks.push(tokio::spawn(async move {
                pm.with_progress(
                    ProgressAction::Update,
                    &git_target.source,
                    &format!("Updating {}...", git_target.source),
                    async || {
                        pm.update_git(
                            &git_target.parsed,
                            match git_target.scope {
                                ScopeInstalled::User => SourceScope::User,
                                ScopeInstalled::Project => SourceScope::Project,
                            },
                        )
                        .await
                    },
                )
                .await
            }));
        }

        for task in tasks {
            task.await.unwrap_or(Ok(()))?;
        }

        Ok(())
    }

    // Workaround: create a reference-like clone for tokio::spawn
    fn _make_self_ref(&self) -> Arc<DefaultPackageManager> {
        Arc::new(DefaultPackageManager {
            cwd: self.cwd.clone(),
            agent_dir: self.agent_dir.clone(),
            settings_manager: self.settings_manager.clone(),
            global_npm_root: tokio::sync::Mutex::new(None),
            progress_callback: None,
        })
    }

    async fn update_npm_batch(
        &self,
        sources: &[NpmUpdateTarget],
        scope: ScopeInstalled,
    ) -> Result<(), String> {
        if sources.is_empty() {
            return Ok(());
        }

        let source_label = if sources.len() == 1 {
            sources[0].source.clone()
        } else {
            format!("{:?} npm packages", scope)
        };
        let message = if sources.len() == 1 {
            format!("Updating {}...", sources[0].source)
        } else {
            format!("Updating {:?} npm packages...", scope)
        };

        let specs: Vec<String> = sources
            .iter()
            .map(|entry| {
                if entry.parsed.version.is_some() {
                    entry.parsed.spec.clone()
                } else {
                    format!("{}@latest", entry.parsed.name)
                }
            })
            .collect();

        let source_scope = match scope {
            ScopeInstalled::User => SourceScope::User,
            ScopeInstalled::Project => SourceScope::Project,
        };

        self.with_progress(
            ProgressAction::Update,
            &source_label,
            &message,
            async || {
                let install_root = self.get_npm_install_root(source_scope, false);
                self.ensure_npm_project(&install_root);
                let args = self.get_npm_install_args(&specs, &install_root);
                self.run_npm_command(&args, None).await
            },
        )
        .await
    }

    // -----------------------------------------------------------------------
    // Accumulator methods
    // -----------------------------------------------------------------------

    fn create_accumulator(&self) -> ResourceAccumulator {
        ResourceAccumulator::default()
    }

    fn get_target_map<'a>(
        &self,
        accumulator: &'a mut ResourceAccumulator,
        resource_type: ResourceType,
    ) -> &'a mut HashMap<PathBuf, AccumulatorEntry> {
        match resource_type {
            ResourceType::Extensions => &mut accumulator.extensions,
            ResourceType::Skills => &mut accumulator.skills,
            ResourceType::Prompts => &mut accumulator.prompts,
            ResourceType::Themes => &mut accumulator.themes,
        }
    }

    fn get_target_map_ref<'a>(
        &self,
        accumulator: &'a ResourceAccumulator,
        resource_type: ResourceType,
    ) -> &'a HashMap<PathBuf, AccumulatorEntry> {
        match resource_type {
            ResourceType::Extensions => &accumulator.extensions,
            ResourceType::Skills => &accumulator.skills,
            ResourceType::Prompts => &accumulator.prompts,
            ResourceType::Themes => &accumulator.themes,
        }
    }

    fn add_resource(
        &self,
        map: &mut HashMap<PathBuf, AccumulatorEntry>,
        path: &Path,
        metadata: &PathMetadata,
        enabled: bool,
    ) {
        if !map.contains_key(path) {
            map.insert(
                path.to_path_buf(),
                AccumulatorEntry {
                    metadata: metadata.clone(),
                    enabled,
                },
            );
        }
    }

    fn to_resolved_paths(&self, accumulator: &ResourceAccumulator) -> ResolvedPaths {
        let map_fn = |entries: &HashMap<PathBuf, AccumulatorEntry>| -> Vec<ResolvedResource> {
            let mut resolved: Vec<ResolvedResource> = entries
                .iter()
                .map(|(path, entry)| ResolvedResource {
                    path: path.clone(),
                    enabled: entry.enabled,
                    metadata: entry.metadata.clone(),
                })
                .collect();

            resolved.sort_by(|a, b| {
                resource_precedence_rank(&a.metadata).cmp(&resource_precedence_rank(&b.metadata))
            });

            let mut seen = HashSet::new();
            resolved
                .into_iter()
                .filter(|entry| {
                    let canonical = paths::canonicalize_path(&entry.path.to_string_lossy());
                    if seen.contains(&canonical) {
                        false
                    } else {
                        seen.insert(canonical);
                        true
                    }
                })
                .collect()
        };

        ResolvedPaths {
            extensions: map_fn(&accumulator.extensions),
            skills: map_fn(&accumulator.skills),
            prompts: map_fn(&accumulator.prompts),
            themes: map_fn(&accumulator.themes),
        }
    }

    fn resolve_local_extension_source_internal(
        &self,
        source: &LocalSource,
        accumulator: &mut ResourceAccumulator,
        filter: Option<&PackageFilter>,
        metadata: &PathMetadata,
        base_dir: &Path,
    ) {
        let resolved = self.resolve_path_from_base(&source.path.to_string_lossy(), base_dir);
        if !resolved.exists() {
            return;
        }

        if resolved.is_file() {
            let mut meta = metadata.clone();
            meta.base_dir = resolved.parent().map(|p| p.to_path_buf());
            self.add_resource(&mut accumulator.extensions, &resolved, &meta, true);
            return;
        }
        if resolved.is_dir() {
            let mut meta = metadata.clone();
            meta.base_dir = Some(resolved.clone());
            let resources_found =
                self.collect_package_resources_internal(&resolved, accumulator, filter, &meta);
            if !resources_found {
                self.add_resource(&mut accumulator.extensions, &resolved, &meta, true);
            }
        }
    }

    fn collect_package_resources_internal(
        &self,
        package_root: &Path,
        accumulator: &mut ResourceAccumulator,
        filter: Option<&PackageFilter>,
        metadata: &PathMetadata,
    ) -> bool {
        if let Some(f) = filter {
            for resource_type in RESOURCE_TYPES {
                let patterns = match resource_type {
                    ResourceType::Extensions => f.extensions.as_ref(),
                    ResourceType::Skills => f.skills.as_ref(),
                    ResourceType::Prompts => f.prompts.as_ref(),
                    ResourceType::Themes => f.themes.as_ref(),
                };
                let target = self.get_target_map(accumulator, *resource_type);
                if let Some(p) = patterns {
                    self.apply_package_filter_internal(
                        package_root,
                        p,
                        *resource_type,
                        target,
                        metadata,
                    );
                } else {
                    self.collect_default_resources_internal(
                        package_root,
                        *resource_type,
                        target,
                        metadata,
                    );
                }
            }
            return true;
        }

        let manifest = self.read_pi_manifest_internal(package_root);
        if let Some(m) = manifest {
            for resource_type in RESOURCE_TYPES {
                let entries = match resource_type {
                    ResourceType::Extensions => m.extensions.as_ref(),
                    ResourceType::Skills => m.skills.as_ref(),
                    ResourceType::Prompts => m.prompts.as_ref(),
                    ResourceType::Themes => m.themes.as_ref(),
                };
                self.add_manifest_entries_internal(
                    entries,
                    package_root,
                    *resource_type,
                    self.get_target_map(accumulator, *resource_type),
                    metadata,
                );
            }
            return true;
        }

        let mut has_any_dir = false;
        for resource_type in RESOURCE_TYPES {
            let dir_name = match resource_type {
                ResourceType::Extensions => "extensions",
                ResourceType::Skills => "skills",
                ResourceType::Prompts => "prompts",
                ResourceType::Themes => "themes",
            };
            let dir = package_root.join(dir_name);
            if dir.exists() {
                let files = collect_resource_files(&dir, *resource_type);
                for f in &files {
                    self.add_resource(
                        self.get_target_map(accumulator, *resource_type),
                        f,
                        metadata,
                        true,
                    );
                }
                has_any_dir = true;
            }
        }
        has_any_dir
    }

    fn collect_default_resources_internal(
        &self,
        package_root: &Path,
        resource_type: ResourceType,
        target: &mut HashMap<PathBuf, AccumulatorEntry>,
        metadata: &PathMetadata,
    ) {
        let manifest = self.read_pi_manifest_internal(package_root);
        if let Some(ref m) = manifest {
            let entries = match resource_type {
                ResourceType::Extensions => m.extensions.as_ref(),
                ResourceType::Skills => m.skills.as_ref(),
                ResourceType::Prompts => m.prompts.as_ref(),
                ResourceType::Themes => m.themes.as_ref(),
            };
            if entries.is_some() {
                self.add_manifest_entries_internal(
                    entries,
                    package_root,
                    resource_type,
                    target,
                    metadata,
                );
                return;
            }
        }
        let dir_name = match resource_type {
            ResourceType::Extensions => "extensions",
            ResourceType::Skills => "skills",
            ResourceType::Prompts => "prompts",
            ResourceType::Themes => "themes",
        };
        let dir = package_root.join(dir_name);
        if dir.exists() {
            let files = collect_resource_files(&dir, resource_type);
            for f in &files {
                self.add_resource(target, &f, metadata, true);
            }
        }
    }

    fn apply_package_filter_internal(
        &self,
        package_root: &Path,
        user_patterns: &[String],
        resource_type: ResourceType,
        target: &mut HashMap<PathBuf, AccumulatorEntry>,
        metadata: &PathMetadata,
    ) {
        let (all_files, _) = self.collect_manifest_files_internal(package_root, resource_type);

        if user_patterns.is_empty() {
            for f in &all_files {
                self.add_resource(target, f, metadata, false);
            }
            return;
        }

        let enabled_by_user = apply_patterns(&all_files, user_patterns, package_root);
        for f in &all_files {
            let enabled = enabled_by_user.contains(f);
            self.add_resource(target, f, metadata, enabled);
        }
    }

    fn collect_manifest_files_internal(
        &self,
        package_root: &Path,
        resource_type: ResourceType,
    ) -> (Vec<PathBuf>, HashSet<PathBuf>) {
        let manifest = self.read_pi_manifest_internal(package_root);
        if let Some(ref m) = manifest {
            let entries = match resource_type {
                ResourceType::Extensions => m.extensions.as_ref(),
                ResourceType::Skills => m.skills.as_ref(),
                ResourceType::Prompts => m.prompts.as_ref(),
                ResourceType::Themes => m.themes.as_ref(),
            };
            if let Some(e) = entries {
                if !e.is_empty() {
                    let all_files = self.collect_files_from_manifest_entries_internal(
                        e,
                        package_root,
                        resource_type,
                    );
                    let manifest_patterns: Vec<String> = e
                        .iter()
                        .filter(|p| is_override_pattern(p))
                        .cloned()
                        .collect();
                    let enabled_by_manifest = if manifest_patterns.is_empty() {
                        all_files.iter().cloned().collect::<HashSet<_>>()
                    } else {
                        apply_patterns(&all_files, &manifest_patterns, package_root)
                    };
                    return (
                        enabled_by_manifest.iter().cloned().collect(),
                        enabled_by_manifest,
                    );
                }
            }
        }

        let dir_name = match resource_type {
            ResourceType::Extensions => "extensions",
            ResourceType::Skills => "skills",
            ResourceType::Prompts => "prompts",
            ResourceType::Themes => "themes",
        };
        let convention_dir = package_root.join(dir_name);
        if convention_dir.exists() {
            let all_files = collect_resource_files(&convention_dir, resource_type);
            let enabled: HashSet<PathBuf> = all_files.iter().cloned().collect();
            return (all_files, enabled);
        }
        (Vec::new(), HashSet::new())
    }

    fn read_pi_manifest_internal(&self, package_root: &Path) -> Option<PiManifest> {
        let package_json = package_root.join("package.json");
        if !package_json.exists() {
            return None;
        }
        let content = fs::read_to_string(&package_json).ok()?;
        let pkg: serde_json::Value = serde_json::from_str(&content).ok()?;
        let pi = pkg.get("pi")?;
        serde_json::from_value(pi.clone()).ok()
    }

    fn add_manifest_entries_internal(
        &self,
        entries: Option<&Vec<String>>,
        root: &Path,
        resource_type: ResourceType,
        target: &mut HashMap<PathBuf, AccumulatorEntry>,
        metadata: &PathMetadata,
    ) {
        let entries = match entries {
            Some(e) => e,
            None => return,
        };

        let all_files =
            self.collect_files_from_manifest_entries_internal(entries, root, resource_type);
        let patterns: Vec<String> = entries
            .iter()
            .filter(|p| is_override_pattern(p))
            .cloned()
            .collect();
        let enabled_paths = apply_patterns(&all_files, &patterns, root);

        for f in &all_files {
            if enabled_paths.contains(f) {
                self.add_resource(target, f, metadata, true);
            }
        }
    }

    fn collect_files_from_manifest_entries_internal(
        &self,
        entries: &[String],
        root: &Path,
        resource_type: ResourceType,
    ) -> Vec<PathBuf> {
        let source_entries: Vec<&String> =
            entries.iter().filter(|e| !is_override_pattern(e)).collect();
        let mut resolved: Vec<PathBuf> = Vec::new();

        for entry in source_entries {
            if !has_glob_pattern(entry) {
                let p = root.join(entry);
                resolved.push(p);
            } else {
                // Use glob crate
                let pattern = root.join(entry).to_string_lossy().to_string();
                if let Ok(entries) = glob::glob(&pattern) {
                    for match_result in entries.flatten() {
                        resolved.push(match_result);
                    }
                }
            }
        }

        self.collect_files_from_paths_internal(&resolved, resource_type)
    }

    fn collect_files_from_paths_internal(
        &self,
        paths: &[PathBuf],
        resource_type: ResourceType,
    ) -> Vec<PathBuf> {
        let mut files = Vec::new();
        for p in paths {
            if !p.exists() {
                continue;
            }
            if p.is_file() {
                files.push(p.clone());
            } else if p.is_dir() {
                files.extend(collect_resource_files(p, resource_type));
            }
        }
        files
    }
}

// ---------------------------------------------------------------------------
// Structs for internal use
// ---------------------------------------------------------------------------

struct GitUpdateTargetInfo {
    r#ref: String,
    head: String,
    fetch_args: Vec<String>,
}

// ---------------------------------------------------------------------------
// Git URL parsing helpers (port of git.ts)
// ---------------------------------------------------------------------------

fn split_git_ref(url: &str) -> (String, Option<String>) {
    // SCP-like: git@github.com:user/repo@v1
    let scp_like = regex::Regex::new(r"^git@([^:]+):(.+)$").unwrap();
    if let Some(caps) = scp_like.captures(url) {
        let path_with_ref = caps.get(2).unwrap().as_str();
        if let Some(ref_sep) = path_with_ref.rfind('@') {
            let repo_path = &path_with_ref[..ref_sep];
            let r#ref = &path_with_ref[ref_sep + 1..];
            if !repo_path.is_empty() && !r#ref.is_empty() {
                return (
                    format!("git@{}:{}", caps.get(1).unwrap().as_str(), repo_path),
                    Some(r#ref.to_string()),
                );
            }
        }
        return (url.to_string(), None);
    }

    // Protocol URLs
    if url.contains("://") {
        if let Ok(parsed) = url::Url::parse(url) {
            let path = parsed.path().trim_start_matches('/');
            if let Some(ref_sep) = path.rfind('@') {
                let repo_path = &path[..ref_sep];
                let r#ref = &path[ref_sep + 1..];
                if !repo_path.is_empty() && !r#ref.is_empty() {
                    let mut new_url = parsed.clone();
                    new_url.set_path(&format!("/{}", repo_path));
                    return (
                        new_url.as_str().trim_end_matches('/').to_string(),
                        Some(r#ref.to_string()),
                    );
                }
            }
        }
        return (url.to_string(), None);
    }

    // host/path@ref shorthand
    let slash_index = url.find('/');
    if let Some(si) = slash_index {
        let path_with_ref = &url[si + 1..];
        if let Some(ref_sep) = path_with_ref.rfind('@') {
            let repo_path = &path_with_ref[..ref_sep];
            let r#ref = &path_with_ref[ref_sep + 1..];
            if !repo_path.is_empty() && !r#ref.is_empty() {
                let host = &url[..si];
                return (format!("{}/{}", host, repo_path), Some(r#ref.to_string()));
            }
        }
    }

    (url.to_string(), None)
}

fn has_unsafe_git_install_part(value: &str, allow_slash: bool) -> bool {
    // Check for URL-encoded unsafe chars
    let decoded = urlencoding::decode(value).ok();
    let candidates = match decoded {
        Some(ref d) => vec![value.to_string(), d.to_string()],
        None => vec![value.to_string()],
    };
    for candidate in &candidates {
        if candidate.contains('\0') || candidate.contains('\\') || candidate.starts_with('/') {
            return true;
        }
        if !allow_slash && candidate.contains('/') {
            return true;
        }
        if candidate.split('/').any(|part| part == "..") {
            return true;
        }
    }
    false
}

fn build_git_source(
    repo: &str,
    host: &str,
    path: &str,
    r#ref: Option<String>,
) -> Option<GitSource> {
    if path.starts_with('/') {
        return None;
    }
    let normalized_path = path
        .trim_end_matches(".git")
        .trim_start_matches('/')
        .to_string();
    if host.is_empty() || normalized_path.is_empty() || normalized_path.split('/').count() < 2 {
        return None;
    }
    if has_unsafe_git_install_part(host, false)
        || has_unsafe_git_install_part(&normalized_path, true)
    {
        return None;
    }
    Some(GitSource {
        repo: repo.to_string(),
        host: host.to_string(),
        path: normalized_path,
        pinned: r#ref.is_some(),
        r#ref,
    })
}

fn parse_hosted_git_info(url: &str) -> Option<(String, String, Option<String>)> {
    // Use hosted-git-info-like pattern matching for well-known hosts
    let hosted_re = regex::Regex::new(
        r"^(github|gitlab|bitbucket|codeberg)\.com[/:#]([^/]+)/([^/#@]+)(?:[/#@](.+))?",
    )
    .unwrap();

    if let Some(caps) = hosted_re.captures(url) {
        let host = caps.get(1).unwrap().as_str().to_string();
        let user = caps.get(2).unwrap().as_str().to_string();
        let project = caps.get(3).unwrap().as_str().to_string();
        let project_clean = project.trim_end_matches(".git").to_string();
        let r#ref = caps
            .get(4)
            .map(|m| {
                let r = m.as_str();
                // Strip leading # or /
                if r.starts_with('#') || r.starts_with('/') {
                    &r[1..]
                } else {
                    r
                }
            })
            .filter(|r| !r.is_empty());
        let path = format!("{}/{}", user, project_clean);
        return Some((format!("{}.com", host), path, r#ref.map(|s| s.to_string())));
    }

    None
}

fn parse_generic_git_url(url: &str, r#ref: Option<String>) -> Option<GitSource> {
    let mut repo = url.to_string();
    let host: String;
    let path: String;

    // SCP-like
    let scp_like = regex::Regex::new(r"^git@([^:]+):(.+)$").unwrap();
    if let Some(caps) = scp_like.captures(url) {
        host = caps.get(1).unwrap().as_str().to_string();
        path = caps.get(2).unwrap().as_str().to_string();
    } else if url.starts_with("https://")
        || url.starts_with("http://")
        || url.starts_with("ssh://")
        || url.starts_with("git://")
    {
        if let Ok(parsed) = url::Url::parse(url) {
            host = parsed.host_str().unwrap_or("").to_string();
            path = parsed.path().trim_start_matches('/').to_string();
        } else {
            return None;
        }
    } else {
        let slash_index = url.find('/')?;
        host = url[..slash_index].to_string();
        path = url[slash_index + 1..].to_string();
        if !host.contains('.') && host != "localhost" {
            return None;
        }
        repo = format!("https://{}", url);
    }

    build_git_source(&repo, &host, &path, r#ref)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_npm_spec_simple() {
        let (name, version) = DefaultPackageManager::parse_npm_spec("lodash");
        assert_eq!(name, "lodash");
        assert!(version.is_none());
    }

    #[test]
    fn test_parse_npm_spec_with_version() {
        let (name, version) = DefaultPackageManager::parse_npm_spec("lodash@4.17.21");
        assert_eq!(name, "lodash");
        assert_eq!(version, Some("4.17.21".to_string()));
    }

    #[test]
    fn test_parse_npm_spec_scoped() {
        let (name, version) = DefaultPackageManager::parse_npm_spec("@scope/package@1.0.0");
        assert_eq!(name, "@scope/package");
        assert_eq!(version, Some("1.0.0".to_string()));
    }

    #[test]
    fn test_is_exact_npm_version() {
        assert!(DefaultPackageManager::is_exact_npm_version("1.0.0"));
        assert!(DefaultPackageManager::is_exact_npm_version("4.17.21"));
        assert!(!DefaultPackageManager::is_exact_npm_version("^1.0.0"));
        assert!(!DefaultPackageManager::is_exact_npm_version("latest"));
    }

    #[test]
    fn test_is_local_path() {
        assert!(DefaultPackageManager::is_local_path("./foo"));
        assert!(DefaultPackageManager::is_local_path("../bar"));
        assert!(DefaultPackageManager::is_local_path("/absolute/path"));
        assert!(DefaultPackageManager::is_local_path("~/home/path"));
        assert!(!DefaultPackageManager::is_local_path("lodash"));
        assert!(!DefaultPackageManager::is_local_path("npm:lodash"));
    }

    #[test]
    fn test_parse_git_https_url() {
        let parsed = crate::core::package_manager::DefaultPackageManager::parse_git_url(
            "https://github.com/user/repo",
        )
        .unwrap();
        assert_eq!(parsed.host, "github.com");
        assert_eq!(parsed.path, "user/repo");
    }

    #[test]
    fn test_parse_git_ssh_url() {
        let parsed = crate::core::package_manager::DefaultPackageManager::parse_git_url(
            "ssh://git@github.com/user/repo",
        )
        .unwrap();
        assert_eq!(parsed.host, "github.com");
        assert_eq!(parsed.path, "user/repo");
    }

    #[test]
    fn test_parse_git_shorthand_with_git_prefix() {
        let parsed = crate::core::package_manager::DefaultPackageManager::parse_git_url(
            "git:github.com/user/repo",
        )
        .unwrap();
        assert_eq!(parsed.host, "github.com");
        assert_eq!(parsed.path, "user/repo");
    }

    #[test]
    fn test_parse_git_ssh_shorthand_with_git_prefix() {
        let parsed = crate::core::package_manager::DefaultPackageManager::parse_git_url(
            "git:git@github.com:user/repo",
        )
        .unwrap();
        assert_eq!(parsed.host, "github.com");
        assert_eq!(parsed.path, "user/repo");
    }

    #[test]
    fn test_parse_git_with_ref() {
        let parsed = crate::core::package_manager::DefaultPackageManager::parse_git_url(
            "https://github.com/user/repo@v1.0.0",
        )
        .unwrap();
        assert_eq!(parsed.r#ref, Some("v1.0.0".to_string()));
        assert!(parsed.pinned);
    }

    #[test]
    fn test_parse_git_shorthand_with_ref() {
        let parsed = crate::core::package_manager::DefaultPackageManager::parse_git_url(
            "git:github.com/user/repo@v2",
        )
        .unwrap();
        assert_eq!(parsed.r#ref, Some("v2".to_string()));
        assert!(parsed.pinned);
    }

    #[test]
    fn test_parse_ssh_shorthand_without_git_prefix_is_none() {
        let parsed = crate::core::package_manager::DefaultPackageManager::parse_git_url(
            "git@github.com:user/repo",
        );
        assert!(parsed.is_none());
    }

    #[test]
    fn test_parse_host_path_without_git_prefix_is_none() {
        let parsed = crate::core::package_manager::DefaultPackageManager::parse_git_url(
            "github.com/user/repo",
        );
        assert!(parsed.is_none());
    }

    #[test]
    fn test_parse_git_trailing_dot_git() {
        let parsed = crate::core::package_manager::DefaultPackageManager::parse_git_url(
            "https://github.com/user/repo.git",
        )
        .unwrap();
        assert_eq!(parsed.host, "github.com");
        assert_eq!(parsed.path, "user/repo");
    }

    #[test]
    fn test_dot_relative_paths_yield_none_from_parse_git_url() {
        let parsed = crate::core::package_manager::DefaultPackageManager::parse_git_url(
            "./packages/agent-timers",
        );
        assert!(parsed.is_none());
    }

    #[test]
    fn test_dot_dot_relative_paths_yield_none_from_parse_git_url() {
        let parsed = crate::core::package_manager::DefaultPackageManager::parse_git_url(
            "../packages/agent-timers",
        );
        assert!(parsed.is_none());
    }
}
