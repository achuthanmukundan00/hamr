//! Port of `../../packages/ai/src/stream.ts`.
//!
//! Top-level streaming entry points: [`stream`], [`complete`], [`stream_simple`],
//! and [`complete_simple`].
//!
//! These resolve a provider from the API registry, fill in an env-derived API key
//! when none was supplied, and dispatch to the provider backend.
//!
//! ## Divergence from TS
//!
//! The TS module does `import "./providers/register-builtins.ts"` for its
//! side-effect of populating the registry at module load. Rust has no module-load
//! side effect, so callers must invoke `register_builtins()` before the first
//! `stream`/`complete` call.
//!
//! `register_built_in_api_providers()` is called automatically at library load
//! time via `hamr-ai/src/lib.rs` and also explicitly in `sdk.rs`.

use crate::api_registry::{ApiProviderInternal, get_api_provider};
use crate::env_api_keys::get_env_api_key;
use crate::types::{
    Api, AssistantMessage, Context, Model, ProviderStreamOptions, SimpleStreamOptions,
    StreamOptions,
};
use crate::utils::event_stream::AssistantMessageEventStream;

/// True when an explicit, non-blank API key was supplied.
///
/// Mirrors the TS `hasExplicitApiKey`.
fn has_explicit_api_key(api_key: Option<&str>) -> bool {
    matches!(api_key, Some(key) if !key.trim().is_empty())
}

/// Fill in `api_key` from environment variables if the caller did not supply one.
///
/// Mirrors the TS `withEnvApiKey` for plain [`StreamOptions`].
fn with_env_api_key(model: &Model, options: Option<StreamOptions>) -> Option<StreamOptions> {
    if has_explicit_api_key(options.as_ref().and_then(|o| o.api_key.as_deref())) {
        return options;
    }
    let env = options.as_ref().and_then(|o| o.env.as_ref());
    let api_key = get_env_api_key(&model.provider, env);
    match api_key {
        None => options,
        Some(key) => {
            let mut opts = options.unwrap_or_default();
            opts.api_key = Some(key);
            Some(opts)
        }
    }
}

/// Fill in `api_key` from environment variables for [`SimpleStreamOptions`].
fn with_env_api_key_simple(
    model: &Model,
    options: Option<SimpleStreamOptions>,
) -> Option<SimpleStreamOptions> {
    if has_explicit_api_key(options.as_ref().and_then(|o| o.base.api_key.as_deref())) {
        return options;
    }
    let env = options.as_ref().and_then(|o| o.base.env.as_ref());
    let api_key = get_env_api_key(&model.provider, env);
    match api_key {
        None => options,
        Some(key) => {
            let mut opts = options.unwrap_or_default();
            opts.base.api_key = Some(key);
            Some(opts)
        }
    }
}

/// Resolve the registered provider for `api`, panicking-free with a descriptive error.
///
/// Mirrors the TS `resolveApiProvider`. Returns an error rather than throwing.
fn resolve_api_provider(api: Api) -> Result<ApiProviderInternal, StreamError> {
    get_api_provider(api).ok_or(StreamError::NoProvider(api))
}

/// Error raised when no provider is registered for a model's API.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StreamError {
    /// No API provider registered for the given api.
    NoProvider(Api),
}

impl std::fmt::Display for StreamError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StreamError::NoProvider(api) => {
                write!(f, "No API provider registered for api: {api}")
            }
        }
    }
}

impl std::error::Error for StreamError {}

/// Start a streaming request, returning the event stream.
///
/// Mirrors the TS `stream`. The TS signature can't fail synchronously (it throws);
/// here it returns a `Result` so callers handle a missing provider explicitly.
pub fn stream(
    model: Model,
    context: Context,
    options: Option<ProviderStreamOptions>,
) -> Result<AssistantMessageEventStream, StreamError> {
    let provider = resolve_api_provider(model.api)?;
    // ProviderStreamOptions is `StreamOptions & Record<string, unknown>`; the
    // registry's stream fn takes the base StreamOptions, so the extra bag is
    // dropped here (providers that need it should read it before this point).
    let base = options.map(|o| o.base);
    let base = with_env_api_key(&model, base);
    Ok((provider.stream)(model, context, base))
}

