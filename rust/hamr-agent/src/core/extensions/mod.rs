//! Extensions infrastructure — traits, runner, loader, types, wrapper.
//!
//! Mirror of `packages/coding-agent/src/core/extensions/`.

pub mod api_impl;
pub mod loader;
pub mod runner;
pub mod types;
pub mod wrapper;

// Re-export key types and functions for convenience
pub use api_impl::ExtensionAPIImpl;
pub use loader::{discover_and_load_extensions, load_extension_from_factory, load_extensions};
pub use runner::{
    ExtensionErrorListener, ExtensionRunner, NoOpUIContext, RunnerCommandContext,
    RunnerExtensionContext, emit_project_trust_event, emit_session_shutdown_event,
};
pub use types::{
    BeforeAgentStartCombinedResult, CompactOptions, ContextUsage, DiscoveredResourcePath,
    Extension, ExtensionAPI, ExtensionActions, ExtensionCommandContext,
    ExtensionCommandContextActions, ExtensionContext, ExtensionContextActions, ExtensionError,
    ExtensionFactory, ExtensionFlag, ExtensionHandlerFn, ExtensionLoadError, ExtensionMode,
    ExtensionRuntime, ExtensionShortcut, ExtensionUIContext, FlagType, InputAction,
    InputEventResult, LoadExtensionsResult, MessageRendererFn, NavigateTreeOptions,
    NewSessionOptions, NewSessionResult, ProjectTrustEmitResult, RegisteredCommand, RegisteredTool,
    ResolvedCommand, ResourcesDiscoveredPaths, RoleMessageRendererFn, SendMessageOptions,
    SendUserContent, SendUserOptions, SessionBeforeResult, SwitchSessionOptions,
    ToolCallEventResult, ToolDefinition, ToolInfo, ToolResultEventResult, UserBashEventResult,
    create_extension_runtime,
};
pub use wrapper::{wrap_registered_tool, wrap_registered_tools};
