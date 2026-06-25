//! Port of `packages/coding-agent/src/core/extensions/loader.ts`.
//!
//! Discovers and loads extensions from filesystem paths and Rhai scripts.
//! Produces a `LoadExtensionsResult` containing parsed `Extension` trait objects,
//! loaded tools, registered commands, shortcuts, flags, and diagnostics.
//!
//! # Porting Instructions
//!
//! 1. Read `../../packages/coding-agent/src/core/extensions/loader.ts`.
//! 2. TS uses `jiti` (dynamic TypeScript import) → Rust uses:
//!    - **Compiled extensions**: loaded via `Box<dyn ExtensionFactory>` closures registered at startup.
//!    - **Rhai extensions**: loaded from `*.rhai` files in the extensions directory.
//! 3. The loader scans configured extension directories, finds `.rhai` files,
//!    compiles them with the Rhai engine, and wraps them as `Extension` trait objects.
//! 4. Built-in extensions (memory, subagents, etc.) are registered via `ExtensionFactory`
//!    closures that the CLI passes to the loader.
//! 5. Diagnostics: collect warnings for duplicate registrations, parse errors, etc.
//!
//! # Key Functions
//!
//! - `discover_and_load_extensions()` — main entry point
//! - Scan `~/.hamr/extensions/` and project-local `.hamr/extensions/` directories
//! - Load Rhai scripts, compile, register as Extension implementations
//! - Load compiled extensions from the provided factory list
//! - Return `LoadExtensionsResult` with extensions + diagnostics
//!
//! # Rhai Integration
//!
//! When `feature = "rhai-extensions"` is enabled:
//! ```rust
//! use rhai::{Engine, Scope};
//!
//! let engine = Engine::new();
//! // Register hamr API bindings
//! engine.register_fn("pi_on", |event_type: &str, handler: rhai::FnPtr| { ... });
//! engine.register_fn("pi_register_tool", |name: &str, config: rhai::Map| { ... });
//! // Compile and evaluate .rhai files
//! let ast = engine.compile_file("~/.hamr/extensions/my-ext.rhai")?;
//! engine.run_ast(&ast)?;
//! ```
//!
//! # Dependencies
//!
//! - `super::types` for Extension, ExtensionFactory, LoadExtensionsResult
//! - `glob` or `ignore` for filesystem scanning
//! - `rhai` (optional, feature-gated) for agent-authored extensions