/// Run a streaming request to completion, returning the final message.
///
/// Mirrors the TS `complete`.
pub async fn complete(
    model: Model,
    context: Context,
    options: Option<ProviderStreamOptions>,
) -> Result<AssistantMessage, StreamError> {
    let s = stream(model, context, options)?;
    Ok(s.result().await)
}

/// Start a streaming request with reasoning options.
///
/// Mirrors the TS `streamSimple`.
pub fn stream_simple(
    model: Model,
    context: Context,
    options: Option<SimpleStreamOptions>,
) -> Result<AssistantMessageEventStream, StreamError> {
    let provider = resolve_api_provider(model.api)?;
    let options = with_env_api_key_simple(&model, options);
    Ok((provider.stream_simple)(model, context, options))
}

/// Run a reasoning request to completion, returning the final message.
///
/// Mirrors the TS `completeSimple`.
pub async fn complete_simple(
    model: Model,
    context: Context,
    options: Option<SimpleStreamOptions>,
) -> Result<AssistantMessage, StreamError> {
    let s = stream_simple(model, context, options)?;
    Ok(s.result().await)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api_registry::{ApiProvider, clear_api_providers, register_api_provider};
    use crate::types::{
        AssistantMessageEvent, DoneReason, MessageRole, ModelCost, StopReason, Usage, UsageCost,
    };
    use crate::utils::event_stream::create_assistant_message_event_stream;
    use chrono::Utc;
    use std::sync::{Arc, Mutex as StdMutex};

    static TEST_LOCK: StdMutex<()> = StdMutex::new(());

    fn lock() -> std::sync::MutexGuard<'static, ()> {
        TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn dummy_message() -> AssistantMessage {
        AssistantMessage {
            role: MessageRole::Assistant,
            content: Vec::new(),
            api: "anthropic-messages".to_string(),
            provider: "anthropic".to_string(),
            model: "done".to_string(),
            response_model: None,
            response_id: None,
            usage: Usage {
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
            },
            stop_reason: StopReason::Stop,
            error_message: None,
            diagnostics: None,
            timestamp: Utc::now(),
        }
    }

    fn dummy_model() -> Model {
        Model {
            id: "m".to_string(),
            name: "m".to_string(),
            api: Api::AnthropicMessages,
            provider: "anthropic".to_string(),
            base_url: "https://example.test".to_string(),
            reasoning: false,
            thinking_level_map: None,
            input: Vec::new(),
            cost: ModelCost {
                input: 0.0,
                output: 0.0,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 0,
            max_tokens: 0,
            headers: None,
            compat: None,
        }
    }

    fn dummy_context() -> Context {
        Context {
            system_prompt: None,
            messages: Vec::new(),
            tools: Vec::new(),
        }
    }

    fn register_dummy() {
        register_api_provider(
            ApiProvider {
                api: Api::AnthropicMessages,
                stream: Arc::new(|_m, _c, _o| {
                    let (mut tx, stream) = create_assistant_message_event_stream();
                    tx.push(AssistantMessageEvent::Done {
                        reason: DoneReason::Stop,
                        message: dummy_message(),
                    });
                    tx.end(None);
                    stream
                }),
                stream_simple: Arc::new(|_m, _c, _o| {
                    let (mut tx, stream) = create_assistant_message_event_stream();
                    tx.push(AssistantMessageEvent::Done {
                        reason: DoneReason::Stop,
                        message: dummy_message(),
                    });
                    tx.end(None);
                    stream
                }),
            },
            Some("stream-test".into()),
        );
    }

    #[tokio::test]
    async fn complete_resolves_final_message() {
        let _g = lock();
        clear_api_providers();
        register_dummy();
        let msg = complete(dummy_model(), dummy_context(), None)
            .await
            .expect("provider present");
        assert_eq!(msg.model, "done");
        clear_api_providers();
    }

    #[test]
    fn stream_errors_when_no_provider() {
        let _g = lock();
        clear_api_providers();
        match stream(dummy_model(), dummy_context(), None) {
            Err(e) => assert_eq!(e, StreamError::NoProvider(Api::AnthropicMessages)),
            Ok(_) => panic!("expected NoProvider error"),
        }
    }
}
