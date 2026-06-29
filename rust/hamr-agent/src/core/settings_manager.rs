//! Settings manager — persistent configuration with file locking, deep merge,
//! migration, and scoped (global/project) writes.
//!
//! Port of `packages/coding-agent/src/core/settings-manager.ts`.

use std::collections::{HashMap, HashSet};
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::os::unix::io::AsRawFd;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde::{Deserialize, Serialize};

use crate::utils::paths::{normalize_path, resolve_path};

// ---------------------------------------------------------------------------
// Defaults
// ---------------------------------------------------------------------------

/// Default HTTP idle timeout (10 minutes).
const DEFAULT_HTTP_IDLE_TIMEOUT_MS: u64 = 600_000;

/// Name of the config directory inside a project.
const CONFIG_DIR_NAME: &str = ".hamr";

// ---------------------------------------------------------------------------
// Path utilities
// ---------------------------------------------------------------------------

fn resolve(input: &str) -> String {
    resolve_path(input, None, &Default::default())
}

/// Return the agent directory path (mirrors `getAgentDir()` in TS).
pub fn get_agent_dir() -> String {
    if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
        PathBuf::from(xdg)
            .join("hamr")
            .to_string_lossy()
            .into_owned()
    } else {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
        PathBuf::from(home)
            .join(".config")
            .join("hamr")
            .to_string_lossy()
            .into_owned()
    }
}

/// Parse and validate a timeout setting value. Mirrors TS `parseTimeoutSetting`.
fn parse_timeout_setting(value: Option<u64>, _setting_name: &str) -> Result<Option<u64>, String> {
    match value {
        Some(ms) => {
            // TS uses parseHttpIdleTimeoutMs which validates numeric values.
            // For Rust, u64 is already numeric; we accept any u64 >= 0.
            // The main validation is type-level, but we keep the check for explicit clarity.
            Ok(Some(ms))
        }
        None => Ok(None),
    }
}

// ---------------------------------------------------------------------------
// Nested settings structs
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CompactionSettings {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reserve_tokens: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keep_recent_tokens: Option<u64>,
}

