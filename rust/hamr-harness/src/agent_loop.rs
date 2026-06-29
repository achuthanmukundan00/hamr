//! Port of `packages/agent/src/agent-loop.ts`.
//!
//! Implements the complete agent loop algorithm:
//! 1. Emit AgentStart
//! 2. Process initial_messages
//! 3. LOOP (outer: follow-up → inner: tool-calls + steering):
//!    - Emit TurnStart (skip first if already emitted above)
//!    - Process pending steering messages before the assistant response
//!    - Transform context → Convert to LLM → Stream from model
//!    - Collect stream events, accumulate AssistantMessage
//!    - Check stop_reason (error/aborted → early exit)
//!    - Push assistant to context
//!    - Extract tool calls → validate → execute (sequential/parallel)
//!    - For each tool: before_tool_call → execute → after_tool_call
//!    - Check termination (all terminate=true → stop)
//!    - Emit TurnEnd
//!    - prepare_next_turn + should_stop_after_turn
//!    - Drain steering → continue inner loop
//!    - Drain follow-up → continue outer loop
//!    - Stop

use crate::types::{
    AfterToolCallContext, AgentContext, AgentEvent, AgentLoopConfig, AgentLoopTurnUpdate,
    AgentMessage, AgentTool, AgentToolResult, BeforeToolCallContext, ToolExecutionMode,
};
use chrono::Utc;
use hamr_ai::types::{
    AssistantContentBlock, AssistantMessage, AssistantMessageEvent, Context, Message, MessageRole,
    Model, SimpleStreamOptions, StopReason, ToolCall, ToolResultMessage,
};
use std::sync::Arc;

type EventCallback = Arc<
    dyn Fn(AgentEvent) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>>
        + Send
        + Sync,
>;

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn create_provider_context(context: &AgentContext, llm_messages: Vec<Message>) -> Context {
    Context {
        system_prompt: Some(context.system_prompt.clone()),
        messages: llm_messages,
        tools: context.tools.iter().map(AgentTool::to_llm_tool).collect(),
    }
}

fn tool_calls_from_message(message: &AssistantMessage) -> Vec<ToolCall> {
    message
        .content
        .iter()
        .filter_map(|block| match block {
            AssistantContentBlock::ToolCall(tool_call) => Some(tool_call.clone()),
            _ => None,
        })
        .collect()
}

fn find_tool<'a>(tools: &'a [AgentTool], name: &str) -> Option<&'a AgentTool> {
    tools.iter().find(|tool| tool.name == name)
}

fn blocked_tool_result(
    tool_call: &ToolCall,
    reason: Option<String>,
) -> (ToolResultMessage, AgentToolResult) {
    let text = reason.unwrap_or_else(|| "Tool call was blocked".to_string());
    let content = vec![hamr_ai::types::MessageContent::Text(
        hamr_ai::types::TextContent {
            text,
            text_signature: None,
        },
    )];
    (
        ToolResultMessage {
            role: MessageRole::ToolResult,
            tool_call_id: tool_call.id.clone(),
            tool_name: tool_call.name.clone(),
            content: content.clone(),
            details: None,
            is_error: true,
            timestamp: Utc::now(),
        },
        AgentToolResult {
            content,
            details: None,
            is_error: true,
            terminate: false,
        },
    )
}

fn tool_result_message(tool_call: &ToolCall, result: &AgentToolResult) -> ToolResultMessage {
    ToolResultMessage {
        role: MessageRole::ToolResult,
        tool_call_id: tool_call.id.clone(),
        tool_name: tool_call.name.clone(),
        content: result.content.clone(),
        details: result.details.clone(),
        is_error: result.is_error,
        timestamp: Utc::now(),
    }
}

async fn apply_turn_update(
    update: AgentLoopTurnUpdate,
    context: &mut AgentContext,
    model: &mut Model,
    reasoning: &mut Option<hamr_ai::types::ThinkingLevel>,
) {
    if let Some(next_context) = update.context {
        *context = next_context;
    }
    if let Some(next_model) = update.model {
        *model = next_model;
    }
    if let Some(next_reasoning) = update.thinking_level {
        *reasoning = Some(next_reasoning);
    }
}

// ─── Tool execution ──────────────────────────────────────────────────────────

