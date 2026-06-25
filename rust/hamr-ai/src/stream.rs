//! Port of `../../packages/ai/src/stream.ts`.
//!
//! Top-level streaming entry points: `streamSimple()` and `completeSimple()`.
//!
//! # Architecture
//!
//! This is the **single entry point** that all agent code calls to make LLM requests.
//! It:
//! 1. Looks up the model in the API registry
//! 2. Resolves the API key from env vars or auth storage
//! 3. Dispatches to the correct provider backend (anthropic, openai, google, etc.)
//! 4. Returns an `EventStream<AssistantMessageEvent>` that the agent loop reads from
//!
//! # Key Functions to Port
//!
//! - `stream_simple(model, context, options?)` → `BoxStream<AssistantMessageEvent>`
//! - `complete_simple(model, context, options?)` → `AssistantMessage` (non-streaming)
//!
//! In Rust, prefer returning a `Pin<Box<dyn Stream<Item = Result<AssistantMessageEvent>>>>` 
//! or use a channel-based approach where the provider spawns a task and sends events.
//!
//! # Provider Dispatch Pattern
//!
//! ```rust
//! match model.api {
//!     Api::AnthropicMessages => anthropic::stream(model, context, options).await,
//!     Api::OpenAiCompletions => openai_completions::stream(model, context, options).await,
//!     Api::GoogleGenerativeAi => google::stream(model, context, options).await,
//!     // ... etc
//! }
//! ```
//!
//! # Dependencies
//!
//! - `super::api_registry` — maps (provider, api) pairs to backend implementations
//! - `super::env_api_keys` — resolves API keys from environment variables
//! - `super::providers::*` — individual provider backends
//! - `super::types` — Model, Context, StreamOptions, AssistantMessageEvent
//! - `super::utils::event_stream` — EventStream implementation
//! - `tokio_stream` for stream traits
//! - `reqwest` for HTTP

