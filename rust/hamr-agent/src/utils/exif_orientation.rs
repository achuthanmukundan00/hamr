//! Port of `packages/coding-agent/src/utils/exif-orientation.ts`
//!
//! EXIF orientation parsing and application.
//! Parses raw byte-level TIFF EXIF data from JPEG and WebP images,
//! then applies the orientation transform to the image.

use crate::utils::photon::PhotonImage;

/// Read the TIFF IFD orientation tag.
/// Returns orientation value 1-8, or 1 if not found / invalid.
fn read_orientation_from_tiff(bytes: &[u8], tiff_start: usize) -> u16 {
    if tiff_start + 8 > bytes.len() {
        return 1;
    }

    let byte_order = ((bytes[tiff_start] as u16) << 8) | bytes[tiff_start + 1] as u16;
    let le = byte_order == 0x4949; // "II" = little-endian

    let read16 = |pos: usize| -> u16 {
        if le {
            bytes[pos] as u16 | ((bytes[pos + 1] as u16) << 8)
        } else {
            ((bytes[pos] as u16) << 8) | bytes[pos + 1] as u16
        }
    };

    let read32 = |pos: usize| -> u32 {
        if le {
            bytes[pos] as u32
                | ((bytes[pos + 1] as u32) << 8)
                | ((bytes[pos + 2] as u32) << 16)
                | ((bytes[pos + 3] as u32) << 24)
        } else {
            ((bytes[pos] as u32) << 24)
                | ((bytes[pos + 1] as u32) << 16)
                | ((bytes[pos + 2] as u32) << 8)
                | bytes[pos + 3] as u32
        }
    };

    let ifd_offset = read32(tiff_start + 4);
    let ifd_start = tiff_start + ifd_offset as usize;
    if ifd_start + 2 > bytes.len() {
        return 1;
    }

    let entry_count = read16(ifd_start);
    for i in 0..entry_count {
        let entry_pos = ifd_start + 2 + i as usize * 12;
        if entry_pos + 12 > bytes.len() {
            return 1;
        }

        if read16(entry_pos) == 0x0112 {
            let value = read16(entry_pos + 8);
            return if (1..=8).contains(&value) { value } else { 1 };
        }
    }

    1
}

/// Find the TIFF offset in a JPEG file.
fn find_jpeg_tiff_offset(bytes: &[u8]) -> isize {
    let mut offset = 2usize;
    while offset < bytes.len().saturating_sub(1) {
        if bytes[offset] != 0xff {
            return -1;
        }
        let marker = bytes[offset + 1];
        if marker == 0xff {
            offset += 1;
            continue;
        }

        if marker == 0xe1 {
            if offset + 4 >= bytes.len() {
                return -1;
            }
            let segment_start = offset + 4;
            if segment_start + 6 > bytes.len() {
                return -1;
            }
            if !has_exif_header(bytes, segment_start) {
                return -1;
            }
            return (segment_start + 6) as isize;
        }

        if offset + 4 > bytes.len() {
            return -1;
        }
        let length = ((bytes[offset + 2] as usize) << 8) | bytes[offset + 3] as usize;
        offset += 2 + length;
    }

    -1
}

/// Find the TIFF offset in a WebP file.
fn find_webp_tiff_offset(bytes: &[u8]) -> isize {
    let mut offset = 12usize;
    while offset + 8 <= bytes.len() {
        let chunk_id = std::str::from_utf8(&bytes[offset..offset + 4]).unwrap_or("");
        let chunk_size = bytes[offset + 4] as u32
            | ((bytes[offset + 5] as u32) << 8)
            | ((bytes[offset + 6] as u32) << 16)
            | ((bytes[offset + 7] as u32) << 24);
        let data_start = offset + 8;

        if chunk_id == "EXIF" {
            if data_start + chunk_size as usize > bytes.len() {
                return -1;
            }
            // Some WebP files have "Exif\0\0" prefix before the TIFF header
            let tiff_start = if chunk_size >= 6 && has_exif_header(bytes, data_start) {
                data_start + 6
            } else {
                data_start
            };
            return tiff_start as isize;
        }

        // RIFF chunks are padded to even size
        offset = data_start + chunk_size as usize + (chunk_size as usize % 2);
    }

    -1
}

fn has_exif_header(bytes: &[u8], offset: usize) -> bool {
    bytes.len() >= offset + 6
        && bytes[offset] == 0x45
        && bytes[offset + 1] == 0x78
        && bytes[offset + 2] == 0x69
        && bytes[offset + 3] == 0x66
        && bytes[offset + 4] == 0x00
        && bytes[offset + 5] == 0x00
}

