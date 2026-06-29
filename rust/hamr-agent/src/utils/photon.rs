//! Port of `packages/coding-agent/src/utils/photon.ts`
//!
//! Photon image processing wrapper.
//!
//! Provides a unified interface to the image processing library that works both
//! in development (Node.js) and compiled binaries (Bun).
//!
//! In Rust, we don't use photon-node. Instead, we use the `image` crate for
//! image manipulation, which provides equivalent functionality natively.

use std::io::Cursor;

/// Re-export for type compatibility — in TS this is `PhotonImage` from `@silvia-odwyer/photon-node`.
/// We use the `image` crate's `DynamicImage` as the equivalent.
pub use image::DynamicImage as PhotonImage;

/// Load image bytes into a PhotonImage.
///
/// Equivalent to `photon.PhotonImage.new_from_byteslice(bytes)` in the TS version.
pub fn photon_new_from_bytes(bytes: &[u8]) -> Result<PhotonImage, String> {
    image::load_from_memory(bytes).map_err(|e| format!("Failed to decode image: {}", e))
}

/// Get PNG bytes from a PhotonImage.
///
/// Equivalent to `photon.PhotonImage.get_bytes()` (or `.get_bytes_jpeg()` etc)
/// in the TS version.
pub fn photon_to_png_bytes(image: &PhotonImage) -> Vec<u8> {
    let mut buf = Cursor::new(Vec::new());
    image
        .write_to(&mut buf, image::ImageFormat::Png)
        .expect("PNG encoding must succeed in memory");
    buf.into_inner()
}

/// Get JPEG bytes from a PhotonImage.
pub fn photon_to_jpeg_bytes(image: &PhotonImage) -> Vec<u8> {
    let mut buf = Cursor::new(Vec::new());
    image
        .write_to(&mut buf, image::ImageFormat::Jpeg)
        .expect("JPEG encoding must succeed in memory");
    buf.into_inner()
}

/// Load the photon module asynchronously.
///
/// In Rust this is synchronous since we use the `image` crate natively.
/// Matches the async `loadPhoton()` signature for API compatibility.
pub async fn load_photon() -> Option<()> {
    // In Rust, we always have image processing available via the `image` crate.
    // The TS version patches fs.readFileSync to redirect WASM reads — not needed here.
    Some(())
}

/// Get raw pixels from a PhotonImage.
pub fn photon_get_raw_pixels(image: &PhotonImage) -> Vec<u8> {
    image.to_rgba8().into_raw()
}

/// Create a new PhotonImage from raw pixels.
pub fn photon_new_from_raw_pixels(
    pixels: &[u8],
    width: u32,
    height: u32,
) -> Result<PhotonImage, String> {
    image::RgbaImage::from_raw(width, height, pixels.to_vec())
        .map(PhotonImage::ImageRgba8)
        .ok_or_else(|| "Invalid dimensions for pixel data".to_string())
}

/// Get width of a PhotonImage.
pub fn photon_get_width(image: &PhotonImage) -> u32 {
    image.width()
}

/// Get height of a PhotonImage.
pub fn photon_get_height(image: &PhotonImage) -> u32 {
    image.height()
}

/// Flip image horizontally in-place.
pub fn photon_fliph(image: &mut PhotonImage) {
    *image = image.fliph();
}

/// Flip image vertically in-place.
pub fn photon_flipv(image: &mut PhotonImage) {
    *image = image.flipv();
}
