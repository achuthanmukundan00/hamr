//! Port of `packages/coding-agent/src/hamr/store/sqlite-loader.ts`.
//!
//! Lazy loader for SQLite via rusqlite (bundled feature).
//!
//! In the TS source, this uses `better-sqlite3`, a native C++ addon loaded
//! via dynamic `require()`. The Rust equivalent uses `rusqlite` with the
//! `bundled` feature (which compiles SQLite from source, no system dependency).
//!
//! If SQLite is unavailable, Hamr continues without persistence.
//! Uses a one-time lazy initialization pattern — tried exactly once per process.

use std::sync::OnceLock;

/// Cached result of the SQLite attempt: either loaded successfully or failed.
static SQLITE_LOADED: OnceLock<Option<()>> = OnceLock::new();

/// Attempt to load SQLite (via rusqlite bundled).
///
/// Returns `true` if SQLite is available. Results are cached — subsequent
/// calls return the same result without retrying.
///
/// Mirror of `loadBetterSqlite3()` in the TS source.
pub fn load_sqlite() -> bool {
    SQLITE_LOADED
        .get_or_init(|| {
            // Try to open an in-memory database to verify rusqlite works.
            match rusqlite::Connection::open_in_memory() {
                Ok(conn) => {
                    // Verify we can actually execute SQL
                    if conn.execute_batch("SELECT 1").is_ok() {
                        let _ = conn.close();
                        Some(())
                    } else {
                        log_sqlite_unavailable(
                            "rusqlite loaded but could not execute SQL",
                        );
                        None
                    }
                }
                Err(e) => {
                    let install_hint = concat!(
                        "  rusqlite failed to load:\n",
                        "    Ensure the 'bundled' feature is enabled in Cargo.toml.\n",
                        "  Or install sqlite3 system library: brew install sqlite3 / apt install libsqlite3-dev"
                    );
                    log_sqlite_unavailable(&format!("rusqlite error: {}\n{}", e, install_hint));
                    None
                }
            }
        })
        .is_some()
}

/// Check if SQLite was already loaded successfully.
/// Returns `None` if it hasn't been tried yet, `Some(false)` if it failed,
/// `Some(true)` if it succeeded.
pub fn is_sqlite_loaded() -> Option<bool> {
    SQLITE_LOADED.get().map(|r| r.is_some())
}

/// Log a warning that SQLite is unavailable.
///
/// Mirror of the `console.warn` calls in the TS source.
fn log_sqlite_unavailable(message: &str) {
    eprintln!("[hamr] SQLite not available. FTS5 memory persistence disabled.");
    eprintln!("[hamr] {}", message);
}

/// Extension name constant.
pub const EXTENSION_NAME: &str = "hamr-sqlite-loader";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_sqlite() {
        // rusqlite with bundled feature should always be available
        assert!(
            load_sqlite(),
            "SQLite should be available (bundled feature)"
        );
    }

    #[test]
    fn test_is_sqlite_loaded_before_call() {
        // Reset the OnceLock for testing — not normally possible, but
        // we can test that after load_sqlite(), is_sqlite_loaded() returns Some(true)
        // Use a different approach: just load it
        let _ = load_sqlite();
        assert_eq!(is_sqlite_loaded(), Some(true));
    }

    #[test]
    fn test_load_sqlite_idempotent() {
        assert!(load_sqlite());
        assert!(load_sqlite()); // second call should return cached result
    }

    #[test]
    fn test_load_sqlite_creates_in_memory_db() {
        assert!(load_sqlite());
        // Verify we can actually create a connection after loading
        let conn = rusqlite::Connection::open_in_memory();
        assert!(conn.is_ok());
    }
}
