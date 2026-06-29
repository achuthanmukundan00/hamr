//! Agent session creation — port of `packages/coding-agent/src/core/sdk.ts`.
//!
//! Provides [`CreateAgentSessionOptions`], [`CreateAgentSessionResult`], and the
//! [`create_agent_session`] async function that wires together auth storage,
//! model registry, session manager, settings, resource loading, and the
//! [`Agent`] loop.
//!
//! # Child processes (subagents)
//!
//! Child hamr processes inherit auth from the parent environment (`process.env`).
//! No temp-file config is written — the `HAMR_CHILD_CONFIG` path from earlier
//! versions has been removed (v0.7.1 security fix).  Children read API keys and
//! provider configuration directly from the inherited environment.

use std::path::PathBuf;
use std::sync::Arc;

use crate::core::defaults::DEFAULT_THINKING_LEVEL;

// ---------------------------------------------------------------------------
// Re-exports from ported sibling modules.
//
// These types are publicly available via `hamr_agent::core::sdk::*`.
// Mirror of the TS `export { ... } from "..."` lines.
// ---------------------------------------------------------------------------

// ---- auth_storage ----
pub use crate::core::auth_storage::{
    ApiKeyCredential, AuthCredential, AuthStatus, AuthStorage, OAuthCredential,
};

// ---- model_registry ----
pub use crate::core::model_registry::ModelRegistry;

// ---- session_manager ----
pub use crate::core::session_manager::{
    BranchSummaryEntry, CURRENT_SESSION_VERSION, CompactionEntry, CustomEntry, CustomMessageEntry,
    FileEntry, ModelChangeEntry, SessionContext, SessionEntry, SessionHeader, SessionInfoEntry,
    SessionManager, SessionMessageEntry, ThinkingLevelChangeEntry, build_session_context,
    find_most_recent_session, get_latest_compaction_entry, migrate_session_entries,
    parse_session_entries,
};

// ---- agent_session ----
pub use crate::core::agent_session::{
    AgentSessionEvent, AgentSharedState, CompactionReason, ExtensionBindings, ModelCycleResult,
    ParsedSkillBlock, PromptOptions, SessionStats, TokenStats, parse_skill_block,
};

// ---- extensions::types ----
pub use crate::core::extensions::types::{
    BeforeAgentStartCombinedResult, CompactOptions, ContextUsage, DiscoveredResourcePath,
    Extension, ExtensionAPI, ExtensionActions, ExtensionCommandContext,
    ExtensionCommandContextActions, ExtensionContext, ExtensionContextActions, ExtensionError,
    ExtensionFactory, ExtensionFlag, ExtensionHandlerFn, ExtensionLoadError, ExtensionMode,
    ExtensionRuntime, ExtensionShortcut, ExtensionUIContext, FlagType, InputAction,
    InputEventResult, LoadExtensionsResult, MessageRendererFn, NavigateTreeOptions,
    NewSessionOptions, NewSessionResult, PendingProviderRegistration, ProjectTrustEmitResult,
    RegisteredCommand, RegisteredTool, ResolvedCommand, ResourcesDiscoveredPaths,
    RoleMessageRendererFn, SendMessageOptions, SendUserContent, SendUserOptions,
    SessionBeforeResult, SwitchSessionOptions, ToolCallEventResult, ToolDefinition, ToolInfo,
    ToolResultEventResult, UserBashEventResult, create_extension_runtime,
};

// ---- extensions::runner ----
pub use crate::core::extensions::runner::{
    ExtensionErrorListener, ExtensionRunner, NoOpUIContext, RunnerCommandContext,
    RunnerExtensionContext, emit_project_trust_event, emit_session_shutdown_event,
};

// ---- extensions::loader ----
pub use crate::core::extensions::loader::{
    discover_and_load_extensions, load_extension_from_factory, load_extensions,
};

// ---- skills ----
pub use crate::core::skills::{
    LoadSkillsFromDirOptions, LoadSkillsOptions, LoadSkillsResult, Skill, SkillFrontmatter,
    format_skills_for_prompt, load_skills, load_skills_from_dir,
};

// ---- tools::tool_definition_wrapper ----
pub use crate::core::tools::tool_definition_wrapper::{
    create_tool_definition_from_agent_tool, wrap_tool_definition, wrap_tool_definitions,
};

// ---- prompt_templates ----
pub use crate::core::prompt_templates::{
    LoadPromptTemplatesOptions, PromptTemplate, expand_prompt_template, load_prompt_templates,
    parse_command_args, substitute_args,
};

// ---- resource_loader ----
pub use crate::core::resource_loader::{
    ContextFile, DefaultResourceLoader, ResourceDiagnostic, load_project_context_files,
};

// ---------------------------------------------------------------------------
// Aliases for types used in this module's signatures.
//
// These shadow the pub-use names so that struct fields and function signatures
// within this module point at the concrete (non-pub-use-exported) path.
// ---------------------------------------------------------------------------

/// Auth storage type alias — re-export is at module scope as `AuthStorage`.
pub use crate::core::auth_storage::AuthStorage as SdkAuthStorage;
pub use crate::core::extensions::types::LoadExtensionsResult as SdkLoadExtensionsResult;
pub use crate::core::extensions::types::SessionBeforeResult as SdkSessionStartEvent;
pub use crate::core::extensions::types::ToolDefinition as SdkToolDefinition;
pub use crate::core::model_registry::ModelRegistry as SdkModelRegistry;
pub use crate::core::prompt_templates::PromptTemplate as SdkPromptTemplate;
pub use crate::core::session_manager::SessionManager as SdkSessionManager;
pub use crate::core::skills::Skill as SdkSkill;

