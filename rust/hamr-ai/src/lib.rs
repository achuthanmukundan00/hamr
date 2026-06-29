//! hamr-ai: Unified LLM API with provider abstraction, streaming, and type system.
//!
//! Architectural mirror of [`@hamr/ai`] (`packages/ai/`).
//!
//! # Crate structure
//!
//! | Module | TS source | Purpose |
//! |--------|-----------|---------|
//! | [`types`] | `types.ts` | Core types: Message, Model, Tool, Context, AssistantMessageEvent, etc. |
//! | [`models`] | `models.ts` + `models.generated.ts` | Model registry with provider detection and cost data |
//! | [`images`] | `images.ts` + `image-models.ts` | Image generation models and APIs |
//! | [`stream`] | `stream.ts` | Top-level `streamSimple()` and `completeSimple()` entry points |
//! | [`api_registry`] | `api-registry.ts` | API-to-provider dispatch registry |
//! | [`env_api_keys`] | `env-api-keys.ts` | Environment-variable API key resolution |
//! | [`oauth`] | `oauth.ts` | OAuth provider interfaces and login flows |
//! | [`session_resources`] | `session-resources.ts` | Session-scoped resource resolution |
//! | [`providers`] | `providers/` | Per-provider implementation modules |
//! | [`utils`] | `utils/` | Shared utilities (event streams, validation, diagnostics, etc.) |

// ---------------------------------------------------------------------------
// Re-export schema tools (TypeBox equivalent)
// ---------------------------------------------------------------------------
pub use schemars;

// ---------------------------------------------------------------------------
// Core types — everything depends on these
// ---------------------------------------------------------------------------
pub mod types;
pub use types::*;

// ---------------------------------------------------------------------------
// Public modules
// ---------------------------------------------------------------------------
pub mod api_registry;
pub mod env_api_keys;
pub mod images;
pub mod models;
pub mod models_generated;
pub mod oauth;
pub mod session_resources;
pub mod stream;

// ---------------------------------------------------------------------------
// Provider implementations
// ---------------------------------------------------------------------------
pub mod providers;

// ---------------------------------------------------------------------------
// Internal utilities — available to dependents
// ---------------------------------------------------------------------------
pub mod utils;

/// Initialize hamr-ai: register all built-in API providers.
///
/// Call this once at startup before using [`stream::stream`] or [`stream::complete`].
/// Mirrors the TS side-effect `import "./providers/register-builtins.ts"`.
pub fn init() {
    providers::register_builtins::register_built_in_api_providers();
}
