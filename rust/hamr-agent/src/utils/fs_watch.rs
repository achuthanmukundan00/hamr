//! Port of `packages/coding-agent/src/utils/fs-watch.ts`
//!
//! Utilities for working with filesystem watchers.

/// Retry delay when watcher setup fails (in milliseconds).
pub const FS_WATCH_RETRY_DELAY_MS: u64 = 5000;

/// Close a filesystem watcher safely.
///
/// Errors are silently ignored (watchers may already be closed).
pub fn close_watcher(watcher: Option<notify::RecommendedWatcher>) {
    if let Some(w) = watcher {
        drop(w); // notify watcher closes on drop
    }
}

/// Create a filesystem watcher with an error handler.
///
/// If creation fails, `on_error` is called once and `None` is returned.
/// If an error occurs during watching, `on_error` is called.
pub fn watch_with_error_handler<P: AsRef<std::path::Path>>(
    path: P,
    on_event: impl Fn(notify::Event) + Send + 'static,
    on_error: impl Fn() + Send + 'static,
) -> Option<notify::RecommendedWatcher> {
    use notify::{Config, EventKind, RecommendedWatcher, RecursiveMode, Watcher};

    let path = path.as_ref().to_path_buf();
    let on_error_cell = std::sync::Arc::new(std::sync::Mutex::new(Some(on_error)));

    let on_error_clone = on_error_cell.clone();
    let mut watcher = match RecommendedWatcher::new(
        move |res: Result<notify::Event, notify::Error>| {
            match res {
                Ok(event) => {
                    // Only notify on content modifications and creations
                    match event.kind {
                        EventKind::Modify(_) | EventKind::Create(_) => {
                            on_event(event);
                        }
                        _ => {}
                    }
                }
                Err(_e) => {
                    if let Ok(mut guard) = on_error_clone.lock() {
                        if let Some(cb) = guard.take() {
                            drop(guard);
                            cb();
                        }
                    }
                }
            }
        },
        Config::default(),
    ) {
        Ok(w) => w,
        Err(_) => {
            on_error_cell
                .lock()
                .ok()
                .and_then(|mut g| g.take())
                .map(|cb| cb());
            return None;
        }
    };

    // Don't need to attach error listener separately — errors are delivered through the callback

    match watcher.watch(&path, RecursiveMode::NonRecursive) {
        Ok(()) => Some(watcher),
        Err(_) => {
            on_error_cell
                .lock()
                .ok()
                .and_then(|mut g| g.take())
                .map(|cb| cb());
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_close_watcher_none_is_safe() {
        close_watcher(None);
    }

    #[test]
    fn test_watch_with_error_handler_nonexistent_path() {
        let called = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let called_clone = called.clone();

        let result = watch_with_error_handler(
            "/nonexistent/path/that/does/not/exist",
            |_| {},
            move || {
                called_clone.store(true, std::sync::atomic::Ordering::SeqCst);
            },
        );

        // On Linux, notify may fail to watch a nonexistent path
        // On macOS with FSEvents, it may succeed (parent dir exists)
        // Either way, the function shouldn't panic
        if result.is_none() {
            assert!(called.load(std::sync::atomic::Ordering::SeqCst));
        }

        close_watcher(result);
    }
}