/// Validate tool arguments against the tool's JSON schema.
/// Mirrors the TS `prepareToolCall` / `validateToolArguments` step.
fn validate_tool_args(
    tool: &AgentTool,
    tool_call: &ToolCall,
    args: &serde_json::Value,
) -> Result<serde_json::Value, String> {
    // Use hamr_ai's validator — mirrors TS `validateToolArguments(tool, toolCall)`
    let llm_tool = tool.to_llm_tool();
    let tc = ToolCall {
        id: tool_call.id.clone(),
        name: tool_call.name.clone(),
        arguments: args.clone(),
        thought_signature: tool_call.thought_signature.clone(),
    };
    hamr_ai::utils::validation::validate_tool_arguments(&llm_tool, &tc)
}

/// Execute a single tool call with full prepare→execute→finalize pipeline.
/// Mirrors the TS `prepareToolCall` + `executePreparedToolCall` + `finalizeExecutedToolCall`.
async fn execute_tool_call(
    tool_call: ToolCall,
    context: AgentContext,
    config: Arc<AgentLoopConfig>,
    assistant_message: AssistantMessage,
    tool_signal: Option<tokio::sync::watch::Receiver<bool>>,
    on_event: EventCallback,
) -> Result<(ToolResultMessage, AgentToolResult), String> {
    // ── Find tool ──────────────────────────────────────────────────────────
    let Some(tool) = find_tool(&context.tools, &tool_call.name).cloned() else {
        let (tool_result_message, result) = blocked_tool_result(
            &tool_call,
            Some(format!("Tool {} not found", tool_call.name)),
        );
        return Ok((tool_result_message, result));
    };

    // ── Prepare arguments (TS prepareToolCallArguments) ────────────────────
    let args = tool
        .prepare_arguments
        .as_ref()
        .map(|prepare| prepare(tool_call.arguments.clone()))
        .unwrap_or_else(|| tool_call.arguments.clone());

    // ── Validate arguments (TS validateToolArguments) ──────────────────────
    if let Err(validation_err) = validate_tool_args(&tool, &tool_call, &args) {
        let (tool_result_message, result) = blocked_tool_result(&tool_call, Some(validation_err));
        return Ok((tool_result_message, result));
    }

    // ── before_tool_call hook (TS beforeToolCall) ──────────────────────────
    if let Some(before_tool_call) = &config.before_tool_call {
        if let Some(result) = before_tool_call(
            BeforeToolCallContext {
                assistant_message: assistant_message.clone(),
                tool_call: tool_call.clone(),
                args: args.clone(),
                context: context.clone(),
            },
            tool_signal.clone(),
        )
        .await
        {
            if result.block {
                let (tool_result_message, agent_result) =
                    blocked_tool_result(&tool_call, result.reason);
                return Ok((tool_result_message, agent_result));
            }
        }
    }

    // ── Emit start + execute (TS executePreparedToolCall) ──────────────────
    on_event(AgentEvent::ToolExecutionStart {
        tool_call_id: tool_call.id.clone(),
        tool_name: tool_call.name.clone(),
        args: args.clone(),
    })
    .await;

    let on_update_callback = {
        let tool_call = tool_call.clone();
        let args = args.clone();
        let on_event = Arc::clone(&on_event);
        Arc::new(move |partial_result: AgentToolResult| {
            let on_event = Arc::clone(&on_event);
            let tool_call = tool_call.clone();
            let args = args.clone();
            tokio::spawn(async move {
                on_event(AgentEvent::ToolExecutionUpdate {
                    tool_call_id: tool_call.id.clone(),
                    tool_name: tool_call.name.clone(),
                    args,
                    partial_result: serde_json::json!({
                        "content": partial_result.content,
                        "details": partial_result.details,
                        "isError": partial_result.is_error,
                        "terminate": partial_result.terminate,
                    }),
                })
                .await;
            });
        }) as Arc<dyn Fn(AgentToolResult) + Send + Sync>
    };

    let mut result = (tool.execute)(
        tool_call.id.clone(),
        args.clone(),
        tool_signal.clone(),
        Some(on_update_callback),
    )
    .await;

    // ── after_tool_call hook (TS finalizeExecutedToolCall) ─────────────────
    if let Some(after_tool_call) = &config.after_tool_call {
        if let Some(patch) = after_tool_call(
            AfterToolCallContext {
                assistant_message: assistant_message.clone(),
                tool_call: tool_call.clone(),
                args: args.clone(),
                result: result.clone(),
                is_error: result.is_error,
                context: context.clone(),
            },
            tool_signal.clone(),
        )
        .await
        {
            if let Some(content) = patch.content {
                result.content = content;
            }
            if let Some(details) = patch.details {
                result.details = Some(details);
            }
            if let Some(is_error) = patch.is_error {
                result.is_error = is_error;
            }
            if let Some(terminate) = patch.terminate {
                result.terminate = terminate;
            }
        }
    }

    // ── Emit end (TS emitToolExecutionEnd) ─────────────────────────────────
    on_event(AgentEvent::ToolExecutionEnd {
        tool_call_id: tool_call.id.clone(),
        tool_name: tool_call.name.clone(),
        result: serde_json::json!({
            "content": result.content,
            "details": result.details,
            "isError": result.is_error,
            "terminate": result.terminate,
        }),
        is_error: result.is_error,
    })
    .await;

    Ok((tool_result_message(&tool_call, &result), result))
}

