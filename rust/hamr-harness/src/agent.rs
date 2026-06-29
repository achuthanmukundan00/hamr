//! Port of `packages/agent/src/agent.ts`.

use crate::agent_loop::{run_agent_loop, run_agent_loop_continue};
use crate::harness::messages::convert_to_llm;
use crate::types::{
    AgentContext, AgentEvent, AgentLoopConfig, AgentMessage, AgentState, AgentTool, StreamFn,
    ToolExecutionMode,
};
use chrono::Utc;
use hamr_ai::stream::{StreamError, stream_simple};
use hamr_ai::types::{
    AssistantMessage, Message, MessageContent, MessageRole, Model, TextContent, ThinkingLevel,
    Transport, UserMessage,
};
use std::collections::HashSet;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::{Mutex, Notify, watch};

type Subscriber = Arc<dyn Fn(AgentEvent) -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + Sync>;

#[derive(Clone)]
struct PendingRun {
    notify: Arc<Notify>,
    abort_tx: watch::Sender<bool>,
}

struct Inner {
    state: AgentState,
    tools: Vec<AgentTool>,
    tool_execution: ToolExecutionMode,
    transport: Option<Transport>,
    max_retry_delay_ms: Option<u64>,
    reasoning: Option<ThinkingLevel>,
    session_id: Option<String>,
    stream_fn: StreamFn,
    convert_to_llm: ConvertToLlmFn,
    get_api_key: Option<GetApiKeyFn>,
    subscribers: Vec<Subscriber>,
    pending: Option<PendingRun>,
}

type ConvertToLlmFn = Arc<
    dyn Fn(Vec<AgentMessage>) -> Pin<Box<dyn Future<Output = Vec<Message>> + Send>> + Send + Sync,
>;

type GetApiKeyFn =
    Arc<dyn Fn(String) -> Pin<Box<dyn Future<Output = Option<String>> + Send>> + Send + Sync>;

pub struct Agent {
    inner: Arc<Mutex<Inner>>,
}

impl Clone for Agent {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

pub struct AgentOptions {
    pub system_prompt: String,
    pub model: Model,
    pub thinking_level: Option<ThinkingLevel>,
    pub tools: Vec<AgentTool>,
    pub stream_fn: Option<StreamFn>,
    pub convert_to_llm: Option<ConvertToLlmFn>,
    pub get_api_key: Option<GetApiKeyFn>,
    pub session_id: Option<String>,
    pub tool_execution: ToolExecutionMode,
    pub transport: Option<Transport>,
    pub max_retry_delay_ms: Option<u64>,
}

impl Agent {
    pub fn new(options: AgentOptions) -> Self {
        let stream_fn = options.stream_fn.unwrap_or_else(|| {
            Arc::new(|model, context, options| {
                Box::pin(async move { stream_simple(model, context, options) })
                    as Pin<Box<dyn Future<Output = Result<_, StreamError>> + Send>>
            })
        });
        let convert_to_llm = options.convert_to_llm.unwrap_or_else(|| {
            Arc::new(|messages: Vec<AgentMessage>| {
                Box::pin(async move { convert_to_llm(&messages) })
                    as Pin<Box<dyn Future<Output = Vec<Message>> + Send>>
            })
        });
        Self {
            inner: Arc::new(Mutex::new(Inner {
                state: AgentState {
                    system_prompt: options.system_prompt,
                    model: options.model,
                    thinking_level: options.thinking_level.unwrap_or(ThinkingLevel::Low),
                    tools: options.tools.iter().map(|tool| tool.name.clone()).collect(),
                    messages: Vec::new(),
                    is_streaming: false,
                    streaming_message: None,
                    pending_tool_calls: HashSet::new(),
                    error_message: None,
                },
                tools: options.tools,
                tool_execution: options.tool_execution,
                transport: options.transport,
                max_retry_delay_ms: options.max_retry_delay_ms,
                reasoning: options.thinking_level,
                session_id: options.session_id,
                stream_fn,
                convert_to_llm,
                get_api_key: options.get_api_key,
                subscribers: Vec::new(),
                pending: None,
            })),
        }
    }