// ---------------------------------------------------------------------------
// CreateAgentSessionOptions
// ---------------------------------------------------------------------------

/// Options for [`create_agent_session`].
///
/// Mirrors the TS `interface CreateAgentSessionOptions`.
pub struct CreateAgentSessionOptions {
    /// Working directory for project-local discovery. Default: `process.cwd()`.
    pub cwd: Option<PathBuf>,

    /// Global config directory. Default: `~/.hamr/agent`.
    pub agent_dir: Option<PathBuf>,

    // ── Dependencies (injected or created with defaults) ──────────────────
    /// Auth storage for credentials. Default: `AuthStorage::create(agent_dir/auth.json)`.
    pub auth_storage: Option<SdkAuthStorage>,

    /// Model registry. Default: `ModelRegistry::create(auth, agent_dir/models.json)`.
    pub model_registry: Option<SdkModelRegistry>,

    // ── Model & thinking ──────────────────────────────────────────────────
    /// Model to use. Default: from settings, else first available.
    pub model: Option<hamr_ai::types::Model>,

    /// Thinking level. Default: from settings, else `Medium` (clamped to model).
    pub thinking_level: Option<hamr_ai::types::ThinkingLevel>,

    /// Models available for cycling (Ctrl+P in interactive mode).
    pub scoped_models: Option<Vec<ScopedModel>>,

    // ── Tool control ──────────────────────────────────────────────────────
    /// Default tool suppression mode when no explicit allowlist is provided.
    ///
    /// - `"all"`: start with no tools enabled
    /// - `"builtin"`: disable default built-in tools (read, bash, edit,
    ///   write) but keep extension/custom tools enabled
    pub no_tools: Option<NoToolsMode>,

    /// Optional allowlist of tool names.
    ///
    /// When omitted, the default built-in tools (read, bash, edit, write) are
    /// enabled and extension/custom tools remain enabled unless `no_tools`
    /// changes that default.  When provided, only the listed tools are enabled.
    pub tools: Option<Vec<String>>,

    /// Optional denylist of tool names.  Applied after `tools` when both are set.
    pub exclude_tools: Option<Vec<String>>,

    /// Custom tools to register (in addition to built-in tools).
    pub custom_tools: Option<Vec<SdkToolDefinition>>,

    // ── Resource loading ──────────────────────────────────────────────────
    /// Resource loader. When omitted, `DefaultResourceLoader` is used.
    pub resource_loader: Option<DefaultResourceLoader>,

    /// Suppress default user/project skills. Explicit `skill_paths` still load.
    pub no_skills: bool,

    /// Additional skill files or directories, normally supplied by the CLI.
    pub skill_paths: Option<Vec<PathBuf>>,

    /// Suppress AGENTS.md / CLAUDE.md discovery.
    pub no_context_files: bool,

    /// Custom system prompt text or file path.
    pub system_prompt: Option<String>,

    /// Text or file paths appended to the system prompt.
    pub append_system_prompt: Option<Vec<String>>,

    /// Suppress default user/project prompt templates.
    pub no_prompt_templates: bool,

    /// Additional prompt-template files or directories.
    pub prompt_template_paths: Option<Vec<PathBuf>>,

    // ── Session & settings ────────────────────────────────────────────────
    /// Session manager. Default: `SessionManager::create(cwd)`.
    pub session_manager: Option<SdkSessionManager>,

    /// Settings manager. Default: `SettingsManager::create(cwd, agent_dir)`.
    pub settings_manager: Option<crate::core::settings_manager::SettingsManager>,

    /// Session start event metadata for extension runtime startup.
    pub session_start_event: Option<SdkSessionStartEvent>,

    // ── Extension wiring ──────────────────────────────────────────────────
    /// Pre-built extension runner (e.g. from CLI `build_extension_runner`).
    /// When provided, skips internal extension loading and uses this runner.
    pub extension_runner: Option<crate::core::extensions::runner::ExtensionRunner>,
}

impl Default for CreateAgentSessionOptions {
    fn default() -> Self {
        Self {
            cwd: None,
            agent_dir: None,
            auth_storage: None,
            model_registry: None,
            model: None,
            thinking_level: None,
            scoped_models: None,
            no_tools: None,
            tools: None,
            exclude_tools: None,
            custom_tools: None,
            resource_loader: None,
            no_skills: false,
            skill_paths: None,
            no_context_files: false,
            system_prompt: None,
            append_system_prompt: None,
            no_prompt_templates: false,
            prompt_template_paths: None,
            session_manager: None,
            settings_manager: None,
            session_start_event: None,
            extension_runner: None,
        }
    }
}

/// A model+thinking-level pair for the model-cycling UI.
#[derive(Debug, Clone)]
pub struct ScopedModel {
    pub model: hamr_ai::types::Model,
    pub thinking_level: Option<hamr_ai::types::ThinkingLevel>,
}

/// Tool suppression mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoToolsMode {
    /// Disable all tools.
    All,
    /// Disable only built-in tools (read, bash, edit, write).
    Builtin,
}

// ---------------------------------------------------------------------------
// CreateAgentSessionResult
// ---------------------------------------------------------------------------

/// Result from [`create_agent_session`].
pub struct CreateAgentSessionResult {
    /// The created session.
    pub session: crate::core::agent_session::AgentSession,

    /// Extensions result (for UI context setup in interactive mode).
    pub extensions_result: SdkLoadExtensionsResult,

