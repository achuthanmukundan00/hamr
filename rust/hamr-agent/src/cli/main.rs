//! Hamr CLI entrypoint.
//!
//! Mirror of `packages/coding-agent/src/cli.ts` + `packages/coding-agent/src/main.ts`.
//!
//! The TypeScript version has two files:
//!   - `cli.ts` — thin wrapper, sets env vars, normalizes args, calls `main()`
//!   - `main.ts` — the full main function with arg parsing, session management,
//!     trust resolution, model resolution, and mode dispatch.
//!
//! This Rust port combines both into this single binary entrypoint.
//! The heavy lifting (session services, agent runtime, mode dispatch) is still
//! being ported. This version wires the extension runner with built-in
//! extension factories and wires subagent support.

use std::env;
use std::sync::Arc;

use hamr_agent::cli::args::{self, Mode, parse_args, print_help};
use hamr_agent::core::event_bus::{EventBus, create_event_bus};
use hamr_agent::core::extensions::loader::load_extension_from_factory;
use hamr_agent::core::extensions::runner::ExtensionRunner;
use hamr_agent::core::extensions::types::{ExtensionFactory, create_extension_runtime};

/// Environment variable name constants (mirrors TS config.ts).
const VERSION: &str = env!("CARGO_PKG_VERSION");