impl Default for CompactionSettings {
    fn default() -> Self {
        Self {
            enabled: Some(true),
            reserve_tokens: Some(16384),
            keep_recent_tokens: Some(20000),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BranchSummarySettings {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reserve_tokens: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skip_prompt: Option<bool>,
}

impl Default for BranchSummarySettings {
    fn default() -> Self {
        Self {
            reserve_tokens: Some(16384),
            skip_prompt: Some(false),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ProviderRetrySettings {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_retries: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_retry_delay_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RetrySettings {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_retries: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_delay_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<ProviderRetrySettings>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TerminalSettings {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub show_images: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_width_cells: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub clear_on_shrink: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub show_terminal_progress: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ImageSettings {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_resize: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub block_images: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ThinkingBudgetsSettings {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub minimal: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub low: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub medium: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub high: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MarkdownSettings {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code_block_indent: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WarningSettings {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub anthropic_extra_usage: Option<bool>,
}

impl Default for WarningSettings {
    fn default() -> Self {
        Self {
            anthropic_extra_usage: Some(true),
        }
    }
}

/// Default project trust policy.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum DefaultProjectTrust {
    Ask,
    Always,
    Never,
}

/// Transport protocol for provider streaming.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum TransportSetting {
    Auto,
    Sse,
    #[serde(rename = "websocket")]
    WebSocket,
    #[serde(rename = "websocket-cached")]
    WebSocketCached,
}

impl Default for TransportSetting {
    fn default() -> Self {
        Self::Auto
    }
}

/// Package source for npm/git packages.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum PackageSource {
    String(String),
    Object {
        source: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        extensions: Option<Vec<String>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        skills: Option<Vec<String>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        prompts: Option<Vec<String>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        themes: Option<Vec<String>>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ThinkingLevel {
    Off,
    Minimal,
    Low,
    Medium,
    High,
    Xhigh,
}

// ---------------------------------------------------------------------------
// Settings struct
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_changelog_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_provider: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_thinking_level: Option<ThinkingLevel>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transport: Option<TransportSetting>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub steering_mode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub follow_up_mode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub theme: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compaction: Option<CompactionSettings>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch_summary: Option<BranchSummarySettings>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry: Option<RetrySettings>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hide_thinking_block: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shell_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quiet_startup: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_project_trust: Option<DefaultProjectTrust>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shell_command_prefix: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub npm_command: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub collapse_changelog: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable_install_telemetry: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable_analytics: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tracking_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub packages: Option<Vec<PackageSource>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extensions: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skills: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompts: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub themes: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable_skill_commands: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub terminal: Option<TerminalSettings>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub images: Option<ImageSettings>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled_models: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub double_escape_action: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tree_filter_mode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking_budgets: Option<ThinkingBudgetsSettings>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub editor_padding_x: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub autocomplete_max_visible: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub show_hardware_cursor: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub markdown: Option<MarkdownSettings>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub warnings: Option<WarningSettings>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_dir: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub http_proxy: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub http_idle_timeout_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub websocket_connect_timeout_ms: Option<u64>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            last_changelog_version: None,
            default_provider: None,
            default_model: None,
            default_thinking_level: None,
            transport: None,
            steering_mode: None,
            follow_up_mode: None,
            theme: None,
            compaction: None,
            branch_summary: None,
            retry: None,
            hide_thinking_block: None,
            shell_path: None,
            quiet_startup: None,
            default_project_trust: None,
            shell_command_prefix: None,
            npm_command: None,
            collapse_changelog: None,
            enable_install_telemetry: None,
            enable_analytics: None,
            tracking_id: None,
            packages: None,
            extensions: None,
            skills: None,
            prompts: None,
            themes: None,
            enable_skill_commands: None,
            terminal: None,
            images: None,
            enabled_models: None,
            double_escape_action: None,
            tree_filter_mode: None,
            thinking_budgets: None,
            editor_padding_x: None,
            autocomplete_max_visible: None,
            show_hardware_cursor: None,
            markdown: None,
            warnings: None,
            session_dir: None,
            http_proxy: None,
            http_idle_timeout_ms: None,
            websocket_connect_timeout_ms: None,
        }
    }
}

// ---------------------------------------------------------------------------
// Deep merge
// ---------------------------------------------------------------------------

fn merge_nested_object<T: Clone>(
    base: &Option<T>,
    overrides: &Option<T>,
    merge_fn: impl FnOnce(T, T) -> T,
) -> Option<T> {
    match (base, overrides) {
        (Some(b), Some(o)) => Some(merge_fn(b.clone(), o.clone())),
        (Some(b), None) => Some(b.clone()),
        (None, Some(o)) => Some(o.clone()),
        (None, None) => None,
    }
}

fn merge_field<T: Clone>(base: &Option<T>, overrides: &Option<T>) -> Option<T> {
    overrides.clone().or_else(|| base.clone())
}

/// Deep merge `overrides` into `base`. Known nested structs are merged
/// recursively; all other fields use simple override semantics.
pub fn deep_merge_settings(base: &Settings, overrides: &Settings) -> Settings {
    Settings {
        last_changelog_version: merge_field(
            &base.last_changelog_version,
            &overrides.last_changelog_version,
        ),
        default_provider: merge_field(&base.default_provider, &overrides.default_provider),
        default_model: merge_field(&base.default_model, &overrides.default_model),
        default_thinking_level: merge_field(
            &base.default_thinking_level,
            &overrides.default_thinking_level,
        ),
        transport: merge_field(&base.transport, &overrides.transport),
        steering_mode: merge_field(&base.steering_mode, &overrides.steering_mode),
        follow_up_mode: merge_field(&base.follow_up_mode, &overrides.follow_up_mode),
        theme: merge_field(&base.theme, &overrides.theme),
        compaction: merge_nested_object(&base.compaction, &overrides.compaction, |b, o| {
            CompactionSettings {
                enabled: o.enabled.or(b.enabled),
                reserve_tokens: o.reserve_tokens.or(b.reserve_tokens),
                keep_recent_tokens: o.keep_recent_tokens.or(b.keep_recent_tokens),
            }
        }),
        branch_summary: merge_nested_object(
            &base.branch_summary,
            &overrides.branch_summary,
            |b, o| BranchSummarySettings {
                reserve_tokens: o.reserve_tokens.or(b.reserve_tokens),
                skip_prompt: o.skip_prompt.or(b.skip_prompt),
            },
        ),
        retry: merge_nested_object(&base.retry, &overrides.retry, |b, o| RetrySettings {
            enabled: o.enabled.or(b.enabled),
            max_retries: o.max_retries.or(b.max_retries),
            base_delay_ms: o.base_delay_ms.or(b.base_delay_ms),
            provider: merge_nested_object(&b.provider, &o.provider, |bp, op| {
                ProviderRetrySettings {
                    timeout_ms: op.timeout_ms.or(bp.timeout_ms),
                    max_retries: op.max_retries.or(bp.max_retries),
                    max_retry_delay_ms: op.max_retry_delay_ms.or(bp.max_retry_delay_ms),
                }
            }),
        }),
        hide_thinking_block: merge_field(&base.hide_thinking_block, &overrides.hide_thinking_block),
        shell_path: merge_field(&base.shell_path, &overrides.shell_path),
        quiet_startup: merge_field(&base.quiet_startup, &overrides.quiet_startup),
        default_project_trust: merge_field(
            &base.default_project_trust,
            &overrides.default_project_trust,
        ),
        shell_command_prefix: merge_field(
            &base.shell_command_prefix,
            &overrides.shell_command_prefix,
        ),
        npm_command: merge_field(&base.npm_command, &overrides.npm_command),
        collapse_changelog: merge_field(&base.collapse_changelog, &overrides.collapse_changelog),
        enable_install_telemetry: merge_field(
            &base.enable_install_telemetry,
            &overrides.enable_install_telemetry,
        ),
        enable_analytics: merge_field(&base.enable_analytics, &overrides.enable_analytics),
        tracking_id: merge_field(&base.tracking_id, &overrides.tracking_id),
        packages: merge_field(&base.packages, &overrides.packages),
        extensions: merge_field(&base.extensions, &overrides.extensions),
        skills: merge_field(&base.skills, &overrides.skills),
        prompts: merge_field(&base.prompts, &overrides.prompts),
        themes: merge_field(&base.themes, &overrides.themes),
        enable_skill_commands: merge_field(
            &base.enable_skill_commands,
            &overrides.enable_skill_commands,
        ),
        terminal: merge_nested_object(&base.terminal, &overrides.terminal, |b, o| {
            TerminalSettings {
                show_images: o.show_images.or(b.show_images),
                image_width_cells: o.image_width_cells.or(b.image_width_cells),
                clear_on_shrink: o.clear_on_shrink.or(b.clear_on_shrink),
                show_terminal_progress: o.show_terminal_progress.or(b.show_terminal_progress),
            }
        }),
        images: merge_nested_object(&base.images, &overrides.images, |b, o| ImageSettings {
            auto_resize: o.auto_resize.or(b.auto_resize),
            block_images: o.block_images.or(b.block_images),
        }),
        enabled_models: merge_field(&base.enabled_models, &overrides.enabled_models),
        double_escape_action: merge_field(
            &base.double_escape_action,
            &overrides.double_escape_action,
        ),
        tree_filter_mode: merge_field(&base.tree_filter_mode, &overrides.tree_filter_mode),
        thinking_budgets: merge_nested_object(
            &base.thinking_budgets,
            &overrides.thinking_budgets,
            |b, o| ThinkingBudgetsSettings {
                minimal: o.minimal.or(b.minimal),
                low: o.low.or(b.low),
                medium: o.medium.or(b.medium),
                high: o.high.or(b.high),
            },
        ),
        editor_padding_x: merge_field(&base.editor_padding_x, &overrides.editor_padding_x),
        autocomplete_max_visible: merge_field(
            &base.autocomplete_max_visible,
            &overrides.autocomplete_max_visible,
        ),
        show_hardware_cursor: merge_field(
            &base.show_hardware_cursor,
            &overrides.show_hardware_cursor,
        ),
        markdown: merge_nested_object(&base.markdown, &overrides.markdown, |b, o| {
            MarkdownSettings {
                code_block_indent: o
                    .code_block_indent
                    .clone()
                    .or_else(|| b.code_block_indent.clone()),
            }
        }),
        warnings: merge_nested_object(&base.warnings, &overrides.warnings, |b, o| {
            WarningSettings {
                anthropic_extra_usage: o.anthropic_extra_usage.or(b.anthropic_extra_usage),
            }
        }),
        session_dir: merge_field(&base.session_dir, &overrides.session_dir),
        http_proxy: merge_field(&base.http_proxy, &overrides.http_proxy),
        http_idle_timeout_ms: merge_field(
            &base.http_idle_timeout_ms,
            &overrides.http_idle_timeout_ms,
        ),
        websocket_connect_timeout_ms: merge_field(
            &base.websocket_connect_timeout_ms,
            &overrides.websocket_connect_timeout_ms,
        ),
    }
}

// ---------------------------------------------------------------------------
// Settings scope
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsScope {
    Global,
    Project,
}

// ---------------------------------------------------------------------------
// Error types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct SettingsError {
    pub scope: SettingsScope,
    pub error: String,
}

impl SettingsError {
    fn new(scope: SettingsScope, error: impl Into<String>) -> Self {
        Self {
            scope,
            error: error.into(),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SettingsManagerError {
    #[error("Project is not trusted; refusing to write project settings")]
    ProjectNotTrusted,
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("{0}")]
    Other(String),
}

// ---------------------------------------------------------------------------
// File locking (Unix flock with exponential-backoff retry)
// ---------------------------------------------------------------------------

fn acquire_flock(file: &File) -> Result<(), std::io::Error> {
    const MAX_ATTEMPTS: u32 = 25;
    const BASE_DELAY_MS: u64 = 20;
    const MAX_DELAY_MS: u64 = 2000;
    for attempt in 1..=MAX_ATTEMPTS {
        let ret = unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) };
        if ret == 0 {
            return Ok(());
        }
        let err = std::io::Error::last_os_error();
        if err.kind() != std::io::ErrorKind::WouldBlock || attempt == MAX_ATTEMPTS {
            return Err(err);
        }
        let delay_ms = std::cmp::min(
            BASE_DELAY_MS.saturating_mul(1_u64 << (attempt - 1)),
            MAX_DELAY_MS,
        );
        std::thread::sleep(std::time::Duration::from_millis(delay_ms));
    }
    Err(std::io::Error::new(
        std::io::ErrorKind::WouldBlock,
        "failed to acquire settings lock after retries",
    ))
}

fn release_flock(fd: std::os::unix::io::RawFd) {
    unsafe { libc::flock(fd, libc::LOCK_UN) };
}

// ---------------------------------------------------------------------------
// SettingsStorage trait
// ---------------------------------------------------------------------------

pub trait SettingsStorage: Send + Sync {
    fn with_lock(&self, scope: SettingsScope, f: &mut dyn FnMut(Option<&str>) -> Option<String>);
}

// ---------------------------------------------------------------------------
// FileSettingsStorage
// ---------------------------------------------------------------------------

pub struct FileSettingsStorage {
    global_settings_path: PathBuf,
    project_settings_path: PathBuf,
}

impl FileSettingsStorage {
    pub fn new(cwd: &str, agent_dir: &str) -> Self {
        let resolved_cwd = resolve(cwd);
        let resolved_agent_dir = resolve(agent_dir);
        Self {
            global_settings_path: PathBuf::from(&resolved_agent_dir).join("settings.json"),
            project_settings_path: PathBuf::from(&resolved_cwd)
                .join(CONFIG_DIR_NAME)
                .join("settings.json"),
        }
    }

    fn path_for_scope(&self, scope: SettingsScope) -> &Path {
        match scope {
            SettingsScope::Global => &self.global_settings_path,
            SettingsScope::Project => &self.project_settings_path,
        }
    }

    fn read_file(path: &Path) -> io::Result<String> {
        let mut buf = String::new();
        let mut f = File::open(path)?;
        f.read_to_string(&mut buf)?;
        Ok(buf)
    }

    fn write_file_atomic(path: &Path, content: &str) -> io::Result<()> {
        let dir = path.parent().unwrap_or(Path::new("."));
        let tmp = tempfile::NamedTempFile::new_in(dir)?;
        let mut f = tmp.as_file();
        f.write_all(content.as_bytes())?;
        f.flush()?;
        tmp.persist(path)?;
        Ok(())
    }
}

impl SettingsStorage for FileSettingsStorage {
    fn with_lock(&self, scope: SettingsScope, f: &mut dyn FnMut(Option<&str>) -> Option<String>) {
        let path = self.path_for_scope(scope);
        let dir = path.parent().unwrap_or(Path::new("."));

        let file_exists = path.exists();

        // Only acquire lock and read if the file already exists.
        // This mirrors TS behavior: lock is only acquired when the file exists,
        // or when we need to write.
        let mut locked_file: Option<File> = None;
        let current: Option<String> = if file_exists {
            match fs::OpenOptions::new().read(true).write(true).open(path) {
                Ok(file) => {
                    if acquire_flock(&file).is_ok() {
                        let cur = Self::read_file(path).ok();
                        locked_file = Some(file);
                        cur
                    } else {
                        // Lock failed; read without lock as fallback
                        Self::read_file(path).ok()
                    }
                }
                Err(_) => None,
            }
        } else {
            None
        };

        let next = f(current.as_deref());

        if let Some(content) = next {
            // Only create directory when we actually need to write
            if !dir.exists() {
                let _ = fs::create_dir_all(dir);
            }

            // If we don't already hold a lock (file didn't exist), acquire one now
            if locked_file.is_none() {
                let _ = fs::create_dir_all(dir);
                match fs::OpenOptions::new()
                    .read(true)
                    .write(true)
                    .create(true)
                    .open(path)
                {
                    Ok(file) => {
                        let _ = acquire_flock(&file);
                        locked_file = Some(file);
                    }
                    Err(_) => {
                        // Cannot lock; write anyway
                        let _ = Self::write_file_atomic(path, &content);
                        return;
                    }
                }
            }

            let _ = Self::write_file_atomic(path, &content);
        }

        if let Some(file) = locked_file {
            release_flock(file.as_raw_fd());
        }
    }
}

// ---------------------------------------------------------------------------
// InMemorySettingsStorage
// ---------------------------------------------------------------------------

pub struct InMemorySettingsStorage {
    global: Mutex<Option<String>>,
    project: Mutex<Option<String>>,
}

impl InMemorySettingsStorage {
    pub fn new() -> Self {
        Self {
            global: Mutex::new(None),
            project: Mutex::new(None),
        }
    }

    #[allow(dead_code)]
    pub fn new_with_global(content: &str) -> Self {
        Self {
            global: Mutex::new(Some(content.to_string())),
            project: Mutex::new(None),
        }
    }
}

impl Default for InMemorySettingsStorage {
    fn default() -> Self {
        Self::new()
    }
}

impl SettingsStorage for InMemorySettingsStorage {
    fn with_lock(&self, scope: SettingsScope, f: &mut dyn FnMut(Option<&str>) -> Option<String>) {
        let mutex = match scope {
            SettingsScope::Global => &self.global,
            SettingsScope::Project => &self.project,
        };
        let mut guard = mutex.lock().unwrap();
        let current: Option<&str> = guard.as_deref();
        let next = f(current);
        if let Some(content) = next {
            *guard = Some(content);
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn generate_uuid_v4() -> String {
    uuid::Uuid::new_v4().to_string()
}

// ---------------------------------------------------------------------------
// SettingsManagerCreateOptions
// ---------------------------------------------------------------------------

pub struct SettingsManagerCreateOptions {
    pub project_trusted: Option<bool>,
    pub default_packages: Option<Vec<PackageSource>>,
}

impl Default for SettingsManagerCreateOptions {
    fn default() -> Self {
        Self {
            project_trusted: Some(true),
            default_packages: None,
        }
    }
}

// ---------------------------------------------------------------------------
// Migration
// ---------------------------------------------------------------------------

/// Migrate settings from legacy formats.
fn migrate_settings(raw: &mut serde_json::Value) {
    // queueMode -> steeringMode
    if let Some(val) = raw.get("queueMode") {
        if !raw.get("steeringMode").and_then(|v| v.as_str()).is_some() {
            raw["steeringMode"] = val.clone();
        }
        raw.as_object_mut().map(|m| m.remove("queueMode"));
    }

    // websockets boolean -> transport
    if raw.get("transport").is_none() {
        if let Some(ws) = raw.get("websockets").and_then(|v| v.as_bool()) {
            raw["transport"] =
                serde_json::Value::String(if ws { "websocket".into() } else { "sse".into() });
            raw.as_object_mut().map(|m| m.remove("websockets"));
        }
    }

    // skills object -> skills array + enableSkillCommands
    // Extract values before any mutation to avoid borrow conflicts.
    let skills_migration = if let Some(skills_val) = raw.get("skills") {
        if skills_val.is_object() {
            let obj = skills_val.as_object().unwrap();
            let enable_cmd = obj
                .get("enableSkillCommands")
                .filter(|_| raw.get("enableSkillCommands").is_none())
                .cloned();
            let dirs = obj
                .get("customDirectories")
                .and_then(|v| v.as_array())
                .cloned();
            Some((enable_cmd, dirs))
        } else {
            None
        }
    } else {
        None
    };

    if let Some((enable_cmd, dirs)) = skills_migration {
        if let Some(enabled) = enable_cmd {
            raw["enableSkillCommands"] = enabled;
        }
        if let Some(ref dirs) = dirs {
            if !dirs.is_empty() {
                raw["skills"] = serde_json::Value::Array(dirs.clone());
            } else {
                raw.as_object_mut().and_then(|m| m.remove("skills"));
            }
        } else {
            raw.as_object_mut().and_then(|m| m.remove("skills"));
        }
    }

    // retry.maxDelayMs -> retry.provider.maxRetryDelayMs
    if let Some(retry) = raw.get_mut("retry").and_then(|v| v.as_object_mut()) {
        if let Some(max_delay) = retry.remove("maxDelayMs").and_then(|v| v.as_f64()) {
            let provider = retry
                .entry("provider".to_string())
                .or_insert_with(|| serde_json::Value::Object(Default::default()));
            if let Some(prov_obj) = provider.as_object_mut() {
                if !prov_obj.contains_key("maxRetryDelayMs") {
                    prov_obj.insert(
                        "maxRetryDelayMs".to_string(),
                        serde_json::Value::Number(
                            serde_json::Number::from_f64(max_delay)
                                .unwrap_or(serde_json::Number::from(60000)),
                        ),
                    );
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// SettingsManager
// ---------------------------------------------------------------------------

pub struct SettingsManager {
    storage: Box<dyn SettingsStorage>,
    global_settings: Settings,
    project_settings: Settings,
    /// Merged view of global + project settings.
    settings: Settings,
    project_trusted: bool,

    modified_fields: HashSet<String>,
    modified_nested_fields: HashMap<String, HashSet<String>>,
    modified_project_fields: HashSet<String>,
    modified_project_nested_fields: HashMap<String, HashSet<String>>,

    global_settings_load_error: Option<String>,
    project_settings_load_error: Option<String>,

    errors: Vec<SettingsError>,

    default_packages: Vec<PackageSource>,
}

// Manual trait implementations needed because Box<dyn SettingsStorage>
// does not automatically derive Debug or Clone.

impl std::fmt::Debug for SettingsManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SettingsManager")
            .field("global_settings", &self.global_settings)
            .field("project_settings", &self.project_settings)
            .field("settings", &self.settings)
            .field("project_trusted", &self.project_trusted)
            .finish_non_exhaustive()
    }
}

impl Clone for SettingsManager {
    fn clone(&self) -> Self {
        // Clone everything except storage (which can't be cloned).
        // This is only used for transient copies; storage is re-created on demand.
        Self {
            storage: Box::new(InMemorySettingsStorage::new()),
            global_settings: self.global_settings.clone(),
            project_settings: self.project_settings.clone(),
            settings: self.settings.clone(),
            project_trusted: self.project_trusted,
            modified_fields: self.modified_fields.clone(),
            modified_nested_fields: self.modified_nested_fields.clone(),
            modified_project_fields: self.modified_project_fields.clone(),
            modified_project_nested_fields: self.modified_project_nested_fields.clone(),
            global_settings_load_error: self.global_settings_load_error.clone(),
            project_settings_load_error: self.project_settings_load_error.clone(),
            errors: self.errors.clone(),
            default_packages: self.default_packages.clone(),
        }
    }
}

impl SettingsManager {
    // -----------------------------------------------------------------------
    // Construction
    // -----------------------------------------------------------------------

    fn new(
        storage: Box<dyn SettingsStorage>,
        initial_global: Settings,
        initial_project: Settings,
        global_settings_load_error: Option<String>,
        project_settings_load_error: Option<String>,
        initial_errors: Vec<SettingsError>,
        project_trusted: bool,
        default_packages: Vec<PackageSource>,
    ) -> Self {
        let settings = deep_merge_settings(&initial_global, &initial_project);
        Self {
            storage,
            global_settings: initial_global,
            project_settings: initial_project,
            settings,
            project_trusted,
            modified_fields: HashSet::new(),
            modified_nested_fields: HashMap::new(),
            modified_project_fields: HashSet::new(),
            modified_project_nested_fields: HashMap::new(),
            global_settings_load_error,
            project_settings_load_error,
            errors: initial_errors,
            default_packages,
        }
    }

    /// Create from filesystem storage.
    pub fn create(cwd: &str, agent_dir: &str, options: SettingsManagerCreateOptions) -> Self {
        if std::env::var("HAMR_CHILD_CONFIG").is_ok() {
            return Self::in_memory(None);
        }
        let storage = Box::new(FileSettingsStorage::new(cwd, agent_dir));
        Self::from_storage(storage, options)
    }

    /// Create from an arbitrary storage backend.
    pub fn from_storage(
        storage: Box<dyn SettingsStorage>,
        options: SettingsManagerCreateOptions,
    ) -> Self {
        let project_trusted = options.project_trusted.unwrap_or(true);
        let global_load = Self::try_load_from_storage(&*storage, SettingsScope::Global, true);
        let project_load =
            Self::try_load_from_storage(&*storage, SettingsScope::Project, project_trusted);

        let mut initial_errors: Vec<SettingsError> = Vec::new();
        if let Some(ref err) = global_load.error {
            initial_errors.push(SettingsError::new(SettingsScope::Global, err.clone()));
        }
        if let Some(ref err) = project_load.error {
            initial_errors.push(SettingsError::new(SettingsScope::Project, err.clone()));
        }

        Self::new(
            storage,
            global_load.settings,
            project_load.settings,
            global_load.error,
            project_load.error,
            initial_errors,
            project_trusted,
            options.default_packages.unwrap_or_default(),
        )
    }

    /// Create an in-memory settings manager (no file I/O).
    pub fn in_memory(initial: Option<Settings>) -> Self {
        let storage = Box::new(InMemorySettingsStorage::new());
        let settings = initial.unwrap_or_default();
        // Apply migration to initial settings (mirrors TS structuredClone + migrateSettings)
        let mut raw: serde_json::Value = serde_json::to_value(&settings).unwrap_or_default();
        migrate_settings(&mut raw);
        let serialized = serde_json::to_string_pretty(&raw).unwrap_or_else(|_| "{}".to_string());
        storage.with_lock(SettingsScope::Global, &mut |_| Some(serialized.clone()));
        Self::from_storage(storage, SettingsManagerCreateOptions::default())
    }

    // -----------------------------------------------------------------------
    // Loading
    // -----------------------------------------------------------------------

    fn load_from_storage(
        storage: &dyn SettingsStorage,
        scope: SettingsScope,
        project_trusted: bool,
    ) -> Settings {
        if scope == SettingsScope::Project && !project_trusted {
            return Settings::default();
        }
        let mut content: Option<String> = None;
        storage.with_lock(scope, &mut |current| {
            content = current.map(|s| s.to_string());
            None
        });

        match content {
            Some(raw) => {
                let mut parsed: serde_json::Value = serde_json::from_str(&raw)
                    .unwrap_or(serde_json::Value::Object(Default::default()));
                migrate_settings(&mut parsed);
                serde_json::from_value(parsed).unwrap_or_default()
            }
            None => Settings::default(),
        }
    }

    fn try_load_from_storage(
        storage: &dyn SettingsStorage,
        scope: SettingsScope,
        project_trusted: bool,
    ) -> SettingsLoadResult {
        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            Self::load_from_storage(storage, scope, project_trusted)
        })) {
            Ok(s) => SettingsLoadResult {
                settings: s,
                error: None,
            },
            Err(e) => {
                let msg = if let Some(s) = e.downcast_ref::<&str>() {
                    s.to_string()
                } else if let Some(s) = e.downcast_ref::<String>() {
                    s.clone()
                } else {
                    "unknown error loading settings".to_string()
                };
                SettingsLoadResult {
                    settings: Settings::default(),
                    error: Some(msg),
                }
            }
        }
    }

    // -----------------------------------------------------------------------
    // Accessors
    // -----------------------------------------------------------------------

    pub fn get_global_settings(&self) -> Settings {
        self.global_settings.clone()
    }

    pub fn get_project_settings(&self) -> Settings {
        self.project_settings.clone()
    }

    pub fn is_project_trusted(&self) -> bool {
        self.project_trusted
    }

    pub fn set_project_trusted(&mut self, trusted: bool) {
        if self.project_trusted == trusted {
            return;
        }
        self.project_trusted = trusted;
        self.modified_project_fields.clear();
        self.modified_project_nested_fields.clear();

        if !trusted {
            self.project_settings = Settings::default();
            self.project_settings_load_error = None;
            self.settings = deep_merge_settings(&self.global_settings, &self.project_settings);
            return;
        }

        let project_load =
            Self::try_load_from_storage(&*self.storage, SettingsScope::Project, true);
        self.project_settings = project_load.settings;
        self.project_settings_load_error = project_load.error.clone();
        if let Some(ref err) = project_load.error {
            self.record_error(SettingsScope::Project, err);
        }
        self.settings = deep_merge_settings(&self.global_settings, &self.project_settings);
    }

    pub fn reload(&mut self) {
        let global_load = Self::try_load_from_storage(&*self.storage, SettingsScope::Global, true);
        if global_load.error.is_none() {
            self.global_settings = global_load.settings;
            self.global_settings_load_error = None;
        } else if let Some(ref err) = global_load.error {
            self.global_settings_load_error = Some(err.clone());
            self.record_error(SettingsScope::Global, err);
        }

        self.modified_fields.clear();
        self.modified_nested_fields.clear();
        self.modified_project_fields.clear();
        self.modified_project_nested_fields.clear();

        let project_load = Self::try_load_from_storage(
            &*self.storage,
            SettingsScope::Project,
            self.project_trusted,
        );
        if project_load.error.is_none() {
            self.project_settings = project_load.settings;
            self.project_settings_load_error = None;
        } else if let Some(ref err) = project_load.error {
            self.project_settings_load_error = Some(err.clone());
            self.record_error(SettingsScope::Project, err);
        }

        self.settings = deep_merge_settings(&self.global_settings, &self.project_settings);
    }

    pub fn apply_overrides(&mut self, overrides: &Settings) {
        self.settings = deep_merge_settings(&self.settings, overrides);
    }

    // -----------------------------------------------------------------------
    // Modified tracking
    // -----------------------------------------------------------------------

    fn mark_modified(&mut self, field: &str, nested_key: Option<&str>) {
        self.modified_fields.insert(field.to_string());
        if let Some(nk) = nested_key {
            self.modified_nested_fields
                .entry(field.to_string())
                .or_default()
                .insert(nk.to_string());
        }
    }

    fn mark_project_modified(&mut self, field: &str, nested_key: Option<&str>) {
        self.modified_project_fields.insert(field.to_string());
        if let Some(nk) = nested_key {
            self.modified_project_nested_fields
                .entry(field.to_string())
                .or_default()
                .insert(nk.to_string());
        }
    }

    fn assert_project_trusted_for_write(&self) -> Result<(), SettingsManagerError> {
        if !self.project_trusted {
            return Err(SettingsManagerError::ProjectNotTrusted);
        }
        Ok(())
    }

    fn record_error(&mut self, scope: SettingsScope, error: &str) {
        self.errors.push(SettingsError::new(scope, error));
    }

    fn clear_modified_scope(&mut self, scope: SettingsScope) {
        match scope {
            SettingsScope::Global => {
                self.modified_fields.clear();
                self.modified_nested_fields.clear();
            }
            SettingsScope::Project => {
                self.modified_project_fields.clear();
                self.modified_project_nested_fields.clear();
            }
        }
    }

    // -----------------------------------------------------------------------
    // Persistence
    // -----------------------------------------------------------------------

    fn persist_scoped_settings(
        &self,
        scope: SettingsScope,
        snapshot_settings: &Settings,
        modified_fields: &HashSet<String>,
        modified_nested_fields: &HashMap<String, HashSet<String>>,
    ) {
        self.storage.with_lock(scope, &mut |current: Option<&str>| {
            let current_file_settings: Settings = current
                .and_then(|c| {
                    let mut v: serde_json::Value = serde_json::from_str(c).ok()?;
                    migrate_settings(&mut v);
                    serde_json::from_value(v).ok()
                })
                .unwrap_or_default();

            let mut merged: serde_json::Value = serde_json::to_value(&current_file_settings)
                .unwrap_or(serde_json::Value::Object(Default::default()));
            let snapshot_value: serde_json::Value = serde_json::to_value(snapshot_settings)
                .unwrap_or(serde_json::Value::Object(Default::default()));

            if let (Some(merged_obj), Some(snapshot_obj)) =
                (merged.as_object_mut(), snapshot_value.as_object())
            {
                for field in modified_fields.iter() {
                    if let Some(modified_nested) = modified_nested_fields.get(field) {
                        // Nested field merge: merge only the modified sub-keys
                        if let Some(snapshot_nested) =
                            snapshot_obj.get(field).and_then(|v| v.as_object())
                        {
                            let base_nested = merged_obj
                                .entry(field.clone())
                                .or_insert_with(|| serde_json::Value::Object(Default::default()));
                            if let Some(base_obj) = base_nested.as_object_mut() {
                                for nk in modified_nested.iter() {
                                    if let Some(nv) = snapshot_nested.get(nk) {
                                        base_obj.insert(nk.clone(), nv.clone());
                                    }
                                }
                            }
                        }
                    } else {
                        // Simple field: override entirely
                        if let Some(val) = snapshot_obj.get(field) {
                            merged_obj.insert(field.clone(), val.clone());
                        }
                    }
                }
            }

            Some(serde_json::to_string_pretty(&merged).unwrap_or_else(|_| "{}".to_string()))
        });
    }

    fn save(&mut self) {
        self.settings = deep_merge_settings(&self.global_settings, &self.project_settings);

        if self.global_settings_load_error.is_some() {
            return;
        }

        let snapshot_global = self.global_settings.clone();
        let modified_fields = self.modified_fields.clone();
        let modified_nested_fields = self.modified_nested_fields.clone();

        self.persist_scoped_settings(
            SettingsScope::Global,
            &snapshot_global,
            &modified_fields,
            &modified_nested_fields,
        );
        self.clear_modified_scope(SettingsScope::Global);
    }

    fn save_project_settings(&mut self, settings: &Settings) {
        if self.assert_project_trusted_for_write().is_err() {
            return;
        }

        self.project_settings = settings.clone();
        self.settings = deep_merge_settings(&self.global_settings, &self.project_settings);

        if self.project_settings_load_error.is_some() {
            return;
        }

        let snapshot_project = self.project_settings.clone();
        let modified_fields = self.modified_project_fields.clone();
        let modified_nested_fields = self.modified_project_nested_fields.clone();

        self.persist_scoped_settings(
            SettingsScope::Project,
            &snapshot_project,
            &modified_fields,
            &modified_nested_fields,
        );
        self.clear_modified_scope(SettingsScope::Project);
    }

    fn update_project_settings(&mut self, field: &str, update: impl FnOnce(&mut Settings)) {
        if self.assert_project_trusted_for_write().is_err() {
            return;
        }
        let mut project = self.project_settings.clone();
        update(&mut project);
        self.mark_project_modified(field, None);
        self.save_project_settings(&project);
    }

    /// Drain and return all accumulated errors.
    pub fn drain_errors(&mut self) -> Vec<SettingsError> {
        std::mem::take(&mut self.errors)
    }

    // -----------------------------------------------------------------------
    // Getters / Setters
    // -----------------------------------------------------------------------

    pub fn get_last_changelog_version(&self) -> Option<&str> {
        self.settings.last_changelog_version.as_deref()
    }

    pub fn set_last_changelog_version(&mut self, version: &str) {
        self.global_settings.last_changelog_version = Some(version.to_string());
        self.mark_modified("lastChangelogVersion", None);
        self.save();
    }

    pub fn get_session_dir(&self) -> Option<String> {
        self.settings
            .session_dir
            .as_ref()
            .map(|d| normalize_path(d, &Default::default()))
    }

    pub fn get_default_provider(&self) -> Option<&str> {
        self.settings.default_provider.as_deref()
    }

    pub fn get_default_model(&self) -> Option<&str> {
        self.settings.default_model.as_deref()
    }

    pub fn set_default_provider(&mut self, provider: &str) {
        self.global_settings.default_provider = Some(provider.to_string());
        self.mark_modified("defaultProvider", None);
        self.save();
    }

    pub fn set_default_model(&mut self, model_id: &str) {
        self.global_settings.default_model = Some(model_id.to_string());
        self.mark_modified("defaultModel", None);
        self.save();
    }

    pub fn set_default_model_and_provider(&mut self, provider: &str, model_id: &str) {
        self.global_settings.default_provider = Some(provider.to_string());
        self.global_settings.default_model = Some(model_id.to_string());
        self.mark_modified("defaultProvider", None);
        self.mark_modified("defaultModel", None);
        self.save();
    }

    pub fn get_steering_mode(&self) -> &str {
        self.settings
            .steering_mode
            .as_deref()
            .unwrap_or("one-at-a-time")
    }

    pub fn set_steering_mode(&mut self, mode: &str) {
        self.global_settings.steering_mode = Some(mode.to_string());
        self.mark_modified("steeringMode", None);
        self.save();
    }

    pub fn get_follow_up_mode(&self) -> &str {
        self.settings
            .follow_up_mode
            .as_deref()
            .unwrap_or("one-at-a-time")
    }

    pub fn set_follow_up_mode(&mut self, mode: &str) {
        self.global_settings.follow_up_mode = Some(mode.to_string());
        self.mark_modified("followUpMode", None);
        self.save();
    }

    pub fn get_theme(&self) -> Option<&str> {
        self.settings.theme.as_deref()
    }

    pub fn set_theme(&mut self, theme: &str) {
        self.global_settings.theme = Some(theme.to_string());
        self.mark_modified("theme", None);
        self.save();
    }

    pub fn get_default_thinking_level(&self) -> Option<&ThinkingLevel> {
        self.settings.default_thinking_level.as_ref()
    }

    pub fn set_default_thinking_level(&mut self, level: ThinkingLevel) {
        self.global_settings.default_thinking_level = Some(level);
        self.mark_modified("defaultThinkingLevel", None);
        self.save();
    }

    pub fn get_transport(&self) -> &TransportSetting {
        self.settings
            .transport
            .as_ref()
            .unwrap_or(&TransportSetting::Auto)
    }

    pub fn set_transport(&mut self, transport: TransportSetting) {
        self.global_settings.transport = Some(transport);
        self.mark_modified("transport", None);
        self.save();
    }

    pub fn get_compaction_enabled(&self) -> bool {
        self.settings
            .compaction
            .as_ref()
            .and_then(|c| c.enabled)
            .unwrap_or(true)
    }

    pub fn set_compaction_enabled(&mut self, enabled: bool) {
        self.global_settings
            .compaction
            .get_or_insert_with(CompactionSettings::default)
            .enabled = Some(enabled);
        self.mark_modified("compaction", Some("enabled"));
        self.save();
    }

    pub fn get_compaction_reserve_tokens(&self) -> u64 {
        self.settings
            .compaction
            .as_ref()
            .and_then(|c| c.reserve_tokens)
            .unwrap_or(16384)
    }

    pub fn get_compaction_keep_recent_tokens(&self) -> u64 {
        self.settings
            .compaction
            .as_ref()
            .and_then(|c| c.keep_recent_tokens)
            .unwrap_or(20000)
    }

    pub fn get_compaction_settings(&self) -> (bool, u64, u64) {
        (
            self.get_compaction_enabled(),
            self.get_compaction_reserve_tokens(),
            self.get_compaction_keep_recent_tokens(),
        )
    }

    pub fn get_branch_summary_reserve_tokens(&self) -> u64 {
        self.settings
            .branch_summary
            .as_ref()
            .and_then(|b| b.reserve_tokens)
            .unwrap_or(16384)
    }

    pub fn get_branch_summary_skip_prompt(&self) -> bool {
        self.settings
            .branch_summary
            .as_ref()
            .and_then(|b| b.skip_prompt)
            .unwrap_or(false)
    }

    pub fn get_branch_summary_settings(&self) -> (u64, bool) {
        (
            self.get_branch_summary_reserve_tokens(),
            self.get_branch_summary_skip_prompt(),
        )
    }

    pub fn get_retry_enabled(&self) -> bool {
        self.settings
            .retry
            .as_ref()
            .and_then(|r| r.enabled)
            .unwrap_or(true)
    }

    pub fn set_retry_enabled(&mut self, enabled: bool) {
        self.global_settings
            .retry
            .get_or_insert_with(|| RetrySettings {
                enabled: Some(true),
                max_retries: None,
                base_delay_ms: None,
                provider: None,
            })
            .enabled = Some(enabled);
        self.mark_modified("retry", Some("enabled"));
        self.save();
    }

    pub fn get_retry_settings(&self) -> (bool, u32, u64) {
        let r = self.settings.retry.as_ref();
        (
            r.and_then(|r| r.enabled).unwrap_or(true),
            r.and_then(|r| r.max_retries).unwrap_or(10),
            r.and_then(|r| r.base_delay_ms).unwrap_or(2000),
        )
    }

    pub fn get_http_idle_timeout_ms(&self) -> u64 {
        parse_timeout_setting(self.settings.http_idle_timeout_ms, "httpIdleTimeoutMs")
            .ok()
            .flatten()
            .unwrap_or(DEFAULT_HTTP_IDLE_TIMEOUT_MS)
    }

    pub fn set_http_idle_timeout_ms(&mut self, timeout_ms: u64) {
        self.global_settings.http_idle_timeout_ms = Some(timeout_ms);
        self.mark_modified("httpIdleTimeoutMs", None);
        self.save();
    }

    pub fn get_provider_retry_settings(&self) -> (Option<u64>, Option<u32>, u64) {
        let p = self
            .settings
            .retry
            .as_ref()
            .and_then(|r| r.provider.as_ref());
        (
            p.and_then(|p| p.timeout_ms),
            p.and_then(|p| p.max_retries),
            p.and_then(|p| p.max_retry_delay_ms).unwrap_or(60000),
        )
    }

    pub fn get_websocket_connect_timeout_ms(&self) -> Option<u64> {
        parse_timeout_setting(
            self.settings.websocket_connect_timeout_ms,
            "websocketConnectTimeoutMs",
        )
        .ok()
        .flatten()
    }

    pub fn get_hide_thinking_block(&self) -> bool {
        self.settings.hide_thinking_block.unwrap_or(false)
    }

    pub fn set_hide_thinking_block(&mut self, hide: bool) {
        self.global_settings.hide_thinking_block = Some(hide);
        self.mark_modified("hideThinkingBlock", None);
        self.save();
    }

    pub fn get_shell_path(&self) -> Option<&str> {
        self.settings.shell_path.as_deref()
    }

    pub fn set_shell_path(&mut self, path: Option<&str>) {
        self.global_settings.shell_path = path.map(|s| s.to_string());
        self.mark_modified("shellPath", None);
        self.save();
    }

    pub fn get_quiet_startup(&self) -> bool {
        self.settings.quiet_startup.unwrap_or(false)
    }

    pub fn set_quiet_startup(&mut self, quiet: bool) {
        self.global_settings.quiet_startup = Some(quiet);
        self.mark_modified("quietStartup", None);
        self.save();
    }

    pub fn get_default_project_trust(&self) -> DefaultProjectTrust {
        self.global_settings
            .default_project_trust
            .clone()
            .unwrap_or(DefaultProjectTrust::Ask)
    }

    pub fn set_default_project_trust(&mut self, trust: DefaultProjectTrust) {
        self.global_settings.default_project_trust = Some(trust);
        self.mark_modified("defaultProjectTrust", None);
        self.save();
    }

    pub fn get_shell_command_prefix(&self) -> Option<&str> {
        self.settings.shell_command_prefix.as_deref()
    }

    pub fn set_shell_command_prefix(&mut self, prefix: Option<&str>) {
        self.global_settings.shell_command_prefix = prefix.map(|s| s.to_string());
        self.mark_modified("shellCommandPrefix", None);
        self.save();
    }

    pub fn get_npm_command(&self) -> Option<&[String]> {
        self.settings.npm_command.as_deref()
    }

    pub fn set_npm_command(&mut self, command: Option<Vec<String>>) {
        self.global_settings.npm_command = command;
        self.mark_modified("npmCommand", None);
        self.save();
    }

    pub fn get_collapse_changelog(&self) -> bool {
        self.settings.collapse_changelog.unwrap_or(false)
    }

    pub fn set_collapse_changelog(&mut self, collapse: bool) {
        self.global_settings.collapse_changelog = Some(collapse);
        self.mark_modified("collapseChangelog", None);
        self.save();
    }

    pub fn get_enable_install_telemetry(&self) -> bool {
        self.settings.enable_install_telemetry.unwrap_or(true)
    }

    pub fn set_enable_install_telemetry(&mut self, enabled: bool) {
        self.global_settings.enable_install_telemetry = Some(enabled);
        self.mark_modified("enableInstallTelemetry", None);
        self.save();
    }

    pub fn get_enable_analytics(&self) -> bool {
        self.settings.enable_analytics.unwrap_or(false)
    }

    pub fn get_tracking_id(&self) -> Option<&str> {
        self.settings.tracking_id.as_deref()
    }

    /// Set analytics opt-in; generates a tracking identifier on first opt-in.
    pub fn set_enable_analytics(&mut self, enabled: bool) {
        self.global_settings.enable_analytics = Some(enabled);
        self.mark_modified("enableAnalytics", None);
        if enabled && self.global_settings.tracking_id.is_none() {
            self.global_settings.tracking_id = Some(generate_uuid_v4());
            self.mark_modified("trackingId", None);
        }
        self.save();
    }

    pub fn get_packages(&self) -> Vec<PackageSource> {
        self.settings.packages.clone().unwrap_or_default()
    }

    pub fn get_default_packages(&self) -> &[PackageSource] {
        &self.default_packages
    }

    pub fn set_packages(&mut self, packages: Vec<PackageSource>) {
        self.global_settings.packages = Some(packages);
        self.mark_modified("packages", None);
        self.save();
    }

    pub fn set_project_packages(&mut self, packages: Vec<PackageSource>) {
        self.update_project_settings("packages", |s| s.packages = Some(packages));
    }

    pub fn get_extension_paths(&self) -> Vec<String> {
        self.settings.extensions.clone().unwrap_or_default()
    }

    pub fn set_extension_paths(&mut self, paths: Vec<String>) {
        self.global_settings.extensions = Some(paths);
        self.mark_modified("extensions", None);
        self.save();
    }

    pub fn set_project_extension_paths(&mut self, paths: Vec<String>) {
        self.update_project_settings("extensions", |s| s.extensions = Some(paths));
    }

    pub fn get_skill_paths(&self) -> Vec<String> {
        self.settings.skills.clone().unwrap_or_default()
    }

    pub fn set_skill_paths(&mut self, paths: Vec<String>) {
        self.global_settings.skills = Some(paths);
        self.mark_modified("skills", None);
        self.save();
    }

    pub fn set_project_skill_paths(&mut self, paths: Vec<String>) {
        self.update_project_settings("skills", |s| s.skills = Some(paths));
    }

    pub fn get_prompt_template_paths(&self) -> Vec<String> {
        self.settings.prompts.clone().unwrap_or_default()
    }

    pub fn set_prompt_template_paths(&mut self, paths: Vec<String>) {
        self.global_settings.prompts = Some(paths);
        self.mark_modified("prompts", None);
        self.save();
    }

    pub fn set_project_prompt_template_paths(&mut self, paths: Vec<String>) {
        self.update_project_settings("prompts", |s| s.prompts = Some(paths));
    }

    pub fn get_theme_paths(&self) -> Vec<String> {
        self.settings.themes.clone().unwrap_or_default()
    }

    pub fn set_theme_paths(&mut self, paths: Vec<String>) {
        self.global_settings.themes = Some(paths);
        self.mark_modified("themes", None);
        self.save();
    }

    pub fn set_project_theme_paths(&mut self, paths: Vec<String>) {
        self.update_project_settings("themes", |s| s.themes = Some(paths));
    }

    pub fn get_enable_skill_commands(&self) -> bool {
        self.settings.enable_skill_commands.unwrap_or(true)
    }

    pub fn set_enable_skill_commands(&mut self, enabled: bool) {
        self.global_settings.enable_skill_commands = Some(enabled);
        self.mark_modified("enableSkillCommands", None);
        self.save();
    }

    pub fn get_thinking_budgets(&self) -> Option<&ThinkingBudgetsSettings> {
        self.settings.thinking_budgets.as_ref()
    }

    pub fn get_show_images(&self) -> bool {
        self.settings
            .terminal
            .as_ref()
            .and_then(|t| t.show_images)
            .unwrap_or(true)
    }

    pub fn set_show_images(&mut self, show: bool) {
        self.global_settings
            .terminal
            .get_or_insert_with(TerminalSettings::default)
            .show_images = Some(show);
        self.mark_modified("terminal", Some("showImages"));
        self.save();
    }

    pub fn get_image_width_cells(&self) -> u32 {
        let width = self
            .settings
            .terminal
            .as_ref()
            .and_then(|t| t.image_width_cells);
        match width {
            Some(w) => std::cmp::max(1, w),
            None => 60,
        }
    }

    pub fn set_image_width_cells(&mut self, width: u32) {
        let clamped = std::cmp::max(1, width);
        self.global_settings
            .terminal
            .get_or_insert_with(TerminalSettings::default)
            .image_width_cells = Some(clamped);
        self.mark_modified("terminal", Some("imageWidthCells"));
        self.save();
    }

    pub fn get_clear_on_shrink(&self) -> bool {
        if let Some(v) = self
            .settings
            .terminal
            .as_ref()
            .and_then(|t| t.clear_on_shrink)
        {
            return v;
        }
        std::env::var("HAMR_CLEAR_ON_SHRINK")
            .as_deref()
            .or(std::env::var("PI_CLEAR_ON_SHRINK").as_deref())
            == Ok("1")
    }

    pub fn set_clear_on_shrink(&mut self, enabled: bool) {
        self.global_settings
            .terminal
            .get_or_insert_with(TerminalSettings::default)
            .clear_on_shrink = Some(enabled);
        self.mark_modified("terminal", Some("clearOnShrink"));
        self.save();
    }

    pub fn get_show_terminal_progress(&self) -> bool {
        self.settings
            .terminal
            .as_ref()
            .and_then(|t| t.show_terminal_progress)
            .unwrap_or(false)
    }

    pub fn set_show_terminal_progress(&mut self, enabled: bool) {
        self.global_settings
            .terminal
            .get_or_insert_with(TerminalSettings::default)
            .show_terminal_progress = Some(enabled);
        self.mark_modified("terminal", Some("showTerminalProgress"));
        self.save();
    }

    pub fn get_image_auto_resize(&self) -> bool {
        self.settings
            .images
            .as_ref()
            .and_then(|i| i.auto_resize)
            .unwrap_or(true)
    }

    pub fn set_image_auto_resize(&mut self, enabled: bool) {
        self.global_settings
            .images
            .get_or_insert_with(ImageSettings::default)
            .auto_resize = Some(enabled);
        self.mark_modified("images", Some("autoResize"));
        self.save();
    }

    pub fn get_block_images(&self) -> bool {
        self.settings
            .images
            .as_ref()
            .and_then(|i| i.block_images)
            .unwrap_or(false)
    }

    pub fn set_block_images(&mut self, blocked: bool) {
        self.global_settings
            .images
            .get_or_insert_with(ImageSettings::default)
            .block_images = Some(blocked);
        self.mark_modified("images", Some("blockImages"));
        self.save();
    }

    pub fn get_enabled_models(&self) -> Option<&[String]> {
        self.settings.enabled_models.as_deref()
    }

    pub fn set_enabled_models(&mut self, patterns: Option<Vec<String>>) {
        self.global_settings.enabled_models = patterns;
        self.mark_modified("enabledModels", None);
        self.save();
    }

    pub fn get_double_escape_action(&self) -> &str {
        self.settings
            .double_escape_action
            .as_deref()
            .unwrap_or("tree")
    }

    pub fn set_double_escape_action(&mut self, action: &str) {
        self.global_settings.double_escape_action = Some(action.to_string());
        self.mark_modified("doubleEscapeAction", None);
        self.save();
    }

    pub fn get_tree_filter_mode(&self) -> &str {
        let mode = self
            .settings
            .tree_filter_mode
            .as_deref()
            .unwrap_or("default");
        let valid = ["default", "no-tools", "user-only", "labeled-only", "all"];
        if valid.contains(&mode) {
            mode
        } else {
            "default"
        }
    }

    pub fn set_tree_filter_mode(&mut self, mode: &str) {
        self.global_settings.tree_filter_mode = Some(mode.to_string());
        self.mark_modified("treeFilterMode", None);
        self.save();
    }

    pub fn get_show_hardware_cursor(&self) -> bool {
        self.settings
            .show_hardware_cursor
            .or_else(|| {
                let v1 = std::env::var("HAMR_HARDWARE_CURSOR");
                let v2 = std::env::var("PI_HARDWARE_CURSOR");
                Some(v1.as_deref() == Ok("1") || v2.as_deref() == Ok("1"))
            })
            .unwrap_or(false)
    }

    pub fn set_show_hardware_cursor(&mut self, enabled: bool) {
        self.global_settings.show_hardware_cursor = Some(enabled);
        self.mark_modified("showHardwareCursor", None);
        self.save();
    }

    pub fn get_editor_padding_x(&self) -> u32 {
        self.settings.editor_padding_x.unwrap_or(0)
    }

    pub fn set_editor_padding_x(&mut self, padding: u32) {
        let clamped = std::cmp::max(0, std::cmp::min(3, padding));
        self.global_settings.editor_padding_x = Some(clamped);
        self.mark_modified("editorPaddingX", None);
        self.save();
    }

    pub fn get_autocomplete_max_visible(&self) -> u32 {
        self.settings.autocomplete_max_visible.unwrap_or(5)
    }

    pub fn set_autocomplete_max_visible(&mut self, max_visible: u32) {
        let clamped = std::cmp::max(3, std::cmp::min(20, max_visible));
        self.global_settings.autocomplete_max_visible = Some(clamped);
        self.mark_modified("autocompleteMaxVisible", None);
        self.save();
    }

    pub fn get_code_block_indent(&self) -> &str {
        self.settings
            .markdown
            .as_ref()
            .and_then(|m| m.code_block_indent.as_deref())
            .unwrap_or("  ")
    }

    pub fn get_warnings(&self) -> WarningSettings {
        self.settings.warnings.clone().unwrap_or_default()
    }

    pub fn set_warnings(&mut self, warnings: WarningSettings) {
        self.global_settings.warnings = Some(warnings);
        self.mark_modified("warnings", None);
        self.save();
    }
}

impl Default for ImageSettings {
    fn default() -> Self {
        Self {
            auto_resize: Some(true),
            block_images: Some(false),
        }
    }
}

impl Default for TerminalSettings {
    fn default() -> Self {
        Self {
            show_images: Some(true),
            image_width_cells: Some(60),
            clear_on_shrink: Some(false),
            show_terminal_progress: Some(false),
        }
    }
}

// ---------------------------------------------------------------------------
// Internal result type for try_load_from_storage
// ---------------------------------------------------------------------------

struct SettingsLoadResult {
    settings: Settings,
    error: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deep_merge_both_empty() {
        let base = Settings::default();
        let overrides = Settings::default();
        let merged = deep_merge_settings(&base, &overrides);
        assert_eq!(merged, Settings::default());
    }

    #[test]
    fn test_deep_merge_base_wins_for_none_overrides() {
        let base = Settings {
            default_provider: Some("anthropic".into()),
            ..Default::default()
        };
        let overrides = Settings::default();
        let merged = deep_merge_settings(&base, &overrides);
        assert_eq!(merged.default_provider, Some("anthropic".into()));
    }

    #[test]
    fn test_deep_merge_overrides_override_base() {
        let base = Settings {
            default_provider: Some("anthropic".into()),
            ..Default::default()
        };
        let overrides = Settings {
            default_provider: Some("openai".into()),
            ..Default::default()
        };
        let merged = deep_merge_settings(&base, &overrides);
        assert_eq!(merged.default_provider, Some("openai".into()));
    }

    #[test]
    fn test_deep_merge_nested_compaction() {
        let base = Settings {
            compaction: Some(CompactionSettings {
                enabled: Some(false),
                reserve_tokens: Some(5000),
                keep_recent_tokens: Some(10000),
            }),
            ..Default::default()
        };
        let overrides = Settings {
            compaction: Some(CompactionSettings {
                enabled: Some(true),
                reserve_tokens: None,
                keep_recent_tokens: None,
            }),
            ..Default::default()
        };
        let merged = deep_merge_settings(&base, &overrides);
        assert_eq!(merged.compaction.as_ref().unwrap().enabled, Some(true));
        // reserve_tokens should come from base since override doesn't set it
        assert_eq!(
            merged.compaction.as_ref().unwrap().reserve_tokens,
            Some(5000)
        );
    }

    #[test]
    fn test_deep_merge_nested_retry() {
        let base = Settings {
            retry: Some(RetrySettings {
                enabled: Some(true),
                max_retries: Some(3),
                base_delay_ms: Some(1000),
                provider: None,
            }),
            ..Default::default()
        };
        let overrides = Settings {
            retry: Some(RetrySettings {
                enabled: None,
                max_retries: Some(5),
                base_delay_ms: None,
                provider: None,
            }),
            ..Default::default()
        };
        let merged = deep_merge_settings(&base, &overrides);
        assert_eq!(merged.retry.as_ref().unwrap().enabled, Some(true));
        assert_eq!(merged.retry.as_ref().unwrap().max_retries, Some(5));
        assert_eq!(merged.retry.as_ref().unwrap().base_delay_ms, Some(1000));
    }

    #[test]
    fn test_default_settings_have_default_values() {
        let s = Settings::default();
        assert_eq!(s.default_thinking_level, None);
        assert_eq!(s.steering_mode.as_deref(), None);
        assert_eq!(s.http_idle_timeout_ms, None);
    }

    #[test]
    fn test_get_agent_dir_returns_path() {
        let dir = get_agent_dir();
        assert!(!dir.is_empty());
        assert!(dir.contains("hamr"));
    }

    #[test]
    fn test_default_compaction_settings() {
        let c = CompactionSettings::default();
        assert_eq!(c.enabled, Some(true));
        assert_eq!(c.reserve_tokens, Some(16384));
        assert_eq!(c.keep_recent_tokens, Some(20000));
    }

    #[test]
    fn test_retry_settings_all_none_by_default() {
        // RetrySettings doesn't implement Default, so construct explicitly
        let r = RetrySettings {
            enabled: None,
            max_retries: None,
            base_delay_ms: None,
            provider: None,
        };
        assert_eq!(r.enabled, None);
        assert_eq!(r.max_retries, None);
        assert_eq!(r.base_delay_ms, None);
    }

    #[test]
    fn test_default_terminal_settings() {
        let t = TerminalSettings::default();
        assert_eq!(t.show_images, Some(true));
        assert_eq!(t.image_width_cells, Some(60));
    }

    #[test]
    fn test_package_source_roundtrip() {
        let src = PackageSource::String("npm:test-pkg".into());
        let json = serde_json::to_value(&src).unwrap();
        assert_eq!(json, serde_json::json!("npm:test-pkg"));

        let src2: PackageSource = serde_json::from_value(json).unwrap();
        assert_eq!(src, src2);
    }

    #[test]
    fn test_package_source_object_roundtrip() {
        let src = PackageSource::Object {
            source: "npm:test-pkg".into(),
            extensions: Some(vec!["ext.ts".into()]),
            skills: None,
            prompts: None,
            themes: None,
        };
        let json = serde_json::to_value(&src).unwrap();
        assert_eq!(json["source"], "npm:test-pkg");
        assert_eq!(json["extensions"][0], "ext.ts");

        let src2: PackageSource = serde_json::from_value(json).unwrap();
        assert!(matches!(src2, PackageSource::Object { .. }));
    }

    #[test]
    fn test_settings_serialize_camel_case() {
        let s = Settings {
            default_provider: Some("anthropic".into()),
            default_model: Some("claude-sonnet-4".into()),
            theme: Some("dark".into()),
            ..Default::default()
        };
        let json = serde_json::to_value(&s).unwrap();
        // Must use camelCase
        assert_eq!(json["defaultProvider"], "anthropic");
        assert_eq!(json["defaultModel"], "claude-sonnet-4");
        // snake_case should NOT be present
        assert!(json.get("default_provider").is_none());
    }

    // --- migrate_settings ---

    #[test]
    fn test_migrate_queue_mode_to_steering_mode() {
        let mut raw = serde_json::json!({"queueMode": "one-at-a-time"});
        migrate_settings(&mut raw);
        assert_eq!(raw["steeringMode"], "one-at-a-time");
        assert!(raw.get("queueMode").is_none());
    }

    #[test]
    fn test_migrate_queue_mode_does_not_override_existing_steering_mode() {
        let mut raw = serde_json::json!({"queueMode": "old", "steeringMode": "existing"});
        migrate_settings(&mut raw);
        assert_eq!(raw["steeringMode"], "existing");
    }

    #[test]
    fn test_migrate_websockets_bool_to_transport_sse() {
        let mut raw = serde_json::json!({"websockets": false});
        migrate_settings(&mut raw);
        assert_eq!(raw["transport"], "sse");
        assert!(raw.get("websockets").is_none());
    }

    #[test]
    fn test_migrate_websockets_bool_to_transport_websocket() {
        let mut raw = serde_json::json!({"websockets": true});
        migrate_settings(&mut raw);
        assert_eq!(raw["transport"], "websocket");
        assert!(raw.get("websockets").is_none());
    }

    #[test]
    fn test_migrate_websockets_skipped_when_transport_already_set() {
        let mut raw = serde_json::json!({"websockets": true, "transport": "auto"});
        migrate_settings(&mut raw);
        assert_eq!(raw["transport"], "auto");
    }

    #[test]
    fn test_migrate_skills_object_to_array_and_enable_skill_commands() {
        let mut raw = serde_json::json!({
            "skills": {
                "enableSkillCommands": true,
                "customDirectories": ["/path/to/skills"]
            }
        });
        migrate_settings(&mut raw);
        assert_eq!(raw["enableSkillCommands"], true);
        assert_eq!(raw["skills"], serde_json::json!(["/path/to/skills"]));
    }

    #[test]
    fn test_migrate_skills_object_empty_dirs_removes_skills() {
        let mut raw = serde_json::json!({
            "skills": {
                "enableSkillCommands": false,
                "customDirectories": []
            }
        });
        migrate_settings(&mut raw);
        assert_eq!(raw["enableSkillCommands"], false);
        assert!(raw.get("skills").is_none());
    }

    #[test]
    fn test_migrate_skills_object_no_dirs_removes_skills() {
        let mut raw = serde_json::json!({
            "skills": {
                "enableSkillCommands": true
            }
        });
        migrate_settings(&mut raw);
        assert_eq!(raw["enableSkillCommands"], true);
        assert!(raw.get("skills").is_none());
    }

    #[test]
    fn test_migrate_skills_array_unchanged() {
        let mut raw = serde_json::json!({"skills": ["/path/to/skills"]});
        migrate_settings(&mut raw);
        assert_eq!(raw["skills"], serde_json::json!(["/path/to/skills"]));
    }

    #[test]
    fn test_migrate_skills_no_skills_unchanged() {
        let mut raw = serde_json::json!({"theme": "dark"});
        migrate_settings(&mut raw);
        assert_eq!(raw["theme"], "dark");
    }

    #[test]
    fn test_migrate_retry_max_delay_ms_to_provider() {
        let mut raw = serde_json::json!({
            "retry": {
                "enabled": true,
                "maxDelayMs": 120000
            }
        });
        migrate_settings(&mut raw);
        assert!(raw["retry"].get("maxDelayMs").is_none());
        assert_eq!(raw["retry"]["provider"]["maxRetryDelayMs"], 120000.0);
    }

    #[test]
    fn test_migrate_retry_max_delay_ms_does_not_override_existing() {
        let mut raw = serde_json::json!({
            "retry": {
                "enabled": true,
                "maxDelayMs": 120000,
                "provider": { "maxRetryDelayMs": 30000 }
            }
        });
        migrate_settings(&mut raw);
        assert_eq!(raw["retry"]["provider"]["maxRetryDelayMs"], 30000.0);
    }

    #[test]
    fn test_parse_timeout_setting_none() {
        let result = parse_timeout_setting(None, "test");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), None);
    }

    #[test]
    fn test_parse_timeout_setting_some() {
        let result = parse_timeout_setting(Some(5000), "test");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Some(5000));
    }

    // --- DefaultProjectTrust ---

    #[test]
    fn test_default_project_trust_default_is_ask() {
        let s = Settings::default();
        // get_default_project_trust on an in-memory manager checks global_settings only
        let mgr = SettingsManager::in_memory(None);
        assert_eq!(mgr.get_default_project_trust(), DefaultProjectTrust::Ask);
    }
}
