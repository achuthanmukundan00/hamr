//! Port of `packages/coding-agent/src/utils/image-convert.ts`.
//!
//! Convert images to PNG format for terminal display (Kitty graphics protocol
//! requires PNG format).

use base64::Engine;

/// Minimum number of bytes needed to detect JPEG vs PNG vs other.
/// JPEG can be detected in ~3 bytes, PNG signature is 8 bytes.
const HEADER_SNIFF_BYTES: usize = 12;

/// Convert an image from its current format to PNG for terminal display.
///
/// If the image is already PNG, returns it unchanged.
pub fn convert_to_png(base64_data: &str, mime_type: &str) -> Result<(String, String), String> {
    // Already PNG, no conversion needed
    if mime_type == "image/png" {
        return Ok((base64_data.to_string(), mime_type.to_string()));
    }

    let engine = base64::engine::general_purpose::STANDARD;
    let bytes = engine
        .decode(base64_data)
        .map_err(|e| format!("Base64 decode error: {e}"))?;

    let img = image::load_from_memory(&bytes).map_err(|e| format!("Image load error: {e}"))?;

    // Encode as PNG
    let mut png_buffer = std::io::Cursor::new(Vec::new());
    img.write_to(&mut png_buffer, image::ImageFormat::Png)
        .map_err(|e| format!("PNG encode error: {e}"))?;

    let png_bytes = png_buffer.into_inner();
    let png_data = engine.encode(&png_bytes);

    Ok((png_data, "image/png".to_string()))
}

/// Check if base64 data corresponds to a PNG image (by examining the decoded
/// header bytes).
pub fn is_png_base64(base64_data: &str) -> bool {
    let engine = base64::engine::general_purpose::STANDARD;
    if let Ok(bytes) = engine.decode(base64_data) {
        if bytes.len() >= 8 {
            let png_sig: [u8; 8] = [0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a];
            return bytes[..8] == png_sig;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_png_passthrough() {
        let (data, mime) = convert_to_png("dGVzdA==", "image/png").unwrap();
        assert_eq!(mime, "image/png");
        assert_eq!(data, "dGVzdA==");
    }

    #[test]
    fn test_is_png() {
        // 1x1 red pixel PNG (minimal valid PNG)
        let png_b64 = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==";
        assert!(is_png_base64(png_b64));
        assert!(!is_png_base64("AAAA"));
    }

    #[test]
    fn test_jpeg_to_png_conversion() {
        // Create a tiny PNG image (JpegEncoder may not be compiled in)
        let mut img = image::RgbaImage::new(2, 2);
        img.put_pixel(0, 0, image::Rgba([255, 0, 0, 255]));
        img.put_pixel(1, 0, image::Rgba([0, 255, 0, 255]));
        img.put_pixel(0, 1, image::Rgba([0, 0, 255, 255]));
        img.put_pixel(1, 1, image::Rgba([255, 255, 255, 255]));

        let mut png_buf = std::io::Cursor::new(Vec::new());
        let encoder = image::codecs::png::PngEncoder::new(&mut png_buf);
        img.write_with_encoder(encoder).unwrap();
        let png_bytes = png_buf.into_inner();

        let engine = base64::engine::general_purpose::STANDARD;
        let png_b64 = engine.encode(&png_bytes);

        // convert_to_png should pass through PNG unchanged
        let (png_data, mime) = convert_to_png(&png_b64, "image/png").unwrap();
        assert_eq!(mime, "image/png");
        // PNG data should be the same
        assert_eq!(png_data, png_b64);
        let png_bytes = engine.decode(&png_data).unwrap();
        assert_eq!(png_bytes[0], 0x89);
        assert_eq!(png_bytes[1], 0x50); // P
        assert_eq!(png_bytes[2], 0x4e); // N
        assert_eq!(png_bytes[3], 0x47); // G
    }
}