    pub async fn subscribe(&self, listener: Subscriber) {
        self.inner.lock().await.subscribers.push(listener);
    }

    async fn emit(&self, event: AgentEvent) {
        let subscribers = { self.inner.lock().await.subscribers.clone() };
        for subscriber in subscribers {
            subscriber(event.clone()).await;
        }
    }

    async fn with_run<F, Fut>(&self, run: F) -> Result<Vec<AgentMessage>, String>
    where
        F: FnOnce(
                AgentContext,
                AgentLoopConfig,
                Arc<dyn Fn(AgentEvent) -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + Sync>,
                watch::Sender<bool>,
                StreamFn,
            ) -> Fut
            + Send
            + 'static,
        Fut: Future<Output = Result<Vec<AgentMessage>, String>> + Send,
    {
        let (context, config, stream_fn, abort_tx) = {
            let mut inner = self.inner.lock().await;
            if inner.pending.is_some() {
                return Err("Agent is busy".to_string());
            }
            let (abort_tx, _) = watch::channel(false);
            inner.pending = Some(PendingRun {
                notify: Arc::new(Notify::new()),
                abort_tx: abort_tx.clone(),
            });
            inner.state.is_streaming = true;

            let context = AgentContext {
                system_prompt: inner.state.system_prompt.clone(),
                messages: inner.state.messages.clone(),
                tools: inner.tools.clone(),
            };
            let config = AgentLoopConfig {
                model: inner.state.model.clone(),
                reasoning: inner.reasoning,
                session_id: inner.session_id.clone(),
                transport: inner.transport,
                tool_execution: inner.tool_execution,
                max_retry_delay_ms: inner.max_retry_delay_ms,
                convert_to_llm: Arc::clone(&inner.convert_to_llm),
                transform_context: None,
                get_api_key: inner.get_api_key.clone(),
                should_stop_after_turn: None,
                prepare_next_turn: None,
                get_steering_messages: None,
                get_follow_up_messages: None,
                before_tool_call: None,
                after_tool_call: None,
            };
            (context, config, Arc::clone(&inner.stream_fn), abort_tx)
        };

        let agent = self.clone();
        let on_event = Arc::new(move |event: AgentEvent| {
            let agent = agent.clone();
            Box::pin(async move {
                match &event {
                    AgentEvent::MessageStart { message }
                        if matches!(message, AgentMessage::Assistant(_)) =>
                    {
                        agent.inner.lock().await.state.streaming_message = Some(message.clone());
                    }
                    AgentEvent::MessageEnd { message } => {
                        let mut inner = agent.inner.lock().await;
                        inner.state.messages.push(message.clone());
                        inner.state.streaming_message = None;
                        if let AgentMessage::Assistant(message) = message {
                            inner.state.error_message = message.error_message.clone();
                        }
                    }
                    AgentEvent::ToolExecutionStart { tool_call_id, .. } => {
                        agent
                            .inner
                            .lock()
                            .await
                            .state
                            .pending_tool_calls
                            .insert(tool_call_id.clone());
                    }
                    AgentEvent::ToolExecutionEnd { tool_call_id, .. } => {
                        agent
                            .inner
                            .lock()
                            .await
                            .state
                            .pending_tool_calls
                            .remove(tool_call_id);
                    }
                    AgentEvent::AgentEnd { .. } => {
                        let pending = {
                            let mut inner = agent.inner.lock().await;
                            inner.state.is_streaming = false;
                            inner.pending.take()
                        };
                        if let Some(pending) = pending {
                            pending.notify.notify_waiters();
                        }
                    }
                    _ => {}
                }
                agent.emit(event).await;
            }) as Pin<Box<dyn Future<Output = ()> + Send>>
        });

        run(context, config, on_event, abort_tx, stream_fn).await
    }

