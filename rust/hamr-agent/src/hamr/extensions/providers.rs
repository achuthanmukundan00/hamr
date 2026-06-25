//! Port of `../../packages/coding-agent/src/hamr/extensions/providers.ts`.
//!
//! Implements tool-call parsing and repair for non-native function-calling models.
//! Registered as a built-in Extension behind `#[cfg(feature = "hamr-providers")]`.
//!
//! # Architecture
//!
//! Why this exists: not all LLMs support native function calling. This extension
//! intercepts `message_end` events and parses tool calls from the model's text output
//! using provider-specific parsers (DeepSeek, Hermes, Mistral, Qwen, etc.).
//!
//! The extension also handles:
//! - `tool_call` repair — fixing malformed JSON in tool arguments
//! - `before_provider_request` — injecting provider-specific headers/tweaks
//! - Registering a relay provider for models that need output parsing
//!
//! # Porting Order
//!
//! 1. Port `hamr/providers/parsers/*.rs` — each parser is a standalone module
//! 2. Port `hamr/providers/repair/*.rs` — JSON repair, XML repair, reasoning sanitizer
//! 3. Port `hamr/providers/relay_provider.rs` — wraps a real provider with parsing
//! 4. Port this file — composes them into an Extension impl

