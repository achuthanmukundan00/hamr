//! Port of `../../packages/ai/src/api-registry.ts`.
//!
//! The API-to-provider dispatch registry. Each [`Api`] maps to a registered
//! provider exposing `stream` / `stream_simple` functions.
//!
//! ## TS → Rust shape
//!
//! The TS `StreamFunction` is `(model, context, options?) => AssistantMessageEventStream`.
//! In Rust, `Model` is non-generic (no `Model<TApi>` type parameter), so the
//! `wrapStream`/`wrapStreamSimple` generic machinery collapses to runtime `api`
//! checks against `model.api`. The registry is a global `Mutex<HashMap<Api, ...>>`.

use std::collections::HashMap;
use std::sync::{Arc, LazyLock, Mutex};

use crate::types::{Api, Context, Model, SimpleStreamOptions, StreamOptions};
use crate::utils::event_stream::AssistantMessageEventStream;

/// A function that turns a request into an [`AssistantMessageEventStream`].
///
/// Mirrors the TS `ApiStreamFunction`.
pub type ApiStreamFunction =
    Arc<dyn Fn(Model, Context, Option<StreamOptions>) -> AssistantMessageEventStream + Send + Sync>;

/// A function that turns a request (with reasoning) into an event stream.
///
/// Mirrors the TS `ApiStreamSimpleFunction`.
pub type ApiStreamSimpleFunction = Arc<
    dyn Fn(Model, Context, Option<SimpleStreamOptions>) -> AssistantMessageEventStream
        + Send
        + Sync,
>;

/// A registerable API provider.
///
/// Mirrors the TS `interface ApiProvider`.
#[derive(Clone)]
pub struct ApiProvider {
    pub api: Api,
    pub stream: ApiStreamFunction,
    pub stream_simple: ApiStreamSimpleFunction,
}

/// Internal provider record after wrapping (mirrors TS `ApiProviderInternal`).
#[derive(Clone)]
pub struct ApiProviderInternal {
    pub api: Api,
    pub stream: ApiStreamFunction,
    pub stream_simple: ApiStreamSimpleFunction,
}

/// Registry entry, tracking the registering source (mirrors TS `RegisteredApiProvider`).
#[derive(Clone)]
struct RegisteredApiProvider {
    provider: ApiProviderInternal,
    source_id: Option<String>,
}

static API_PROVIDER_REGISTRY: LazyLock<Mutex<HashMap<Api, RegisteredApiProvider>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

fn registry() -> std::sync::MutexGuard<'static, HashMap<Api, RegisteredApiProvider>> {
    API_PROVIDER_REGISTRY
        .lock()
        .unwrap_or_else(|e| e.into_inner())
}

/// Wrap a stream function to assert `model.api == api` before dispatch.
///
/// Mirrors the TS `wrapStream`.
fn wrap_stream(api: Api, stream: ApiStreamFunction) -> ApiStreamFunction {
    Arc::new(
        move |model: Model, context: Context, options: Option<StreamOptions>| {
            assert!(
                model.api == api,
                "Mismatched api: {} expected {}",
                model.api,
                api
            );
            stream(model, context, options)
        },
    )
}

/// Wrap a simple-stream function to assert `model.api == api` before dispatch.
///
/// Mirrors the TS `wrapStreamSimple`.
fn wrap_stream_simple(api: Api, stream_simple: ApiStreamSimpleFunction) -> ApiStreamSimpleFunction {
    Arc::new(
        move |model: Model, context: Context, options: Option<SimpleStreamOptions>| {
            assert!(
                model.api == api,
                "Mismatched api: {} expected {}",
                model.api,
                api
            );
            stream_simple(model, context, options)
        },
    )
}

/// Register (or replace) an API provider, optionally tagged with a `source_id`.
///
/// Mirrors the TS `registerApiProvider`.
pub fn register_api_provider(provider: ApiProvider, source_id: Option<String>) {
    let api = provider.api;
    let wrapped = RegisteredApiProvider {
        provider: ApiProviderInternal {
            api,
            stream: wrap_stream(api, provider.stream),
            stream_simple: wrap_stream_simple(api, provider.stream_simple),
        },
        source_id,
    };
    registry().insert(api, wrapped);
}

