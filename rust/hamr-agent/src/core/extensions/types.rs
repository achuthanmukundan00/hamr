//! Extension system types.
//!
//! Port of `packages/coding-agent/src/core/extensions/types.ts`.
//!
//! Defines the extension system contract: event types, registration APIs,
//! context types, tool definitions, and all event result types.

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use crate::core::source_info::SourceInfo;
use crate::core::system_prompt::BuildSystemPromptOptions;

// ---------------------------------------------------------------------------
// Extension Factory
// ---------------------------------------------------------------------------

/// Extension factory function type. Supports both sync and async initialization.
/// Mirrors TS `ExtensionFactory = (pi: ExtensionAPI) => void | Promise<void>`.
pub type ExtensionFactory =
    Arc<dyn Fn(Arc<dyn ExtensionAPI>) -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + Sync>;

// ---------------------------------------------------------------------------
// Extension API trait — passed to extension factory functions
// ---------------------------------------------------------------------------

/// Extension API trait — the primary interface passed to extension factories.
/// Mirrors TS `ExtensionAPI`.
pub trait ExtensionAPI: Send + Sync {
    fn on(&self, event: &str, handler: ExtensionHandlerFn);
    fn register_tool(&self, tool: ToolDefinition);
    fn register_command(&self, name: &str, command: RegisteredCommand);
    fn register_shortcut(&self, shortcut: String, options: ExtensionShortcut);
    fn register_flag(&self, name: &str, options: ExtensionFlag);
    fn register_message_renderer(&self, custom_type: &str, renderer: MessageRendererFn);
    fn register_role_message_renderer(&self, role: &str, renderer: RoleMessageRendererFn);
    fn get_flag(&self, name: &str) -> Option<serde_json::Value>;
    fn send_message(&self, message: serde_json::Value, options: Option<SendMessageOptions>);
    fn send_user_message(&self, content: SendUserContent, options: Option<SendUserOptions>);
    fn append_entry(&self, custom_type: &str, data: Option<serde_json::Value>);
    fn set_session_name(&self, name: &str);
    fn get_session_name(&self) -> Option<String>;
    fn set_label(&self, entry_id: &str, label: Option<&str>);
    fn get_active_tools(&self) -> Vec<String>;
    fn set_active_tools(&self, tool_names: &[String]);
    fn set_model(&self, model: serde_json::Value) -> Pin<Box<dyn Future<Output = bool> + Send>>;
    fn get_thinking_level(&self) -> String;
    fn set_thinking_level(&self, level: &str);
    fn register_provider(&self, name: &str, config: serde_json::Value);
    fn unregister_provider(&self, name: &str);
    fn events(&self) -> Arc<dyn crate::core::event_bus::EventBus>;
}

// ---------------------------------------------------------------------------
// Event Handler
// ---------------------------------------------------------------------------

/// Generic async handler function for extension events.
/// Receives event JSON and context, returns optional result JSON.
pub type ExtensionHandlerFn = Arc<
    dyn Fn(
            serde_json::Value,
            Arc<dyn ExtensionContext>,
        ) -> Pin<Box<dyn Future<Output = Option<serde_json::Value>> + Send>>
        + Send
        + Sync,
>;

// ---------------------------------------------------------------------------
// Renderers
// ---------------------------------------------------------------------------

/// Message renderer function.
pub type MessageRendererFn = Arc<
    dyn Fn(serde_json::Value, serde_json::Value, serde_json::Value) -> Option<serde_json::Value>
        + Send
        + Sync,
>;

/// Role message renderer function.
pub type RoleMessageRendererFn =
    Arc<dyn Fn(serde_json::Value, serde_json::Value) -> Option<serde_json::Value> + Send + Sync>;

// ---------------------------------------------------------------------------
// Extension Mode
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExtensionMode {
    Tui,
    Rpc,
    Json,
    Print,
}

impl ExtensionMode {
    pub fn from_str(s: &str) -> Self {
        match s {
            "tui" => Self::Tui,
            "rpc" => Self::Rpc,
            "json" => Self::Json,
            _ => Self::Print,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Tui => "tui",
            Self::Rpc => "rpc",
            Self::Json => "json",
            Self::Print => "print",
        }
    }
}

// ---------------------------------------------------------------------------
// Context Usage
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct ContextUsage {
    pub tokens: Option<u64>,
    pub context_window: u64,
    pub percent: Option<f64>,
}

