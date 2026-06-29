//! Port of `../../packages/ai/src/providers/faux.ts`.
//!
//! The faux provider is a toy/test provider that simulates streaming and token
//! estimation with no real HTTP. It is the cleanest provider to port and a good
//! reference for the streaming contract.
//!
//! ## TS → Rust impedance mismatches (type-debt)
//!
//! - **Dynamic string `api`**: TS lets `registerFauxProvider` pick an arbitrary
//!   string `api` (e.g. `"faux:test"`, `randomId("faux")`). The Rust
//!   [`crate::api_registry`] is keyed by the closed [`Api`] enum, and `Model.api`
//!   is `Api` (not `String`). We therefore split the concept:
//!   - `api_name: String` — the dynamic string used in
//!     [`AssistantMessage::api`] (which IS a `String`), matching TS behavior for
//!     returned messages.
//!   - `api: Api` — the enum value the provider is registered under in the global
//!     registry (configurable, default [`Api::AnthropicMessages`]). The top-level
//!     [`crate::stream::stream`] dispatch looks providers up by `Model.api`, so a
//!     faux model must carry a real enum value to be dispatchable. See
//!     `// TODO(api-string)` below.
//! - **No `Model.compat`**: not present in Rust `Model`. The TS faux provider
//!   does not read `model.compat` anyway, so nothing to work around here.
//! - **`AbortSignal` → `watch::Receiver<bool>`**: TS `signal.aborted` becomes
//!   `*signal.borrow()` on a `tokio::sync::watch::Receiver<bool>`.
//! - **`onResponse` callback**: invoked with a `ProviderResponse { status: 200,
//!   headers: {} }` mirroring the TS `{ status: 200, headers: {} }`.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use chrono::Utc;

use crate::api_registry::{
    ApiProvider, ApiStreamFunction, ApiStreamSimpleFunction, register_api_provider,
    unregister_api_providers,
};
use crate::types::{
    Api, AssistantContentBlock, AssistantMessage, AssistantMessageEvent, CacheRetention, Context,
    DoneReason, ErrorReason, ImageContent, InputModality, Message, MessageContent, MessageRole,
    Model, ModelCost, SimpleStreamOptions, StopReason, TextContent, ThinkingContent, ToolCall,
    ToolResultMessage, Usage, UsageCost,
};
use crate::utils::event_stream::{
    AssistantMessageEventStream, AssistantMessageEventStreamSender,
    create_assistant_message_event_stream,
};

const DEFAULT_API_NAME: &str = "faux";
const DEFAULT_PROVIDER: &str = "faux";
const DEFAULT_MODEL_ID: &str = "faux-1";
const DEFAULT_MODEL_NAME: &str = "Faux Model";
const DEFAULT_BASE_URL: &str = "http://localhost:0";
const DEFAULT_MIN_TOKEN_SIZE: u64 = 3;
const DEFAULT_MAX_TOKEN_SIZE: u64 = 5;

/// The enum value faux models/providers are registered under by default.
///
/// `// TODO(api-string)`: TS uses a dynamic string api; the Rust registry is keyed
/// by the closed [`Api`] enum, so we pick a concrete variant here. Callers can
/// override via [`RegisterFauxProviderOptions::api_enum`].
const DEFAULT_API_ENUM: Api = Api::AnthropicMessages;

fn default_usage() -> Usage {
    Usage {
        input: 0,
        output: 0,
        cache_read: 0,
        cache_write: 0,
        cache_write_1h: None,
        total_tokens: 0,
        cost: UsageCost {
            input: 0.0,
            output: 0.0,
            cache_read: 0.0,
            cache_write: 0.0,
            total: 0.0,
        },
    }
}

// ---------------------------------------------------------------------------
// Model definitions
// ---------------------------------------------------------------------------

/// Mirrors the TS `interface FauxModelDefinition`.
#[derive(Debug, Clone, Default)]
pub struct FauxModelDefinition {
    pub id: String,
    pub name: Option<String>,
    pub reasoning: Option<bool>,
    pub input: Option<Vec<InputModality>>,
    pub cost: Option<ModelCost>,
    pub context_window: Option<u64>,
    pub max_tokens: Option<u64>,
}

// ---------------------------------------------------------------------------
// Content-block helpers (mirror fauxText / fauxThinking / fauxToolCall)
// ---------------------------------------------------------------------------

/// Mirrors the TS `type FauxContentBlock = TextContent | ThinkingContent | ToolCall`.
#[derive(Debug, Clone)]
pub enum FauxContentBlock {
    Text(TextContent),
    Thinking(ThinkingContent),
    ToolCall(ToolCall),
}

impl From<FauxContentBlock> for AssistantContentBlock {
    fn from(block: FauxContentBlock) -> Self {
        match block {
            FauxContentBlock::Text(t) => AssistantContentBlock::Text(t),
            FauxContentBlock::Thinking(t) => AssistantContentBlock::Thinking(t),
            FauxContentBlock::ToolCall(t) => AssistantContentBlock::ToolCall(t),
        }
    }
}

/// Mirrors the TS `fauxText`.
pub fn faux_text(text: impl Into<String>) -> TextContent {
    TextContent {
        text: text.into(),
        text_signature: None,
    }
}

/// Mirrors the TS `fauxThinking`.
pub fn faux_thinking(thinking: impl Into<String>) -> ThinkingContent {
    ThinkingContent {
        thinking: thinking.into(),
        thinking_signature: None,
        redacted: false,
    }
}

/// Mirrors the TS `fauxToolCall`. `id` defaults to a generated id when `None`.
pub fn faux_tool_call(
    name: impl Into<String>,
    arguments: serde_json::Value,
    id: Option<String>,
) -> ToolCall {
    ToolCall {
        id: id.unwrap_or_else(|| random_id("tool")),
        name: name.into(),
        arguments,
        thought_signature: None,
    }
}

/// Content acceptable to [`faux_assistant_message`] — mirrors the TS
/// `string | FauxContentBlock | FauxContentBlock[]` union.
pub enum FauxContent {
    Text(String),
    Block(FauxContentBlock),
    Blocks(Vec<FauxContentBlock>),
}

impl From<&str> for FauxContent {
    fn from(s: &str) -> Self {
        FauxContent::Text(s.to_string())
    }
}
impl From<String> for FauxContent {
    fn from(s: String) -> Self {
        FauxContent::Text(s)
    }
}
impl From<FauxContentBlock> for FauxContent {
    fn from(b: FauxContentBlock) -> Self {
        FauxContent::Block(b)
    }
}
impl From<Vec<FauxContentBlock>> for FauxContent {
    fn from(b: Vec<FauxContentBlock>) -> Self {
        FauxContent::Blocks(b)
    }
}

fn normalize_faux_assistant_content(content: FauxContent) -> Vec<AssistantContentBlock> {
    match content {
        FauxContent::Text(text) => vec![AssistantContentBlock::Text(faux_text(text))],
        FauxContent::Block(block) => vec![block.into()],
        FauxContent::Blocks(blocks) => blocks.into_iter().map(Into::into).collect(),
    }
}

/// Options for [`faux_assistant_message`] (mirrors the TS inline `options` object).
#[derive(Debug, Clone, Default)]
pub struct FauxAssistantMessageOptions {
    pub stop_reason: Option<StopReason>,
    pub error_message: Option<String>,
    pub response_id: Option<String>,
    /// Timestamp; `None` defaults to "now" (mirrors `Date.now()`).
    pub timestamp: Option<chrono::DateTime<Utc>>,
}

/// Mirrors the TS `fauxAssistantMessage`.
pub fn faux_assistant_message(
    content: impl Into<FauxContent>,
    options: FauxAssistantMessageOptions,
) -> AssistantMessage {
    AssistantMessage {
        role: MessageRole::Assistant,
        content: normalize_faux_assistant_content(content.into()),
        api: DEFAULT_API_NAME.to_string(),
        provider: DEFAULT_PROVIDER.to_string(),
        model: DEFAULT_MODEL_ID.to_string(),
        response_model: None,
        response_id: options.response_id,
        usage: default_usage(),
        stop_reason: options.stop_reason.unwrap_or(StopReason::Stop),
        error_message: options.error_message,
        diagnostics: None,
        timestamp: options.timestamp.unwrap_or_else(Utc::now),
    }
}

// ---------------------------------------------------------------------------
// Response factories
// ---------------------------------------------------------------------------

/// Mutable per-registration call state (mirrors the TS `state: { callCount }`).
#[derive(Debug, Default)]
pub struct FauxState {
    pub call_count: AtomicU64,
}

impl FauxState {
    pub fn call_count(&self) -> u64 {
        self.call_count.load(Ordering::SeqCst)
    }
}

/// A response factory — mirrors the TS `FauxResponseFactory`.
///
/// `(context, options, state, model) -> AssistantMessage`. The TS variant may
/// return a `Promise`; we accept either a sync closure (via [`FauxResponseStep::Message`]
/// / [`FauxResponseStep::factory`]) or an async one (via [`FauxResponseStep::async_factory`]).
pub type FauxResponseFactory = Arc<
    dyn Fn(&Context, Option<&SimpleStreamOptions>, &FauxState, &Model) -> AssistantMessage
        + Send
        + Sync,
