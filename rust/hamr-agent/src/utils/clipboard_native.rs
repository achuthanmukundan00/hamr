//! Port of `packages/coding-agent/src/utils/clipboard-native.ts`
//!
//! Native clipboard loading via @mariozechner/clipboard (Node addon).
//! In Rust, clipboard access is platform-specific and not currently needed
//! for the initial port — the RPC mode uses stdin/stdout.

/// Clipboard module trait (mirrors the TS ClipboardModule interface).
pub trait ClipboardModule: Send + Sync {
    fn set_text(&self, text: &str) -> Result<(), String>;
    fn has_image(&self) -> bool;
    fn get_image_binary(&self) -> Result<Vec<u8>, String>;
}

/// Attempt to load a clipboard implementation.
///
/// The TS version tries multiple require roots to find `@mariozechner/clipboard`.
/// In Rust, we provide a no-op implementation by default; platform-specific
/// implementations can be registered via feature flags or external crates.
pub fn load_clipboard_native() -> Option<Box<dyn ClipboardModule>> {
    // The TS version tries multiple resolution roots starting from:
    // 1. moduleRequire (CJS require relative to the module)
    // 2. executableDirRequire (relative to the executable's package.json)
    //
    // In Rust, clipboard access is inherently platform-specific. For the initial
    // port, we return None (clipboard not available). Platform-specific implementations
    // can be added via conditional compilation or external crates.
    None
}

/// Global clipboard instance (lazily loaded).
///
/// Matches the TS pattern: `clipboard` is loaded once at module init.
/// Returns None when TERMUX_VERSION is set or there's no display.
pub fn get_clipboard() -> Option<&'static dyn ClipboardModule> {
    use std::sync::LazyLock;
    static CLIPBOARD: LazyLock<Option<Box<dyn ClipboardModule>>> = LazyLock::new(|| {
        // Skip clipboard on Termux
        if std::env::var("TERMUX_VERSION").is_ok() {
            return None;
        }
        // Skip clipboard on Linux without DISPLAY or WAYLAND_DISPLAY
        if cfg!(target_os = "linux")
            && std::env::var("DISPLAY").is_err()
            && std::env::var("WAYLAND_DISPLAY").is_err()
        {
            return None;
        }
        load_clipboard_native()
    });

    CLIPBOARD.as_deref()
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeClipboard;

    impl ClipboardModule for FakeClipboard {
        fn set_text(&self, _text: &str) -> Result<(), String> {
            Ok(())
        }

        fn has_image(&self) -> bool {
            true
        }

        fn get_image_binary(&self) -> Result<Vec<u8>, String> {
            Ok(vec![1, 2, 3])
        }
    }

    #[test]
    fn test_fake_clipboard_works() {
        let cb = FakeClipboard;
        assert!(cb.has_image());
        assert_eq!(cb.get_image_binary().unwrap(), vec![1, 2, 3]);
        assert!(cb.set_text("hello").is_ok());
    }

    #[test]
    fn test_load_clipboard_native_returns_none_by_default() {
        assert!(load_clipboard_native().is_none());
    }
}