// ---------------------------------------------------------------------------
// Compact Options
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default)]
pub struct CompactOptions {
    pub custom_instructions: Option<String>,
}

// ---------------------------------------------------------------------------
// Extension Context
// ---------------------------------------------------------------------------

/// Context passed to extension event handlers.
pub trait ExtensionContext: Send + Sync {
    fn ui(&self) -> Arc<dyn ExtensionUIContext>;
    fn mode(&self) -> ExtensionMode;
    fn has_ui(&self) -> bool;
    fn cwd(&self) -> String;
    fn model(&self) -> Option<serde_json::Value>;
    fn is_idle(&self) -> bool;
    fn is_project_trusted(&self) -> bool;
    fn abort(&self);
    fn has_pending_messages(&self) -> bool;
    fn shutdown(&self);
    fn get_context_usage(&self) -> Option<ContextUsage>;
    fn compact(&self, options: Option<CompactOptions>);
    fn get_system_prompt(&self) -> String;
}

// ---------------------------------------------------------------------------
// Extension Command Context
// ---------------------------------------------------------------------------

/// Extended context for command handlers.
pub trait ExtensionCommandContext: ExtensionContext {
    fn get_system_prompt_options(&self) -> BuildSystemPromptOptions;
    fn wait_for_idle(&self) -> Pin<Box<dyn Future<Output = ()> + Send>>;
    fn new_session(
        &self,
        options: Option<NewSessionOptions>,
    ) -> Pin<Box<dyn Future<Output = NewSessionResult> + Send>>;
    fn fork(
        &self,
        entry_id: String,
        options: Option<ForkOptions>,
    ) -> Pin<Box<dyn Future<Output = NewSessionResult> + Send>>;
    fn navigate_tree(
        &self,
        target_id: String,
        options: Option<NavigateTreeOptions>,
    ) -> Pin<Box<dyn Future<Output = NewSessionResult> + Send>>;
    fn switch_session(
        &self,
        session_path: String,
        options: Option<SwitchSessionOptions>,
    ) -> Pin<Box<dyn Future<Output = NewSessionResult> + Send>>;
    fn reload(&self) -> Pin<Box<dyn Future<Output = ()> + Send>>;
}

// ---------------------------------------------------------------------------
// Session / Command context options
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default)]
pub struct NewSessionOptions {
    pub parent_session: Option<String>,
}

#[derive(Debug, Clone)]
pub struct NewSessionResult {
    pub cancelled: bool,
}

#[derive(Debug, Clone)]
pub struct ForkOptions {
    pub position: Option<String>,
}

#[derive(Debug, Clone)]
pub struct NavigateTreeOptions {
    pub summarize: Option<bool>,
    pub custom_instructions: Option<String>,
    pub replace_instructions: Option<bool>,
    pub label: Option<String>,
}

#[derive(Clone)]
pub struct SwitchSessionOptions {
    pub with_session: Option<
        Arc<
            dyn Fn(Arc<dyn ExtensionCommandContext>) -> Pin<Box<dyn Future<Output = ()> + Send>>
                + Send
                + Sync,
        >,
    >,
}

// ---------------------------------------------------------------------------
// Send message options / content
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct SendMessageOptions {
    pub trigger_turn: Option<bool>,
    pub deliver_as: Option<String>,
}

#[derive(Debug, Clone)]
pub enum SendUserContent {
    Text(String),
    Blocks(Vec<serde_json::Value>),
}

#[derive(Debug, Clone)]
pub struct SendUserOptions {
    pub deliver_as: Option<String>,
}

// ---------------------------------------------------------------------------
// Extension UI Context
// ---------------------------------------------------------------------------

/// UI context for extensions to use interactive UI primitives.
pub trait ExtensionUIContext: Send + Sync {
    fn select(
        &self,
        title: &str,
        options: &[String],
    ) -> Pin<Box<dyn Future<Output = Option<String>> + Send>>;
    fn confirm(&self, title: &str, message: &str) -> Pin<Box<dyn Future<Output = bool> + Send>>;
    fn input(
        &self,
        title: &str,
        placeholder: Option<&str>,
    ) -> Pin<Box<dyn Future<Output = Option<String>> + Send>>;
    fn notify(&self, message: &str, notification_type: Option<&str>);
    fn set_status(&self, key: &str, text: Option<&str>);
    fn set_working_message(&self, message: Option<&str>);
    fn set_working_visible(&self, visible: bool);
    fn set_title(&self, title: &str);
}