/// Execute a batch of tool calls (sequential or parallel).
async fn execute_tool_batch(
    tool_calls: Vec<ToolCall>,
    context: AgentContext,
    config: Arc<AgentLoopConfig>,
    assistant_message: AssistantMessage,
    tool_execution: ToolExecutionMode,
    tool_signal: Option<tokio::sync::watch::Receiver<bool>>,
    on_event: EventCallback,
) -> Result<Vec<(ToolResultMessage, AgentToolResult)>, String> {
    let run_sequential = tool_execution == ToolExecutionMode::Sequential
        || tool_calls.iter().any(|tool_call| {
            find_tool(&context.tools, &tool_call.name).and_then(|tool| tool.execution_mode)
                == Some(ToolExecutionMode::Sequential)
        });

    if !run_sequential {
        let mut tasks = Vec::new();
        for tool_call in tool_calls {
            tasks.push(tokio::spawn(execute_tool_call(
                tool_call,
                context.clone(),
                Arc::clone(&config),
                assistant_message.clone(),
                tool_signal.clone(),
                Arc::clone(&on_event),
            )));
        }
        let mut results = Vec::new();
        for task in tasks {
            results.push(task.await.map_err(|e| e.to_string())??);
        }
        Ok(results)
    } else {
        let mut results = Vec::new();
        for tool_call in tool_calls {
            results.push(
                execute_tool_call(
                    tool_call,
                    context.clone(),
                    Arc::clone(&config),
                    assistant_message.clone(),
                    tool_signal.clone(),
                    Arc::clone(&on_event),
                )
                .await?,
            );
        }
        Ok(results)
    }
}

/// Check whether the entire tool batch requests termination.
fn should_terminate_tool_batch(finalized: &[(ToolResultMessage, AgentToolResult)]) -> bool {
    !finalized.is_empty() && finalized.iter().all(|(_, result)| result.terminate)
}

// ─── Stream assistant response ───────────────────────────────────────────────