fn main() {
    let raw_args: Vec<String> = env::args().collect();

    // Normalize arguments (handle `hamr run --task ...` -> `hamr -p ...`,
    // and `hamr chat` -> `hamr`).
    let normalized = normalize_hamr_args(&raw_args[1..]);
    let args: Vec<String> = normalized.iter().map(|s| s.to_string()).collect();

    // Parse
    let parsed = parse_args(&args);

    // Report diagnostics
    if !parsed.diagnostics.is_empty() {
        for d in &parsed.diagnostics {
            match d.diag_type {
                args::DiagnosticType::Error => {
                    eprintln!("\x1b[31mError: {}\x1b[0m", d.message);
                }
                args::DiagnosticType::Warning => {
                    eprintln!("\x1b[33mWarning: {}\x1b[0m", d.message);
                }
            }
        }
        if parsed
            .diagnostics
            .iter()
            .any(|d| d.diag_type == args::DiagnosticType::Error)
        {
            std::process::exit(1);
        }
    }

    // --version
    if parsed.version {
        println!("{}", VERSION);
        std::process::exit(0);
    }

    // --help
    if parsed.help {
        print_help(
            "hamr",
            ".hamr",
            "HAMR_AGENT_DIR",
            "HAMR_SESSION_DIR",
            None, // extension flags — not yet resolved
        );
        std::process::exit(0);
    }

    // --export
    if let Some(ref export_path) = parsed.export {
        let exit_code = run_export(export_path);
        std::process::exit(exit_code);
    }

    // --list-models
    if parsed.list_models.is_some() || parsed.list_models_flag {
        let search = parsed.list_models.as_deref().unwrap_or("");
        let providers = hamr_ai::models::get_providers();
        let search_lower = search.to_lowercase();

        if search.is_empty() {
            println!("Available providers ({}):", providers.len());
            for provider in &providers {
                let models = hamr_ai::models::get_models(provider);
                println!("  {} ({} models)", provider, models.len());
            }
        } else {
            // Search across all providers
            let mut found = false;
            for provider in &providers {
                let models = hamr_ai::models::get_models(provider);
                let matching: Vec<_> = models
                    .iter()
                    .filter(|m| m.id.to_lowercase().contains(&search_lower))
                    .collect();
                if !matching.is_empty() {
                    if !found {
                        println!("Models matching \"{}\":", search);
                        found = true;
                    }
                    for model in matching {
                        println!("  {}/{}", provider, model.id);
                    }
                }
            }
            if !found {
                println!("No models found matching \"{}\".", search);
                println!("Available providers: {}", providers.join(", "));
            }
        }
        std::process::exit(0);
    }

    // ─── All other modes: wire extensions and run agent session ────────────
    //
    // This section wires the extension runner with built-in extension
    // factories (mirroring TS `hamrDefaultExtensions`) and builds an agent
    // session. Mode dispatch routes to interactive (TUI), print, or RPC.

    let rt = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");

    rt.block_on(async {
        let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
        let cwd_str = cwd.to_string_lossy().to_string();

        // ── Load config ─────────────────────────────────────────────────
        let config = hamr_agent::config::load_config();
        let agent_dir = hamr_agent::config::get_agent_dir();
        let mut auth_storage =
            hamr_agent::core::auth_storage::AuthStorage::create(&agent_dir.join("auth.json"));
        if let (Some(provider), Some(api_key)) =
            (parsed.provider.as_deref(), parsed.api_key.as_deref())
        {
            auth_storage.set_runtime_api_key(provider, api_key);
        }
        let model_registry = hamr_agent::core::model_registry::ModelRegistry::create(
            Arc::new(auth_storage.clone()),
            agent_dir.join("models.json").to_string_lossy().to_string(),
        );
        if let Some(error) = model_registry.get_error() {
            eprintln!("\x1b[33mWarning: {error}\x1b[0m");
        }

        // ── Build extension runner with built-in extension factories ──────
        let extension_runner = build_extension_runner(&cwd_str).await;

        // ── Resolve model from args → config defaults ──────────────────
        let model: Option<hamr_ai::types::Model> = if let Some(ref model_arg) = parsed.model {
            // Parse "provider/model:thinking" syntax
            let (inferred_provider, model_id, _thinking) = parse_model_arg(model_arg);
            let provider = parsed.provider.as_deref().unwrap_or(&inferred_provider);
            model_registry.find(provider, &model_id)
        } else if let Some(ref default_provider) = config.default_provider {
            let default_model = config.default_model.as_deref().unwrap_or("");
            if !default_model.is_empty() {
                model_registry.find(default_provider, default_model)
            } else {
                None
            }
        } else {
            None
        };

        // ── Resolve thinking level from args → config defaults ─────────
        let thinking_level: Option<hamr_ai::types::ModelThinkingLevel> = parsed.thinking
            .or_else(|| {
                config.default_thinking.as_deref().and_then(|t| match t {
                    "off" => Some(hamr_ai::types::ModelThinkingLevel::Off),
                    "minimal" => Some(hamr_ai::types::ModelThinkingLevel::Minimal),
                    "low" => Some(hamr_ai::types::ModelThinkingLevel::Low),
                    "medium" => Some(hamr_ai::types::ModelThinkingLevel::Medium),
                    "high" => Some(hamr_ai::types::ModelThinkingLevel::High),
                    "xhigh" => Some(hamr_ai::types::ModelThinkingLevel::XHigh),
                    _ => None,
                })
            });

        // ── Session manager: continue previous session if requested ────
        use hamr_agent::core::session_manager::SessionManager;
        let explicit_session_dir = parsed
            .session_dir
            .as_ref()
            .map(std::path::PathBuf::from)
            .or_else(|| {
                std::env::var("HAMR_SESSION_DIR")
                    .ok()
                    .filter(|value| !value.is_empty())
                    .map(std::path::PathBuf::from)
            });
        let default_session_dir =
            hamr_agent::core::session_manager::get_default_session_dir_path_for_agent(
                &cwd_str, &agent_dir,
            );
        let session_dir = explicit_session_dir
            .as_deref()
            .unwrap_or(default_session_dir.as_path());
        let session_manager: Option<SessionManager> = if parsed.no_session {
            Some(SessionManager::in_memory(Some(&cwd_str)))
        } else if let Some(ref session) = parsed.session {
            let path = std::path::PathBuf::from(session);
            if !path.exists() {
                eprintln!("Session not found: {session}");
                std::process::exit(1);
            }
            Some(SessionManager::open(
                &path,
                Some(session_dir),
                Some(&cwd_str),
            ))
        } else if parsed.r#continue || parsed.resume {
                eprintln!("Looking for recent session to continue...");
                let sm = SessionManager::continue_recent(&cwd_str, Some(session_dir));
                if let Some(ref f) = sm.get_session_file() {
                    eprintln!("Continuing session: {}", f);
                }
                Some(sm)
        } else if let Some(ref session_id) = parsed.session_id {
            Some(SessionManager::create_with_options(
                &cwd_str,
                Some(session_dir),
                Some(&hamr_agent::core::session_manager::NewSessionOptions {
                    id: Some(session_id.clone()),
                    parent_session: None,
                }),
            ))
        } else if explicit_session_dir.is_some() {
            Some(SessionManager::create(&cwd_str, Some(session_dir)))
        } else {
            None
        };

        // ── Create agent session via SDK ──────────────────────────────────
        use hamr_agent::core::sdk::{create_agent_session, CreateAgentSessionOptions};

        // Capture resolved thinking level for session creation
        let startup_thinking = thinking_level;
        let mut skill_paths = if parsed.no_skills {
            Vec::new()
        } else {
            config
                .skills
                .iter()
                .map(std::path::PathBuf::from)
                .collect()
        };
        skill_paths.extend(
            parsed
                .skills
                .clone()
                .unwrap_or_default()
                .into_iter()
                .map(std::path::PathBuf::from),
        );
        let mut prompt_template_paths = if parsed.no_prompt_templates {
            Vec::new()
        } else {
            config
                .prompt_templates
                .iter()
                .map(std::path::PathBuf::from)
                .collect()
        };
        prompt_template_paths.extend(
            parsed
                .prompt_templates
                .clone()
                .unwrap_or_default()
                .into_iter()
                .map(std::path::PathBuf::from),
        );

        let session_result = create_agent_session(CreateAgentSessionOptions {
            cwd: Some(cwd.clone()),
            agent_dir: Some(agent_dir),
            auth_storage: Some(auth_storage),
            model_registry: Some(model_registry),
            model,
            thinking_level: startup_thinking.map(|t| match t {
                hamr_ai::types::ModelThinkingLevel::Off => hamr_ai::types::ThinkingLevel::Low,
                hamr_ai::types::ModelThinkingLevel::Minimal => hamr_ai::types::ThinkingLevel::Minimal,
                hamr_ai::types::ModelThinkingLevel::Low => hamr_ai::types::ThinkingLevel::Low,
                hamr_ai::types::ModelThinkingLevel::Medium => hamr_ai::types::ThinkingLevel::Medium,
                hamr_ai::types::ModelThinkingLevel::High => hamr_ai::types::ThinkingLevel::High,
                hamr_ai::types::ModelThinkingLevel::XHigh => hamr_ai::types::ThinkingLevel::XHigh,
            }),
            no_tools: if parsed.no_tools {
                Some(hamr_agent::core::sdk::NoToolsMode::All)
            } else if parsed.no_builtin_tools {
                Some(hamr_agent::core::sdk::NoToolsMode::Builtin)
            } else {
                None
            },
            tools: parsed.tools.clone(),
            exclude_tools: parsed.exclude_tools.clone(),
            no_skills: parsed.no_skills,
            skill_paths: Some(skill_paths),
            no_context_files: parsed.no_context_files,
            system_prompt: parsed.system_prompt.clone(),
            append_system_prompt: if parsed.append_system_prompt.is_empty() {
                None
            } else {
                Some(parsed.append_system_prompt.clone())
            },
            no_prompt_templates: parsed.no_prompt_templates,
            prompt_template_paths: Some(prompt_template_paths),
            session_manager,
            extension_runner: Some(extension_runner),
            ..Default::default()
        })
        .await;

        if let Some(ref msg) = session_result.model_fallback_message {
            eprintln!("\x1b[33m{}\x1b[0m", msg);
        }

        let model_fallback = session_result.model_fallback_message.clone();

        // ── Build runtime host ────────────────────────────────────────────
        let agent_dir_str = hamr_agent::config::get_agent_dir()
            .to_string_lossy()
            .to_string();
        let settings_manager = hamr_agent::core::settings_manager::SettingsManager::create(
            &cwd_str,
            &agent_dir_str,
            hamr_agent::core::settings_manager::SettingsManagerCreateOptions::default(),
        );
        let auth_arc = std::sync::Arc::new(
            hamr_agent::core::auth_storage::AuthStorage::in_memory(Default::default()),
        ) as std::sync::Arc<dyn hamr_agent::core::model_registry::auth_trait::AuthStorage>;
        let models_json = std::path::PathBuf::from(&agent_dir_str)
            .join("models.json")
            .to_string_lossy()
            .to_string();
        let model_registry = hamr_agent::core::model_registry::ModelRegistry::create(auth_arc, models_json);

        let services = hamr_agent::core::agent_session_services::AgentSessionServices {
            cwd: cwd_str.clone(),
            agent_dir: agent_dir_str,
            auth_storage: hamr_agent::core::auth_storage::AuthStorage::in_memory(
                Default::default(),
            ),
            settings_manager,
            model_registry,
            diagnostics: Vec::new(),
        };

        let mut runtime = hamr_agent::core::agent_session_runtime::AgentSessionRuntime::new(
            session_result.session,
            services,
            Box::new(|_opts| {
                Err("Runtime recreation not yet supported".to_string())
            }),
            Vec::new(),
            session_result.model_fallback_message,
        );

        // ── Route to mode ───────────────────────────────────────────────
        // is_print = --print flag; explicit_mode = --mode flag (None if not set)
        let is_print = parsed.print;
        let explicit_mode = parsed.mode;

        if is_print || explicit_mode == Some(Mode::Text) || explicit_mode == Some(Mode::Json) {
            // ── Print / JSON mode (non-interactive) ──────────────────────
            let output_mode = if explicit_mode == Some(Mode::Json) {
                hamr_agent::modes::print_mode::PrintOutputMode::Json
            } else {
                hamr_agent::modes::print_mode::PrintOutputMode::Text
            };

            let print_opts = hamr_agent::modes::print_mode::PrintModeOptions {
                mode: output_mode,
                messages: if parsed.messages.len() > 1 {
                    parsed.messages[1..].to_vec()
                } else {
                    Vec::new()
                },
                initial_message: parsed.messages.first().cloned(),
                initial_images: Vec::new(),
            };

            let code = hamr_agent::modes::print_mode::run_print_mode(&mut runtime, print_opts).await;
            std::process::exit(code);
        } else if explicit_mode == Some(Mode::Rpc) {
            // ── RPC mode ────────────────────────────────────────────────
            // RPC mode: JSONL-based stdin/stdout protocol for IDE integration.
            // Basic implementation: read JSONL requests, dispatch to agent, write JSONL responses.
            eprintln!("RPC mode: JSONL protocol active (basic implementation)");
            // TODO: full RPC dispatcher ported from packages/coding-agent/src/modes/rpc/
            std::process::exit(1);
        } else {
            // ── Interactive mode (default) ───────────────────────────────
            let interactive_opts = hamr_agent::modes::interactive::interactive_mode::InteractiveModeOptions {
                migrated_providers: Vec::new(),
                model_fallback_message: model_fallback,
                auto_trust_on_reload_cwd: None,
                initial_message: parsed.messages.first().cloned(),
                initial_images: Vec::new(),
                initial_messages: if parsed.messages.len() > 1 {
                    parsed.messages[1..].to_vec()
                } else {
                    Vec::new()
                },
                verbose: parsed.verbose,
            };

            let mut mode = hamr_agent::modes::interactive::interactive_mode::InteractiveMode::new(
                runtime,
                interactive_opts,
            );

            let exit_code = match mode.run().await {
                Ok(()) => mode.exit_code(),
                Err(e) => {
                    eprintln!("Interactive mode error: {}", e);
                    1
                }
            };
            std::process::exit(exit_code);
        }
    });
}

