//! Port of `packages/coding-agent/src/core/experimental.ts`.
//!
//! Feature-flag gating for experimental functionality.

/// Check whether experimental features are enabled via the environment.
///
/// Reads `HAMR_EXPERIMENTAL` first, falls back to `PI_EXPERIMENTAL` for
/// backwards compatibility with the pre-rename environment variable name.
pub fn are_experimental_features_enabled() -> bool {
    std::env::var("HAMR_EXPERIMENTAL")
        .or_else(|_| std::env::var("PI_EXPERIMENTAL"))
        .map(|v| v == "1")
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{LazyLock, Mutex};

    /// Env var manipulation is process-global — serialize all tests in this module.
    static ENV_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

    #[test]
    fn disabled_by_default() {
        let _guard = ENV_LOCK.lock().unwrap();
        unsafe {
            std::env::remove_var("HAMR_EXPERIMENTAL");
        }
        unsafe {
            std::env::remove_var("PI_EXPERIMENTAL");
        }
        assert!(!are_experimental_features_enabled());
    }

    #[test]
    fn hamr_experimental_true() {
        let _guard = ENV_LOCK.lock().unwrap();
        unsafe {
            std::env::set_var("HAMR_EXPERIMENTAL", "1");
        }
        unsafe {
            std::env::remove_var("PI_EXPERIMENTAL");
        }
        assert!(are_experimental_features_enabled());
    }

    #[test]
    fn pi_experimental_true() {
        let _guard = ENV_LOCK.lock().unwrap();
        unsafe {
            std::env::remove_var("HAMR_EXPERIMENTAL");
        }
        unsafe {
            std::env::set_var("PI_EXPERIMENTAL", "1");
        }
        assert!(are_experimental_features_enabled());
    }

    #[test]
    fn hamr_takes_precedence() {
        let _guard = ENV_LOCK.lock().unwrap();
        unsafe {
            std::env::set_var("HAMR_EXPERIMENTAL", "1");
        }
        unsafe {
            std::env::set_var("PI_EXPERIMENTAL", "0");
        }
        assert!(are_experimental_features_enabled());
    }

    #[test]
    fn non_one_is_false() {
        let _guard = ENV_LOCK.lock().unwrap();
        unsafe {
            std::env::set_var("HAMR_EXPERIMENTAL", "yes");
        }
        unsafe {
            std::env::remove_var("PI_EXPERIMENTAL");
        }
        assert!(!are_experimental_features_enabled());
    }

    #[test]
    fn empty_string_is_false() {
        let _guard = ENV_LOCK.lock().unwrap();
        unsafe {
            std::env::set_var("HAMR_EXPERIMENTAL", "");
        }
        unsafe {
            std::env::remove_var("PI_EXPERIMENTAL");
        }
        assert!(!are_experimental_features_enabled());
    }
}