/// Stream a single assistant response from the LLM.
/// Mirrors the TS `streamAssistantResponse`.
async fn stream_assistant_response(
    context: &mut AgentContext,
    config: &Arc<AgentLoopConfig>,
    model: &Model,
    reasoning: &Option<hamr_ai::types::ThinkingLevel>,
    lifecycle_signal: Option<tokio::sync::watch::Receiver<bool>>,
    on_event: &EventCallback,
    stream_fn: &crate::types::StreamFn,
) -> Result<AssistantMessage, String> {
    // Apply context transform if configured (AgentMessage[] → AgentMessage[])
    let messages = if let Some(transform_context) = &config.transform_context {
        transform_context(context.messages.clone(), lifecycle_signal.clone()).await
    } else {
        context.messages.clone()
    };

    // Convert to LLM-compatible messages (AgentMessage[] → Message[])
    let llm_messages = (config.convert_to_llm)(messages).await;

    // Build LLM context
    let provider_context = create_provider_context(context, llm_messages);

    let mut stream_options = SimpleStreamOptions::default();
    stream_options.base.session_id = config.session_id.clone();
    stream_options.base.transport = config.transport;
    stream_options.base.max_retry_delay_ms = config.max_retry_delay_ms;
    stream_options.base.signal = lifecycle_signal.clone();
    stream_options.reasoning = *reasoning;
    if let Some(get_api_key) = &config.get_api_key {
        stream_options.base.api_key = get_api_key(model.provider.clone()).await;
    }

    let mut stream = stream_fn(model.clone(), provider_context, Some(stream_options))
        .await
        .map_err(|error| error.to_string())?;

    let mut partial_message: Option<AssistantMessage> = None;
    let mut added_partial = false;

    while let Some(event) = stream.next_event().await {
        match &event {
            AssistantMessageEvent::Start { partial } => {
                partial_message = Some(partial.clone());
                context
                    .messages
                    .push(AgentMessage::Assistant(partial.clone()));
                added_partial = true;
                on_event(AgentEvent::MessageStart {
                    message: AgentMessage::Assistant(partial.clone()),
                })
                .await;
            }
            AssistantMessageEvent::TextStart { partial, .. }
            | AssistantMessageEvent::TextDelta { partial, .. }
            | AssistantMessageEvent::TextEnd { partial, .. }
            | AssistantMessageEvent::ThinkingStart { partial, .. }
            | AssistantMessageEvent::ThinkingDelta { partial, .. }
            | AssistantMessageEvent::ThinkingEnd { partial, .. }
            | AssistantMessageEvent::ToolCallStart { partial, .. }
            | AssistantMessageEvent::ToolCallDelta { partial, .. }
            | AssistantMessageEvent::ToolCallEnd { partial, .. } => {
                partial_message = Some(partial.clone());
                if added_partial {
                    // Replace the last partial in context
                    if let Some(last) = context.messages.last_mut() {
                        *last = AgentMessage::Assistant(partial.clone());
                    }
                }
                on_event(AgentEvent::MessageUpdate {
                    message: AgentMessage::Assistant(partial.clone()),
                    assistant_message_event: event,
                })
                .await;
            }
            AssistantMessageEvent::Loading { model, elapsed_ms } => {
                on_event(AgentEvent::ModelLoading {
                    model: model.clone(),
                    elapsed_ms: *elapsed_ms,
                })
                .await;
            }
            AssistantMessageEvent::Done { message, .. } => {
                let final_msg = message.clone();
                if added_partial {
                    if let Some(last) = context.messages.last_mut() {
                        *last = AgentMessage::Assistant(final_msg.clone());
                    }
                } else {
                    context
                        .messages
                        .push(AgentMessage::Assistant(final_msg.clone()));
                }
                if !added_partial {
                    on_event(AgentEvent::MessageStart {
                        message: AgentMessage::Assistant(final_msg.clone()),
                    })
                    .await;
                }
                on_event(AgentEvent::MessageEnd {
                    message: AgentMessage::Assistant(final_msg.clone()),
                })
                .await;
                return Ok(final_msg);
            }
            AssistantMessageEvent::Error { error, .. } => {
                let final_msg = error.clone();
                if added_partial {
                    if let Some(last) = context.messages.last_mut() {
                        *last = AgentMessage::Assistant(final_msg.clone());
                    }
                } else {
                    context
                        .messages
                        .push(AgentMessage::Assistant(final_msg.clone()));
                }
                if !added_partial {
                    on_event(AgentEvent::MessageStart {
                        message: AgentMessage::Assistant(final_msg.clone()),
                    })
                    .await;
                }
                on_event(AgentEvent::MessageEnd {
                    message: AgentMessage::Assistant(final_msg.clone()),
                })
                .await;
                return Ok(final_msg);
            }
        }
    }

    // Stream ended without Done/Error — use last partial or fail
    if let Some(partial) = partial_message {
        if added_partial {
            if let Some(last) = context.messages.last_mut() {
                *last = AgentMessage::Assistant(partial.clone());
            }
        } else {
            context
                .messages
                .push(AgentMessage::Assistant(partial.clone()));
        }
        if !added_partial {
            on_event(AgentEvent::MessageStart {
                message: AgentMessage::Assistant(partial.clone()),
            })
            .await;
        }
        on_event(AgentEvent::MessageEnd {
            message: AgentMessage::Assistant(partial.clone()),
        })
        .await;
        return Ok(partial);
    }

    Err("provider stream ended without a terminal assistant message".to_string())
}

// ─── Main loop ───────────────────────────────────────────────────────────────