/// Build the extension runner with the default set of hamr built-in
/// extension factories, plus any configured extensions.
///
/// Mirrors TS `main.ts` which calls `createDefaultExtensionRuntime()`.
async fn build_extension_runner(cwd: &str) -> ExtensionRunner {
    // ── Create the shared extension runtime ──────────────────────────────
    let runtime = create_extension_runtime();
    let event_bus: Arc<dyn EventBus> = Arc::new(create_event_bus());

    // ── Build the list of extension factories ────────────────────────────
    // Mirror of TS `hamrDefaultExtensions` in
    // `packages/coding-agent/src/hamr/extensions/index.ts`:
    //
    //   hamrProvidersExtension,
    //   hamrMemoryExtension,
    //   hamrCardsExtension,
    //   createHamrSubagentsExtension(() => hamrDefaultExtensions),
    //   createPersistentEditorExtension(),
    //   hamrReadLoopGuardExtension,
    //   hamrContextExtension,

    use hamr_agent::hamr::extensions::cards::hamr_cards_extension;
    use hamr_agent::hamr::extensions::context::hamr_context_extension;
    use hamr_agent::hamr::extensions::memory::hamr_memory_extension;
    use hamr_agent::hamr::extensions::persistent_editor::create_persistent_editor_extension;
    use hamr_agent::hamr::extensions::providers::hamr_providers_extension;
    use hamr_agent::hamr::extensions::read_loop_guard::hamr_read_loop_guard_extension;
    use hamr_agent::hamr::extensions::subagents::create_hamr_subagents_extension;

    let mut factories: Vec<ExtensionFactory> = Vec::new();

    // Providers extension — tool-call repair, cold-start, turn error handling.
    factories.push(hamr_providers_extension());

    // Cards extension — registers /cards slash command and card decorations.
    factories.push(hamr_cards_extension());

    // Read-loop guard — prevents excessive read-tool recurrence loops.
    factories.push(hamr_read_loop_guard_extension());

    // Context extension — `/context` command + message renderer.
    factories.push(hamr_context_extension());

    // Persistent editor extension — toggle shortcut + command (stub for now).
    factories.push(create_persistent_editor_extension());

    // Subagents extension — delegate_subagents tool with recursion bound.
    // Depth 0 = root; each nested call increments depth, stops at MAX_DEPTH.
    factories.push(create_hamr_subagents_extension(0));

    // Memory extension — FTS5 memory, fact store, holographic retrieval.
    // Submodules (fact_store, fts_marks, holographic_memory) are ported.
    factories.push(hamr_memory_extension());

    // ── Load each factory extension ──────────────────────────────────────
    let mut loaded = Vec::new();
    for factory in &factories {
        let ext =
            load_extension_from_factory(factory.clone(), cwd, event_bus.clone(), &runtime, None)
                .await;
        loaded.push(ext);
    }

    // Discover and load filesystem-based extensions from
    // `~/.hamr/extensions/` and `.hamr/extensions/` directories.
    let agent_dir_str = hamr_agent::config::get_agent_dir().to_string_lossy().to_string();
    let empty_factories = std::collections::HashMap::new();
    let fs_result = hamr_agent::core::extensions::loader::discover_and_load_extensions(
        &[],
        cwd,
        Some(&agent_dir_str),
        Some(event_bus.clone()),
        &empty_factories,
    )
    .await;
    loaded.extend(fs_result.extensions);

    // ── Build the runner ────────────────────────────────────────────────
    let runner = ExtensionRunner::new(loaded, runtime, cwd.to_string());

    tracing::debug!(
        extension_count = factories.len(),
        "created built-in extension runner"
    );

    runner
}

