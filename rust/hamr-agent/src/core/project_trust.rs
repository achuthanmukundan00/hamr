//! Port of `packages/coding-agent/src/core/project_trust.ts`.
//!
//! Resolve whether a project directory should be trusted. Coordinates
//! between the trust store, extension event handlers, default settings,
//! and interactive user prompts.

use crate::core::trust_manager::{
    ProjectTrustOption, ProjectTrustOptionsInput, ProjectTrustStore, get_project_trust_options,
    has_trust_requiring_project_resources,
};
use std::future::Future;
use std::pin::Pin;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// Application run mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppMode {
    Interactive,
    Print,
    Json,
    Rpc,
}

/// Default project trust setting from settings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DefaultProjectTrust {
    /// Always trust when no explicit decision exists.
    Always,
    /// Never trust unless explicitly trusted.
    Never,
    /// Ask the user interactively.
    Ask,
}

/// Context required for extension project_trust event handling.
/// Stubbed — will be fleshed out when extensions are ported.
#[derive(Clone)]
pub struct ProjectTrustContext {
    /// Current working directory.
    pub cwd: String,
    /// Operation mode ("tui", "print", "json", "rpc").
    pub mode: String,
    /// Whether a UI is available for prompts.
    pub has_ui: bool,
    /// UI select callback (stub — not yet wired).
    pub ui_select: Option<
        std::sync::Arc<
            dyn Fn(String, Vec<String>) -> Pin<Box<dyn Future<Output = Option<String>> + Send>>
                + Send
                + Sync,
        >,
    >,
}

/// Result of loading extensions (stub).
pub struct LoadExtensionsResult {
    // Stub — will be populated when extensions module is ported
}

/// Result from a project_trust extension event.
pub struct ExtensionTrustResult {
    pub trusted: String, // "yes" or "no"
    pub remember: bool,
}

/// Error from an extension event.
pub struct ExtensionError {
    pub extension_path: String,
    pub error: String,
}

/// Result of emitting a project trust event.
pub struct EmitProjectTrustEventResult {
    pub result: Option<ExtensionTrustResult>,
    pub errors: Vec<ExtensionError>,
}

// ---------------------------------------------------------------------------
// Stub for emitProjectTrustEvent (will be replaced when extensions ported)
// ---------------------------------------------------------------------------

fn emit_project_trust_event(
    _extensions_result: &LoadExtensionsResult,
    _event: serde_json::Value,
    _ctx: &ProjectTrustContext,
) -> EmitProjectTrustEventResult {
    // Stub — returns no result and no errors
    EmitProjectTrustEventResult {
        result: None,
        errors: vec![],
    }
}

// ---------------------------------------------------------------------------
// Options
// ---------------------------------------------------------------------------

/// Options for `resolve_project_trusted`.
pub struct ResolveProjectTrustedOptions<'a> {
    pub cwd: String,
    pub trust_store: &'a ProjectTrustStore,
    pub trust_override: Option<bool>,
    pub default_project_trust: Option<DefaultProjectTrust>,
    pub extensions_result: Option<&'a LoadExtensionsResult>,
    pub project_trust_context: &'a ProjectTrustContext,
    pub on_extension_error: Option<Box<dyn Fn(String) + 'a>>,
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn format_project_trust_prompt(cwd: &str) -> String {
    format!(
        "Trust project folder?\n{}\n\nThis allows hamr to load .hamr settings and resources, install missing project packages, and execute project extensions.",
        cwd
    )
}

async fn select_project_trust_option(
    cwd: &str,
    ctx: &ProjectTrustContext,
) -> Option<ProjectTrustOption> {
    let options_input = ProjectTrustOptionsInput {
        include_session_only: true,
    };
    let options = get_project_trust_options(cwd, Some(&options_input));

    let labels: Vec<String> = options.iter().map(|o| o.label.clone()).collect();

    let selected = match &ctx.ui_select {
        Some(select_fn) => select_fn(format_project_trust_prompt(cwd), labels).await,
        None => None,
    };

    options.into_iter().find(|o| {
        if let Some(ref sel) = selected {
            o.label == *sel
        } else {
            false
        }
    })
}

fn save_project_trust_prompt_result(trust_store: &ProjectTrustStore, result: &ProjectTrustOption) {
    if !result.updates.is_empty() {
        trust_store.set_many(&result.updates);
    }
}

// ---------------------------------------------------------------------------
// Main resolution function
// ---------------------------------------------------------------------------

/// Resolve whether the project directory should be trusted.
///
/// Checks in order:
/// 1. trust_override (explicit CLI flag)
/// 2. Whether trust-requiring resources exist
/// 3. Extension project_trust event handlers
/// 4. Persistent trust store
/// 5. Default project trust setting
/// 6. Interactive user prompt (if UI available)
pub async fn resolve_project_trusted(options: ResolveProjectTrustedOptions<'_>) -> bool {
    // 1. Trust override
    if let Some(trusted) = options.trust_override {
        return trusted;
    }

    // 2. No trust-requiring resources → implicitly trusted
    if !has_trust_requiring_project_resources(&options.cwd) {
        return true;
    }

    // 3. Extension project_trust event
    if let Some(ext_result) = options.extensions_result {
        let event = serde_json::json!({
            "type": "project_trust",
            "cwd": options.cwd
        });
        let EmitProjectTrustEventResult { result, errors } =
            emit_project_trust_event(ext_result, event, options.project_trust_context);

        for error in &errors {
            if let Some(ref on_error) = options.on_extension_error {
                on_error(format!(
                    "Extension \"{}\" project_trust error: {}",
                    error.extension_path, error.error
                ));
            }
        }

        if let Some(result) = result {
            let trusted = result.trusted == "yes";
            if result.remember {
                options.trust_store.set(&options.cwd, Some(trusted));
            }
            return trusted;
        }
    }

    // 4. Persistent trust store
    let decision = options.trust_store.get(&options.cwd);
    if decision.is_some() {
        return decision.unwrap();
    }

    // 5. Default project trust setting
    match options
        .default_project_trust
        .unwrap_or(DefaultProjectTrust::Ask)
    {
        DefaultProjectTrust::Always => return true,
        DefaultProjectTrust::Never => return false,
        DefaultProjectTrust::Ask => {
            // Fall through to interactive prompt
        }
    }

    // 6. Interactive prompt
    if !options.project_trust_context.has_ui {
        return false;
    }

    let selected = select_project_trust_option(&options.cwd, options.project_trust_context).await;
    if let Some(ref result) = selected {
        save_project_trust_prompt_result(options.trust_store, result);
        return result.trusted;
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_project_trust_display() {
        assert_eq!(format!("{:?}", DefaultProjectTrust::Always), "Always");
        assert_eq!(format!("{:?}", DefaultProjectTrust::Never), "Never");
        assert_eq!(format!("{:?}", DefaultProjectTrust::Ask), "Ask");
    }

    #[test]
    fn test_format_project_trust_prompt_contains_cwd() {
        let prompt = format_project_trust_prompt("/my/project");
        assert!(prompt.contains("/my/project"));
        assert!(prompt.contains("Trust project folder"));
    }

    #[test]
    fn test_app_mode_properties() {
        assert_eq!(AppMode::Interactive as isize, 0);
        assert_eq!(AppMode::Print as isize, 1);
        assert_eq!(AppMode::Json as isize, 2);
        assert_eq!(AppMode::Rpc as isize, 3);
    }
}
