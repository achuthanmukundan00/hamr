//! Port of `packages/ai/src/images.ts`.
//!
//! Top-level image generation dispatch. Resolves the registered provider for an
//! [`ImagesApi`] and delegates to it.
//!
//! ## Types defined locally (not yet in `crate::types`)
//!
//! See the `local_types` sub-module below. These mirror the TS types in
//! `packages/ai/src/types.ts` that are not (yet) part of `crate::types`.
//!
//! TODO(images-types): move to types.rs

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Local type definitions — mirror TS types not yet in crate::types.
// TODO(images-types): move to types.rs
// ---------------------------------------------------------------------------

pub mod local_types {
    //! Images-specific types that belong in `crate::types` but are defined here
    //! to avoid touching `types.rs` in a parallel build.

    use super::*;

    /// Known image-generation API identifiers.
    pub const KNOWN_IMAGES_API_OPENROUTER: &str = "openrouter-images";

    /// A string-typed API identifier for image generation.
    pub type ImagesApi = String;

    /// Known image-generation provider identifiers.
    pub const KNOWN_IMAGES_PROVIDER_OPENROUTER: &str = "openrouter";

    /// A string-typed provider identifier for image generation.
    pub type ImagesProvider = String;

    /// An image generation model descriptor.
    ///
    /// Mirrors the TS `ImagesModel<TApi>` (which extends `Model` minus some fields).
    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct ImagesModel {
        pub id: String,
        pub name: String,
        pub api: ImagesApi,
        pub provider: ImagesProvider,
        pub base_url: String,
        /// Supported output modalities ("text", "image").
        #[serde(default)]
        pub output: Vec<String>,
        pub cost: ImagesModelCost,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub headers: Option<HashMap<String, String>>,
    }

    /// Per-million-token costs for an image generation model.
    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct ImagesModelCost {
        pub input: f64,
        pub output: f64,
        pub cache_read: f64,
        pub cache_write: f64,
    }

    /// Input content block for image generation.
    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(tag = "type")]
    pub enum ImagesInputContent {
        #[serde(rename = "text")]
        Text(ImagesInputTextContent),
        #[serde(rename = "image")]
        Image(ImagesInputImageContent),
    }

    /// Text content block in image generation input.
    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct ImagesInputTextContent {
        pub text: String,
    }

    /// Image content block in image generation input (base64).
    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct ImagesInputImageContent {
        pub data: String,
        pub mime_type: String,
    }

    /// Output content block from image generation.
    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(tag = "type")]
    pub enum ImagesOutputContent {
        #[serde(rename = "text")]
        Text(ImagesOutputTextContent),
        #[serde(rename = "image")]
        Image(ImagesOutputImageContent),
    }

    /// Text content block in image generation output.
    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct ImagesOutputTextContent {
        pub text: String,
    }

    /// Image content block in image generation output (base64 + mime).
    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct ImagesOutputImageContent {
        pub data: String,
        pub mime_type: String,
    }

    /// Context for an image generation request.
    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct ImagesContext {
        pub input: Vec<ImagesInputContent>,
    }

    /// Why image generation stopped.
    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub enum ImagesStopReason {
        #[serde(rename = "stop")]
        Stop,
        #[serde(rename = "error")]
        Error,
        #[serde(rename = "aborted")]
        Aborted,
    }

    /// Token usage for an image generation response.
    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct ImagesUsage {
        pub input: u64,
        pub output: u64,
        pub cache_read: u64,
        pub cache_write: u64,
        pub total_tokens: u64,
        pub cost: ImagesUsageCost,
    }

    /// Cost breakdown for an image generation response.
    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct ImagesUsageCost {
        pub input: f64,
        pub output: f64,
        pub cache_read: f64,
        pub cache_write: f64,
        pub total: f64,
    }

    /// The result of an image generation call.
    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct AssistantImages {
        pub api: ImagesApi,
        pub provider: ImagesProvider,
        pub model: String,
        #[serde(default)]
        pub output: Vec<ImagesOutputContent>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub response_id: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub usage: Option<ImagesUsage>,
        pub stop_reason: ImagesStopReason,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub error_message: Option<String>,
        /// Unix timestamp in milliseconds.
        pub timestamp: i64,
    }

    /// Options for an image generation request.
    #[derive(Clone)]
    pub struct ImagesOptions {
        /// Abort signal receiver.
        pub signal: Option<tokio::sync::watch::Receiver<bool>>,
        pub api_key: Option<String>,
        /// Inspect/replace provider payloads before sending. Return `None` to keep unchanged.
        pub on_payload: Option<
            Arc<
                dyn Fn(
                        serde_json::Value,
                        ImagesModel,
                    )
                        -> Pin<Box<dyn Future<Output = Option<serde_json::Value>> + Send>>
                    + Send
                    + Sync,
            >,
        >,
        /// Invoked after an HTTP response is received, before consuming the body.
        pub on_response: Option<
            Arc<
                dyn Fn(
                        crate::types::ProviderResponse,
                        ImagesModel,
                    ) -> Pin<Box<dyn Future<Output = ()> + Send>>
                    + Send
                    + Sync,
            >,
        >,
        pub headers: Option<HashMap<String, String>>,
        pub timeout_ms: Option<u64>,
        pub max_retries: Option<u32>,
        /// Provider-scoped environment overrides.
        pub env: Option<crate::types::ProviderEnv>,
    }

    impl std::fmt::Debug for ImagesOptions {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("ImagesOptions")
                .field("api_key", &self.api_key.as_ref().map(|_| "<redacted>"))
                .field("signal", &self.signal.as_ref().map(|_| "<signal>"))
                .field("on_payload", &self.on_payload.as_ref().map(|_| "<fn>"))
                .field("on_response", &self.on_response.as_ref().map(|_| "<fn>"))
                .field("headers", &self.headers)
                .field("timeout_ms", &self.timeout_ms)
                .field("max_retries", &self.max_retries)
                .field("env", &self.env)
                .finish()
        }
    }

    /// The function signature for an image generation provider.
    ///
    /// Mirrors the TS `ImagesFunction`.
    pub type ImagesFunction = Arc<
        dyn Fn(
                ImagesModel,
                ImagesContext,
                Option<ImagesOptions>,
            ) -> Pin<Box<dyn Future<Output = Result<AssistantImages, String>> + Send>>
            + Send
            + Sync,
    >;
}