/// Get EXIF orientation from raw image bytes.
/// Returns orientation value 1-8, or 1 if not found / not a supported format.
fn get_exif_orientation(bytes: &[u8]) -> u16 {
    let tiff_offset: isize;

    // JPEG: starts with FF D8
    if bytes.len() >= 2 && bytes[0] == 0xff && bytes[1] == 0xd8 {
        tiff_offset = find_jpeg_tiff_offset(bytes);
    }
    // WebP: starts with RIFF....WEBP
    else if bytes.len() >= 12
        && bytes[0] == 0x52 // R
        && bytes[1] == 0x49 // I
        && bytes[2] == 0x46 // F
        && bytes[3] == 0x46 // F
        && bytes[8] == 0x57  // W
        && bytes[9] == 0x45  // E
        && bytes[10] == 0x42 // B
        && bytes[11] == 0x50
    // P
    {
        tiff_offset = find_webp_tiff_offset(bytes);
    } else {
        tiff_offset = -1;
    }

    if tiff_offset == -1 {
        return 1;
    }
    read_orientation_from_tiff(bytes, tiff_offset as usize)
}

/// Rotate image 90 degrees, mapping each source pixel to a destination pixel
/// via the provided index function.
fn rotate90(
    image: &PhotonImage,
    dst_index: fn(x: u32, y: u32, w: u32, h: u32) -> u32,
) -> PhotonImage {
    use crate::utils::photon;
    let w = photon::photon_get_width(image);
    let h = photon::photon_get_height(image);
    let src = photon::photon_get_raw_pixels(image);
    // 90-degree rotation swaps width and height
    let (dw, dh) = (h, w);
    let mut dst = vec![0u8; (dw * dh * 4) as usize];

    for y in 0..h {
        for x in 0..w {
            let src_idx = ((y * w + x) * 4) as usize;
            let dst_idx = (dst_index(x, y, w, h) * 4) as usize;
            dst[dst_idx] = src[src_idx];
            dst[dst_idx + 1] = src[src_idx + 1];
            dst[dst_idx + 2] = src[src_idx + 2];
            dst[dst_idx + 3] = src[src_idx + 3];
        }
    }

    image::RgbaImage::from_raw(dw, dh, dst)
        .map(PhotonImage::ImageRgba8)
        .expect("rotate90 dimensions must be valid")
}

