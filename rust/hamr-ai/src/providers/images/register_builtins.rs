//! Port of `packages/ai/src/providers/images/register-builtins.ts`.
//!
//! Registers the built-in OpenRouter image generation provider.
//!
//! Unlike the TS version (which uses dynamic `import()` for lazy loading), the
//! Rust version imports [`crate::providers::images::openrouter`] directly —
//! there is no cost to static linking, so the lazy-wrap is a simple delegate.

use crate::images::local_types::{AssistantImages, ImagesContext, ImagesModel, ImagesOptions};
use crate::images::register_images_api_provider;
use crate::providers::images::openrouter;
use std::sync::Arc;

/// A `generate_images` function that delegates to [`openrouter::generate_images_open_router`].
///
/// Mirrors the TS `generateImagesOpenRouter` in `register-builtins.ts` which
/// dynamically imports the openrouter module and calls it.
pub async fn generate_images_open_router(
    model: ImagesModel,
    context: ImagesContext,
    options: Option<ImagesOptions>,
) -> Result<AssistantImages, String> {
    openrouter::generate_images_open_router(model, context, options).await
}

/// Register every built-in image-generation API provider.
///
/// Mirrors the TS `registerBuiltInImagesApiProviders()`.
pub fn register_built_in_images_api_providers() {
    register_images_api_provider(
        crate::images::local_types::KNOWN_IMAGES_API_OPENROUTER,
        Arc::new(
            |model: ImagesModel, context: ImagesContext, options: Option<ImagesOptions>| {
                Box::pin(async move { generate_images_open_router(model, context, options).await })
            },
        ),
        Some("builtin".to_string()),
    );
}
