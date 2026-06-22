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
import { createRequire } from "node:module";
let _Database;
const require = createRequire(import.meta.url);
/**
 * Synchronously load better-sqlite3.
 *
 * Uses a cached dynamic require() — tried exactly once per process.
 * Returns the Database constructor on success, null on failure.
 */
export function loadBetterSqlite3() {
    if (_Database !== undefined)
        return _Database;
    try {
        // Dynamic require — caught at runtime, doesn't block module loading
        const mod = require("better-sqlite3");
        _Database = mod;
        return _Database;
    }
    catch (err) {
        _Database = null;
        console.warn(`[hamr] better-sqlite3 not available. FTS5 memory persistence disabled.`, `Install: cd packages/coding-agent && npm install better-sqlite3 --include=optional`);
        if (err instanceof Error && err.stack) {
            console.warn(`[hamr] better-sqlite3 load error: ${err.message}`);
        }
        return null;
    }
}
//# sourceMappingURL=sqlite-loader.js.map