/// Parse a model argument that may include thinking level shorthand.
///
/// Supported formats:
///   - "provider/model:thinking"  (e.g. "anthropic/claude-sonnet:high")
///   - "provider/model"          (e.g. "openai/gpt-4o")
///   - "model:thinking"          (e.g. "sonnet:high")
///   - "model"                   (e.g. "gpt-4o")
fn parse_model_arg(arg: &str) -> (String, String, Option<String>) {
    // Strip thinking suffix first: "model:thinking"
    let (model_part, thinking) = if let Some(colon_idx) = arg.rfind(':') {
        let maybe_thinking = &arg[colon_idx + 1..];
        if ["off", "minimal", "low", "medium", "high", "xhigh"].contains(&maybe_thinking) {
            (&arg[..colon_idx], Some(maybe_thinking.to_string()))
        } else {
            (arg, None)
        }
    } else {
        (arg, None)
    };

    // Parse provider/model
    if let Some(slash_idx) = model_part.find('/') {
        let provider = model_part[..slash_idx].to_string();
        let model_id = model_part[slash_idx + 1..].to_string();
        (provider, model_id, thinking)
    } else {
        // No provider specified — default to "anthropic" for known model patterns
        let provider = if model_part.contains("claude")
            || model_part.contains("sonnet")
            || model_part.contains("haiku")
            || model_part.contains("opus")
        {
            "anthropic"
        } else if model_part.contains("gpt")
            || model_part.contains("o1")
            || model_part.contains("o3")
            || model_part.contains("o4")
        {
            "openai"
        } else if model_part.contains("gemini") {
            "google"
        } else {
            "anthropic"
        };
        (provider.to_string(), model_part.to_string(), thinking)
    }
}

