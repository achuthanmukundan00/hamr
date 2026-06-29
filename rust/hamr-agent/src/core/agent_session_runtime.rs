//! AgentSession runtime — owns the current AgentSession and its cwd-bound services.
//!
//! Session replacement methods tear down the current runtime first, then create
//! and apply the next runtime.
//!
//! Ported from `packages/coding-agent/src/core/agent-session-runtime.ts`.

use std::path::Path;

use crate::core::agent_session_services::{AgentSessionRuntimeDiagnostic, AgentSessionServices};
use crate::core::session_cwd::assert_session_cwd_exists;
use crate::core::session_manager::SessionManager;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// Result returned by runtime creation.
pub struct CreateAgentSessionRuntimeResult {
    pub session: crate::core::agent_session::AgentSession,
    pub services: AgentSessionServices,
    pub diagnostics: Vec<AgentSessionRuntimeDiagnostic>,
    pub model_fallback_message: Option<String>,
}

/// Factory closure that creates a runtime for a target cwd and session manager.
pub type CreateAgentSessionRuntimeFactory = Box<
    dyn Fn(CreateAgentSessionRuntimeOptions) -> Result<CreateAgentSessionRuntimeResult, String>
        + Send
        + Sync,
>;

/// Options passed to the runtime factory.
pub struct CreateAgentSessionRuntimeOptions {
    pub cwd: String,
    pub agent_dir: String,
    pub session_manager: SessionManager,
}

/// Thrown when /import references a JSONL file path that does not exist.
#[derive(Debug)]
pub struct SessionImportFileNotFoundError {
    pub file_path: String,
}

impl std::fmt::Display for SessionImportFileNotFoundError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "File not found: {}", self.file_path)
    }
}

impl std::error::Error for SessionImportFileNotFoundError {}

// ---------------------------------------------------------------------------
// AgentSessionRuntime
// ---------------------------------------------------------------------------

/// Owns the current AgentSession plus its cwd-bound services.
pub struct AgentSessionRuntime {
    session: crate::core::agent_session::AgentSession,
    services: AgentSessionServices,
    create_runtime: CreateAgentSessionRuntimeFactory,
    diagnostics: Vec<AgentSessionRuntimeDiagnostic>,
    model_fallback_message: Option<String>,
}

impl AgentSessionRuntime {
    pub fn new(
        session: crate::core::agent_session::AgentSession,
        services: AgentSessionServices,
        create_runtime: CreateAgentSessionRuntimeFactory,
        diagnostics: Vec<AgentSessionRuntimeDiagnostic>,
        model_fallback_message: Option<String>,
    ) -> Self {
        Self {
            session,
            services,
            create_runtime,
            diagnostics,
            model_fallback_message,
        }
    }

    pub fn services(&self) -> &AgentSessionServices {
        &self.services
    }

    pub fn session(&self) -> &crate::core::agent_session::AgentSession {
        &self.session
    }

    pub fn session_mut(&mut self) -> &mut crate::core::agent_session::AgentSession {
        &mut self.session
    }

    pub fn cwd(&self) -> &str {
        &self.services.cwd
    }

    pub fn diagnostics(&self) -> &[AgentSessionRuntimeDiagnostic] {
        &self.diagnostics
    }

    pub fn model_fallback_message(&self) -> Option<&str> {
        self.model_fallback_message.as_deref()
    }

    /// Switch to a different session (resume).
    pub fn switch_session(
        &mut self,
        session_path: &Path,
        cwd_override: Option<&str>,
    ) -> Result<(), String> {
        let session_manager = SessionManager::open(session_path, None, cwd_override);
        assert_session_cwd_exists(
            &SessionCwdAdapter {
                cwd: session_manager.get_cwd(),
                session_file: session_manager.get_session_file(),
            },
            &self.services.cwd,
        )
        .map_err(|e| e.to_string())?;

        // Call the factory to create the new runtime
        let result = (self.create_runtime)(CreateAgentSessionRuntimeOptions {
            cwd: session_manager.get_cwd(),
            agent_dir: self.services.agent_dir.clone(),
            session_manager,
        })?;

        self.services = result.services;
        self.diagnostics = result.diagnostics;
        self.model_fallback_message = result.model_fallback_message;

        Ok(())
    }

    /// Create a new session, preserving the same cwd.
    pub fn new_session(&mut self) -> Result<(), String> {
        let session_dir = self.services.agent_dir.clone(); // simplified
        let session_manager =
            SessionManager::create(&self.services.cwd, Some(Path::new(&session_dir)));

        let result = (self.create_runtime)(CreateAgentSessionRuntimeOptions {
            cwd: self.services.cwd.clone(),
            agent_dir: self.services.agent_dir.clone(),
            session_manager,
        })?;

        self.services = result.services;
        self.diagnostics = result.diagnostics;
        self.model_fallback_message = result.model_fallback_message;

        Ok(())
    }

    /// Import a session JSONL file and switch to it.
    pub fn import_from_jsonl(
        &mut self,
        input_path: &Path,
        cwd_override: Option<&str>,
    ) -> Result<(), String> {
        let resolved = input_path
            .canonicalize()
            .unwrap_or_else(|_| input_path.to_path_buf());

        if !resolved.exists() {
            return Err(SessionImportFileNotFoundError {
                file_path: resolved.to_string_lossy().to_string(),
            }
            .to_string());
        }

        let session_manager = SessionManager::open(&resolved, None, cwd_override);
        assert_session_cwd_exists(
            &SessionCwdAdapter {
                cwd: session_manager.get_cwd(),
                session_file: session_manager.get_session_file(),
            },
            &self.services.cwd,
        )
        .map_err(|e| e.to_string())?;

        let result = (self.create_runtime)(CreateAgentSessionRuntimeOptions {
            cwd: session_manager.get_cwd(),
            agent_dir: self.services.agent_dir.clone(),
            session_manager,
        })?;

        self.services = result.services;
        self.diagnostics = result.diagnostics;
        self.model_fallback_message = result.model_fallback_message;

        Ok(())
    }
}

// ---------------------------------------------------------------------------
// SessionCwd adapter (bridges SessionManager to the session_cwd trait)
// ---------------------------------------------------------------------------

use crate::core::session_cwd::SessionCwdSource;

struct SessionCwdAdapter {
    cwd: String,
    session_file: Option<String>,
}

impl SessionCwdSource for SessionCwdAdapter {
    fn get_cwd(&self) -> String {
        self.cwd.clone()
    }

    fn get_session_file(&self) -> Option<String> {
        self.session_file.clone()
    }
}

// ---------------------------------------------------------------------------
// Factory function
// ---------------------------------------------------------------------------

/// Create the initial runtime from a runtime factory and initial session target.
pub fn create_agent_session_runtime(
    create_runtime: CreateAgentSessionRuntimeFactory,
    cwd: &str,
    agent_dir: &str,
    session_manager: SessionManager,
) -> Result<AgentSessionRuntime, String> {
    assert_session_cwd_exists(
        &SessionCwdAdapter {
            cwd: session_manager.get_cwd(),
            session_file: session_manager.get_session_file(),
        },
        cwd,
    )
    .map_err(|e| e.to_string())?;

    let result = create_runtime(CreateAgentSessionRuntimeOptions {
        cwd: cwd.to_string(),
        agent_dir: agent_dir.to_string(),
        session_manager,
    })?;

    Ok(AgentSessionRuntime::new(
        result.session,
        result.services,
        create_runtime,
        result.diagnostics,
        result.model_fallback_message,
    ))
}
