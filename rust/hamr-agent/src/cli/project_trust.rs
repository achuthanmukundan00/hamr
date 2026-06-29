//! Create ProjectTrustContext with UI callbacks for the CLI.
//!
//! Port of `packages/coding-agent/src/cli/project-trust.ts`.

use crate::core::project_trust::ProjectTrustContext;
use crate::core::settings_manager::SettingsManager;

/// Options for creating a ProjectTrustContext.
pub struct ProjectTrustContextOptions<'a> {
    pub cwd: String,
    /// "interactive", "print", "json", "rpc", or "tui"
    pub mode: &'a str,
    pub settings_manager: &'a SettingsManager,
    pub has_ui: bool,
}

/// Create a ProjectTrustContext.
///
/// The TS version wires up UI callbacks (select, confirm, input, notify)
/// using the TUI startup selectors. These callbacks are stubbed here until
/// the TUI layer is fully ported.
///
/// This mirrors `createProjectTrustContext` from the TS source.
pub fn create_project_trust_context(
    options: ProjectTrustContextOptions<'_>,
) -> ProjectTrustContext {
    let tui_mode = if options.mode == "interactive" {
        "tui"
    } else {
        options.mode
    };

    ProjectTrustContext {
        cwd: options.cwd,
        mode: tui_mode.to_string(),
        has_ui: options.has_ui,
        ui_select: None, // Stub — TUI callbacks not yet wired
    }
}