/// Main agent-loop logic shared by `run_agent_loop` and `run_agent_loop_continue`.
/// Mirrors the TS `runLoop` with outer (follow-up) and inner (tool-call / steering) loops.
async fn run_loop(
    mut context: AgentContext,
    new_messages: &mut Vec<AgentMessage>,
    config: Arc<AgentLoopConfig>,
    mut model: Model,
    mut reasoning: Option<hamr_ai::types::ThinkingLevel>,
    tool_execution: ToolExecutionMode,
    lifecycle_signal: Option<tokio::sync::watch::Receiver<bool>>,
    tool_signal: Option<tokio::sync::watch::Receiver<bool>>,
    on_event: EventCallback,
    stream_fn: crate::types::StreamFn,
    first_turn: bool,
) -> Result<(), String> {
    let mut first_turn = first_turn;

    // Check for steering messages at start (user may have typed while waiting)
    let mut pending_messages: Vec<AgentMessage> =
        if let Some(get_steering) = &config.get_steering_messages {
            get_steering().await
        } else {
            Vec::new()
        };

    // ── Outer loop: continues when queued follow-up messages arrive ────────
    loop {
        let mut has_more_tool_calls = true;

        // ── Inner loop: process tool calls and steering messages ───────────
        while has_more_tool_calls || !pending_messages.is_empty() {
            // Emit turn_start (skip for the very first turn, already emitted above)
            if !first_turn {
                on_event(AgentEvent::TurnStart).await;
            } else {
                first_turn = false;
            }

            // ── Process pending steering messages BEFORE assistant response ──
            // (TS: injects before the next assistant response so the model sees them immediately)
            if !pending_messages.is_empty() {
                for message in pending_messages.drain(..) {
                    on_event(AgentEvent::MessageStart {
                        message: message.clone(),
                    })
                    .await;
                    on_event(AgentEvent::MessageEnd {
                        message: message.clone(),
                    })
                    .await;
                    context.messages.push(message.clone());
                    new_messages.push(message);
                }
            }

            // ── Stream assistant response ──────────────────────────────────
            let assistant_message = stream_assistant_response(
                &mut context,
                &config,
                &model,
                &reasoning,
                lifecycle_signal.clone(),
                &on_event,
                &stream_fn,
            )
            .await?;

            new_messages.push(AgentMessage::Assistant(assistant_message.clone()));

            // ── Check stop_reason for error / aborted (TS early exit) ──────
            match assistant_message.stop_reason {
                StopReason::Error | StopReason::Aborted => {
                    on_event(AgentEvent::TurnEnd {
                        message: AgentMessage::Assistant(assistant_message),
                        tool_results: Vec::new(),
                    })
                    .await;
                    on_event(AgentEvent::AgentEnd {
                        messages: new_messages.clone(),
                    })
                    .await;
                    return Ok(());
                }
                _ => {}
            }

            // ── Extract tool calls ─────────────────────────────────────────
            let tool_calls = tool_calls_from_message(&assistant_message);
            let mut tool_results: Vec<ToolResultMessage> = Vec::new();
            has_more_tool_calls = false;

            if !tool_calls.is_empty() {
                // Execute tool batch
                let finalized_results = execute_tool_batch(
                    tool_calls,
                    context.clone(),
                    Arc::clone(&config),
                    assistant_message.clone(),
                    tool_execution,
                    tool_signal.clone(),
                    Arc::clone(&on_event),
                )
                .await?;

                // Determine if batch requests termination
                let should_terminate = should_terminate_tool_batch(&finalized_results);
                has_more_tool_calls = !should_terminate;

                // Emit tool result messages and push to context
                for (message, _) in &finalized_results {
                    let agent_message = AgentMessage::ToolResult(message.clone());
                    on_event(AgentEvent::MessageStart {
                        message: agent_message.clone(),
                    })
                    .await;
                    on_event(AgentEvent::MessageEnd {
                        message: agent_message.clone(),
                    })
                    .await;
                    context.messages.push(agent_message.clone());
                    new_messages.push(agent_message);
                    tool_results.push(message.clone());
                }
            }

            // ── Emit turn end ──────────────────────────────────────────────
            on_event(AgentEvent::TurnEnd {
                message: AgentMessage::Assistant(assistant_message.clone()),
                tool_results: tool_results.clone(),
            })
            .await;

            // ── prepare_next_turn ──────────────────────────────────────────
            if let Some(prepare_next_turn) = &config.prepare_next_turn {
                if let Some(update) = prepare_next_turn(crate::types::ShouldStopAfterTurnContext {
                    message: assistant_message.clone(),
                    tool_results: tool_results.clone(),
                    context: context.clone(),
                    new_messages: new_messages.clone(),
                })
                .await
                {
                    apply_turn_update(update, &mut context, &mut model, &mut reasoning).await;
                }
            }

            // ── should_stop_after_turn ─────────────────────────────────────
            if let Some(should_stop) = &config.should_stop_after_turn {
                if should_stop(crate::types::ShouldStopAfterTurnContext {
                    message: assistant_message.clone(),
                    tool_results: tool_results.clone(),
                    context: context.clone(),
                    new_messages: new_messages.clone(),
                })
                .await
                {
                    on_event(AgentEvent::AgentEnd {
                        messages: new_messages.clone(),
                    })
                    .await;
                    return Ok(());
                }
            }

            // ── Drain steering messages for next inner-loop iteration ──────
            // (TS: getSteeringMessages at the end of each inner-loop iteration;
            //  if any, the inner loop continues and they're injected before the
            //  next assistant response)
            pending_messages = if let Some(get_steering) = &config.get_steering_messages {
                get_steering().await
            } else {
                Vec::new()
            };
        }

        // ── Agent would stop here. Check for follow-up messages ────────────
        // (TS: if follow-up messages exist, set them as pending and continue outer loop)
        let follow_up_messages = if let Some(get_follow_up) = &config.get_follow_up_messages {
            get_follow_up().await
        } else {
            Vec::new()
        };

        if !follow_up_messages.is_empty() {
            pending_messages = follow_up_messages;
            continue; // restart outer loop → inner loop picks up pending
        }

        // ── No more messages, exit ─────────────────────────────────────────
        break;
    }

    on_event(AgentEvent::AgentEnd {
        messages: new_messages.clone(),
    })
    .await;
    Ok(())
}

