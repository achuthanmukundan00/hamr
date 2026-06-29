//! Tool wrappers for extension-registered tools.
//!
//! Port of `packages/coding-agent/src/core/extensions/wrapper.ts`.
//!
//! These wrappers adapt tool execution so extension tools receive the runner context.
//! Tool call and tool result interception is handled by the agent session.

use std::sync::Arc;

use hamr_harness::types::AgentTool;

use super::runner::ExtensionRunner;
use super::types::{RegisteredTool, ToolDefinition};

/// Wrap a RegisteredTool into an AgentTool.
///
/// Uses the runner's `create_context()` for consistent context across tools
/// and event handlers.
///
/// Mirrors TS `wrapRegisteredTool()`.
pub fn wrap_registered_tool(
    registered_tool: &RegisteredTool,
    runner: &ExtensionRunner,
) -> AgentTool {
    wrap_tool_definition_with_context(&registered_tool.definition, runner)
}

/// Wrap multiple RegisteredTools into AgentTools.
///
/// Mirrors TS `wrapRegisteredTools()`.
pub fn wrap_registered_tools(
    registered_tools: &[RegisteredTool],
    runner: &ExtensionRunner,
) -> Vec<AgentTool> {
    registered_tools
        .iter()
        .map(|t| wrap_registered_tool(t, runner))
        .collect()
}

/// Wrap a ToolDefinition into an AgentTool with runner context injection.
fn wrap_tool_definition_with_context(
    definition: &ToolDefinition,
    runner: &ExtensionRunner,
) -> AgentTool {
    let execute = definition.execute.clone();
    let ctx_factory = {
        let runner_ref = runner.create_context();
        move || runner_ref.clone()
    };

    AgentTool {
        label: definition.label.clone(),
        name: definition.name.clone(),
        description: definition.description.clone(),
        parameters: definition.parameters.clone(),
        prepare_arguments: definition.prepare_arguments.clone(),
        execution_mode: definition.execution_mode,
        execute: Arc::new(move |tool_call_id, params, signal, on_update| {
            let execute = execute.clone();
            let ctx = ctx_factory();
            Box::pin(async move { execute(tool_call_id, params, signal, on_update, ctx).await })
        }),
    }
}
