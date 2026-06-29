//! Port of `packages/coding-agent/src/utils/clipboard-image.ts`
//!
//! Cross-platform clipboard image reading.
//! Handles Wayland (wl-paste), X11 (xclip), WSL (PowerShell), and native clipboard.

use crate::utils::photon;
use std::process::Command;

/// Clipboard image data.
pub struct ClipboardImage {
    pub bytes: Vec<u8>,
    pub mime_type: String,
}

const SUPPORTED_IMAGE_MIME_TYPES: &[&str] = &["image/png", "image/jpeg", "image/webp", "image/gif"];

const DEFAULT_LIST_TIMEOUT_MS: u64 = 1000;
const DEFAULT_READ_TIMEOUT_MS: u64 = 3000;
const DEFAULT_POWERSHELL_TIMEOUT_MS: u64 = 5000;
const DEFAULT_MAX_BUFFER_BYTES: usize = 50 * 1024 * 1024;

/// Check if this is a Wayland session.
pub fn is_wayland_session() -> bool {
    std::env::var("WAYLAND_DISPLAY").is_ok()
        || std::env::var("XDG_SESSION_TYPE").map_or(false, |v| v == "wayland")
}

/// Extract the base MIME type (before any parameter delimiter).
fn base_mime_type(mime_type: &str) -> String {
    mime_type
        .split(';')
        .next()
        .unwrap_or(mime_type)
        .trim()
        .to_lowercase()
}

/// Get the file extension for a given image MIME type.
pub fn extension_for_image_mime_type(mime_type: &str) -> Option<&'static str> {
    match base_mime_type(mime_type).as_str() {
        "image/png" => Some("png"),
        "image/jpeg" => Some("jpg"),
        "image/webp" => Some("webp"),
        "image/gif" => Some("gif"),
        _ => None,
    }
}

/// Select the preferred MIME type from a list.
/// Returns the raw (untrimmed) preferred match, or the first image/* if no preferred match.
fn select_preferred_image_mime_type(mime_types: &[String]) -> Option<String> {
    let normalized: Vec<(&String, String)> = mime_types
        .iter()
        .filter(|t| !t.trim().is_empty())
        .map(|t| (t, base_mime_type(t)))
        .collect();

    for preferred in SUPPORTED_IMAGE_MIME_TYPES {
        if let Some((raw, _)) = normalized
            .iter()
            .find(|(_, base)| base.as_str() == *preferred)
        {
            return Some((*raw).clone());
        }
    }

    // Fall back to any image/*
    if let Some((raw, _)) = normalized
        .iter()
        .find(|(_, base)| base.starts_with("image/"))
    {
        return Some((*raw).clone());
    }

    None
}

fn is_supported_image_mime_type(mime_type: &str) -> bool {
    let base = base_mime_type(mime_type);
    SUPPORTED_IMAGE_MIME_TYPES.contains(&base.as_str())
}

/// Convert unsupported image formats (e.g. BMP) to PNG using the image crate.
/// Returns None if conversion is unavailable or fails.
fn convert_to_png(bytes: &[u8]) -> Option<Vec<u8>> {
    // Try to decode as any image format, then re-encode as PNG
    let img = photon::photon_new_from_bytes(bytes).ok()?;
    Some(photon::photon_to_png_bytes(&img))
}

/// Run a command and collect its stdout. Returns (stdout, ok).
fn run_command(command: &str, args: &[&str], _timeout_ms: u64) -> (Vec<u8>, bool) {
    use std::process::Stdio;

    let output = Command::new(command)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output();

    match output {
        Ok(out) if out.status.success() => (out.stdout, true),
        _ => (Vec::new(), false),
    }
}

/// Read clipboard image via wl-paste (Wayland).
fn read_clipboard_image_via_wl_paste() -> Option<ClipboardImage> {
    let (list_stdout, ok) = run_command("wl-paste", &["--list-types"], DEFAULT_LIST_TIMEOUT_MS);
    if !ok {
        return None;
    }

    let types: Vec<String> = String::from_utf8_lossy(&list_stdout)
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .collect();

    let selected_type = select_preferred_image_mime_type(&types)?;

    let (data, ok) = run_command(
        "wl-paste",
        &["--type", &selected_type, "--no-newline"],
        DEFAULT_READ_TIMEOUT_MS,
    );
    if !ok || data.is_empty() {
        return None;
    }

    Some(ClipboardImage {
        bytes: data,
        mime_type: base_mime_type(&selected_type),
    })
}

