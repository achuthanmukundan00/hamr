//! Port of `packages/coding-agent/src/modes/rpc/rpc-client.ts`
//!
//! RPC Client for programmatic access to the coding agent.
//! Spawns the agent in RPC mode and provides a typed API for all operations.

use crate::modes::rpc::jsonl::JsonlLineReader;
use crate::modes::rpc::rpc_types::{RpcCommand, RpcResponse, RpcSessionState, RpcSlashCommand};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::{Mutex, mpsc};

// ============================================================================
// Types
// ============================================================================

pub type ModelInfo = serde_json::Value;
pub type AgentEvent = serde_json::Value;

pub type RpcEventListener = Arc<dyn Fn(AgentEvent) + Send + Sync>;

pub struct RpcClientOptions {
    pub cli_path: Option<String>,
    pub cwd: Option<String>,
    pub env: Option<HashMap<String, String>>,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub args: Option<Vec<String>>,
}

impl Default for RpcClientOptions {
    fn default() -> Self {
        Self {
            cli_path: None,
            cwd: None,
            env: None,
            provider: None,
            model: None,
            args: None,
        }
    }
}

type PendingRequest = tokio::sync::oneshot::Sender<Result<RpcResponse, String>>;

// ============================================================================
// Shared state between send() and the response handler task
// ============================================================================

struct RpcSharedState {
    pending_requests: HashMap<String, PendingRequest>,
    request_id: u64,
}

// ============================================================================
// RPC Client
// ============================================================================

pub struct RpcClient {
    process: Option<Child>,
    event_listeners: Vec<RpcEventListener>,
    /// Stdin writer for sending commands to the agent process.
    stdin: Option<Arc<tokio::sync::Mutex<tokio::process::ChildStdin>>>,
    /// Shared state for pending requests (accessed from send() and response task).
    shared: Arc<Mutex<RpcSharedState>>,
    stderr: String,
    exit_error: Option<String>,
    options: RpcClientOptions,
}

impl RpcClient {
    pub fn new(options: RpcClientOptions) -> Self {
        Self {
            process: None,
            event_listeners: Vec::new(),
            stdin: None,
            shared: Arc::new(Mutex::new(RpcSharedState {
                pending_requests: HashMap::new(),
                request_id: 0,
            })),
            stderr: String::new(),
            exit_error: None,
            options,
        }
    }

    /// Start the RPC agent process.
    pub async fn start(&mut self) -> Result<(), String> {
        if self.process.is_some() {
            return Err("Client already started".to_string());
        }

        self.exit_error = None;

        let cli_path = self.options.cli_path.as_deref().unwrap_or("dist/cli.js");
        let mut args = vec!["--mode".to_string(), "rpc".to_string()];

        if let Some(ref provider) = self.options.provider {
            args.push("--provider".to_string());
            args.push(provider.clone());
        }
        if let Some(ref model) = self.options.model {
            args.push("--model".to_string());
            args.push(model.clone());
        }
        if let Some(ref extra_args) = self.options.args {
            args.extend(extra_args.clone());
        }

        let mut cmd = Command::new("node");
        cmd.arg(cli_path);
        cmd.args(&args);
        cmd.stdin(std::process::Stdio::piped());
        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::piped());

        if let Some(ref cwd) = self.options.cwd {
            cmd.current_dir(cwd);
        }

        for (k, v) in std::env::vars() {
            cmd.env(k, v);
        }
        if let Some(ref env) = self.options.env {
            for (k, v) in env {
                cmd.env(k, v);
            }
        }

        cmd.kill_on_drop(true);

        let mut child = cmd
            .spawn()
            .map_err(|e| format!("Failed to spawn agent: {}", e))?;

        // Take stdin before moving child
        let stdin = child.stdin.take();

        // Collect stderr
        let stderr = child.stderr.take();

        // Create channels for events and responses
        let (event_tx, mut event_rx) = mpsc::unbounded_channel::<AgentEvent>();
        let (response_tx, mut response_rx) =
            mpsc::unbounded_channel::<Result<RpcResponse, String>>();

        // Clone shared state for the response handler task
        let shared_for_response = self.shared.clone();

        // Spawn stdout reader task — parses JSONL lines into events/responses
        if let Some(stdout) = child.stdout.take() {
            tokio::spawn(async move {
                let mut reader = BufReader::new(stdout);
                let mut line_reader = JsonlLineReader::new();
                let mut line_buf = String::new();

                loop {
                    line_buf.clear();
                    match reader.read_line(&mut line_buf).await {
                        Ok(0) => {
                            if let Some(line) = line_reader.end() {
                                process_line(&line, &event_tx, &response_tx);
                            }
                            break;
                        }
                        Ok(_) => {
                            for line in line_reader.feed(&line_buf) {
                                process_line(&line, &event_tx, &response_tx);
                            }
                        }
                        Err(_) => break,
                    }
                }
            });
        }

