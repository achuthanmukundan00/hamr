//! Port of `packages/agent/src/harness/agent-harness.ts`.

use crate::agent_loop::run_agent_loop;
use crate::harness::compaction::branch_summarization::{
    GenerateBranchSummaryOptions, collect_entries_for_branch_summary, generate_branch_summary,
};
use crate::harness::compaction::compaction::{
    CompactionResult, DEFAULT_COMPACTION_SETTINGS, compact, prepare_compaction,
};
use crate::harness::messages::convert_to_llm;
use crate::harness::prompt_templates::format_prompt_template_invocation;
use crate::harness::session::session::Session;
use crate::harness::skills::format_skill_invocation;
use crate::harness::types::{AgentHarnessResources, ExecutionEnv};
use crate::types::{AgentContext, AgentEvent, AgentLoopConfig, AgentMessage, AgentTool, QueueMode};
use chrono::Utc;
use hamr_ai::stream::{StreamError, stream_simple};
use hamr_ai::types::{
    AssistantMessage, Context, MessageContent, MessageRole, Model, SimpleStreamOptions,
    TextContent, ThinkingLevel, Transport, UserMessage,
};
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::{Mutex, Notify, watch};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentHarnessPhase {
    Idle,
    Turn,
    Compaction,
    BranchSummary,
}

#[derive(Debug, Clone)]
pub struct AgentHarnessStreamOptions {
    pub transport: Option<Transport>,
    pub timeout_ms: Option<u64>,
    pub max_retries: Option<u32>,
    pub max_retry_delay_ms: Option<u64>,
    pub headers: Option<HashMap<String, String>>,
    pub metadata: Option<serde_json::Value>,
    pub cache_retention: Option<hamr_ai::types::CacheRetention>,
}

impl Default for AgentHarnessStreamOptions {
    fn default() -> Self {
        Self {
            transport: None,
            timeout_ms: None,
            max_retries: None,
            max_retry_delay_ms: None,
            headers: None,
            metadata: None,
            cache_retention: None,
        }
    }
}

#[derive(Debug, Clone)]
pub enum AgentHarnessEvent {
    Agent(AgentEvent),
    QueueUpdate {
        steer: Vec<AgentMessage>,
        follow_up: Vec<AgentMessage>,
        next_turn: Vec<AgentMessage>,
    },
    SavePoint {
        had_pending_mutations: bool,
    },
    Abort {
        cleared_steer: Vec<AgentMessage>,
        cleared_follow_up: Vec<AgentMessage>,
    },
    Settled {
        next_turn_count: usize,
    },
}

#[derive(Debug, Clone)]
pub struct AbortResult {
    pub cleared_steer: Vec<AgentMessage>,
    pub cleared_follow_up: Vec<AgentMessage>,
}

#[derive(Debug, Clone)]
pub struct NavigateTreeResult {
    pub cancelled: bool,
    pub editor_text: Option<String>,
    pub summary_entry_id: Option<String>,
}

type Subscriber =
    Arc<dyn Fn(AgentHarnessEvent) -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + Sync>;

fn create_user_message(text: String) -> AgentMessage {
    AgentMessage::User(UserMessage {
        role: MessageRole::User,
        content: vec![MessageContent::Text(TextContent {
            text,
            text_signature: None,
        })],
        timestamp: Utc::now(),
    })
}

#[derive(Clone)]
struct PendingState {
    notify: Arc<Notify>,
    abort_tx: watch::Sender<bool>,
}

struct Inner<TMetadata: Clone + Send + Sync + 'static> {
    _env: Arc<dyn ExecutionEnv>,
    session: Session<TMetadata>,
    phase: AgentHarnessPhase,
    model: Model,
    thinking_level: Option<ThinkingLevel>,
    system_prompt: String,
    resources: AgentHarnessResources,
    stream_options: AgentHarnessStreamOptions,
    tools: HashMap<String, AgentTool>,
    active_tool_names: Vec<String>,
    steering_mode: QueueMode,
    follow_up_mode: QueueMode,
    steer_queue: Vec<AgentMessage>,
    follow_up_queue: Vec<AgentMessage>,
    next_turn_queue: Vec<AgentMessage>,
    pending_state: Option<PendingState>,
    subscribers: Vec<Subscriber>,
}

