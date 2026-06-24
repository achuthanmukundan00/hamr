/**
 * Path confinement for file mutation/read tools.
 *
 * Hamr's file tools (read/write/edit) intentionally accept absolute paths so
 * the agent can operate across a repo and its dependencies. That openness is
 * also a host-compromise vector: a prompt-injected or jailbroken model could
 * read `~/.ssh/id_rsa` / `~/.aws/credentials` or establish persistence by
 * writing `~/.ssh/authorized_keys` or `~/.bashrc`.
 *
 * `PathGuard` applies a hard denylist of credential and persistence locations
 * on top of normal path resolution. It is on by default and can be disabled or
 * extended via settings. It is deliberately conservative: it blocks only
 * well-known sensitive locations so it does not break legitimate cross-tree
 * file operations.
 */
export interface PathGuardOptions {
    /** Master switch. Default: true. When false, all checks pass. */
    enabled?: boolean;
    /** Extra absolute paths (or globs anchored at home/cwd) to allow despite the denylist. */
    allowedPaths?: string[];
    /**
     * When true, enforce strict cwd-based sandboxing: all file operations are
     * confined to paths within the cwd. This is opt-in (default false) because
     * hamr agents legitimately operate across repo boundaries (dependencies,
     * system config, etc.). The denylist-based PathGuard remains the primary
     * security boundary.
     */
    strictCwd?: string;
}
export declare class PathGuard {
    readonly enabled: boolean;
    private readonly allowed;
    private readonly strictCwd;
    constructor(options?: PathGuardOptions);
    /**
     * Returns true if the candidate path is NOT within the strict cwd (if set).
     * When strictCwd is undefined, this always returns false (no restriction).
     */
    private isOutsideStrictCwd;
    private isExplicitlyAllowed;
    /**
     * Returns a denial reason if the write/edit target is forbidden, or
     * `undefined` if it is allowed.
     */
    deniedWriteReason(absolutePath: string): string | undefined;
    /**
     * Returns a denial reason if the read target is a credential file, or
     * `undefined` if it is allowed.
     */
    deniedReadReason(absolutePath: string): string | undefined;
    /** Assert a write target is allowed, throwing with a clear message if not. */
    assertWritable(absolutePath: string): void;
    /** Assert a read target is allowed, throwing with a clear message if not. */
    assertReadable(absolutePath: string): void;
}
export declare class PathGuardError extends Error {
    readonly path: string;
    readonly operation: "read" | "write";
    constructor(message: string, path: string, operation: "read" | "write");
}
//# sourceMappingURL=path-guard.d.ts.map