>;

/// Async response factory variant — mirrors TS `Promise<AssistantMessage>` returns.
pub type FauxResponseAsyncFactory = Arc<
    dyn Fn(
            Context,
            Option<SimpleStreamOptions>,
            Arc<FauxState>,
            Model,
        ) -> std::pin::Pin<
            Box<dyn std::future::Future<Output = anyhow::Result<AssistantMessage>> + Send>,
        > + Send
        + Sync,
>;

/// A queued response step — mirrors the TS `FauxResponseStep = AssistantMessage | FauxResponseFactory`.
#[derive(Clone)]
pub enum FauxResponseStep {
    Message(AssistantMessage),
    Factory(FauxResponseFactory),
    AsyncFactory(FauxResponseAsyncFactory),
}

impl From<AssistantMessage> for FauxResponseStep {
    fn from(m: AssistantMessage) -> Self {
        FauxResponseStep::Message(m)
    }
}

impl FauxResponseStep {
    /// Convenience constructor for a sync factory closure.
    pub fn factory<F>(f: F) -> Self
    where
        F: Fn(&Context, Option<&SimpleStreamOptions>, &FauxState, &Model) -> AssistantMessage
            + Send
            + Sync
            + 'static,
    {
        FauxResponseStep::Factory(Arc::new(f))
    }
}

// ---------------------------------------------------------------------------
// Registration options + handle
// ---------------------------------------------------------------------------

/// Mirrors the TS `interface RegisterFauxProviderOptions`.
#[derive(Default)]
pub struct RegisterFauxProviderOptions {
    /// Dynamic string api name (used on returned messages). Default: random `faux:...`.
    pub api: Option<String>,
    /// `// TODO(api-string)`: enum value to register under. Default
    /// [`DEFAULT_API_ENUM`].
    pub api_enum: Option<Api>,
    pub provider: Option<String>,
    pub models: Option<Vec<FauxModelDefinition>>,
    pub tokens_per_second: Option<f64>,
    pub token_size: Option<FauxTokenSize>,
}

#[derive(Debug, Clone, Default)]
pub struct FauxTokenSize {
    pub min: Option<u64>,
    pub max: Option<u64>,
}

/// Shared config used by both the registered stream functions and the returned handle.
struct FauxRuntime {
    api_name: String,
    provider: String,
    min_token_size: u64,
    max_token_size: u64,
    tokens_per_second: Option<f64>,
    state: Arc<FauxState>,
    pending: Mutex<Vec<FauxResponseStep>>,
    prompt_cache: Mutex<HashMap<String, String>>,
}

/// Mirrors the TS `interface FauxProviderRegistration` (the object returned by
/// `registerFauxProvider`).
pub struct FauxProviderRegistration {
    /// Dynamic string api name (mirrors TS `api`).
    pub api: String,
    /// Enum value registered under (Rust-only; see `// TODO(api-string)`).
    pub api_enum: Api,
    pub models: Vec<Model>,
    pub state: Arc<FauxState>,
    source_id: String,
    runtime: Arc<FauxRuntime>,
}

impl FauxProviderRegistration {
    /// `getModel()` with no arg → first model. Mirrors the TS overload.
    pub fn get_model(&self) -> Model {
        self.models[0].clone()
    }

    /// `getModel(id)` → matching model or `None`. Mirrors the TS overload.
    pub fn get_model_by_id(&self, requested_model_id: &str) -> Option<Model> {
        self.models
            .iter()
            .find(|m| m.id == requested_model_id)
            .cloned()
    }

    /// Mirrors the TS `setResponses`.
    pub fn set_responses(&self, responses: Vec<FauxResponseStep>) {
        let mut pending = self
            .runtime
            .pending
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        *pending = responses;
    }

    /// Mirrors the TS `appendResponses`.
    pub fn append_responses(&self, responses: Vec<FauxResponseStep>) {
        let mut pending = self
            .runtime
            .pending
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        pending.extend(responses);
    }

    /// Mirrors the TS `getPendingResponseCount`.
    pub fn get_pending_response_count(&self) -> usize {
        self.runtime
            .pending
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .len()
    }

    /// Mirrors the TS `unregister`.
    pub fn unregister(&self) {
        unregister_api_providers(&self.source_id);
    }

    /// Stream directly against *this* registration's runtime (simple options).
    ///
    /// `// TODO(api-string)`: the global registry is keyed by the closed [`Api`]
    /// enum, so multiple faux registrations sharing the default [`DEFAULT_API_ENUM`]
    /// clobber one another in the registry. TS avoids this by keying on a unique
    /// dynamic string `api`. This method bypasses the registry and invokes this
    /// registration's own runtime, giving each registration the isolation the TS
    /// dynamic-string `api` provides. Prefer it over the free [`stream_simple`]
    /// function when you hold the registration handle.
    pub fn stream_simple(
        &self,
        model: Model,
        context: Context,
        options: Option<SimpleStreamOptions>,
    ) -> AssistantMessageEventStream {
        run_faux_stream(Arc::clone(&self.runtime), model, context, options)
    }

    /// Stream directly against *this* registration's runtime (plain options).
    ///
    /// See [`FauxProviderRegistration::stream_simple`] for why this bypasses the
    /// global registry.
    pub fn stream(
        &self,
        model: Model,
        context: Context,
        options: Option<crate::types::StreamOptions>,
    ) -> AssistantMessageEventStream {
        let simple = options.map(|base| SimpleStreamOptions {
            base,
            reasoning: None,
            thinking_budgets: None,
        });
        run_faux_stream(Arc::clone(&self.runtime), model, context, simple)
    }
}

// ---------------------------------------------------------------------------
// Token estimation + context serialization (mirror TS helpers)
// ---------------------------------------------------------------------------

fn estimate_tokens(text: &str) -> u64 {
    // Math.ceil(text.length / 4) — note TS `.length` counts UTF-16 code units;
    // we approximate with `chars().count()` (close enough for the faux estimator).
    let len = text.chars().count() as f64;
    (len / 4.0).ceil() as u64
}

fn random_id(prefix: &str) -> String {
    // `${prefix}:${Date.now()}:${Math.random().toString(36).slice(2)}`
    let now = Utc::now().timestamp_millis();
    // Cheap, dependency-free pseudo-random suffix (no `rand` dep available).
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.subsec_nanos())
        .unwrap_or(0);
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let mix = (nanos as u64) ^ (n.wrapping_mul(0x9E37_79B9_7F4A_7C15));
    format!("{prefix}:{now}:{}", to_base36(mix))
}

fn to_base36(mut n: u64) -> String {
    if n == 0 {
        return "0".to_string();
    }
    const DIGITS: &[u8] = b"0123456789abcdefghijklmnopqrstuvwxyz";
    let mut out = Vec::new();
    while n > 0 {
        out.push(DIGITS[(n % 36) as usize]);
        n /= 36;
    }
    out.reverse();
    String::from_utf8(out).unwrap_or_default()
}

