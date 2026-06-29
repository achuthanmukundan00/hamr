//! Port of `packages/ai/src/providers/register-builtins.ts`.
//!
//! Registers every implemented provider backend with the global
//! [`crate::api_registry`].  In Rust modules are always linked — the TS
//! lazy-import machinery is unnecessary.  A thin closure wraps each
//! provider-specific options type into the generic [`ApiStreamFunction`] /
//! [`ApiStreamSimpleFunction`] signatures.

use std::sync::Arc;

use crate::api_registry::{ApiProvider, register_api_provider};
use crate::types::{Api, Context, Model, SimpleStreamOptions, StreamOptions};

// ---------------------------------------------------------------------------
// Completed providers
// ---------------------------------------------------------------------------
use crate::providers::azure_openai_responses::{
    AzureOpenAIResponsesOptions, stream_azure_openai_responses,
    stream_simple_azure_openai_responses,
};
use crate::providers::google::{GoogleOptions, stream_google, stream_simple_google};
use crate::providers::mistral::{MistralOptions, stream_mistral, stream_simple_mistral};
use crate::providers::openai_completions::{
    OpenAiCompletionsOptions, stream_openai_completions, stream_simple_openai_completions,
};
use crate::providers::openai_responses::{
    OpenAIResponsesOptions, stream_openai_responses, stream_simple_openai_responses,
};

use crate::providers::amazon_bedrock::{BedrockOptions, stream_bedrock, stream_simple_bedrock};
use crate::providers::anthropic::{AnthropicOptions, stream_anthropic, stream_simple_anthropic};
use crate::providers::google_vertex::{
    GoogleVertexOptions, stream_google_vertex, stream_simple_google_vertex,
};
use crate::providers::openai_codex_responses::{
    CodexResponsesOptions, stream_openai_codex_responses, stream_simple_openai_codex_responses,
};
// ---------------------------------------------------------------------------
// Registration helpers
// ---------------------------------------------------------------------------

