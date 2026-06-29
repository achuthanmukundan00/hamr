//! Concrete ExtensionAPI implementation.
//!
//! Port of the `createExtensionAPI()` function in
//! `packages/coding-agent/src/core/extensions/loader.ts`.
//!
//! This bridges the extension factory API to the Extension struct:
//! - Registration methods (on, registerTool, registerCommand, etc.) write
//!   directly into the Extension's collections.
//! - Action methods (sendMessage, setModel, etc.) delegate to the shared
//!   ExtensionRuntime / ExtensionActions.
//! - All methods guard against stale runtimes via `runtime.assert_active()`.

use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};

use super::types::*;
use crate::core::event_bus::EventBus;

// ---------------------------------------------------------------------------
// Concrete ExtensionAPI
// ---------------------------------------------------------------------------

/// Concrete ExtensionAPI implementation that bridges factory closures to an
/// Extension struct and shared runtime state.
pub struct ExtensionAPIImpl {
    extension: Mutex<Extension>,
    runtime: Mutex<ExtensionRuntime>,
    cwd: String,
    event_bus: Arc<dyn EventBus>,
    actions: Mutex<ExtensionActions>,
}

impl ExtensionAPIImpl {
    /// Create a new ExtensionAPI implementation for the given extension.
    ///
    /// Mirrors the `createExtensionAPI()` function in loader.ts.
    pub fn new(
        extension: Extension,
        runtime: ExtensionRuntime,
        cwd: String,
        event_bus: Arc<dyn EventBus>,
    ) -> Self {
        Self {
            extension: Mutex::new(extension),
            runtime: Mutex::new(runtime),
            cwd,
            event_bus,
            actions: Mutex::new(ExtensionActions::default()),
        }
    }

    /// Update the actions after the runner binds core.
    pub fn set_actions(&self, actions: ExtensionActions) {
        *self.actions.lock().unwrap() = actions;
    }

    /// Consume the API and return the populated Extension.
    pub fn into_extension(self) -> Extension {
        self.extension.into_inner().unwrap()
    }

    /// Clone the extension without consuming the API.
    /// Used as a fallback when Arc::try_unwrap fails.
    pub fn clone_extension(&self) -> Extension {
        self.extension.lock().unwrap().clone()
    }

    /// Consume the API and return the ExtensionRuntime.
    pub fn into_runtime(self) -> ExtensionRuntime {
        self.runtime.into_inner().unwrap()
    }
}

impl ExtensionAPI for ExtensionAPIImpl {
    fn on(&self, event: &str, handler: ExtensionHandlerFn) {
        let runtime = self.runtime.lock().unwrap();
        runtime.assert_active().expect("Extension runtime is stale");
        let mut ext = self.extension.lock().unwrap();
        ext.handlers
            .entry(event.to_string())
            .or_default()
            .push(handler);
    }

    fn register_tool(&self, tool: ToolDefinition) {
        let runtime = self.runtime.lock().unwrap();
        runtime.assert_active().expect("Extension runtime is stale");
        // Clone source_info before the mutable borrow
        let source_info = {
            let ext = self.extension.lock().unwrap();
            ext.source_info.clone()
        };
        let mut ext = self.extension.lock().unwrap();
        ext.tools.insert(
            tool.name.clone(),
            RegisteredTool {
                definition: tool,
                source_info,
            },
        );
        // Notify runtime to refresh tools
        let actions = self.actions.lock().unwrap();
        if let Some(ref refresh) = actions.refresh_tools {
            refresh();
        }
    }

    fn register_command(&self, name: &str, command: RegisteredCommand) {
        let runtime = self.runtime.lock().unwrap();
        runtime.assert_active().expect("Extension runtime is stale");
        let mut ext = self.extension.lock().unwrap();
        ext.commands.insert(name.to_string(), command);
    }

    fn register_shortcut(&self, shortcut: String, options: ExtensionShortcut) {
        let runtime = self.runtime.lock().unwrap();
        runtime.assert_active().expect("Extension runtime is stale");
        let mut ext = self.extension.lock().unwrap();
        ext.shortcuts.insert(shortcut, options);
    }

    fn register_flag(&self, name: &str, options: ExtensionFlag) {
        let runtime = &mut *self.runtime.lock().unwrap();
        runtime.assert_active().expect("Extension runtime is stale");
        let mut ext = self.extension.lock().unwrap();
        ext.flags.insert(name.to_string(), options.clone());
        // Set default flag value if not already set
        if let Some(ref default) = options.default {
            runtime
                .flag_values
                .entry(name.to_string())
                .or_insert(default.clone());
        }
    }