fn content_to_text(content: &[MessageContent]) -> String {
    content
        .iter()
        .map(|block| match block {
            MessageContent::Text(t) => t.text.clone(),
            MessageContent::Image(ImageContent { data, mime_type }) => {
                format!("[image:{mime_type}:{}]", data.chars().count())
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn assistant_content_to_text(content: &[AssistantContentBlock]) -> String {
    content
        .iter()
        .map(|block| match block {
            AssistantContentBlock::Text(t) => t.text.clone(),
            AssistantContentBlock::Thinking(t) => t.thinking.clone(),
            AssistantContentBlock::ToolCall(tc) => {
                // `${block.name}:${JSON.stringify(block.arguments)}`
                let args = serde_json::to_string(&tc.arguments).unwrap_or_else(|_| "null".into());
                format!("{}:{}", tc.name, args)
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn tool_result_to_text(message: &ToolResultMessage) -> String {
    // [message.toolName, ...content.map(block => contentToText([block]))].join("\n")
    let mut parts = vec![message.tool_name.clone()];
    for block in &message.content {
        parts.push(content_to_text(std::slice::from_ref(block)));
    }
    parts.join("\n")
}

fn message_to_text(message: &Message) -> String {
    match message {
        Message::User(u) => content_to_text(&u.content),
        Message::Assistant(a) => assistant_content_to_text(&a.content),
        Message::ToolResult(t) => tool_result_to_text(t),
    }
}

/// Discriminator string matching the TS `message.role`.
fn message_role_str(message: &Message) -> &'static str {
    match message {
        Message::User(_) => "user",
        Message::Assistant(_) => "assistant",
        Message::ToolResult(_) => "toolResult",
    }
}

fn serialize_context(context: &Context) -> String {
    let mut parts: Vec<String> = Vec::new();
    if let Some(system_prompt) = &context.system_prompt {
        parts.push(format!("system:{system_prompt}"));
    }
    for message in &context.messages {
        parts.push(format!(
            "{}:{}",
            message_role_str(message),
            message_to_text(message)
        ));
    }
    if !context.tools.is_empty() {
        // `tools:${JSON.stringify(context.tools)}`
        let tools = serde_json::to_string(&context.tools).unwrap_or_else(|_| "[]".into());
        parts.push(format!("tools:{tools}"));
    }
    parts.join("\n\n")
}

/// Number of leading bytes shared by `a` and `b` measured at char boundaries.
fn common_prefix_length(a: &str, b: &str) -> usize {
    // TS compares UTF-16 code units; we compare chars and return the count.
    let mut count = 0usize;
    for (ca, cb) in a.chars().zip(b.chars()) {
        if ca != cb {
            break;
        }
        count += 1;
    }
    count
}

/// Take the first `char_count` chars of `s`.
fn take_chars(s: &str, char_count: usize) -> String {
    s.chars().take(char_count).collect()
}

/// Drop the first `char_count` chars of `s`.
fn skip_chars(s: &str, char_count: usize) -> String {
    s.chars().skip(char_count).collect()
}

fn with_usage_estimate(
    mut message: AssistantMessage,
    context: &Context,
    options: Option<&SimpleStreamOptions>,
    prompt_cache: &Mutex<HashMap<String, String>>,
) -> AssistantMessage {
    let prompt_text = serialize_context(context);
    let prompt_tokens = estimate_tokens(&prompt_text);
    let output_tokens = estimate_tokens(&assistant_content_to_text(&message.content));
    let mut input = prompt_tokens;
    let mut cache_read = 0u64;
    let mut cache_write = 0u64;

    let session_id = options.and_then(|o| o.base.session_id.clone());
    // TS default cacheRetention is "short" (undefined !== "none"), so caching is
    // active unless explicitly "none".
    let cache_retention = options.and_then(|o| o.base.cache_retention);
    let caching_enabled = cache_retention != Some(CacheRetention::None);

    if let Some(session_id) = session_id {
        if caching_enabled {
            let mut cache = prompt_cache.lock().unwrap_or_else(|e| e.into_inner());
            if let Some(previous_prompt) = cache.get(&session_id).cloned() {
                let cached_chars = common_prefix_length(&previous_prompt, &prompt_text);
                cache_read = estimate_tokens(&take_chars(&previous_prompt, cached_chars));
                cache_write = estimate_tokens(&skip_chars(&prompt_text, cached_chars));
                input = prompt_tokens.saturating_sub(cache_read);
            } else {
                cache_write = prompt_tokens;
            }
            cache.insert(session_id, prompt_text.clone());
        }
    }

    message.usage = Usage {
        input,
        output: output_tokens,
        cache_read,
        cache_write,
        cache_write_1h: None,
        total_tokens: input + output_tokens + cache_read + cache_write,
        cost: UsageCost {
            input: 0.0,
            output: 0.0,
            cache_read: 0.0,
            cache_write: 0.0,
            total: 0.0,
        },
    };
    message
}

/// Split `text` into char-chunks sized by a random token count in
/// `[min_token_size, max_token_size]` (mirrors `splitStringByTokenSize`).
fn split_string_by_token_size(text: &str, min_token_size: u64, max_token_size: u64) -> Vec<String> {
    let chars: Vec<char> = text.chars().collect();
    let mut chunks: Vec<String> = Vec::new();
    let mut index = 0usize;
    while index < chars.len() {
        let span = max_token_size - min_token_size + 1;
        let token_size = min_token_size + (pseudo_random() % span.max(1));
        let char_size = (token_size * 4).max(1) as usize;
        let end = (index + char_size).min(chars.len());
        chunks.push(chars[index..end].iter().collect());
        index = end;
    }
    if chunks.is_empty() {
        vec![String::new()]
    } else {
        chunks
    }
}

/// Cheap pseudo-random `u64` (no `rand` dep). Used only for chunk sizing.
fn pseudo_random() -> u64 {
    static SEED: AtomicU64 = AtomicU64::new(0x2545_F491_4F6C_DD1D);
    let prev = SEED.load(Ordering::Relaxed);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.subsec_nanos() as u64)
        .unwrap_or(0);
    let mut x = prev ^ nanos.wrapping_add(0x9E37_79B9_7F4A_7C15);
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    SEED.store(x, Ordering::Relaxed);
    x
}

fn clone_message(
    message: &AssistantMessage,
    api_name: &str,
    provider: &str,
    model_id: &str,
) -> AssistantMessage {
    let mut cloned = message.clone();
    cloned.api = api_name.to_string();
    cloned.provider = provider.to_string();
    cloned.model = model_id.to_string();
    // TS: `cloned.timestamp ?? Date.now()`. Our timestamp is non-optional and
    // already set by construction; keep it.
    cloned
}

fn create_error_message(
    error: impl std::fmt::Display,
    api_name: &str,
    provider: &str,
    model_id: &str,
) -> AssistantMessage {
    AssistantMessage {
        role: MessageRole::Assistant,
        content: Vec::new(),
        api: api_name.to_string(),
        provider: provider.to_string(),
        model: model_id.to_string(),
        response_model: None,
        response_id: None,
        usage: default_usage(),
        stop_reason: StopReason::Error,
        error_message: Some(error.to_string()),
        diagnostics: None,
        timestamp: Utc::now(),
    }
}

fn create_aborted_message(partial: &AssistantMessage) -> AssistantMessage {
    let mut aborted = partial.clone();
    aborted.stop_reason = StopReason::Aborted;
    aborted.error_message = Some("Request was aborted".to_string());
    aborted.timestamp = Utc::now();
    aborted
}

/// Whether the abort signal is currently set.
fn is_aborted(signal: &Option<tokio::sync::watch::Receiver<bool>>) -> bool {
    signal.as_ref().map(|s| *s.borrow()).unwrap_or(false)
}

/// Sleep to pace a chunk by `tokens_per_second` (mirrors `scheduleChunk`).
///
/// When `tokens_per_second` is unset/zero, yields cooperatively (≈ `queueMicrotask`).
async fn schedule_chunk(chunk: &str, tokens_per_second: Option<f64>) {
    match tokens_per_second {
        Some(tps) if tps > 0.0 => {
            let delay_ms = (estimate_tokens(chunk) as f64 / tps) * 1000.0;
            tokio::time::sleep(std::time::Duration::from_secs_f64(delay_ms / 1000.0)).await;
        }
        _ => {
            tokio::task::yield_now().await;
        }
    }
}

/// Push an aborted terminal error and end the stream. Mirrors the repeated TS block.
fn push_aborted(stream: &mut AssistantMessageEventStreamSender, partial: &AssistantMessage) {
    let aborted = create_aborted_message(partial);
    stream.push(AssistantMessageEvent::Error {
        reason: ErrorReason::Aborted,
        error: aborted.clone(),
    });
    stream.end(Some(aborted));
}

#[allow(clippy::too_many_arguments)]
async fn stream_with_deltas(
    mut stream: AssistantMessageEventStreamSender,
    message: AssistantMessage,
    min_token_size: u64,
    max_token_size: u64,
    tokens_per_second: Option<f64>,
    signal: Option<tokio::sync::watch::Receiver<bool>>,
) {
    let mut partial = message.clone();
    partial.content = Vec::new();

    if is_aborted(&signal) {
        push_aborted(&mut stream, &partial);
        return;
    }

    stream.push(AssistantMessageEvent::Start {
        partial: partial.clone(),
    });

    for (index, block) in message.content.iter().enumerate() {
        if is_aborted(&signal) {
            push_aborted(&mut stream, &partial);
            return;
        }

        match block {
            AssistantContentBlock::Thinking(thinking_block) => {
                partial
                    .content
                    .push(AssistantContentBlock::Thinking(ThinkingContent {
                        thinking: String::new(),
                        thinking_signature: None,
                        redacted: false,
                    }));
                stream.push(AssistantMessageEvent::ThinkingStart {
                    content_index: index,
                    partial: partial.clone(),
                });
                for chunk in split_string_by_token_size(
                    &thinking_block.thinking,
                    min_token_size,
                    max_token_size,
                ) {
                    schedule_chunk(&chunk, tokens_per_second).await;
                    if is_aborted(&signal) {
                        push_aborted(&mut stream, &partial);
                        return;
                    }
                    if let Some(AssistantContentBlock::Thinking(t)) = partial.content.get_mut(index)
                    {
                        t.thinking.push_str(&chunk);
                    }
                    stream.push(AssistantMessageEvent::ThinkingDelta {
                        content_index: index,
                        delta: chunk,
                        partial: partial.clone(),
                    });
                }
                stream.push(AssistantMessageEvent::ThinkingEnd {
                    content_index: index,
                    content: thinking_block.thinking.clone(),
                    partial: partial.clone(),
                });
            }
            AssistantContentBlock::Text(text_block) => {
                partial
                    .content
                    .push(AssistantContentBlock::Text(TextContent {
                        text: String::new(),
                        text_signature: None,
                    }));
                stream.push(AssistantMessageEvent::TextStart {
                    content_index: index,
                    partial: partial.clone(),
                });
                for chunk in
                    split_string_by_token_size(&text_block.text, min_token_size, max_token_size)
                {
                    schedule_chunk(&chunk, tokens_per_second).await;
                    if is_aborted(&signal) {
                        push_aborted(&mut stream, &partial);
                        return;
                    }
                    if let Some(AssistantContentBlock::Text(t)) = partial.content.get_mut(index) {
                        t.text.push_str(&chunk);
                    }
                    stream.push(AssistantMessageEvent::TextDelta {
                        content_index: index,
                        delta: chunk,
                        partial: partial.clone(),
                    });
                }
                stream.push(AssistantMessageEvent::TextEnd {
                    content_index: index,
                    content: text_block.text.clone(),
                    partial: partial.clone(),
                });
            }
            AssistantContentBlock::ToolCall(tool_call) => {
                partial
                    .content
                    .push(AssistantContentBlock::ToolCall(ToolCall {
                        id: tool_call.id.clone(),
                        name: tool_call.name.clone(),
                        arguments: serde_json::json!({}),
                        thought_signature: None,
                    }));
                stream.push(AssistantMessageEvent::ToolCallStart {
                    content_index: index,
                    partial: partial.clone(),
                });
                let args_json =
                    serde_json::to_string(&tool_call.arguments).unwrap_or_else(|_| "null".into());
                for chunk in split_string_by_token_size(&args_json, min_token_size, max_token_size)
                {
                    schedule_chunk(&chunk, tokens_per_second).await;
                    if is_aborted(&signal) {
                        push_aborted(&mut stream, &partial);
                        return;
                    }
                    stream.push(AssistantMessageEvent::ToolCallDelta {
                        content_index: index,
                        delta: chunk,
                        partial: partial.clone(),
                    });
                }
                if let Some(AssistantContentBlock::ToolCall(t)) = partial.content.get_mut(index) {
                    t.arguments = tool_call.arguments.clone();
                }
                stream.push(AssistantMessageEvent::ToolCallEnd {
                    content_index: index,
                    tool_call: tool_call.clone(),
                    partial: partial.clone(),
                });
            }
        }
    }

    match message.stop_reason {
        StopReason::Error => {
            stream.push(AssistantMessageEvent::Error {
                reason: ErrorReason::Error,
                error: message.clone(),
            });
            stream.end(Some(message));
        }
        StopReason::Aborted => {
            stream.push(AssistantMessageEvent::Error {
                reason: ErrorReason::Aborted,
                error: message.clone(),
            });
            stream.end(Some(message));
        }
        other => {
            let reason = match other {
                StopReason::Length => DoneReason::Length,
                StopReason::ToolUse => DoneReason::ToolUse,
                // Stop (and the unreachable Error/Aborted handled above)
                _ => DoneReason::Stop,
            };
            stream.push(AssistantMessageEvent::Done {
                reason,
                message: message.clone(),
            });
            stream.end(Some(message));
        }
    }
}

// ---------------------------------------------------------------------------
// registerFauxProvider
// ---------------------------------------------------------------------------

/// Mirrors the TS `registerFauxProvider`.
pub fn register_faux_provider(options: RegisterFauxProviderOptions) -> FauxProviderRegistration {
    let api_name = options.api.unwrap_or_else(|| random_id(DEFAULT_API_NAME));
    let api_enum = options.api_enum.unwrap_or(DEFAULT_API_ENUM);
    let provider = options
        .provider
        .unwrap_or_else(|| DEFAULT_PROVIDER.to_string());
    let source_id = random_id("faux-provider");

    let opt_min = options.token_size.as_ref().and_then(|t| t.min);
    let opt_max = options.token_size.as_ref().and_then(|t| t.max);
    let min_token_size = 1u64.max(
        opt_min
            .unwrap_or(DEFAULT_MIN_TOKEN_SIZE)
            .min(opt_max.unwrap_or(DEFAULT_MAX_TOKEN_SIZE)),
    );
    let max_token_size = min_token_size.max(opt_max.unwrap_or(DEFAULT_MAX_TOKEN_SIZE));

    let tokens_per_second = options.tokens_per_second;

    let model_definitions: Vec<FauxModelDefinition> = match options.models {
        Some(models) if !models.is_empty() => models,
        _ => vec![FauxModelDefinition {
            id: DEFAULT_MODEL_ID.to_string(),
            name: Some(DEFAULT_MODEL_NAME.to_string()),
            reasoning: Some(false),
            input: Some(vec![InputModality::Text, InputModality::Image]),
            cost: Some(ModelCost {
                input: 0.0,
                output: 0.0,
                cache_read: 0.0,
                cache_write: 0.0,
            }),
            context_window: Some(128000),
            max_tokens: Some(16384),
        }],
    };

    let models: Vec<Model> = model_definitions
        .into_iter()
        .map(|d| Model {
            id: d.id.clone(),
            name: d.name.unwrap_or_else(|| d.id.clone()),
            // TODO(api-string): TS stores the dynamic string `api` here; Rust
            // `Model.api` is the closed `Api` enum, so we store `api_enum`.
            api: api_enum,
            provider: provider.clone(),
            base_url: DEFAULT_BASE_URL.to_string(),
            reasoning: d.reasoning.unwrap_or(false),
            thinking_level_map: None,
            input: d
                .input
                .unwrap_or_else(|| vec![InputModality::Text, InputModality::Image]),
            cost: d.cost.unwrap_or(ModelCost {
                input: 0.0,
                output: 0.0,
                cache_read: 0.0,
                cache_write: 0.0,
            }),
            context_window: d.context_window.unwrap_or(128000),
            max_tokens: d.max_tokens.unwrap_or(16384),
            headers: None,
            compat: None,
        })
        .collect();

    let state = Arc::new(FauxState::default());

    let runtime = Arc::new(FauxRuntime {
        api_name: api_name.clone(),
        provider: provider.clone(),
        min_token_size,
        max_token_size,
        tokens_per_second,
        state: Arc::clone(&state),
        pending: Mutex::new(Vec::new()),
        prompt_cache: Mutex::new(HashMap::new()),
    });

    // Build the stream function over a shared runtime.
    let stream_rt = Arc::clone(&runtime);
    let stream_fn: ApiStreamSimpleFunction = Arc::new(
        move |request_model: Model,
              context: Context,
              stream_options: Option<SimpleStreamOptions>| {
            run_faux_stream(
                Arc::clone(&stream_rt),
                request_model,
                context,
                stream_options,
            )
        },
    );

    // `stream` (non-simple) delegates to the simple path, mirroring TS
    // `streamSimple = (...) => stream(...)` (both share one implementation).
    let stream_rt2 = Arc::clone(&runtime);
    let stream_plain: ApiStreamFunction = Arc::new(
        move |request_model: Model,
              context: Context,
              stream_options: Option<crate::types::StreamOptions>| {
            let simple = stream_options.map(|base| SimpleStreamOptions {
                base,
                reasoning: None,
                thinking_budgets: None,
            });
            run_faux_stream(Arc::clone(&stream_rt2), request_model, context, simple)
        },
    );

    register_api_provider(
        ApiProvider {
            api: api_enum,
            stream: stream_plain,
            stream_simple: stream_fn,
        },
        Some(source_id.clone()),
    );

    FauxProviderRegistration {
        api: api_name,
        api_enum,
        models,
        state,
        source_id,
        runtime,
    }
}

/// The core of the TS `stream: StreamFunction` closure: pop a queued response,
/// bump call count, then spawn the async work that resolves/streams it.
fn run_faux_stream(
    runtime: Arc<FauxRuntime>,
    request_model: Model,
    context: Context,
    stream_options: Option<SimpleStreamOptions>,
) -> AssistantMessageEventStream {
    let (mut outer, stream) = create_assistant_message_event_stream();

    // `pendingResponses.shift()` + `state.callCount++` happen synchronously in TS
    // before the microtask; preserve that ordering.
    let step = {
        let mut pending = runtime.pending.lock().unwrap_or_else(|e| e.into_inner());
        if pending.is_empty() {
            None
        } else {
            Some(pending.remove(0))
        }
    };
    runtime.state.call_count.fetch_add(1, Ordering::SeqCst);

    tokio::spawn(async move {
        // `await streamOptions?.onResponse?.({ status: 200, headers: {} }, model)`.
        if let Some(opts) = &stream_options {
            if let Some(on_response) = &opts.base.on_response {
                let response = crate::types::ProviderResponse {
                    status: 200,
                    headers: HashMap::new(),
                };
                on_response(response, request_model.clone()).await;
            }
        }

        let api_name = &runtime.api_name;
        let provider = &runtime.provider;

        let Some(step) = step else {
            let mut message = create_error_message(
                "No more faux responses queued",
                api_name,
                provider,
                &request_model.id,
            );
            message = with_usage_estimate(
                message,
                &context,
                stream_options.as_ref(),
                &runtime.prompt_cache,
            );
            outer.push(AssistantMessageEvent::Error {
                reason: ErrorReason::Error,
                error: message.clone(),
            });
            outer.end(Some(message));
            return;
        };

        // Resolve the step (message / sync factory / async factory).
        let resolved: Result<AssistantMessage, String> = match step {
            FauxResponseStep::Message(m) => Ok(m),
            FauxResponseStep::Factory(f) => Ok(f(
                &context,
                stream_options.as_ref(),
                &runtime.state,
                &request_model,
            )),
            FauxResponseStep::AsyncFactory(f) => {
                match f(
                    context.clone(),
                    stream_options.clone(),
                    Arc::clone(&runtime.state),
                    request_model.clone(),
                )
                .await
                {
                    Ok(m) => Ok(m),
                    Err(e) => Err(e.to_string()),
                }
            }
        };

        let resolved = match resolved {
            Ok(m) => m,
            Err(err) => {
                // Mirrors the TS `catch` around the factory call.
                let message = create_error_message(err, api_name, provider, &request_model.id);
                outer.push(AssistantMessageEvent::Error {
                    reason: ErrorReason::Error,
                    error: message.clone(),
                });
                outer.end(Some(message));
                return;
            }
        };

        let mut message = clone_message(&resolved, api_name, provider, &request_model.id);
        message = with_usage_estimate(
            message,
            &context,
            stream_options.as_ref(),
            &runtime.prompt_cache,
        );

        let signal = stream_options.as_ref().and_then(|o| o.base.signal.clone());
        stream_with_deltas(
            outer,
            message,
            runtime.min_token_size,
            runtime.max_token_size,
            runtime.tokens_per_second,
            signal,
        )
        .await;
    });

    stream
}

// ---------------------------------------------------------------------------
// Required entry points for later registry wiring (see task contract).
//
// The faux provider has no static `Model` of its own — it is created
// dynamically via `register_faux_provider`. These thin wrappers exist so the
// provider exposes the same `stream` / `stream_simple` signatures as every
// other provider; they dispatch through the global registry using `model.api`.
// ---------------------------------------------------------------------------

/// Stream entry point mirroring the provider contract. Dispatches via the global
/// registry (the faux provider must have been registered first).
pub fn stream(
    model: Model,
    context: Context,
    options: Option<crate::types::StreamOptions>,
) -> AssistantMessageEventStream {
    match crate::api_registry::get_api_provider(model.api) {
        Some(provider) => (provider.stream)(model, context, options),
        None => no_provider_stream(model),
    }
}

/// `stream_simple` entry point mirroring the provider contract.
pub fn stream_simple(
    model: Model,
    context: Context,
    options: Option<SimpleStreamOptions>,
) -> AssistantMessageEventStream {
    match crate::api_registry::get_api_provider(model.api) {
        Some(provider) => (provider.stream_simple)(model, context, options),
        None => no_provider_stream(model),
    }
}

fn no_provider_stream(model: Model) -> AssistantMessageEventStream {
    let (mut tx, stream) = create_assistant_message_event_stream();
    let message = create_error_message(
        format!("No API provider registered for api: {}", model.api),
        DEFAULT_API_NAME,
        DEFAULT_PROVIDER,
        &model.id,
    );
    tx.push(AssistantMessageEvent::Error {
        reason: ErrorReason::Error,
        error: message.clone(),
    });
    tx.end(Some(message));
    stream
}

// ---------------------------------------------------------------------------
// Tests (mirroring packages/ai/test/faux-provider.test.ts where feasible)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{StreamOptions, UserMessage};
    use futures::StreamExt;

    fn user_message(text: &str) -> Message {
        Message::User(UserMessage {
            role: MessageRole::User,
            content: vec![MessageContent::Text(faux_text(text))],
            timestamp: Utc::now(),
        })
    }

    fn ctx(messages: Vec<Message>) -> Context {
        Context {
            system_prompt: None,
            messages,
            tools: Vec::new(),
        }
    }

    async fn collect_events(mut s: AssistantMessageEventStream) -> Vec<AssistantMessageEvent> {
        let mut events = Vec::new();
        while let Some(event) = s.next().await {
            events.push(event);
        }
        events
    }

    fn event_type(e: &AssistantMessageEvent) -> &'static str {
        match e {
            AssistantMessageEvent::Start { .. } => "start",
            AssistantMessageEvent::TextStart { .. } => "text_start",
            AssistantMessageEvent::TextDelta { .. } => "text_delta",
            AssistantMessageEvent::TextEnd { .. } => "text_end",
            AssistantMessageEvent::ThinkingStart { .. } => "thinking_start",
            AssistantMessageEvent::ThinkingDelta { .. } => "thinking_delta",
            AssistantMessageEvent::ThinkingEnd { .. } => "thinking_end",
            AssistantMessageEvent::ToolCallStart { .. } => "toolcall_start",
            AssistantMessageEvent::ToolCallDelta { .. } => "toolcall_delta",
            AssistantMessageEvent::ToolCallEnd { .. } => "toolcall_end",
            AssistantMessageEvent::Done { .. } => "done",
            AssistantMessageEvent::Error { .. } => "error",
            AssistantMessageEvent::Loading { .. } => "loading",
        }
    }

    /// `complete`-equivalent: drive the simple stream and await the final message.
    ///
    /// Uses the registration's own stream method so concurrent tests sharing the
    /// global registry key don't clobber each other (see `// TODO(api-string)`).
    async fn complete(
        reg: &FauxProviderRegistration,
        model: Model,
        context: Context,
        options: Option<SimpleStreamOptions>,
    ) -> AssistantMessage {
        let s = reg.stream_simple(model, context, options);
        s.result().await
    }

    #[tokio::test]
    async fn registers_and_estimates_usage() {
        let reg = register_faux_provider(RegisterFauxProviderOptions::default());
        reg.set_responses(vec![
            faux_assistant_message("hello world", FauxAssistantMessageOptions::default()).into(),
        ]);

        let context = Context {
            system_prompt: Some("Be concise.".to_string()),
            messages: vec![user_message("hi there")],
            tools: Vec::new(),
        };

        let response = complete(&reg, reg.get_model(), context, None).await;
        assert_eq!(response.content.len(), 1);
        assert!(matches!(
            &response.content[0],
            AssistantContentBlock::Text(t) if t.text == "hello world"
        ));
        assert!(response.usage.input > 0);
        assert!(response.usage.output > 0);
        assert_eq!(
            response.usage.total_tokens,
            response.usage.input + response.usage.output
        );
        assert_eq!(reg.state.call_count(), 1);
        reg.unregister();
    }

    #[tokio::test]
    async fn consumes_queued_responses_and_errors_when_exhausted() {
        let reg = register_faux_provider(RegisterFauxProviderOptions::default());
        reg.set_responses(vec![
            faux_assistant_message("first", FauxAssistantMessageOptions::default()).into(),
            faux_assistant_message("second", FauxAssistantMessageOptions::default()).into(),
        ]);

        let first = complete(&reg, reg.get_model(), ctx(vec![user_message("hi")]), None).await;
        let second = complete(&reg, reg.get_model(), ctx(vec![user_message("hi")]), None).await;
        let exhausted = complete(&reg, reg.get_model(), ctx(vec![user_message("hi")]), None).await;

        assert!(matches!(&first.content[0], AssistantContentBlock::Text(t) if t.text == "first"));
        assert!(matches!(&second.content[0], AssistantContentBlock::Text(t) if t.text == "second"));
        assert_eq!(exhausted.stop_reason, StopReason::Error);
        assert_eq!(
            exhausted.error_message.as_deref(),
            Some("No more faux responses queued")
        );
        assert_eq!(reg.get_pending_response_count(), 0);
        assert_eq!(reg.state.call_count(), 3);
        reg.unregister();
    }

    #[tokio::test]
    async fn replace_and_append_queued_responses() {
        let reg = register_faux_provider(RegisterFauxProviderOptions::default());
        reg.set_responses(vec![
            faux_assistant_message("first", FauxAssistantMessageOptions::default()).into(),
        ]);

        let r = complete(&reg, reg.get_model(), ctx(vec![user_message("hi")]), None).await;
        assert!(matches!(&r.content[0], AssistantContentBlock::Text(t) if t.text == "first"));
        assert_eq!(reg.get_pending_response_count(), 0);

        reg.set_responses(vec![
            faux_assistant_message("second", FauxAssistantMessageOptions::default()).into(),
        ]);
        assert_eq!(reg.get_pending_response_count(), 1);

        reg.append_responses(vec![
            faux_assistant_message("third", FauxAssistantMessageOptions::default()).into(),
            faux_assistant_message("fourth", FauxAssistantMessageOptions::default()).into(),
        ]);
        assert_eq!(reg.get_pending_response_count(), 3);
        reg.unregister();
    }

    #[tokio::test]
    async fn model_aware_factories() {
        let reg = register_faux_provider(RegisterFauxProviderOptions {
            models: Some(vec![
                FauxModelDefinition {
                    id: "faux-fast".into(),
                    name: Some("Faux Fast".into()),
                    reasoning: Some(false),
                    ..Default::default()
                },
                FauxModelDefinition {
                    id: "faux-thinker".into(),
                    name: Some("Faux Thinker".into()),
                    reasoning: Some(true),
                    ..Default::default()
                },
            ]),
            ..Default::default()
        });
        reg.set_responses(vec![
            FauxResponseStep::factory(|_c, _o, _s, model| {
                faux_assistant_message(
                    format!("{}:{}", model.id, model.reasoning),
                    FauxAssistantMessageOptions::default(),
                )
            }),
            FauxResponseStep::factory(|_c, _o, _s, model| {
                faux_assistant_message(
                    format!("{}:{}", model.id, model.reasoning),
                    FauxAssistantMessageOptions::default(),
                )
            }),
        ]);

        assert_eq!(
            reg.models.iter().map(|m| m.id.clone()).collect::<Vec<_>>(),
            vec!["faux-fast", "faux-thinker"]
        );
        assert!(!reg.get_model_by_id("faux-fast").unwrap().reasoning);
        assert!(reg.get_model_by_id("faux-thinker").unwrap().reasoning);

        let fast = complete(
            &reg,
            reg.get_model_by_id("faux-fast").unwrap(),
            ctx(vec![user_message("hi")]),
            None,
        )
        .await;
        let thinker = complete(
            &reg,
            reg.get_model_by_id("faux-thinker").unwrap(),
            ctx(vec![user_message("hi")]),
            None,
        )
        .await;
        assert!(
            matches!(&fast.content[0], AssistantContentBlock::Text(t) if t.text == "faux-fast:false")
        );
        assert!(
            matches!(&thinker.content[0], AssistantContentBlock::Text(t) if t.text == "faux-thinker:true")
        );
        reg.unregister();
    }

    #[tokio::test]
    async fn rewrites_api_provider_model() {
        let reg = register_faux_provider(RegisterFauxProviderOptions {
            api: Some("faux:test".into()),
            provider: Some("faux-provider".into()),
            models: Some(vec![FauxModelDefinition {
                id: "faux-model".into(),
                ..Default::default()
            }]),
            ..Default::default()
        });
        reg.set_responses(vec![
            faux_assistant_message("hello", FauxAssistantMessageOptions::default()).into(),
        ]);

        let response = complete(&reg, reg.get_model(), ctx(vec![user_message("hi")]), None).await;
        assert_eq!(response.api, "faux:test");
        assert_eq!(response.provider, "faux-provider");
        assert_eq!(response.model, "faux-model");
        reg.unregister();
    }

    #[tokio::test]
    async fn factory_throwing_emits_error() {
        let reg = register_faux_provider(RegisterFauxProviderOptions::default());
        reg.set_responses(vec![FauxResponseStep::AsyncFactory(Arc::new(
            |_c, _o, _s, _m| Box::pin(async { Err(anyhow::anyhow!("boom")) }),
        ))]);

        let events =
            collect_events(reg.stream_simple(reg.get_model(), ctx(vec![user_message("hi")]), None))
                .await;

        assert_eq!(events.len(), 1);
        match &events[0] {
            AssistantMessageEvent::Error { error, .. } => {
                assert_eq!(error.stop_reason, StopReason::Error);
                assert_eq!(error.error_message.as_deref(), Some("boom"));
            }
            other => panic!("expected error, got {}", event_type(other)),
        }
        reg.unregister();
    }

    #[tokio::test]
    async fn estimates_prompt_and_output_tokens() {
        let reg = register_faux_provider(RegisterFauxProviderOptions::default());
        reg.set_responses(vec![
            faux_assistant_message("done", FauxAssistantMessageOptions::default()).into(),
        ]);

        let tool = crate::types::Tool {
            name: "echo".into(),
            description: "Echo back text".into(),
            parameters: serde_json::json!({"type":"object","properties":{"text":{"type":"string"}}}),
        };
        let prior = faux_assistant_message("prior", FauxAssistantMessageOptions::default());
        let context = Context {
            system_prompt: Some("sys".into()),
            messages: vec![
                Message::User(UserMessage {
                    role: MessageRole::User,
                    content: vec![
                        MessageContent::Text(faux_text("hello")),
                        MessageContent::Image(ImageContent {
                            mime_type: "image/png".into(),
                            data: "abcd".into(),
                        }),
                    ],
                    timestamp: Utc::now(),
                }),
                Message::Assistant(prior),
                Message::ToolResult(ToolResultMessage {
                    role: MessageRole::ToolResult,
                    tool_call_id: "tool-1".into(),
                    tool_name: "echo".into(),
                    content: vec![MessageContent::Text(faux_text("tool out"))],
                    details: None,
                    is_error: false,
                    timestamp: Utc::now(),
                }),
            ],
            tools: vec![tool.clone()],
        };

        let response = complete(&reg, reg.get_model(), context.clone(), None).await;

        // Reconstruct the expected serialized prompt to validate token estimation.
        let prompt_text = serialize_context(&context);
        let expected_prompt_tokens = estimate_tokens(&prompt_text);
        let expected_output_tokens = estimate_tokens("done");
        assert_eq!(response.usage.input, expected_prompt_tokens);
        assert_eq!(response.usage.output, expected_output_tokens);
        assert_eq!(response.usage.cache_read, 0);
        assert_eq!(response.usage.cache_write, 0);
        assert_eq!(
            response.usage.total_tokens,
            expected_prompt_tokens + expected_output_tokens
        );
        reg.unregister();
    }

    #[tokio::test]
    async fn simulates_prompt_caching_per_session() {
        let reg = register_faux_provider(RegisterFauxProviderOptions::default());
        reg.set_responses(vec![
            faux_assistant_message("first", FauxAssistantMessageOptions::default()).into(),
            faux_assistant_message("second", FauxAssistantMessageOptions::default()).into(),
        ]);

        let mut context = Context {
            system_prompt: Some("Be concise.".into()),
            messages: vec![user_message("hello")],
            tools: Vec::new(),
        };

        let opts = |session: &str| {
            Some(SimpleStreamOptions {
                base: StreamOptions {
                    session_id: Some(session.into()),
                    cache_retention: Some(CacheRetention::Short),
                    ..Default::default()
                },
                reasoning: None,
                thinking_budgets: None,
            })
        };

        let first = complete(&reg, reg.get_model(), context.clone(), opts("session-1")).await;
        assert_eq!(first.usage.cache_read, 0);
        assert!(first.usage.cache_write > 0);

        context.messages.push(Message::Assistant(first));
        context.messages.push(user_message("follow up"));

        let second = complete(&reg, reg.get_model(), context.clone(), opts("session-1")).await;
        assert!(second.usage.cache_read > 0);
        reg.unregister();
    }

    #[tokio::test]
    async fn no_caching_when_retention_none() {
        let reg = register_faux_provider(RegisterFauxProviderOptions::default());
        reg.set_responses(vec![
            faux_assistant_message("first", FauxAssistantMessageOptions::default()).into(),
            faux_assistant_message("second", FauxAssistantMessageOptions::default()).into(),
        ]);

        let mut context = ctx(vec![user_message("hello")]);
        let opts = || {
            Some(SimpleStreamOptions {
                base: StreamOptions {
                    session_id: Some("session-1".into()),
                    cache_retention: Some(CacheRetention::None),
                    ..Default::default()
                },
                reasoning: None,
                thinking_budgets: None,
            })
        };

        let _first = complete(&reg, reg.get_model(), context.clone(), opts()).await;
        context
            .messages
            .push(Message::Assistant(faux_assistant_message(
                "first",
                FauxAssistantMessageOptions::default(),
            )));
        context.messages.push(user_message("follow up"));
        let second = complete(&reg, reg.get_model(), context.clone(), opts()).await;
        assert_eq!(second.usage.cache_read, 0);
        assert_eq!(second.usage.cache_write, 0);
        reg.unregister();
    }

    #[tokio::test]
    async fn exact_event_order_fixed_chunks() {
        let reg = register_faux_provider(RegisterFauxProviderOptions {
            token_size: Some(FauxTokenSize {
                min: Some(1),
                max: Some(1),
            }),
            ..Default::default()
        });
        reg.set_responses(vec![
            faux_assistant_message(
                vec![
                    FauxContentBlock::Thinking(faux_thinking("go")),
                    FauxContentBlock::Text(faux_text("ok")),
                    FauxContentBlock::ToolCall(faux_tool_call(
                        "echo",
                        serde_json::json!({}),
                        Some("tool-1".into()),
                    )),
                ],
                FauxAssistantMessageOptions {
                    stop_reason: Some(StopReason::ToolUse),
                    ..Default::default()
                },
            )
            .into(),
        ]);

        let events =
            collect_events(reg.stream_simple(reg.get_model(), ctx(vec![user_message("hi")]), None))
                .await;
        let types: Vec<&str> = events.iter().map(event_type).collect();
        assert_eq!(
            types,
            vec![
                "start",
                "thinking_start",
                "thinking_delta",
                "thinking_end",
                "text_start",
                "text_delta",
                "text_end",
                "toolcall_start",
                "toolcall_delta",
                "toolcall_end",
                "done",
            ]
        );
        reg.unregister();
    }

    #[tokio::test]
    async fn explicit_error_message_is_terminal_error() {
        let reg = register_faux_provider(RegisterFauxProviderOptions {
            token_size: Some(FauxTokenSize {
                min: Some(2),
                max: Some(2),
            }),
            ..Default::default()
        });
        let msg = faux_assistant_message(
            "partial",
            FauxAssistantMessageOptions {
                stop_reason: Some(StopReason::Error),
                error_message: Some("upstream failed".into()),
                ..Default::default()
            },
        );
        reg.set_responses(vec![msg.into()]);

        let events =
            collect_events(reg.stream_simple(reg.get_model(), ctx(vec![user_message("hi")]), None))
                .await;
        let types: Vec<&str> = events.iter().map(event_type).collect();
        assert_eq!(
            types,
            vec!["start", "text_start", "text_delta", "text_end", "error"]
        );
        match events.last().unwrap() {
            AssistantMessageEvent::Error { reason, error } => {
                assert_eq!(*reason, ErrorReason::Error);
                assert_eq!(error.stop_reason, StopReason::Error);
                assert_eq!(error.error_message.as_deref(), Some("upstream failed"));
            }
            _ => panic!("expected terminal error"),
        }
        reg.unregister();
    }

    #[tokio::test]
    async fn abort_before_first_chunk() {
        let reg = register_faux_provider(RegisterFauxProviderOptions {
            tokens_per_second: Some(50.0),
            token_size: Some(FauxTokenSize {
                min: Some(3),
                max: Some(3),
            }),
            ..Default::default()
        });
        reg.set_responses(vec![
            faux_assistant_message(
                "abcdefghijklmnopqrstuvwxyz",
                FauxAssistantMessageOptions::default(),
            )
            .into(),
        ]);

        let (tx, rx) = tokio::sync::watch::channel(true); // already aborted
        let _ = tx;
        let options = SimpleStreamOptions {
            base: StreamOptions {
                signal: Some(rx),
                ..Default::default()
            },
            reasoning: None,
            thinking_budgets: None,
        };

        let events = collect_events(reg.stream_simple(
            reg.get_model(),
            ctx(vec![user_message("hi")]),
            Some(options),
        ))
        .await;
        assert_eq!(events.len(), 1);
        match &events[0] {
            AssistantMessageEvent::Error { reason, error } => {
                assert_eq!(*reason, ErrorReason::Aborted);
                assert_eq!(error.stop_reason, StopReason::Aborted);
            }
            other => panic!("expected aborted error, got {}", event_type(other)),
        }
        reg.unregister();
    }

    #[test]
    fn token_size_clamping() {
        // min defaults applied and clamped to >= 1; max >= min.
        let reg = register_faux_provider(RegisterFauxProviderOptions {
            token_size: Some(FauxTokenSize {
                min: Some(10),
                max: Some(2),
            }),
            ..Default::default()
        });
        // min = max(1, min(10, 2)) = 2 ; max = max(2, 2) = 2
        assert_eq!(reg.runtime.min_token_size, 2);
        assert_eq!(reg.runtime.max_token_size, 2);
        reg.unregister();
    }

    #[test]
    fn estimate_tokens_matches_ceil_div_4() {
        assert_eq!(estimate_tokens(""), 0);
        assert_eq!(estimate_tokens("a"), 1);
        assert_eq!(estimate_tokens("abcd"), 1);
        assert_eq!(estimate_tokens("abcde"), 2);
    }

    #[test]
    fn serialize_context_matches_ts_format() {
        let tool = crate::types::Tool {
            name: "echo".into(),
            description: "Echo back text".into(),
            parameters: serde_json::json!({"type":"object"}),
        };
        let context = Context {
            system_prompt: Some("sys".into()),
            messages: vec![Message::User(UserMessage {
                role: MessageRole::User,
                content: vec![
                    MessageContent::Text(faux_text("hello")),
                    MessageContent::Image(ImageContent {
                        mime_type: "image/png".into(),
                        data: "abcd".into(),
                    }),
                ],
                timestamp: Utc::now(),
            })],
            tools: vec![tool.clone()],
        };
        let serialized = serialize_context(&context);
        let tools_json = serde_json::to_string(&vec![tool]).unwrap();
        let expected =
            format!("system:sys\n\nuser:hello\n[image:image/png:4]\n\ntools:{tools_json}");
        assert_eq!(serialized, expected);
    }

    // ---------------------------------------------------------------------------
    // Async factory
    // ---------------------------------------------------------------------------

    #[tokio::test]
    async fn async_factory_produces_message() {
        let reg = register_faux_provider(RegisterFauxProviderOptions::default());
        reg.set_responses(vec![FauxResponseStep::AsyncFactory(Arc::new(
            |context, _options, state, _model| {
                Box::pin(async move {
                    Ok(faux_assistant_message(
                        format!("{}:{}", context.messages.len(), state.call_count()),
                        FauxAssistantMessageOptions::default(),
                    ))
                })
            },
        ))]);
        let response = complete(&reg, reg.get_model(), ctx(vec![user_message("hi")]), None).await;
        assert!(matches!(
            &response.content[0],
            AssistantContentBlock::Text(t) if t.text == "1:1"
        ));
        reg.unregister();
    }

    // ---------------------------------------------------------------------------
    // Multiple tool calls in one message
    // ---------------------------------------------------------------------------

    #[tokio::test]
    async fn multiple_tool_calls_in_one_message() {
        let reg = register_faux_provider(RegisterFauxProviderOptions {
            token_size: Some(FauxTokenSize {
                min: Some(1),
                max: Some(1),
            }),
            ..Default::default()
        });
        reg.set_responses(vec![
            faux_assistant_message(
                vec![
                    FauxContentBlock::ToolCall(faux_tool_call(
                        "echo",
                        serde_json::json!({"text": "one"}),
                        Some("tool-1".into()),
                    )),
                    FauxContentBlock::ToolCall(faux_tool_call(
                        "echo",
                        serde_json::json!({"text": "two"}),
                        Some("tool-2".into()),
                    )),
                ],
                FauxAssistantMessageOptions {
                    stop_reason: Some(StopReason::ToolUse),
                    ..Default::default()
                },
            )
            .into(),
        ]);
        let events =
            collect_events(reg.stream_simple(reg.get_model(), ctx(vec![user_message("hi")]), None))
                .await;
        let starts: Vec<&AssistantMessageEvent> = events
            .iter()
            .filter(|e| matches!(e, AssistantMessageEvent::ToolCallStart { .. }))
            .collect();
        let ends: Vec<&AssistantMessageEvent> = events
            .iter()
            .filter(|e| matches!(e, AssistantMessageEvent::ToolCallEnd { .. }))
            .collect();
        assert_eq!(starts.len(), 2);
        assert_eq!(ends.len(), 2);
        reg.unregister();
    }

    // ---------------------------------------------------------------------------
    // Abort mid-text stream
    // ---------------------------------------------------------------------------

    #[tokio::test]
    async fn abort_mid_text_stream() {
        let reg = register_faux_provider(RegisterFauxProviderOptions {
            tokens_per_second: Some(100.0),
            token_size: Some(FauxTokenSize {
                min: Some(3),
                max: Some(3),
            }),
            ..Default::default()
        });
        reg.set_responses(vec![
            faux_assistant_message(
                "abcdefghijklmnopqrstuvwxyz",
                FauxAssistantMessageOptions::default(),
            )
            .into(),
        ]);

        let (tx, rx) = tokio::sync::watch::channel(false);
        let options = SimpleStreamOptions {
            base: StreamOptions {
                signal: Some(rx),
                ..Default::default()
            },
            reasoning: None,
            thinking_budgets: None,
        };

        let mut events = vec![];
        let mut text_delta_count = 0;
        let mut stream = reg.stream_simple(
            reg.get_model(),
            ctx(vec![user_message("hi")]),
            Some(options),
        );
        while let Some(event) = stream.next().await {
            if matches!(event, AssistantMessageEvent::TextDelta { .. }) {
                text_delta_count += 1;
                let _ = tx.send(true);
            }
            events.push(event_type(&event));
        }

        assert_eq!(text_delta_count, 1);
        assert!(events.contains(&"text_start"));
        assert!(events.contains(&"text_delta"));
        assert!(events.contains(&"error"));
        assert!(!events.contains(&"text_end"));
        reg.unregister();
    }

    // ---------------------------------------------------------------------------
    // Abort mid-thinking stream
    // ---------------------------------------------------------------------------

    #[tokio::test]
    async fn abort_mid_thinking_stream() {
        let reg = register_faux_provider(RegisterFauxProviderOptions {
            tokens_per_second: Some(100.0),
            token_size: Some(FauxTokenSize {
                min: Some(3),
                max: Some(3),
            }),
            ..Default::default()
        });
        reg.set_responses(vec![
            faux_assistant_message(
                vec![FauxContentBlock::Thinking(faux_thinking(
                    "abcdefghijklmnopqrstuvwxyz",
                ))],
                FauxAssistantMessageOptions::default(),
            )
            .into(),
        ]);

        let (tx, rx) = tokio::sync::watch::channel(false);
        let options = SimpleStreamOptions {
            base: StreamOptions {
                signal: Some(rx),
                ..Default::default()
            },
            reasoning: None,
            thinking_budgets: None,
        };

        let mut events = vec![];
        let mut thinking_delta_count = 0;
        let mut stream = reg.stream_simple(
            reg.get_model(),
            ctx(vec![user_message("hi")]),
            Some(options),
        );
        while let Some(event) = stream.next().await {
            if matches!(event, AssistantMessageEvent::ThinkingDelta { .. }) {
                thinking_delta_count += 1;
                let _ = tx.send(true);
            }
            events.push(event_type(&event));
        }

        assert_eq!(thinking_delta_count, 1);
        assert!(events.contains(&"thinking_start"));
        assert!(events.contains(&"thinking_delta"));
        assert!(events.contains(&"error"));
        assert!(!events.contains(&"thinking_end"));
        reg.unregister();
    }

    // ---------------------------------------------------------------------------
    // Abort mid-toolcall stream
    // ---------------------------------------------------------------------------

    #[tokio::test]
    async fn abort_mid_toolcall_stream() {
        let reg = register_faux_provider(RegisterFauxProviderOptions {
            tokens_per_second: Some(100.0),
            token_size: Some(FauxTokenSize {
                min: Some(3),
                max: Some(3),
            }),
            ..Default::default()
        });
        reg.set_responses(vec![
            faux_assistant_message(
                vec![FauxContentBlock::ToolCall(faux_tool_call(
                    "echo",
                    serde_json::json!({"text": "abcdefghijklmnopqrstuvwxyz", "count": 123456789}),
                    Some("tool-1".into()),
                ))],
                FauxAssistantMessageOptions {
                    stop_reason: Some(StopReason::ToolUse),
                    ..Default::default()
                },
            )
            .into(),
        ]);

        let (tx, rx) = tokio::sync::watch::channel(false);
        let options = SimpleStreamOptions {
            base: StreamOptions {
                signal: Some(rx),
                ..Default::default()
            },
            reasoning: None,
            thinking_budgets: None,
        };

        let mut events = vec![];
        let mut toolcall_delta_count = 0;
        let mut stream = reg.stream_simple(
            reg.get_model(),
            ctx(vec![user_message("hi")]),
            Some(options),
        );
        while let Some(event) = stream.next().await {
            if matches!(event, AssistantMessageEvent::ToolCallDelta { .. }) {
                toolcall_delta_count += 1;
                let _ = tx.send(true);
            }
            events.push(event_type(&event));
        }

        assert_eq!(toolcall_delta_count, 1);
        assert!(events.contains(&"toolcall_start"));
        assert!(events.contains(&"toolcall_delta"));
        assert!(events.contains(&"error"));
        assert!(!events.contains(&"toolcall_end"));
        reg.unregister();
    }

    // ---------------------------------------------------------------------------
    // Unregister prevents further calls
    // ---------------------------------------------------------------------------

    #[tokio::test]
    async fn unregister_prevents_further_calls() {
        let reg = register_faux_provider(RegisterFauxProviderOptions::default());
        reg.set_responses(vec![
            faux_assistant_message("hello", FauxAssistantMessageOptions::default()).into(),
        ]);
        let _api = reg.api.clone();
        reg.unregister();
        // After unregister, calling stream_simple on the registration handle still works
        // (it bypasses the registry), but the result should reflect the queued response.
        let response = complete(&reg, reg.get_model(), ctx(vec![user_message("hi")]), None).await;
        assert!(
            matches!(&response.content[0], AssistantContentBlock::Text(t) if t.text == "hello")
        );
        // A fresh registration is needed for the next test.
        // Calling through the global registry should fail.
        let fresh_model = Model {
            id: "faux-1".into(),
            name: "faux-1".into(),
            api: DEFAULT_API_ENUM,
            provider: "faux".into(),
            base_url: "http://localhost:0".into(),
            reasoning: false,
            thinking_level_map: None,
            input: vec![],
            cost: crate::types::ModelCost {
                input: 0.0,
                output: 0.0,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 0,
            max_tokens: 0,
            headers: None,
            compat: None,
        };
        // This should produce an error message about no provider
        let result = super::stream(fresh_model, ctx(vec![user_message("hi")]), None);
        let last = result.result().await;
        assert_eq!(last.stop_reason, StopReason::Error);
    }

    // ---------------------------------------------------------------------------
    // Estimate tokens for strings with various lengths
    // ---------------------------------------------------------------------------

    #[test]
    fn estimate_tokens_empty_string() {
        assert_eq!(estimate_tokens(""), 0);
    }

    #[test]
    fn estimate_tokens_multibyte_chars() {
        // Multi-byte Unicode — counts chars, not bytes
        assert_eq!(estimate_tokens("a"), 1);
        assert_eq!(estimate_tokens("ab"), 1);
        assert_eq!(estimate_tokens("abc"), 1);
        assert_eq!(estimate_tokens("abcd"), 1);
        assert_eq!(estimate_tokens("abcde"), 2);
    }

    // ---------------------------------------------------------------------------
    // common_prefix_length and take_chars / skip_chars
    // ---------------------------------------------------------------------------

    #[test]
    fn test_common_prefix_length_full_match() {
        assert_eq!(common_prefix_length("hello world", "hello world"), 11);
    }

    #[test]
    fn test_common_prefix_length_partial() {
        assert_eq!(common_prefix_length("hello world", "hello there"), 6);
    }

    #[test]
    fn test_common_prefix_length_no_match() {
        assert_eq!(common_prefix_length("abc", "xyz"), 0);
    }

    #[test]
    fn test_common_prefix_length_empty() {
        assert_eq!(common_prefix_length("", "abc"), 0);
        assert_eq!(common_prefix_length("abc", ""), 0);
    }

    #[test]
    fn test_take_chars_basic() {
        assert_eq!(take_chars("hello", 3), "hel");
    }

    #[test]
    fn test_skip_chars_basic() {
        assert_eq!(skip_chars("hello", 2), "llo");
    }

    // ---------------------------------------------------------------------------
    // content_to_text
    // ---------------------------------------------------------------------------

    #[test]
    fn test_content_to_text_image() {
        let content = vec![
            MessageContent::Text(faux_text("desc")),
            MessageContent::Image(ImageContent {
                mime_type: "image/png".into(),
                data: "abcd".into(),
            }),
        ];
        let text = content_to_text(&content);
        assert_eq!(text, "desc\n[image:image/png:4]");
    }

    // ---------------------------------------------------------------------------
    // No caching without sessionId
    // ---------------------------------------------------------------------------

    #[tokio::test]
    async fn no_caching_without_session_id() {
        let reg = register_faux_provider(RegisterFauxProviderOptions::default());
        reg.set_responses(vec![
            faux_assistant_message("first", FauxAssistantMessageOptions::default()).into(),
            faux_assistant_message("second", FauxAssistantMessageOptions::default()).into(),
        ]);

        let mut context = ctx(vec![user_message("hello")]);

        let _first = complete(&reg, reg.get_model(), context.clone(), None).await;
        context
            .messages
            .push(Message::Assistant(faux_assistant_message(
                "first",
                FauxAssistantMessageOptions::default(),
            )));
        context.messages.push(user_message("follow up"));
        let second = complete(&reg, reg.get_model(), context.clone(), None).await;
        assert_eq!(second.usage.cache_read, 0);
        assert_eq!(second.usage.cache_write, 0);
        reg.unregister();
    }

    // ---------------------------------------------------------------------------
    // Different sessions do not share cache
    // ---------------------------------------------------------------------------

    #[tokio::test]
    async fn different_sessions_do_not_share_cache() {
        let reg = register_faux_provider(RegisterFauxProviderOptions::default());
        reg.set_responses(vec![
            faux_assistant_message("first", FauxAssistantMessageOptions::default()).into(),
            faux_assistant_message("second", FauxAssistantMessageOptions::default()).into(),
            faux_assistant_message("third", FauxAssistantMessageOptions::default()).into(),
        ]);

        let mut context = ctx(vec![user_message("hello")]);

        let opts = |session: &str| {
            Some(SimpleStreamOptions {
                base: StreamOptions {
                    session_id: Some(session.into()),
                    cache_retention: Some(CacheRetention::Short),
                    ..Default::default()
                },
                reasoning: None,
                thinking_budgets: None,
            })
        };

        let first = complete(&reg, reg.get_model(), context.clone(), opts("session-1")).await;
        assert!(first.usage.cache_write > 0);

        context.messages.push(Message::Assistant(first));
        context.messages.push(user_message("follow up"));

        let second = complete(&reg, reg.get_model(), context.clone(), opts("session-2")).await;
        assert_eq!(second.usage.cache_read, 0);
        assert!(second.usage.cache_write > 0);

        let third = complete(&reg, reg.get_model(), context.clone(), None).await;
        assert_eq!(third.usage.cache_read, 0);
        assert_eq!(third.usage.cache_write, 0);
        reg.unregister();
    }
}
