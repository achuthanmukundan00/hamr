//! Hamr built-in extensions — implementations of the Extension trait.
//!
//! Mirror of `packages/coding-agent/src/hamr/extensions/`.
//!
//! Each extension is a separate module implementing the [`Extension`] trait.
//! They're composed into the default extension set at startup.
//!
//! See [`Extension`]: crate::core::extensions::types::Extension

pub mod cards;
pub mod context;
pub mod memory;
pub mod persistent_editor;
pub mod providers;
pub mod read_loop_guard;
pub mod subagents;

// The default extension set is assembled at runtime by the CLI.
// Mirror of `hamrDefaultExtensions` in the TS source.