    /// Extension runner, created from loaded extensions.
    pub extension_runner: Option<crate::core::extensions::runner::ExtensionRunner>,

    /// Warning if session was restored with a different model than saved.
    pub model_fallback_message: Option<String>,
}

// ---------------------------------------------------------------------------
// Helper: resolve agent dir
// ---------------------------------------------------------------------------

/// Resolve the agent directory path.
fn get_default_agent_dir() -> PathBuf {
    // Mirror TS `getAgentDir()` which delegates to `crate::config::getAgentDir`.
    // Resolve agent directory from XDG or $HOME.
    if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
        PathBuf::from(xdg).join("hamr").join("agent")
    } else if let Ok(home) = std::env::var("HOME") {
        PathBuf::from(home).join(".hamr").join("agent")
    } else {
        PathBuf::from(".hamr").join("agent")
    }
}

/// Resolve a path, guarding against empty strings.
fn resolve_path(input: Option<&PathBuf>) -> PathBuf {
    match input {
        Some(p) if p.as_os_str().is_empty() => {
            std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
        }
        Some(p) => {
            // If relative, resolve against cwd.
            if p.is_relative() {
                std::env::current_dir()
                    .unwrap_or_else(|_| PathBuf::from("."))
                    .join(p)
            } else {
                p.clone()
            }
        }
        None => std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
    }
}

// ---------------------------------------------------------------------------
// create_agent_session — main entry point
// ---------------------------------------------------------------------------

