//! Cwd-bound runtime services and AgentSession creation.
//!
//! Ported from `packages/coding-agent/src/core/agent-session-services.ts`.
//!
//! Provides:
//! - `create_agent_session_services`: create cwd-bound runtime services
//! - `create_agent_session_from_services`: create an AgentSession from
//!   already-created services, resolving model, thinking level, tools, and
//!   session options against those services.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::core::auth_storage::AuthStorage;
use crate::core::model_registry::ModelRegistry;
use crate::core::session_manager::SessionManager;
use crate::core::settings_manager::{SettingsManager, SettingsManagerCreateOptions};

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// Non-fatal issues collected while creating services or sessions.
#[derive(Debug, Clone)]
pub struct AgentSessionRuntimeDiagnostic {
    pub diagnostic_type: String,
    pub message: String,
}

/// Inputs for creating cwd-bound runtime services.
pub struct CreateAgentSessionServicesOptions {
    pub cwd: String,
    pub agent_dir: Option<String>,
    pub auth_storage: Option<AuthStorage>,
    pub settings_manager: Option<SettingsManager>,
    pub model_registry: Option<ModelRegistry>,
    pub extension_flag_values: Option<std::collections::HashMap<String, String>>,
}

/// Inputs for creating an AgentSession from already-created services.
pub struct CreateAgentSessionFromServicesOptions {
    pub services: AgentSessionServices,
    pub session_manager: SessionManager,
    pub model: Option<hamr_ai::types::Model>,
    pub thinking_level: Option<hamr_ai::types::ThinkingLevel>,
    pub tools: Option<Vec<String>>,
    pub exclude_tools: Option<Vec<String>>,
    pub no_tools: bool,
    pub custom_tools: Option<Vec<crate::core::extensions::types::ToolDefinition>>,
    pub session_start_event: Option<crate::core::extensions::types::SessionBeforeResult>,
}

/// Coherent cwd-bound runtime services for one effective session cwd.
pub struct AgentSessionServices {
    pub cwd: String,
    pub agent_dir: String,
    pub auth_storage: AuthStorage,
    pub settings_manager: SettingsManager,
    pub model_registry: ModelRegistry,
    pub diagnostics: Vec<AgentSessionRuntimeDiagnostic>,
}

impl AgentSessionServices {
    /// Create a stub instance for tests.
    pub fn for_test() -> Self {
        let auth = AuthStorage::in_memory(Default::default());
        let auth_arc = std::sync::Arc::new(AuthStorage::in_memory(Default::default()))
            as std::sync::Arc<dyn crate::core::model_registry::auth_trait::AuthStorage>;
        Self {
            cwd: "/tmp".to_string(),
            agent_dir: "/tmp/.hamr".to_string(),
            auth_storage: auth,
            settings_manager: SettingsManager::in_memory(None),
            model_registry: ModelRegistry::in_memory(auth_arc),
            diagnostics: Vec::new(),
        }
    }
}

/// Result of creating an AgentSession.
pub struct CreateAgentSessionFromServicesResult {
    pub session: crate::core::agent_session::AgentSession,
    pub diagnostics: Vec<AgentSessionRuntimeDiagnostic>,
    pub model_fallback_message: Option<String>,
}

// ---------------------------------------------------------------------------
// Factory functions
// ---------------------------------------------------------------------------

/// Get the agent directory (defaults to ~/.config/hamr/agent).
fn get_agent_dir() -> String {
    crate::core::settings_manager::get_agent_dir()
}

/// Resolve a path to an absolute path.
fn resolve_path(input: &str) -> String {
    Path::new(input)
        .canonicalize()
        .unwrap_or_else(|_| PathBuf::from(input))
        .to_string_lossy()
        .to_string()
}

/// Create cwd-bound runtime services.
///
/// Creates SettingsManager, ModelRegistry, and AuthStorage from the provided
/// options (or defaults). Registers any pending providers from extensions.
pub fn create_agent_session_services(
    options: CreateAgentSessionServicesOptions,
) -> AgentSessionServices {
    let cwd = resolve_path(&options.cwd);
    let agent_dir = options
        .agent_dir
        .map(|d| resolve_path(&d))
        .unwrap_or_else(get_agent_dir);

    let auth_storage = options.auth_storage.unwrap_or_else(|| {
        AuthStorage::create(&PathBuf::from(&agent_dir).join("auth.json"))
    });

    let settings_manager = options.settings_manager.unwrap_or_else(|| {
        SettingsManager::create(
            &cwd,
            &agent_dir,
            SettingsManagerCreateOptions::default(),
        )
    });

    let auth_storage_arc: Arc<dyn crate::core::model_registry::auth_trait::AuthStorage> =
        Arc::new(AuthStorage::create(
            &PathBuf::from(&agent_dir).join("auth.json"),
        ));

    let model_registry = options.model_registry.unwrap_or_else(|| {
        let models_json = PathBuf::from(&agent_dir)
            .join("models.json")
            .to_string_lossy()
            .to_string();
        ModelRegistry::create(auth_storage_arc, models_json)
    });

    let diagnostics = Vec::new();

    // TODO: load extensions via DefaultResourceLoader, register pending
    // providers from extensions, apply extension flag values.
    // Requires DefaultResourceLoader integration.

    AgentSessionServices {
        cwd,
        agent_dir,
        auth_storage,
        settings_manager,
        model_registry,
        diagnostics,
    }
}

/// Create an AgentSession from previously created services.
///
/// Delegates to [`crate::core::sdk::create_agent_session`] with options
/// resolved from the provided services.
pub async fn create_agent_session_from_services(
    options: CreateAgentSessionFromServicesOptions,
) -> CreateAgentSessionFromServicesResult {
    let sdk_result =
        crate::core::sdk::create_agent_session(crate::core::sdk::CreateAgentSessionOptions {
            cwd: Some(PathBuf::from(&options.services.cwd)),
            agent_dir: Some(PathBuf::from(&options.services.agent_dir)),
            auth_storage: Some(options.services.auth_storage),
            model_registry: Some(options.services.model_registry),
            model: options.model,
            thinking_level: options.thinking_level,
            scoped_models: None,
            no_tools: if options.no_tools {
                Some(crate::core::sdk::NoToolsMode::All)
            } else {
                None
            },
            tools: options.tools,
            exclude_tools: options.exclude_tools,
            custom_tools: options.custom_tools,
            session_manager: Some(options.session_manager),
            session_start_event: options.session_start_event,
            ..Default::default()
        })
        .await;

    CreateAgentSessionFromServicesResult {
        session: sdk_result.session,
        diagnostics: vec![],
        model_fallback_message: sdk_result.model_fallback_message,
    }
}