/// Register all providers that are fully ported.
///
/// Mirrors the TS `registerBuiltInApiProviders`.  Provider stubs
/// (`anthropic`, `google_vertex`, `openai_codex_responses`)
/// are **not** registered yet — they will be added when those files are
/// implemented, matching the dynamic-import escape hatch in the TS.
pub fn register_built_in_api_providers() {
    // -- Google Generative AI --------------------------------------------------
    register_api_provider(
        ApiProvider {
            api: Api::GoogleGenerativeAi,
            stream: Arc::new(
                move |model: Model, context: Context, options: Option<StreamOptions>| {
                    let opts = options.map(|base| GoogleOptions {
                        base,
                        ..Default::default()
                    });
                    stream_google(model, context, opts)
                },
            ) as crate::api_registry::ApiStreamFunction,
            stream_simple: Arc::new(
                move |model: Model, context: Context, options: Option<SimpleStreamOptions>| {
                    stream_simple_google(model, context, options)
                },
            ) as crate::api_registry::ApiStreamSimpleFunction,
        },
        None,
    );

    // -- Mistral ----------------------------------------------------------------
    register_api_provider(
        ApiProvider {
            api: Api::MistralConversations,
            stream: Arc::new(
                move |model: Model, context: Context, options: Option<StreamOptions>| {
                    let opts = options.map(|base| MistralOptions {
                        base,
                        ..Default::default()
                    });
                    stream_mistral(model, context, opts)
                },
            ) as crate::api_registry::ApiStreamFunction,
            stream_simple: Arc::new(
                move |model: Model, context: Context, options: Option<SimpleStreamOptions>| {
                    stream_simple_mistral(model, context, options)
                },
            ) as crate::api_registry::ApiStreamSimpleFunction,
        },
        None,
    );

    // -- OpenAI Completions -----------------------------------------------------
    register_api_provider(
        ApiProvider {
            api: Api::OpenAiCompletions,
            stream: Arc::new(
                move |model: Model, context: Context, options: Option<StreamOptions>| {
                    let opts = options.map(|base| OpenAiCompletionsOptions {
                        base,
                        ..Default::default()
                    });
                    stream_openai_completions(model, context, opts)
                },
            ) as crate::api_registry::ApiStreamFunction,
            stream_simple: Arc::new(
                move |model: Model, context: Context, options: Option<SimpleStreamOptions>| {
                    stream_simple_openai_completions(model, context, options)
                },
            ) as crate::api_registry::ApiStreamSimpleFunction,
        },
        None,
    );

    // -- OpenAI Responses -------------------------------------------------------
    register_api_provider(
        ApiProvider {
            api: Api::OpenAiResponses,
            stream: Arc::new(
                move |model: Model, context: Context, options: Option<StreamOptions>| {
                    let opts = options.map(|base| OpenAIResponsesOptions {
                        base,
                        ..Default::default()
                    });
                    stream_openai_responses(model, context, opts)
                },
            ) as crate::api_registry::ApiStreamFunction,
            stream_simple: Arc::new(
                move |model: Model, context: Context, options: Option<SimpleStreamOptions>| {
                    stream_simple_openai_responses(model, context, options)
                },
            ) as crate::api_registry::ApiStreamSimpleFunction,
        },
        None,
    );

    // -- Azure OpenAI Responses --------------------------------------------------
    register_api_provider(
        ApiProvider {
            api: Api::AzureOpenAiResponses,
            stream: Arc::new(
                move |model: Model, context: Context, options: Option<StreamOptions>| {
                    let opts = options.map(|base| AzureOpenAIResponsesOptions {
                        base,
                        ..Default::default()
                    });
                    stream_azure_openai_responses(model, context, opts)
                },
            ) as crate::api_registry::ApiStreamFunction,
            stream_simple: Arc::new(
                move |model: Model, context: Context, options: Option<SimpleStreamOptions>| {
                    stream_simple_azure_openai_responses(model, context, options)
                },
            ) as crate::api_registry::ApiStreamSimpleFunction,
        },
        None,
    );

    // -- AWS Bedrock Converse Stream -------------------------------------------
    register_api_provider(
        ApiProvider {
            api: Api::BedrockConverseStream,
            stream: Arc::new(
                move |model: Model, context: Context, options: Option<StreamOptions>| {
                    let opts = options.map(|base| BedrockOptions {
                        base,
                        ..Default::default()
                    });
                    stream_bedrock(model, context, opts)
                },
            ) as crate::api_registry::ApiStreamFunction,
            stream_simple: Arc::new(
                move |model: Model, context: Context, options: Option<SimpleStreamOptions>| {
                    stream_simple_bedrock(model, context, options)
                },
            ) as crate::api_registry::ApiStreamSimpleFunction,
        },
        None,
    );

    // -- Anthropic Messages ----------------------------------------------------
    register_api_provider(
        ApiProvider {
            api: Api::AnthropicMessages,
            stream: Arc::new(
                move |model: Model, context: Context, options: Option<StreamOptions>| {
                    let opts = options.map(|base| AnthropicOptions {
                        base,
                        ..Default::default()
                    });
                    stream_anthropic(model, context, opts)
                },
            ) as crate::api_registry::ApiStreamFunction,
            stream_simple: Arc::new(
                move |model: Model, context: Context, options: Option<SimpleStreamOptions>| {
                    stream_simple_anthropic(model, context, options)
                },
            ) as crate::api_registry::ApiStreamSimpleFunction,
        },
        None,
    );

    // -- Google Vertex AI ---------------------------------------------------------
    register_api_provider(
        ApiProvider {
            api: Api::GoogleVertex,
            stream: Arc::new(
                move |model: Model, context: Context, options: Option<StreamOptions>| {
                    let opts = options.map(|base| GoogleVertexOptions {
                        base,
                        ..Default::default()
                    });
                    stream_google_vertex(model, context, opts)
                },
            ) as crate::api_registry::ApiStreamFunction,
            stream_simple: Arc::new(
                move |model: Model, context: Context, options: Option<SimpleStreamOptions>| {
                    stream_simple_google_vertex(model, context, options)
                },
            ) as crate::api_registry::ApiStreamSimpleFunction,
        },
        None,
    );

    // -- OpenAI Codex Responses ---------------------------------------------------
    register_api_provider(
        ApiProvider {
            api: Api::OpenAiCodexResponses,
            stream: Arc::new(
                move |model: Model, context: Context, options: Option<StreamOptions>| {
                    let opts = options.map(|base| CodexResponsesOptions {
                        base,
                        ..Default::default()
                    });
                    stream_openai_codex_responses(model, context, opts)
                },
            ) as crate::api_registry::ApiStreamFunction,
            stream_simple: Arc::new(
                move |model: Model, context: Context, options: Option<SimpleStreamOptions>| {
                    stream_simple_openai_codex_responses(model, context, options)
                },
            ) as crate::api_registry::ApiStreamSimpleFunction,
        },
        None,
    );
}

/// Clear and re-register (mirrors the TS `resetApiProviders`).
pub fn reset_api_providers() {
    crate::api_registry::clear_api_providers();
    register_built_in_api_providers();
}