/// Create an [`AgentSession`] with the specified options.
///
/// 1. Resolve cwd, agent dir, and dependencies (auth, model registry, settings,
///    session manager).
/// 2. Load resources (extensions, skills, prompts, themes, context files).
/// 3. Restore session state if continuing.
/// 4. Resolve model (options → settings → first available).
/// 5. Resolve thinking level (options → settings → DEFAULT_THINKING_LEVEL).
/// 6. Set up tools (defaults + allowlist + denylist).
/// 7. Build `Agent` with stream function, extension wiring, image blocking,
///    retry settings, steering/followUp mode.
/// 8. Restore messages if continuing.
/// 9. Build and return `AgentSession`.
///
/// # Child processes (subagents)
///
/// Child hamr processes inherit auth from the parent environment.  No temp-file
/// config is read — `HAMR_CHILD_CONFIG` was removed in v0.7.1.  Children go
/// through the normal path; API keys are resolved from the inherited environment.
#[tracing::instrument(skip_all)]
pub async fn create_agent_session(options: CreateAgentSessionOptions) -> CreateAgentSessionResult {
    // ─── Normal path ─────────────────────────────────────────────────────────
    // Mirrors the TS `createAgentSession()` normal path.
    //
    // 1. Resolve cwd, agent dir
    // 2. Create AuthStorage, ModelRegistry, SettingsManager, SessionManager
    // 3. Resolve model (options → settings → first available)
    // 4. Resolve thinking level (options → settings → DEFAULT_THINKING_LEVEL)
    // 5. Set up tools
    // 6. Build system prompt
    // 7. Create Agent with stream_fn
    // 8. Create AgentSession

    use std::sync::Arc;

    let cwd = resolve_path(options.cwd.as_ref());
    let cwd_str = cwd.to_string_lossy().to_string();
    let agent_dir = options
        .agent_dir
        .clone()
        .unwrap_or_else(get_default_agent_dir);
    let agent_dir_str = agent_dir.to_string_lossy().to_string();

    // ── Auth storage ────────────────────────────────────────────────────────
    let auth_storage = Arc::new(options.auth_storage.unwrap_or_else(|| {
        crate::core::auth_storage::AuthStorage::create(&agent_dir.join("auth.json"))
    }));

    // ── Model registry ─────────────────────────────────────────────────────
    let model_registry = Arc::new(options.model_registry.unwrap_or_else(|| {
        let registry_auth: Arc<dyn crate::core::model_registry::auth_trait::AuthStorage> =
            auth_storage.clone();
        crate::core::model_registry::ModelRegistry::create(
            registry_auth,
            agent_dir.join("models.json").to_string_lossy().to_string(),
        )
    }));

    // ── Settings manager ───────────────────────────────────────────────────
    let settings_manager = options.settings_manager.unwrap_or_else(|| {
        crate::core::settings_manager::SettingsManager::create(
            &cwd_str,
            &agent_dir_str,
            crate::core::settings_manager::SettingsManagerCreateOptions::default(),
        )
    });

    // Resolve resources exposed by already-installed configured packages.
    // Package installation/update remains an explicit CLI operation; startup
    // only reads packages that are present on disk.
    let mut package_skill_paths = Vec::new();
    let mut package_prompt_paths = Vec::new();
    if !options.no_skills || !options.no_prompt_templates {
        use crate::core::package_manager::{
            DefaultPackageManager, PackageManager, PackageManagerOptions, ResolveExtensionOptions,
        };
        use crate::core::settings_manager::PackageSource;

        let package_manager = DefaultPackageManager::new(
            PackageManagerOptions {
                cwd: cwd.clone(),
                agent_dir: agent_dir.clone(),
            },
            settings_manager.clone(),
        );
        let source_string = |package: PackageSource| match package {
            PackageSource::String(source) => source,
            PackageSource::Object { source, .. } => source,
        };
        let global_sources: Vec<String> = settings_manager
            .get_global_settings()
            .packages
            .unwrap_or_default()
            .into_iter()
            .map(source_string)
            .collect();
        let project_sources: Vec<String> = settings_manager
            .get_project_settings()
            .packages
            .unwrap_or_default()
            .into_iter()
            .map(source_string)
            .collect();
        let global_resources = package_manager
            .resolve_extension_sources(&global_sources, None)
            .await;
        let project_resources = package_manager
            .resolve_extension_sources(
                &project_sources,
                Some(ResolveExtensionOptions {
                    local: true,
                    temporary: false,
                }),
            )
            .await;
        package_skill_paths.extend(
            global_resources
                .skills
                .into_iter()
                .chain(project_resources.skills)
                .filter(|resource| resource.enabled)
                .map(|resource| resource.path),
        );
        package_prompt_paths.extend(
            global_resources
                .prompts
                .into_iter()
                .chain(project_resources.prompts)
                .filter(|resource| resource.enabled)
                .map(|resource| resource.path),
        );
    }

    // ── Resource loader ───────────────────────────────────────────────────
    // The TypeScript SDK reloads resources before building the system prompt.
    // Keep CLI paths distinct so --no-skills can suppress configured/default
    // skills while still permitting explicitly requested skills.
    let mut resource_loader = options.resource_loader.unwrap_or_else(|| {
        let mut skill_paths = if options.no_skills {
            vec![]
        } else {
            settings_manager
                .get_skill_paths()
                .into_iter()
                .map(PathBuf::from)
                .collect()
        };
        if !options.no_skills {
            let package_dir = crate::config::get_package_dir();
            let browser_skills = package_dir.join("examples/extensions/hamr-browser/skills");
            let askr_skills = package_dir.join("dist/askr/skills");
            if browser_skills.exists() {
                skill_paths.push(browser_skills);
            }
            if askr_skills.exists() {
                skill_paths.push(askr_skills);
            }
        }
        skill_paths.extend(package_skill_paths);
        skill_paths.extend(options.skill_paths.clone().unwrap_or_default());

        let mut loader = crate::core::resource_loader::DefaultResourceLoader::with_options(
            cwd.clone(),
            agent_dir.clone(),
            settings_manager.is_project_trusted(),
            false,
            options.no_skills,
            false,
            false,
            options.no_context_files,
            options.system_prompt.clone(),
            options.append_system_prompt.clone(),
        );
        loader.set_skill_paths(skill_paths);
        loader
    });
    resource_loader.reload();

    let mut prompt_template_paths = if options.no_prompt_templates {
        vec![]
    } else {
        settings_manager
            .get_prompt_template_paths()
            .into_iter()
            .map(PathBuf::from)
            .collect()
    };
    if !options.no_prompt_templates {
        prompt_template_paths.extend(package_prompt_paths);
    }
    prompt_template_paths.extend(options.prompt_template_paths.clone().unwrap_or_default());
    let prompt_templates = crate::core::prompt_templates::load_prompt_templates(
        crate::core::prompt_templates::LoadPromptTemplatesOptions {
            cwd: cwd_str.clone(),
            agent_dir: agent_dir_str.clone(),
            prompt_paths: prompt_template_paths
                .into_iter()
                .map(|path| path.to_string_lossy().to_string())
                .collect(),
            include_defaults: !options.no_prompt_templates,
        },
    );

    // ── Session manager ────────────────────────────────────────────────────
    let mut session_manager = options.session_manager.unwrap_or_else(|| {
        let session_dir = crate::core::session_manager::get_default_session_dir_path_for_agent(
            &cwd_str, &agent_dir,
        );
        crate::core::session_manager::SessionManager::create(&cwd_str, Some(&session_dir))
    });
    let existing_session = session_manager.build_session_context();
    let has_existing_session = !existing_session.messages.is_empty();

    // ── Model resolution ────────────────────────────────────────────────────
    // Priority: options.model → settings default → first available
    let mut model = options.model;
    let mut model_fallback_message: Option<String> = None;

    if model.is_none() && has_existing_session {
        if let Some(restored) = existing_session.model.as_ref() {
            model = model_registry.find(&restored.provider, &restored.model_id);
            if model.is_none() {
                model_fallback_message = Some(format!(
                    "Could not restore model {}/{}",
                    restored.provider, restored.model_id
                ));
            }
        }
    }

    if model.is_none() {
        if let (Some(prov), Some(mid)) = (
            settings_manager.get_default_provider(),
            settings_manager.get_default_model(),
        ) {
            model = model_registry.find(prov, mid);
        }
    }

    if model.is_none() {
        // Check env for ANTHROPIC_API_KEY first — mirrors TS findInitialModel priority
        if std::env::var("ANTHROPIC_API_KEY").is_ok() {
            eprintln!("[debug] ANTHROPIC_API_KEY found, resolving anthropic model");
            model = hamr_ai::models::get_model("anthropic", "claude-sonnet-4-20250514");
            eprintln!(
                "[debug] anthropic model resolved: {:?}",
                model.as_ref().map(|m| &m.id)
            );
        }
    }

    if model.is_none() {
        // Check env for OPENAI_API_KEY
        if std::env::var("OPENAI_API_KEY").is_ok() {
            model = hamr_ai::models::get_model("openai", "gpt-4o");
        }
    }

    if model.is_none() {
        let available = model_registry.get_available();
        if let Some(first) = available.first() {
            model = Some(first.clone());
        } else {
            // Try built-in models — prefer anthropic
            if let Some(m) = hamr_ai::models::get_model("anthropic", "claude-sonnet-4-20250514") {
                model = Some(m);
            } else {
                let all = model_registry.get_all();
                if let Some(first) = all.first() {
                    model = Some(first.clone());
                } else {
                    model_fallback_message = Some(
                        "No models available. Set ANTHROPIC_API_KEY or configure auth.".to_string(),
                    );
                }
            }
        }
    }

    // ── Thinking level resolution ───────────────────────────────────────────
    let restored_thinking_level =
        has_existing_session.then(|| match existing_session.thinking_level.as_str() {
            "minimal" => hamr_ai::types::ThinkingLevel::Minimal,
            "medium" => hamr_ai::types::ThinkingLevel::Medium,
            "high" => hamr_ai::types::ThinkingLevel::High,
            "xhigh" => hamr_ai::types::ThinkingLevel::XHigh,
            _ => hamr_ai::types::ThinkingLevel::Low,
        });
    let thinking_level = options
        .thinking_level
        .or(restored_thinking_level)
        .or_else(|| {
            settings_manager
                .get_default_thinking_level()
                .map(|l| match l {
                    crate::core::settings_manager::ThinkingLevel::Off => {
                        hamr_ai::types::ThinkingLevel::Low
                    }
                    crate::core::settings_manager::ThinkingLevel::Minimal => {
                        hamr_ai::types::ThinkingLevel::Minimal
                    }
                    crate::core::settings_manager::ThinkingLevel::Low => {
                        hamr_ai::types::ThinkingLevel::Low
                    }
                    crate::core::settings_manager::ThinkingLevel::Medium => {
                        hamr_ai::types::ThinkingLevel::Medium
                    }
                    crate::core::settings_manager::ThinkingLevel::High => {
                        hamr_ai::types::ThinkingLevel::High
                    }
                    crate::core::settings_manager::ThinkingLevel::Xhigh => {
                        hamr_ai::types::ThinkingLevel::XHigh
                    }
                })
        })
        .unwrap_or(DEFAULT_THINKING_LEVEL);

    // ── Tool registry and active names ──────────────────────────────────────
    // Extension and SDK tools participate in the same allow/deny semantics as
    // built-ins. Later definitions override earlier definitions by name.
    let mut tool_definitions = crate::core::tools::create_all_tool_definitions(&cwd);
    let mut additional_tool_names = Vec::new();
    if let Some(ref runner) = options.extension_runner {
        for registered in runner.get_all_registered_tools() {
            additional_tool_names.push(registered.definition.name.clone());
            tool_definitions.retain(|tool| tool.name != registered.definition.name);
            tool_definitions.push(registered.definition);
        }
    }
    for custom in options.custom_tools.clone().unwrap_or_default() {
        additional_tool_names.push(custom.name.clone());
        tool_definitions.retain(|tool| tool.name != custom.name);
        tool_definitions.push(custom);
    }
    additional_tool_names.sort();
    additional_tool_names.dedup();

    let default_active_tool_names: Vec<String> = crate::core::tools::default_active_tool_names();

    let excluded_tool_name_set: Option<std::collections::HashSet<String>> = options
        .exclude_tools
        .as_ref()
        .map(|v| v.iter().cloned().collect());

    let initial_active_tool_names: Vec<String> = match &options.tools {
        Some(tools) => tools
            .iter()
            .filter(|name| {
                !excluded_tool_name_set
                    .as_ref()
                    .map_or(false, |set| set.contains(*name))
            })
            .cloned()
            .collect(),
        None => {
            let mut names = match options.no_tools {
                Some(NoToolsMode::All) => vec![],
                Some(NoToolsMode::Builtin) => additional_tool_names,
                None => {
                    let mut names = default_active_tool_names;
                    names.extend(additional_tool_names);
                    names
                }
            };
            names.retain(|name| {
                !excluded_tool_name_set
                    .as_ref()
                    .map_or(false, |set| set.contains(name))
            });
            names
        }
    };

    // ── Build system prompt ──────────────────────────────────────────────────
    let context_files = resource_loader
        .get_agents_files()
        .iter()
        .map(|file| crate::core::system_prompt::ContextFile {
            path: file.path.clone(),
            content: file.content.clone(),
        })
        .collect();
    let skills = resource_loader
        .get_skills()
        .0
        .iter()
        .map(|skill| crate::core::system_prompt::Skill {
            name: skill.name.clone(),
            description: skill.description.clone(),
            file_path: skill.file_path.to_string_lossy().to_string(),
            disable_model_invocation: skill.disable_model_invocation,
        })
        .collect();
    let append_system_prompt = match resource_loader.get_append_system_prompt() {
        [] => None,
        parts => Some(parts.join("\n\n")),
    };
    let active_tool_name_set: std::collections::HashSet<&str> = initial_active_tool_names
        .iter()
        .map(String::as_str)
        .collect();
    let tool_snippets = tool_definitions
        .iter()
        .filter(|tool| active_tool_name_set.contains(tool.name.as_str()))
        .filter_map(|tool| {
            tool.prompt_snippet
                .as_ref()
                .map(|snippet| (tool.name.clone(), snippet.clone()))
        })
        .collect();
    let prompt_guidelines = tool_definitions
        .iter()
        .filter(|tool| active_tool_name_set.contains(tool.name.as_str()))
        .flat_map(|tool| tool.prompt_guidelines.clone().unwrap_or_default())
        .collect();
    let base_system_prompt_options = crate::core::system_prompt::BuildSystemPromptOptions {
        custom_prompt: resource_loader.get_system_prompt().map(str::to_string),
        selected_tools: Some(initial_active_tool_names.clone()),
        tool_snippets: Some(tool_snippets),
        prompt_guidelines: Some(prompt_guidelines),
        append_system_prompt,
        cwd: cwd_str.clone(),
        context_files: Some(context_files),
        skills: Some(skills),
        ..Default::default()
    };
    let system_prompt =
        crate::core::system_prompt::build_system_prompt(&base_system_prompt_options);

    // ── Load extensions ────────────────────────────────────────────────────
    let extensions_result = crate::core::extensions::types::LoadExtensionsResult {
        extensions: vec![],
        errors: vec![],
    };

    // ── ExtensionRunner: use provided or build from loaded extensions ────
    //
    // Create shared state before the runner so both the context closures
    // and the AgentSession can reference the same atomic/mutex fields.
    let shared_state = crate::core::agent_session::AgentSharedState::new();
    shared_state.set_system_prompt(system_prompt.clone());
    if let Some(ref m) = model {
        if let Ok(json) = serde_json::to_value(m) {
            shared_state.set_model(Some(json));
        }
    }

    let mut extension_runner = options.extension_runner;
    if let Some(ref mut runner) = extension_runner {
        let context_actions = build_extension_context_actions(&shared_state);
        runner.bind_core(
            crate::core::extensions::types::ExtensionActions::default(),
            context_actions,
        );
    }
    let extension_context = extension_runner
        .as_ref()
        .map(crate::core::extensions::runner::ExtensionRunner::create_context);

    // ── Create Agent with stream_fn ────────────────────────────────────────
    let agent = create_agent_with_tools(
        &system_prompt,
        model.clone(),
        Some(thinking_level),
        &initial_active_tool_names,
        &tool_definitions,
        extension_context,
        Arc::clone(&model_registry),
    );
    let restored_messages: Vec<hamr_harness::types::AgentMessage> = existing_session
        .messages
        .into_iter()
        .filter_map(|message| hamr_harness::types::agent_message_from_value(message).ok())
        .collect();
    if !restored_messages.is_empty() {
        agent.set_messages(restored_messages).await;
    }

    // ── Save initial model and thinking level for new sessions ────────────
    if !has_existing_session {
        if let Some(ref m) = model {
            session_manager.append_model_change(&m.provider, &m.id);
        }
        session_manager
            .append_thinking_level_change(&format!("{:?}", thinking_level).to_lowercase());
    }

    // ── Create AgentSession ────────────────────────────────────────────────
    let session = crate::core::agent_session::AgentSession::new(
        crate::core::agent_session::AgentSessionConfig {
            agent,
            session_manager,
            cwd: cwd_str,
            model,
            base_system_prompt: Some(system_prompt),
            base_system_prompt_options: Some(base_system_prompt_options),
            prompt_templates,
            extension_runner,
            shared_state: Some(shared_state.clone()),
            max_retry_attempts: 3,
        },
    );

    // Runner moved into session — return None for the standalone field.
    CreateAgentSessionResult {
        session,
        extensions_result,
        extension_runner: None,
        model_fallback_message,
    }
}

