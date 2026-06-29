//! Port of `packages/coding-agent/src/core/tools/tool-definition-wrapper.ts`.
//!
//! Wraps [`ToolDefinition`] (from extensions/types.rs) into [`AgentTool`]
//! for the core runtime, bridging the `ExtensionContext` that extension tools
//! require by providing it at call time.

use std::sync::Arc;

use crate::core::extensions::runner::NoOpUIContext;
use crate::core::extensions::types::{
    CompactOptions, ExtensionContext, ExtensionMode, ExtensionUIContext, ToolDefinition,
};
use hamr_harness::types::AgentTool;

/// Wrap a [`ToolDefinition`] into an [`AgentTool`] for the core runtime.
///
/// Mirrors TS `wrapToolDefinition()`.
///
/// The returned `AgentTool::execute` provides a no-op extension context since
/// extension tool execution requires context but the core runtime does not
/// supply one. Callers should use [`wrap_registered_tool`] instead when an
/// actual `ExtensionContext` is available.
pub fn wrap_tool_definition(definition: ToolDefinition) -> AgentTool {
    wrap_tool_definition_with_context(definition, create_stub_context())
}

/// Wrap a tool definition using the live extension context from its runner.
pub fn wrap_tool_definition_with_context(
    definition: ToolDefinition,
    context: Arc<dyn ExtensionContext>,
) -> AgentTool {
    let execute_with_context = definition.execute.clone();
    AgentTool {
        name: definition.name,
        label: definition.label,
        description: definition.description,
        parameters: definition.parameters,
        prepare_arguments: definition.prepare_arguments.clone(),
        execution_mode: definition.execution_mode,
        execute: Arc::new(
            move |id: String,
                  params: serde_json::Value,
                  signal: Option<tokio::sync::watch::Receiver<bool>>,
                  on_update: Option<hamr_harness::types::AgentToolUpdateCallback>| {
                let exec = execute_with_context.clone();
                let context = context.clone();
                Box::pin(async move { exec(id, params, signal, on_update, context).await })
            },
        ),
    }
}

/// Wrap multiple [`ToolDefinition`]s into [`AgentTool`]s.
pub fn wrap_tool_definitions(definitions: Vec<ToolDefinition>) -> Vec<AgentTool> {
    definitions.into_iter().map(wrap_tool_definition).collect()
}

/// Synthesize a minimal [`ToolDefinition`] from an [`AgentTool`].
pub fn create_tool_definition_from_agent_tool(tool: &AgentTool) -> ToolDefinition {
    let tool_execute = tool.execute.clone();
    ToolDefinition {
        name: tool.name.clone(),
        label: tool.label.clone(),
        description: tool.description.clone(),
        prompt_snippet: None,
        prompt_guidelines: None,
        parameters: tool.parameters.clone(),
        render_shell: None,
        prepare_arguments: tool.prepare_arguments.clone(),
        execution_mode: tool.execution_mode,
        execute: Arc::new(
            move |id: String,
                  params: serde_json::Value,
                  signal: Option<tokio::sync::watch::Receiver<bool>>,
                  on_update: Option<hamr_harness::types::AgentToolUpdateCallback>,
                  _ctx: Arc<dyn ExtensionContext>| {
                let exec = tool_execute.clone();
                Box::pin(async move { exec(id, params, signal, on_update).await })
            },
        ),
    }
}

// ---------------------------------------------------------------------------
// Stub ExtensionContext for the default wrapping path
// ---------------------------------------------------------------------------

fn create_stub_context() -> Arc<dyn ExtensionContext> {
    Arc::new(StubExtensionContext)
}

struct StubExtensionContext;

impl ExtensionContext for StubExtensionContext {
    fn ui(&self) -> Arc<dyn ExtensionUIContext> {
        Arc::new(NoOpUIContext)
    }
    fn mode(&self) -> ExtensionMode {
        ExtensionMode::Print
    }
    fn has_ui(&self) -> bool {
        false
    }
    fn cwd(&self) -> String {
        std::env::current_dir()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default()
    }
    fn model(&self) -> Option<serde_json::Value> {
        None
    }
    fn is_idle(&self) -> bool {
        true
    }
    fn is_project_trusted(&self) -> bool {
        true
    }
    fn abort(&self) {}
    fn has_pending_messages(&self) -> bool {
        false
    }
    fn shutdown(&self) {}
    fn get_context_usage(&self) -> Option<crate::core::extensions::types::ContextUsage> {
        None
    }
    fn compact(&self, _options: Option<CompactOptions>) {}
    fn get_system_prompt(&self) -> String {
        String::new()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use hamr_ai::types::MessageContent;
    use hamr_harness::types::AgentToolResult;

    #[tokio::test]
    async fn test_wrap_tool_definition() {
        use std::sync::Arc;

        let def = ToolDefinition {
            name: "test_tool".to_string(),
            label: "Test Tool".to_string(),
            description: "A test tool".to_string(),
            prompt_snippet: None,
            prompt_guidelines: None,
            parameters: serde_json::json!({"type": "object"}),
            render_shell: None,
            prepare_arguments: None,
            execution_mode: None,
            execute: Arc::new(|_id, params, _signal, _on_update, _ctx| {
                Box::pin(async move {
                    AgentToolResult {
                        content: vec![MessageContent::Text(hamr_ai::types::TextContent {
                            text: format!("got: {}", params["key"].as_str().unwrap_or("none")),
                            text_signature: None,
                        })],
                        details: None,
                        is_error: false,
                        terminate: false,
                    }
                })
            }),
        };

        let agent_tool = wrap_tool_definition(def);
        assert_eq!(agent_tool.name, "test_tool");
        assert_eq!(agent_tool.label, "Test Tool");

        let result = (agent_tool.execute)(
            "id1".to_string(),
            serde_json::json!({"key": "val1"}),
            None,
            None,
        )
        .await;

        assert!(!result.is_error);
        assert_eq!(result.content.len(), 1);
    }

    #[test]
    fn test_create_tool_definition_from_agent_tool() {
        let agent_tool = AgentTool {
            name: "source_tool".to_string(),
            label: "Source".to_string(),
            description: "A source tool".to_string(),
            parameters: serde_json::json!({"type": "object"}),
            prepare_arguments: None,
            execution_mode: None,
            execute: Arc::new(|_id, _params, _signal, _on_update| {
                Box::pin(async move {
                    AgentToolResult {
                        content: vec![],
                        details: None,
                        is_error: false,
                        terminate: false,
                    }
                })
            }),
        };

        let def = create_tool_definition_from_agent_tool(&agent_tool);
        assert_eq!(def.name, "source_tool");
        assert_eq!(def.label, "Source");
        assert_eq!(def.description, "A source tool");
    }
}