pub struct AgentHarness<TMetadata: Clone + Send + Sync + 'static> {
    inner: Arc<Mutex<Inner<TMetadata>>>,
}

impl<TMetadata: Clone + Send + Sync + 'static> Clone for AgentHarness<TMetadata> {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

impl<TMetadata: Clone + Send + Sync + 'static> AgentHarness<TMetadata> {
    pub fn new(env: Arc<dyn ExecutionEnv>, session: Session<TMetadata>, model: Model) -> Self {
        Self {
            inner: Arc::new(Mutex::new(Inner {
                _env: env,
                session,
                phase: AgentHarnessPhase::Idle,
                model,
                thinking_level: None,
                system_prompt: "You are a helpful assistant.".to_string(),
                resources: AgentHarnessResources {
                    prompt_templates: None,
                    skills: None,
                },
                stream_options: AgentHarnessStreamOptions::default(),
                tools: HashMap::new(),
                active_tool_names: Vec::new(),
                steering_mode: QueueMode::OneAtATime,
                follow_up_mode: QueueMode::OneAtATime,
                steer_queue: Vec::new(),
                follow_up_queue: Vec::new(),
                next_turn_queue: Vec::new(),
                pending_state: None,
                subscribers: Vec::new(),
            })),
        }
    }

    async fn emit(&self, event: AgentHarnessEvent) {
        let subscribers = {
            let inner = self.inner.lock().await;
            inner.subscribers.clone()
        };
        for subscriber in subscribers {
            subscriber(event.clone()).await;
        }
    }

    async fn emit_queue_update(&self) {
        let (steer, follow_up, next_turn) = {
            let inner = self.inner.lock().await;
            (
                inner.steer_queue.clone(),
                inner.follow_up_queue.clone(),
                inner.next_turn_queue.clone(),
            )
        };
        self.emit(AgentHarnessEvent::QueueUpdate {
            steer,
            follow_up,
            next_turn,
        })
        .await;
    }

    pub async fn subscribe(&self, listener: Subscriber) {
        self.inner.lock().await.subscribers.push(listener);
    }

    pub async fn set_system_prompt(&self, system_prompt: String) {
        self.inner.lock().await.system_prompt = system_prompt;
    }

    pub async fn set_resources(&self, resources: AgentHarnessResources) {
        self.inner.lock().await.resources = resources;
    }

    pub async fn set_tools(&self, tools: Vec<AgentTool>, active_tool_names: Option<Vec<String>>) {
        let mut inner = self.inner.lock().await;
        inner.tools = tools
            .into_iter()
            .map(|tool| (tool.name.clone(), tool))
            .collect();
        inner.active_tool_names =
            active_tool_names.unwrap_or_else(|| inner.tools.keys().cloned().collect());
    }

    pub async fn set_stream_options(&self, options: AgentHarnessStreamOptions) {
        self.inner.lock().await.stream_options = options;
    }

    pub async fn set_model(&self, model: Model) {
        self.inner.lock().await.model = model;
    }

    pub async fn set_thinking_level(&self, thinking_level: Option<ThinkingLevel>) {
        self.inner.lock().await.thinking_level = thinking_level;
    }

    pub async fn set_steering_mode(&self, mode: QueueMode) {
        self.inner.lock().await.steering_mode = mode;
    }

    pub async fn set_follow_up_mode(&self, mode: QueueMode) {
        self.inner.lock().await.follow_up_mode = mode;
    }

    async fn drain_queue(&self, which: &'static str) -> Vec<AgentMessage> {
        let drained = {
            let mut inner = self.inner.lock().await;
            let mode = if which == "steer" {
                inner.steering_mode
            } else {
                inner.follow_up_mode
            };
            let queue = if which == "steer" {
                &mut inner.steer_queue
            } else {
                &mut inner.follow_up_queue
            };
            if queue.is_empty() {
                Vec::new()
            } else if mode == QueueMode::All {
                std::mem::take(queue)
            } else {
                vec![queue.remove(0)]
            }
        };
        if !drained.is_empty() {
            self.emit_queue_update().await;
        }
        drained
    }

    async fn create_context(
        &self,
    ) -> (
        AgentContext,
        Model,
        Option<ThinkingLevel>,
        Session<TMetadata>,
    ) {
        let inner = self.inner.lock().await;
        let session = inner.session.clone();
        let session_context = session.build_context().await.unwrap_or_else(|_| {
            crate::harness::types::SessionContext {
                messages: Vec::new(),
                thinking_level: "off".to_string(),
                model: None,
                active_tool_names: None,
            }
        });
        let active_tools = inner
            .active_tool_names
            .iter()
            .filter_map(|name| inner.tools.get(name).cloned())
            .collect::<Vec<_>>();
        (
            AgentContext {
                system_prompt: inner.system_prompt.clone(),
                messages: session_context.messages,
                tools: active_tools,
            },
            inner.model.clone(),
            inner.thinking_level,
            session,
        )
    }

    async fn handle_agent_event(&self, event: AgentEvent) {
        if let AgentEvent::MessageEnd { message } = &event {
            let session = { self.inner.lock().await.session.clone() };
            let _ = session.append_message(message.clone()).await;
        }
        self.emit(AgentHarnessEvent::Agent(event)).await;
    }

    pub async fn prompt(&self, text: impl Into<String>) -> Result<AssistantMessage, String> {
        {
            let mut inner = self.inner.lock().await;
            if inner.phase != AgentHarnessPhase::Idle {
                return Err("AgentHarness is busy".to_string());
            }
            inner.phase = AgentHarnessPhase::Turn;
            let (abort_tx, _) = watch::channel(false);
            inner.pending_state = Some(PendingState {
                notify: Arc::new(Notify::new()),
                abort_tx,
            });
        }

        let prompt = create_user_message(text.into());
        let queued_messages = {
            let mut inner = self.inner.lock().await;
            std::mem::take(&mut inner.next_turn_queue)
        };
        if !queued_messages.is_empty() {
            self.emit_queue_update().await;
        }
        let mut initial_messages = queued_messages;
        initial_messages.push(prompt);

        let (context, model, reasoning, _session) = self.create_context().await;
        let state = {
            self.inner
                .lock()
                .await
                .pending_state
                .clone()
                .expect("pending_state must be set before prompt dispatch")
        };
        let harness = self.clone();
        let on_event = Arc::new(move |event: AgentEvent| {
            let harness = harness.clone();
            Box::pin(async move { harness.handle_agent_event(event).await })
                as Pin<Box<dyn Future<Output = ()> + Send>>
        });
        let harness_for_steer = self.clone();
        let harness_for_follow = self.clone();
        let stream_fn = Arc::new(
            move |model: Model, context: Context, options: Option<SimpleStreamOptions>| {
                Box::pin(async move { stream_simple(model, context, options) })
                    as Pin<Box<dyn Future<Output = Result<_, StreamError>> + Send>>
            },
        );
        let config = AgentLoopConfig {
            model: model.clone(),
            reasoning,
            session_id: None,
            transport: None,
            tool_execution: crate::types::ToolExecutionMode::Parallel,
            max_retry_delay_ms: None,
            convert_to_llm: Arc::new(|messages| Box::pin(async move { convert_to_llm(&messages) })),
            transform_context: None,
            get_api_key: None,
            should_stop_after_turn: None,
            prepare_next_turn: None,
            get_steering_messages: Some(Arc::new(move || {
                let harness = harness_for_steer.clone();
                Box::pin(async move { harness.drain_queue("steer").await })
            })),
            get_follow_up_messages: Some(Arc::new(move || {
                let harness = harness_for_follow.clone();
                Box::pin(async move { harness.drain_queue("follow").await })
            })),
            before_tool_call: None,
            after_tool_call: None,
        };

        let run_result = run_agent_loop(
            initial_messages,
            context,
            config,
            on_event,
            Some(state.abort_tx.subscribe()),
            Some(state.abort_tx.subscribe()),
            stream_fn,
        )
        .await;

        let assistant = match run_result {
            Ok(messages) => messages
                .into_iter()
                .rev()
                .find_map(|message| match message {
                    AgentMessage::Assistant(message) => Some(message),
                    _ => None,
                })
                .ok_or_else(|| "run completed without assistant message".to_string())?,
            Err(error) => {
                self.finish_run().await;
                return Err(error);
            }
        };

        self.finish_run().await;
        Ok(assistant)
    }

    async fn finish_run(&self) {
        let pending = {
            let mut inner = self.inner.lock().await;
            inner.phase = AgentHarnessPhase::Idle;
            inner.pending_state.take()
        };
        if let Some(pending) = pending {
            pending.notify.notify_waiters();
        }
        let next_turn_count = { self.inner.lock().await.next_turn_queue.len() };
        self.emit(AgentHarnessEvent::Settled { next_turn_count })
            .await;
    }

    pub async fn steer(&self, text: impl Into<String>) -> Result<(), String> {
        let message = create_user_message(text.into());
        {
            let mut inner = self.inner.lock().await;
            if inner.phase == AgentHarnessPhase::Idle {
                return Err("Cannot steer while idle".to_string());
            }
            inner.steer_queue.push(message);
        }
        self.emit_queue_update().await;
        Ok(())
    }

    pub async fn follow_up(&self, text: impl Into<String>) -> Result<(), String> {
        let message = create_user_message(text.into());
        {
            let mut inner = self.inner.lock().await;
            if inner.phase == AgentHarnessPhase::Idle {
                return Err("Cannot follow up while idle".to_string());
            }
            inner.follow_up_queue.push(message);
        }
        self.emit_queue_update().await;
        Ok(())
    }

    pub async fn next_turn(&self, text: impl Into<String>) {
        self.inner
            .lock()
            .await
            .next_turn_queue
            .push(create_user_message(text.into()));
        self.emit_queue_update().await;
    }

    pub async fn append_message(&self, message: AgentMessage) {
        let session = { self.inner.lock().await.session.clone() };
        let _ = session.append_message(message).await;
    }

    pub async fn abort(&self) -> Result<AbortResult, String> {
        let (cleared_steer, cleared_follow_up, pending) = {
            let mut inner = self.inner.lock().await;
            let cleared_steer = std::mem::take(&mut inner.steer_queue);
            let cleared_follow_up = std::mem::take(&mut inner.follow_up_queue);
            let pending = inner.pending_state.clone();
            (cleared_steer, cleared_follow_up, pending)
        };
        self.emit_queue_update().await;
        if let Some(pending) = pending {
            let _ = pending.abort_tx.send(true);
            pending.notify.notified().await;
        }
        self.emit(AgentHarnessEvent::Abort {
            cleared_steer: cleared_steer.clone(),
            cleared_follow_up: cleared_follow_up.clone(),
        })
        .await;
        Ok(AbortResult {
            cleared_steer,
            cleared_follow_up,
        })
    }

    pub async fn wait_for_idle(&self) {
        let pending = { self.inner.lock().await.pending_state.clone() };
        if let Some(pending) = pending {
            pending.notify.notified().await;
        }
    }

    pub async fn compact(&self) -> Result<CompactionResult, String> {
        let (session, model, thinking_level) = {
            let mut inner = self.inner.lock().await;
            if inner.phase != AgentHarnessPhase::Idle {
                return Err("compact() requires idle harness".to_string());
            }
            inner.phase = AgentHarnessPhase::Compaction;
            (
                inner.session.clone(),
                inner.model.clone(),
                inner.thinking_level,
            )
        };

        // Emit compaction-start event
        self.emit(AgentHarnessEvent::Agent(AgentEvent::CompactionStart {
            reason: "Context limit reached".to_string(),
        }))
        .await;

        let result = async {
            let branch_entries = session.get_branch(None).await.map_err(|e| e.message)?;
            let preparation = prepare_compaction(&branch_entries, &DEFAULT_COMPACTION_SETTINGS)
                .map_err(|e| e.to_string())?
                .ok_or_else(|| "Nothing to compact".to_string())?;
            compact(
                preparation,
                model,
                String::new(),
                None,
                None,
                None,
                thinking_level,
            )
            .await
            .map_err(|e| e.to_string())
        }
        .await;

        // Emit compaction-end and summary events
        match &result {
            Ok(compaction_result) => {
                let tokens_after = ((compaction_result.summary.len() as f64) / 4.0).ceil() as u64;
                self.emit(AgentHarnessEvent::Agent(AgentEvent::CompactionEnd {
                    aborted: false,
                    reason: String::new(),
                    result: Some(crate::types::CompactionResult {
                        summary: compaction_result.summary.clone(),
                        tokens_before: compaction_result.tokens_before,
                        tokens_after,
                    }),
                }))
                .await;
                self.emit(AgentHarnessEvent::Agent(AgentEvent::CompactionSummary {
                    summary: compaction_result.summary.clone(),
                    tokens_before: compaction_result.tokens_before,
                }))
                .await;
            }
            Err(error_message) => {
                self.emit(AgentHarnessEvent::Agent(AgentEvent::CompactionEnd {
                    aborted: true,
                    reason: error_message.clone(),
                    result: None,
                }))
                .await;
            }
        }

        self.inner.lock().await.phase = AgentHarnessPhase::Idle;
        result
    }

    pub async fn skill(
        &self,
        name: &str,
        additional_instructions: Option<String>,
    ) -> Result<AssistantMessage, String> {
        let resources = { self.inner.lock().await.resources.clone() };
        let skill = resources
            .skills
            .unwrap_or_default()
            .into_iter()
            .find(|skill| skill.name == name)
            .ok_or_else(|| format!("Unknown skill: {name}"))?;
        self.prompt(format_skill_invocation(
            &skill,
            additional_instructions.as_deref(),
        ))
        .await
    }

    pub async fn prompt_from_template(
        &self,
        name: &str,
        args: &[String],
    ) -> Result<AssistantMessage, String> {
        let resources = { self.inner.lock().await.resources.clone() };
        let template = resources
            .prompt_templates
            .unwrap_or_default()
            .into_iter()
            .find(|template| template.name == name)
            .ok_or_else(|| format!("Unknown prompt template: {name}"))?;
        self.prompt(format_prompt_template_invocation(&template, Some(args)))
            .await
    }

    pub async fn navigate_tree(
        &self,
        target_id: &str,
        summarize: bool,
    ) -> Result<NavigateTreeResult, String> {
        let (session, model) = {
            let mut inner = self.inner.lock().await;
            if inner.phase != AgentHarnessPhase::Idle {
                return Err("navigate_tree() requires idle harness".to_string());
            }
            inner.phase = AgentHarnessPhase::BranchSummary;
            (inner.session.clone(), inner.model.clone())
        };

        let result = async {
            let old_leaf_id = session.get_leaf_id().await.map_err(|e| e.message)?;
            if old_leaf_id.as_deref() == Some(target_id) {
                return Ok(NavigateTreeResult {
                    cancelled: false,
                    editor_text: None,
                    summary_entry_id: None,
                });
            }
            let target_entry = session
                .get_entry(target_id)
                .await
                .map_err(|e| e.message)?
                .ok_or_else(|| format!("Entry {target_id} not found"))?;
            let collected =
                collect_entries_for_branch_summary(&session, old_leaf_id.clone(), target_id)
                    .await
                    .map_err(|e| e.to_string())?;
            let summary = if summarize && !collected.entries.is_empty() {
                Some(
                    generate_branch_summary(
                        &collected.entries,
                        GenerateBranchSummaryOptions {
                            model,
                            api_key: String::new(),
                            headers: None,
                            signal: None,
                            custom_instructions: None,
                            replace_instructions: false,
                            reserve_tokens: 16_384,
                        },
                    )
                    .await
                    .map_err(|e| e.to_string())?
                    .summary,
                )
            } else {
                None
            };
            let new_leaf_id = if matches!(
                target_entry,
                crate::harness::types::SessionTreeEntry::Message { .. }
                    | crate::harness::types::SessionTreeEntry::CustomMessage { .. }
            ) {
                target_entry.parent_id().map(ToOwned::to_owned)
            } else {
                Some(target_id.to_string())
            };
            let summary_id = session
                .move_to(new_leaf_id, summary.map(|summary| (summary, None, None)))
                .await
                .map_err(|e| e.message)?;
            Ok(NavigateTreeResult {
                cancelled: false,
                editor_text: None,
                summary_entry_id: summary_id,
            })
        }
        .await;

        self.inner.lock().await.phase = AgentHarnessPhase::Idle;
        result
    }
}

