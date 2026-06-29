//! Port of `packages/coding-agent/src/utils/mime.ts`.
//!
//! Sniff image MIME types from byte buffers (magic bytes).

const IMAGE_TYPE_SNIFF_BYTES: usize = 4100;

/// Detect supported image MIME types from a byte buffer by examining magic
/// bytes / signatures.
///
/// Supports JPEG, PNG (static only), GIF, and WebP.
/// Returns `None` if the buffer does not match a supported image type.
pub fn detect_supported_image_mime_type(buffer: &[u8]) -> Option<&'static str> {
    if starts_with(buffer, &[0xff, 0xd8, 0xff]) {
        // JPEG — check for JFIF/EXIF marker (byte 3 should be 0xe0-0xef)
        // 0xf7 is reserved/unknown; reject those.
        if buffer.get(3) == Some(&0xf7) {
            return None;
        }
        return Some("image/jpeg");
    }

    if starts_with(buffer, &[0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]) {
        if is_png(buffer) && !is_animated_png(buffer) {
            return Some("image/png");
        }
        return None;
    }

    if starts_with_ascii(buffer, 0, "GIF") {
        return Some("image/gif");
    }

    if starts_with_ascii(buffer, 0, "RIFF") && starts_with_ascii(buffer, 8, "WEBP") {
        return Some("image/webp");
    }

    None
}

/// Read the first 4100 bytes from a file and detect its image MIME type.
pub fn detect_supported_image_mime_type_from_file(
    file_path: &std::path::Path,
) -> Result<Option<String>, std::io::Error> {
    use std::io::Read;
    let mut file = std::fs::File::open(file_path)?;
    let mut buffer = vec![0u8; IMAGE_TYPE_SNIFF_BYTES];
    let bytes_read = file.read(&mut buffer)?;
    buffer.truncate(bytes_read);
    Ok(detect_supported_image_mime_type(&buffer).map(String::from))
}

fn is_png(buffer: &[u8]) -> bool {
    // PNG must have IHDR chunk as first chunk (length 13, type "IHDR")
    if buffer.len() < 16 {
        return false;
    }
    let chunk_length = read_u32_be(buffer, 8);
    chunk_length == 13 && starts_with_ascii(buffer, 12, "IHDR")
}

fn is_animated_png(buffer: &[u8]) -> bool {
    let mut offset = 8; // after PNG signature
    while offset + 8 <= buffer.len() {
        let chunk_length = read_u32_be(buffer, offset);
        let chunk_type_offset = offset + 4;
        if starts_with_ascii(buffer, chunk_type_offset, "acTL") {
            return true;
        }
        if starts_with_ascii(buffer, chunk_type_offset, "IDAT") {
            return false;
        }
        let next_offset = offset + 8 + chunk_length as usize;
        if next_offset <= offset || next_offset > buffer.len() {
            return false;
        }
        offset = next_offset;
    }
    false
}

fn read_u32_be(buffer: &[u8], offset: usize) -> u32 {
    if offset + 4 > buffer.len() {
        return 0;
    }
    (buffer[offset] as u32) << 24
        | (buffer[offset + 1] as u32) << 16
        | (buffer[offset + 2] as u32) << 8
        | (buffer[offset + 3] as u32)
}

fn starts_with(buffer: &[u8], needle: &[u8]) -> bool {
    buffer.len() >= needle.len() && buffer[..needle.len()] == *needle
}

fn starts_with_ascii(buffer: &[u8], offset: usize, text: &str) -> bool {
    let text_bytes = text.as_bytes();
    if buffer.len() < offset + text_bytes.len() {
        return false;
    }
    buffer[offset..offset + text_bytes.len()] == *text_bytes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jpeg() {
        // Minimal JPEG SOI + APP0 marker
        let buf = [0xff, 0xd8, 0xff, 0xe0];
        assert_eq!(detect_supported_image_mime_type(&buf), Some("image/jpeg"));
    }

    #[test]
    fn test_jpeg_reserved() {
        // 0xff marker with 0xf7 (reserved)
        let buf = [0xff, 0xd8, 0xff, 0xf7];
        assert_eq!(detect_supported_image_mime_type(&buf), None);
    }

    #[test]
    fn test_png() {
        // Minimal PNG with IHDR
        let mut buf = vec![0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a];
        // IHDR chunk: length=13, type=IHDR, then 13 bytes data, then CRC
        buf.extend_from_slice(&13u32.to_be_bytes()); // length
        buf.extend_from_slice(b"IHDR"); // type
        buf.extend_from_slice(&[0u8; 13]); // data (width=0, height=0, etc.)
        buf.extend_from_slice(&[0u8; 4]); // CRC (dummy)
        assert_eq!(detect_supported_image_mime_type(&buf), Some("image/png"));
    }

    #[test]
    fn test_gif() {
        let buf = b"GIF89a...";
        assert_eq!(detect_supported_image_mime_type(buf), Some("image/gif"));
    }

    #[test]
    fn test_webp() {
        // RIFF + WEBP header
        let mut buf = vec![];
        buf.extend_from_slice(b"RIFF");
        buf.extend_from_slice(&[0u8; 4]); // file size
        buf.extend_from_slice(b"WEBP");
        assert_eq!(detect_supported_image_mime_type(&buf), Some("image/webp"));
    }

    #[test]
    fn test_unknown() {
        let buf = b"not an image";
        assert_eq!(detect_supported_image_mime_type(buf), None);
    }

    #[test]
    fn test_empty() {
        assert_eq!(detect_supported_image_mime_type(&[]), None);
    }
}
