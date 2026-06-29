//! Port of `packages/ai/src/session-resources.ts`.
//!
//! A global registry of session-scoped cleanup callbacks. Mirrors the TS
//! `Set<SessionResourceCleanup>` with register/unregister and a `cleanup`
//! sweep that collects failures (the TS `AggregateError`).

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, LazyLock, Mutex};

/// A cleanup callback invoked with the optional session id being torn down.
pub type SessionResourceCleanup = Arc<dyn Fn(Option<&str>) + Send + Sync>;

static NEXT_ID: AtomicU64 = AtomicU64::new(0);
static CLEANUPS: LazyLock<Mutex<Vec<(u64, SessionResourceCleanup)>>> =
    LazyLock::new(|| Mutex::new(Vec::new()));

/// Register a cleanup callback. Returns an unregister function (mirrors the TS
/// `() => void` deregistration handle).
pub fn register_session_resource_cleanup(
    cleanup: SessionResourceCleanup,
) -> Box<dyn Fn() + Send + Sync> {
    let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
    if let Ok(mut cleanups) = CLEANUPS.lock() {
        cleanups.push((id, cleanup));
    }
    Box::new(move || {
        if let Ok(mut cleanups) = CLEANUPS.lock() {
            cleanups.retain(|(existing, _)| *existing != id);
        }
    })
}

/// Run all registered cleanups for `session_id`, collecting failures.
///
/// Returns `Err(n)` when `n` callbacks panicked, mirroring the TS
/// `AggregateError(errors, "Failed to cleanup session resources")`.
pub fn cleanup_session_resources(session_id: Option<&str>) -> Result<(), usize> {
    let snapshot: Vec<SessionResourceCleanup> = CLEANUPS
        .lock()
        .map(|cleanups| cleanups.iter().map(|(_, c)| Arc::clone(c)).collect())
        .unwrap_or_default();

    let mut failures = 0;
    for cleanup in snapshot {
        if catch_unwind(AssertUnwindSafe(|| cleanup(session_id))).is_err() {
            failures += 1;
        }
    }

    if failures > 0 { Err(failures) } else { Ok(()) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicUsize;

    // The registry is global, so serialize tests that mutate it and ensure each
    // unregisters its callbacks before releasing the lock.
    static TEST_LOCK: Mutex<()> = Mutex::new(());

    fn lock() -> std::sync::MutexGuard<'static, ()> {
        TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner())
    }

    #[test]
    fn register_invoke_and_unregister() {
        let _guard = lock();
        let calls = Arc::new(AtomicUsize::new(0));
        let calls2 = Arc::clone(&calls);
        let unregister = register_session_resource_cleanup(Arc::new(move |_id| {
            calls2.fetch_add(1, Ordering::Relaxed);
        }));

        cleanup_session_resources(Some("s1")).unwrap();
        assert_eq!(calls.load(Ordering::Relaxed), 1);

        unregister();
        cleanup_session_resources(Some("s1")).unwrap();
        assert_eq!(
            calls.load(Ordering::Relaxed),
            1,
            "unregistered callback should not run"
        );
    }

    #[test]
    fn collects_panicking_cleanups() {
        let _guard = lock();
        let unregister = register_session_resource_cleanup(Arc::new(|_id| {
            panic!("boom");
        }));
        let result = cleanup_session_resources(None);
        unregister();
        assert!(matches!(result, Err(n) if n >= 1));
    }
}
