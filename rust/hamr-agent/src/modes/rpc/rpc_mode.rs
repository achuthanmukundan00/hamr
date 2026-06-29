//! Port of `packages/coding-agent/src/modes/rpc/rpc-mode.ts`
//!
//! RPC mode: Headless operation with JSON stdin/stdout protocol.
//!
//! Used for embedding the agent in other applications.
//! Receives commands as JSON on stdin, outputs events and responses as JSON on stdout.

use crate::modes::rpc::jsonl::JsonlLineReader;

// Re-export key types for consumers
pub use crate::modes::rpc::rpc_types::{
    RpcCommand, RpcExtensionUIRequest, RpcResponse, RpcSessionState,
};

use crate::modes::rpc::rpc_types::RpcExtensionUIResponse;

use serde_json::Value;
use std::collections::HashMap;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::sync::mpsc;

/// Run in RPC mode.
/// Listens for JSON commands on stdin, outputs events and responses on stdout.
///
/// This is a simplified port — the full implementation requires the agent session
/// runtime and extension system to be ported first.
pub async fn run_rpc_mode() -> ! {
    let stdin = tokio::io::stdin();
    let mut reader = BufReader::new(stdin);
    let mut line_reader = JsonlLineReader::new();
    let mut line_buf = String::new();

    let (_output_tx, _output_rx) = mpsc::unbounded_channel::<String>();

    // Pending extension UI requests
    let pending_extension_requests: HashMap<String, mpsc::Sender<Value>> = HashMap::new();

    // Signal handlers — on SIGTERM/SIGHUP, shutdown
    let mut sigterm = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
        .expect("failed to register SIGTERM handler");
    let mut sighup = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::hangup())
        .expect("failed to register SIGHUP handler");

    let shutdown = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let shutdown_clone = shutdown.clone();

    let output_fn = |obj: Value| {
        let line = serde_json::to_string(&obj).unwrap();
        println!("{}", line);
    };

    // Spawn signal handler
    tokio::spawn(async move {
        tokio::select! {
            _ = sigterm.recv() => {
                shutdown_clone.store(true, std::sync::atomic::Ordering::SeqCst);
            }
            _ = sighup.recv() => {
                shutdown_clone.store(true, std::sync::atomic::Ordering::SeqCst);
            }
        }
    });

    // Main loop: read JSONL commands from stdin
    loop {
        if shutdown.load(std::sync::atomic::Ordering::SeqCst) {
            std::process::exit(0);
        }

        line_buf.clear();
        match reader.read_line(&mut line_buf).await {
            Ok(0) => {
                std::process::exit(0);
            }
            Ok(_) => {}
            Err(e) => {
                eprintln!("Error reading stdin: {}", e);
                std::process::exit(1);
            }
        }

        let lines = line_reader.feed(&line_buf);
        for line in lines {
            let parsed: Value = match serde_json::from_str(&line) {
                Ok(v) => v,
                Err(e) => {
                    let resp = RpcResponse::failure(
                        None,
                        "parse",
                        format!("Failed to parse command: {}", e),
                    );
                    output_fn(serde_json::to_value(resp).unwrap());
                    continue;
                }
            };

            // Handle extension UI responses
            if parsed.get("type").and_then(|v| v.as_str()) == Some("extension_ui_response") {
                if let Ok(ui_resp) = serde_json::from_value::<RpcExtensionUIResponse>(parsed) {
                    if let Some(tx) = pending_extension_requests.get(&ui_resp.id) {
                        let _ = tx.send(serde_json::to_value(&ui_resp).unwrap());
                    }
                }
                continue;
            }

            // Handle RPC command
            match serde_json::from_value::<RpcCommand>(parsed) {
                Ok(command) => {
                    let _id = get_command_id(&command);

                    let response = handle_command(&command).await;
                    if let Some(resp) = response {
                        output_fn(serde_json::to_value(resp).unwrap());
                    }
                }
                Err(e) => {
                    output_fn(
                        serde_json::to_value(RpcResponse::failure(
                            None,
                            "parse",
                            format!("Failed to parse command: {}", e),
                        ))
                        .unwrap(),
                    );
                }
            }
        }
    }
}