#[cfg(test)]
mod harness_e2e_tests {
    //! P0-A session-layer proof: `AgentHarness::prompt` drives the agent loop
    //! against the deterministic `faux` provider through the GLOBAL provider
    //! registry (the real dispatch path — no injected stream_fn), and the full
    //! transcript is **persisted to the Session storage** via the
    //! `MessageEnd → append_message` path. We then re-read the session to prove
    //! durability.
    //!
    //! Registry isolation: other faux tests register under the default
    //! `Api::AnthropicMessages`. We register under `Api::GoogleVertex` so this
    //! test's dispatch can never race with theirs in the shared process-global
    //! registry.
    use super::*;
    use crate::harness::env::nodejs::NodeExecutionEnv;
    use crate::harness::session::memory_storage::InMemorySessionStorage;
    use crate::harness::types::SessionMetadata;
    use crate::types::AgentToolResult;
    use hamr_ai::providers::faux::{
        FauxAssistantMessageOptions, FauxContentBlock, RegisterFauxProviderOptions,
        faux_assistant_message, faux_text, faux_tool_call, register_faux_provider,
    };
    use hamr_ai::types::{Api, AssistantContentBlock, StopReason};
    use std::sync::atomic::{AtomicBool, Ordering};

    fn echo_tool(ran: Arc<AtomicBool>) -> AgentTool {
        AgentTool {
            label: "Echo".to_string(),
            name: "echo".to_string(),
            description: "Echo the provided message back.".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": { "message": { "type": "string" } },
                "required": ["message"],
                "additionalProperties": false
            }),
            prepare_arguments: None,
            execution_mode: None,
            execute: Arc::new(move |_id, params, _signal, _on_update| {
                let ran = ran.clone();
                Box::pin(async move {
                    ran.store(true, Ordering::SeqCst);
                    let message = params
                        .get("message")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or("");
                    AgentToolResult {
                        content: vec![MessageContent::Text(TextContent {
                            text: format!("echoed: {message}"),
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

    fn role_of(message: &AgentMessage) -> &'static str {
        match message {
            AgentMessage::User(_) => "user",
            AgentMessage::Assistant(_) => "assistant",
            AgentMessage::ToolResult(_) => "toolResult",
            _ => "other",
        }
    }

    #[tokio::test]
    async fn e2e_harness_prompt_persists_full_transcript_to_session() {
        // Serialize against other tests that dispatch through the global faux
        // registry (see `crate::faux_registry_guard`).
        let _registry_guard = crate::faux_registry_guard();
        let reg = Arc::new(register_faux_provider(RegisterFauxProviderOptions {
            api_enum: Some(Api::GoogleVertex),
            ..Default::default()
        }));
        reg.set_responses(vec![
            faux_assistant_message(
                vec![
                    FauxContentBlock::Text(faux_text("Echoing now.")),
                    FauxContentBlock::ToolCall(faux_tool_call(
                        "echo",
                        serde_json::json!({ "message": "hi" }),
                        Some("tool-1".to_string()),
                    )),
                ],
                FauxAssistantMessageOptions {
                    stop_reason: Some(StopReason::ToolUse),
                    ..Default::default()
                },
            )
            .into(),
            faux_assistant_message(
                "Done — echoed the greeting.",
                FauxAssistantMessageOptions::default(),
            )
            .into(),
        ]);

        // In-memory session storage (Session holds an Arc, so the harness's clone
        // and our read-back handle observe the same tree).
        let storage = Arc::new(
            InMemorySessionStorage::<SessionMetadata>::new(None, None)
                .expect("in-memory storage must init"),
        );
        let session = Session::new(storage);
        let env: Arc<dyn ExecutionEnv> = Arc::new(NodeExecutionEnv::new(std::env::temp_dir()));
        let harness = AgentHarness::new(env, session.clone(), reg.get_model());

        let ran = Arc::new(AtomicBool::new(false));
        harness.set_tools(vec![echo_tool(ran.clone())], None).await;

        let final_message = harness
            .prompt("Please echo 'hi'.")
            .await
            .expect("harness prompt should complete via the global faux dispatch");

        // ── 1. Tool actually executed through the real harness path ─────────
        assert!(ran.load(Ordering::SeqCst), "echo tool must have executed");

        // ── 2. prompt() returned the terminal assistant answer ──────────────
        assert_eq!(final_message.stop_reason, StopReason::Stop);

        // ── 3. PERSISTENCE: re-read the session and assert the full transcript
        //        was durably appended (user → assistant(toolcall) → toolResult →
        //        assistant(final)) ──────────────────────────────────────────────
        let restored = session
            .build_context()
            .await
            .expect("session must rebuild context")
            .messages;
        let roles: Vec<&str> = restored.iter().map(role_of).collect();
        assert_eq!(
            roles,
            vec!["user", "assistant", "toolResult", "assistant"],
            "session must persist the complete transcript"
        );

        // ── 4. The persisted assistant message carried the tool call ────────
        let persisted_assistant = match &restored[1] {
            AgentMessage::Assistant(a) => a,
            _ => panic!("expected assistant at index 1"),
        };
        assert!(
            persisted_assistant
                .content
                .iter()
                .any(|b| matches!(b, AssistantContentBlock::ToolCall(tc) if tc.name == "echo")),
            "persisted assistant message must contain the echo tool call"
        );

        // ── 5. The persisted tool result carries the executed output ────────
        let persisted_tool_result = match &restored[2] {
            AgentMessage::ToolResult(t) => t,
            _ => panic!("expected toolResult at index 2"),
        };
        assert_eq!(persisted_tool_result.tool_name, "echo");
        assert!(!persisted_tool_result.is_error);
        match persisted_tool_result.content.first() {
            Some(MessageContent::Text(t)) => assert_eq!(t.text, "echoed: hi"),
            _ => panic!("tool result must persist text content"),
        }

        reg.unregister();
    }

    /// P0-A on-disk durability: the same prompt path, but backed by
    /// `JsonlSessionStorage` writing to a real file. We then OPEN a fresh
    /// storage instance from that file and assert the transcript was persisted
    /// to disk (not just held in memory).
    #[tokio::test]
    async fn e2e_harness_prompt_persists_to_jsonl_file_on_disk() {
        use crate::harness::session::jsonl_storage::JsonlSessionStorage;
        use crate::harness::types::JsonlSessionMetadata;

        let _registry_guard = crate::faux_registry_guard();
        let reg = Arc::new(register_faux_provider(RegisterFauxProviderOptions {
            api_enum: Some(Api::GoogleGenerativeAi),
            ..Default::default()
        }));
        reg.set_responses(vec![
            faux_assistant_message(
                vec![
                    FauxContentBlock::Text(faux_text("Echoing.")),
                    FauxContentBlock::ToolCall(faux_tool_call(
                        "echo",
                        serde_json::json!({ "message": "hi" }),
                        Some("tool-1".to_string()),
                    )),
                ],
                FauxAssistantMessageOptions {
                    stop_reason: Some(StopReason::ToolUse),
                    ..Default::default()
                },
            )
            .into(),
            faux_assistant_message("Done.", FauxAssistantMessageOptions::default()).into(),
        ]);

        let dir = tempfile::tempdir().expect("tempdir");
        let file_path = dir
            .path()
            .join("session.jsonl")
            .to_string_lossy()
            .into_owned();
        let cwd = dir.path().to_string_lossy().into_owned();

        let storage = JsonlSessionStorage::create(
            crate::harness::env::nodejs::NodeExecutionEnv::new(dir.path()),
            file_path.clone(),
            cwd,
            "test-session-1".to_string(),
            None,
        )
        .await
        .expect("create jsonl session");
        let session: Session<JsonlSessionMetadata> = Session::new(Arc::new(storage));

        let env: Arc<dyn ExecutionEnv> = Arc::new(
            crate::harness::env::nodejs::NodeExecutionEnv::new(dir.path()),
        );
        let harness = AgentHarness::new(env, session, reg.get_model());
        let ran = Arc::new(AtomicBool::new(false));
        harness.set_tools(vec![echo_tool(ran.clone())], None).await;

        harness.prompt("echo hi").await.expect("prompt ok");
        assert!(ran.load(Ordering::SeqCst), "tool must execute");

        // The file must exist and be non-trivial.
        assert!(
            std::path::Path::new(&file_path).exists(),
            "session JSONL file must be written to disk"
        );

        // Reopen with a FRESH storage instance → proves bytes hit the file.
        let reopened = JsonlSessionStorage::open(
            crate::harness::env::nodejs::NodeExecutionEnv::new(dir.path()),
            file_path.clone(),
        )
        .await
        .expect("reopen jsonl session from disk");
        let reopened_session: Session<JsonlSessionMetadata> = Session::new(Arc::new(reopened));
        let restored = reopened_session
            .build_context()
            .await
            .expect("rebuild context from disk")
            .messages;
        let roles: Vec<&str> = restored.iter().map(role_of).collect();
        assert_eq!(
            roles,
            vec!["user", "assistant", "toolResult", "assistant"],
            "full transcript must be durable across a fresh open from disk"
        );
        reg.unregister();
    }
}
