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
import type Database from "better-sqlite3";

let _Database: typeof Database | null | undefined;
const require = createRequire(import.meta.url);

/**
 * Synchronously load better-sqlite3.
 *
 * Uses a cached dynamic require() — tried exactly once per process.
 * Returns the Database constructor on success, null on failure.
 */
export function loadBetterSqlite3(): typeof Database | null {
	if (_Database !== undefined) return _Database;

	try {
		// Dynamic require — caught at runtime, doesn't block module loading
		const mod = require("better-sqlite3") as typeof Database;
		// Verify the native binding actually loads (require() can succeed while
		// the .node file is missing — the JS loads but the native addon fails
		// when you call `new Database()`).
		const db = new mod(":memory:");
		db.close();
		_Database = mod;
		return _Database;
	} catch (err) {
		_Database = null;
		console.warn(
			`[hamr] better-sqlite3 not available. FTS5 memory persistence disabled.\n` +
				`  If you just updated Node, run: npm install -g @skaft/hamr --build-from-source`,
		);
		if (err instanceof Error) {
			// Only log the message (not full stack) — the native addon path list is
			// already shown by the bindings module.
			console.warn(`[hamr] better-sqlite3 error: ${err.message}`);
		}
		return null;
	}
}

/**
 * Re-export the Database type for use in type annotations.
 * This is a type-only re-export — it doesn't trigger a runtime import.
 */
export type { Database };
