//! Port of `../../packages/ai/src/providers/anthropic.ts`.
//!
//! Anthropic Messages API provider backend.
//! Uses `reqwest` for HTTP + SSE streaming (Anthropic's SDK is not required).
//!
//! # What to Port
//!
//! 1. `streamAnthropic(model, context, options)` — the main streaming function
//! 2. Request body construction: system prompt, messages array, tools, thinking config
//! 3. SSE event parsing: `message_start`, `content_block_start/delta/stop`, `message_delta`, `message_stop`
//! 4. Prompt caching via `cache_control` markers on system prompt, last user message, last tool
//! 5. Extended thinking: `thinking: { type: "enabled", budget_tokens: N }` per thinking level
//! 6. `eager_input_streaming` for tool calls (beta header)
//! 7. Abort signal handling — cancel the HTTP request on abort
//! 8. Error handling: map HTTP errors, rate limits, overloaded to appropriate stop reasons
//!
//! # Anthropic-specific Types
//!
//! - `AnthropicOptions` — extends StreamOptions with Anthropic-specific fields
//! - `AnthropicThinkingDisplay` — how thinking content is displayed
//! - `AnthropicEffort` — effort level mapping
//!
//! # Key Behaviors
//!
//! - Session affinity: send `x-session-affinity` header from `options.sessionId`
//! - Cache control: annotate system prompt, last tool definition, last user/assistant text
//! - Tool streaming: send `fine-grained-tool-streaming-2025-05-14` beta header
//! - Temperature: some Claude models reject non-default temperature — check `supportsTemperature` compat flag
//! - Adaptive thinking: some models require `thinking.type: "adaptive"` instead of `"enabled"`
//!
//! # Dependencies
//!
//! - `reqwest` for HTTP + SSE
//! - `serde_json` for request/response serialization
//! - `tokio::sync::watch` for abort signal
//! - `super::super::utils::event_stream` for EventStream
//! - `super::super::types` for AssistantMessageEvent, etc.