pub use local_types::*;

use std::sync::{Arc, LazyLock, Mutex};

// ---------------------------------------------------------------------------
// Images API Registry
// ---------------------------------------------------------------------------

/// Registered provider entry for an image generation API.
#[derive(Clone)]
pub struct ImagesApiProviderInternal {
    pub api: ImagesApi,
    pub generate_images: ImagesFunction,
}

#[derive(Clone)]
struct RegisteredImagesApiProvider {
    provider: ImagesApiProviderInternal,
    #[allow(dead_code)]
    source_id: Option<String>,
}

static IMAGES_API_PROVIDER_REGISTRY: LazyLock<Mutex<HashMap<String, RegisteredImagesApiProvider>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

fn images_registry() -> std::sync::MutexGuard<'static, HashMap<String, RegisteredImagesApiProvider>>
{
    IMAGES_API_PROVIDER_REGISTRY
        .lock()
        .unwrap_or_else(|e| e.into_inner())
}

/// Wrap a `generate_images` function to assert `model.api == api` before dispatch.
fn wrap_generate_images(api: &str, generate_images: ImagesFunction) -> ImagesFunction {
    let api_owned = api.to_string();
    Arc::new(
        move |model: ImagesModel, context: ImagesContext, options: Option<ImagesOptions>| {
            let api_check = api_owned.clone();
            if model.api != api_check {
                return Box::pin(async move {
                    Err(format!(
                        "Mismatched api: {} expected {}",
                        model.api, api_check
                    ))
                });
            }
            generate_images(model, context, options)
        },
    )
}

/// Register (or replace) an image-generation API provider.
///
/// Mirrors the TS `registerImagesApiProvider`.
pub fn register_images_api_provider(
    api: &str,
    generate_images: ImagesFunction,
    source_id: Option<String>,
) {
    let wrapped = RegisteredImagesApiProvider {
        provider: ImagesApiProviderInternal {
            api: api.to_string(),
            generate_images: wrap_generate_images(api, generate_images),
        },
        source_id,
    };
    images_registry().insert(api.to_string(), wrapped);
}

/// Look up the provider registered for `api`, if any.
///
/// Mirrors the TS `getImagesApiProvider`.
pub fn get_images_api_provider(api: &str) -> Option<ImagesApiProviderInternal> {
    images_registry()
        .get(api)
        .map(|entry| entry.provider.clone())
}