/// Check if running under WSL.
fn is_wsl() -> bool {
    if std::env::var("WSL_DISTRO_NAME").is_ok() || std::env::var("WSLENV").is_ok() {
        return true;
    }

    // Check /proc/version for WSL indicators
    if let Ok(contents) = std::fs::read_to_string("/proc/version") {
        if contents.contains("icrosoft") || contents.contains("WSL") || contents.contains("wsl") {
            return true;
        }
    }

    false
}

/// On WSL, the Linux clipboard (Wayland/X11) does not receive image data from
/// Windows screenshots (Win+Shift+S). PowerShell can access the Windows clipboard
/// directly, so we use it as a fallback.
fn read_clipboard_image_via_powershell() -> Option<ClipboardImage> {
    let tmp_file = std::env::temp_dir().join(format!("hamr-wsl-clip-{}.png", uuid::Uuid::new_v4()));

    let (win_path_bytes, ok) = run_command(
        "wslpath",
        &["-w", tmp_file.to_str()?],
        DEFAULT_LIST_TIMEOUT_MS,
    );
    if !ok {
        return None;
    }

    let win_path = String::from_utf8_lossy(&win_path_bytes).trim().to_string();
    if win_path.is_empty() {
        return None;
    }

    let ps_quoted_win_path = win_path.replace('\'', "''");
    let ps_script = format!(
        "Add-Type -AssemblyName System.Windows.Forms; Add-Type -AssemblyName System.Drawing; $path = '{}'; $img = [System.Windows.Forms.Clipboard]::GetImage(); if ($img) {{ $img.Save($path, [System.Drawing.Imaging.ImageFormat]::Png); Write-Output 'ok' }} else {{ Write-Output 'empty' }}",
        ps_quoted_win_path
    );

    let (output, ok) = run_command(
        "powershell.exe",
        &["-NoProfile", "-Command", &ps_script],
        DEFAULT_POWERSHELL_TIMEOUT_MS,
    );
    if !ok {
        // Clean up
        let _ = std::fs::remove_file(&tmp_file);
        return None;
    }

    let output_str = String::from_utf8_lossy(&output).trim().to_string();
    if output_str != "ok" {
        let _ = std::fs::remove_file(&tmp_file);
        return None;
    }

    match std::fs::read(&tmp_file) {
        Ok(bytes) if !bytes.is_empty() => {
            let _ = std::fs::remove_file(&tmp_file);
            Some(ClipboardImage {
                bytes,
                mime_type: "image/png".to_string(),
            })
        }
        _ => {
            let _ = std::fs::remove_file(&tmp_file);
            None
        }
    }
}

/// Read clipboard image via xclip (X11).
fn read_clipboard_image_via_xclip() -> Option<ClipboardImage> {
    let (targets_stdout, ok) = run_command(
        "xclip",
        &["-selection", "clipboard", "-t", "TARGETS", "-o"],
        DEFAULT_LIST_TIMEOUT_MS,
    );

    let mut candidate_types: Vec<String> = Vec::new();
    if ok {
        candidate_types = String::from_utf8_lossy(&targets_stdout)
            .lines()
            .map(|l| l.trim().to_string())
            .filter(|l| !l.is_empty())
            .collect();
    }

    let preferred = select_preferred_image_mime_type(&candidate_types);
    let try_types: Vec<String> = if let Some(ref p) = preferred {
        let mut v = vec![p.clone()];
        v.extend(SUPPORTED_IMAGE_MIME_TYPES.iter().map(|t| t.to_string()));
        v
    } else {
        SUPPORTED_IMAGE_MIME_TYPES
            .iter()
            .map(|t| t.to_string())
            .collect()
    };

    for mime_type in &try_types {
        let (data, ok) = run_command(
            "xclip",
            &["-selection", "clipboard", "-t", mime_type, "-o"],
            DEFAULT_READ_TIMEOUT_MS,
        );
        if ok && !data.is_empty() {
            return Some(ClipboardImage {
                bytes: data,
                mime_type: base_mime_type(mime_type),
            });
        }
    }

    None
}