// ---------------------------------------------------------------------------
// Tool Definition
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct ToolDefinition {
    pub name: String,
    pub label: String,
    pub description: String,
    pub prompt_snippet: Option<String>,
    pub prompt_guidelines: Option<Vec<String>>,
    pub parameters: serde_json::Value,
    pub render_shell: Option<String>,
    pub prepare_arguments:
        Option<Arc<dyn Fn(serde_json::Value) -> serde_json::Value + Send + Sync>>,
    pub execution_mode: Option<hamr_harness::types::ToolExecutionMode>,
    pub execute: Arc<
        dyn Fn(
                String,
                serde_json::Value,
                Option<tokio::sync::watch::Receiver<bool>>,
                Option<hamr_harness::types::AgentToolUpdateCallback>,
                Arc<dyn ExtensionContext>,
            )
                -> Pin<Box<dyn Future<Output = hamr_harness::types::AgentToolResult> + Send>>
            + Send
            + Sync,
    >,
}

// ---------------------------------------------------------------------------
// Registered Tool & Registered Command
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct RegisteredTool {
    pub definition: ToolDefinition,
    pub source_info: SourceInfo,
}

#[derive(Clone)]
pub struct RegisteredCommand {
    pub name: String,
    pub source_info: SourceInfo,
    pub description: Option<String>,
    pub handler: Arc<
        dyn Fn(String, Arc<dyn ExtensionCommandContext>) -> Pin<Box<dyn Future<Output = ()> + Send>>
            + Send
            + Sync,
    >,
}

#[derive(Clone)]
pub struct ResolvedCommand {
    pub name: String,
    pub invocation_name: String,
    pub source_info: SourceInfo,
    pub description: Option<String>,
    pub handler: Arc<
        dyn Fn(String, Arc<dyn ExtensionCommandContext>) -> Pin<Box<dyn Future<Output = ()> + Send>>
            + Send
            + Sync,
    >,
}