/// Clear the entire image provider registry.
pub fn clear_images_api_providers() {
    images_registry().clear();
}

// ---------------------------------------------------------------------------
// Top-level dispatch
// ---------------------------------------------------------------------------

/// Generate images by dispatching to the provider registered for the model's API.
///
/// Mirrors the TS `generateImages` export.
pub async fn generate_images(
    model: ImagesModel,
    context: ImagesContext,
    options: Option<ImagesOptions>,
) -> Result<AssistantImages, String> {
    let provider = get_images_api_provider(&model.api)
        .ok_or_else(|| format!("No API provider registered for api: {}", model.api))?;
    (provider.generate_images)(model, context, options).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use std::sync::Mutex as StdMutex;

    static IMAGES_TEST_LOCK: StdMutex<()> = StdMutex::new(());

    #[tokio::test]
    async fn dispatch_to_registered_provider() {
        let _g = IMAGES_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        clear_images_api_providers();
        let api = "test-api".to_string();
        let fn_impl: ImagesFunction = Arc::new(
            |model: ImagesModel, _ctx: ImagesContext, _opts: Option<ImagesOptions>| {
                Box::pin(async move {
                    Ok(AssistantImages {
                        api: model.api,
                        provider: "test-provider".to_string(),
                        model: model.id,
                        output: vec![],
                        response_id: None,
                        usage: None,
                        stop_reason: ImagesStopReason::Stop,
                        error_message: None,
                        timestamp: Utc::now().timestamp_millis(),
                    })
                })
            },
        );

        register_images_api_provider(&api, fn_impl, None);

        let model = ImagesModel {
            id: "m".to_string(),
            name: "m".to_string(),
            api: api.clone(),
            provider: "test-provider".to_string(),
            base_url: "https://test".to_string(),
            output: vec!["image".to_string()],
            cost: ImagesModelCost {
                input: 0.0,
                output: 0.0,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            headers: None,
        };
        let ctx = ImagesContext { input: vec![] };

        let result = generate_images(model, ctx, None).await;
        assert!(result.is_ok());
        let images = result.unwrap();
        assert_eq!(images.api, "test-api");
        assert_eq!(images.model, "m");

        clear_images_api_providers();
    }

    #[tokio::test]
    async fn no_provider_returns_error() {
        let _g = IMAGES_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        clear_images_api_providers();
        let api = "nonexistent".to_string();

        let model = ImagesModel {
            id: "m".to_string(),
            name: "m".to_string(),
            api,
            provider: "none".to_string(),
            base_url: "https://test".to_string(),
            output: vec!["image".to_string()],
            cost: ImagesModelCost {
                input: 0.0,
                output: 0.0,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            headers: None,
        };
        let ctx = ImagesContext { input: vec![] };

        let result = generate_images(model, ctx, None).await;
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .contains("No API provider registered for api: nonexistent")
        );
    }

    #[tokio::test]
    async fn api_mismatch_returns_error() {
        let _g = IMAGES_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        clear_images_api_providers();
        let api_a = "api-a".to_string();
        let fn_impl: ImagesFunction = Arc::new(
            |_model: ImagesModel, _ctx: ImagesContext, _opts: Option<ImagesOptions>| {
                Box::pin(async move {
                    Ok(AssistantImages {
                        api: "api-a".to_string(),
                        provider: "test".to_string(),
                        model: "m".to_string(),
                        output: vec![],
                        response_id: None,
                        usage: None,
                        stop_reason: ImagesStopReason::Stop,
                        error_message: None,
                        timestamp: 0,
                    })
                })
            },
        );

        register_images_api_provider(&api_a, fn_impl, None);

        // Model with a different API should fail the wrap check.
        let model = ImagesModel {
            id: "m".to_string(),
            name: "m".to_string(),
            api: "api-b".to_string(),
            provider: "test".to_string(),
            base_url: "https://test".to_string(),
            output: vec!["image".to_string()],
            cost: ImagesModelCost {
                input: 0.0,
                output: 0.0,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            headers: None,
        };
        let ctx = ImagesContext { input: vec![] };
        let provider = get_images_api_provider("api-a").unwrap();
        let result = (provider.generate_images)(model, ctx, None).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Mismatched api"));

        clear_images_api_providers();
    }
}