// ─── Public API ──────────────────────────────────────────────────────────────

/// Start an agent loop with initial prompt messages.
/// The prompts are added to the context and events are emitted for them.
///
/// Mirrors the TS `runAgentLoop`.
pub async fn run_agent_loop(
    initial_messages: Vec<AgentMessage>,
    mut context: AgentContext,
    config: AgentLoopConfig,
    on_event: EventCallback,
    lifecycle_signal: Option<tokio::sync::watch::Receiver<bool>>,
    tool_signal: Option<tokio::sync::watch::Receiver<bool>>,
    stream_fn: crate::types::StreamFn,
) -> Result<Vec<AgentMessage>, String> {
    let mut new_messages: Vec<AgentMessage> = initial_messages.clone();
    let model = config.model.clone();
    let reasoning = config.reasoning;
    let tool_execution = config.tool_execution;
    let config = Arc::new(config);

    // 1. Emit AgentStart
    on_event(AgentEvent::AgentStart).await;

    // 2. Emit TurnStart (first turn — mirrors TS emitting turn_start here)
    on_event(AgentEvent::TurnStart).await;

    // 3. Process initial messages
    for message in initial_messages {
        on_event(AgentEvent::MessageStart {
            message: message.clone(),
        })
        .await;
        context.messages.push(message.clone());
        on_event(AgentEvent::MessageEnd { message }).await;
    }

    // 4. Enter main loop (skip first TurnStart since we emitted it above)
    run_loop(
        context,
        &mut new_messages,
        config,
        model,
        reasoning,
        tool_execution,
        lifecycle_signal,
        tool_signal,
        on_event,
        stream_fn,
        true, // first_turn = true → inner loop skips its own TurnStart
    )
    .await?;

    Ok(new_messages)
}

#[cfg(test)]
mod e2e_tests {
    //! End-to-end proof of the release-critical agent loop (P0-A), driven by the
    //! deterministic `faux` provider — no network, no API keys.
    //!
    //! Proves the full vertical:
    //!   user task → model stream → tool call → tool execution → tool result
    //!   returned to model → final answer → transcript serializes (persists).
    use super::*;
    use hamr_ai::providers::faux::{
        FauxAssistantMessageOptions, FauxContentBlock, RegisterFauxProviderOptions,
        faux_assistant_message, faux_text, faux_tool_call, register_faux_provider,
    };
    use hamr_ai::stream::StreamError;
    use hamr_ai::types::{MessageContent, TextContent, UserMessage};
    use std::pin::Pin;
    use std::sync::Mutex;
    use std::sync::atomic::{AtomicBool, Ordering};

    fn user_msg(text: &str) -> AgentMessage {
        AgentMessage::User(UserMessage {
            role: MessageRole::User,
            content: vec![MessageContent::Text(faux_text(text))],
            timestamp: Utc::now(),
        })
    }

    /// A deterministic tool that records that it ran and echoes its `message`
    /// argument back to the model. The `AtomicBool` is the observable side
    /// effect proving the tool actually executed.
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

    fn collector_callback(sink: Arc<Mutex<Vec<AgentEvent>>>) -> EventCallback {
        Arc::new(move |event| {
            let sink = sink.clone();
            Box::pin(async move {
                sink.lock().unwrap_or_else(|e| e.into_inner()).push(event);
            })
        })
    }

    fn minimal_config(model: Model) -> AgentLoopConfig {
        AgentLoopConfig {
            model,
            reasoning: None,
            session_id: None,
            transport: None,
            tool_execution: ToolExecutionMode::Sequential,
            max_retry_delay_ms: None,
            convert_to_llm: Arc::new(|messages| {
                Box::pin(async move { crate::harness::messages::convert_to_llm(&messages) })
            }),
            transform_context: None,
            get_api_key: None,
            should_stop_after_turn: None,
            prepare_next_turn: None,
            get_steering_messages: None,
            get_follow_up_messages: None,
            before_tool_call: None,
            after_tool_call: None,
        }
    }