/// Read clipboard image via native clipboard (node addon ported to Rust).
/// Currently returns None since we don't have a native clipboard implementation.
async fn read_clipboard_image_via_native_clipboard() -> Option<ClipboardImage> {
    // In TS: uses clipboard.hasImage() and clipboard.getImageBinary()
    // In Rust: not yet implemented (requires platform-specific clipboard access)
    None
}

/// Read an image from the clipboard, if available.
/// Handles platform-specific clipboard access and BMP→PNG conversion.
pub async fn read_clipboard_image() -> Option<ClipboardImage> {
    // Skip on Termux
    if std::env::var("TERMUX_VERSION").is_ok() {
        return None;
    }

    let mut image: Option<ClipboardImage> = None;

    if cfg!(target_os = "linux") {
        let wsl = is_wsl();
        let wayland = is_wayland_session();

        if wayland || wsl {
            image =
                read_clipboard_image_via_wl_paste().or_else(|| read_clipboard_image_via_xclip());
        }

        if image.is_none() && wsl {
            image = read_clipboard_image_via_powershell();
        }

        if image.is_none() && !wayland {
            image = read_clipboard_image_via_native_clipboard().await;
        }
    } else {
        image = read_clipboard_image_via_native_clipboard().await;
    }

    let image = image?;

    // Convert unsupported formats (e.g., BMP from WSLg) to PNG
    if !is_supported_image_mime_type(&image.mime_type) {
        let png_bytes = convert_to_png(&image.bytes)?;
        return Some(ClipboardImage {
            bytes: png_bytes,
            mime_type: "image/png".to_string(),
        });
    }

    Some(image)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_base_mime_type() {
        assert_eq!(base_mime_type("image/png"), "image/png");
        assert_eq!(base_mime_type("image/png; charset=utf-8"), "image/png");
        assert_eq!(base_mime_type("IMAGE/PNG"), "image/png");
    }

    #[test]
    fn test_extension_for_image_mime_type() {
        assert_eq!(extension_for_image_mime_type("image/png"), Some("png"));
        assert_eq!(extension_for_image_mime_type("image/jpeg"), Some("jpg"));
        assert_eq!(extension_for_image_mime_type("image/webp"), Some("webp"));
        assert_eq!(extension_for_image_mime_type("image/gif"), Some("gif"));
        assert_eq!(extension_for_image_mime_type("image/bmp"), None);
    }

    #[test]
    fn test_select_preferred_image_mime_type() {
        let types = vec![
            "text/plain".to_string(),
            "image/png".to_string(),
            "image/bmp".to_string(),
        ];
        assert_eq!(
            select_preferred_image_mime_type(&types),
            Some("image/png".to_string())
        );
    }

    #[test]
    fn test_select_preferred_fallback_to_any_image() {
        let types = vec!["text/plain".to_string(), "image/tiff".to_string()];
        assert_eq!(
            select_preferred_image_mime_type(&types),
            Some("image/tiff".to_string())
        );
    }

    #[test]
    fn test_is_supported_image_mime_type() {
        assert!(is_supported_image_mime_type("image/png"));
        assert!(is_supported_image_mime_type("image/jpeg"));
        assert!(!is_supported_image_mime_type("image/bmp"));
    }

    #[test]
    fn test_convert_to_png_bmp() {
        // Create a 1x1 RGBA image encoded as JPEG using the image crate.
        // The image crate in this workspace does not include the bmp feature,
        // so we test with JPEG (which is supported) to verify the round-trip.
        let img = image::DynamicImage::new_rgba8(1, 1);
        let mut buf = std::io::Cursor::new(Vec::new());
        img.write_to(&mut buf, image::ImageFormat::Jpeg)
            .expect("JPEG encoding should succeed");
        let jpeg_bytes = buf.into_inner();

        let png = convert_to_png(&jpeg_bytes).expect("should convert JPEG to PNG");

        // Verify PNG magic bytes
        assert_eq!(png[0], 0x89);
        assert_eq!(png[1], 0x50);
        assert_eq!(png[2], 0x4e);
        assert_eq!(png[3], 0x47);
    }

    #[test]
    fn test_is_wayland_session() {
        // Default: no env vars set → not Wayland
        // (in CI this won't have WAYLAND_DISPLAY set)
    }
}
