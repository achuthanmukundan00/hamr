/**
 * Pure-JS fallback for better-sqlite3, backed by Node's built-in `node:sqlite`.
 *
 * better-sqlite3 is a native C++ addon that needs a prebuilt binary (or a
 * working node-gyp toolchain) for the running Node ABI. On bleeding-edge Node
 * releases no prebuild exists yet, so `npm install -g @skaft/hamr` falls back to
 * source compilation — which fails on machines without build tools, silently
 * disabling memory.
 *
 * `node:sqlite` ships *inside* Node (stable since Node 24), needs no
 * compilation, and is built with FTS5 enabled — exactly the subset Hamr's
 * memory uses. This adapter wraps `DatabaseSync` to expose the small slice of
 * the better-sqlite3 API that HolographicMemory / FactStore depend on:
 *
 *   new Database(path) · db.prepare() · db.exec() · db.pragma() · db.close()
 *   stmt.run() · stmt.get() · stmt.all()  (with `@name` bare-named params)
 */
import type Database from "better-sqlite3";
/**
 * Build a better-sqlite3-compatible Database constructor backed by node:sqlite,
 * or return null if node:sqlite is unavailable (Node < 22.5) or fails to load.
 *
 * Loads node:sqlite via a runtime import string so bundlers/older Node don't
 * choke on the specifier at module-eval time.
 */
export declare function loadNodeSqliteDatabase(): typeof Database | null;
//# sourceMappingURL=node-sqlite-adapter.d.ts.map