        // Spawn event dispatcher task — forwards events to registered listeners
        let event_listeners = self.event_listeners.clone();
        tokio::spawn(async move {
            while let Some(event) = event_rx.recv().await {
                for listener in &event_listeners {
                    listener(event.clone());
                }
            }
        });

        // Spawn response handler task — resolves pending requests by ID
        tokio::spawn(async move {
            while let Some(result) = response_rx.recv().await {
                let resp = match result {
                    Ok(r) => r,
                    Err(e) => {
                        // Parse error — can't match to a request, log and continue
                        eprintln!("RPC response parse error: {}", e);
                        continue;
                    }
                };
                if let Some(id) = &resp.id {
                    let mut state = shared_for_response.lock().await;
                    if let Some(tx) = state.pending_requests.remove(id) {
                        let _ = tx.send(Ok(resp));
                    }
                }
            }
        });

        // Collect stderr
        if let Some(stderr_read) = stderr {
            tokio::spawn(async move {
                let mut reader = BufReader::new(stderr_read);
                let mut buf = String::new();
                while reader
                    .read_line(&mut buf)
                    .await
                    .ok()
                    .map_or(false, |n| n > 0)
                {
                    eprint!("{}", buf);
                    buf.clear();
                }
            });
        }

        // Wait for process to initialize
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        if let Ok(Some(status)) = child.try_wait() {
            let error = self.create_process_exit_error(status.code(), None);
            self.exit_error = Some(error.clone());
            return Err(error);
        }

        self.process = Some(child);
        self.stdin = stdin.map(|s| Arc::new(tokio::sync::Mutex::new(s)));