/// Look up the provider registered for `api`, if any.
///
/// Mirrors the TS `getApiProvider`.
pub fn get_api_provider(api: Api) -> Option<ApiProviderInternal> {
    registry().get(&api).map(|entry| entry.provider.clone())
}

/// All registered providers (mirrors TS `getApiProviders`).
pub fn get_api_providers() -> Vec<ApiProviderInternal> {
    registry()
        .values()
        .map(|entry| entry.provider.clone())
        .collect()
}

/// Remove every provider registered under `source_id` (mirrors TS `unregisterApiProviders`).
pub fn unregister_api_providers(source_id: &str) {
    registry().retain(|_, entry| entry.source_id.as_deref() != Some(source_id));
}

/// Clear the entire registry (mirrors TS `clearApiProviders`).
pub fn clear_api_providers() {
    registry().clear();
}

/// Returns true if the registry has no providers.
pub fn is_registry_empty() -> bool {
    registry().is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{AssistantMessageEvent, MessageRole, StopReason, Usage, UsageCost};
    use crate::utils::event_stream::create_assistant_message_event_stream;
    use chrono::Utc;
    use std::sync::Mutex as StdMutex;

    // The registry is global; serialize tests that mutate it.
    static TEST_LOCK: StdMutex<()> = StdMutex::new(());

    fn lock() -> std::sync::MutexGuard<'static, ()> {
        TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn dummy_message(api: Api) -> crate::types::AssistantMessage {
        crate::types::AssistantMessage {
            role: MessageRole::Assistant,
            content: Vec::new(),
            api: api.to_string(),
            provider: "test".to_string(),
            model: "m".to_string(),
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

    fn dummy_model(api: Api) -> Model {
        Model {
            id: "m".to_string(),
            name: "m".to_string(),
            api,
            provider: "test".to_string(),
            base_url: "https://example.test".to_string(),
            reasoning: false,
            thinking_level_map: None,
            input: Vec::new(),
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
        }
    }

    fn make_provider(api: Api) -> ApiProvider {
        ApiProvider {
            api,
            stream: Arc::new(move |_model, _ctx, _opts| {
                let (mut tx, stream) = create_assistant_message_event_stream();
                tx.push(AssistantMessageEvent::Done {
                    reason: crate::types::DoneReason::Stop,
                    message: dummy_message(api),
                });
                tx.end(None);
                stream
            }),
            stream_simple: Arc::new(move |_model, _ctx, _opts| {
                let (mut tx, stream) = create_assistant_message_event_stream();
                tx.push(AssistantMessageEvent::Done {
                    reason: crate::types::DoneReason::Stop,
                    message: dummy_message(api),
                });
                tx.end(None);
                stream
            }),
        }
    }

    #[tokio::test]
    async fn register_and_get() {
        let _g = lock();
        clear_api_providers();
        register_api_provider(make_provider(Api::AnthropicMessages), Some("src".into()));

        let provider = get_api_provider(Api::AnthropicMessages).expect("registered");
        let stream = (provider.stream)(dummy_model(Api::AnthropicMessages), test_context(), None);
        let result = stream.result().await;
        assert_eq!(result.api, "anthropic-messages");

        unregister_api_providers("src");
        assert!(get_api_provider(Api::AnthropicMessages).is_none());
    }

    #[test]
    fn unregister_only_matching_source() {
        let _g = lock();
        clear_api_providers();
        register_api_provider(make_provider(Api::AnthropicMessages), Some("a".into()));
        register_api_provider(make_provider(Api::OpenAiCompletions), Some("b".into()));

        unregister_api_providers("a");
        assert!(get_api_provider(Api::AnthropicMessages).is_none());
        assert!(get_api_provider(Api::OpenAiCompletions).is_some());
        clear_api_providers();
    }

    #[tokio::test]
    #[should_panic(expected = "Mismatched api")]
    async fn api_mismatch_panics() {
        let _g = lock();
        clear_api_providers();
        register_api_provider(make_provider(Api::AnthropicMessages), None);
        let provider = get_api_provider(Api::AnthropicMessages).expect("registered");
        // Pass a model with a different api → wrapper assertion fires.
        let _ = (provider.stream)(dummy_model(Api::OpenAiCompletions), test_context(), None);
    }

    fn test_context() -> Context {
        Context {
            system_prompt: None,
            messages: Vec::new(),
            tools: Vec::new(),
        }
    }
}