// ---------------------------------------------------------------------------
// Helper: build ExtensionContextActions from AgentSharedState
// ---------------------------------------------------------------------------

/// Build real [`ExtensionContextActions`] closures backed by [`AgentSharedState`].
///
/// Each closure captures a clone of the `Arc<AgentSharedState>` so that
/// the extension runner can query live agent state without a direct
/// reference to [`AgentSession`].
fn build_extension_context_actions(
    state: &Arc<crate::core::agent_session::AgentSharedState>,
) -> crate::core::extensions::types::ExtensionContextActions {
    use std::sync::atomic::Ordering;

    let s = state.clone();
    let get_model = Arc::new(move || s.model.lock().ok()?.clone());

    let s = state.clone();
    let is_idle = Arc::new(move || s.is_idle.load(Ordering::SeqCst));

    let s = state.clone();
    let abort = Arc::new(move || {
        s.abort_requested.store(true, Ordering::SeqCst);
    });

    let s = state.clone();
    let has_pending_messages = Arc::new(move || s.has_pending_messages.load(Ordering::SeqCst));

    let s = state.clone();
    let shutdown = Arc::new(move || {
        s.abort_requested.store(true, Ordering::SeqCst);
    });

    let s = state.clone();
    let get_context_usage = Arc::new(move || s.context_usage.lock().ok()?.clone());

    let s = state.clone();
    let get_system_prompt = Arc::new(move || {
        s.system_prompt
            .lock()
            .map(|g| g.clone())
            .unwrap_or_default()
    });

    crate::core::extensions::types::ExtensionContextActions {
        get_model,
        is_idle,
        is_project_trusted: Arc::new(|| true),
        abort,
        has_pending_messages,
        shutdown,
        get_context_usage,
        compact: Arc::new(|_| {}),
        get_system_prompt,
        get_system_prompt_options: None,
    }
}