/// Handle a single RPC command.
async fn handle_command(command: &RpcCommand) -> Option<RpcResponse> {
    let id = get_command_id(command);

    match command {
        // =================================================================
        // Prompting
        // =================================================================
        RpcCommand::Prompt { .. } => Some(RpcResponse::success(id, "prompt", None)),
        RpcCommand::Steer { .. } => Some(RpcResponse::success(id, "steer", None)),
        RpcCommand::FollowUp { .. } => Some(RpcResponse::success(id, "follow_up", None)),
        RpcCommand::Abort { .. } => Some(RpcResponse::success(id, "abort", None)),
        RpcCommand::NewSession { .. } => {
            let data = serde_json::json!({"cancelled": false});
            Some(RpcResponse::success(id, "new_session", Some(data)))
        }

        // =================================================================
        // State
        // =================================================================
        RpcCommand::GetState { .. } => {
            let state = RpcSessionState {
                model: None,
                thinking_level: hamr_ai::types::ThinkingLevel::Medium,
                is_streaming: false,
                is_compacting: false,
                steering_mode: "all".to_string(),
                follow_up_mode: "all".to_string(),
                session_file: None,
                session_id: "stub".to_string(),
                session_name: None,
                auto_compaction_enabled: false,
                message_count: 0,
                pending_message_count: 0,
            };
            Some(RpcResponse::success(
                id,
                "get_state",
                Some(serde_json::to_value(state).unwrap()),
            ))
        }

        // =================================================================
        // Model
        // =================================================================
        RpcCommand::SetModel { .. } => Some(RpcResponse::success(
            id,
            "set_model",
            Some(serde_json::json!({"provider": "", "id": ""})),
        )),
        RpcCommand::CycleModel { .. } => Some(RpcResponse::success(
            id,
            "cycle_model",
            Some(serde_json::Value::Null),
        )),
        RpcCommand::GetAvailableModels { .. } => Some(RpcResponse::success(
            id,
            "get_available_models",
            Some(serde_json::json!({"models": []})),
        )),

        // =================================================================
        // Thinking
        // =================================================================
        RpcCommand::SetThinkingLevel { .. } => {
            Some(RpcResponse::success(id, "set_thinking_level", None))
        }
        RpcCommand::CycleThinkingLevel { .. } => Some(RpcResponse::success(
            id,
            "cycle_thinking_level",
            Some(serde_json::Value::Null),
        )),

        // =================================================================
        // Queue Modes
        // =================================================================
        RpcCommand::SetSteeringMode { .. } => {
            Some(RpcResponse::success(id, "set_steering_mode", None))
        }
        RpcCommand::SetFollowUpMode { .. } => {
            Some(RpcResponse::success(id, "set_follow_up_mode", None))
        }

        // =================================================================
        // Compaction
        // =================================================================
        RpcCommand::Compact { .. } => Some(RpcResponse::success(
            id,
            "compact",
            Some(serde_json::Value::Null),
        )),
        RpcCommand::SetAutoCompaction { .. } => {
            Some(RpcResponse::success(id, "set_auto_compaction", None))
        }

        // =================================================================
        // Retry
        // =================================================================
        RpcCommand::SetAutoRetry { .. } => Some(RpcResponse::success(id, "set_auto_retry", None)),
        RpcCommand::AbortRetry { .. } => Some(RpcResponse::success(id, "abort_retry", None)),

        // =================================================================
        // Bash
        // =================================================================
        RpcCommand::Bash { .. } => Some(RpcResponse::success(
            id,
            "bash",
            Some(serde_json::Value::Null),
        )),
        RpcCommand::AbortBash { .. } => Some(RpcResponse::success(id, "abort_bash", None)),

        // =================================================================
        // Session
        // =================================================================
        RpcCommand::GetSessionStats { .. } => Some(RpcResponse::success(
            id,
            "get_session_stats",
            Some(serde_json::Value::Null),
        )),
        RpcCommand::ExportHtml { .. } => Some(RpcResponse::success(
            id,
            "export_html",
            Some(serde_json::json!({"path": ""})),
        )),
        RpcCommand::SwitchSession { .. } => {
            let data = serde_json::json!({"cancelled": false});
            Some(RpcResponse::success(id, "switch_session", Some(data)))
        }
        RpcCommand::Fork { .. } => {
            let data = serde_json::json!({"text": "", "cancelled": false});
            Some(RpcResponse::success(id, "fork", Some(data)))
        }
        RpcCommand::Clone { .. } => {
            let data = serde_json::json!({"cancelled": false});
            Some(RpcResponse::success(id, "clone", Some(data)))
        }
        RpcCommand::GetForkMessages { .. } => Some(RpcResponse::success(
            id,
            "get_fork_messages",
            Some(serde_json::json!({"messages": []})),
        )),
        RpcCommand::GetLastAssistantText { .. } => Some(RpcResponse::success(
            id,
            "get_last_assistant_text",
            Some(serde_json::json!({"text": null})),
        )),
        RpcCommand::SetSessionName { name, .. } => {
            if name.trim().is_empty() {
                return Some(RpcResponse::failure(
                    id,
                    "set_session_name",
                    "Session name cannot be empty".to_string(),
                ));
            }
            Some(RpcResponse::success(id, "set_session_name", None))
        }

        // =================================================================
        // Messages
        // =================================================================
        RpcCommand::GetMessages { .. } => Some(RpcResponse::success(
            id,
            "get_messages",
            Some(serde_json::json!({"messages": []})),
        )),

        // =================================================================
        // Commands
        // =================================================================
        RpcCommand::GetCommands { .. } => Some(RpcResponse::success(
            id,
            "get_commands",
            Some(serde_json::json!({"commands": []})),
        )),
    }
}

