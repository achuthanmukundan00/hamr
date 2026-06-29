//! Integration tests for ported modules.
//!
//! Tests that need external infrastructure (CLI binary, HTTP mocks, TUI harness)
//! remain as documented placeholders at the bottom.

// ── Session manager ──────────────────────────────────────────────────────

#[test]
fn session_manager_in_memory_starts_empty() {
    use hamr_agent::core::session_manager::SessionManager;
    let mgr = SessionManager::in_memory(None);
    assert!(mgr.get_entries().is_empty());
}

#[test]
fn session_manager_creates_with_cwd() {
    use hamr_agent::core::session_manager::SessionManager;
    let mgr = SessionManager::create(".", None);
    let _ = mgr;
}

// ── Settings manager ─────────────────────────────────────────────────────

#[test]
fn settings_manager_creates_with_defaults() {
    use hamr_agent::core::settings_manager::{SettingsManager, SettingsManagerCreateOptions};
    let cwd = std::env::current_dir()
        .unwrap()
        .to_string_lossy()
        .to_string();
    let _mgr = SettingsManager::create(".", &cwd, SettingsManagerCreateOptions::default());
}

// ── Tools ────────────────────────────────────────────────────────────────

#[test]
fn tool_definitions_create_all_non_empty() {
    let cwd = std::env::current_dir().unwrap();
    let defs = hamr_agent::core::tools::create_all_tool_definitions(&cwd);
    assert!(!defs.is_empty(), "Should have built-in tool definitions");
}

#[test]
fn default_active_tool_names_non_empty() {
    let names = hamr_agent::core::tools::default_active_tool_names();
    assert!(!names.is_empty());
}

// ── AgentSession ─────────────────────────────────────────────────────────

#[test]
fn agent_shared_state_new() {
    use hamr_agent::core::agent_session::AgentSharedState;
    let state = AgentSharedState::new();
    state.set_system_prompt("test".into());
    // Shared state created and settable
    let _ = state;
}

#[test]
fn agent_session_creates_minimal() {
    use hamr_agent::core::agent_session::{AgentSession, AgentSessionConfig};
    use hamr_harness::agent::{Agent, AgentOptions};

    let model = hamr_ai::models::get_model("anthropic", "claude-sonnet-4-20250514");
    let model = model.unwrap_or_else(|| hamr_ai::types::Model {
        id: "test".into(),
        name: "test".into(),
        api: hamr_ai::types::Api::AnthropicMessages,
        provider: "anthropic".into(),
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
    });

    let agent = Agent::new(AgentOptions {
        system_prompt: "test".into(),
        model: model.clone(),
        thinking_level: None,
        tools: vec![],
        stream_fn: None,
        convert_to_llm: None,
        get_api_key: None,
        session_id: None,
        tool_execution: hamr_harness::types::ToolExecutionMode::Sequential,
        transport: None,
        max_retry_delay_ms: None,
    });

    let mgr = hamr_agent::core::session_manager::SessionManager::in_memory(None);
    let _session = AgentSession::new(AgentSessionConfig {
        agent,
        session_manager: mgr,
        cwd: ".".into(),
        model: Some(model),
        base_system_prompt: Some("test".into()),
        base_system_prompt_options: None,
        prompt_templates: vec![],
        extension_runner: None,
        shared_state: None,
        max_retry_attempts: 3,
    });
}

// ── Fact store types ─────────────────────────────────────────────────────

#[test]
fn fact_store_types_constructible() {
    use hamr_agent::hamr::memory::fact_store::{FactEntry, FactWithScore};
    let entry = FactEntry {
        fact_id: 1,
        content: "test".into(),
        tags: "".into(),
        trust_score: 0.5,
        retrieval_count: 0,
        helpful_count: 0,
        created_at: "".into(),
        updated_at: "".into(),
    };
    let with_score: FactWithScore = entry.into();
    assert_eq!(with_score.fact_id, 1);
}

// ── Memory DB path resolution ────────────────────────────────────────────

#[test]
fn memory_db_path_returns_default() {
    use hamr_agent::hamr::extensions::memory::memory_db_path;
    let path = memory_db_path(std::path::Path::new("/test"));
    assert!(path.to_string_lossy().contains("memory.sqlite"));
}

// ── Context breakdown ────────────────────────────────────────────────────

#[test]
fn context_breakdown_default_is_empty() {
    use hamr_agent::hamr::extensions::context::ContextBreakdown;
    let b = ContextBreakdown::default();
    assert!(!b.from_api);
    assert!(b.categories.is_empty());
}

// ── Memory types ─────────────────────────────────────────────────────────

#[test]
fn memory_types_constructible() {
    use hamr_agent::hamr::memory::holographic_memory::{MemoryEntry, MemorySearchResult};
    let entry = MemoryEntry {
        session_id: "s1".into(),
        turn_id: 1,
        role: "user".into(),
        tool_name: None,
        file_paths: None,
        content: "test".into(),
        domain_tags: None,
    };
    assert_eq!(entry.content, "test");
    let _result = MemorySearchResult {
        turn_id: 1,
        session_id: "s1".into(),
        role: "user".into(),
        tool_name: None,
        file_paths: None,
        content: "test".into(),
        domain_tags: None,
        rank: 0.0,
    };
}

// ── Entity extraction ────────────────────────────────────────────────────

#[test]
fn extract_entities_from_quoted() {
    use hamr_agent::hamr::memory::fact_store::extract_entities;
    let entities = extract_entities(r#"The "Python" language"#);
    assert!(entities.contains(&"Python".to_string()));
}

// ==========================================================================
// Infrastructure-dependent placeholders
// ==========================================================================

#[test]
fn cli_binary_placeholder() {
    // Requires: full CLI binary for integration tests
    // TS source: stdout-cleanliness.test.ts, startup-session-name.test.ts
}

#[test]
fn http_mock_placeholder() {
    // Requires: HTTP mock server for provider integration tests
    // TS source: various provider e2e tests
}

#[test]
fn tui_harness_placeholder() {
    // Requires: sexy-tui-rs VirtualTerminal wired into interactive mode
    // TS source: various TUI component tests
}

#[test]
fn migration_runner_placeholder() {
    // Requires: migration runner implementation
    // TS source: migration integration tests
}

#[test]
fn extension_runner_placeholder() {
    // Requires: Node.js/Rhai extension runtime for full integration
    // TS source: extension runner tests
}