// ---------------------------------------------------------------------------
// Helper: create Agent with tools and stream_fn
// ---------------------------------------------------------------------------

/// Create an [`hamr_harness::agent::Agent`] with the built-in tools,
/// system prompt, model, and a stream_fn that resolves API keys from the
/// environment via `hamr_ai::stream::stream_simple`.
///
/// Mirrors the TS `new Agent({ ... })` call in `createAgentSession()`.
fn create_agent_with_tools(
    system_prompt: &str,
    model: Option<hamr_ai::types::Model>,
    thinking_level: Option<hamr_ai::types::ThinkingLevel>,
    active_tool_names: &[String],
    all_definitions: &[crate::core::extensions::types::ToolDefinition],
    extension_context: Option<Arc<dyn crate::core::extensions::types::ExtensionContext>>,
    model_registry: Arc<crate::core::model_registry::ModelRegistry>,
) -> hamr_harness::agent::Agent {
    use hamr_harness::agent::{Agent, AgentOptions};
    use hamr_harness::types::ToolExecutionMode;

    // Build tool definitions and wrap them into AgentTools
    let tools: Vec<hamr_harness::types::AgentTool> = active_tool_names
        .iter()
        .filter_map(|name| {
            all_definitions
                .iter()
                .find(|definition| definition.name == *name)
                .map(|definition| {
                    match extension_context.clone() {
                        Some(context) => crate::core::tools::tool_definition_wrapper::
                            wrap_tool_definition_with_context(definition.clone(), context),
                        None => crate::core::tools::tool_definition_wrapper::wrap_tool_definition(
                            definition.clone(),
                        ),
                    }
                })
        })
        .collect();

    // Use the default model if none provided
    let model = model.unwrap_or_else(|| {
        hamr_ai::models::get_model("anthropic", "claude-sonnet-4-20250514").unwrap_or_else(|| {
            hamr_ai::types::Model {
                id: "unknown".to_string(),
                name: "unknown".to_string(),
                api: hamr_ai::types::Api::AnthropicMessages,
                provider: "anthropic".to_string(),
                base_url: String::new(),
                reasoning: false,
                thinking_level_map: None,
                input: vec![],
                cost: hamr_ai::types::ModelCost {
                    input: 0.0,
                    output: 0.0,
                    cache_read: 0.0,
                    cache_write: 0.0,
                },
                context_window: 200000,
                max_tokens: 8192,
                headers: None,
            compat: None,
            }
        })
    });

    // stream_fn: resolves API key from env, calls stream_simple
    let stream_fn: hamr_harness::types::StreamFn = std::sync::Arc::new(
        |model: hamr_ai::types::Model,
         context: hamr_ai::types::Context,
         options: Option<hamr_ai::types::SimpleStreamOptions>| {
            Box::pin(async move {
                // Register built-in API providers on first use
                hamr_ai::providers::register_builtins::register_built_in_api_providers();

                // Resolve API key from environment
                let api_key = options
                    .as_ref()
                    .and_then(|o| o.base.api_key.clone())
                    .or_else(|| hamr_ai::env_api_keys::get_env_api_key(&model.provider, None));

                let mut opts = options.unwrap_or_default();
                opts.base.api_key = api_key;

                hamr_ai::stream::stream_simple(model, context, Some(opts))
            })
        },
    );

    // convert_to_llm: use the coding-agent's convert_to_llm
    let convert_to_llm: std::sync::Arc<
        dyn Fn(
                Vec<hamr_harness::types::AgentMessage>,
            ) -> std::pin::Pin<
                Box<dyn std::future::Future<Output = Vec<hamr_ai::types::Message>> + Send>,
            > + Send
            + Sync,
    > = std::sync::Arc::new(|messages: Vec<hamr_harness::types::AgentMessage>| {
        Box::pin(async move { crate::core::messages::convert_to_llm(&messages) })
            as std::pin::Pin<
                Box<dyn std::future::Future<Output = Vec<hamr_ai::types::Message>> + Send>,
            >
    });

    let api_key_registry = Arc::clone(&model_registry);
    let get_api_key = Arc::new(move |provider: String| {
        let api_key_registry = Arc::clone(&api_key_registry);
        Box::pin(async move { api_key_registry.get_api_key_for_provider(&provider).await })
            as std::pin::Pin<Box<dyn std::future::Future<Output = Option<String>> + Send>>
    });

    Agent::new(AgentOptions {
        system_prompt: system_prompt.to_string(),
        model,
        thinking_level,
        tools,
        stream_fn: Some(stream_fn),
        convert_to_llm: Some(convert_to_llm),
        get_api_key: Some(get_api_key),
        session_id: None,
        tool_execution: ToolExecutionMode::Parallel,
        transport: None,
        max_retry_delay_ms: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use hamr_ai::types::{MessageContent, MessageRole, TextContent, UserMessage};
    use hamr_harness::types::{AgentMessage, AgentToolResult};
    use std::fs;
    use tempfile::TempDir;

    fn custom_tool(name: &str) -> SdkToolDefinition {
        SdkToolDefinition {
            name: name.to_string(),
            label: "Custom".to_string(),
            description: "A custom SDK tool".to_string(),
            prompt_snippet: Some("Run the custom SDK action".to_string()),
            prompt_guidelines: Some(vec!["Use custom_action for custom work.".to_string()]),
            parameters: serde_json::json!({"type": "object"}),
            render_shell: None,
            prepare_arguments: None,
            execution_mode: None,
            execute: Arc::new(|_id, _params, _signal, _on_update, _context| {
                Box::pin(async move {
                    AgentToolResult {
                        content: vec![MessageContent::Text(TextContent {
                            text: "done".to_string(),
                            text_signature: None,
                        })],
                        details: None,
                        is_error: false,
                        terminate: false,
                    }
                })
            }),
        }
    }

    #[tokio::test]
    async fn create_session_loads_context_and_skills_into_system_prompt() {
        let cwd = TempDir::new().unwrap();
        let agent_dir = TempDir::new().unwrap();
        fs::write(cwd.path().join("AGENTS.md"), "Always verify the release.").unwrap();
        let skill_dir = cwd.path().join(".hamr/skills/release-check");
        fs::create_dir_all(&skill_dir).unwrap();
        fs::write(
            skill_dir.join("SKILL.md"),
            "---\nname: release-check\ndescription: Verify release parity\n---\n",
        )
        .unwrap();
        let prompt_dir = cwd.path().join(".hamr/prompts");
        fs::create_dir_all(&prompt_dir).unwrap();
        fs::write(
            prompt_dir.join("release.md"),
            "---\ndescription: Prepare a release\n---\nRelease $1",
        )
        .unwrap();

        let result = create_agent_session(CreateAgentSessionOptions {
            cwd: Some(cwd.path().to_path_buf()),
            agent_dir: Some(agent_dir.path().to_path_buf()),
            model: hamr_ai::models::get_model("anthropic", "claude-sonnet-4-20250514"),
            no_tools: Some(NoToolsMode::All),
            session_manager: Some(SessionManager::in_memory(Some(
                cwd.path().to_string_lossy().as_ref(),
            ))),
            ..Default::default()
        })
        .await;

        assert!(
            result
                .session
                .context_files()
                .iter()
                .any(|file| file.path.ends_with("AGENTS.md"))
        );
        assert!(
            result
                .session
                .skills()
                .iter()
                .any(|skill| skill.name == "release-check")
        );
        assert!(
            result
                .session
                .prompt_templates()
                .iter()
                .any(|template| template.name == "release")
        );
        let prompt = result.session.base_system_prompt();
        assert!(prompt.contains("Always verify the release."));
        assert!(prompt.contains("<name>release-check</name>"));
    }

    #[tokio::test]
    async fn no_resource_flags_remove_defaults_from_session() {
        let cwd = TempDir::new().unwrap();
        let agent_dir = TempDir::new().unwrap();
        fs::write(cwd.path().join("AGENTS.md"), "Do not load this.").unwrap();

        let result = create_agent_session(CreateAgentSessionOptions {
            cwd: Some(cwd.path().to_path_buf()),
            agent_dir: Some(agent_dir.path().to_path_buf()),
            model: hamr_ai::models::get_model("anthropic", "claude-sonnet-4-20250514"),
            no_tools: Some(NoToolsMode::All),
            no_skills: true,
            no_context_files: true,
            no_prompt_templates: true,
            session_manager: Some(SessionManager::in_memory(Some(
                cwd.path().to_string_lossy().as_ref(),
            ))),
            ..Default::default()
        })
        .await;

        assert!(result.session.context_files().is_empty());
        assert!(result.session.skills().is_empty());
        assert!(result.session.prompt_templates().is_empty());
        assert!(
            !result
                .session
                .base_system_prompt()
                .contains("Do not load this.")
        );
    }

    #[tokio::test]
    async fn custom_tools_are_active_and_no_builtin_tools_keeps_them() {
        let cwd = TempDir::new().unwrap();
        let agent_dir = TempDir::new().unwrap();
        let result = create_agent_session(CreateAgentSessionOptions {
            cwd: Some(cwd.path().to_path_buf()),
            agent_dir: Some(agent_dir.path().to_path_buf()),
            model: hamr_ai::models::get_model("anthropic", "claude-sonnet-4-20250514"),
            no_tools: Some(NoToolsMode::Builtin),
            custom_tools: Some(vec![custom_tool("custom_action")]),
            no_skills: true,
            no_context_files: true,
            no_prompt_templates: true,
            session_manager: Some(SessionManager::in_memory(Some(
                cwd.path().to_string_lossy().as_ref(),
            ))),
            ..Default::default()
        })
        .await;

        let state = result.session.state().await;
        assert_eq!(state.tools, vec!["custom_action"]);
        assert!(
            result
                .session
                .base_system_prompt()
                .contains("custom_action: Run the custom SDK action")
        );
        assert!(
            result
                .session
                .base_system_prompt()
                .contains("Use custom_action for custom work.")
        );
    }

    #[tokio::test]
    async fn existing_session_restores_model_thinking_and_messages() {
        let cwd = TempDir::new().unwrap();
        let agent_dir = TempDir::new().unwrap();
        let mut session_manager =
            SessionManager::in_memory(Some(cwd.path().to_string_lossy().as_ref()));
        session_manager.append_model_change("anthropic", "claude-sonnet-4-20250514");
        session_manager.append_thinking_level_change("high");
        let message = AgentMessage::User(UserMessage {
            role: MessageRole::User,
            content: vec![MessageContent::Text(TextContent {
                text: "remember this".to_string(),
                text_signature: None,
            })],
            timestamp: Utc::now(),
        });
        session_manager.append_message(&serde_json::to_value(message).unwrap());
        let entry_count = session_manager.get_entries().len();

        let result = create_agent_session(CreateAgentSessionOptions {
            cwd: Some(cwd.path().to_path_buf()),
            agent_dir: Some(agent_dir.path().to_path_buf()),
            no_tools: Some(NoToolsMode::All),
            no_skills: true,
            no_context_files: true,
            no_prompt_templates: true,
            session_manager: Some(session_manager),
            ..Default::default()
        })
        .await;

        let state = result.session.state().await;
        assert_eq!(state.model.id, "claude-sonnet-4-20250514");
        assert_eq!(state.thinking_level, hamr_ai::types::ThinkingLevel::High);
        assert_eq!(state.messages.len(), 1);
        assert_eq!(
            result.session.session_manager().get_entries().len(),
            entry_count,
            "restoring must not append duplicate model/thinking entries"
        );
    }
}
