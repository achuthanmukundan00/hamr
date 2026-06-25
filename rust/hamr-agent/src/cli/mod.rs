//! CLI entrypoint — args parsing, config, startup UI, session picking.
//!
//! Mirror of `packages/coding-agent/src/cli/`.

pub mod args;
pub mod config_selector;
pub mod file_processor;
pub mod initial_message;
pub mod list_models;
pub mod project_trust;
pub mod session_picker;
pub mod startup_ui;
// Note: main.rs is the binary entrypoint, not part of the library module tree.