// ---------------------------------------------------------------------------
// Extension Flag
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct ExtensionFlag {
    pub name: String,
    pub description: Option<String>,
    pub flag_type: FlagType,
    pub default: Option<serde_json::Value>,
    pub extension_path: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlagType {
    Boolean,
    String,
}

// ---------------------------------------------------------------------------
// Extension Shortcut
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct ExtensionShortcut {
    pub shortcut: String,
    pub description: Option<String>,
    pub handler: Arc<
        dyn Fn(Arc<dyn ExtensionContext>) -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + Sync,
    >,
    pub extension_path: String,
}

// ---------------------------------------------------------------------------
// Tool Info
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct ToolInfo {
    pub name: String,
    pub description: String,
    pub source_info: SourceInfo,
}

// ---------------------------------------------------------------------------
// Extension — a loaded extension with all registered items
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct Extension {
    pub path: String,
    pub resolved_path: String,
    pub source_info: SourceInfo,
    pub handlers: HashMap<String, Vec<ExtensionHandlerFn>>,
    pub tools: HashMap<String, RegisteredTool>,
    pub message_renderers: HashMap<String, MessageRendererFn>,
    pub role_message_renderers: HashMap<String, RoleMessageRendererFn>,
    pub commands: HashMap<String, RegisteredCommand>,
    pub flags: HashMap<String, ExtensionFlag>,
    pub shortcuts: HashMap<String, ExtensionShortcut>,
}

impl Extension {
    pub fn new(path: String, resolved_path: String, source_info: SourceInfo) -> Self {
        Self {
            path,
            resolved_path,
            source_info,
            handlers: HashMap::new(),
            tools: HashMap::new(),
            message_renderers: HashMap::new(),
            role_message_renderers: HashMap::new(),
            commands: HashMap::new(),
            flags: HashMap::new(),
            shortcuts: HashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// LoadExtensionsResult
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct LoadExtensionsResult {
    pub extensions: Vec<Extension>,
    pub errors: Vec<ExtensionLoadError>,
}

#[derive(Debug, Clone)]
pub struct ExtensionLoadError {
    pub path: String,
    pub error: String,
}

// ---------------------------------------------------------------------------
// Extension Runtime
// ---------------------------------------------------------------------------

/// Shared runtime state shared between all extensions.
#[derive(Clone)]
pub struct ExtensionRuntime {
    pub flag_values: HashMap<String, serde_json::Value>,
    pub pending_provider_registrations: Vec<PendingProviderRegistration>,
    pub state: Arc<std::sync::Mutex<RuntimeState>>,
}

#[derive(Debug, Clone)]
pub struct RuntimeState {
    pub stale_message: Option<String>,
}

impl ExtensionRuntime {
    pub fn new() -> Self {
        Self {
            flag_values: HashMap::new(),
            pending_provider_registrations: Vec::new(),
            state: Arc::new(std::sync::Mutex::new(RuntimeState {
                stale_message: None,
            })),
        }
    }

    pub fn assert_active(&self) -> Result<(), String> {
        let state = self.state.lock().unwrap();
        if let Some(ref msg) = state.stale_message {
            return Err(msg.clone());
        }
        Ok(())
    }

    pub fn invalidate(&self, message: Option<&str>) {
        let mut state = self.state.lock().unwrap();
        if state.stale_message.is_none() {
            state.stale_message = Some(
                message
                    .unwrap_or(
                        "This extension ctx is stale after session replacement or reload. \
                         Do not use a captured pi or command ctx after ctx.newSession(), \
                         ctx.fork(), ctx.switchSession(), or ctx.reload().",
                    )
                    .to_string(),
            );
        }
    }
}

impl Default for ExtensionRuntime {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct PendingProviderRegistration {
    pub name: String,
    pub config: serde_json::Value,
    pub extension_path: String,
}

// ---------------------------------------------------------------------------
// Extension Actions — provided by the mode, copied into runtime
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct ExtensionActions {
    pub send_message:
        Option<Arc<dyn Fn(serde_json::Value, Option<SendMessageOptions>) + Send + Sync>>,
    pub send_user_message:
        Option<Arc<dyn Fn(SendUserContent, Option<SendUserOptions>) + Send + Sync>>,
    pub append_entry: Option<Arc<dyn Fn(String, Option<serde_json::Value>) + Send + Sync>>,
    pub set_session_name: Option<Arc<dyn Fn(String) + Send + Sync>>,
    pub get_session_name: Option<Arc<dyn Fn() -> Option<String> + Send + Sync>>,
    pub set_label: Option<Arc<dyn Fn(String, Option<String>) + Send + Sync>>,
    pub get_active_tools: Option<Arc<dyn Fn() -> Vec<String> + Send + Sync>>,
    pub get_all_tools: Option<Arc<dyn Fn() -> Vec<ToolInfo> + Send + Sync>>,
    pub set_active_tools: Option<Arc<dyn Fn(Vec<String>) + Send + Sync>>,
    pub refresh_tools: Option<Arc<dyn Fn() + Send + Sync>>,
    pub get_commands: Option<Arc<dyn Fn() -> Vec<serde_json::Value> + Send + Sync>>,
    pub set_model: Option<
        Arc<dyn Fn(serde_json::Value) -> Pin<Box<dyn Future<Output = bool> + Send>> + Send + Sync>,
    >,
    pub get_thinking_level: Option<Arc<dyn Fn() -> String + Send + Sync>>,
    pub set_thinking_level: Option<Arc<dyn Fn(String) + Send + Sync>>,
}

impl Default for ExtensionActions {
    fn default() -> Self {
        Self {
            send_message: None,
            send_user_message: None,
            append_entry: None,
            set_session_name: None,
            get_session_name: None,
            set_label: None,
            get_active_tools: None,
            get_all_tools: None,
            set_active_tools: None,
            refresh_tools: None,
            get_commands: None,
            set_model: None,
            get_thinking_level: None,
            set_thinking_level: None,
        }
    }
}

// ---------------------------------------------------------------------------
// Extension Context Actions
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct ExtensionContextActions {
    pub get_model: Arc<dyn Fn() -> Option<serde_json::Value> + Send + Sync>,
    pub is_idle: Arc<dyn Fn() -> bool + Send + Sync>,
    pub is_project_trusted: Arc<dyn Fn() -> bool + Send + Sync>,
    pub abort: Arc<dyn Fn() + Send + Sync>,
    pub has_pending_messages: Arc<dyn Fn() -> bool + Send + Sync>,
    pub shutdown: Arc<dyn Fn() + Send + Sync>,
    pub get_context_usage: Arc<dyn Fn() -> Option<ContextUsage> + Send + Sync>,
    pub compact: Arc<dyn Fn(Option<CompactOptions>) + Send + Sync>,
    pub get_system_prompt: Arc<dyn Fn() -> String + Send + Sync>,
    pub get_system_prompt_options: Option<Arc<dyn Fn() -> BuildSystemPromptOptions + Send + Sync>>,
}

// ---------------------------------------------------------------------------
// Extension Command Context Actions
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct ExtensionCommandContextActions {
    pub wait_for_idle: Arc<dyn Fn() -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + Sync>,
    pub new_session: Arc<
        dyn Fn(Option<NewSessionOptions>) -> Pin<Box<dyn Future<Output = NewSessionResult> + Send>>
            + Send
            + Sync,
    >,
    pub fork: Arc<
        dyn Fn(
                String,
                Option<ForkOptions>,
            ) -> Pin<Box<dyn Future<Output = NewSessionResult> + Send>>
            + Send
            + Sync,
    >,
    pub navigate_tree: Arc<
        dyn Fn(
                String,
                Option<NavigateTreeOptions>,
            ) -> Pin<Box<dyn Future<Output = NewSessionResult> + Send>>
            + Send
            + Sync,
    >,
    pub switch_session: Arc<
        dyn Fn(
                String,
                Option<SwitchSessionOptions>,
            ) -> Pin<Box<dyn Future<Output = NewSessionResult> + Send>>
            + Send
            + Sync,
    >,
    pub reload: Arc<dyn Fn() -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + Sync>,
}

// ---------------------------------------------------------------------------
// Extension Error
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct ExtensionError {
    pub extension_path: String,
    pub event: String,
    pub error: String,
    pub stack: Option<String>,
}

// ---------------------------------------------------------------------------
// Event Result Types
// ---------------------------------------------------------------------------

/// Result from a tool_call event handler.
#[derive(Debug, Clone)]
pub struct ToolCallEventResult {
    pub block: Option<bool>,
    pub reason: Option<String>,
}

/// Result from a tool_result event handler.
#[derive(Debug, Clone)]
pub struct ToolResultEventResult {
    pub content: Option<Vec<hamr_ai::types::MessageContent>>,
    pub details: Option<serde_json::Value>,
    pub is_error: Option<bool>,
}

/// Combined result from all before_agent_start handlers.
#[derive(Debug, Clone, Default)]
pub struct BeforeAgentStartCombinedResult {
    pub messages: Option<Vec<serde_json::Value>>,
    pub system_prompt: Option<String>,
}

/// Result from session_before_* handlers (cancellable).
#[derive(Debug, Clone, Default)]
pub struct SessionBeforeResult {
    pub cancel: Option<bool>,
}

/// Result from user_bash handler.
#[derive(Debug, Clone)]
pub struct UserBashEventResult {
    pub operations: Option<serde_json::Value>,
    pub result: Option<serde_json::Value>,
}

/// Result from input event handler.
#[derive(Debug, Clone)]
pub struct InputEventResult {
    pub action: InputAction,
    pub text: Option<String>,
    pub images: Option<Vec<serde_json::Value>>,
}

/// Action for input events.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputAction {
    Continue,
    Transform,
    Handled,
}

// ---------------------------------------------------------------------------
// Discovered resources
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct DiscoveredResourcePath {
    pub path: String,
    pub extension_path: String,
}

/// Result from emit_resources_discover.
#[derive(Debug, Clone, Default)]
pub struct ResourcesDiscoveredPaths {
    pub skill_paths: Vec<DiscoveredResourcePath>,
    pub prompt_paths: Vec<DiscoveredResourcePath>,
    pub theme_paths: Vec<DiscoveredResourcePath>,
}

/// Result from emitting a project_trust event.
#[derive(Debug, Clone)]
pub struct ProjectTrustEmitResult {
    pub result: Option<serde_json::Value>,
    pub errors: Vec<ExtensionError>,
}

// ---------------------------------------------------------------------------
// Helper: create extension runtime
// ---------------------------------------------------------------------------

pub fn create_extension_runtime() -> ExtensionRuntime {
    ExtensionRuntime::new()
}
