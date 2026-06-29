//! Port of `packages/coding-agent/src/utils/image-resize.ts`.
//!
//! Async wrapper around `image_resize_core`, running the CPU-intensive
//! resize work on a blocking thread to avoid blocking the async runtime.

use crate::utils::image_resize_core;
pub use crate::utils::image_resize_core::{ImageResizeOptions, ResizedImage};

/// Resize an image asynchronously, running the CPU-bound work on a blocking
/// thread so it does not stall the async runtime.
///
/// In Rust, we don't need worker threads for WASM isolation the way the TS
/// version does — `spawn_blocking` suffices.
pub async fn resize_image(
    input_bytes: Vec<u8>,
    mime_type: String,
    options: Option<ImageResizeOptions>,
) -> Result<Option<ResizedImage>, String> {
    tokio::task::spawn_blocking(move || {
        image_resize_core::resize_image(&input_bytes, &mime_type, options.as_ref())
    })
    .await
    .map_err(|e| format!("Resize task join error: {e}"))?
}

/// Format a dimension note for resized images — helps the model understand
/// coordinate mapping.
pub fn format_dimension_note(result: &ResizedImage) -> Option<String> {
    image_resize_core::format_dimension_note(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_resize_image_async() {
        let mut img = image::RgbaImage::new(100, 100);
        for x in 0..100 {
            for y in 0..100 {
                img.put_pixel(x, y, image::Rgba([255, 0, 0, 255]));
            }
        }
        let mut buf = std::io::Cursor::new(Vec::new());
        img.write_to(&mut buf, image::ImageFormat::Png).unwrap();
        let bytes = buf.into_inner();

        let result = resize_image(bytes, "image/png".to_string(), None)
            .await
            .unwrap()
            .unwrap();
        assert!(!result.was_resized);
        assert_eq!(result.width, 100);
    }
}