/// Apply EXIF orientation to an image.
///
/// Returns the (possibly modified) image. For in-place flips, the input image is mutated.
/// For rotations, a new image is returned (the caller should discard the old one).
pub fn apply_exif_orientation(image: &mut PhotonImage, original_bytes: &[u8]) -> PhotonImage {
    use crate::utils::photon;

    let orientation = get_exif_orientation(original_bytes);
    if orientation == 1 {
        return image.clone();
    }

    match orientation {
        2 => {
            photon::photon_fliph(image);
            image.clone()
        }
        3 => {
            photon::photon_fliph(image);
            photon::photon_flipv(image);
            image.clone()
        }
        4 => {
            photon::photon_flipv(image);
            image.clone()
        }
        5 => {
            let mut rotated = rotate90(image, |x, y, _w, h| x * h + (h - 1 - y));
            photon::photon_fliph(&mut rotated);
            rotated
        }
        6 => rotate90(image, |x, y, _w, h| x * h + (h - 1 - y)),
        7 => {
            let mut rotated = rotate90(image, |x, y, w, h| (w - 1 - x) * h + y);
            photon::photon_fliph(&mut rotated);
            rotated
        }
        8 => rotate90(image, |x, y, w, h| (w - 1 - x) * h + y),
        _ => image.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ImageBuffer, RgbImage, RgbaImage};

    fn make_test_rgba_image() -> PhotonImage {
        // Create a 3x2 RGBA image
        let img: RgbaImage = ImageBuffer::from_fn(3, 2, |x, y| {
            image::Rgba([(x * 80 + y * 40) as u8, 100, 150, 255])
        });
        PhotonImage::ImageRgba8(img)
    }

    #[test]
    fn test_exif_orientation_1_is_noop() {
        let mut img = make_test_rgba_image();
        let original_bytes = vec![0u8; 100]; // no EXIF data → orientation 1
        let result = apply_exif_orientation(&mut img, &original_bytes);
        assert_eq!(
            image::imageops::resize(&img.to_rgba8(), 3, 2, image::imageops::FilterType::Nearest,)
                .as_raw(),
            result.to_rgba8().as_raw()
        );
    }

    #[test]
    fn test_orientation_6_rotates_90_cw() {
        let mut img = make_test_rgba_image();
        // Build a minimal JPEG with EXIF orientation 6
        let bytes = build_minimal_exif(6);
        let result = apply_exif_orientation(&mut img, &bytes);
        // After 90° CW, 3x2 becomes 2x3
        assert_eq!(result.width(), 2);
        assert_eq!(result.height(), 3);
    }

    /// Build a minimal valid JPEG with EXIF tag set to the given orientation.
    fn build_minimal_exif(orientation: u16) -> Vec<u8> {
        // EXIF data in TIFF format: little-endian, 1 IFD entry, orientation tag
        let mut tiff = Vec::new();
        // TIFF header: "II" (little-endian), 42, offset to IFD
        tiff.extend_from_slice(&[0x49, 0x49, 0x2a, 0x00, 0x08, 0x00, 0x00, 0x00]);
        // IFD: 1 entry
        tiff.extend_from_slice(&[0x01, 0x00]); // 1 entry
        // Entry: tag=0x0112 (orientation), type=3 (SHORT), count=1, value=orientation
        tiff.extend_from_slice(&[0x12, 0x01, 0x03, 0x00, 0x01, 0x00, 0x00, 0x00]);
        tiff.push(orientation as u8);
        tiff.push(0x00);
        // Next IFD offset = 0 (end)
        tiff.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]);

        // JPEG APP1 marker
        let mut jpeg = Vec::new();
        jpeg.extend_from_slice(&[0xff, 0xd8]); // SOI
        jpeg.push(0xff);
        jpeg.push(0xe1); // APP1
        let length = (2 + 6 + tiff.len()) as u16; // "Exif\0\0" + TIFF
        jpeg.extend_from_slice(&length.to_be_bytes());
        jpeg.extend_from_slice(b"Exif\0\0");
        jpeg.extend_from_slice(&tiff);
        // SOS marker (minimal)
        jpeg.extend_from_slice(&[0xff, 0xda, 0x00, 0x08, 0x01, 0x01, 0x00, 0x00, 0x3f, 0x00]);
        jpeg
    }

    #[test]
    fn test_read_orientation_empty_bytes() {
        assert_eq!(get_exif_orientation(&[]), 1);
        assert_eq!(get_exif_orientation(&[0xff, 0xd8]), 1);
    }

    #[test]
    fn test_read_orientation_from_valid_exif() {
        let bytes = build_minimal_exif(8);
        assert_eq!(get_exif_orientation(&bytes), 8);
    }

    #[test]
    fn test_orientation_2_flip_horizontal() {
        let mut img = make_test_rgba_image();
        let bytes = build_minimal_exif(2);
        let result = apply_exif_orientation(&mut img, &bytes);
        // Horizontal flip should keep dimensions
        assert_eq!(result.width(), 3);
        assert_eq!(result.height(), 2);
    }

    #[test]
    fn test_orientation_3_rotate_180() {
        let mut img = make_test_rgba_image();
        let bytes = build_minimal_exif(3);
        let result = apply_exif_orientation(&mut img, &bytes);
        assert_eq!(result.width(), 3);
        assert_eq!(result.height(), 2);
    }

    #[test]
    fn test_orientation_4_flip_vertical() {
        let mut img = make_test_rgba_image();
        let bytes = build_minimal_exif(4);
        let result = apply_exif_orientation(&mut img, &bytes);
        assert_eq!(result.width(), 3);
        assert_eq!(result.height(), 2);
    }

    #[test]
    fn test_orientation_5_rotate_90_cw_and_flip() {
        let mut img = make_test_rgba_image();
        let bytes = build_minimal_exif(5);
        let result = apply_exif_orientation(&mut img, &bytes);
        // 90° rotation swaps dimensions
        assert_eq!(result.width(), 2);
        assert_eq!(result.height(), 3);
    }

    #[test]
    fn test_orientation_7_rotate_90_ccw_and_flip() {
        let mut img = make_test_rgba_image();
        let bytes = build_minimal_exif(7);
        let result = apply_exif_orientation(&mut img, &bytes);
        assert_eq!(result.width(), 2);
        assert_eq!(result.height(), 3);
    }

    #[test]
    fn test_orientation_8_rotate_90_ccw() {
        let mut img = make_test_rgba_image();
        let bytes = build_minimal_exif(8);
        let result = apply_exif_orientation(&mut img, &bytes);
        assert_eq!(result.width(), 2);
        assert_eq!(result.height(), 3);
    }
}