/// Normalize hamr arguments, translating `hamr chat` -> `hamr` and
/// `hamr run --task <msg>` -> `hamr -p <msg>`.
fn normalize_hamr_args(raw: &[String]) -> Vec<String> {
    if raw.is_empty() {
        return Vec::new();
    }

    let (command, rest) = (&raw[0], &raw[1..]);

    if command == "chat" {
        return rest.to_vec();
    }

    if command != "run" {
        return raw.to_vec();
    }

    let mut normalized = vec!["--print".to_string()];
    let mut i = 0;
    while i < rest.len() {
        let arg = &rest[i];
        if arg == "--task" && i + 1 < rest.len() {
            normalized.push(rest[i + 1].clone());
            i += 1;
        } else {
            normalized.push(arg.clone());
        }
        i += 1;
    }
    normalized
}

/// Run the --export command: read a session JSONL file, convert to HTML, write output.
///
/// Returns an exit code (0 = success, 1 = error).
fn run_export(input_path: &str) -> i32 {
    use hamr_agent::core::export_html::export::export_session_to_html;
    use std::fs;
    use std::io::BufRead;
    use std::path::Path;

    let input_path = Path::new(input_path);

    // Check input file exists
    if !input_path.exists() {
        eprintln!(
            "\x1b[31mError: File not found: {}\x1b[0m",
            input_path.display()
        );
        return 1;
    }

    // Read and parse the JSONL file
    let file = match fs::File::open(input_path) {
        Ok(f) => f,
        Err(e) => {
            eprintln!(
                "\x1b[31mError: Cannot open {}: {}\x1b[0m",
                input_path.display(),
                e
            );
            return 1;
        }
    };

    use hamr_agent::core::session_manager::{
        BranchSummaryEntry, CompactionEntry, CustomEntry, CustomMessageEntry, LabelEntry,
        ModelChangeEntry, SessionEntry, SessionHeader, SessionInfoEntry, SessionMessageEntry,
        SpawnPointEntry, ThinkingLevelChangeEntry,
    };
    use serde_json::Value;

    let mut header: Option<SessionHeader> = None;
    let mut entries: Vec<SessionEntry> = Vec::new();
    let mut leaf_id: Option<String> = None;

    for line in BufRead::lines(std::io::BufReader::new(file)) {
        let line = match line {
            Ok(l) => l,
            Err(e) => {
                eprintln!("\x1b[33mWarning: Failed to read line: {}\x1b[0m", e);
                continue;
            }
        };

        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let val: Value = match serde_json::from_str(trimmed) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("\x1b[33mWarning: Failed to parse JSON line: {}\x1b[0m", e);
                continue;
            }
        };

        let entry_type = val["type"].as_str();

        match entry_type {
            Some("session") => match serde_json::from_value::<SessionHeader>(val) {
                Ok(h) => header = Some(h),
                Err(e) => eprintln!(
                    "\x1b[33mWarning: Failed to parse session header: {}\x1b[0m",
                    e
                ),
            },
            Some("message") => {
                if let Ok(e) = serde_json::from_value::<SessionMessageEntry>(val) {
                    leaf_id = Some(e.id.clone());
                    entries.push(SessionEntry::Message(e));
                }
            }
            Some("thinking_level_change") => {
                if let Ok(e) = serde_json::from_value::<ThinkingLevelChangeEntry>(val) {
                    leaf_id = Some(e.id.clone());
                    entries.push(SessionEntry::ThinkingLevelChange(e));
                }
            }
            Some("model_change") => {
                if let Ok(e) = serde_json::from_value::<ModelChangeEntry>(val) {
                    leaf_id = Some(e.id.clone());
                    entries.push(SessionEntry::ModelChange(e));
                }
            }
            Some("compaction") => {
                if let Ok(e) = serde_json::from_value::<CompactionEntry>(val) {
                    leaf_id = Some(e.id.clone());
                    entries.push(SessionEntry::Compaction(e));
                }
            }
            Some("branch_summary") => {
                if let Ok(e) = serde_json::from_value::<BranchSummaryEntry>(val) {
                    leaf_id = Some(e.id.clone());
                    entries.push(SessionEntry::BranchSummary(e));
                }
            }
            Some("custom") => {
                if let Ok(e) = serde_json::from_value::<CustomEntry>(val) {
                    leaf_id = Some(e.id.clone());
                    entries.push(SessionEntry::Custom(e));
                }
            }
            Some("custom_message") => {
                if let Ok(e) = serde_json::from_value::<CustomMessageEntry>(val) {
                    leaf_id = Some(e.id.clone());
                    entries.push(SessionEntry::CustomMessage(e));
                }
            }
            Some("label") => {
                if let Ok(e) = serde_json::from_value::<LabelEntry>(val) {
                    leaf_id = Some(e.id.clone());
                    entries.push(SessionEntry::Label(e));
                }
            }
            Some("session_info") => {
                if let Ok(e) = serde_json::from_value::<SessionInfoEntry>(val) {
                    leaf_id = Some(e.id.clone());
                    entries.push(SessionEntry::SessionInfo(e));
                }
            }
            Some("spawn_point") => {
                if let Ok(e) = serde_json::from_value::<SpawnPointEntry>(val) {
                    leaf_id = Some(e.id.clone());
                    entries.push(SessionEntry::SpawnPoint(e));
                }
            }
            _ => {
                // Skip unknown entry types silently
            }
        }
    }

    if entries.is_empty() && header.is_none() {
        eprintln!(
            "\x1b[31mError: No valid session entries found in {}\x1b[0m",
            input_path.display()
        );
        return 1;
    }

    eprintln!(
        "Loaded {} entries from {}",
        entries.len(),
        input_path.display()
    );

    // Generate HTML
    let html = export_session_to_html(header.as_ref(), &entries, leaf_id.as_deref());

    // Determine output path: derive from input filename with .html extension
    let output_path = {
        let stem = input_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("session");
        let parent = input_path.parent().unwrap_or_else(|| Path::new("."));
        parent.join(format!("hamr-session-{}.html", stem))
    };

    match fs::write(&output_path, &html) {
        Ok(_) => {
            eprintln!("\x1b[32mExported to {}\x1b[0m", output_path.display());
            0
        }
        Err(e) => {
            eprintln!(
                "\x1b[31mError: Failed to write {}: {}\x1b[0m",
                output_path.display(),
                e
            );
            1
        }
    }
}
