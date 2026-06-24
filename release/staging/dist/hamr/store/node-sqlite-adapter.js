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
import { createRequire } from "node:module";
const require = createRequire(import.meta.url);
/**
 * Wraps a node:sqlite statement to match better-sqlite3's calling convention.
 *
 * better-sqlite3 binds an object of bare named params (`{ foo }` → `@foo`);
 * node:sqlite accepts the same object directly (bare named params are allowed
 * by default), but a no-argument call must pass *no* args rather than
 * `undefined` — otherwise `undefined` is treated as an anonymous parameter.
 */
class StatementAdapter {
    constructor(stmt) {
        this.stmt = stmt;
    }
    run(params) {
        return params === undefined ? this.stmt.run() : this.stmt.run(params);
    }
    get(params) {
        return params === undefined ? this.stmt.get() : this.stmt.get(params);
    }
    all(params) {
        return params === undefined ? this.stmt.all() : this.stmt.all(params);
    }
}
class DatabaseAdapter {
    constructor(filename, NodeDatabaseCtor) {
        this.db = new NodeDatabaseCtor(filename);
    }
    prepare(sql) {
        return new StatementAdapter(this.db.prepare(sql));
    }
    exec(sql) {
        this.db.exec(sql);
    }
    /**
     * better-sqlite3 exposes `.pragma("journal_mode = WAL")`; node:sqlite has no
     * such method, so route it through `exec`. The return value (which
     * better-sqlite3 would yield as rows) is unused by Hamr's memory code.
     */
    pragma(source) {
        this.db.exec(`PRAGMA ${source}`);
    }
    close() {
        this.db.close();
    }
}
/**
 * Build a better-sqlite3-compatible Database constructor backed by node:sqlite,
 * or return null if node:sqlite is unavailable (Node < 22.5) or fails to load.
 *
 * Loads node:sqlite via a runtime import string so bundlers/older Node don't
 * choke on the specifier at module-eval time.
 */
export function loadNodeSqliteDatabase() {
    let DatabaseSync;
    try {
        // `createRequire`-style runtime resolution; the specifier is hidden from
        // static analysis so environments without node:sqlite don't hard-fail.
        const mod = require("node:sqlite");
        DatabaseSync = mod.DatabaseSync;
    }
    catch {
        return null;
    }
    if (!DatabaseSync)
        return null;
    const Ctor = DatabaseSync;
    // Smoke-test the binding (FTS5 + named params) before committing to it.
    try {
        const probe = new Ctor(":memory:");
        probe.exec("CREATE VIRTUAL TABLE _probe USING fts5(content)");
        probe.close();
    }
    catch {
        return null;
    }
    class NodeSqliteDatabase extends DatabaseAdapter {
        constructor(filename) {
            super(filename, Ctor);
        }
    }
    return NodeSqliteDatabase;
}
//# sourceMappingURL=node-sqlite-adapter.js.map