    fn register_message_renderer(&self, custom_type: &str, renderer: MessageRendererFn) {
        let runtime = self.runtime.lock().unwrap();
        runtime.assert_active().expect("Extension runtime is stale");
        let mut ext = self.extension.lock().unwrap();
        ext.message_renderers
            .insert(custom_type.to_string(), renderer);
    }

    fn register_role_message_renderer(&self, role: &str, renderer: RoleMessageRendererFn) {
        let runtime = self.runtime.lock().unwrap();
        runtime.assert_active().expect("Extension runtime is stale");
        let mut ext = self.extension.lock().unwrap();
        ext.role_message_renderers
            .insert(role.to_string(), renderer);
    }

    fn get_flag(&self, name: &str) -> Option<serde_json::Value> {
        let runtime = self.runtime.lock().unwrap();
        let _ = runtime.assert_active().ok();
        let ext = self.extension.lock().unwrap();
        if ext.flags.contains_key(name) {
            runtime.flag_values.get(name).cloned()
        } else {
            None
        }
    }

    fn send_message(&self, message: serde_json::Value, options: Option<SendMessageOptions>) {
        let actions = self.actions.lock().unwrap();
        if let Some(ref send) = actions.send_message {
            send(message, options);
        }
    }

    fn send_user_message(&self, content: SendUserContent, options: Option<SendUserOptions>) {
        let actions = self.actions.lock().unwrap();
        if let Some(ref send) = actions.send_user_message {
            send(content, options);
        }
    }

    fn append_entry(&self, custom_type: &str, data: Option<serde_json::Value>) {
        let actions = self.actions.lock().unwrap();
        if let Some(ref append) = actions.append_entry {
            append(custom_type.to_string(), data);
        }
    }

    fn set_session_name(&self, name: &str) {
        let actions = self.actions.lock().unwrap();
        if let Some(ref set) = actions.set_session_name {
            set(name.to_string());
        }
    }

    fn get_session_name(&self) -> Option<String> {
        let actions = self.actions.lock().unwrap();
        actions.get_session_name.as_ref().and_then(|f| f())
    }

    fn set_label(&self, entry_id: &str, label: Option<&str>) {
        let actions = self.actions.lock().unwrap();
        if let Some(ref set) = actions.set_label {
            set(entry_id.to_string(), label.map(|s| s.to_string()));
        }
    }

    fn get_active_tools(&self) -> Vec<String> {
        let actions = self.actions.lock().unwrap();
        actions
            .get_active_tools
            .as_ref()
            .map(|f| f())
            .unwrap_or_default()
    }

    fn set_active_tools(&self, tool_names: &[String]) {
        let actions = self.actions.lock().unwrap();
        if let Some(ref set) = actions.set_active_tools {
            set(tool_names.to_vec());
        }
    }

    fn set_model(&self, model: serde_json::Value) -> Pin<Box<dyn Future<Output = bool> + Send>> {
        let actions = self.actions.lock().unwrap();
        if let Some(ref set) = actions.set_model {
            set(model)
        } else {
            Box::pin(std::future::ready(false))
        }
    }

    fn get_thinking_level(&self) -> String {
        let actions = self.actions.lock().unwrap();
        actions
            .get_thinking_level
            .as_ref()
            .map(|f| f())
            .unwrap_or_else(|| "none".to_string())
    }

    fn set_thinking_level(&self, level: &str) {
        let actions = self.actions.lock().unwrap();
        if let Some(ref set) = actions.set_thinking_level {
            set(level.to_string());
        }
    }

    fn register_provider(&self, name: &str, config: serde_json::Value) {
        let mut runtime = self.runtime.lock().unwrap();
        let extension_path = self.extension.lock().unwrap().path.clone();
        runtime
            .pending_provider_registrations
            .push(PendingProviderRegistration {
                name: name.to_string(),
                config,
                extension_path,
            });
    }

    fn unregister_provider(&self, name: &str) {
        let mut runtime = self.runtime.lock().unwrap();
        runtime
            .pending_provider_registrations
            .retain(|r| r.name != name);
    }

    fn events(&self) -> Arc<dyn EventBus> {
        self.event_bus.clone()
    }
}