    pub async fn prompt(&self, text: impl Into<String>) -> Result<AssistantMessage, String> {
        let message = AgentMessage::User(UserMessage {
            role: MessageRole::User,
            content: vec![MessageContent::Text(TextContent {
                text: text.into(),
                text_signature: None,
            })],
            timestamp: Utc::now(),
        });
        let result = self
            .with_run(
                move |context, config, on_event, abort_tx, stream_fn| async move {
                    run_agent_loop(
                        vec![message],
                        context,
                        config,
                        on_event,
                        Some(abort_tx.subscribe()),
                        Some(abort_tx.subscribe()),
                        stream_fn,
                    )
                    .await
                },
            )
            .await?;

        result
            .into_iter()
            .rev()
            .find_map(|message| match message {
                AgentMessage::Assistant(message) => Some(message),
                _ => None,
            })
            .ok_or_else(|| "run completed without assistant message".to_string())
    }

    /// Set the agent's tools. Mirrors TS `agent.state.tools = tools`.
    pub async fn set_tools(&self, tools: Vec<AgentTool>) {
        let mut inner = self.inner.lock().await;
        inner.tools = tools.clone();
        inner.state.tools = tools.into_iter().map(|t| t.name).collect();
    }

    /// Replace the restored transcript before the next run.
    pub async fn set_messages(&self, messages: Vec<AgentMessage>) {
        self.inner.lock().await.state.messages = messages;
    }

    /// Get a snapshot of the agent's current state.
    pub async fn state(&self) -> AgentStateSnapshot {
        let inner = self.inner.lock().await;
        AgentStateSnapshot {
            system_prompt: inner.state.system_prompt.clone(),
            model: inner.state.model.clone(),
            thinking_level: inner.state.thinking_level,
            tools: inner.state.tools.clone(),
            messages: inner.state.messages.clone(),
            is_streaming: inner.state.is_streaming,
            streaming_message: inner.state.streaming_message.clone(),
            pending_tool_calls: inner.state.pending_tool_calls.clone(),
            error_message: inner.state.error_message.clone(),
        }
    }

    /// Send a prompt from a batch of pre-built messages.
    /// Mirrors TS `Agent.prompt(messages: AgentMessage[])`.
    pub async fn prompt_messages(
        &self,
        messages: Vec<AgentMessage>,
    ) -> Result<Vec<AgentMessage>, String> {
        self.with_run(
            move |context, config, on_event, abort_tx, stream_fn| async move {
                run_agent_loop(
                    messages,
                    context,
                    config,
                    on_event,
                    Some(abort_tx.subscribe()),
                    Some(abort_tx.subscribe()),
                    stream_fn,
                )
                .await
            },
        )
        .await
    }

    pub async fn continue_run(&self) -> Result<Vec<AgentMessage>, String> {
        self.with_run(
            move |context, config, on_event, abort_tx, stream_fn| async move {
                run_agent_loop_continue(
                    context,
                    config,
                    on_event,
                    Some(abort_tx.subscribe()),
                    Some(abort_tx.subscribe()),
                    stream_fn,
                )
                .await
            },
        )
        .await
    }

    pub async fn abort(&self) {
        let pending = { self.inner.lock().await.pending.clone() };
        if let Some(pending) = pending {
            let _ = pending.abort_tx.send(true);
        }
    }

    pub async fn wait_for_idle(&self) {
        let pending = { self.inner.lock().await.pending.clone() };
        if let Some(pending) = pending {
            pending.notify.notified().await;
        }
    }
}

/// Read-only snapshot of agent state.
/// Mirrors the TS `AgentState` interface.
#[derive(Debug, Clone)]
pub struct AgentStateSnapshot {
    pub system_prompt: String,
    pub model: Model,
    pub thinking_level: ThinkingLevel,
    pub tools: Vec<String>,
    pub messages: Vec<AgentMessage>,
    pub is_streaming: bool,
    pub streaming_message: Option<AgentMessage>,
    pub pending_tool_calls: HashSet<String>,
    pub error_message: Option<String>,
}
