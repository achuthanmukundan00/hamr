//! Port of `packages/coding-agent/src/utils/deprecation.ts`
//!
//! Deprecation warning emitter that deduplicates per message.

use std::sync::LazyLock;
use std::sync::Mutex;

static EMITTED_WARNINGS: LazyLock<Mutex<std::collections::HashSet<String>>> =
    LazyLock::new(|| Mutex::new(std::collections::HashSet::new()));

/// Emit a deprecation warning to stderr. Each unique message is emitted at most once.
pub fn warn_deprecation(message: &str) {
    let mut emitted = EMITTED_WARNINGS.lock().unwrap();
    if !emitted.insert(message.to_string()) {
        return;
    }
    eprintln!("\x1b[33mDeprecation warning: {}\x1b[0m", message);
}

/// Clear deprecation warning state. Exported for tests.
pub fn clear_deprecation_warnings_for_tests() {
    EMITTED_WARNINGS.lock().unwrap().clear();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_warn_deprecation_emits_once() {
        clear_deprecation_warnings_for_tests();
        // First call — should "emit" (i.e., be inserted into the set).
        warn_deprecation("test message");
        // Second call — should be deduplicated.
        warn_deprecation("test message");

        let emitted = EMITTED_WARNINGS.lock().unwrap();
        assert_eq!(emitted.len(), 1);
        assert!(emitted.contains("test message"));
    }

    #[test]
    fn test_clear_deprecation_resets_state() {
        clear_deprecation_warnings_for_tests();
        warn_deprecation("msg1");
        clear_deprecation_warnings_for_tests();

        let emitted = EMITTED_WARNINGS.lock().unwrap();
        assert!(emitted.is_empty());
    }
}