/// Extract the id field from any RpcCommand variant.
fn get_command_id(command: &RpcCommand) -> Option<String> {
    match command {
        RpcCommand::Prompt { id, .. } => id.clone(),
        RpcCommand::Steer { id, .. } => id.clone(),
        RpcCommand::FollowUp { id, .. } => id.clone(),
        RpcCommand::Abort { id } => id.clone(),
        RpcCommand::NewSession { id, .. } => id.clone(),
        RpcCommand::GetState { id } => id.clone(),
        RpcCommand::SetModel { id, .. } => id.clone(),
        RpcCommand::CycleModel { id } => id.clone(),
        RpcCommand::GetAvailableModels { id } => id.clone(),
        RpcCommand::SetThinkingLevel { id, .. } => id.clone(),
        RpcCommand::CycleThinkingLevel { id } => id.clone(),
        RpcCommand::SetSteeringMode { id, .. } => id.clone(),
        RpcCommand::SetFollowUpMode { id, .. } => id.clone(),
        RpcCommand::Compact { id, .. } => id.clone(),
        RpcCommand::SetAutoCompaction { id, .. } => id.clone(),
        RpcCommand::SetAutoRetry { id, .. } => id.clone(),
        RpcCommand::AbortRetry { id } => id.clone(),
        RpcCommand::Bash { id, .. } => id.clone(),
        RpcCommand::AbortBash { id } => id.clone(),
        RpcCommand::GetSessionStats { id } => id.clone(),
        RpcCommand::ExportHtml { id, .. } => id.clone(),
        RpcCommand::SwitchSession { id, .. } => id.clone(),
        RpcCommand::Fork { id, .. } => id.clone(),
        RpcCommand::Clone { id } => id.clone(),
        RpcCommand::GetForkMessages { id } => id.clone(),
        RpcCommand::GetLastAssistantText { id } => id.clone(),
        RpcCommand::SetSessionName { id, .. } => id.clone(),
        RpcCommand::GetMessages { id } => id.clone(),
        RpcCommand::GetCommands { id } => id.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_command_id() {
        let cmd = RpcCommand::Prompt {
            id: Some("req_1".to_string()),
            message: "test".to_string(),
            images: None,
            streaming_behavior: None,
        };
        assert_eq!(get_command_id(&cmd), Some("req_1".to_string()));

        let cmd = RpcCommand::GetState { id: None };
        assert_eq!(get_command_id(&cmd), None);
    }

    #[test]
    fn test_get_command_id_prompt_no_id() {
        let cmd = RpcCommand::Prompt {
            id: None,
            message: "test".to_string(),
            images: None,
            streaming_behavior: None,
        };
        assert_eq!(get_command_id(&cmd), None);
    }

    #[test]
    fn test_get_command_id_abort() {
        let cmd = RpcCommand::Abort {
            id: Some("abort_1".to_string()),
        };
        assert_eq!(get_command_id(&cmd), Some("abort_1".to_string()));
    }

    #[test]
    fn test_get_command_id_new_session() {
        let cmd = RpcCommand::NewSession {
            id: Some("ns_1".to_string()),
            parent_session: None,
        };
        assert_eq!(get_command_id(&cmd), Some("ns_1".to_string()));
    }

    #[test]
    fn test_get_command_id_set_model() {
        let cmd = RpcCommand::SetModel {
            id: Some("sm_1".to_string()),
            provider: "anthropic".to_string(),
            model_id: "claude-3".to_string(),
        };
        assert_eq!(get_command_id(&cmd), Some("sm_1".to_string()));
    }

    #[test]
    fn test_get_command_id_bash() {
        let cmd = RpcCommand::Bash {
            id: Some("bash_1".to_string()),
            command: "ls".to_string(),
            exclude_from_context: None,
        };
        assert_eq!(get_command_id(&cmd), Some("bash_1".to_string()));
    }

    #[test]
    fn test_get_command_id_fork() {
        let cmd = RpcCommand::Fork {
            id: Some("fork_1".to_string()),
            entry_id: "entry-1".to_string(),
        };
        assert_eq!(get_command_id(&cmd), Some("fork_1".to_string()));
    }

    #[test]
    fn test_get_command_id_switch_session() {
        let cmd = RpcCommand::SwitchSession {
            id: Some("ss_1".to_string()),
            session_path: "/sessions/test".to_string(),
        };
        assert_eq!(get_command_id(&cmd), Some("ss_1".to_string()));
    }

    #[test]
    fn test_get_command_id_get_messages() {
        let cmd = RpcCommand::GetMessages {
            id: Some("gm_1".to_string()),
        };
        assert_eq!(get_command_id(&cmd), Some("gm_1".to_string()));
    }

    #[test]
    fn test_get_command_id_get_last_assistant_text() {
        let cmd = RpcCommand::GetLastAssistantText {
            id: Some("gl_1".to_string()),
        };
        assert_eq!(get_command_id(&cmd), Some("gl_1".to_string()));
    }

    #[test]
    fn test_get_command_id_set_session_name() {
        let cmd = RpcCommand::SetSessionName {
            id: Some("sn_1".to_string()),
            name: "test session".to_string(),
        };
        assert_eq!(get_command_id(&cmd), Some("sn_1".to_string()));
    }

    #[test]
    fn test_get_command_id_get_commands() {
        let cmd = RpcCommand::GetCommands {
            id: Some("gc_1".to_string()),
        };
        assert_eq!(get_command_id(&cmd), Some("gc_1".to_string()));
    }

    #[test]
    fn test_get_command_id_compact() {
        let cmd = RpcCommand::Compact {
            id: Some("cp_1".to_string()),
            custom_instructions: None,
        };
        assert_eq!(get_command_id(&cmd), Some("cp_1".to_string()));
    }

    #[test]
    fn test_get_command_id_cycle_model() {
        let cmd = RpcCommand::CycleModel {
            id: Some("cm_1".to_string()),
        };
        assert_eq!(get_command_id(&cmd), Some("cm_1".to_string()));
    }

    #[test]
    fn test_get_command_id_get_available_models() {
        let cmd = RpcCommand::GetAvailableModels {
            id: Some("gam_1".to_string()),
        };
        assert_eq!(get_command_id(&cmd), Some("gam_1".to_string()));
    }

    #[test]
    fn test_get_command_id_cycle_thinking_level() {
        let cmd = RpcCommand::CycleThinkingLevel {
            id: Some("ctl_1".to_string()),
        };
        assert_eq!(get_command_id(&cmd), Some("ctl_1".to_string()));
    }

    #[test]
    fn test_get_command_id_set_steering_mode() {
        let cmd = RpcCommand::SetSteeringMode {
            id: Some("ssm_1".to_string()),
            mode: "steer".to_string(),
        };
        assert_eq!(get_command_id(&cmd), Some("ssm_1".to_string()));
    }

    #[test]
    fn test_get_command_id_set_follow_up_mode() {
        let cmd = RpcCommand::SetFollowUpMode {
            id: Some("sfu_1".to_string()),
            mode: "followUp".to_string(),
        };
        assert_eq!(get_command_id(&cmd), Some("sfu_1".to_string()));
    }

    #[test]
    fn test_get_command_id_set_auto_compaction() {
        let cmd = RpcCommand::SetAutoCompaction {
            id: Some("sac_1".to_string()),
            enabled: true,
        };
        assert_eq!(get_command_id(&cmd), Some("sac_1".to_string()));
    }

    #[test]
    fn test_get_command_id_set_auto_retry() {
        let cmd = RpcCommand::SetAutoRetry {
            id: Some("sar_1".to_string()),
            enabled: false,
        };
        assert_eq!(get_command_id(&cmd), Some("sar_1".to_string()));
    }

    #[test]
    fn test_get_command_id_abort_retry() {
        let cmd = RpcCommand::AbortRetry {
            id: Some("ar_1".to_string()),
        };
        assert_eq!(get_command_id(&cmd), Some("ar_1".to_string()));
    }

    #[test]
    fn test_get_command_id_abort_bash() {
        let cmd = RpcCommand::AbortBash {
            id: Some("ab_1".to_string()),
        };
        assert_eq!(get_command_id(&cmd), Some("ab_1".to_string()));
    }

    #[test]
    fn test_get_command_id_get_session_stats() {
        let cmd = RpcCommand::GetSessionStats {
            id: Some("gss_1".to_string()),
        };
        assert_eq!(get_command_id(&cmd), Some("gss_1".to_string()));
    }

    #[test]
    fn test_get_command_id_export_html() {
        let cmd = RpcCommand::ExportHtml {
            id: Some("eh_1".to_string()),
            output_path: None,
        };
        assert_eq!(get_command_id(&cmd), Some("eh_1".to_string()));
    }

    #[test]
    fn test_get_command_id_clone_session() {
        let cmd = RpcCommand::Clone {
            id: Some("cl_1".to_string()),
        };
        assert_eq!(get_command_id(&cmd), Some("cl_1".to_string()));
    }

    #[test]
    fn test_get_command_id_get_fork_messages() {
        let cmd = RpcCommand::GetForkMessages {
            id: Some("gfm_1".to_string()),
        };
        assert_eq!(get_command_id(&cmd), Some("gfm_1".to_string()));
    }

    // ─── handle_command dispatch tests ───────────────────────────────────────
    //
    // handle_command is a pure function mapping RpcCommand to RpcResponse.
    // Every command variant is tested below.

    fn make_id() -> Option<String> {
        Some("test_id".to_string())
    }

    // ── Prompting ────────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_handle_prompt() {
        let cmd = RpcCommand::Prompt {
            id: make_id(),
            message: "hello".to_string(),
            images: None,
            streaming_behavior: None,
        };
        let resp = handle_command(&cmd).await.unwrap();
        assert!(resp.success);
        assert_eq!(resp.command, "prompt");
        assert_eq!(resp.id, make_id());
        assert!(resp.data.is_none());
    }

    #[tokio::test]
    async fn test_handle_steer() {
        let cmd = RpcCommand::Steer {
            id: make_id(),
            message: "steer this".to_string(),
            images: None,
        };
        let resp = handle_command(&cmd).await.unwrap();
        assert!(resp.success);
        assert_eq!(resp.command, "steer");
    }

    #[tokio::test]
    async fn test_handle_follow_up() {
        let cmd = RpcCommand::FollowUp {
            id: make_id(),
            message: "follow up".to_string(),
            images: None,
        };
        let resp = handle_command(&cmd).await.unwrap();
        assert!(resp.success);
        assert_eq!(resp.command, "follow_up");
    }

    #[tokio::test]
    async fn test_handle_abort() {
        let cmd = RpcCommand::Abort { id: make_id() };
        let resp = handle_command(&cmd).await.unwrap();
        assert!(resp.success);
        assert_eq!(resp.command, "abort");
    }

    #[tokio::test]
    async fn test_handle_new_session() {
        let cmd = RpcCommand::NewSession {
            id: make_id(),
            parent_session: None,
        };
        let resp = handle_command(&cmd).await.unwrap();
        assert!(resp.success);
        assert_eq!(resp.command, "new_session");
        assert!(resp.data.is_some());
        assert_eq!(resp.data.unwrap(), serde_json::json!({"cancelled": false}));
    }

    // ── State ────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_handle_get_state() {
        let cmd = RpcCommand::GetState { id: make_id() };
        let resp = handle_command(&cmd).await.unwrap();
        assert!(resp.success);
        assert_eq!(resp.command, "get_state");
        let data = resp.data.unwrap();
        assert_eq!(data["sessionId"], "stub");
        assert_eq!(data["isStreaming"], false);
        assert_eq!(data["messageCount"], 0);
    }

    // ── Model ────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_handle_set_model() {
        let cmd = RpcCommand::SetModel {
            id: make_id(),
            provider: "anthropic".to_string(),
            model_id: "claude-3".to_string(),
        };
        let resp = handle_command(&cmd).await.unwrap();
        assert!(resp.success);
        assert_eq!(resp.command, "set_model");
        let data = resp.data.unwrap();
        assert_eq!(data["provider"], "");
        assert_eq!(data["id"], "");
    }

    #[tokio::test]
    async fn test_handle_cycle_model() {
        let cmd = RpcCommand::CycleModel { id: make_id() };
        let resp = handle_command(&cmd).await.unwrap();
        assert!(resp.success);
        assert_eq!(resp.command, "cycle_model");
    }

    #[tokio::test]
    async fn test_handle_get_available_models() {
        let cmd = RpcCommand::GetAvailableModels { id: make_id() };
        let resp = handle_command(&cmd).await.unwrap();
        assert!(resp.success);
        assert_eq!(resp.command, "get_available_models");
        let data = resp.data.unwrap();
        assert_eq!(data["models"].as_array().unwrap().len(), 0);
    }

    // ── Thinking ─────────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_handle_set_thinking_level() {
        let cmd = RpcCommand::SetThinkingLevel {
            id: make_id(),
            level: hamr_ai::types::ThinkingLevel::High,
        };
        let resp = handle_command(&cmd).await.unwrap();
        assert!(resp.success);
        assert_eq!(resp.command, "set_thinking_level");
    }

    #[tokio::test]
    async fn test_handle_cycle_thinking_level() {
        let cmd = RpcCommand::CycleThinkingLevel { id: make_id() };
        let resp = handle_command(&cmd).await.unwrap();
        assert!(resp.success);
        assert_eq!(resp.command, "cycle_thinking_level");
    }

    // ── Queue Modes ──────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_handle_set_steering_mode() {
        let cmd = RpcCommand::SetSteeringMode {
            id: make_id(),
            mode: "all".to_string(),
        };
        let resp = handle_command(&cmd).await.unwrap();
        assert!(resp.success);
        assert_eq!(resp.command, "set_steering_mode");
    }

    #[tokio::test]
    async fn test_handle_set_follow_up_mode() {
        let cmd = RpcCommand::SetFollowUpMode {
            id: make_id(),
            mode: "all".to_string(),
        };
        let resp = handle_command(&cmd).await.unwrap();
        assert!(resp.success);
        assert_eq!(resp.command, "set_follow_up_mode");
    }

    // ── Compaction ───────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_handle_compact() {
        let cmd = RpcCommand::Compact {
            id: make_id(),
            custom_instructions: None,
        };
        let resp = handle_command(&cmd).await.unwrap();
        assert!(resp.success);
        assert_eq!(resp.command, "compact");
    }

    #[tokio::test]
    async fn test_handle_set_auto_compaction() {
        let cmd = RpcCommand::SetAutoCompaction {
            id: make_id(),
            enabled: true,
        };
        let resp = handle_command(&cmd).await.unwrap();
        assert!(resp.success);
        assert_eq!(resp.command, "set_auto_compaction");
    }

    // ── Retry ────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_handle_set_auto_retry() {
        let cmd = RpcCommand::SetAutoRetry {
            id: make_id(),
            enabled: false,
        };
        let resp = handle_command(&cmd).await.unwrap();
        assert!(resp.success);
        assert_eq!(resp.command, "set_auto_retry");
    }

    #[tokio::test]
    async fn test_handle_abort_retry() {
        let cmd = RpcCommand::AbortRetry { id: make_id() };
        let resp = handle_command(&cmd).await.unwrap();
        assert!(resp.success);
        assert_eq!(resp.command, "abort_retry");
    }

    // ── Bash ─────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_handle_bash() {
        let cmd = RpcCommand::Bash {
            id: make_id(),
            command: "ls".to_string(),
            exclude_from_context: None,
        };
        let resp = handle_command(&cmd).await.unwrap();
        assert!(resp.success);
        assert_eq!(resp.command, "bash");
    }

    #[tokio::test]
    async fn test_handle_abort_bash() {
        let cmd = RpcCommand::AbortBash { id: make_id() };
        let resp = handle_command(&cmd).await.unwrap();
        assert!(resp.success);
        assert_eq!(resp.command, "abort_bash");
    }

    // ── Session ──────────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_handle_get_session_stats() {
        let cmd = RpcCommand::GetSessionStats { id: make_id() };
        let resp = handle_command(&cmd).await.unwrap();
        assert!(resp.success);
        assert_eq!(resp.command, "get_session_stats");
    }

    #[tokio::test]
    async fn test_handle_export_html() {
        let cmd = RpcCommand::ExportHtml {
            id: make_id(),
            output_path: None,
        };
        let resp = handle_command(&cmd).await.unwrap();
        assert!(resp.success);
        assert_eq!(resp.command, "export_html");
        let data = resp.data.unwrap();
        assert_eq!(data["path"], "");
    }

    #[tokio::test]
    async fn test_handle_switch_session() {
        let cmd = RpcCommand::SwitchSession {
            id: make_id(),
            session_path: "/sessions/test".to_string(),
        };
        let resp = handle_command(&cmd).await.unwrap();
        assert!(resp.success);
        assert_eq!(resp.command, "switch_session");
        assert_eq!(resp.data.unwrap(), serde_json::json!({"cancelled": false}));
    }

    #[tokio::test]
    async fn test_handle_fork() {
        let cmd = RpcCommand::Fork {
            id: make_id(),
            entry_id: "entry-1".to_string(),
        };
        let resp = handle_command(&cmd).await.unwrap();
        assert!(resp.success);
        assert_eq!(resp.command, "fork");
        assert_eq!(
            resp.data.unwrap(),
            serde_json::json!({"text": "", "cancelled": false})
        );
    }

    #[tokio::test]
    async fn test_handle_clone() {
        let cmd = RpcCommand::Clone { id: make_id() };
        let resp = handle_command(&cmd).await.unwrap();
        assert!(resp.success);
        assert_eq!(resp.command, "clone");
        assert_eq!(resp.data.unwrap(), serde_json::json!({"cancelled": false}));
    }

    #[tokio::test]
    async fn test_handle_get_fork_messages() {
        let cmd = RpcCommand::GetForkMessages { id: make_id() };
        let resp = handle_command(&cmd).await.unwrap();
        assert!(resp.success);
        assert_eq!(resp.command, "get_fork_messages");
        assert_eq!(resp.data.unwrap(), serde_json::json!({"messages": []}));
    }

    #[tokio::test]
    async fn test_handle_get_last_assistant_text() {
        let cmd = RpcCommand::GetLastAssistantText { id: make_id() };
        let resp = handle_command(&cmd).await.unwrap();
        assert!(resp.success);
        assert_eq!(resp.command, "get_last_assistant_text");
        assert_eq!(resp.data.unwrap(), serde_json::json!({"text": null}));
    }

    #[tokio::test]
    async fn test_handle_set_session_name() {
        let cmd = RpcCommand::SetSessionName {
            id: make_id(),
            name: "test session".to_string(),
        };
        let resp = handle_command(&cmd).await.unwrap();
        assert!(resp.success);
        assert_eq!(resp.command, "set_session_name");
    }

    #[tokio::test]
    async fn test_handle_set_session_name_empty_fails() {
        let cmd = RpcCommand::SetSessionName {
            id: make_id(),
            name: "   ".to_string(),
        };
        let resp = handle_command(&cmd).await.unwrap();
        assert!(!resp.success);
        assert_eq!(resp.command, "set_session_name");
        assert_eq!(resp.error.unwrap(), "Session name cannot be empty");
    }

    // ── Messages ─────────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_handle_get_messages() {
        let cmd = RpcCommand::GetMessages { id: make_id() };
        let resp = handle_command(&cmd).await.unwrap();
        assert!(resp.success);
        assert_eq!(resp.command, "get_messages");
        assert_eq!(resp.data.unwrap(), serde_json::json!({"messages": []}));
    }

    // ── Commands ─────────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_handle_get_commands() {
        let cmd = RpcCommand::GetCommands { id: make_id() };
        let resp = handle_command(&cmd).await.unwrap();
        assert!(resp.success);
        assert_eq!(resp.command, "get_commands");
        assert_eq!(resp.data.unwrap(), serde_json::json!({"commands": []}));
    }
}