        Ok(())
    }

    /// Stop the RPC agent process.
    pub async fn stop(&mut self) -> Result<(), String> {
        self.stdin = None;
        if let Some(mut child) = self.process.take() {
            child.start_kill().ok();
            let timeout = tokio::time::sleep(tokio::time::Duration::from_secs(1));
            tokio::select! {
                _ = child.wait() => {}
                _ = timeout => {
                    child.kill().await.ok();
                }
            }
            let mut state = self.shared.lock().await;
            state.pending_requests.clear();
        }
        Ok(())
    }

    /// Subscribe to agent events.
    pub fn on_event<F>(&mut self, listener: F)
    where
        F: Fn(AgentEvent) + Send + Sync + 'static,
    {
        self.event_listeners.push(Arc::new(listener));
    }

    /// Get collected stderr output.
    pub fn get_stderr(&self) -> &str {
        &self.stderr
    }

    // =========================================================================
    // Command Methods
    // =========================================================================

    pub async fn prompt(
        &self,
        message: &str,
        images: Option<Vec<hamr_ai::types::ImageContent>>,
    ) -> Result<(), String> {
        self.send(&RpcCommand::Prompt {
            id: None,
            message: message.to_string(),
            images,
            streaming_behavior: None,
        })
        .await
        .map(|_| ())
    }

    pub async fn steer(
        &self,
        message: &str,
        images: Option<Vec<hamr_ai::types::ImageContent>>,
    ) -> Result<(), String> {
        self.send(&RpcCommand::Steer {
            id: None,
            message: message.to_string(),
            images,
        })
        .await
        .map(|_| ())
    }

    pub async fn follow_up(
        &self,
        message: &str,
        images: Option<Vec<hamr_ai::types::ImageContent>>,
    ) -> Result<(), String> {
        self.send(&RpcCommand::FollowUp {
            id: None,
            message: message.to_string(),
            images,
        })
        .await
        .map(|_| ())
    }

    pub async fn abort(&self) -> Result<(), String> {
        self.send(&RpcCommand::Abort { id: None }).await.map(|_| ())
    }

    pub async fn new_session(&self, parent_session: Option<&str>) -> Result<Value, String> {
        let resp = self
            .send(&RpcCommand::NewSession {
                id: None,
                parent_session: parent_session.map(|s| s.to_string()),
            })
            .await?;
        self.get_data(resp)
    }

    pub async fn get_state(&self) -> Result<RpcSessionState, String> {
        let resp = self.send(&RpcCommand::GetState { id: None }).await?;
        Ok(serde_json::from_value(self.get_data::<Value>(resp)?).map_err(|e| e.to_string())?)
    }

    pub async fn set_model(&self, provider: &str, model_id: &str) -> Result<Value, String> {
        let resp = self
            .send(&RpcCommand::SetModel {
                id: None,
                provider: provider.to_string(),
                model_id: model_id.to_string(),
            })
            .await?;
        self.get_data(resp)
    }

    pub async fn cycle_model(&self) -> Result<Option<Value>, String> {
        let resp = self.send(&RpcCommand::CycleModel { id: None }).await?;
        let data = self.get_data::<Value>(resp)?;
        if data.is_null() {
            Ok(None)
        } else {
            Ok(Some(data))
        }
    }

    pub async fn get_available_models(&self) -> Result<Vec<Value>, String> {
        let resp = self
            .send(&RpcCommand::GetAvailableModels { id: None })
            .await?;
        let data = self.get_data::<Value>(resp)?;
        Ok(data
            .get("models")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default())
    }

    pub async fn set_thinking_level(
        &self,
        level: hamr_ai::types::ThinkingLevel,
    ) -> Result<(), String> {
        self.send(&RpcCommand::SetThinkingLevel { id: None, level })
            .await
            .map(|_| ())
    }

    pub async fn cycle_thinking_level(&self) -> Result<Option<Value>, String> {
        let resp = self
            .send(&RpcCommand::CycleThinkingLevel { id: None })
            .await?;
        let data = self.get_data::<Value>(resp)?;
        if data.is_null() {
            Ok(None)
        } else {
            Ok(Some(data))
        }
    }

    pub async fn set_steering_mode(&self, mode: &str) -> Result<(), String> {
        self.send(&RpcCommand::SetSteeringMode {
            id: None,
            mode: mode.to_string(),
        })
        .await
        .map(|_| ())
    }

    pub async fn set_follow_up_mode(&self, mode: &str) -> Result<(), String> {
        self.send(&RpcCommand::SetFollowUpMode {
            id: None,
            mode: mode.to_string(),
        })
        .await
        .map(|_| ())
    }

    pub async fn compact(&self, custom_instructions: Option<&str>) -> Result<Value, String> {
        let resp = self
            .send(&RpcCommand::Compact {
                id: None,
                custom_instructions: custom_instructions.map(|s| s.to_string()),
            })
            .await?;
        self.get_data(resp)
    }

    pub async fn set_auto_compaction(&self, enabled: bool) -> Result<(), String> {
        self.send(&RpcCommand::SetAutoCompaction { id: None, enabled })
            .await
            .map(|_| ())
    }

    pub async fn set_auto_retry(&self, enabled: bool) -> Result<(), String> {
        self.send(&RpcCommand::SetAutoRetry { id: None, enabled })
            .await
            .map(|_| ())
    }

    pub async fn abort_retry(&self) -> Result<(), String> {
        self.send(&RpcCommand::AbortRetry { id: None })
            .await
            .map(|_| ())
    }

    pub async fn bash(&self, command: &str) -> Result<Value, String> {
        let resp = self
            .send(&RpcCommand::Bash {
                id: None,
                command: command.to_string(),
                exclude_from_context: None,
            })
            .await?;
        self.get_data(resp)
    }

    pub async fn abort_bash(&self) -> Result<(), String> {
        self.send(&RpcCommand::AbortBash { id: None })
            .await
            .map(|_| ())
    }

    pub async fn get_session_stats(&self) -> Result<Value, String> {
        let resp = self.send(&RpcCommand::GetSessionStats { id: None }).await?;
        self.get_data(resp)
    }

    pub async fn export_html(&self, output_path: Option<&str>) -> Result<Value, String> {
        let resp = self
            .send(&RpcCommand::ExportHtml {
                id: None,
                output_path: output_path.map(|s| s.to_string()),
            })
            .await?;
        self.get_data(resp)
    }

    pub async fn switch_session(&self, session_path: &str) -> Result<Value, String> {
        let resp = self
            .send(&RpcCommand::SwitchSession {
                id: None,
                session_path: session_path.to_string(),
            })
            .await?;
        self.get_data(resp)
    }

    pub async fn fork(&self, entry_id: &str) -> Result<Value, String> {
        let resp = self
            .send(&RpcCommand::Fork {
                id: None,
                entry_id: entry_id.to_string(),
            })
            .await?;
        self.get_data(resp)
    }

    pub async fn clone_session(&self) -> Result<Value, String> {
        let resp = self.send(&RpcCommand::Clone { id: None }).await?;
        self.get_data(resp)
    }

    pub async fn get_fork_messages(&self) -> Result<Vec<Value>, String> {
        let resp = self.send(&RpcCommand::GetForkMessages { id: None }).await?;
        let data = self.get_data::<Value>(resp)?;
        Ok(data
            .get("messages")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default())
    }

    pub async fn get_last_assistant_text(&self) -> Result<Option<String>, String> {
        let resp = self
            .send(&RpcCommand::GetLastAssistantText { id: None })
            .await?;
        let data = self.get_data::<Value>(resp)?;
        Ok(data
            .get("text")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()))
    }

    pub async fn set_session_name(&self, name: &str) -> Result<(), String> {
        self.send(&RpcCommand::SetSessionName {
            id: None,
            name: name.to_string(),
        })
        .await
        .map(|_| ())
    }

    pub async fn get_messages(&self) -> Result<Vec<Value>, String> {
        let resp = self.send(&RpcCommand::GetMessages { id: None }).await?;
        let data = self.get_data::<Value>(resp)?;
        Ok(data
            .get("messages")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default())
    }

    pub async fn get_commands(&self) -> Result<Vec<RpcSlashCommand>, String> {
        let resp = self.send(&RpcCommand::GetCommands { id: None }).await?;
        let data = self.get_data::<Value>(resp)?;
        let arr = data
            .get("commands")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        arr.into_iter()
            .map(|v| serde_json::from_value(v).map_err(|e| e.to_string()))
            .collect()
    }

    // =========================================================================
    // Helpers
    // =========================================================================

    pub async fn wait_for_idle(&self, _timeout_ms: u64) -> Result<(), String> {
        Err("wait_for_idle not yet fully implemented".to_string())
    }

    pub async fn collect_events(&self, _timeout_ms: u64) -> Result<Vec<AgentEvent>, String> {
        Err("collect_events not yet fully implemented".to_string())
    }

    pub async fn prompt_and_wait(
        &self,
        message: &str,
        images: Option<Vec<hamr_ai::types::ImageContent>>,
        timeout_ms: u64,
    ) -> Result<Vec<AgentEvent>, String> {
        let events_promise = self.collect_events(timeout_ms);
        self.prompt(message, images).await?;
        events_promise.await
    }

    // =========================================================================
    // Internal
    // =========================================================================

    fn create_process_exit_error(&self, code: Option<i32>, signal: Option<String>) -> String {
        format!(
            "Agent process exited (code={:?} signal={:?}). Stderr: {}",
            code, signal, self.stderr
        )
    }

    /// Send a command to the agent process via stdin and register a pending response.
    async fn send(&self, command: &RpcCommand) -> Result<RpcResponse, String> {
        let stdin = self
            .stdin
            .as_ref()
            .ok_or("RPC client not started — call start() first")?;

        // Assign a request ID
        let mut state = self.shared.lock().await;
        state.request_id += 1;
        let request_id = format!("req-{}", state.request_id);
        drop(state);

        // Inject the request ID into the command
        let mut cmd_value = serde_json::to_value(command).map_err(|e| e.to_string())?;
        if let Some(obj) = cmd_value.as_object_mut() {
            obj.insert(
                "id".to_string(),
                serde_json::Value::String(request_id.clone()),
            );
        }

        // Create oneshot channel for the response
        let (tx, rx) = tokio::sync::oneshot::channel::<Result<RpcResponse, String>>();

        // Register pending request
        {
            let mut state = self.shared.lock().await;
            state.pending_requests.insert(request_id, tx);
        }

        // Write command to stdin as JSONL
        let line = format!("{}\n", serde_json::to_string(&cmd_value).map_err(|e| e.to_string())?);
        {
            let mut stdin_guard = stdin.lock().await;
            stdin_guard
                .write_all(line.as_bytes())
                .await
                .map_err(|e| format!("Failed to write to stdin: {}", e))?;
            stdin_guard
                .flush()
                .await
                .map_err(|e| format!("Failed to flush stdin: {}", e))?;
        }

        // Wait for response
        rx.await
            .map_err(|_| "Response channel closed — agent may have exited".to_string())?
    }

    fn get_data<T: serde::de::DeserializeOwned>(&self, response: RpcResponse) -> Result<T, String> {
        if !response.success {
            return Err(response
                .error
                .unwrap_or_else(|| "Unknown error".to_string()));
        }
        match response.data {
            Some(data) => serde_json::from_value(data).map_err(|e| e.to_string()),
            None => Err("Response has no data".to_string()),
        }
    }
}

fn process_line(
    line: &str,
    event_tx: &mpsc::UnboundedSender<AgentEvent>,
    response_tx: &mpsc::UnboundedSender<Result<RpcResponse, String>>,
) {
    let value: Value = match serde_json::from_str(line) {
        Ok(v) => v,
        Err(_) => return,
    };

    if value.get("type").and_then(|v| v.as_str()) == Some("response") {
        match serde_json::from_value::<RpcResponse>(value) {
            Ok(resp) => {
                let _ = response_tx.send(Ok(resp));
            }
            Err(e) => {
                let _ = response_tx.send(Err(e.to_string()));
            }
        }
        return;
    }

    let _ = event_tx.send(value);
}