    /// The headline P0-A test: a two-turn loop where the model first emits a
    /// tool call, the loop executes the tool, feeds the result back, and the
    /// model emits a final answer. Then we assert the transcript round-trips
    /// through serde (the same contract the JSONL session store relies on).
    #[tokio::test]
    async fn e2e_full_loop_toolcall_then_final_answer_persists() {
        let reg = Arc::new(register_faux_provider(
            RegisterFauxProviderOptions::default(),
        ));

        // Turn 1: assistant explains, then calls `echo`. stop_reason = ToolUse.
        // Turn 2: assistant produces the final answer. stop_reason = Stop.
        reg.set_responses(vec![
            faux_assistant_message(
                vec![
                    FauxContentBlock::Text(faux_text("Let me echo that for you.")),
                    FauxContentBlock::ToolCall(faux_tool_call(
                        "echo",
                        serde_json::json!({ "message": "hello" }),
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
                "All done — the echo tool returned the greeting.",
                FauxAssistantMessageOptions::default(),
            )
            .into(),
        ]);

        let ran = Arc::new(AtomicBool::new(false));
        let context = AgentContext {
            system_prompt: "You are a test agent.".to_string(),
            messages: Vec::new(),
            tools: vec![echo_tool(ran.clone())],
        };

        let events = Arc::new(Mutex::new(Vec::<AgentEvent>::new()));
        let on_event = collector_callback(events.clone());

        let reg_stream = Arc::clone(&reg);
        let stream_fn: crate::types::StreamFn = Arc::new(move |model, ctx, options| {
            let reg = Arc::clone(&reg_stream);
            Box::pin(async move { Ok(reg.stream_simple(model, ctx, options)) })
                as Pin<
                    Box<
                        dyn std::future::Future<
                                Output = Result<
                                    hamr_ai::utils::event_stream::AssistantMessageEventStream,
                                    StreamError,
                                >,
                            > + Send,
                    >,
                >
        });

        let result = run_agent_loop(
            vec![user_msg("Please echo 'hello'.")],
            context,
            minimal_config(reg.get_model()),
            on_event,
            None,
            None,
            stream_fn,
        )
        .await
        .expect("agent loop should complete without error");

        // ── 1. Tool actually executed (observable side effect) ──────────────
        assert!(ran.load(Ordering::SeqCst), "echo tool must have executed");

        // ── 2. Transcript shape: user → assistant(toolcall) → toolResult →
        //        assistant(final) ─────────────────────────────────────────────
        let roles: Vec<&str> = result
            .iter()
            .map(|m| match m {
                AgentMessage::User(_) => "user",
                AgentMessage::Assistant(_) => "assistant",
                AgentMessage::ToolResult(_) => "toolResult",
                _ => "other",
            })
            .collect();
        assert_eq!(
            roles,
            vec!["user", "assistant", "toolResult", "assistant"],
            "full loop transcript order"
        );

        // ── 3. First assistant message carried a tool call ──────────────────
        let first_assistant = match &result[1] {
            AgentMessage::Assistant(a) => a,
            _ => panic!("expected assistant at index 1"),
        };
        assert!(
            first_assistant
                .content
                .iter()
                .any(|b| matches!(b, AssistantContentBlock::ToolCall(tc) if tc.name == "echo")),
            "first assistant message must contain the echo tool call"
        );

        // ── 4. Tool result was fed back with the executed output ────────────
        let tool_result = match &result[2] {
            AgentMessage::ToolResult(t) => t,
            _ => panic!("expected toolResult at index 2"),
        };
        assert_eq!(tool_result.tool_name, "echo");
        assert!(!tool_result.is_error, "tool result must not be an error");
        let tool_text = match tool_result.content.first() {
            Some(MessageContent::Text(t)) => t.text.clone(),
            _ => panic!("tool result must have text content"),
        };
        assert_eq!(tool_text, "echoed: hello");

        // ── 5. Final assistant answer is present and terminal ───────────────
        let final_assistant = match result.last() {
            Some(AgentMessage::Assistant(a)) => a,
            _ => panic!("transcript must end with an assistant message"),
        };
        assert_eq!(final_assistant.stop_reason, StopReason::Stop);
        let final_text: String = final_assistant
            .content
            .iter()
            .filter_map(|b| match b {
                AssistantContentBlock::Text(t) => Some(t.text.as_str()),
                _ => None,
            })
            .collect();
        assert!(
            final_text.contains("All done"),
            "final answer text mismatch: {final_text:?}"
        );

        // ── 6. Loop emitted tool-execution lifecycle events ─────────────────
        let collected = events.lock().unwrap_or_else(|e| e.into_inner());
        assert!(
            collected.iter().any(|e| matches!(
                e,
                AgentEvent::ToolExecutionEnd { tool_name, is_error, .. }
                    if tool_name == "echo" && !*is_error
            )),
            "must emit a successful ToolExecutionEnd for echo"
        );

        // ── 7. Transcript persists: every message round-trips through serde,
        //        the exact contract the JSONL session store depends on ─────────
        for message in &result {
            let value = serde_json::to_value(message).expect("message must serialize");
            let restored = crate::types::agent_message_from_value(value)
                .expect("persisted message must deserialize by role");
            assert_eq!(
                std::mem::discriminant(message),
                std::mem::discriminant(&restored),
                "round-trip must preserve message role"
            );
        }

        reg.unregister();
    }

    /// Sanity floor: a single-turn loop with no tool call still streams text and
    /// terminates with a final answer.
    #[tokio::test]
    async fn e2e_single_turn_text_only() {
        let reg = Arc::new(register_faux_provider(
            RegisterFauxProviderOptions::default(),
        ));
        reg.set_responses(vec![
            faux_assistant_message(
                "Hello, I am a deterministic test agent.",
                FauxAssistantMessageOptions::default(),
            )
            .into(),
        ]);

        let context = AgentContext {
            system_prompt: "test".to_string(),
            messages: Vec::new(),
            tools: Vec::new(),
        };
        let events = Arc::new(Mutex::new(Vec::<AgentEvent>::new()));
        let reg_stream = Arc::clone(&reg);
        let stream_fn: crate::types::StreamFn = Arc::new(move |model, ctx, options| {
            let reg = Arc::clone(&reg_stream);
            Box::pin(async move { Ok(reg.stream_simple(model, ctx, options)) })
                as Pin<
                    Box<
                        dyn std::future::Future<
                                Output = Result<
                                    hamr_ai::utils::event_stream::AssistantMessageEventStream,
                                    StreamError,
                                >,
                            > + Send,
                    >,
                >
        });

        let result = run_agent_loop(
            vec![user_msg("hi")],
            context,
            minimal_config(reg.get_model()),
            collector_callback(events.clone()),
            None,
            None,
            stream_fn,
        )
        .await
        .expect("single-turn loop should complete");

        assert!(matches!(result.last(), Some(AgentMessage::Assistant(_))));
        let final_assistant = match result.last() {
            Some(AgentMessage::Assistant(a)) => a,
            _ => unreachable!(),
        };
        assert_eq!(final_assistant.stop_reason, StopReason::Stop);
        reg.unregister();
    }
}

/// Continue an agent loop from the current context without adding a new message.
/// Used for retries — context already has the user message or tool results.
///
/// Mirrors the TS `runAgentLoopContinue`.
pub async fn run_agent_loop_continue(
    context: AgentContext,
    config: AgentLoopConfig,
    on_event: EventCallback,
    lifecycle_signal: Option<tokio::sync::watch::Receiver<bool>>,
    tool_signal: Option<tokio::sync::watch::Receiver<bool>>,
    stream_fn: crate::types::StreamFn,
) -> Result<Vec<AgentMessage>, String> {
    if context.messages.is_empty() {
        return Err("Cannot continue: no messages in context".to_string());
    }

    // TS check: cannot continue from an assistant message
    if matches!(context.messages.last(), Some(AgentMessage::Assistant(_))) {
        return Err("Cannot continue from message role: assistant".to_string());
    }

    let mut new_messages: Vec<AgentMessage> = Vec::new();
    let model = config.model.clone();
    let reasoning = config.reasoning;
    let tool_execution = config.tool_execution;
    let config = Arc::new(config);

    // 1. Emit AgentStart
    on_event(AgentEvent::AgentStart).await;

    // 2. Emit TurnStart (first turn for continue)
    on_event(AgentEvent::TurnStart).await;

    // 3. Enter main loop
    run_loop(
        context,
        &mut new_messages,
        config,
        model,
        reasoning,
        tool_execution,
        lifecycle_signal,
        tool_signal,
        on_event,
        stream_fn,
        true, // first_turn = true
    )
    .await?;

    Ok(new_messages)
}
