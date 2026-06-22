/**
 * Lazy loader for better-sqlite3 — a native C++ addon.
 *
 * better-sqlite3 ships prebuilds for common platforms (macOS, Linux, Windows
 * on x64/arm64). On unusual platforms it falls back to node-gyp compilation.
 *
 * If compilation fails or better-sqlite3 is unavailable, Hamr continues
 * without SQLite persistence (memory, event store, FTS5 are no-ops).
 *
 * Uses a synchronous dynamic require() inside try/catch so that the
 * rest of the module graph loads fine regardless.
 */
import type Database from "better-sqlite3";
/**
 * Synchronously load better-sqlite3.
 *
 * Uses a cached dynamic require() — tried exactly once per process.
 * Returns the Database constructor on success, null on failure.
 */
export declare function loadBetterSqlite3(): typeof Database | null;
/**
 * Re-export the Database type for use in type annotations.
 * This is a type-only re-export — it doesn't trigger a runtime import.
 */
export type { Database };
//# sourceMappingURL=sqlite-loader.d.ts.map